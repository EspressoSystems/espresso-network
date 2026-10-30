use anyhow::{Context, Result};
use committable::Commitment;
use espresso_types::{
    Certificate2, FeeAccount, FeeMerkleTree, Leaf2,
    v0_3::{ChainConfig, RewardAccountV1, RewardMerkleTreeV1},
    v0_4::{RewardAccountV2, RewardMerkleTreeV2},
};
use hotshot_types::{data::VidShare, simple_certificate::LightClientStateUpdateCertificateV2};
use light_client::consensus::header::HeaderProof;
use request_response::{Serializable, request::Request as RequestTrait};
use serde::{Deserialize, Serialize};

use crate::{SeqTypes, api::BlocksFrontier};

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
    /// A request for data to reconstruct the reward merkle tree at a given height
    RewardMerkleTreeV2(u64, ViewNumber),
    /// A request for the cert2 at or above the given height
    Cert2(Height),
    /// Header at `height` with its block Merkle path at snapshot `root`; `height < root`.
    HeaderProof { root: Height, height: Height },
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
    /// A response with data to reconstruct the reward merkle tree at a given height
    RewardMerkleTreeV2(#[serde(with = "serde_bytes")] Vec<u8>),
    /// A response with the earliest cert2 (fast finality protocol)
    Cert2(Certificate2<SeqTypes>),
    /// A response with a header and its block Merkle path
    HeaderProof(Box<HeaderProof>),
}

/// Implement the `RequestTrait` trait for the `Request` type. This tells the request response
/// protocol how to validate the request and what the response type is.
impl RequestTrait for Request {
    type Response = Response;

    fn validate(&self) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use hotshot_query_service::node::BlockId;
    use light_client::{client::Client, testing::TestClient};

    use super::*;

    /// The appended variants round-trip through bincode and keep the tags of the older ones.
    #[tokio::test]
    async fn header_proof_variants_round_trip() {
        let request = Request::HeaderProof { root: 7, height: 3 };
        let decoded = Request::from_bytes(&request.to_bytes().unwrap()).unwrap();
        assert!(matches!(
            decoded,
            Request::HeaderProof { root: 7, height: 3 }
        ));

        let client = TestClient::default();
        client.leaf(7).await;
        let proof = client.header_proof(7, BlockId::Number(3)).await.unwrap();
        let response = Response::HeaderProof(Box::new(proof.clone()));
        let decoded = Response::from_bytes(&response.to_bytes().unwrap()).unwrap();
        let Response::HeaderProof(decoded) = decoded else {
            panic!("expected a header proof");
        };
        assert_eq!(*decoded, proof);

        // The variant is appended: `Cert2` keeps its tag and `HeaderProof` follows it.
        let tag = |request: &Request| bincode::serialize(request).unwrap()[0];
        assert_eq!(tag(&Request::Cert2(1)), 9);
        assert_eq!(tag(&request), 10);
    }
}
