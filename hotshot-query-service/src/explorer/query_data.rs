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

use std::{
    collections::VecDeque,
    fmt::{Debug, Display},
    num::{NonZeroUsize, TryFromIntError},
};

use hotshot_types::traits::{block_contents::BlockHeader, node_implementation::NodeType};
use serde::{Deserialize, Serialize};
use tide_disco::StatusCode;
use time::format_description::well_known::Rfc3339;

use super::{
    errors::{BadQuery, ExplorerAPIError, InvalidLimit, NotFound, QueryError, Unimplemented},
    monetary_value::MonetaryValue,
    traits::{ExplorerHeader, ExplorerTransaction},
};
use crate::{
    availability::{BlockQueryData, QueryablePayload, TransactionHash},
    node::BlockHash,
    types::HeightIndexed,
    Header, Payload, Resolvable, Transaction,
};

/// BlockIdentifier is an enum that represents multiple ways of referring to
/// a specific Block.  These use cases are specific to a Block Explorer and
/// can be used to reference a specific individual block.
///
/// Any identifier specified here is not guaranteed to be valid, and may not
/// guarantee that a Block can actually be identified with the information
/// provided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockIdentifier<Types: NodeType> {
    Latest,
    Height(usize),
    Hash(BlockHash<Types>),
}

impl<Types: NodeType> Display for BlockIdentifier<Types> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockIdentifier::Latest => write!(f, "latest"),
            BlockIdentifier::Height(height) => write!(f, "{}", height),
            BlockIdentifier::Hash(hash) => write!(f, "{}", hash),
        }
    }
}

/// TransactionIdentifier is an enum that represents multiple ways of of
/// identifying a specific Transaction.  These use cases are specific to a
/// Block Explorer and can be used to **ideally** uniquely identify a
/// `Transaction` within the Block Chain.
///
/// Any identified specified here is not guaranteed to actually point to a
/// transaction, and does not guarantee that a transaction with the specified
/// identification actually exists.
///
/// A TransactionHash is not actually guaranteed to point to a unique
/// transaction at the moment, however we will assume that it does for the
/// purposes of this API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionIdentifier<Types: NodeType> {
    Latest,
    HeightAndOffset(usize, usize),
    Hash(TransactionHash<Types>),
}

impl<Types: NodeType> Display for TransactionIdentifier<Types> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionIdentifier::Latest => write!(f, "latest"),
            TransactionIdentifier::HeightAndOffset(height, offset) => {
                write!(f, "{} {}", height, offset)
            },
            TransactionIdentifier::Hash(hash) => write!(f, "{}", hash),
        }
    }
}

/// BlockRange is a struct that represents a range for a specific set of
/// blocks, starting at the given `BlockIdentifier`.
///
/// This range is expected to be descending starting and the `target` and
/// descending toward `0`.
///
/// Given a stable and resolved Block Chain this should always refer to the
/// same set of blocks when the parameters themselves are the same.
///
/// If the `num_blocks` is not possible, then this should be considered as
/// referring to as many `Block`s as are possible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRange<Types: NodeType> {
    pub target: BlockIdentifier<Types>,
    pub num_blocks: NonZeroUsize,
}

/// TransactionRange is a struct that represents a range for a specific set of
/// transactions, starting at the given `TransactionIdentifier`.
///
/// This range is expected to be descending starting at the `target` and
/// descending toward the first transaction in the `Block Chain`.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionRange<Types: NodeType> {
    pub target: TransactionIdentifier<Types>,
    pub num_transactions: NonZeroUsize,
}

/// [Timestamp] represents a specific point in time that has a possible
/// offset.
///
/// This specific type is utilized in order to ensure that the timestamp is
/// always serialized in a specific format, specifically as string following
/// the RFC3339 standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp(pub time::OffsetDateTime);

impl Serialize for Timestamp {
    /// serialize converts the timestamp into a string representation of a
    /// RFC3339 formatted date.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let formatted = self.0.format(&Rfc3339).map_err(serde::ser::Error::custom)?;
        formatted.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    /// deserialize converts a string representation of a RFC3339 formatted
    /// date.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let dt = time::OffsetDateTime::parse(&s, &Rfc3339).map_err(serde::de::Error::custom)?;
        Ok(Timestamp(dt))
    }
}

pub type WalletAddress<Types> = <Header<Types> as ExplorerHeader<Types>>::WalletAddress;
pub type BlockNamespaceId<Types> = <Header<Types> as ExplorerHeader<Types>>::NamespaceId;
pub type TransactionNamespaceId<Types> = <Transaction<Types> as ExplorerTransaction>::NamespaceId;
pub type ProposerId<Types> = <Header<Types> as ExplorerHeader<Types>>::ProposerId;
pub type BalanceAmount<Types> = <Header<Types> as ExplorerHeader<Types>>::BalanceAmount;

/// [BlockDetail] is a struct that represents the details of a specific block
/// for use in a Block Explorer.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct BlockDetail<Types: NodeType>
where
    Header<Types>: ExplorerHeader<Types>,
{
    pub hash: BlockHash<Types>,
    pub height: u64,
    pub time: Timestamp,
    pub num_transactions: u64,
    pub proposer_id: ProposerId<Types>,
    pub fee_recipient: WalletAddress<Types>,
    pub size: u64,
    pub block_reward: Vec<MonetaryValue>,
}

impl<Types: NodeType> TryFrom<BlockQueryData<Types>> for BlockDetail<Types>
where
    BlockQueryData<Types>: HeightIndexed,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: BlockHeader<Types> + ExplorerHeader<Types>,
    BalanceAmount<Types>: Into<MonetaryValue>,
{
    type Error = TimestampConversionError;

    fn try_from(value: BlockQueryData<Types>) -> Result<Self, Self::Error> {
        let seconds = i64::try_from(value.header.timestamp())?;

        Ok(Self {
            hash: value.hash(),
            height: value.height(),
            time: Timestamp(time::OffsetDateTime::from_unix_timestamp(seconds)?),
            num_transactions: value.num_transactions,
            proposer_id: value.header().proposer_id(),
            fee_recipient: value.header().fee_info_account(),
            size: value.size,
            block_reward: vec![value.header().fee_info_balance().into()],
        })
    }
}

/// [BlockSummary] is a struct that represents a summary overview of a specific
/// block.  It does not have all of the details of a [BlockDetail], but it is
/// useful for displaying information in a list of Blocks.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct BlockSummary<Types: NodeType>
where
    Header<Types>: ExplorerHeader<Types>,
{
    pub hash: BlockHash<Types>,
    pub height: u64,
    pub proposer_id: ProposerId<Types>,
    pub num_transactions: u64,
    pub size: u64,
    pub time: Timestamp,
}

/// [TimestampConversionError] represents an error that has occurred when
/// attempting to convert a timestamp from a specific format to another.
/// It is primarily used when attempting to deserialize a [Timestamp] from
/// its serialized string representation.
#[derive(Debug, PartialEq, Eq)]
pub enum TimestampConversionError {
    TimeError(time::error::ComponentRange),
    IntError(TryFromIntError),
}

impl Display for TimestampConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimestampConversionError::TimeError(err) => write!(f, "{:?}", err),
            TimestampConversionError::IntError(err) => write!(f, "{:?}", err),
        }
    }
}

impl std::error::Error for TimestampConversionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TimestampConversionError::TimeError(err) => Some(err),
            TimestampConversionError::IntError(err) => Some(err),
        }
    }
}

impl From<time::error::ComponentRange> for TimestampConversionError {
    fn from(value: time::error::ComponentRange) -> Self {
        Self::TimeError(value)
    }
}

impl From<TryFromIntError> for TimestampConversionError {
    fn from(value: TryFromIntError) -> Self {
        Self::IntError(value)
    }
}

impl From<TimestampConversionError> for crate::QueryError {
    fn from(value: TimestampConversionError) -> Self {
        Self::Error {
            message: format!("{:?}", value),
        }
    }
}

impl<Types: NodeType> TryFrom<BlockQueryData<Types>> for BlockSummary<Types>
where
    BlockQueryData<Types>: HeightIndexed,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: BlockHeader<Types> + ExplorerHeader<Types>,
{
    type Error = TimestampConversionError;

    fn try_from(value: BlockQueryData<Types>) -> Result<Self, Self::Error> {
        let seconds = i64::try_from(value.header.timestamp())?;

        Ok(Self {
            hash: value.hash(),
            height: value.height(),
            proposer_id: value.header().proposer_id(),
            num_transactions: value.num_transactions,
            size: value.size,
            time: Timestamp(time::OffsetDateTime::from_unix_timestamp(seconds)?),
        })
    }
}

/// [FeeAttribution] represents a specific attribution of fees for a specific
/// purpose.
///
/// The current documentation lists attribution as potentially being
/// accountable for the following entries:
/// - Sequencer
/// - DA Layer
/// - Ethereum Mainnet
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeeAttribution {
    pub target: String,
    pub fees: Vec<MonetaryValue>,
}

/// [TransactionDetail] is a struct that represents the details of a specific
/// transaction / payload contained within a Block.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct TransactionDetail<Types: NodeType> {
    pub hash: TransactionHash<Types>,
    pub height: u64,
    pub block_confirmed: bool,
    pub offset: u64,
    pub num_transactions: u64,
    pub size: u64,
    pub time: Timestamp,
    pub sequencing_fees: Vec<MonetaryValue>,
    pub fee_details: Vec<FeeAttribution>,
}

/// [TransactionDetailResponse] is a struct that represents the information
/// returned concerning a request for a Transaction Detail. It contains the
/// data payloads separately from the details of the Transaction itself.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct TransactionDetailResponse<Types: NodeType> {
    pub details: TransactionDetail<Types>,
    pub data: Vec<Transaction<Types>>,
}

/// [TransactionSummary] is a struct that represents a summary overview of a
/// specific transaction / payload contained within a Block. It does not have
/// all of the details of a [TransactionDetail], but it is useful for displaying
/// information in a list of Transactions.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct TransactionSummary<Types: NodeType>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction,
{
    pub hash: TransactionHash<Types>,
    pub rollups: Vec<TransactionNamespaceId<Types>>,
    pub height: u64,
    pub offset: u64,
    pub num_transactions: u64,
    pub time: Timestamp,
}

impl<Types: NodeType>
    TryFrom<(
        &BlockQueryData<Types>,
        usize,
        <Types as NodeType>::Transaction,
    )> for TransactionSummary<Types>
where
    BlockQueryData<Types>: HeightIndexed,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: BlockHeader<Types> + ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction,
{
    type Error = TimestampConversionError;

    fn try_from(
        (block, offset, transaction): (
            &BlockQueryData<Types>,
            usize,
            <Types as NodeType>::Transaction,
        ),
    ) -> Result<Self, Self::Error> {
        let seconds = i64::try_from(block.header.timestamp())?;

        Ok(Self {
            hash: transaction.commitment(),
            height: block.height(),
            offset: offset as u64,
            num_transactions: block.num_transactions,
            time: Timestamp(time::OffsetDateTime::from_unix_timestamp(seconds)?),
            rollups: vec![transaction.namespace_id()],
        })
    }
}

impl<Types: NodeType>
    TryFrom<(
        &BlockQueryData<Types>,
        usize,
        <Types as NodeType>::Transaction,
    )> for TransactionDetailResponse<Types>
where
    BlockQueryData<Types>: HeightIndexed,
    Payload<Types>: QueryablePayload<Types>,
    Header<Types>: BlockHeader<Types> + ExplorerHeader<Types>,
    <Types as NodeType>::Transaction: ExplorerTransaction,
{
    type Error = TimestampConversionError;

    fn try_from(
        (block, offset, transaction): (
            &BlockQueryData<Types>,
            usize,
            <Types as NodeType>::Transaction,
        ),
    ) -> Result<Self, Self::Error> {
        let seconds = i64::try_from(block.header.timestamp())?;

        Ok(Self {
            details: TransactionDetail {
                hash: transaction.commitment(),
                height: block.height(),
                block_confirmed: true,
                offset: offset as u64,
                num_transactions: block.num_transactions,
                size: transaction.payload_size(),
                time: Timestamp(time::OffsetDateTime::from_unix_timestamp(seconds)?),
                sequencing_fees: vec![],
                fee_details: vec![],
            },
            data: vec![transaction],
        })
    }
}

/// GetBlockSummariesRequest is a struct that represents an incoming request
/// for Block Summaries.  This isn't sent on the line, but an endpoint will
/// be mapped to this struct in order for the request to be processed.
#[derive(Debug, PartialEq, Eq)]
pub struct GetBlockSummariesRequest<Types: NodeType>(pub BlockRange<Types>);

/// [TransactionSummaryFilter] represents the various filters that can be
/// applied when retrieving a list of [TransactionSummary] entries.
#[derive(Debug, Deserialize, Serialize)]
pub enum TransactionSummaryFilter {
    None,
    RollUp(usize),
    Block(usize),
}

/// GetTransactionSummariesRequest is a struct that represents an incoming
/// request for Transaction Summaries.  This isn't sent on the line, but an
/// endpoint will be mapped to this struct in order for the request to be
/// processed.
#[derive(Debug)]
pub struct GetTransactionSummariesRequest<Types: NodeType> {
    pub range: TransactionRange<Types>,
    pub filter: TransactionSummaryFilter,
}

impl<Types: NodeType> Default for GetTransactionSummariesRequest<Types> {
    fn default() -> Self {
        Self {
            range: TransactionRange {
                target: TransactionIdentifier::Latest,
                num_transactions: NonZeroUsize::new(20).unwrap(),
            },
            filter: TransactionSummaryFilter::None,
        }
    }
}

/// [GenesisOverview] provides a summary overview of the block chain since
/// it's genesis. At a high level it includes the total number of unique
/// rollups, transactions, and blocks that are in the block chain.
#[derive(Debug, Serialize, Deserialize)]
pub struct GenesisOverview {
    pub rollups: u64,
    pub transactions: u64,
    pub blocks: u64,
    // pub sequencer_nodes: u64,
}

/// [ExplorerHistograms] provides a series of data points that can be used to
/// draw simple histograms for the Block Explorer.  The data returned is meant
/// to be an optimal packing of the values being returned.
///
/// It contains data for the last N blocks, indicated by the length of the
/// vectors contained within the struct.  All of the vectors **MUST** have the
/// same length.  The labels of the graph points is the `block_heights` vector.
/// The remaining data points are the `block_time`, `block_size`, and
/// `block_transactions` for those `block_heights`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExplorerHistograms {
    pub block_time: VecDeque<Option<u64>>,
    pub block_size: VecDeque<Option<u64>>,
    pub block_transactions: VecDeque<u64>,
    pub block_heights: VecDeque<u64>,
}

/// [ExplorerSummary] is a struct that represents an at-a-glance snapshot of
/// the Block Chain.  It contains some helpful information that can be used
/// to display a simple health check of the Block Chain.
///
/// It contains the latest block for reference, the most recent blocks, the
/// most recent transactions, some statistics concerning the total number
/// of elements contained within the chain, and some histograms that can be
/// used to draw graphs for the Block Explorer.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct ExplorerSummary<Types: NodeType>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction,
{
    pub latest_block: BlockDetail<Types>,
    pub genesis_overview: GenesisOverview,
    pub latest_blocks: Vec<BlockSummary<Types>>,
    pub latest_transactions: Vec<TransactionSummary<Types>>,
    //  Most Active Rollups
    pub histograms: ExplorerHistograms,
}

/// [SearchResult] is a struct that represents the results of executing a
/// search query against the chain.  It contains a list of blocks and
/// transactions that match the search query.
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct SearchResult<Types: NodeType>
where
    Header<Types>: ExplorerHeader<Types>,
    Transaction<Types>: ExplorerTransaction,
{
    pub blocks: Vec<BlockSummary<Types>>,
    pub transactions: Vec<TransactionSummary<Types>>,
}

/// [GetBlockDetailError] represents an error that has occurred in response to
/// the `get_block_detail` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetBlockDetailError {
    Unimplemented(Unimplemented),
    BlockNotFound(NotFound),
    QueryError(QueryError),
}

impl GetBlockDetailError {
    pub fn status(&self) -> StatusCode {
        match self {
            GetBlockDetailError::Unimplemented(err) => err.status(),
            GetBlockDetailError::QueryError(err) => err.status(),
            GetBlockDetailError::BlockNotFound(err) => err.status(),
        }
    }
}

impl Display for GetBlockDetailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetBlockDetailError::Unimplemented(err) => write!(f, "{err}"),
            GetBlockDetailError::QueryError(err) => write!(f, "{err}"),
            GetBlockDetailError::BlockNotFound(err) => write!(f, "{err}"),
        }
    }
}

impl ExplorerAPIError for GetBlockDetailError {
    fn code(&self) -> &str {
        match self {
            GetBlockDetailError::Unimplemented(err) => err.code(),
            GetBlockDetailError::QueryError(err) => err.code(),
            GetBlockDetailError::BlockNotFound(err) => err.code(),
        }
    }
}

impl std::error::Error for GetBlockDetailError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GetBlockDetailError::Unimplemented(err) => Some(err),
            GetBlockDetailError::QueryError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<crate::QueryError> for GetBlockDetailError {
    fn from(value: crate::QueryError) -> Self {
        GetBlockDetailError::QueryError(QueryError { error: value })
    }
}

/// [GetBlockSummariesError] represents an error that has occurred in response
/// to the [GetBlockSummariesRequest] request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetBlockSummariesError {
    Unimplemented(Unimplemented),
    InvalidLimit(InvalidLimit),
    TargetNotFound(NotFound),
    QueryError(QueryError),
}

impl GetBlockSummariesError {
    pub fn status(&self) -> StatusCode {
        match self {
            GetBlockSummariesError::Unimplemented(err) => err.status(),
            GetBlockSummariesError::InvalidLimit(err) => err.status(),
            GetBlockSummariesError::QueryError(err) => err.status(),
            GetBlockSummariesError::TargetNotFound(err) => err.status(),
        }
    }
}

impl Display for GetBlockSummariesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetBlockSummariesError::Unimplemented(err) => write!(f, "{err}"),
            GetBlockSummariesError::InvalidLimit(err) => write!(f, "{err}"),
            GetBlockSummariesError::QueryError(err) => write!(f, "{err}"),
            GetBlockSummariesError::TargetNotFound(err) => write!(f, "{err}"),
        }
    }
}

impl ExplorerAPIError for GetBlockSummariesError {
    fn code(&self) -> &str {
        match self {
            GetBlockSummariesError::Unimplemented(err) => err.code(),
            GetBlockSummariesError::InvalidLimit(err) => err.code(),
            GetBlockSummariesError::QueryError(err) => err.code(),
            GetBlockSummariesError::TargetNotFound(err) => err.code(),
        }
    }
}

impl std::error::Error for GetBlockSummariesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GetBlockSummariesError::Unimplemented(err) => Some(err),
            GetBlockSummariesError::InvalidLimit(err) => Some(err),
            GetBlockSummariesError::QueryError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<crate::QueryError> for GetBlockSummariesError {
    fn from(value: crate::QueryError) -> Self {
        GetBlockSummariesError::QueryError(QueryError { error: value })
    }
}

/// [GetTransactionDetailError] represents an error that has occurred in
/// response to the `get_transaction_detail` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetTransactionDetailError {
    Unimplemented(Unimplemented),
    TransactionNotFound(NotFound),
    QueryError(QueryError),
}

impl GetTransactionDetailError {
    pub fn status(&self) -> StatusCode {
        match self {
            GetTransactionDetailError::Unimplemented(err) => err.status(),
            GetTransactionDetailError::QueryError(err) => err.status(),
            GetTransactionDetailError::TransactionNotFound(err) => err.status(),
        }
    }
}

impl Display for GetTransactionDetailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetTransactionDetailError::Unimplemented(err) => write!(f, "{err}"),
            GetTransactionDetailError::QueryError(err) => write!(f, "{err}"),
            GetTransactionDetailError::TransactionNotFound(err) => write!(f, "{err}"),
        }
    }
}

impl ExplorerAPIError for GetTransactionDetailError {
    fn code(&self) -> &str {
        match self {
            GetTransactionDetailError::Unimplemented(err) => err.code(),
            GetTransactionDetailError::QueryError(err) => err.code(),
            GetTransactionDetailError::TransactionNotFound(err) => err.code(),
        }
    }
}

impl std::error::Error for GetTransactionDetailError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GetTransactionDetailError::Unimplemented(err) => Some(err),
            GetTransactionDetailError::QueryError(err) => Some(err),
            _ => None,
        }
    }
}

// Implement implicit conversion between these errors for the branch operator.

impl From<crate::QueryError> for GetTransactionDetailError {
    fn from(value: crate::QueryError) -> Self {
        GetTransactionDetailError::QueryError(QueryError { error: value })
    }
}

impl From<TimestampConversionError> for GetTransactionDetailError {
    fn from(value: TimestampConversionError) -> Self {
        GetTransactionDetailError::QueryError(QueryError {
            error: value.into(),
        })
    }
}

/// [GetTransactionSummariesError] represents an error that has occurred in
/// response to the [GetTransactionSummariesRequest] request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetTransactionSummariesError {
    Unimplemented(Unimplemented),
    InvalidLimit(InvalidLimit),
    TargetNotFound(NotFound),
    QueryError(QueryError),
}

impl GetTransactionSummariesError {
    pub fn status(&self) -> StatusCode {
        match self {
            GetTransactionSummariesError::Unimplemented(err) => err.status(),
            GetTransactionSummariesError::InvalidLimit(err) => err.status(),
            GetTransactionSummariesError::QueryError(err) => err.status(),
            GetTransactionSummariesError::TargetNotFound(err) => err.status(),
        }
    }
}

impl Display for GetTransactionSummariesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetTransactionSummariesError::Unimplemented(err) => write!(f, "{err}"),
            GetTransactionSummariesError::InvalidLimit(err) => write!(f, "{err}"),
            GetTransactionSummariesError::QueryError(err) => write!(f, "{err}"),
            GetTransactionSummariesError::TargetNotFound(err) => write!(f, "{err}"),
        }
    }
}

impl ExplorerAPIError for GetTransactionSummariesError {
    fn code(&self) -> &str {
        match self {
            GetTransactionSummariesError::Unimplemented(err) => err.code(),
            GetTransactionSummariesError::InvalidLimit(err) => err.code(),
            GetTransactionSummariesError::QueryError(err) => err.code(),
            GetTransactionSummariesError::TargetNotFound(err) => err.code(),
        }
    }
}

impl std::error::Error for GetTransactionSummariesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GetTransactionSummariesError::Unimplemented(err) => Some(err),
            GetTransactionSummariesError::InvalidLimit(err) => Some(err),
            GetTransactionSummariesError::QueryError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<crate::QueryError> for GetTransactionSummariesError {
    fn from(value: crate::QueryError) -> Self {
        GetTransactionSummariesError::QueryError(QueryError { error: value })
    }
}

/// [GetExplorerSummaryError] represents an error that has occurred in response
/// to the `get_explorer_summary` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetExplorerSummaryError {
    Unimplemented(Unimplemented),
    QueryError(QueryError),
    GetBlockDetailError(GetBlockDetailError),
    GetBlockSummariesError(GetBlockSummariesError),
    GetTransactionSummariesError(GetTransactionSummariesError),
}

impl GetExplorerSummaryError {
    pub fn status(&self) -> StatusCode {
        match self {
            GetExplorerSummaryError::QueryError(err) => err.status(),
            GetExplorerSummaryError::Unimplemented(err) => err.status(),
            GetExplorerSummaryError::GetBlockDetailError(err) => err.status(),
            GetExplorerSummaryError::GetBlockSummariesError(err) => err.status(),
            GetExplorerSummaryError::GetTransactionSummariesError(err) => err.status(),
        }
    }
}

impl Display for GetExplorerSummaryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetExplorerSummaryError::QueryError(err) => write!(f, "{err}"),
            GetExplorerSummaryError::Unimplemented(err) => write!(f, "{err}"),
            GetExplorerSummaryError::GetBlockDetailError(err) => write!(f, "{err}"),
            GetExplorerSummaryError::GetBlockSummariesError(err) => write!(f, "{err}"),
            GetExplorerSummaryError::GetTransactionSummariesError(err) => write!(f, "{err}"),
        }
    }
}

impl ExplorerAPIError for GetExplorerSummaryError {
    fn code(&self) -> &str {
        match self {
            GetExplorerSummaryError::QueryError(err) => err.code(),
            GetExplorerSummaryError::Unimplemented(err) => err.code(),
            GetExplorerSummaryError::GetBlockDetailError(err) => err.code(),
            GetExplorerSummaryError::GetBlockSummariesError(err) => err.code(),
            GetExplorerSummaryError::GetTransactionSummariesError(err) => err.code(),
        }
    }
}

impl std::error::Error for GetExplorerSummaryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GetExplorerSummaryError::Unimplemented(err) => Some(err),
            GetExplorerSummaryError::QueryError(err) => Some(err),
            GetExplorerSummaryError::GetBlockDetailError(err) => Some(err),
            GetExplorerSummaryError::GetBlockSummariesError(err) => Some(err),
            GetExplorerSummaryError::GetTransactionSummariesError(err) => Some(err),
        }
    }
}

// Implement implicit conversion between these errors for the branch operator.

impl From<crate::QueryError> for GetExplorerSummaryError {
    fn from(value: crate::QueryError) -> Self {
        GetExplorerSummaryError::QueryError(QueryError { error: value })
    }
}

impl From<GetBlockDetailError> for GetExplorerSummaryError {
    fn from(value: GetBlockDetailError) -> Self {
        GetExplorerSummaryError::GetBlockDetailError(value)
    }
}

impl From<GetBlockSummariesError> for GetExplorerSummaryError {
    fn from(value: GetBlockSummariesError) -> Self {
        GetExplorerSummaryError::GetBlockSummariesError(value)
    }
}

impl From<GetTransactionSummariesError> for GetExplorerSummaryError {
    fn from(value: GetTransactionSummariesError) -> Self {
        GetExplorerSummaryError::GetTransactionSummariesError(value)
    }
}

/// [GetSearchResultsError] represents an error that has occurred in response
/// to the `get_search_results` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GetSearchResultsError {
    Unimplemented(Unimplemented),
    QueryError(QueryError),
    InvalidQuery(BadQuery),
}

impl GetSearchResultsError {
    pub fn status(&self) -> StatusCode {
        match self {
            GetSearchResultsError::QueryError(err) => err.status(),
            GetSearchResultsError::Unimplemented(err) => err.status(),
            GetSearchResultsError::InvalidQuery(err) => err.status(),
        }
    }
}

impl Display for GetSearchResultsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetSearchResultsError::QueryError(err) => write!(f, "{err}"),
            GetSearchResultsError::Unimplemented(err) => write!(f, "{err}"),
            GetSearchResultsError::InvalidQuery(err) => write!(f, "{err}"),
        }
    }
}

impl ExplorerAPIError for GetSearchResultsError {
    fn code(&self) -> &str {
        match self {
            GetSearchResultsError::QueryError(err) => err.code(),
            GetSearchResultsError::Unimplemented(err) => err.code(),
            GetSearchResultsError::InvalidQuery(err) => err.code(),
        }
    }
}

impl std::error::Error for GetSearchResultsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GetSearchResultsError::Unimplemented(err) => Some(err),
            GetSearchResultsError::QueryError(err) => Some(err),
            GetSearchResultsError::InvalidQuery(err) => Some(err),
        }
    }
}

impl From<crate::QueryError> for GetSearchResultsError {
    fn from(value: crate::QueryError) -> Self {
        GetSearchResultsError::QueryError(QueryError { error: value })
    }
}
