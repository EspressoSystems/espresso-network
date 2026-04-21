//! Functionality for using the [`LightClient`] as a query service fetching [`Provider`].

use async_trait::async_trait;
use espresso_types::{Payload, SeqTypes};
use hotshot_query_service::{
    availability::{BlockQueryData, LeafId, LeafQueryData, VidCommonQueryData},
    fetching::{
        NonEmptyRange, Provider,
        request::{
            BlockRangeRequest, LeafRangeRequest, LeafRequest, PayloadRequest,
            VidCommonRangeRequest, VidCommonRequest,
        },
    },
    node::BlockId,
};
use hotshot_types::data::VidCommon;

use crate::{LightClient, client::Client, storage::Storage};

#[async_trait]
impl<P, S> Provider<SeqTypes, LeafRequest> for LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    async fn fetch(&self, req: LeafRequest) -> Option<LeafQueryData<SeqTypes>> {
        match self.fetch_leaf(LeafId::Number(req.height as usize)).await {
            Ok(leaf) => Some(leaf),
            Err(err) => {
                tracing::warn!(?req, "failed to fetch leaf: {err:#}");
                None
            },
        }
    }
}

#[async_trait]
impl<P, S> Provider<SeqTypes, PayloadRequest> for LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    async fn fetch(&self, req: PayloadRequest) -> Option<Payload> {
        match self.fetch_payload(BlockId::PayloadHash(req.0)).await {
            Ok(payload) => Some(payload.data),
            Err(err) => {
                tracing::warn!(?req, "failed to fetch payload: {err:#}");
                None
            },
        }
    }
}

#[async_trait]
impl<P, S> Provider<SeqTypes, VidCommonRequest> for LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    async fn fetch(&self, req: VidCommonRequest) -> Option<VidCommon> {
        match self.fetch_vid_common(BlockId::PayloadHash(req.0)).await {
            Ok(vid) => Some(vid.common),
            Err(err) => {
                tracing::warn!(?req, "failed to fetch VID common: {err:#}");
                None
            },
        }
    }
}

#[async_trait]
impl<P, S> Provider<SeqTypes, LeafRangeRequest> for LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    async fn fetch(&self, req: LeafRangeRequest) -> Option<NonEmptyRange<LeafQueryData<SeqTypes>>> {
        let leaves = match self
            .fetch_leaves_in_range(req.start as usize, req.end as usize)
            .await
        {
            Ok(leaves) => leaves,
            Err(err) => {
                tracing::warn!(?req, "failed to fetch leaf: {err:#}");
                return None;
            },
        };
        match leaves.try_into() {
            Ok(leaves) => Some(leaves),
            Err(err) => {
                tracing::warn!(?req, "received invalid leaf range: {err:#}");
                None
            },
        }
    }
}

#[async_trait]
impl<P, S> Provider<SeqTypes, BlockRangeRequest> for LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    async fn fetch(
        &self,
        req: BlockRangeRequest,
    ) -> Option<NonEmptyRange<BlockQueryData<SeqTypes>>> {
        let blocks = match self
            .fetch_blocks_in_range(req.start as usize, req.end as usize)
            .await
        {
            Ok(blocks) => blocks,
            Err(err) => {
                tracing::warn!(?req, "failed to fetch block range: {err:#}");
                return None;
            },
        };
        match blocks.try_into() {
            Ok(blocks) => Some(blocks),
            Err(err) => {
                tracing::warn!(?req, "received invalid block range: {err:#}");
                None
            },
        }
    }
}

#[async_trait]
impl<P, S> Provider<SeqTypes, VidCommonRangeRequest> for LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    async fn fetch(
        &self,
        req: VidCommonRangeRequest,
    ) -> Option<NonEmptyRange<VidCommonQueryData<SeqTypes>>> {
        let vid_common = match self
            .fetch_vid_common_in_range(req.start as usize, req.end as usize)
            .await
        {
            Ok(vid_common) => vid_common,
            Err(err) => {
                tracing::warn!(?req, "failed to fetch VID common range: {err:#}");
                return None;
            },
        };
        match vid_common.try_into() {
            Ok(vid_common) => Some(vid_common),
            Err(err) => {
                tracing::warn!(?req, "received invalid VID common range: {err:#}");
                None
            },
        }
    }
}
