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
use sqlx::Row;

use super::{
    super::transaction::{Transaction, TransactionMode},
    BLOCK_COLUMNS, LEAF_COLUMNS, PAYLOAD_COLUMNS, PAYLOAD_METADATA_COLUMNS, QueryBuilder,
    VID_COMMON_COLUMNS, VID_COMMON_METADATA_COLUMNS,
};
use crate::{
    Header, MissingSnafu, Payload, QueryResult,
    availability::{
        BlockId, BlockQueryData, LeafId, LeafQueryData, NamespaceInfo, NamespaceMap,
        PayloadQueryData, QueryableHeader, QueryablePayload, TransactionHash, VidCommonQueryData,
    },
    data_source::storage::{AvailabilityStorage, PayloadMetadata, VidCommonMetadata},
    types::HeightIndexed,
    with_backend,
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
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = match id {
            LeafId::Number(n) => format!("height = {}", query.bind(n as i64)?),
            LeafId::Hash(h) => format!("hash = {}", query.bind(h.to_string())?),
        };
        let sql = format!("SELECT {LEAF_COLUMNS} FROM leaf2 WHERE {where_clause} LIMIT 1");
        let row = query.query(&sql).fetch_one(self).await?;
        Ok(row.from_row()?)
    }

    async fn get_block(&mut self, id: BlockId<Types>) -> QueryResult<BlockQueryData<Types>> {
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.header_where_clause(id)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              WHERE {where_clause}
              ORDER BY h.height
              LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self).await?;
        Ok(row.from_row()?)
    }

    async fn get_header(&mut self, id: BlockId<Types>) -> QueryResult<Header<Types>> {
        self.load_header(id).await
    }

    async fn get_payload(&mut self, id: BlockId<Types>) -> QueryResult<PayloadQueryData<Types>> {
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.header_where_clause(id)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {PAYLOAD_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              WHERE {where_clause}
              ORDER BY h.height
              LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self).await?;
        Ok(row.from_row()?)
    }

    async fn get_payload_metadata(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<PayloadMetadata<Types>> {
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.header_where_clause(id)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {PAYLOAD_METADATA_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              WHERE {where_clause}
              ORDER BY h.height ASC
              LIMIT 1"
        );
        let row = query
            .query(&sql)
            .fetch_optional(self)
            .await?
            .context(MissingSnafu)?;
        let mut payload: PayloadMetadata<Types> = row.from_row()?;
        payload.namespaces = self
            .load_namespaces::<Types>(payload.height(), payload.size)
            .await?;
        Ok(payload)
    }

    async fn get_vid_common(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<VidCommonQueryData<Types>> {
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.header_where_clause(id)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {VID_COMMON_COLUMNS}
              FROM header AS h
              JOIN vid_common AS v ON h.payload_hash = v.hash
              WHERE {where_clause}
              ORDER BY h.height
              LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self).await?;
        Ok(row.from_row()?)
    }

    async fn get_vid_common_metadata(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<VidCommonMetadata<Types>> {
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.header_where_clause(id)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {VID_COMMON_METADATA_COLUMNS}
              FROM header AS h
              JOIN vid_common AS v ON h.payload_hash = v.hash
              WHERE {where_clause}
              ORDER BY h.height ASC
              LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self).await?;
        Ok(row.from_row()?)
    }

    async fn get_leaf_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<LeafQueryData<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.bounds_to_where_clause(range, "height")?;
        let sql = format!("SELECT {LEAF_COLUMNS} FROM leaf2 {where_clause} ORDER BY height ASC");
        Ok(query
            .query(&sql)
            .fetch(self)
            .map(|res| Ok(res?.from_row()?))
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
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              {where_clause}
              ORDER BY h.height"
        );
        Ok(query
            .query(&sql)
            .fetch(self)
            .map(|res| Ok(res?.from_row()?))
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
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;

        let headers = query
            .query(&format!(
                "SELECT data
                  FROM header AS h
                  {where_clause}
                  ORDER BY h.height"
            ))
            .fetch(self)
            .map(|res| {
                let row = res?;
                let data: serde_json::Value = row.try_get("data")?;
                Ok(serde_json::from_value(data).unwrap())
            })
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
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let sql = format!(
            "SELECT {PAYLOAD_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              {where_clause}
              ORDER BY h.height"
        );
        Ok(query
            .query(&sql)
            .fetch(self)
            .map(|res| Ok(res?.from_row()?))
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
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let sql = format!(
            "SELECT {PAYLOAD_METADATA_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              {where_clause}
              ORDER BY h.height ASC"
        );
        let rows: Vec<_> = query.query(&sql).fetch(self).collect::<Vec<_>>().await;
        let mut payloads = vec![];
        for row in rows {
            let res = async {
                let mut meta: PayloadMetadata<Types> = row?.from_row()?;
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
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let sql = format!(
            "SELECT {VID_COMMON_COLUMNS}
              FROM header AS h
              JOIN vid_common AS v ON h.payload_hash = v.hash
              {where_clause}
              ORDER BY h.height"
        );
        Ok(query
            .query(&sql)
            .fetch(self)
            .map(|res| Ok(res?.from_row()?))
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
        let mut query = QueryBuilder::new(self.backend());
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let sql = format!(
            "SELECT {VID_COMMON_METADATA_COLUMNS}
              FROM header AS h
              JOIN vid_common AS v ON h.payload_hash = v.hash
              {where_clause}
              ORDER BY h.height ASC"
        );
        Ok(query
            .query(&sql)
            .fetch(self)
            .map(|res| Ok(res?.from_row()?))
            .collect()
            .await)
    }

    async fn get_block_with_transaction(
        &mut self,
        hash: TransactionHash<Types>,
    ) -> QueryResult<BlockQueryData<Types>> {
        let mut query = QueryBuilder::new(self.backend());
        let hash_param = query.bind(hash.to_string())?;

        // ORDER BY ASC ensures that if there are duplicate transactions, we return the first
        // one.
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
                JOIN transactions AS t ON t.block_height = h.height
                WHERE t.hash = {hash_param}
                ORDER BY t.block_height, t.ns_id, t.position
                LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self).await?;
        Ok(row.from_row()?)
    }

    async fn first_available_leaf(&mut self, from: u64) -> QueryResult<LeafQueryData<Types>> {
        let mut query = QueryBuilder::new(self.backend());
        let param = query.bind(from as i64)?;
        let sql = format!(
            "SELECT {LEAF_COLUMNS} FROM leaf2 WHERE height >= {param} ORDER BY height ASC LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self).await?;
        Ok(row.from_row()?)
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
        let map = with_backend!(self, |tx| {
            sqlx::query(
                "SELECT ns_id, ns_index, max(position) + 1 AS count
                   FROM  transactions
                   WHERE block_height = $1
                   GROUP BY ns_id, ns_index",
            )
            .bind(height as i64)
            .fetch(tx.as_mut())
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
            .await
        })?;
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
        QueryError,
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
