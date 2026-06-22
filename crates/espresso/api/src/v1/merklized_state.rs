//! V1 merklized state APIs (block-state and fee-state).
//!
//! Mirrors the tide-disco endpoints defined in `hotshot-query-service/api/state.toml`
//! and `crates/espresso/node/api/fee.toml`.

use async_trait::async_trait;
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum Snapshot {
    Height(u64),
    Commit(String),
}

#[async_trait]
pub trait BlockStateApi {
    type MerkleProof: Serialize + Send + Sync + 'static;

    async fn get_block_state_path(
        &self,
        snapshot: Snapshot,
        key: String,
    ) -> anyhow::Result<Self::MerkleProof>;

    async fn get_block_state_height(&self) -> anyhow::Result<u64>;
}

#[async_trait]
pub trait FeeStateApi {
    type MerkleProof: Serialize + Send + Sync + 'static;
    type FeeAmount: Serialize + Send + Sync + 'static;

    async fn get_fee_state_path(
        &self,
        snapshot: Snapshot,
        key: String,
    ) -> anyhow::Result<Self::MerkleProof>;

    async fn get_fee_state_height(&self) -> anyhow::Result<u64>;

    async fn get_fee_balance_latest(
        &self,
        address: String,
    ) -> anyhow::Result<Option<Self::FeeAmount>>;
}
