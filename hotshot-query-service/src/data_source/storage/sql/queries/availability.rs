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
    super::transaction::{query, Transaction, TransactionMode},
    QueryBuilder, BLOCK_COLUMNS, LEAF_COLUMNS, PAYLOAD_COLUMNS, PAYLOAD_METADATA_COLUMNS,
    STATE_CERT_COLUMNS, VID_COMMON_COLUMNS, VID_COMMON_METADATA_COLUMNS,
};
use crate::{
    availability::{
        BlockId, BlockQueryData, LeafId, LeafQueryData, PayloadQueryData, QueryableHeader,
        QueryablePayload, StateCertQueryData, TransactionHash, TransactionQueryData,
        VidCommonQueryData,
    },
    data_source::storage::{
        sql::sqlx::Row, AvailabilityStorage, PayloadMetadata, VidCommonMetadata,
    },
    types::HeightIndexed,
    ErrorSnafu, Header, MissingSnafu, Payload, QueryError, QueryResult,
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
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON h.height = p.height
              WHERE {where_clause}
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
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {PAYLOAD_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON h.height = p.height
              WHERE {where_clause}
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
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {PAYLOAD_METADATA_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON h.height = p.height
              WHERE {where_clause} AND p.num_transactions IS NOT NULL
              ORDER BY h.height ASC
              LIMIT 1"
        );
        let row = query
            .query(&sql)
            .fetch_optional(self.as_mut())
            .await?
            .context(MissingSnafu)?;
        let payload = PayloadMetadata::from_row(&row)?;
        Ok(payload)
    }

    async fn get_vid_common(
        &mut self,
        id: BlockId<Types>,
    ) -> QueryResult<VidCommonQueryData<Types>> {
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {VID_COMMON_COLUMNS}
              FROM header AS h
              JOIN vid2 AS v ON h.height = v.height
              WHERE {where_clause}
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
        let mut query = QueryBuilder::default();
        let where_clause = query.header_where_clause(id)?;
        // ORDER BY h.height ASC ensures that if there are duplicate blocks (this can happen when
        // selecting by payload ID, as payloads are not unique), we return the first one.
        let sql = format!(
            "SELECT {VID_COMMON_METADATA_COLUMNS}
              FROM header AS h
              JOIN vid2 AS v ON h.height = v.height
              WHERE {where_clause}
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
              JOIN payload AS p ON h.height = p.height
              {where_clause}
              ORDER BY h.height"
        );
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
              JOIN payload AS p ON h.height = p.height
              {where_clause}
              ORDER BY h.height"
        );
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
        let mut query = QueryBuilder::default();
        let where_clause = query.bounds_to_where_clause(range, "h.height")?;
        let sql = format!(
            "SELECT {PAYLOAD_METADATA_COLUMNS}
              FROM header AS h
              JOIN payload AS p ON h.height = p.height
              {where_clause} AND p.num_transactions IS NOT NULL
              ORDER BY h.height ASC"
        );
        Ok(query
            .query(&sql)
            .fetch(self.as_mut())
            .map(|res| PayloadMetadata::from_row(&res?))
            .map_err(QueryError::from)
            .collect()
            .await)
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
              JOIN vid2 AS v ON h.height = v.height
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
              JOIN vid2 AS v ON h.height = v.height
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

    async fn get_transaction(
        &mut self,
        hash: TransactionHash<Types>,
    ) -> QueryResult<TransactionQueryData<Types>> {
        let mut query = QueryBuilder::default();
        let hash_param = query.bind(hash.to_string())?;

        // ORDER BY ASC ensures that if there are duplicate transactions, we return the first
        // one.
        let sql = format!(
            "SELECT {BLOCK_COLUMNS}, t.idx AS tx_index
                FROM header AS h
                JOIN payload AS p ON h.height = p.height
                JOIN transactions AS t ON t.block_height = h.height
                WHERE t.hash = {hash_param}
                ORDER BY t.block_height, t.idx
                LIMIT 1"
        );
        let row = query.query(&sql).fetch_one(self.as_mut()).await?;

        // Extract the block.
        let block = BlockQueryData::from_row(&row)?;

        TransactionQueryData::with_hash(&block, hash).context(ErrorSnafu {
            message: format!(
                "transaction index inconsistent: block {} contains no transaction {hash}",
                block.height()
            ),
        })
    }

    async fn first_available_leaf(&mut self, from: u64) -> QueryResult<LeafQueryData<Types>> {
        let row = query(&format!(
            "SELECT {LEAF_COLUMNS} FROM leaf2 WHERE height >= $1 ORDER BY height ASC LIMIT 1"
        ))
        .bind(from as i64)
        .fetch_one(self.as_mut())
        .await?;
        let leaf = LeafQueryData::from_row(&row)?;
        Ok(leaf)
    }

    async fn get_state_cert(&mut self, epoch: u64) -> QueryResult<StateCertQueryData<Types>> {
        let row = query(&format!(
            "SELECT {STATE_CERT_COLUMNS} FROM finalized_state_cert WHERE epoch = $1 LIMIT 1"
        ))
        .bind(epoch as i64)
        .fetch_one(self.as_mut())
        .await?;
        Ok(StateCertQueryData::from_row(&row)?)
    }
}
