//! V1 node API.
//!
//! Mirrors the endpoints of the legacy `hotshot-query-service` node API (its tide-disco
//! `node.toml`, since removed).

use std::collections::HashMap;

use alloy::primitives::Address;
use async_trait::async_trait;
use espresso_types::{
    PubKey, SeqTypes,
    v0_3::{AuthenticatedValidator, RegisteredValidator, RewardAmount},
};
use hotshot_query_service_types::{
    Header,
    availability::{BlockQueryData, LeafQueryData},
    node::{Limits, SyncStatusQueryData, TimeWindowQueryData},
};
use hotshot_types::{
    PeerConfig,
    data::{EpochNumber, VidShare},
    traits::node_implementation::NodeType,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize)]
#[serde(bound = "T: NodeType")]
pub struct StakeTableWithEpochNumber<T: NodeType> {
    pub epoch: Option<EpochNumber>,
    pub stake_table: Vec<PeerConfig<T>>,
}

#[async_trait]
pub trait NodeApi {
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

    async fn get_vid_share(&self, id: VidShareId) -> anyhow::Result<VidShare>;

    async fn sync_status(&self) -> anyhow::Result<SyncStatusQueryData>;

    async fn get_header_window(
        &self,
        start: HeaderWindowStart,
        end: u64,
    ) -> anyhow::Result<TimeWindowQueryData<Header<SeqTypes>>>;

    async fn limits(&self) -> anyhow::Result<Limits>;

    async fn stake_table(&self, epoch: u64) -> anyhow::Result<Vec<PeerConfig<SeqTypes>>>;
    async fn stake_table_current(&self) -> anyhow::Result<StakeTableWithEpochNumber<SeqTypes>>;
    async fn da_stake_table(&self, epoch: u64) -> anyhow::Result<Vec<PeerConfig<SeqTypes>>>;
    async fn da_stake_table_current(&self) -> anyhow::Result<StakeTableWithEpochNumber<SeqTypes>>;

    async fn get_validators(
        &self,
        epoch: u64,
    ) -> anyhow::Result<IndexMap<Address, AuthenticatedValidator<PubKey>>>;
    async fn get_all_validators(
        &self,
        epoch: u64,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>>;

    async fn current_proposal_participation(&self) -> anyhow::Result<HashMap<PubKey, f64>>;
    async fn proposal_participation(&self, epoch: u64) -> anyhow::Result<HashMap<PubKey, f64>>;
    async fn current_vote_participation(&self) -> anyhow::Result<HashMap<PubKey, f64>>;
    async fn vote_participation(&self, epoch: u64) -> anyhow::Result<HashMap<PubKey, f64>>;

    async fn get_block_reward(&self, epoch: Option<u64>) -> anyhow::Result<Option<RewardAmount>>;

    async fn get_oldest_block(&self) -> anyhow::Result<Option<BlockQueryData<SeqTypes>>>;
    async fn get_oldest_leaf(&self) -> anyhow::Result<Option<LeafQueryData<SeqTypes>>>;
}
