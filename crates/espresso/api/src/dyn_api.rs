//! Type-erased mirrors of the [`crate::v1`] API traits.
//!
//! The `router_*` builders in [`crate::axum`] used to be generic over the state type, so every
//! handler closure, and the aide/axum machinery behind it, was codegened once per concrete state
//! (three in the node binary, plus the test mock). The traits here repeat the `v1` traits with
//! their associated types replaced by [`Erased`], which makes them object safe: the routers take
//! an `Arc<dyn ...>` and are compiled once. Cost at runtime is one virtual call and one box per
//! response.
//!
//! Each trait has a blanket impl forwarding to the corresponding `v1` trait, so every state type
//! gets the erased view for free, and only those forwarders are monomorphized per state.

use std::sync::Arc;

use async_trait::async_trait;
use axum::http::HeaderMap;
use futures::stream::{BoxStream, StreamExt};

use crate::{
    axum::decode_body,
    error::{ApiError, classify},
    v1,
};

/// A response value whose type has been erased. `Box<dyn erased_serde::Serialize>` serializes
/// exactly like the value it holds, so JSON and VBS bodies are unchanged.
pub(crate) type Erased = Box<dyn erased_serde::Serialize + Send + Sync>;

fn erase<T: serde::Serialize + Send + Sync + 'static>(value: T) -> Erased {
    Box::new(value)
}

// The state types the `router_*` builders take, one per router.
pub(crate) type RewardState = Arc<dyn DynRewardApi>;
pub(crate) type AvailabilityState = Arc<dyn DynAvailability>;
pub(crate) type BlockState = Arc<dyn DynBlockStateApi>;
pub(crate) type FeeState = Arc<dyn DynFeeStateApi>;
pub(crate) type StatusState = Arc<dyn DynStatusApi>;
pub(crate) type ConfigState = Arc<dyn DynConfigApi>;
pub(crate) type NodeState = Arc<dyn DynNodeApi>;
pub(crate) type CatchupState = Arc<dyn DynCatchupApi>;
pub(crate) type SubmitState = Arc<dyn DynSubmitApi>;
pub(crate) type StateSignatureState = Arc<dyn DynStateSignatureApi>;
pub(crate) type HotShotEventsState = Arc<dyn DynHotShotEventsApi>;
pub(crate) type LightClientState = Arc<dyn DynLightClientApi>;
pub(crate) type ExplorerState = Arc<dyn DynExplorerApi>;
pub(crate) type DatabaseState = Arc<dyn DynDatabaseApi>;
/// [`v1::TokenApi`] has no associated types, so it is already object safe.
pub(crate) type TokenState = Arc<dyn v1::TokenApi + Send + Sync>;

#[async_trait]
pub(crate) trait DynRewardApi: Send + Sync {
    async fn get_reward_state_height(&self) -> anyhow::Result<u64>;
    async fn get_reward_state_v2_height(&self) -> anyhow::Result<u64>;
    async fn get_reward_account_proof_v1(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Erased>;
    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: String,
    ) -> anyhow::Result<Erased>;
    async fn get_reward_balance(&self, height: u64, address: String) -> anyhow::Result<Erased>;
    async fn get_latest_reward_balance(&self, address: String) -> anyhow::Result<Erased>;
    async fn get_reward_account_proof(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Erased>;
    async fn get_latest_reward_account_proof(&self, address: String) -> anyhow::Result<Erased>;
    async fn get_reward_amounts(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Erased>;
    async fn get_reward_merkle_tree_v2(&self, height: u64) -> anyhow::Result<Erased>;
    async fn get_reward_state_path_v1(
        &self,
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Erased>;
    async fn get_reward_state_path_v2(
        &self,
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::RewardApi + Send + Sync> DynRewardApi for T {
    async fn get_reward_state_height(&self) -> anyhow::Result<u64> {
        v1::RewardApi::get_reward_state_height(self).await
    }
    async fn get_reward_state_v2_height(&self) -> anyhow::Result<u64> {
        v1::RewardApi::get_reward_state_v2_height(self).await
    }
    async fn get_reward_account_proof_v1(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Erased> {
        v1::RewardApi::get_reward_account_proof_v1(self, height, address)
            .await
            .map(erase)
    }
    async fn get_reward_claim_input(
        &self,
        block_height: u64,
        address: String,
    ) -> anyhow::Result<Erased> {
        v1::RewardApi::get_reward_claim_input(self, block_height, address)
            .await
            .map(erase)
    }
    async fn get_reward_balance(&self, height: u64, address: String) -> anyhow::Result<Erased> {
        v1::RewardApi::get_reward_balance(self, height, address)
            .await
            .map(erase)
    }
    async fn get_latest_reward_balance(&self, address: String) -> anyhow::Result<Erased> {
        v1::RewardApi::get_latest_reward_balance(self, address)
            .await
            .map(erase)
    }
    async fn get_reward_account_proof(
        &self,
        height: u64,
        address: String,
    ) -> anyhow::Result<Erased> {
        v1::RewardApi::get_reward_account_proof(self, height, address)
            .await
            .map(erase)
    }
    async fn get_latest_reward_account_proof(&self, address: String) -> anyhow::Result<Erased> {
        v1::RewardApi::get_latest_reward_account_proof(self, address)
            .await
            .map(erase)
    }
    async fn get_reward_amounts(
        &self,
        height: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Erased> {
        v1::RewardApi::get_reward_amounts(self, height, offset, limit)
            .await
            .map(erase)
    }
    async fn get_reward_merkle_tree_v2(&self, height: u64) -> anyhow::Result<Erased> {
        v1::RewardApi::get_reward_merkle_tree_v2(self, height)
            .await
            .map(erase)
    }
    async fn get_reward_state_path_v1(
        &self,
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Erased> {
        v1::RewardApi::get_reward_state_path_v1(self, snapshot, key)
            .await
            .map(erase)
    }
    async fn get_reward_state_path_v2(
        &self,
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Erased> {
        v1::RewardApi::get_reward_state_path_v2(self, snapshot, key)
            .await
            .map(erase)
    }
}

/// The availability router serves both availability traits from one state object.
pub(crate) trait DynAvailability: DynAvailabilityApi + DynHotShotAvailabilityApi {}
impl<T: DynAvailabilityApi + DynHotShotAvailabilityApi> DynAvailability for T {}

#[async_trait]
pub(crate) trait DynAvailabilityApi: Send + Sync {
    async fn get_namespace_proof(
        &self,
        block_id: v1::BlockId,
        namespace: u32,
    ) -> anyhow::Result<Erased>;
    async fn get_namespace_proof_range(
        &self,
        from: u64,
        until: u64,
        namespace: u32,
    ) -> anyhow::Result<Erased>;
    async fn stream_namespace_proofs(
        &self,
        from: usize,
        namespace: u32,
    ) -> anyhow::Result<BoxStream<'static, Erased>>;
    async fn get_incorrect_encoding_proof(
        &self,
        block_id: v1::BlockId,
        namespace: u32,
    ) -> anyhow::Result<Erased>;
    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Erased>;
    async fn get_state_cert_v2(&self, epoch: u64) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::AvailabilityApi + Send + Sync> DynAvailabilityApi for T {
    async fn get_namespace_proof(
        &self,
        block_id: v1::BlockId,
        namespace: u32,
    ) -> anyhow::Result<Erased> {
        v1::AvailabilityApi::get_namespace_proof(self, block_id, namespace)
            .await
            .map(erase)
    }
    async fn get_namespace_proof_range(
        &self,
        from: u64,
        until: u64,
        namespace: u32,
    ) -> anyhow::Result<Erased> {
        v1::AvailabilityApi::get_namespace_proof_range(self, from, until, namespace)
            .await
            .map(erase)
    }
    async fn stream_namespace_proofs(
        &self,
        from: usize,
        namespace: u32,
    ) -> anyhow::Result<BoxStream<'static, Erased>> {
        Ok(
            v1::AvailabilityApi::stream_namespace_proofs(self, from, namespace)
                .await?
                .map(erase)
                .boxed(),
        )
    }
    async fn get_incorrect_encoding_proof(
        &self,
        block_id: v1::BlockId,
        namespace: u32,
    ) -> anyhow::Result<Erased> {
        v1::AvailabilityApi::get_incorrect_encoding_proof(self, block_id, namespace)
            .await
            .map(erase)
    }
    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Erased> {
        v1::AvailabilityApi::get_state_cert(self, epoch)
            .await
            .map(erase)
    }
    async fn get_state_cert_v2(&self, epoch: u64) -> anyhow::Result<Erased> {
        v1::AvailabilityApi::get_state_cert_v2(self, epoch)
            .await
            .map(erase)
    }
}

#[async_trait]
pub(crate) trait DynHotShotAvailabilityApi: Send + Sync {
    async fn get_leaf(&self, id: v1::LeafId) -> anyhow::Result<Erased>;
    async fn get_leaf_range(&self, from: usize, until: usize) -> anyhow::Result<Erased>;
    async fn get_header(&self, id: v1::BlockId) -> anyhow::Result<Erased>;
    async fn get_header_range(&self, from: usize, until: usize) -> anyhow::Result<Erased>;
    async fn get_block(&self, id: v1::BlockId) -> anyhow::Result<Erased>;
    async fn get_block_range(&self, from: usize, until: usize) -> anyhow::Result<Erased>;
    async fn get_payload(&self, id: v1::PayloadId) -> anyhow::Result<Erased>;
    async fn get_payload_range(&self, from: usize, until: usize) -> anyhow::Result<Erased>;
    async fn get_vid_common(&self, id: v1::BlockId) -> anyhow::Result<Erased>;
    async fn get_vid_common_range(&self, from: usize, until: usize) -> anyhow::Result<Erased>;
    async fn get_transaction_by_position(&self, height: u64, index: u64) -> anyhow::Result<Erased>;
    async fn get_transaction_by_hash(&self, hash: String) -> anyhow::Result<Erased>;
    async fn get_transaction_proof_by_position(
        &self,
        height: u64,
        index: u64,
    ) -> anyhow::Result<Erased>;
    async fn get_transaction_proof_by_hash(&self, hash: String) -> anyhow::Result<Erased>;
    async fn get_block_summary(&self, height: usize) -> anyhow::Result<Erased>;
    async fn get_block_summary_range(&self, from: usize, until: usize) -> anyhow::Result<Erased>;
    async fn get_limits(&self) -> anyhow::Result<Erased>;
    /// `None` is a 404 rather than a body, so the option survives erasure.
    async fn get_cert2(&self, height: u64) -> anyhow::Result<Option<Erased>>;
    async fn stream_leaves(&self, from: usize) -> anyhow::Result<BoxStream<'static, Erased>>;
    async fn stream_headers(&self, from: usize) -> anyhow::Result<BoxStream<'static, Erased>>;
    async fn stream_blocks(&self, from: usize) -> anyhow::Result<BoxStream<'static, Erased>>;
    async fn stream_payloads(&self, from: usize) -> anyhow::Result<BoxStream<'static, Erased>>;
    async fn stream_vid_common(&self, from: usize) -> anyhow::Result<BoxStream<'static, Erased>>;
    async fn stream_transactions(
        &self,
        from: usize,
        namespace: Option<u32>,
    ) -> anyhow::Result<BoxStream<'static, Erased>>;
}

#[async_trait]
impl<T: v1::HotShotAvailabilityApi + Send + Sync> DynHotShotAvailabilityApi for T {
    async fn get_leaf(&self, id: v1::LeafId) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_leaf(self, id)
            .await
            .map(erase)
    }
    async fn get_leaf_range(&self, from: usize, until: usize) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_leaf_range(self, from, until)
            .await
            .map(erase)
    }
    async fn get_header(&self, id: v1::BlockId) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_header(self, id)
            .await
            .map(erase)
    }
    async fn get_header_range(&self, from: usize, until: usize) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_header_range(self, from, until)
            .await
            .map(erase)
    }
    async fn get_block(&self, id: v1::BlockId) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_block(self, id)
            .await
            .map(erase)
    }
    async fn get_block_range(&self, from: usize, until: usize) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_block_range(self, from, until)
            .await
            .map(erase)
    }
    async fn get_payload(&self, id: v1::PayloadId) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_payload(self, id)
            .await
            .map(erase)
    }
    async fn get_payload_range(&self, from: usize, until: usize) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_payload_range(self, from, until)
            .await
            .map(erase)
    }
    async fn get_vid_common(&self, id: v1::BlockId) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_vid_common(self, id)
            .await
            .map(erase)
    }
    async fn get_vid_common_range(&self, from: usize, until: usize) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_vid_common_range(self, from, until)
            .await
            .map(erase)
    }
    async fn get_transaction_by_position(&self, height: u64, index: u64) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_transaction_by_position(self, height, index)
            .await
            .map(erase)
    }
    async fn get_transaction_by_hash(&self, hash: String) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_transaction_by_hash(self, hash)
            .await
            .map(erase)
    }
    async fn get_transaction_proof_by_position(
        &self,
        height: u64,
        index: u64,
    ) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_transaction_proof_by_position(self, height, index)
            .await
            .map(erase)
    }
    async fn get_transaction_proof_by_hash(&self, hash: String) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_transaction_proof_by_hash(self, hash)
            .await
            .map(erase)
    }
    async fn get_block_summary(&self, height: usize) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_block_summary(self, height)
            .await
            .map(erase)
    }
    async fn get_block_summary_range(&self, from: usize, until: usize) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_block_summary_range(self, from, until)
            .await
            .map(erase)
    }
    async fn get_limits(&self) -> anyhow::Result<Erased> {
        v1::HotShotAvailabilityApi::get_limits(self)
            .await
            .map(erase)
    }
    async fn get_cert2(&self, height: u64) -> anyhow::Result<Option<Erased>> {
        Ok(v1::HotShotAvailabilityApi::get_cert2(self, height)
            .await?
            .map(erase))
    }
    async fn stream_leaves(&self, from: usize) -> anyhow::Result<BoxStream<'static, Erased>> {
        Ok(v1::HotShotAvailabilityApi::stream_leaves(self, from)
            .await?
            .map(erase)
            .boxed())
    }
    async fn stream_headers(&self, from: usize) -> anyhow::Result<BoxStream<'static, Erased>> {
        Ok(v1::HotShotAvailabilityApi::stream_headers(self, from)
            .await?
            .map(erase)
            .boxed())
    }
    async fn stream_blocks(&self, from: usize) -> anyhow::Result<BoxStream<'static, Erased>> {
        Ok(v1::HotShotAvailabilityApi::stream_blocks(self, from)
            .await?
            .map(erase)
            .boxed())
    }
    async fn stream_payloads(&self, from: usize) -> anyhow::Result<BoxStream<'static, Erased>> {
        Ok(v1::HotShotAvailabilityApi::stream_payloads(self, from)
            .await?
            .map(erase)
            .boxed())
    }
    async fn stream_vid_common(&self, from: usize) -> anyhow::Result<BoxStream<'static, Erased>> {
        Ok(v1::HotShotAvailabilityApi::stream_vid_common(self, from)
            .await?
            .map(erase)
            .boxed())
    }
    async fn stream_transactions(
        &self,
        from: usize,
        namespace: Option<u32>,
    ) -> anyhow::Result<BoxStream<'static, Erased>> {
        Ok(
            v1::HotShotAvailabilityApi::stream_transactions(self, from, namespace)
                .await?
                .map(erase)
                .boxed(),
        )
    }
}

#[async_trait]
pub(crate) trait DynBlockStateApi: Send + Sync {
    async fn get_block_state_path(
        &self,
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Erased>;
    async fn get_block_state_height(&self) -> anyhow::Result<u64>;
}

#[async_trait]
impl<T: v1::BlockStateApi + Send + Sync> DynBlockStateApi for T {
    async fn get_block_state_path(
        &self,
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Erased> {
        v1::BlockStateApi::get_block_state_path(self, snapshot, key)
            .await
            .map(erase)
    }
    async fn get_block_state_height(&self) -> anyhow::Result<u64> {
        v1::BlockStateApi::get_block_state_height(self).await
    }
}

#[async_trait]
pub(crate) trait DynFeeStateApi: Send + Sync {
    async fn get_fee_state_path(
        &self,
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Erased>;
    async fn get_fee_state_height(&self) -> anyhow::Result<u64>;
    async fn get_fee_balance_latest(&self, address: String) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::FeeStateApi + Send + Sync> DynFeeStateApi for T {
    async fn get_fee_state_path(
        &self,
        snapshot: v1::Snapshot,
        key: String,
    ) -> anyhow::Result<Erased> {
        v1::FeeStateApi::get_fee_state_path(self, snapshot, key)
            .await
            .map(erase)
    }
    async fn get_fee_state_height(&self) -> anyhow::Result<u64> {
        v1::FeeStateApi::get_fee_state_height(self).await
    }
    async fn get_fee_balance_latest(&self, address: String) -> anyhow::Result<Erased> {
        v1::FeeStateApi::get_fee_balance_latest(self, address)
            .await
            .map(erase)
    }
}

#[async_trait]
pub(crate) trait DynStatusApi: Send + Sync {
    async fn block_height(&self) -> anyhow::Result<u64>;
    async fn success_rate(&self) -> anyhow::Result<f64>;
    async fn time_since_last_decide(&self) -> anyhow::Result<u64>;
    async fn metrics(&self) -> anyhow::Result<String>;
    async fn keys(&self) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::StatusApi + Send + Sync> DynStatusApi for T {
    async fn block_height(&self) -> anyhow::Result<u64> {
        v1::StatusApi::block_height(self).await
    }
    async fn success_rate(&self) -> anyhow::Result<f64> {
        v1::StatusApi::success_rate(self).await
    }
    async fn time_since_last_decide(&self) -> anyhow::Result<u64> {
        v1::StatusApi::time_since_last_decide(self).await
    }
    async fn metrics(&self) -> anyhow::Result<String> {
        v1::StatusApi::metrics(self).await
    }
    async fn keys(&self) -> anyhow::Result<Erased> {
        v1::StatusApi::keys(self).await.map(erase)
    }
}

#[async_trait]
pub(crate) trait DynConfigApi: Send + Sync {
    async fn hotshot_config(&self) -> anyhow::Result<Erased>;
    async fn env(&self) -> anyhow::Result<Vec<String>>;
    async fn runtime_config(&self) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::ConfigApi + Send + Sync> DynConfigApi for T {
    async fn hotshot_config(&self) -> anyhow::Result<Erased> {
        v1::ConfigApi::hotshot_config(self).await.map(erase)
    }
    async fn env(&self) -> anyhow::Result<Vec<String>> {
        v1::ConfigApi::env(self).await
    }
    async fn runtime_config(&self) -> anyhow::Result<Erased> {
        v1::ConfigApi::runtime_config(self).await.map(erase)
    }
}

#[async_trait]
pub(crate) trait DynNodeApi: Send + Sync {
    async fn block_height(&self) -> anyhow::Result<u64>;
    async fn count_transactions(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u64>,
    ) -> anyhow::Result<u64>;
    async fn payload_size(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u64>,
    ) -> anyhow::Result<u64>;
    async fn get_vid_share(&self, id: v1::VidShareId) -> anyhow::Result<Erased>;
    async fn sync_status(&self) -> anyhow::Result<Erased>;
    async fn get_header_window(
        &self,
        start: v1::HeaderWindowStart,
        end: u64,
    ) -> anyhow::Result<Erased>;
    async fn limits(&self) -> anyhow::Result<Erased>;
    async fn stake_table(&self, epoch: u64) -> anyhow::Result<Erased>;
    async fn stake_table_current(&self) -> anyhow::Result<Erased>;
    async fn da_stake_table(&self, epoch: u64) -> anyhow::Result<Erased>;
    async fn da_stake_table_current(&self) -> anyhow::Result<Erased>;
    async fn get_validators(&self, epoch: u64) -> anyhow::Result<Erased>;
    async fn get_all_validators(
        &self,
        epoch: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Erased>;
    async fn current_proposal_participation(&self) -> anyhow::Result<Erased>;
    async fn proposal_participation(&self, epoch: u64) -> anyhow::Result<Erased>;
    async fn current_vote_participation(&self) -> anyhow::Result<Erased>;
    async fn vote_participation(&self, epoch: u64) -> anyhow::Result<Erased>;
    async fn get_block_reward(&self, epoch: Option<u64>) -> anyhow::Result<Erased>;
    async fn get_oldest_block(&self) -> anyhow::Result<Erased>;
    async fn get_oldest_leaf(&self) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::NodeApi + Send + Sync> DynNodeApi for T {
    async fn block_height(&self) -> anyhow::Result<u64> {
        v1::NodeApi::block_height(self).await
    }
    async fn count_transactions(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u64>,
    ) -> anyhow::Result<u64> {
        v1::NodeApi::count_transactions(self, from, to, namespace).await
    }
    async fn payload_size(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u64>,
    ) -> anyhow::Result<u64> {
        v1::NodeApi::payload_size(self, from, to, namespace).await
    }
    async fn get_vid_share(&self, id: v1::VidShareId) -> anyhow::Result<Erased> {
        v1::NodeApi::get_vid_share(self, id).await.map(erase)
    }
    async fn sync_status(&self) -> anyhow::Result<Erased> {
        v1::NodeApi::sync_status(self).await.map(erase)
    }
    async fn get_header_window(
        &self,
        start: v1::HeaderWindowStart,
        end: u64,
    ) -> anyhow::Result<Erased> {
        v1::NodeApi::get_header_window(self, start, end)
            .await
            .map(erase)
    }
    async fn limits(&self) -> anyhow::Result<Erased> {
        v1::NodeApi::limits(self).await.map(erase)
    }
    async fn stake_table(&self, epoch: u64) -> anyhow::Result<Erased> {
        v1::NodeApi::stake_table(self, epoch).await.map(erase)
    }
    async fn stake_table_current(&self) -> anyhow::Result<Erased> {
        v1::NodeApi::stake_table_current(self).await.map(erase)
    }
    async fn da_stake_table(&self, epoch: u64) -> anyhow::Result<Erased> {
        v1::NodeApi::da_stake_table(self, epoch).await.map(erase)
    }
    async fn da_stake_table_current(&self) -> anyhow::Result<Erased> {
        v1::NodeApi::da_stake_table_current(self).await.map(erase)
    }
    async fn get_validators(&self, epoch: u64) -> anyhow::Result<Erased> {
        v1::NodeApi::get_validators(self, epoch).await.map(erase)
    }
    async fn get_all_validators(
        &self,
        epoch: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Erased> {
        v1::NodeApi::get_all_validators(self, epoch, offset, limit)
            .await
            .map(erase)
    }
    async fn current_proposal_participation(&self) -> anyhow::Result<Erased> {
        v1::NodeApi::current_proposal_participation(self)
            .await
            .map(erase)
    }
    async fn proposal_participation(&self, epoch: u64) -> anyhow::Result<Erased> {
        v1::NodeApi::proposal_participation(self, epoch)
            .await
            .map(erase)
    }
    async fn current_vote_participation(&self) -> anyhow::Result<Erased> {
        v1::NodeApi::current_vote_participation(self)
            .await
            .map(erase)
    }
    async fn vote_participation(&self, epoch: u64) -> anyhow::Result<Erased> {
        v1::NodeApi::vote_participation(self, epoch)
            .await
            .map(erase)
    }
    async fn get_block_reward(&self, epoch: Option<u64>) -> anyhow::Result<Erased> {
        v1::NodeApi::get_block_reward(self, epoch).await.map(erase)
    }
    async fn get_oldest_block(&self) -> anyhow::Result<Erased> {
        v1::NodeApi::get_oldest_block(self).await.map(erase)
    }
    async fn get_oldest_leaf(&self) -> anyhow::Result<Erased> {
        v1::NodeApi::get_oldest_leaf(self).await.map(erase)
    }
}

#[async_trait]
pub(crate) trait DynCatchupApi: Send + Sync {
    async fn get_account(&self, height: u64, view: u64, address: String) -> anyhow::Result<Erased>;
    /// The account list is decoded from the request body and its element type is erased, so the
    /// decode happens behind the trait object. Errors match the typed path: a bad body is the
    /// 400 [`decode_body`] returns, a handler failure goes through [`classify`].
    async fn get_accounts(
        &self,
        height: u64,
        view: u64,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Erased, ApiError>;
    async fn get_blocks_frontier(&self, height: u64, view: u64) -> anyhow::Result<Erased>;
    async fn get_chain_config(&self, commitment: String) -> anyhow::Result<Erased>;
    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Erased>;
    async fn get_cert2(&self, height: u64) -> anyhow::Result<Erased>;
    async fn get_reward_account_v1(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Erased>;
    async fn get_reward_accounts_v1(
        &self,
        height: u64,
        view: u64,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Erased, ApiError>;
    async fn get_reward_account_v2(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Erased>;
    async fn get_reward_merkle_tree_v2(&self, height: u64, view: u64) -> anyhow::Result<Erased>;
    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::CatchupApi + Send + Sync> DynCatchupApi for T {
    async fn get_account(&self, height: u64, view: u64, address: String) -> anyhow::Result<Erased> {
        v1::CatchupApi::get_account(self, height, view, address)
            .await
            .map(erase)
    }
    async fn get_accounts(
        &self,
        height: u64,
        view: u64,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Erased, ApiError> {
        let accounts: Vec<<T as v1::CatchupApi>::FeeAccount> = decode_body(headers, body)?;
        v1::CatchupApi::get_accounts(self, height, view, accounts)
            .await
            .map(erase)
            .map_err(classify)
    }
    async fn get_blocks_frontier(&self, height: u64, view: u64) -> anyhow::Result<Erased> {
        v1::CatchupApi::get_blocks_frontier(self, height, view)
            .await
            .map(erase)
    }
    async fn get_chain_config(&self, commitment: String) -> anyhow::Result<Erased> {
        v1::CatchupApi::get_chain_config(self, commitment)
            .await
            .map(erase)
    }
    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Erased> {
        v1::CatchupApi::get_leaf_chain(self, height)
            .await
            .map(erase)
    }
    async fn get_cert2(&self, height: u64) -> anyhow::Result<Erased> {
        v1::CatchupApi::get_cert2(self, height).await.map(erase)
    }
    async fn get_reward_account_v1(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Erased> {
        v1::CatchupApi::get_reward_account_v1(self, height, view, address)
            .await
            .map(erase)
    }
    async fn get_reward_accounts_v1(
        &self,
        height: u64,
        view: u64,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Result<Erased, ApiError> {
        let accounts: Vec<<T as v1::CatchupApi>::RewardAccountV1> = decode_body(headers, body)?;
        v1::CatchupApi::get_reward_accounts_v1(self, height, view, accounts)
            .await
            .map(erase)
            .map_err(classify)
    }
    async fn get_reward_account_v2(
        &self,
        height: u64,
        view: u64,
        address: String,
    ) -> anyhow::Result<Erased> {
        v1::CatchupApi::get_reward_account_v2(self, height, view, address)
            .await
            .map(erase)
    }
    async fn get_reward_merkle_tree_v2(&self, height: u64, view: u64) -> anyhow::Result<Erased> {
        v1::CatchupApi::get_reward_merkle_tree_v2(self, height, view)
            .await
            .map(erase)
    }
    async fn get_state_cert(&self, epoch: u64) -> anyhow::Result<Erased> {
        v1::CatchupApi::get_state_cert(self, epoch).await.map(erase)
    }
}

#[async_trait]
pub(crate) trait DynSubmitApi: Send + Sync {
    /// The transaction type is erased, so the body is decoded behind the trait object; a bad
    /// body is the 400 [`decode_body`] returns and a submit failure a 500, as on the typed path.
    async fn submit(&self, headers: &HeaderMap, body: &[u8]) -> Result<Erased, ApiError>;
}

#[async_trait]
impl<T: v1::SubmitApi + Send + Sync> DynSubmitApi for T {
    async fn submit(&self, headers: &HeaderMap, body: &[u8]) -> Result<Erased, ApiError> {
        let tx: <T as v1::SubmitApi>::Transaction = decode_body(headers, body)?;
        v1::SubmitApi::submit(self, tx)
            .await
            .map(erase)
            .map_err(ApiError::Internal)
    }
}

#[async_trait]
pub(crate) trait DynStateSignatureApi: Send + Sync {
    async fn get_state_signature(&self, height: u64) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::StateSignatureApi + Send + Sync> DynStateSignatureApi for T {
    async fn get_state_signature(&self, height: u64) -> anyhow::Result<Erased> {
        v1::StateSignatureApi::get_state_signature(self, height)
            .await
            .map(erase)
    }
}

#[async_trait]
pub(crate) trait DynHotShotEventsApi: Send + Sync {
    async fn startup_info(&self) -> anyhow::Result<Erased>;
    async fn events(&self) -> anyhow::Result<BoxStream<'static, Erased>>;
}

#[async_trait]
impl<T: v1::HotShotEventsApi + Send + Sync> DynHotShotEventsApi for T {
    async fn startup_info(&self) -> anyhow::Result<Erased> {
        v1::HotShotEventsApi::startup_info(self).await.map(erase)
    }
    async fn events(&self) -> anyhow::Result<BoxStream<'static, Erased>> {
        Ok(v1::HotShotEventsApi::events(self).await?.map(erase).boxed())
    }
}

#[async_trait]
pub(crate) trait DynLightClientApi: Send + Sync {
    async fn get_leaf_proof(
        &self,
        query: v1::LeafQuery,
        finalized: Option<u64>,
    ) -> anyhow::Result<Erased>;
    async fn get_header_proof(
        &self,
        root: u64,
        requested: v1::HeaderQuery,
    ) -> anyhow::Result<Erased>;
    async fn get_light_client_stake_table(&self, epoch: u64) -> anyhow::Result<Erased>;
    async fn get_payload_proof(&self, height: u64) -> anyhow::Result<Erased>;
    async fn get_payload_proof_range(&self, start: u64, end: u64) -> anyhow::Result<Erased>;
    async fn get_lc_namespace_proof(&self, height: u64, namespace: u64) -> anyhow::Result<Erased>;
    async fn get_lc_namespace_proof_range(
        &self,
        start: u64,
        end: u64,
        namespace: u64,
    ) -> anyhow::Result<Erased>;
    async fn get_lc_namespaces_proof_range(
        &self,
        start: u64,
        end: u64,
        namespaces: String,
    ) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::LightClientApi + Send + Sync> DynLightClientApi for T {
    async fn get_leaf_proof(
        &self,
        query: v1::LeafQuery,
        finalized: Option<u64>,
    ) -> anyhow::Result<Erased> {
        v1::LightClientApi::get_leaf_proof(self, query, finalized)
            .await
            .map(erase)
    }
    async fn get_header_proof(
        &self,
        root: u64,
        requested: v1::HeaderQuery,
    ) -> anyhow::Result<Erased> {
        v1::LightClientApi::get_header_proof(self, root, requested)
            .await
            .map(erase)
    }
    async fn get_light_client_stake_table(&self, epoch: u64) -> anyhow::Result<Erased> {
        v1::LightClientApi::get_light_client_stake_table(self, epoch)
            .await
            .map(erase)
    }
    async fn get_payload_proof(&self, height: u64) -> anyhow::Result<Erased> {
        v1::LightClientApi::get_payload_proof(self, height)
            .await
            .map(erase)
    }
    async fn get_payload_proof_range(&self, start: u64, end: u64) -> anyhow::Result<Erased> {
        v1::LightClientApi::get_payload_proof_range(self, start, end)
            .await
            .map(erase)
    }
    async fn get_lc_namespace_proof(&self, height: u64, namespace: u64) -> anyhow::Result<Erased> {
        v1::LightClientApi::get_lc_namespace_proof(self, height, namespace)
            .await
            .map(erase)
    }
    async fn get_lc_namespace_proof_range(
        &self,
        start: u64,
        end: u64,
        namespace: u64,
    ) -> anyhow::Result<Erased> {
        v1::LightClientApi::get_lc_namespace_proof_range(self, start, end, namespace)
            .await
            .map(erase)
    }
    async fn get_lc_namespaces_proof_range(
        &self,
        start: u64,
        end: u64,
        namespaces: String,
    ) -> anyhow::Result<Erased> {
        v1::LightClientApi::get_lc_namespaces_proof_range(self, start, end, namespaces)
            .await
            .map(erase)
    }
}

#[async_trait]
pub(crate) trait DynExplorerApi: Send + Sync {
    async fn get_block_detail(&self, ident: v1::BlockIdent) -> anyhow::Result<Erased>;
    async fn get_block_summaries(
        &self,
        target: v1::BlockIdent,
        limit: u64,
    ) -> anyhow::Result<Erased>;
    async fn get_transaction_detail(&self, ident: v1::TxIdent) -> anyhow::Result<Erased>;
    async fn get_transaction_summaries(
        &self,
        target: v1::TxIdent,
        limit: u64,
        filter: v1::TxSummaryFilter,
    ) -> anyhow::Result<Erased>;
    async fn get_explorer_summary(&self) -> anyhow::Result<Erased>;
    async fn get_search_result(&self, query: String) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::ExplorerApi + Send + Sync> DynExplorerApi for T {
    async fn get_block_detail(&self, ident: v1::BlockIdent) -> anyhow::Result<Erased> {
        v1::ExplorerApi::get_block_detail(self, ident)
            .await
            .map(erase)
    }
    async fn get_block_summaries(
        &self,
        target: v1::BlockIdent,
        limit: u64,
    ) -> anyhow::Result<Erased> {
        v1::ExplorerApi::get_block_summaries(self, target, limit)
            .await
            .map(erase)
    }
    async fn get_transaction_detail(&self, ident: v1::TxIdent) -> anyhow::Result<Erased> {
        v1::ExplorerApi::get_transaction_detail(self, ident)
            .await
            .map(erase)
    }
    async fn get_transaction_summaries(
        &self,
        target: v1::TxIdent,
        limit: u64,
        filter: v1::TxSummaryFilter,
    ) -> anyhow::Result<Erased> {
        v1::ExplorerApi::get_transaction_summaries(self, target, limit, filter)
            .await
            .map(erase)
    }
    async fn get_explorer_summary(&self) -> anyhow::Result<Erased> {
        v1::ExplorerApi::get_explorer_summary(self).await.map(erase)
    }
    async fn get_search_result(&self, query: String) -> anyhow::Result<Erased> {
        v1::ExplorerApi::get_search_result(self, query)
            .await
            .map(erase)
    }
}

#[async_trait]
pub(crate) trait DynDatabaseApi: Send + Sync {
    async fn get_table_sizes(&self) -> anyhow::Result<Erased>;
    async fn get_migration_status(&self) -> anyhow::Result<Erased>;
}

#[async_trait]
impl<T: v1::DatabaseApi + Send + Sync> DynDatabaseApi for T {
    async fn get_table_sizes(&self) -> anyhow::Result<Erased> {
        v1::DatabaseApi::get_table_sizes(self).await.map(erase)
    }
    async fn get_migration_status(&self) -> anyhow::Result<Erased> {
        v1::DatabaseApi::get_migration_status(self).await.map(erase)
    }
}
