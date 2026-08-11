//! V1 node API.

use async_trait::async_trait;
use serde::Serialize;

/// Espresso's extensions to the node API.
///
/// The base node surface (block height, transaction counts, payload sizes, VID shares, sync
/// status, header windows and limits) is served by `hotshot_query_service::node::router`, which the
/// binary mounts alongside these routes; see [`crate::create_router_v1`].
#[async_trait]
pub trait NodeApiExtension {
    type StakeTable: Serialize + Send + Sync + 'static;
    type StakeTableCurrent: Serialize + Send + Sync + 'static;
    type Validators: Serialize + Send + Sync + 'static;
    type AllValidators: Serialize + Send + Sync + 'static;
    type Participation: Serialize + Send + Sync + 'static;
    type BlockReward: Serialize + Send + Sync + 'static;
    type Block: Serialize + Send + Sync + 'static;
    type Leaf: Serialize + Send + Sync + 'static;

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
