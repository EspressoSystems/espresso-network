// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

//! Node storage implementation for a database query engine.

use std::ops::{Bound, RangeBounds};

use alloy::primitives::map::HashMap;
use anyhow::anyhow;
use async_trait::async_trait;
use futures::stream::{StreamExt, TryStreamExt};
use hotshot_types::{
    data::VidShare,
    simple_certificate::CertificatePair,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
};
use snafu::OptionExt;
use tracing::instrument;

use super::{
    super::transaction::{Transaction, TransactionMode, Write, query, query_as},
    DecodeError, HEADER_COLUMNS, QueryBuilder, parse_header,
};
use crate::{
    Header, MissingSnafu, QueryError, QueryResult,
    availability::{NamespaceId, QueryableHeader},
    data_source::storage::{
        Aggregate, AggregatesStorage, NodeStorage, PayloadMetadata, UpdateAggregatesStorage,
    },
    node::{
        BlockId, ResourceSyncStatus, SyncStatus, SyncStatusQueryData, SyncStatusRange,
        TimeWindowQueryData, WindowStart,
    },
    types::HeightIndexed,
};

#[async_trait]
impl<Mode, Types> NodeStorage<Types> for Transaction<Mode>
where
    Mode: TransactionMode,
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
{
    async fn block_height(&mut self) -> QueryResult<usize> {
        match query_as::<(Option<i64>,)>("SELECT max(height) FROM header")
            .fetch_one(self.as_mut())
            .await?
        {
            (Some(height),) => {
                // The height of the block is the number of blocks below it, so the total number of
                // blocks is one more than the height of the highest block.
                Ok(height as usize + 1)
            },
            (None,) => {
                // If there are no blocks yet, the height is 0.
                Ok(0)
            },
        }
    }

    async fn count_transactions_in_range(
        &mut self,
        range: impl RangeBounds<usize> + Send,
        namespace: Option<NamespaceId<Types>>,
    ) -> QueryResult<usize> {
        let namespace: i64 = namespace.map(|ns| ns.into()).unwrap_or(-1);
        let Some((from, to)) = aggregate_range_bounds::<Types>(self, range).await? else {
            return Ok(0);
        };
        let (count,) = query_as::<(i64,)>(
            "SELECT num_transactions FROM aggregate WHERE height = $1 AND namespace = $2",
        )
        .bind(to as i64)
        .bind(namespace)
        .fetch_one(self.as_mut())
        .await?;
        let mut count = count as usize;

        if from > 0 {
            let (prev_count,) = query_as::<(i64,)>(
                "SELECT num_transactions FROM aggregate WHERE height = $1 AND namespace = $2",
            )
            .bind((from - 1) as i64)
            .bind(namespace)
            .fetch_one(self.as_mut())
            .await?;
            count = count.saturating_sub(prev_count as usize);
        }

        Ok(count)
    }

    async fn payload_size_in_range(
        &mut self,
        range: impl RangeBounds<usize> + Send,
        namespace: Option<NamespaceId<Types>>,
    ) -> QueryResult<usize> {
        let namespace: i64 = namespace.map(|ns| ns.into()).unwrap_or(-1);
        let Some((from, to)) = aggregate_range_bounds::<Types>(self, range).await? else {
            return Ok(0);
        };
        let (size,) = query_as::<(i64,)>(
            "SELECT payload_size FROM aggregate WHERE height = $1 AND namespace = $2",
        )
        .bind(to as i64)
        .bind(namespace)
        .fetch_one(self.as_mut())
        .await?;
        let mut size = size as usize;

        if from > 0 {
            let (prev_size,) = query_as::<(i64,)>(
                "SELECT payload_size FROM aggregate WHERE height = $1 AND namespace = $2",
            )
            .bind((from - 1) as i64)
            .bind(namespace)
            .fetch_one(self.as_mut())
            .await?;
            size = size.saturating_sub(prev_size as usize);
        }

        Ok(size)
    }

    async fn vid_share<ID>(&mut self, id: ID) -> QueryResult<VidShare>
    where
        ID: Into<BlockId<Types>> + Send + Sync,
    {
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id.into())?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT v.share AS share FROM vid2 AS v
               JOIN header AS h ON v.height = h.height
              WHERE {where_clause}
              ORDER BY h.height
              LIMIT 1"
        );
        let (share_data,) = query
            .query_as::<(Option<Vec<u8>>,)>(&sql)
            .fetch_one(self.as_mut())
            .await?;
        let share_data = share_data.context(MissingSnafu)?;
        let share = bincode::deserialize(&share_data).decode_error("malformed VID share")?;
        Ok(share)
    }

    async fn sync_status_for_range(
        &mut self,
        from: usize,
        to: usize,
    ) -> QueryResult<SyncStatusQueryData> {
        // A block can be missing if its corresponding leaf is missing or if the block's `size`,
        // `data`, and `num_transactions` fields are `NULL`. We use `size` as the indicator column
        // to capture this while avoiding touching `data`, which can be quite large.
        let blocks = self.sync_status_ranges("payload", "size", from, to).await?;

        let leaves = if blocks.is_fully_synced() {
            // A common special case is that there are no missing blocks. In this case, we already
            // know there are no missing leaves either, since a block can only be present if we
            // already have the corresponding leaf. Just return the fully-synced status for leaves
            // without doing another expensive query.
            blocks.clone()
        } else {
            // A leaf can only be missing if there is no row for it in the database (all its columns
            // are non-nullable). We use `height` as an indicator for `NULL` rows in an inner join,
            // which allows an index-only scan.
            self.sync_status_ranges("leaf2", "height", from, to).await?
        };

        // For VID, common data can only be missing if the entire row is missing, so we use an
        // index-only scan over `height`.
        let vid_common = self.sync_status_ranges("vid2", "height", from, to).await?;
        // VID shares can be missing in that case _or_ if the row is present but share data is NULL,
        // so we use the `share` column as an indicator.
        let vid_shares = self.sync_status_ranges("vid2", "share", from, to).await?;

        Ok(SyncStatusQueryData {
            leaves,
            blocks,
            vid_common,
            vid_shares,
            pruned_height: None,
        })
    }

    async fn get_header_window(
        &mut self,
        start: impl Into<WindowStart<Types>> + Send + Sync,
        end: u64,
        limit: usize,
    ) -> QueryResult<TimeWindowQueryData<Header<Types>>> {
        // Find the specific block that starts the requested window.
        let first_block = match start.into() {
            WindowStart::Time(t) => {
                // If the request is not to start from a specific block, but from a timestamp, we
                // use a different method to find the window, as detecting whether we have
                // sufficient data to answer the query is not as simple as just trying `load_header`
                // for a specific block ID.
                return self.time_window::<Types>(t, end, limit).await;
            },
            WindowStart::Height(h) => h,
            WindowStart::Hash(h) => self.load_header::<Types>(h).await?.block_number(),
        };

        // Find all blocks starting from `first_block` with timestamps less than `end`. Block
        // timestamps are monotonically increasing, so this query is guaranteed to return a
        // contiguous range of blocks ordered by increasing height.
        let sql = format!(
            "SELECT {HEADER_COLUMNS}
               FROM header AS h
              WHERE h.height >= $1 AND h.timestamp < $2
              ORDER BY h.height
              LIMIT $3"
        );
        let rows = query(&sql)
            .bind(first_block as i64)
            .bind(end as i64)
            .bind(limit as i64)
            .fetch(self.as_mut());
        let window = rows
            .map(|row| parse_header::<Types>(row?))
            .try_collect::<Vec<_>>()
            .await?;

        // Find the block just before the window.
        let prev = if first_block > 0 {
            Some(self.load_header::<Types>(first_block as usize - 1).await?)
        } else {
            None
        };

        let next = if window.len() < limit {
            // If we are not limited, complete the window by finding the block just after the
            // window. We order by timestamp _then_ height, because the timestamp order allows the
            // query planner to use the index on timestamp to also efficiently solve the WHERE
            // clause, but this process may turn up multiple results, due to the 1-second resolution
            // of block timestamps. The final sort by height guarantees us a unique, deterministic
            // result (the first block with a given timestamp). This sort may not be able to use an
            // index, but it shouldn't be too expensive, since there will never be more than a
            // handful of blocks with the same timestamp.
            let sql = format!(
                "SELECT {HEADER_COLUMNS}
               FROM header AS h
              WHERE h.timestamp >= $1
              ORDER BY h.timestamp, h.height
              LIMIT 1"
            );
            query(&sql)
                .bind(end as i64)
                .fetch_optional(self.as_mut())
                .await?
                .map(parse_header::<Types>)
                .transpose()?
        } else {
            // If we have been limited, return a `null` next block indicating an incomplete window.
            // The client will have to query again with an adjusted starting point to get subsequent
            // results.
            tracing::debug!(limit, "cutting off header window request due to limit");
            None
        };

        Ok(TimeWindowQueryData { window, prev, next })
    }

    async fn latest_qc_chain(&mut self) -> QueryResult<Option<[CertificatePair<Types>; 2]>> {
        let Some((json,)) = query_as("SELECT qcs FROM latest_qc_chain LIMIT 1")
            .fetch_optional(self.as_mut())
            .await?
        else {
            return Ok(None);
        };
        let qcs = serde_json::from_value(json).decode_error("malformed QC")?;
        Ok(qcs)
    }
}

impl<Mode> Transaction<Mode>
where
    Mode: TransactionMode,
{
    /// Characterize consecutive ranges of objects in the given `height`-indexed table by status.
    ///
    /// This function will find all ranges in `[0, block_height)`. If `pruned_height` is specified,
    /// an initial range will be created with bounds `[0, pruned_height]` and status
    /// [`SyncStatus::Pruned`]. Then only the range `[pruned_height + 1, block_height)` will
    /// actually be searched.
    ///
    /// The search process uses an indexed outer self-join on `table`, which requires traversing
    /// the table twice. Thus, it can be fairly expensive on large tables, but it is still linear in
    /// the size of the table.
    ///
    /// The value of `indicator_column` in the outer join results is used to check for missing
    /// objects (indicated by a `NULL` value). If `indicator_column` is a `NOT NULL` column, such as
    /// `height`, then this function will only consider objects missing if there is no corresponding
    /// row in the database at all. However, `indicator_column` may also be a nullable column (such
    /// as `payload.data`, in which case objects are treated as missing if there is no corresponding
    /// row _or_ if there is a row but it has an explicit `NULL` value for `indicator_column`).
    #[instrument(skip(self))]
    async fn sync_status_ranges(
        &mut self,
        table: &str,
        indicator_column: &str,
        start: usize,
        end: usize,
    ) -> QueryResult<ResourceSyncStatus> {
        let mut ranges = vec![];
        tracing::debug!("searching for missing ranges");

        // Find every height in the range `[start, end)` which is the first height in a sequence of
        // present objects (i.e. the object just before it is missing).
        //
        // We do this by outer joining the table with itself, with height shifted by one, to get a
        // combined table where each row contains a successor object and its immediate predecessor,
        // if present. If the predecessor is missing, its height will be `NULL` (which is impossible
        // otherwise, because `height` is a `NOT NULL` column).
        //
        // For each table in the self-join, we _first_ sub-select just the range of interest (i.e.
        // [start, end) for the successor table and [start - 1, end - 1) for the predecessor table).
        // It is more efficient to do this first to reduce the number of rows involved in the join,
        // which is the expensive part of the operation. In fact, due to the nature of the outer
        // join, it is impossible to do this filtering after the join for the predecessor table,
        // since at that point the table will not necessarily be indexed and will contain some rows
        // with `NULL` height.
        let query = format!(
            "WITH range AS (SELECT height, {indicator_column} AS indicator FROM {table}
                WHERE height >= $1 AND height < $2)
            SELECT successor.height FROM range AS predecessor
            RIGHT JOIN range AS successor
            ON successor.height = predecessor.height + 1
            WHERE successor.indicator IS NOT NULL
              AND predecessor.indicator IS NULL
            ORDER BY successor.height"
        );
        let range_starts = query_as::<(i64,)>(&query)
            .bind(start as i64)
            .bind(end as i64)
            .fetch_all(self.as_mut())
            .await?;
        tracing::debug!(
            ?range_starts,
            "found {} starting heights for present ranges",
            range_starts.len()
        );

        let range_ends = if range_starts.len() <= 10 {
            // In the common case, where we are mostly or entirely synced and only missing a few
            // objects, chopping the space into only a small number of present ranges, it is more
            // efficient to pick out reach range's end individually with a specific, efficient,
            // height-indexed query, rather than execute another very expensive query which is the
            // mirror of the `range_starts` query to load all the range ends in bulk.
            let mut ends = vec![];
            for (i, &(start,)) in range_starts.iter().enumerate() {
                // We can easily find the end of the range from the start by finding the maximum
                // height which is still present between the start and the next range's start.
                let query = format!(
                    "SELECT max(height) from {table}
                      WHERE height < $1 AND {indicator_column} IS NOT NULL"
                );
                let upper_bound = if i + 1 < range_starts.len() {
                    range_starts[i + 1].0
                } else {
                    end as i64
                };
                let (end,) = query_as::<(i64,)>(&query)
                    .bind(upper_bound)
                    // This query is guaranteed to return one result, since `start` satisfies the
                    // requirements even if nothing else does.
                    .fetch_one(self.as_mut())
                    .await?;
                tracing::debug!(start, end, "found end for present range");
                ends.push((end,));
            }
            ends
        } else {
            // When the number of distinct ranges becomes large, making many small queries to fetch
            // each specific range end becomes inefficient because it is dominated by overhead. In
            // this case, we fall back to fetching the range ends using a single moderately
            // expensive query, which is the mirror image of the query we used to fetch the range
            // starts.
            let query = format!(
                "WITH range AS (SELECT height, {indicator_column} AS indicator FROM {table}
                    WHERE height >= $1 AND height < $2)
                SELECT predecessor.height FROM range AS predecessor
                LEFT JOIN range AS successor
                ON successor.height = predecessor.height + 1
                WHERE predecessor.indicator IS NOT NULL
                  AND successor.indicator IS NULL
                ORDER BY predecessor.height"
            );
            let ends = query_as::<(i64,)>(&query)
                .bind(start as i64)
                .bind(end as i64)
                .fetch_all(self.as_mut())
                .await?;
            tracing::debug!(
                ?ends,
                "found {} ending heights for present ranges",
                ends.len()
            );
            ends
        };

        // Sanity check: every range has a start and an end.
        if range_starts.len() != range_ends.len() {
            return Err(QueryError::Error {
                message: format!(
                    "number of present range starts ({}) does not match number of present range \
                     ends ({})",
                    range_starts.len(),
                    range_ends.len(),
                ),
            });
        }

        // Now we can simply zip `range_starts` and `range_ends` to find the full sequence of
        // [`SyncStatus::Present`] ranges. We can then interpolate [`SyncStatus::Missing`] ranges
        // between each present range.
        let mut prev = start;
        for ((start,), (end,)) in range_starts.into_iter().zip(range_ends) {
            let start = start as usize;
            let end = end as usize;

            // Sanity check range bounds.
            if start < prev {
                return Err(QueryError::Error {
                    message: format!(
                        "found present ranges out of order: range start {start} is before \
                         previous range end {prev}"
                    ),
                });
            }
            if end < start {
                return Err(QueryError::Error {
                    message: format!("malformed range: start={start}, end={end}"),
                });
            }

            if start != prev {
                // There is a range in between this one and the previous one, which must correspond
                // to missing objects.
                tracing::debug!(start = prev, end = start, "found missing range");
                ranges.push(SyncStatusRange {
                    start: prev,
                    end: start,
                    status: SyncStatus::Missing,
                });
            }

            ranges.push(SyncStatusRange {
                start,
                end: end + 1, // convert inclusive range to exclusive
                status: SyncStatus::Present,
            });
            prev = end + 1;
        }

        // There is possibly one more missing range, between the final present range and the overall
        // block height.
        if prev != end {
            tracing::debug!(start = prev, end, "found missing range");
            ranges.push(SyncStatusRange {
                start: prev,
                end,
                status: SyncStatus::Missing,
            });
        }

        let missing = ranges
            .iter()
            .filter_map(|range| {
                if range.status == SyncStatus::Missing {
                    Some(range.end - range.start)
                } else {
                    None
                }
            })
            .sum();
        tracing::debug!(
            missing,
            "found missing objects in {} total ranges",
            ranges.len()
        );

        Ok(ResourceSyncStatus { missing, ranges })
    }
}

impl<Types, Mode: TransactionMode> AggregatesStorage<Types> for Transaction<Mode>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
{
    async fn aggregates_height(&mut self) -> anyhow::Result<usize> {
        let (height,): (i64,) = query_as("SELECT coalesce(max(height) + 1, 0) FROM aggregate")
            .fetch_one(self.as_mut())
            .await?;
        Ok(height as usize)
    }

    async fn load_prev_aggregate(&mut self) -> anyhow::Result<Option<Aggregate<Types>>> {
        // Get the maximum height for which we have stored aggregated results
        // then query all the namespace info for that height
        let res: (Option<i64>,) =
            query_as("SELECT max(height) FROM aggregate WHERE namespace = -1")
                .fetch_one(self.as_mut())
                .await?;

        let (Some(max_height),) = res else {
            return Ok(None);
        };

        let rows: Vec<(i64, i64, i64)> = query_as(
            r#"
        SELECT namespace, num_transactions, payload_size from aggregate WHERE height = $1
        "#,
        )
        .bind(max_height)
        .fetch_all(self.as_mut())
        .await?;

        let mut num_transactions = HashMap::default();
        let mut payload_size = HashMap::default();

        for (namespace_id, num_tx, payload_sz) in rows {
            // Null namespace is represented as - 1 in database
            // as it is part of primary key and primary key can not be NULL
            // This namespace represents the cumulative sum of all the namespaces
            let key = if namespace_id == -1 {
                None
            } else {
                Some(namespace_id.into())
            };
            num_transactions.insert(key, num_tx as usize);
            payload_size.insert(key, payload_sz as usize);
        }

        Ok(Some(Aggregate {
            height: max_height,
            num_transactions,
            payload_size,
        }))
    }
}

impl<Types: NodeType> UpdateAggregatesStorage<Types> for Transaction<Write>
where
    Header<Types>: QueryableHeader<Types>,
{
    async fn update_aggregates(
        &mut self,
        prev: Aggregate<Types>,
        blocks: &[PayloadMetadata<Types>],
    ) -> anyhow::Result<Aggregate<Types>> {
        let height = blocks[0].height();
        let (prev_tx_count, prev_size) = (prev.num_transactions, prev.payload_size);

        let mut rows = Vec::new();

        // Cumulatively sum up new statistics for each block in this chunk.
        let aggregates = blocks
            .iter()
            .scan(
                (height, prev_tx_count, prev_size),
                |(height, tx_count, size), block| {
                    if *height != block.height {
                        return Some(Err(anyhow!(
                            "blocks in update_aggregates are not sequential; expected {}, got {}",
                            *height,
                            block.height()
                        )));
                    }
                    *height += 1;

                    //  Update total global stats
                    // `None` represents stats across all namespaces.
                    // It is represented as -1 in database

                    *tx_count.entry(None).or_insert(0) += block.num_transactions as usize;
                    *size.entry(None).or_insert(0) += block.size as usize;

                    // Add row for global cumulative stats (namespace = -1)

                    rows.push((
                        block.height as i64,
                        -1,
                        tx_count[&None] as i64,
                        size[&None] as i64,
                    ));

                    // Update per-namespace cumulative stats
                    for (&ns_id, info) in &block.namespaces {
                        let key = Some(ns_id);

                        *tx_count.entry(key).or_insert(0) += info.num_transactions as usize;
                        *size.entry(key).or_insert(0) += info.size as usize;
                    }

                    //  Insert cumulative stats for all known namespaces
                    // Even if a namespace wasn't present in this block,
                    // we still insert its latest cumulative stats at this height.
                    for ns_id in tx_count.keys().filter_map(|k| k.as_ref()) {
                        let key = Some(*ns_id);
                        rows.push((
                            block.height as i64,
                            (*ns_id).into(),
                            tx_count[&key] as i64,
                            size[&key] as i64,
                        ));
                    }

                    Some(Ok((block.height as i64, tx_count.clone(), size.clone())))
                },
            )
            .collect::<anyhow::Result<Vec<_>>>()?;
        let last_aggregate = aggregates.last().cloned();

        let (height, num_transactions, payload_size) =
            last_aggregate.ok_or_else(|| anyhow!("no row"))?;

        self.upsert(
            "aggregate",
            ["height", "namespace", "num_transactions", "payload_size"],
            ["height", "namespace"],
            rows,
        )
        .await?;
        Ok(Aggregate {
            height,
            num_transactions,
            payload_size,
        })
    }
}

impl<Mode: TransactionMode> Transaction<Mode> {
    async fn time_window<Types: NodeType>(
        &mut self,
        start: u64,
        end: u64,
        limit: usize,
    ) -> QueryResult<TimeWindowQueryData<Header<Types>>> {
        // Find all blocks whose timestamps fall within the window [start, end). Block timestamps
        // are monotonically increasing, so this query is guaranteed to return a contiguous range of
        // blocks ordered by increasing height.
        //
        // We order by timestamp _then_ height, because the timestamp order allows the query planner
        // to use the index on timestamp to also efficiently solve the WHERE clause, but this
        // process may turn up multiple results, due to the 1-second resolution of block timestamps.
        // The final sort by height guarantees us a unique, deterministic result (the first block
        // with a given timestamp). This sort may not be able to use an index, but it shouldn't be
        // too expensive, since there will never be more than a handful of blocks with the same
        // timestamp.
        let sql = format!(
            "SELECT {HEADER_COLUMNS}
               FROM header AS h
              WHERE h.timestamp >= $1 AND h.timestamp < $2
              ORDER BY h.timestamp, h.height
              LIMIT $3"
        );
        let rows = query(&sql)
            .bind(start as i64)
            .bind(end as i64)
            .bind(limit as i64)
            .fetch(self.as_mut());
        let window: Vec<_> = rows
            .map(|row| parse_header::<Types>(row?))
            .try_collect()
            .await?;

        let next = if window.len() < limit {
            // If we are not limited, complete the window by finding the block just after.
            let sql = format!(
                "SELECT {HEADER_COLUMNS}
               FROM header AS h
              WHERE h.timestamp >= $1
              ORDER BY h.timestamp, h.height
              LIMIT 1"
            );
            query(&sql)
                .bind(end as i64)
                .fetch_optional(self.as_mut())
                .await?
                .map(parse_header::<Types>)
                .transpose()?
        } else {
            // If we have been limited, return a `null` next block indicating an incomplete window.
            // The client will have to query again with an adjusted starting point to get subsequent
            // results.
            tracing::debug!(limit, "cutting off header window request due to limit");
            None
        };

        // If the `next` block exists, _or_ if any block in the window exists, we know we have
        // enough information to definitively say at least where the window starts (we may or may
        // not have where it ends, depending on how many blocks have thus far been produced).
        // However, if we have neither a block in the window nor a block after it, we cannot say
        // whether the next block produced will have a timestamp before or after the window start.
        // In this case, we don't know what the `prev` field of the response should be, so we return
        // an error: the caller must try again after more blocks have been produced.
        if window.is_empty() && next.is_none() {
            return Err(QueryError::NotFound);
        }

        // Find the block just before the window.
        let sql = format!(
            "SELECT {HEADER_COLUMNS}
               FROM header AS h
              WHERE h.timestamp < $1
              ORDER BY h.timestamp DESC, h.height DESC
              LIMIT 1"
        );
        let prev = query(&sql)
            .bind(start as i64)
            .fetch_optional(self.as_mut())
            .await?
            .map(parse_header::<Types>)
            .transpose()?;

        Ok(TimeWindowQueryData { window, prev, next })
    }
}

/// Get inclusive start and end bounds for a range to pull aggregate statistics.
///
/// Returns [`None`] if there are no blocks in the given range, in which case the result should be
/// the default value of the aggregate statistic.
async fn aggregate_range_bounds<Types>(
    tx: &mut Transaction<impl TransactionMode>,
    range: impl RangeBounds<usize>,
) -> QueryResult<Option<(usize, usize)>>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
{
    let from = match range.start_bound() {
        Bound::Included(from) => *from,
        Bound::Excluded(from) => *from + 1,
        Bound::Unbounded => 0,
    };
    let to = match range.end_bound() {
        Bound::Included(to) => *to,
        Bound::Excluded(0) => return Ok(None),
        Bound::Excluded(to) => *to - 1,
        Bound::Unbounded => {
            let height = AggregatesStorage::<Types>::aggregates_height(tx)
                .await
                .map_err(|err| QueryError::Error {
                    message: format!("{err:#}"),
                })?;
            if height == 0 {
                return Ok(None);
            }
            if height < from {
                return Ok(None);
            }
            height - 1
        },
    };
    Ok(Some((from, to)))
}

#[cfg(test)]
mod test {
    use hotshot_example_types::node_types::TEST_VERSIONS;
    use itertools::Itertools;

    use super::*;
    use crate::{
        availability::LeafQueryData,
        data_source::{
            Transaction as _, VersionedDataSource,
            sql::testing::TmpDb,
            storage::{SqlStorage, StorageConnectionType, UpdateAvailabilityStorage},
        },
        testing::mocks::MockTypes,
    };

    async fn test_sync_status_ranges(start: usize, end: usize, present_ranges: &[(usize, usize)]) {
        let storage = TmpDb::init().await;
        let db = SqlStorage::connect(storage.config(), StorageConnectionType::Query)
            .await
            .unwrap();

        // Generate some mock leaves to insert.
        let mut leaves: Vec<LeafQueryData<MockTypes>> = vec![
            LeafQueryData::<MockTypes>::genesis(
                &Default::default(),
                &Default::default(),
                TEST_VERSIONS.test,
            )
            .await,
        ];
        for i in 1..end {
            let mut leaf = leaves[i - 1].clone();
            leaf.leaf.block_header_mut().block_number = i as u64;
            leaves.push(leaf);
        }

        // Set up.
        {
            let mut tx = db.write().await.unwrap();

            for &(start, end) in present_ranges {
                for leaf in leaves[start..end].iter() {
                    tx.insert_leaf(leaf.clone()).await.unwrap();
                }
            }

            tx.commit().await.unwrap();
        }

        let sync_status = db
            .read()
            .await
            .unwrap()
            .sync_status_ranges("leaf2", "height", start, end)
            .await
            .unwrap();

        // Verify missing.
        let present: usize = present_ranges.iter().map(|(start, end)| end - start).sum();
        let total = end - start;
        assert_eq!(sync_status.missing, total - present);

        // Verify ranges.
        let mut ranges = sync_status.ranges.into_iter();
        let mut prev = start;
        for &(start, end) in present_ranges {
            if start != prev {
                let range = ranges.next().unwrap();
                assert_eq!(
                    range,
                    SyncStatusRange {
                        start: prev,
                        end: start,
                        status: SyncStatus::Missing,
                    }
                );
            }
            let range = ranges.next().unwrap();
            assert_eq!(
                range,
                SyncStatusRange {
                    start,
                    end,
                    status: SyncStatus::Present,
                }
            );
            prev = end;
        }

        if prev != end {
            let range = ranges.next().unwrap();
            assert_eq!(
                range,
                SyncStatusRange {
                    start: prev,
                    end,
                    status: SyncStatus::Missing,
                }
            );
        }

        assert_eq!(ranges.next(), None);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_ranges_bookends_present() {
        test_sync_status_ranges(0, 6, &[(0, 2), (4, 6)]).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_ranges_bookends_missing() {
        test_sync_status_ranges(0, 6, &[(2, 4)]).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_ranges_start_offset_bookends_present() {
        test_sync_status_ranges(1, 8, &[(2, 4), (6, 8)]).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_ranges_start_offset_bookends_missing() {
        test_sync_status_ranges(1, 8, &[(4, 6)]).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_ranges_singleton_ranges() {
        test_sync_status_ranges(0, 3, &[(0, 1), (2, 3)]).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_ranges_many_ranges_bookends_present() {
        let ranges = (0..=100).map(|i| (2 * i, 2 * i + 1)).collect_vec();
        test_sync_status_ranges(0, 201, &ranges).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_ranges_many_ranges_bookends_missing() {
        let ranges = (1..=100).map(|i| (2 * i, 2 * i + 1)).collect_vec();
        test_sync_status_ranges(0, 202, &ranges).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_ranges_many_ranges_start_offset_bookends_present() {
        let ranges = (1..=100).map(|i| (2 * i, 2 * i + 1)).collect_vec();
        test_sync_status_ranges(1, 201, &ranges).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_sync_status_ranges_many_ranges_start_offset_bookends_missing() {
        let ranges = (2..=100).map(|i| (2 * i, 2 * i + 1)).collect_vec();
        test_sync_status_ranges(1, 202, &ranges).await;
    }
}
