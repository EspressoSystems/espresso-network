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

use std::{
    fmt::Display,
    ops::{Range, RangeBounds},
};

use async_trait::async_trait;
use futures::stream::{self, StreamExt, TryStreamExt};
use hotshot_types::traits::{block_contents::BlockHeader, node_implementation::NodeType};
use snafu::OptionExt;
use sqlx::FromRow;

use super::{
    super::transaction::{Transaction, TransactionMode, query, query_as},
    BLOCK_COLUMNS, DecodeError, LEAF_COLUMNS, PAYLOAD_COLUMNS, PAYLOAD_METADATA_COLUMNS,
    QueryBuilder, VID_COMMON_COLUMNS, VID_COMMON_METADATA_COLUMNS,
};
use crate::{
    Header, MissingSnafu, Payload, QueryError, QueryResult,
    availability::{
        BlockId, BlockQueryData, Certificate2, LeafId, LeafQueryData, NamespaceInfo, NamespaceMap,
        PayloadQueryData, QueryableHeader, QueryablePayload, TransactionHash, VidCommonQueryData,
        sql::{BlockRow, PayloadSource},
    },
    data_source::storage::{
        AvailabilityStorage, PayloadMetadata, VidCommonMetadata, blob::BlobLoc, sql::sqlx::Row,
    },
    types::HeightIndexed,
};

/// Payload reads in flight per block range.
const LOAD_CONCURRENCY: usize = 4;

#[async_trait]
impl<Mode, Types> AvailabilityStorage<Types> for Transaction<Mode>
where
    Types: NodeType,
    Mode: TransactionMode,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: QueryableHeader<Types>,
{
    async fn get_leaf(&mut self, id: LeafId<Types>) -> QueryResult<LeafQueryData<Types>> {
        let mut query = QueryBuilder::default();
        let where_clause = match id {
            LeafId::Number(n) => format!("height = {}", query.bind(n as i64)?),
            LeafId::Hash(h) => format!("hash = {}", query.bind(h.to_string())?),
        };
        let row = query
            .query(&format!(
                "SELECT {LEAF_COLUMNS} FROM leaf2 WHERE {where_clause} LIMIT 1"
            ))
            .fetch_one(self.as_mut())
            .await?;
        let leaf = LeafQueryData::from_row(&row)?;
        Ok(leaf)
    }

    async fn get_block(&mut self, id: BlockId<Types>) -> QueryResult<BlockQueryData<Types>> {
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              LEFT JOIN payload_loc AS pl ON pl.height = h.height
              WHERE {where_clause}
              LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self.as_mut()).await?;
        self.load_block(BlockRow::<Types>::from_row(&row)?).await
    }

    async fn get_header(&mut self, id: BlockId<Types>) -> QueryResult<Header<Types>> {
        self.load_header(id).await
    }

    async fn get_payload(&mut self, id: BlockId<Types>) -> QueryResult<PayloadQueryData<Types>> {
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        let sql = format!(
            "SELECT {PAYLOAD_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              LEFT JOIN payload_loc AS pl ON pl.height = h.height
              WHERE {where_clause}
              LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self.as_mut()).await?;
        Ok(self
            .load_block(BlockRow::<Types>::from_row(&row)?)
            .await?
            .into())
    }

    async fn get_payload_metadata(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<PayloadMetadata<Types>> {
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        let sql = format!(
            "SELECT {PAYLOAD_METADATA_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              WHERE {where_clause}
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
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        let sql = format!(
            "SELECT {VID_COMMON_COLUMNS}
              FROM header AS h
              JOIN vid_common AS v ON h.payload_hash = v.hash
              WHERE {where_clause}
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
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        let sql = format!(
            "SELECT {VID_COMMON_METADATA_COLUMNS}
              FROM header AS h
              JOIN vid_common AS v ON h.payload_hash = v.hash
              WHERE {where_clause}
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
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "height")?;
        let sql = format!("SELECT {LEAF_COLUMNS} FROM leaf2 {where_clause} ORDER BY height ASC");
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
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              LEFT JOIN payload_loc AS pl ON pl.height = h.height
              {where_clause}
              ORDER BY h.height"
        );
        let rows = query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| Ok::<_, QueryError>(BlockRow::<Types>::from_row(&res?)?))
            .collect::<Vec<_>>()
            .await;
        Ok(self.load_blocks(rows).await)
    }

    async fn get_header_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<Header<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;

        let headers = query
            .query(&format!(
                "SELECT data
                  FROM header AS h
                  {where_clause}
                  ORDER BY h.height"
            ))
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
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let sql = format!(
            "SELECT {PAYLOAD_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              LEFT JOIN payload_loc AS pl ON pl.height = h.height
              {where_clause}
              ORDER BY h.height"
        );
        let rows = query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| Ok::<_, QueryError>(BlockRow::<Types>::from_row(&res?)?))
            .collect::<Vec<_>>()
            .await;
        Ok(self
            .load_blocks(rows)
            .await
            .into_iter()
            .map(|block| block.map(PayloadQueryData::from))
            .collect())
    }

    async fn get_payload_metadata_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<PayloadMetadata<Types>>>>
    where
        R: RangeBounds<usize> + Send + 'static,
    {
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let sql = format!(
            "SELECT {PAYLOAD_METADATA_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              {where_clause}
              ORDER BY h.height ASC"
        );
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
        let mut query = QueryBuilder::default();
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
            .fetch(self.as_mut())
            .map(|res| VidCommonQueryData::from_row(&res?))
            .map_err(QueryError::from)
            .collect()
            .await)
    }

    async fn get_leaf_ranges(
        &mut self,
        ranges: &[Range<u64>],
    ) -> QueryResult<Vec<LeafQueryData<Types>>> {
        if ranges.is_empty() {
            return Ok(vec![]);
        }

        let mut query = QueryBuilder::default();
        let where_clause = query.ranges_to_where_clause(ranges, "height")?;
        let sql = format!("SELECT {LEAF_COLUMNS} FROM leaf2 {where_clause} ORDER BY height ASC");
        query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| LeafQueryData::from_row(&res?))
            .map_err(QueryError::from)
            .try_collect()
            .await
    }

    async fn get_block_ranges(
        &mut self,
        ranges: &[Range<u64>],
    ) -> QueryResult<Vec<BlockQueryData<Types>>> {
        if ranges.is_empty() {
            return Ok(vec![]);
        }

        let mut query = QueryBuilder::default();
        let where_clause = query.ranges_to_where_clause(ranges, "h.height")?;
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
              LEFT JOIN payload_loc AS pl ON pl.height = h.height
              {where_clause}
              ORDER BY h.height"
        );
        let rows = query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| Ok::<_, QueryError>(BlockRow::<Types>::from_row(&res?)?))
            .collect::<Vec<_>>()
            .await;
        self.load_blocks(rows).await.into_iter().collect()
    }

    async fn get_vid_common_ranges(
        &mut self,
        ranges: &[Range<u64>],
    ) -> QueryResult<Vec<VidCommonQueryData<Types>>> {
        if ranges.is_empty() {
            return Ok(vec![]);
        }

        let mut query = QueryBuilder::default();
        let where_clause = query.ranges_to_where_clause(ranges, "h.height")?;
        let sql = format!(
            "SELECT {VID_COMMON_COLUMNS}
              FROM header AS h
              JOIN vid_common AS v ON h.payload_hash = v.hash
              {where_clause}
              ORDER BY h.height"
        );
        query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| VidCommonQueryData::from_row(&res?))
            .map_err(QueryError::from)
            .try_collect()
            .await
    }

    async fn get_vid_common_metadata_range<R>(
        &mut self,
        range: R,
    ) -> QueryResult<Vec<QueryResult<VidCommonMetadata<Types>>>>
    where
        R: RangeBounds<usize> + Send,
    {
        let mut query = QueryBuilder::default();
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
        let mut query = QueryBuilder::default();
        let hash_param = query.bind(hash.to_string())?;

        // ORDER BY ASC ensures that if there are duplicate transactions, we return the first
        // one.
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}
                FROM header AS h
                JOIN payload AS p ON (h.payload_hash, h.ns_table) = (p.hash, p.ns_table)
                LEFT JOIN payload_loc AS pl ON pl.height = h.height
                JOIN transactions AS t ON t.block_height = h.height
                WHERE t.hash = {hash_param}
                ORDER BY t.block_height, t.ns_id, t.position
                LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self.as_mut()).await?;
        self.load_block(BlockRow::<Types>::from_row(&row)?).await
    }

    async fn load_cert2(&mut self, height: u64) -> QueryResult<Option<Certificate2<Types>>> {
        let Some((json,)) = query_as("SELECT data FROM cert2 WHERE height = $1")
            .bind(height as i64)
            .fetch_optional(self.as_mut())
            .await?
        else {
            return Ok(None);
        };
        let cert2 = serde_json::from_value(json).decode_error("malformed cert2")?;
        Ok(cert2)
    }
}

impl<Mode> Transaction<Mode>
where
    Mode: TransactionMode,
{
    /// Resolve the payload of a decoded block row and build the block.
    pub async fn load_block<Types>(
        &self,
        mut row: BlockRow<Types>,
    ) -> QueryResult<BlockQueryData<Types>>
    where
        Types: NodeType,
        Header<Types>: QueryableHeader<Types>,
        Payload<Types>: QueryablePayload<Types>,
    {
        let source = std::mem::take(&mut row.payload);
        let bytes = self
            .payload_bytes(row.header.block_number(), source)
            .await?;
        Ok(row.into_block(&bytes))
    }

    /// Load rows in parallel, preserving order.
    pub(crate) async fn load_blocks<Types>(
        &self,
        rows: Vec<QueryResult<BlockRow<Types>>>,
    ) -> Vec<QueryResult<BlockQueryData<Types>>>
    where
        Types: NodeType,
        Header<Types>: QueryableHeader<Types>,
        Payload<Types>: QueryablePayload<Types>,
    {
        stream::iter(rows)
            .map(|row| async move { self.load_block(row?).await })
            .buffered(LOAD_CONCURRENCY)
            .collect()
            .await
    }

    async fn payload_bytes(&self, height: u64, source: PayloadSource) -> QueryResult<Vec<u8>> {
        match source {
            PayloadSource::Inline(bytes) => Ok(bytes),
            PayloadSource::Empty => Ok(vec![]),
            PayloadSource::Blob(loc) => {
                let blobs = self
                    .blob_store()
                    .ok_or_else(|| query_error("payload locator present but no blob store"))?;
                let loc = loc.parse::<BlobLoc>().map_err(query_error)?;
                let bytes = blobs.read_payload(loc).await.map_err(query_error)?;
                bytes.ok_or_else(|| {
                    tracing::warn!(height, %loc, "payload record unreadable");
                    QueryError::Missing
                })
            },
            PayloadSource::Missing => Err(QueryError::Missing),
        }
    }

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

fn query_error(err: impl Display) -> QueryError {
    QueryError::Error {
        message: err.to_string(),
    }
}

#[cfg(test)]
mod test {
    use hotshot_example_types::node_types::TEST_VERSIONS;
    use hotshot_types::{data::VidCommon, traits::EncodeBytes, vid::advz::advz_scheme};
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

    #[tokio::test]
    #[test_log::test]
    async fn test_load_block_payload_sources() {
        let storage = TmpDb::init().await;
        let db = SqlStorage::connect(storage.config(), StorageConnectionType::Query)
            .await
            .unwrap();
        let block = BlockQueryData::<MockTypes>::genesis(
            &Default::default(),
            &Default::default(),
            TEST_VERSIONS.test.base,
        )
        .await;
        let row = |payload| BlockRow {
            header: block.header().clone(),
            hash: block.hash(),
            size: block.size(),
            payload,
        };

        let tx = db.read().await.unwrap();
        let bytes = block.payload().encode().to_vec();
        assert_eq!(
            tx.load_block(row(PayloadSource::Inline(bytes)))
                .await
                .unwrap(),
            block
        );
        assert_eq!(
            tx.load_block(row(PayloadSource::Empty)).await.unwrap(),
            block
        );
        assert!(matches!(
            tx.load_block(row(PayloadSource::Missing))
                .await
                .unwrap_err(),
            QueryError::Missing
        ));
        // No blob store is configured on this connection.
        assert!(matches!(
            tx.load_block(row(PayloadSource::Blob("0-0-0-0".into())))
                .await
                .unwrap_err(),
            QueryError::Error { .. }
        ));
    }

    fn assert_absent(err: QueryError) {
        assert!(
            matches!(err, QueryError::Missing | QueryError::NotFound),
            "{err:#}"
        );
    }
}
