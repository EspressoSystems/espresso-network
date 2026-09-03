//! Functionality for using the [`LightClient`] as a query service fetching [`Provider`].

use async_trait::async_trait;
use espresso_types::{Certificate2, Payload, SeqTypes};
use hotshot_query_service::{
    availability::{BlockQueryData, LeafId, LeafQueryData, VidCommonQueryData},
    fetching::{
        NonEmptyRange, Provider,
        request::{
            BlockBatchRequest, BlockBatchResponse, BlockRangeRequest, Certificate2Request,
            LeafBatchRequest, LeafRangeRequest, LeafRequest, PayloadRequest, VidCommonBatchRequest,
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

#[async_trait]
impl<P, S> Provider<SeqTypes, Certificate2Request> for LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    async fn fetch(&self, req: Certificate2Request) -> Option<Option<Certificate2<SeqTypes>>> {
        match self.fetch_certificate2(req.height).await {
            Ok(cert2) => Some(cert2),
            Err(err) => {
                tracing::info!(?req, %err, "failed to fetch cert2, will retry");
                None
            },
        }
    }
}

// Leaves are fetched in one batch request and verified per contiguous run, since each run needs one
// leaf whose finality is proven and the rest chain to it.
//
// Blocks and VID arrive as payload proofs, fetched in one batch request too. Each proof verifies
// the block and its VID common together, so the block batch returns both and the VID pass only has
// to cover heights where the block was already present.
#[async_trait]
impl<P, S> Provider<SeqTypes, LeafBatchRequest> for LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    async fn fetch(&self, req: LeafBatchRequest) -> Option<Vec<LeafQueryData<SeqTypes>>> {
        match self.fetch_leaves_for_ranges(&req.0).await {
            Ok(leaves) => Some(leaves),
            Err(err) => {
                tracing::warn!(?req, "failed to fetch leaf batch: {err:#}");
                None
            },
        }
    }
}

#[async_trait]
impl<P, S> Provider<SeqTypes, BlockBatchRequest> for LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    async fn fetch(&self, req: BlockBatchRequest) -> Option<BlockBatchResponse<SeqTypes>> {
        // One range is a range fetch, whose endpoints are cacheable GETs.
        let fetched = match req.0.as_slice() {
            [range] => {
                self.fetch_blocks_and_vid_common_in_range(range.start as usize, range.end as usize)
                    .await
            },
            _ => self.fetch_blocks_and_vid_common_for_ranges(&req.0).await,
        };
        match fetched {
            Ok(fetched) => {
                let (blocks, vid_common) = fetched.into_iter().unzip();
                Some(BlockBatchResponse { blocks, vid_common })
            },
            Err(err) => {
                tracing::warn!(?req, "failed to fetch block batch: {err:#}");
                None
            },
        }
    }
}

#[async_trait]
impl<P, S> Provider<SeqTypes, VidCommonBatchRequest> for LightClient<P, S>
where
    P: Storage,
    S: Client,
{
    async fn fetch(&self, req: VidCommonBatchRequest) -> Option<Vec<VidCommonQueryData<SeqTypes>>> {
        let fetched = match req.0.as_slice() {
            [range] => {
                self.fetch_blocks_and_vid_common_in_range(range.start as usize, range.end as usize)
                    .await
            },
            _ => self.fetch_blocks_and_vid_common_for_ranges(&req.0).await,
        };
        match fetched {
            Ok(fetched) => Some(fetched.into_iter().map(|(_, common)| common).collect()),
            Err(err) => {
                tracing::warn!(?req, "failed to fetch VID common batch: {err:#}");
                None
            },
        }
    }
}

#[cfg(test)]
mod test {
    use hotshot_query_service::types::HeightIndexed;

    use super::*;
    use crate::{storage::SqliteStorage, testing::TestClient};

    #[tokio::test]
    #[test_log::test]
    async fn test_block_batch_carries_vid_common() {
        let client = TestClient::default();
        let lc = LightClient::from_genesis(
            SqliteStorage::default().await.unwrap(),
            client.clone(),
            client.genesis().await,
        );
        for height in 1..8 {
            client.payload(height).await;
        }

        let batch = Provider::<SeqTypes, BlockBatchRequest>::fetch(
            &lc,
            BlockBatchRequest(vec![1..3, 5..8]),
        )
        .await
        .unwrap();

        assert_eq!(
            batch
                .blocks
                .iter()
                .map(|block| block.height())
                .collect::<Vec<_>>(),
            [1, 2, 5, 6, 7]
        );
        assert_eq!(batch.vid_common.len(), batch.blocks.len());
        for (block, common) in batch.blocks.iter().zip(&batch.vid_common) {
            assert_eq!(common.height(), block.height());
            assert_eq!(common.payload_hash(), block.payload_hash());
        }
    }
}
