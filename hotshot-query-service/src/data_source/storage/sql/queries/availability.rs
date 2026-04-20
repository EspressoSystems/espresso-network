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

//! Availability storage implementation for a database query engine.

use std::ops::RangeBounds;

use async_trait::async_trait;
use futures::stream::{StreamExt, TryStreamExt};
use hotshot_types::traits::node_implementation::NodeType;
use snafu::OptionExt;
use sqlx::FromRow;

use super::{
    super::transaction::{Transaction, TransactionMode, query},
    BLOCK_COLUMNS, LEAF_COLUMNS, PAYLOAD_COLUMNS, PAYLOAD_METADATA_COLUMNS, QueryBuilder,
    VID_COMMON_COLUMNS, VID_COMMON_METADATA_COLUMNS,
};
use crate::{
    Header, MissingSnafu, Payload, QueryError, QueryResult,
    availability::{
        BlockId, BlockQueryData, LeafId, LeafQueryData, NamespaceInfo, NamespaceMap,
        PayloadQueryData, QueryableHeader, QueryablePayload, TransactionHash, VidCommonQueryData,
    },
    data_source::storage::{
        AvailabilityStorage, PayloadMetadata, VidCommonMetadata, pruning::PrunedHeightStorage,
        sql::sqlx::Row,
    },
    types::HeightIndexed,
};

#[async_trait]
impl<Mode, Types> AvailabilityStorage<Types> for Transaction<Mode>
where
    Types: NodeType,
    Mode: TransactionMode,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
{
    async fn get_leaf(&mut self, id: LeafId<Types>) -> QueryResult<LeafQueryData<Types>> {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = match id {
            LeafId::Number(n) => format!("height = {}", query.bind(n as i64)?),
            LeafId::Hash(h) => format!("hash = {}", query.bind(h.to_string())?),
        };
        let ph = query.bind(pruned_height)?;
        let row = query
            .query(&format!(
                "SELECT {LEAF_COLUMNS} FROM leaf2 WHERE {where_clause} AND height > {ph} LIMIT 1"
            ))
            .fetch_one(self.as_mut())
            .await?;
        let leaf = LeafQueryData::from_row(&row)?;
        Ok(leaf)
    }

    async fn get_block(&mut self, id: BlockId<Types>) -> QueryResult<BlockQueryData<Types>> {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        let ph = query.bind(pruned_height)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON h.height = p.height
              WHERE {where_clause} AND h.height > {ph}
              ORDER BY h.height
              LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self.as_mut()).await?;
        let block = BlockQueryData::from_row(&row)?;
        Ok(block)
    }

    async fn get_header(&mut self, id: BlockId<Types>) -> QueryResult<Header<Types>> {
        self.load_header(id).await
    }

    async fn get_payload(&mut self, id: BlockId<Types>) -> QueryResult<PayloadQueryData<Types>> {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        let ph = query.bind(pruned_height)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {PAYLOAD_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON h.height = p.height
              WHERE {where_clause} AND h.height > {ph}
              ORDER BY h.height
              LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self.as_mut()).await?;
        let payload = PayloadQueryData::from_row(&row)?;
        Ok(payload)
    }

    async fn get_payload_metadata(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<PayloadMetadata<Types>> {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        let ph = query.bind(pruned_height)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {PAYLOAD_METADATA_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON h.height = p.height
              WHERE {where_clause} AND h.height > {ph} AND p.num_transactions IS NOT NULL
              ORDER BY h.height ASC
              LIMIT 1"
        );
        let row = query
            .query(&sql)
            .fetch_optional(self.as_mut())
            .await?
            .context(MissingSnafu)?;
        let mut payload = PayloadMetadata::from_row(&row)?;
        payload.namespaces = self
            .load_namespaces::<Types>(payload.height(), payload.size)
            .await?;
        Ok(payload)
    }

    async fn get_vid_common(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<VidCommonQueryData<Types>> {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        let ph = query.bind(pruned_height)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {VID_COMMON_COLUMNS}
              FROM header AS h
              JOIN vid2 AS v ON h.height = v.height
              WHERE {where_clause} AND h.height > {ph}
              ORDER BY h.height
              LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self.as_mut()).await?;
        let common = VidCommonQueryData::from_row(&row)?;
        Ok(common)
    }

    async fn get_vid_common_metadata(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<VidCommonMetadata<Types>> {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        let ph = query.bind(pruned_height)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {VID_COMMON_METADATA_COLUMNS}
              FROM header AS h
              JOIN vid2 AS v ON h.height = v.height
              WHERE {where_clause} AND h.height > {ph}
              ORDER BY h.height ASC
              LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self.as_mut()).await?;
        let common = VidCommonMetadata::from_row(&row)?;
        Ok(common)
    }

    async fn get_leaf_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<LeafQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "height")?;
        let ph = query.bind(pruned_height)?;
        let sql = if where_clause.is_empty() {
            format!("SELECT {LEAF_COLUMNS} FROM leaf2 WHERE height > {ph} ORDER BY height ASC")
        } else {
            format!(
                "SELECT {LEAF_COLUMNS} FROM leaf2 {where_clause} AND height > {ph} ORDER BY \
                 height ASC"
            )
        };
        Ok(query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| LeafQueryData::from_row(&res?))
            .map_err(QueryError::from)
            .collect()
            .await)
    }

    async fn get_block_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<BlockQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let ph = query.bind(pruned_height)?;
        let sql = if where_clause.is_empty() {
            format!(
                "SELECT {BLOCK_COLUMNS}
                  FROM header AS h
                  JOIN payload AS p ON h.height = p.height
                  WHERE h.height > {ph}
                  ORDER BY h.height"
            )
        } else {
            format!(
                "SELECT {BLOCK_COLUMNS}
                  FROM header AS h
                  JOIN payload AS p ON h.height = p.height
                  {where_clause} AND h.height > {ph}
                  ORDER BY h.height"
            )
        };
        Ok(query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| BlockQueryData::from_row(&res?))
            .map_err(QueryError::from)
            .collect()
            .await)
    }

    async fn get_header_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<Header<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let ph = query.bind(pruned_height)?;
        let sql = if where_clause.is_empty() {
            format!(
                "SELECT data
                  FROM header AS h
                  WHERE h.height > {ph}
                  ORDER BY h.height"
            )
        } else {
            format!(
                "SELECT data
                  FROM header AS h
                  {where_clause} AND h.height > {ph}
                  ORDER BY h.height"
            )
        };
        let headers = query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| serde_json::from_value(res?.get("data")).unwrap())
            .collect()
            .await;

        Ok(headers)
    }

    async fn get_payload_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<PayloadQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let ph = query.bind(pruned_height)?;
        let sql = if where_clause.is_empty() {
            format!(
                "SELECT {PAYLOAD_COLUMNS}
                  FROM header AS h
                  JOIN payload AS p ON h.height = p.height
                  WHERE h.height > {ph}
                  ORDER BY h.height"
            )
        } else {
            format!(
                "SELECT {PAYLOAD_COLUMNS}
                  FROM header AS h
                  JOIN payload AS p ON h.height = p.height
                  {where_clause} AND h.height > {ph}
                  ORDER BY h.height"
            )
        };
        Ok(query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| PayloadQueryData::from_row(&res?))
            .map_err(QueryError::from)
            .collect()
            .await)
    }

    async fn get_payload_metadata_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<PayloadMetadata<Types>>>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let ph = query.bind(pruned_height)?;
        let sql = if where_clause.is_empty() {
            format!(
                "SELECT {PAYLOAD_METADATA_COLUMNS}
                  FROM header AS h
                  JOIN payload AS p ON h.height = p.height
                  WHERE h.height > {ph} AND p.num_transactions IS NOT NULL
                  ORDER BY h.height ASC"
            )
        } else {
            format!(
                "SELECT {PAYLOAD_METADATA_COLUMNS}
                  FROM header AS h
                  JOIN payload AS p ON h.height = p.height
                  {where_clause} AND h.height > {ph} AND p.num_transactions IS NOT NULL
                  ORDER BY h.height ASC"
            )
        };
        let rows = query
            .query(&sql)
            .fetch(self.as_mut())
            .collect::<Vec<_>>()
            .await;
        let mut payloads = vec![];
        for row in rows {
            let res = async {
                let mut meta = PayloadMetadata::from_row(&row?)?;
                meta.namespaces = self
                    .load_namespaces::<Types>(meta.height(), meta.size)
                    .await?;
                Ok(meta)
            }
            .await;
            payloads.push(res);
        }
        Ok(payloads)
    }

    async fn get_vid_common_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<VidCommonQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let ph = query.bind(pruned_height)?;
        let sql = if where_clause.is_empty() {
            format!(
                "SELECT {VID_COMMON_COLUMNS}
                  FROM header AS h
                  JOIN vid2 AS v ON h.height = v.height
                  WHERE h.height > {ph}
                  ORDER BY h.height"
            )
        } else {
            format!(
                "SELECT {VID_COMMON_COLUMNS}
                  FROM header AS h
                  JOIN vid2 AS v ON h.height = v.height
                  {where_clause} AND h.height > {ph}
                  ORDER BY h.height"
            )
        };
        Ok(query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| VidCommonQueryData::from_row(&res?))
            .map_err(QueryError::from)
            .collect()
            .await)
    }

    async fn get_vid_common_metadata_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<VidCommonMetadata<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let ph = query.bind(pruned_height)?;
        let sql = if where_clause.is_empty() {
            format!(
                "SELECT {VID_COMMON_METADATA_COLUMNS}
                  FROM header AS h
                  JOIN vid2 AS v ON h.height = v.height
                  WHERE h.height > {ph}
                  ORDER BY h.height ASC"
            )
        } else {
            format!(
                "SELECT {VID_COMMON_METADATA_COLUMNS}
                  FROM header AS h
                  JOIN vid2 AS v ON h.height = v.height
                  {where_clause} AND h.height > {ph}
                  ORDER BY h.height ASC"
            )
        };
        Ok(query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| VidCommonMetadata::from_row(&res?))
            .map_err(QueryError::from)
            .collect()
            .await)
    }

    async fn get_block_with_transaction(
        &mut self,
        hash: TransactionHash<Types>,
    ) -> QueryResult<BlockQueryData<Types>> {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let mut query = QueryBuilder::default();
        let hash_param = query.bind(hash.to_string())?;
        let ph = query.bind(pruned_height)?;

        // ORDER BY ASC ensures that if there are duplicate transactions, we return the first
        // one.
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
                JOIN transactions AS t ON t.block_height = h.height
                WHERE t.hash = {hash_param} AND h.height > {ph}
                ORDER BY t.block_height, t.ns_id, t.position
                LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self.as_mut()).await?;
        Ok(BlockQueryData::from_row(&row)?)
    }

    async fn first_available_leaf(&mut self, from: u64) -> QueryResult<LeafQueryData<Types>> {
        let pruned_height: i64 = self
            .load_pruned_height()
            .await
            .map_err(|err| QueryError::Error {
                message: format!("{err:#}"),
            })?
            .map(|h| h as i64)
            .unwrap_or(-1);
        let row = query(&format!(
            "SELECT {LEAF_COLUMNS} FROM leaf2 WHERE height >= $1 AND height > $2 ORDER BY height \
             ASC LIMIT 1"
        ))
        .bind(from as i64)
        .bind(pruned_height)
        .fetch_one(self.as_mut())
        .await?;
        let leaf = LeafQueryData::from_row(&row)?;
        Ok(leaf)
    }
}

impl<Mode> Transaction<Mode>
where
    Mode: TransactionMode,
{
    async fn load_namespaces<Types>(
        &mut self,
        height: u64,
        payload_size: u64,
    ) -> QueryResult<NamespaceMap<Types>>
    where
        Types: NodeType,
        Header<Types>: QueryableHeader<Types>,
        Payload<Types>: QueryablePayload<Types>,
    {
        let header = self
            .get_header(BlockId::<Types>::from(height as usize))
            .await?;
        let map = query(
            "SELECT ns_id, ns_index, max(position) + 1 AS count
               FROM  transactions
               WHERE block_height = $1
               GROUP BY ns_id, ns_index",
        )
        .bind(height as i64)
        .fetch(self.as_mut())
        .map_ok(|row| {
            let ns = row.get::<i64, _>("ns_index").into();
            let id = row.get::<i64, _>("ns_id").into();
            let num_transactions = row.get::<i64, _>("count") as u64;
            let size = header.namespace_size(&ns, payload_size as usize);
            (
                id,
                NamespaceInfo {
                    num_transactions,
                    size,
                },
            )
        })
        .try_collect()
        .await?;
        Ok(map)
    }
}

#[cfg(test)]
mod test {
    use hotshot_example_types::node_types::TEST_VERSIONS;
    use hotshot_types::{data::VidCommon, vid::advz::advz_scheme};
    use jf_advz::VidScheme;
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{
        data_source::{
            Transaction, VersionedDataSource,
            sql::testing::TmpDb,
            storage::{SqlStorage, StorageConnectionType, UpdateAvailabilityStorage},
        },
        testing::mocks::MockTypes,
    };

    #[tokio::test]
    #[test_log::test]
    async fn test_duplicate_payload() {
        let storage = TmpDb::init().await;
        let db = SqlStorage::connect(storage.config(), StorageConnectionType::Query)
            .await
            .unwrap();
        let mut vid = advz_scheme(2);

        // Create two blocks with the same (empty) payload.
        let mut leaves = vec![
            LeafQueryData::<MockTypes>::genesis(
                &Default::default(),
                &Default::default(),
                TEST_VERSIONS.test,
            )
            .await,
        ];
        let mut blocks = vec![
            BlockQueryData::<MockTypes>::genesis(
                &Default::default(),
                &Default::default(),
                TEST_VERSIONS.test.base,
            )
            .await,
        ];
        let dispersal = vid.disperse([]).unwrap();
        let mut vid = vec![VidCommonQueryData::<MockTypes>::new(
            leaves[0].header().clone(),
            VidCommon::V0(dispersal.common.clone()),
        )];

        let mut leaf = leaves[0].clone();
        leaf.leaf.block_header_mut().block_number += 1;
        let block = BlockQueryData::new(leaf.header().clone(), blocks[0].payload().clone());
        let common =
            VidCommonQueryData::new(leaf.header().clone(), VidCommon::V0(dispersal.common));
        leaves.push(leaf);
        blocks.push(block);
        vid.push(common);

        // Insert the first leaf without payload or VID data.
        {
            let mut tx = db.write().await.unwrap();
            tx.insert_leaf(&leaves[0]).await.unwrap();
            tx.commit().await.unwrap();
        }

        // The block and VID data are missing.
        {
            let mut tx = db.read().await.unwrap();
            assert_eq!(tx.get_leaf(LeafId::Number(0)).await.unwrap(), leaves[0]);
            assert_absent(
                tx.get_block(BlockId::<MockTypes>::Number(0))
                    .await
                    .unwrap_err(),
            );
            assert_absent(
                tx.get_vid_common(BlockId::<MockTypes>::Number(0))
                    .await
                    .unwrap_err(),
            );
        }

        // Insert the second block with all data.
        {
            let mut tx = db.write().await.unwrap();
            tx.insert_leaf(&leaves[1]).await.unwrap();
            tx.insert_block(&blocks[1]).await.unwrap();
            tx.insert_vid(&vid[1], None).await.unwrap();
            tx.commit().await.unwrap();
        }

        // The identical block and VID common are shared by both leaves.
        for i in 0..2 {
            let mut tx = db.read().await.unwrap();
            assert_eq!(tx.get_leaf(LeafId::Number(i)).await.unwrap(), leaves[i]);
            assert_eq!(tx.get_block(BlockId::Number(i)).await.unwrap(), blocks[i]);
            assert_eq!(tx.get_vid_common(BlockId::Number(i)).await.unwrap(), vid[i]);
        }
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_same_payload_different_ns_table() {
        let storage = TmpDb::init().await;
        let db = SqlStorage::connect(storage.config(), StorageConnectionType::Query)
            .await
            .unwrap();
        let mut vid = advz_scheme(2);

        // Create two blocks with byte-identical payloads, but different namespace tables (meaning
        // the interpretation of the payload is different).
        // Create two blocks with the same (empty) payload.
        let mut leaves = vec![
            LeafQueryData::<MockTypes>::genesis(
                &Default::default(),
                &Default::default(),
                TEST_VERSIONS.test,
            )
            .await,
        ];
        let mut blocks = vec![
            BlockQueryData::<MockTypes>::genesis(
                &Default::default(),
                &Default::default(),
                TEST_VERSIONS.test.base,
            )
            .await,
        ];
        let dispersal = vid.disperse([]).unwrap();
        let mut vid = vec![VidCommonQueryData::<MockTypes>::new(
            leaves[0].header().clone(),
            VidCommon::V0(dispersal.common.clone()),
        )];

        let mut leaf = leaves[0].clone();
        leaf.leaf.block_header_mut().block_number += 1;
        leaf.leaf.block_header_mut().metadata.num_transactions += 1;
        let block = BlockQueryData::new(leaf.header().clone(), blocks[0].payload().clone());
        let common =
            VidCommonQueryData::new(leaf.header().clone(), VidCommon::V0(dispersal.common));
        leaves.push(leaf);
        blocks.push(block);
        vid.push(common);

        // Insert the first leaf without payload or VID data.
        {
            let mut tx = db.write().await.unwrap();
            tx.insert_leaf(&leaves[0]).await.unwrap();
            tx.commit().await.unwrap();
        }

        // The block and VID data are missing.
        {
            let mut tx = db.read().await.unwrap();
            assert_eq!(tx.get_leaf(LeafId::Number(0)).await.unwrap(), leaves[0]);
            assert_absent(
                tx.get_block(BlockId::<MockTypes>::Number(0))
                    .await
                    .unwrap_err(),
            );
            assert_absent(
                tx.get_vid_common(BlockId::<MockTypes>::Number(0))
                    .await
                    .unwrap_err(),
            );
        }

        // Insert the second block with all data.
        {
            let mut tx = db.write().await.unwrap();
            tx.insert_leaf(&leaves[1]).await.unwrap();
            tx.insert_block(&blocks[1]).await.unwrap();
            tx.insert_vid(&vid[1], None).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Both leaves and VID common are present.
        let mut tx = db.read().await.unwrap();
        for i in 0..2 {
            assert_eq!(tx.get_leaf(LeafId::Number(i)).await.unwrap(), leaves[i]);
            assert_eq!(tx.get_vid_common(BlockId::Number(i)).await.unwrap(), vid[i]);
        }

        // The first block is still missing, since the payload cannot be shared.
        assert_absent(
            tx.get_block(BlockId::<MockTypes>::Number(0))
                .await
                .unwrap_err(),
        );
        assert_eq!(tx.get_block(BlockId::Number(1)).await.unwrap(), blocks[1]);
    }

    fn assert_absent(err: QueryError) {
        assert!(
            matches!(err, QueryError::Missing | QueryError::NotFound),
            "{err:#}"
        );
    }
}
