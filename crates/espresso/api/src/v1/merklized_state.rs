//! V1 merklized state APIs (block-state and fee-state).
//!
//! Mirrors the endpoints of the legacy `hotshot-query-service` merklized-state API (its
//! tide-disco `state.toml`, since removed).

use async_trait::async_trait;
use committable::Commitment;
use espresso_types::{BlockMerkleTree, FeeAccount, FeeAmount, FeeMerkleTree, Header};
use jf_merkle_tree_compat::{
    MerkleTreeScheme,
    prelude::{MerkleProof, Sha3Node},
};

#[derive(Debug, Clone)]
pub enum Snapshot {
    Height(u64),
    Commit(String),
}

pub type BlockStatePath = MerkleProof<
    Commitment<Header>,
    u64,
    Sha3Node,
    { <BlockMerkleTree as MerkleTreeScheme>::ARITY },
>;

pub type FeeStatePath =
    MerkleProof<FeeAmount, FeeAccount, Sha3Node, { <FeeMerkleTree as MerkleTreeScheme>::ARITY }>;

#[async_trait]
pub trait BlockStateApi {
    async fn get_block_state_path(
        &self,
        snapshot: Snapshot,
        key: String,
    ) -> anyhow::Result<BlockStatePath>;

    async fn get_block_state_height(&self) -> anyhow::Result<u64>;
}

#[async_trait]
pub trait FeeStateApi {
    async fn get_fee_state_path(
        &self,
        snapshot: Snapshot,
        key: String,
    ) -> anyhow::Result<FeeStatePath>;

    async fn get_fee_state_height(&self) -> anyhow::Result<u64>;

    async fn get_fee_balance_latest(&self, address: String) -> anyhow::Result<Option<FeeAmount>>;
}
