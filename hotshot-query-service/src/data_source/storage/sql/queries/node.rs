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

use std::{
    collections::HashMap,
    ops::{Bound, RangeBounds},
};

use anyhow::anyhow;
use async_trait::async_trait;
use futures::stream::{StreamExt, TryStreamExt};
use hotshot_types::{
    data::VidShare,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
};
use snafu::OptionExt;
use sqlx::Row;

use super::{
    super::transaction::{query, query_as, Transaction, TransactionMode, Write},
    parse_header, DecodeError, QueryBuilder, HEADER_COLUMNS,
};
use crate::{
    data_source::storage::{
        Aggregate, AggregatesStorage, NodeStorage, PayloadMetadata, UpdateAggregatesStorage,
    },
    node::{BlockId, SyncStatus, TimeWindowQueryData, WindowStart},
    types::HeightIndexed,
    Header, MissingSnafu, NotFoundSnafu, QueryError, QueryResult,
};

#[async_trait]
impl<Mode, Types> NodeStorage<Types> for Transaction<Mode>
where
    Mode: TransactionMode,
    Types: NodeType,
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
        namespace: Option<u32>,
    ) -> QueryResult<usize> {
        let namespace = namespace.map(|ns| ns as i32).unwrap_or(-1);
        let Some((from, to)) = aggregate_range_bounds(self, range).await? else {
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
        namespace: Option<u32>,
    ) -> QueryResult<usize> {
        let namespace = namespace.map(|ns| ns as i32).unwrap_or(-1);
        let Some((from, to)) = aggregate_range_bounds(self, range).await? else {
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

    async fn sync_status(&mut self) -> QueryResult<SyncStatus> {
        // A leaf can only be missing if there is no row for it in the database (all its columns are
        // non-nullable). A block can be missing if its corresponding leaf is missing or if the
        // block's `data` field is `NULL`. We can find the number of missing leaves and blocks by
        // getting the number of fully missing leaf rows and the number of present but null-payload
        // block rows.
        //
        // Note that it should not be possible for a block's row to be missing (as opposed to
        // present but having a `NULL` payload) if the corresponding leaf is present. The schema
        // enforces this, since the payload table `REFERENCES` the corresponding leaf row. Thus,
        // missing block rows should be a subset of missing leaf rows and do not need to be counted
        // separately. This is very important: if we had to count the number of block rows that were
        // missing whose corresponding leaf row was present, this would require an expensive join
        // and table traversal.
        //
        // We can get the number of missing leaf rows very efficiently, by subtracting the total
        // number of leaf rows from the block height (since the block height by definition is the
        // height of the highest leaf we do have). We can also get the number of null payloads
        // directly using an `IS NULL` filter.
        //
        // For VID, common data can only be missing if the entire row is missing. Shares can be
        // missing in that case _or_ if the row is present but share data is NULL. Thus, we also
        // need to select the total number of VID rows and the number of present VID rows with a
        // NULL share.
        let sql = "SELECT l.max_height, l.total_leaves, p.null_payloads, v.total_vid, vn.null_vid, pruned_height FROM
                (SELECT max(leaf2.height) AS max_height, count(*) AS total_leaves FROM leaf2) AS l,
                (SELECT count(*) AS null_payloads FROM payload WHERE data IS NULL) AS p,
                (SELECT count(*) AS total_vid FROM vid2) AS v,
                (SELECT count(*) AS null_vid FROM vid2 WHERE share IS NULL) AS vn,
                (SELECT(SELECT last_height FROM pruned_height ORDER BY id DESC LIMIT 1) as pruned_height)
            ";
        let row = query(sql)
            .fetch_optional(self.as_mut())
            .await?
            .context(NotFoundSnafu)?;

        let block_height = match row.get::<Option<i64>, _>("max_height") {
            Some(height) => {
                // The height of the block is the number of blocks below it, so the total number of
                // blocks is one more than the height of the highest block.
                height as usize + 1
            },
            None => {
                // If there are no blocks yet, the height is 0.
                0
            },
        };
        let total_leaves = row.get::<i64, _>("total_leaves") as usize;
        let null_payloads = row.get::<i64, _>("null_payloads") as usize;
        let total_vid = row.get::<i64, _>("total_vid") as usize;
        let null_vid = row.get::<i64, _>("null_vid") as usize;
        let pruned_height = row
            .get::<Option<i64>, _>("pruned_height")
            .map(|h| h as usize);

        let missing_leaves = block_height.saturating_sub(total_leaves);
        let missing_blocks = missing_leaves + null_payloads;
        let missing_vid_common = block_height.saturating_sub(total_vid);
        let missing_vid_shares = missing_vid_common + null_vid;

        Ok(SyncStatus {
            missing_leaves,
            missing_blocks,
            missing_vid_common,
            missing_vid_shares,
            pruned_height,
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
}

impl<Mode: TransactionMode> AggregatesStorage for Transaction<Mode> {
    async fn aggregates_height(&mut self) -> anyhow::Result<usize> {
        let (height,): (i64,) = query_as("SELECT coalesce(max(height) + 1, 0) FROM aggregate")
            .fetch_one(self.as_mut())
            .await?;
        Ok(height as usize)
    }

    async fn load_prev_aggregate(&mut self) -> anyhow::Result<Option<Aggregate>> {
        // Get the maximum height for which we have stored aggregated results
        // then query all the namespace info for that height
        let res: Option<(i64,)> = query_as(
            "SELECT height FROM aggregate WHERE namespace = -1 ORDER BY height DESC LIMIT 1",
        )
        .fetch_optional(self.as_mut())
        .await?;

        let Some((max_height,)) = res else {
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

        let mut num_transactions = HashMap::new();
        let mut payload_size = HashMap::new();

        for (namespace_id, num_tx, payload_sz) in rows {
            // Null namespace is represented as - 1 in database
            // as it is part of primary key and primary key can not be NULL
            // This namespace represents the cumulative sum of all the namespaces
            let key = if namespace_id == -1 {
                None
            } else {
                Some(namespace_id as u32)
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

impl<Types: NodeType> UpdateAggregatesStorage<Types> for Transaction<Write> {
    async fn update_aggregates(
        &mut self,
        prev: Aggregate,
        blocks: &[PayloadMetadata<Types>],
    ) -> anyhow::Result<Aggregate> {
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

                    *tx_count.entry(None).or_insert(0) += block.num_transactions as usize;
                    *size.entry(None).or_insert(0) += block.size as usize;

                    // None represents the total of all the namespaces
                    // and is represented as -1 in databasr
                    rows.push((
                        block.height as i64,
                        -1,
                        tx_count[&None] as i64,
                        size[&None] as i64,
                    ));

                    for (&ns_id, info) in &block.namespaces {
                        let key = Some(ns_id);

                        *tx_count.entry(key).or_insert(0) += info.num_transactions as usize;
                        *size.entry(key).or_insert(0) += info.size as usize;

                        rows.push((
                            block.height as i64,
                            ns_id as i64,
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
async fn aggregate_range_bounds(
    tx: &mut Transaction<impl TransactionMode>,
    range: impl RangeBounds<usize>,
) -> QueryResult<Option<(usize, usize)>> {
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
            let height = tx
                .aggregates_height()
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
