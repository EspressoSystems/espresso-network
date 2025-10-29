use anyhow::{Context, Result};
use async_trait::async_trait;
use committable::Commitment;
use espresso_types::{
    v0_3::{ChainConfig, RewardAccountV1, RewardMerkleTreeV1},
    v0_4::{RewardAccountV2, RewardMerkleTreeV2},
    FeeAccount, FeeMerkleTree, Leaf2,
};
use hotshot_types::{data::VidShare, simple_certificate::LightClientStateUpdateCertificateV2};
use request_response::{request::Request as RequestTrait, Serializable};
use serde::{Deserialize, Serialize};

use crate::{api::BlocksFrontier, SeqTypes};

// Some type aliases for readability
type Height = u64;
type ViewNumber = u64;
type RequestId = u64;

/// The outermost request type. This an enum that contains all the possible requests that the
/// sequencer can make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// A request for the accounts at a given height and view
    Accounts(Height, ViewNumber, Vec<FeeAccount>),
    /// A request for the leaf chain at a given height
    Leaf(Height),
    /// A request for a chain config with a particular commitment
    ChainConfig(Commitment<ChainConfig>),
    /// A request for the blocks frontier
    BlocksFrontier(Height, ViewNumber),
    /// A request for the reward accounts at a given height and view
    RewardAccountsV2(Height, ViewNumber, Vec<RewardAccountV2>),
    /// A request for the v1 reward accounts at a given height and view
    RewardAccountsV1(Height, ViewNumber, Vec<RewardAccountV1>),
    /// A request for the VID share at the given block height
    VidShare(Height, RequestId),
    /// A request for the state certificate at a given epoch
    StateCert(u64),
}

/// The outermost response type. This an enum that contains all the possible responses that the
/// sequencer can make.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// A response for the accounts at a given height and view
    Accounts(FeeMerkleTree),
    /// A request for the leaf chain at a given height
    Leaf(Vec<Leaf2>),
    /// A response for a chain config with a particular commitment
    ChainConfig(ChainConfig),
    /// A response for the blocks frontier
    BlocksFrontier(BlocksFrontier),
    /// A response for the reward accounts at a given height and view
    RewardAccountsV2(RewardMerkleTreeV2),
    /// A response for the v1 reward accounts at a given height and view
    RewardAccountsV1(RewardMerkleTreeV1),
    /// A response for a VID share at the given block height
    VidShare(VidShare),
    /// A response for a state certificate at a given epoch
    StateCert(LightClientStateUpdateCertificateV2<SeqTypes>),
}

/// Implement the `RequestTrait` trait for the `Request` type. This tells the request response
/// protocol how to validate the request and what the response type is.
#[async_trait]
impl RequestTrait for Request {
    type Response = Response;

    async fn validate(&self) -> Result<()> {
        // Right now, all requests are valid
        Ok(())
    }
}

/// Implement the `Serializable` trait for the `Request` type. This tells the request response
/// protocol how to serialize and deserialize the request
impl Serializable for Request {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(&self).with_context(|| "failed to serialize")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).with_context(|| "failed to deserialize")
    }
}

/// Implement the `Serializable` trait for the `Response` type. This tells the request response
/// protocol how to serialize and deserialize the response.
impl Serializable for Response {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).with_context(|| "failed to serialize")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).with_context(|| "failed to deserialize")
    }
}
