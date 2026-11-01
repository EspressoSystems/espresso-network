//! The sources a node's light client fetches from: peers over the query lane, and HTTP query
//! services.

use std::{collections::HashMap, ops::Range};

use ::light_client::{
    client::{Client, QueryServiceClient},
    consensus::{
        header::HeaderProof, leaf::LeafProof, namespace::NamespaceProof, payload::PayloadProof,
    },
    storage::LeafRequest,
};
use anyhow::{Context, Result, bail};
use espresso_api::v1;
use espresso_types::{Certificate2, DECAF_CHAIN_ID, NamespaceId, SeqTypes, v0_3::StakeTableEvent};
use hotshot_query_service::{
    availability::{LeafId, LeafQueryData},
    node::BlockId,
};
use hotshot_types::data::EpochNumber;
use serde::de::DeserializeOwned;

use super::ApiState;
use crate::query_lane::QueryRequest;

/// A source a node's light client fetches from.
#[derive(Clone, Debug)]
pub enum NodeClient {
    Peers(PeerClient),
    Http(QueryServiceClient),
}

/// Each call asks one peer over the query lane, which answers with its HTTP API handlers.
/// Calls for data the lane does not carry fail, and the light client verifies whatever comes
/// back.
#[derive(Clone, Debug)]
pub struct PeerClient {
    state: ApiState,
}

impl PeerClient {
    pub(super) fn new(state: ApiState) -> PeerClient {
        PeerClient { state }
    }

    async fn ask<T: DeserializeOwned>(&self, request: QueryRequest) -> Result<T> {
        let lane = self
            .state
            .context()
            .await
            .query_lane()
            .context("this node has no query lane")?;
        let response = lane.ask(request).await?;
        bincode::deserialize(&response).context("decoding a peer's answer")
    }
}

impl Client for PeerClient {
    async fn block_height(&self) -> Result<u64> {
        bail!("block height is not served over the query lane")
    }

    async fn leaf_proof(
        &self,
        id: impl Into<LeafRequest> + Send,
        finalized: Option<u64>,
    ) -> Result<LeafProof> {
        let leaf = match id.into() {
            LeafRequest::Leaf(LeafId::Number(n)) | LeafRequest::Header(BlockId::Number(n)) => {
                v1::LeafQuery::Height(n as u64)
            },
            LeafRequest::Leaf(LeafId::Hash(hash)) => v1::LeafQuery::Hash(hash.to_string()),
            LeafRequest::Header(BlockId::Hash(hash)) => v1::LeafQuery::BlockHash(hash.to_string()),
            LeafRequest::Header(BlockId::PayloadHash(hash)) => {
                v1::LeafQuery::PayloadHash(hash.to_string())
            },
        };
        self.ask(QueryRequest::LeafProof { leaf, finalized }).await
    }

    async fn header_proof(&self, root: u64, id: BlockId<SeqTypes>) -> Result<HeaderProof> {
        let header = match id {
            BlockId::Number(n) => v1::HeaderQuery::Height(n as u64),
            BlockId::Hash(hash) => v1::HeaderQuery::Hash(hash.to_string()),
            BlockId::PayloadHash(hash) => v1::HeaderQuery::PayloadHash(hash.to_string()),
        };
        self.ask(QueryRequest::HeaderProof { root, header }).await
    }

    async fn get_leaves_in_range(
        &self,
        start: usize,
        end: usize,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        let range = start as u64..end as u64;
        self.get_leaves_for_ranges(std::slice::from_ref(&range))
            .await
    }

    async fn get_leaves_for_ranges(
        &self,
        ranges: &[Range<u64>],
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        self.ask(QueryRequest::Leaves(ranges.to_vec())).await
    }

    async fn payload_proof(&self, height: u64) -> Result<PayloadProof> {
        self.ask(QueryRequest::PayloadProof(height)).await
    }

    async fn payload_proofs_in_range(&self, start: u64, end: u64) -> Result<Vec<PayloadProof>> {
        self.payload_proofs_for_ranges(std::slice::from_ref(&(start..end)))
            .await
    }

    async fn payload_proofs_for_ranges(&self, ranges: &[Range<u64>]) -> Result<Vec<PayloadProof>> {
        self.ask(QueryRequest::PayloadProofs(ranges.to_vec())).await
    }

    async fn namespace_proof(
        &self,
        _height: u64,
        _namespace: NamespaceId,
    ) -> Result<NamespaceProof> {
        bail!("namespace proofs are not served over the query lane")
    }

    async fn namespace_proofs_in_range(
        &self,
        _start: u64,
        _end: u64,
        _namespace: NamespaceId,
    ) -> Result<Vec<NamespaceProof>> {
        bail!("namespace proofs are not served over the query lane")
    }

    async fn namespaces_proofs_in_range(
        &self,
        _start: u64,
        _end: u64,
        _namespaces: &[NamespaceId],
    ) -> Result<Vec<HashMap<NamespaceId, NamespaceProof>>> {
        bail!("namespace proofs are not served over the query lane")
    }

    async fn stake_table_events(&self, epoch: EpochNumber) -> Result<Vec<StakeTableEvent>> {
        // The light client trusts its source for the events of Decaf's first epochs, whose roots
        // carry no stake table hash to check them against. Any staked peer is no such source.
        let chain_id = self
            .state
            .context()
            .await
            .node_state()
            .genesis_chain_config
            .chain_id;
        if chain_id == DECAF_CHAIN_ID {
            bail!("Decaf stake table events are not taken from peers");
        }
        self.ask(QueryRequest::StakeTableEvents(*epoch)).await
    }

    async fn cert2(&self, height: u64) -> Result<Option<Certificate2<SeqTypes>>> {
        self.ask(QueryRequest::Cert2(height)).await.map(Some)
    }
}

impl Client for NodeClient {
    async fn block_height(&self) -> Result<u64> {
        match self {
            Self::Peers(client) => client.block_height().await,
            Self::Http(client) => client.block_height().await,
        }
    }

    async fn leaf_proof(
        &self,
        id: impl Into<LeafRequest> + Send,
        finalized: Option<u64>,
    ) -> Result<LeafProof> {
        let id = id.into();
        match self {
            Self::Peers(client) => client.leaf_proof(id, finalized).await,
            Self::Http(client) => client.leaf_proof(id, finalized).await,
        }
    }

    async fn header_proof(&self, root: u64, id: BlockId<SeqTypes>) -> Result<HeaderProof> {
        match self {
            Self::Peers(client) => client.header_proof(root, id).await,
            Self::Http(client) => client.header_proof(root, id).await,
        }
    }

    async fn get_leaves_in_range(
        &self,
        start: usize,
        end: usize,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        match self {
            Self::Peers(client) => client.get_leaves_in_range(start, end).await,
            Self::Http(client) => client.get_leaves_in_range(start, end).await,
        }
    }

    async fn get_leaves_for_ranges(
        &self,
        ranges: &[Range<u64>],
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        match self {
            Self::Peers(client) => client.get_leaves_for_ranges(ranges).await,
            Self::Http(client) => client.get_leaves_for_ranges(ranges).await,
        }
    }

    async fn payload_proof(&self, height: u64) -> Result<PayloadProof> {
        match self {
            Self::Peers(client) => client.payload_proof(height).await,
            Self::Http(client) => client.payload_proof(height).await,
        }
    }

    async fn payload_proofs_in_range(&self, start: u64, end: u64) -> Result<Vec<PayloadProof>> {
        match self {
            Self::Peers(client) => client.payload_proofs_in_range(start, end).await,
            Self::Http(client) => client.payload_proofs_in_range(start, end).await,
        }
    }

    async fn payload_proofs_for_ranges(&self, ranges: &[Range<u64>]) -> Result<Vec<PayloadProof>> {
        match self {
            Self::Peers(client) => client.payload_proofs_for_ranges(ranges).await,
            Self::Http(client) => client.payload_proofs_for_ranges(ranges).await,
        }
    }

    async fn namespace_proof(&self, height: u64, namespace: NamespaceId) -> Result<NamespaceProof> {
        match self {
            Self::Peers(client) => client.namespace_proof(height, namespace).await,
            Self::Http(client) => client.namespace_proof(height, namespace).await,
        }
    }

    async fn namespace_proofs_in_range(
        &self,
        start: u64,
        end: u64,
        namespace: NamespaceId,
    ) -> Result<Vec<NamespaceProof>> {
        match self {
            Self::Peers(client) => {
                client
                    .namespace_proofs_in_range(start, end, namespace)
                    .await
            },
            Self::Http(client) => {
                client
                    .namespace_proofs_in_range(start, end, namespace)
                    .await
            },
        }
    }

    async fn namespaces_proofs_in_range(
        &self,
        start: u64,
        end: u64,
        namespaces: &[NamespaceId],
    ) -> Result<Vec<HashMap<NamespaceId, NamespaceProof>>> {
        match self {
            Self::Peers(client) => {
                client
                    .namespaces_proofs_in_range(start, end, namespaces)
                    .await
            },
            Self::Http(client) => {
                client
                    .namespaces_proofs_in_range(start, end, namespaces)
                    .await
            },
        }
    }

    async fn stake_table_events(&self, epoch: EpochNumber) -> Result<Vec<StakeTableEvent>> {
        match self {
            Self::Peers(client) => client.stake_table_events(epoch).await,
            Self::Http(client) => client.stake_table_events(epoch).await,
        }
    }

    async fn cert2(&self, height: u64) -> Result<Option<Certificate2<SeqTypes>>> {
        match self {
            Self::Peers(client) => client.cert2(height).await,
            Self::Http(client) => client.cert2(height).await,
        }
    }
}
