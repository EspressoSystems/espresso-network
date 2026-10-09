#![cfg(feature = "sqlx")]
//! Mappings between availability types and SQL types.

use std::fmt::Display;

use anyhow::Context;
use hotshot_types::traits::BlockPayload;
use serde_json::Value;
use sqlx::{ColumnIndex, prelude::*, types::Json};

use super::*;
use crate::QueryError;

/// Columns which must be selected for `LeafQueryData::from_row` to work.
pub const LEAF_COLUMNS: &str = "leaf, qc";

impl<'r, Types, R> FromRow<'r, R> for LeafQueryData<Types>
where
    Types: NodeType,
    R: Row,
    for<'a> &'a str: ColumnIndex<R>,
    for<'a> Json<Value>: Type<R::Database> + Decode<'a, R::Database>,
{
    fn from_row(row: &'r R) -> sqlx::Result<Self> {
        let leaf = row.try_get("leaf")?;
        let leaf: Leaf2<Types> = serde_json::from_value(leaf).decode_error("malformed leaf")?;

        let qc = row.try_get("qc")?;
        let qc: QuorumCertificate2<Types> =
            serde_json::from_value(qc).decode_error("malformed QC")?;

        Ok(Self { leaf, qc })
    }
}

/// Columns which must be selected for `BlockRow::from_row` to work.
///
/// The query must also `LEFT JOIN payload_loc AS pl ON pl.height = h.height`.
pub const BLOCK_COLUMNS: &str = "h.hash AS hash, h.data AS header_data, p.size AS payload_size, \
                                 p.data AS payload_data, pl.loc AS payload_loc";

/// A block row as stored, before the payload bytes are loaded.
///
/// Decoding performs no I/O. Pass it to `Transaction::load_block` to obtain the full block.
#[derive(Clone, Debug)]
pub struct BlockRow<Types: NodeType> {
    pub header: Header<Types>,
    pub hash: BlockHash<Types>,
    pub size: u64,
    pub payload: PayloadSource,
}

/// Where the payload bytes of a [`BlockRow`] live.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum PayloadSource {
    /// The payload has no bytes.
    #[default]
    Empty,
    /// The bytes are in the `payload.data` column.
    Inline(Vec<u8>),
    /// Raw blob store locator, parsed by `Transaction::load_block`.
    Blob(String),
    /// The record is known to exist but cannot be read.
    Missing,
}

impl<Types: NodeType> BlockRow<Types> {
    /// Build the block from the loaded payload bytes.
    pub fn into_block(self, payload: &[u8]) -> BlockQueryData<Types>
    where
        Header<Types>: QueryableHeader<Types>,
        Payload<Types>: QueryablePayload<Types>,
    {
        let payload = Payload::<Types>::from_bytes(payload, self.header.metadata());
        BlockQueryData {
            num_transactions: payload.len(self.header.metadata()) as u64,
            header: self.header,
            payload,
            size: self.size,
            hash: self.hash,
        }
    }
}

impl<'r, Types, R> FromRow<'r, R> for BlockRow<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    R: Row,
    for<'a> &'a str: ColumnIndex<R>,
    for<'a> i32: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> Vec<u8>: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> String: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> Json<Value>: Type<R::Database> + Decode<'a, R::Database>,
{
    fn from_row(row: &'r R) -> sqlx::Result<Self> {
        let size = row.try_get::<i32, _>("payload_size")? as u64;
        let data = row.try_get::<Vec<u8>, _>("payload_data")?;
        let loc = row.try_get::<Option<String>, _>("payload_loc")?;
        // Locator, then inline bytes, then empty. A non-empty payload with none of these is lost.
        let payload = match loc {
            Some(loc) => PayloadSource::Blob(loc),
            None if !data.is_empty() => PayloadSource::Inline(data),
            None if size == 0 => PayloadSource::Empty,
            None => PayloadSource::Missing,
        };

        let header_data = row.try_get("header_data")?;
        let header: Header<Types> =
            serde_json::from_value(header_data).decode_error("malformed header")?;

        let hash: String = row.try_get("hash")?;
        let hash = hash.parse().decode_error("malformed block hash")?;

        Ok(Self {
            header,
            hash,
            size,
            payload,
        })
    }
}

/// Columns selected for payload queries, the same as [`BLOCK_COLUMNS`].
pub const PAYLOAD_COLUMNS: &str = BLOCK_COLUMNS;

/// Columns which must be selected for `PayloadMetadata::from_row` to work.
pub const PAYLOAD_METADATA_COLUMNS: &str = "h.height AS height, h.hash AS hash, h.payload_hash AS \
                                            payload_hash, p.size AS payload_size, \
                                            p.num_transactions AS num_transactions";

impl<'r, Types, R> FromRow<'r, R> for PayloadMetadata<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    R: Row,
    for<'a> &'a str: ColumnIndex<R>,
    for<'a> i32: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> i64: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> String: Type<R::Database> + Decode<'a, R::Database>,
{
    fn from_row(row: &'r R) -> sqlx::Result<Self> {
        Ok(Self {
            height: row.try_get::<i64, _>("height")? as u64,
            block_hash: row
                .try_get::<String, _>("hash")?
                .parse()
                .decode_error("malformed block hash")?,
            hash: row
                .try_get::<String, _>("payload_hash")?
                .parse()
                .decode_error("malformed payload hash")?,
            size: row.try_get::<i32, _>("payload_size")? as u64,
            num_transactions: row.try_get::<i32, _>("num_transactions")? as u64,

            // Per-namespace info must be loaded in a separate query.
            namespaces: Default::default(),
        })
    }
}

/// Columns which must be selected for `VidCommonQueryData::from_row` to work.
pub const VID_COMMON_COLUMNS: &str = "h.height AS height, h.hash AS block_hash, h.payload_hash AS \
                                      payload_hash, v.data AS common_data";

impl<'r, Types, R> FromRow<'r, R> for VidCommonQueryData<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    R: Row,
    for<'a> &'a str: ColumnIndex<R>,
    for<'a> i64: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> String: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> Vec<u8>: Type<R::Database> + Decode<'a, R::Database>,
{
    fn from_row(row: &'r R) -> sqlx::Result<Self> {
        let height = row.try_get::<i64, _>("height")? as u64;
        let block_hash: String = row.try_get("block_hash")?;
        let block_hash = block_hash.parse().decode_error("malformed block hash")?;
        let payload_hash: String = row.try_get("payload_hash")?;
        let payload_hash = payload_hash
            .parse()
            .decode_error("malformed payload hash")?;
        let common_data: Vec<u8> = row.try_get("common_data")?;
        let common =
            bincode::deserialize(&common_data).decode_error("malformed VID common data")?;
        Ok(Self {
            height,
            block_hash,
            payload_hash,
            common,
        })
    }
}

/// Columns which must be selected for `VidCommonMetadata::from_row` to work.
pub const VID_COMMON_METADATA_COLUMNS: &str =
    "h.height AS height, h.hash AS block_hash, h.payload_hash AS payload_hash";

impl<'r, Types, R> FromRow<'r, R> for VidCommonMetadata<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    R: Row,
    for<'a> &'a str: ColumnIndex<R>,
    for<'a> i64: Type<R::Database> + Decode<'a, R::Database>,
    for<'a> String: Type<R::Database> + Decode<'a, R::Database>,
{
    fn from_row(row: &'r R) -> sqlx::Result<Self> {
        let height = row.try_get::<i64, _>("height")? as u64;
        let block_hash: String = row.try_get("block_hash")?;
        let block_hash = block_hash.parse().decode_error("malformed block hash")?;
        let payload_hash: String = row.try_get("payload_hash")?;
        let payload_hash = payload_hash
            .parse()
            .decode_error("malformed payload hash")?;
        Ok(Self {
            height,
            block_hash,
            payload_hash,
        })
    }
}

/// Columns which must be selected for [`parse_header`] to work.
pub const HEADER_COLUMNS: &str = "h.data AS data";

/// Extract a [`Header`] from a row.
///
/// We can't implement [`FromRow`] for `Header<Types>` since `Header<Types>` is not actually a type
/// defined in this crate; it's just an alias for `Types::BlockHeader`. So this standalone function
/// will have to do.
pub fn parse_header<Types, R>(row: R) -> sqlx::Result<Header<Types>>
where
    Types: NodeType,
    R: Row,
    for<'a> &'a str: ColumnIndex<R>,
    for<'a> Value: Type<R::Database> + Decode<'a, R::Database>,
{
    // Reconstruct the full header.
    let data = row.try_get("data")?;
    serde_json::from_value(data).decode_error("malformed header")
}

impl From<sqlx::Error> for QueryError {
    fn from(err: sqlx::Error) -> Self {
        if matches!(err, sqlx::Error::RowNotFound) {
            Self::NotFound
        } else {
            Self::Error {
                message: err.to_string(),
            }
        }
    }
}

/// [`Result`] adapter for converting generic errors into [`sqlx`] errors.
pub trait DecodeError {
    type Ok;
    fn decode_error(self, msg: impl Display) -> sqlx::Result<Self::Ok>;
}

impl<T, E> DecodeError for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    type Ok = T;
    fn decode_error(self, msg: impl Display) -> sqlx::Result<<Self as DecodeError>::Ok> {
        self.context(msg.to_string())
            .map_err(|err| sqlx::Error::Decode(err.into()))
    }
}
