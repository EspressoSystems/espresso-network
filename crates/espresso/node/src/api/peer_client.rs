//! A light client [`Client`] served by staked peers over the cliquenet query lane.

use std::{collections::HashMap, ops::Range};

use ::light_client::{
    client::Client,
    consensus::{
        header::HeaderProof, leaf::LeafProof, namespace::NamespaceProof, payload::PayloadProof,
    },
    storage::LeafRequest,
};
use anyhow::{Result, bail};
use espresso_api::v1;
use espresso_types::{Certificate2, NamespaceId, SeqTypes, v0_3::StakeTableEvent};
use futures::future::try_join_all;
use hotshot_query_service::{
    availability::{LeafId, LeafQueryData},
    node::BlockId,
};
use hotshot_types::data::EpochNumber;
use serde::de::DeserializeOwned;

use super::ApiState;
use crate::query_peers::QueryRequest;

/// Each call asks one staked peer over the query lane, which answers with its HTTP API handlers.
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
        let response = self.state.context().await.query_peers(request).await?;
        Ok(bincode::deserialize(&response)?)
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

    async fn header_proof(&self, _root: u64, _id: BlockId<SeqTypes>) -> Result<HeaderProof> {
        bail!("header proofs are not served over the query lane")
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
        try_join_all((start..end).map(|height| self.payload_proof(height))).await
    }

    async fn payload_proofs_for_ranges(&self, ranges: &[Range<u64>]) -> Result<Vec<PayloadProof>> {
        let heights = ranges.iter().cloned().flatten();
        try_join_all(heights.map(|height| self.payload_proof(height))).await
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
        self.ask(QueryRequest::StakeTableEvents(*epoch)).await
    }

    async fn cert2(&self, height: u64) -> Result<Option<Certificate2<SeqTypes>>> {
        self.ask(QueryRequest::Cert2(height)).await.map(Some)
    }
}
