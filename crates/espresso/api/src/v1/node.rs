//! V1 node API.
//!
//! Mirrors the tide-disco endpoints defined in `hotshot-query-service/api/node.toml`
//! and `crates/espresso/node/api/node.toml`.

use async_trait::async_trait;
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum VidShareId {
    Height(u64),
    Hash(String),
    PayloadHash(String),
}

#[derive(Debug, Clone)]
pub enum HeaderWindowStart {
    Time(u64),
    Height(u64),
    Hash(String),
}

#[async_trait]
pub trait NodeApi {
    type VidShare: Serialize + Send + Sync + 'static;
    type SyncStatus: Serialize + Send + Sync + 'static;
    type HeaderWindow: Serialize + Send + Sync + 'static;
    type Limits: Serialize + Send + Sync + 'static;
    type StakeTable: Serialize + Send + Sync + 'static;
    type StakeTableCurrent: Serialize + Send + Sync + 'static;
    type Validators: Serialize + Send + Sync + 'static;
    type AllValidators: Serialize + Send + Sync + 'static;
    type Participation: Serialize + Send + Sync + 'static;
    type BlockReward: Serialize + Send + Sync + 'static;
    type Block: Serialize + Send + Sync + 'static;
    type Leaf: Serialize + Send + Sync + 'static;

    async fn block_height(&self) -> anyhow::Result<u64>;

    async fn count_transactions(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u32>,
    ) -> anyhow::Result<u64>;

    async fn payload_size(
        &self,
        from: Option<u64>,
        to: Option<u64>,
        namespace: Option<u32>,
    ) -> anyhow::Result<u64>;

    async fn get_vid_share(&self, id: VidShareId) -> anyhow::Result<Self::VidShare>;

    async fn sync_status(&self) -> anyhow::Result<Self::SyncStatus>;

    async fn get_header_window(
        &self,
        start: HeaderWindowStart,
        end: u64,
    ) -> anyhow::Result<Self::HeaderWindow>;

    async fn limits(&self) -> anyhow::Result<Self::Limits>;

    async fn stake_table(&self, epoch: u64) -> anyhow::Result<Self::StakeTable>;
    async fn stake_table_current(&self) -> anyhow::Result<Self::StakeTableCurrent>;
    async fn da_stake_table(&self, epoch: u64) -> anyhow::Result<Self::StakeTable>;
    async fn da_stake_table_current(&self) -> anyhow::Result<Self::StakeTableCurrent>;

    async fn get_validators(&self, epoch: u64) -> anyhow::Result<Self::Validators>;
    async fn get_all_validators(
        &self,
        epoch: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Self::AllValidators>;

    async fn current_proposal_participation(&self) -> anyhow::Result<Self::Participation>;
    async fn proposal_participation(&self, epoch: u64) -> anyhow::Result<Self::Participation>;
    async fn current_vote_participation(&self) -> anyhow::Result<Self::Participation>;
    async fn vote_participation(&self, epoch: u64) -> anyhow::Result<Self::Participation>;

    async fn get_block_reward(&self, epoch: Option<u64>) -> anyhow::Result<Self::BlockReward>;

    async fn get_oldest_block(&self) -> anyhow::Result<Option<Self::Block>>;
    async fn get_oldest_leaf(&self) -> anyhow::Result<Option<Self::Leaf>>;
}
