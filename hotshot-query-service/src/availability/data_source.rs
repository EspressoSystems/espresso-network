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
    cmp::Ordering,
    ops::{Bound, RangeBounds},
};

use async_trait::async_trait;
use derivative::Derivative;
use derive_more::{Display, From};
use futures::{
    future::Future,
    stream::{BoxStream, StreamExt},
};
use hotshot_types::{
    data::{VidCommitment, VidShare},
    simple_certificate::CertificatePair,
    traits::node_implementation::NodeType,
};

use super::{
    fetch::Fetch,
    query_data::{
        BlockHash, BlockQueryData, LeafHash, LeafQueryData, PayloadMetadata, PayloadQueryData,
        QueryableHeader, QueryablePayload, TransactionHash, VidCommonMetadata, VidCommonQueryData,
    },
    BlockWithTransaction,
};
use crate::{types::HeightIndexed, Header, Payload};

#[derive(Derivative, From, Display)]
#[derivative(Ord = "feature_allow_slow_enum")]
#[derivative(
    Copy(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    Ord(bound = ""),
    Hash(bound = "")
)]
pub enum LeafId<Types: NodeType> {
    #[display("{_0}")]
    Number(usize),
    #[display("{_0}")]
    Hash(LeafHash<Types>),
}

impl<Types: NodeType> Clone for LeafId<Types> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Types: NodeType> PartialOrd for LeafId<Types> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Derivative, From, Display)]
#[derivative(Ord = "feature_allow_slow_enum")]
#[derivative(
    Copy(bound = ""),
    Debug(bound = ""),
    PartialEq(bound = ""),
    Eq(bound = ""),
    Ord(bound = ""),
    Hash(bound = "")
)]
pub enum BlockId<Types: NodeType> {
    #[display("{_0}")]
    Number(usize),
    #[display("{_0}")]
    Hash(BlockHash<Types>),
    #[display("{_0}")]
    #[from(ignore)]
    PayloadHash(VidCommitment),
}

impl<Types: NodeType> Clone for BlockId<Types> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Types: NodeType> PartialOrd for BlockId<Types> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub type FetchStream<T> = BoxStream<'static, Fetch<T>>;

/// An interface for querying a HotShot blockchain.
///
/// This interface provides expressive queries over all the data which is made available by HotShot
/// consensus. The data exposed by this interface consists entirely of _normative_ data: data which
/// every correct HotShot node or light client will agree on, and which is guaranteed by consensus
/// to be immutable. This immutability has an interesting consequence: all of the methods exposed by
/// this trait are _pure_: given equivalent inputs, the same method will always return equivalent
/// outputs[^1].
///
/// This purity property has a further consequence: none of the methods defined here can fail! Even
/// if you query for a block at a position past the end of the current chain -- a block which does
/// not exist yet -- the query will not fail. It will return an in-progress [`Fetch`] which, when it
/// finally does resolve, resolves to the unique block at that position in the chain. All subsequent
/// queries for the same position will eventually resolve to the same block.
///
/// In other words, the abstraction is that of an infinitely long chain which exists statically, in
/// its entirety, at all times. In reality, of course, this chain is being produced incrementally
/// and has a finite length at any given time. But all this means is that some queries may take a
/// long time to resolve while others may resolve immediately.
///
/// [^1]: The data returned by these methods are wrapped in [`Fetch`], which does not implement
///       [`PartialEq]`. So to speak of equivalent outputs, we need to define an equivalence
///       relation on [`Fetch<T>`]. The relation we will use is defined when `T: PartialEq`, and
///       defines two fetches `f1` and `f2` as equivalent when `f1.await == f2.await`. That is,
///       depending on when you call a certain method, you may or may not get a response
///       immediately. But whenever you do get the data you requested, it is unique for that
///       combination of inputs.
#[async_trait]
pub trait AvailabilityDataSource<Types: NodeType>
where
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    async fn get_leaf<ID>(&self, id: ID) -> Fetch<LeafQueryData<Types>>
    where
        ID: Into<LeafId<Types>> + Send + Sync;

    async fn get_header<ID>(&self, id: ID) -> Fetch<Header<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync;

    async fn get_block<ID>(&self, id: ID) -> Fetch<BlockQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync;

    async fn get_payload<ID>(&self, id: ID) -> Fetch<PayloadQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync;

    async fn get_payload_metadata<ID>(&self, id: ID) -> Fetch<PayloadMetadata<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync;

    async fn get_vid_common<ID>(&self, id: ID) -> Fetch<VidCommonQueryData<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync;

    async fn get_vid_common_metadata<ID>(&self, id: ID) -> Fetch<VidCommonMetadata<Types>>
    where
        ID: Into<BlockId<Types>> + Send + Sync;

    async fn get_leaf_range<R>(&self, range: R) -> FetchStream<LeafQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static;

    async fn get_header_range<R>(&self, range: R) -> FetchStream<Header<Types>>
    where
        R: RangeBounds<usize> + Send + 'static;

    async fn get_block_range<R>(&self, range: R) -> FetchStream<BlockQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static;

    async fn get_payload_range<R>(&self, range: R) -> FetchStream<PayloadQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static;

    async fn get_payload_metadata_range<R>(&self, range: R) -> FetchStream<PayloadMetadata<Types>>
    where
        R: RangeBounds<usize> + Send + 'static;

    async fn get_vid_common_range<R>(&self, range: R) -> FetchStream<VidCommonQueryData<Types>>
    where
        R: RangeBounds<usize> + Send + 'static;

    async fn get_vid_common_metadata_range<R>(
        &self,
        range: R,
    ) -> FetchStream<VidCommonMetadata<Types>>
    where
        R: RangeBounds<usize> + Send + 'static;

    async fn get_leaf_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<LeafQueryData<Types>>;

    async fn get_block_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<BlockQueryData<Types>>;

    async fn get_payload_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<PayloadQueryData<Types>>;

    async fn get_payload_metadata_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<PayloadMetadata<Types>>;

    async fn get_vid_common_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<VidCommonQueryData<Types>>;

    async fn get_vid_common_metadata_range_rev(
        &self,
        start: Bound<usize>,
        end: usize,
    ) -> FetchStream<VidCommonMetadata<Types>>;

    async fn get_block_containing_transaction(
        &self,
        h: TransactionHash<Types>,
    ) -> Fetch<BlockWithTransaction<Types>>;

    async fn subscribe_blocks(&self, from: usize) -> BoxStream<'static, BlockQueryData<Types>> {
        self.get_block_range(from..)
            .await
            .then(Fetch::resolve)
            .boxed()
    }

    async fn subscribe_payloads(&self, from: usize) -> BoxStream<'static, PayloadQueryData<Types>> {
        self.get_payload_range(from..)
            .await
            .then(Fetch::resolve)
            .boxed()
    }

    async fn subscribe_payload_metadata(
        &self,
        from: usize,
    ) -> BoxStream<'static, PayloadMetadata<Types>> {
        self.get_payload_metadata_range(from..)
            .await
            .then(Fetch::resolve)
            .boxed()
    }

    async fn subscribe_leaves(&self, from: usize) -> BoxStream<'static, LeafQueryData<Types>> {
        self.get_leaf_range(from..)
            .await
            .then(Fetch::resolve)
            .boxed()
    }

    async fn subscribe_headers(&self, from: usize) -> BoxStream<'static, Header<Types>> {
        self.get_header_range(from..)
            .await
            .then(Fetch::resolve)
            .boxed()
    }

    async fn subscribe_vid_common(
        &self,
        from: usize,
    ) -> BoxStream<'static, VidCommonQueryData<Types>> {
        self.get_vid_common_range(from..)
            .await
            .then(Fetch::resolve)
            .boxed()
    }

    async fn subscribe_vid_common_metadata(
        &self,
        from: usize,
    ) -> BoxStream<'static, VidCommonMetadata<Types>> {
        self.get_vid_common_metadata_range(from..)
            .await
            .then(Fetch::resolve)
            .boxed()
    }
}

/// Information about a block.
///
/// This type encapsulate all the information we might have about a decided HotShot block:
/// * The leaf, including a header and consensus metadata
/// * The block itself, which may be missing if this node did not receive a DA proposal for this
///   block
/// * VID common and a unique VID share, which may be missing if this node did not receive a VID
///   share for this block
#[derive(Clone, Debug)]
pub struct BlockInfo<Types: NodeType> {
    pub leaf: LeafQueryData<Types>,
    pub block: Option<BlockQueryData<Types>>,
    pub vid_common: Option<VidCommonQueryData<Types>>,
    pub vid_share: Option<VidShare>,
    pub qc_chain: Option<[CertificatePair<Types>; 2]>,
}

impl<Types: NodeType> From<LeafQueryData<Types>> for BlockInfo<Types> {
    fn from(leaf: LeafQueryData<Types>) -> Self {
        Self::new(leaf, None, None, None)
    }
}

impl<Types: NodeType> HeightIndexed for BlockInfo<Types> {
    fn height(&self) -> u64 {
        self.leaf.height()
    }
}

impl<Types: NodeType> BlockInfo<Types> {
    pub fn new(
        leaf: LeafQueryData<Types>,
        block: Option<BlockQueryData<Types>>,
        vid_common: Option<VidCommonQueryData<Types>>,
        vid_share: Option<VidShare>,
    ) -> Self {
        Self {
            leaf,
            block,
            vid_common,
            vid_share,
            qc_chain: None,
        }
    }

    pub fn with_qc_chain(mut self, qc_chain: [CertificatePair<Types>; 2]) -> Self {
        self.qc_chain = Some(qc_chain);
        self
    }
}

pub trait UpdateAvailabilityData<Types: NodeType> {
    /// Append information about a new block to the database.
    fn append(&self, info: BlockInfo<Types>) -> impl Send + Future<Output = anyhow::Result<()>>;
}
