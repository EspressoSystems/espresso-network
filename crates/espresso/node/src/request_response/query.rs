//! Query service requests between staked peers, carried by request-response over the cliquenet
//! query lane, so large replies never share a connection with consensus messages.

use std::{ops::Range, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;
use espresso_types::{Certificate2, PubKey, SeqTypes};
use hotshot_new_protocol::network::{QueryReceiver, QuerySender};
use hotshot_query_service::{
    availability::{BlockHash, LeafHash, LeafQueryData},
    types::HeightIndexed,
};
use hotshot_types::data::VidCommitment;
use light_client::consensus::{leaf::LeafProof, payload::PayloadProof};
use request_response::{
    RequestResponse, Serializable,
    data_source::DataSource,
    network::{Bytes, Receiver, Sender},
    request::Request,
};
use serde::{Deserialize, Serialize};

use super::{data_source::RequestStorage, recipient_source::RecipientSource};

/// Request-response over the query lane, answered from this node's query service.
pub type QueryProtocol<I> = RequestResponse<
    QueryLaneSender,
    QueryLaneReceiver,
    QueryRequest,
    RecipientSource<I>,
    QueryDataSource,
    PubKey,
>;

/// What a light client asks a peer for. Each is answered from stored data only.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueryRequest {
    /// A finality proof for a leaf, as the light client API serves it.
    LeafProof {
        leaf: LeafRef,
        finalized: Option<u64>,
    },
    /// The leaves in a range, unproven.
    Leaves(Range<u64>),
    /// The payload and VID common at a height.
    PayloadProof(u64),
    /// The certificate2 at a height, unverified.
    Cert2(u64),
}

/// The ways a light client names a leaf.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LeafRef {
    Height(u64),
    Hash(LeafHash<SeqTypes>),
    BlockHash(BlockHash<SeqTypes>),
    PayloadHash(VidCommitment),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum QueryResponse {
    LeafProof(LeafProof),
    Leaves(Vec<LeafQueryData<SeqTypes>>),
    PayloadProof(PayloadProof),
    Cert2(Option<Certificate2<SeqTypes>>),
}

impl QueryResponse {
    /// Whether this is the kind of response `request` asks for, covering exactly its heights.
    ///
    /// Proofs are verified by the light client. This only lets the requester wait for another
    /// peer instead of failing on a response of the wrong shape.
    pub fn answers(&self, request: &QueryRequest) -> bool {
        match (request, self) {
            (QueryRequest::LeafProof { .. }, QueryResponse::LeafProof(_))
            | (QueryRequest::PayloadProof(_), QueryResponse::PayloadProof(_))
            | (QueryRequest::Cert2(_), QueryResponse::Cert2(_)) => true,
            (QueryRequest::Leaves(range), QueryResponse::Leaves(leaves)) => {
                leaves.iter().map(|leaf| leaf.height()).eq(range.clone())
            },
            (
                QueryRequest::LeafProof { .. }
                | QueryRequest::Leaves(_)
                | QueryRequest::PayloadProof(_)
                | QueryRequest::Cert2(_),
                _,
            ) => false,
        }
    }
}

impl Request for QueryRequest {
    type Response = QueryResponse;

    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

impl Serializable for QueryRequest {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("failed to serialize query request")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("failed to deserialize query request")
    }
}

impl Serializable for QueryResponse {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("failed to serialize query response")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("failed to deserialize query response")
    }
}

/// Answers query requests from the node's query storage. A miss is an error, so no reply is sent
/// and the requester moves on to another peer.
#[derive(Clone)]
pub struct QueryDataSource(pub Arc<dyn RequestStorage>);

#[async_trait]
impl DataSource<QueryRequest> for QueryDataSource {
    async fn derive_response_for(&self, request: &QueryRequest) -> Result<QueryResponse> {
        match request {
            QueryRequest::LeafProof { leaf, finalized } => Ok(QueryResponse::LeafProof(
                self.0.get_leaf_proof(*leaf, *finalized).await?,
            )),
            QueryRequest::Leaves(range) => Ok(QueryResponse::Leaves(
                self.0.get_leaves(range.clone()).await?,
            )),
            QueryRequest::PayloadProof(height) => Ok(QueryResponse::PayloadProof(
                self.0.get_payload_proof(*height).await?,
            )),
            QueryRequest::Cert2(height) => {
                Ok(QueryResponse::Cert2(self.0.load_cert2(*height).await?))
            },
        }
    }
}

#[derive(Clone)]
pub struct QueryLaneSender(pub QuerySender<SeqTypes>);

#[async_trait]
impl Sender<PubKey> for QueryLaneSender {
    async fn send_direct_message(&self, message: &Bytes, recipient: PubKey) -> Result<()> {
        Ok(self.0.send(&recipient, message.to_vec())?)
    }
}

pub struct QueryLaneReceiver(pub QueryReceiver<SeqTypes>);

#[async_trait]
impl Receiver for QueryLaneReceiver {
    async fn receive_message(&mut self) -> Result<Bytes> {
        let (_, bytes) = self.0.receive().await.context("query lane closed")?;
        Ok(Arc::new(bytes.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Peers on different releases must agree on the encoding, so variant order is fixed.
    #[test]
    fn request_encoding_is_stable() {
        assert_eq!(
            QueryRequest::Leaves(3..5).to_bytes().unwrap(),
            Vec::from([
                1, 0, 0, 0, // Leaves
                3, 0, 0, 0, 0, 0, 0, 0, // start
                5, 0, 0, 0, 0, 0, 0, 0, // end
            ])
        );
        assert_eq!(
            QueryRequest::Cert2(9).to_bytes().unwrap(),
            Vec::from([3, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0])
        );
    }
}
