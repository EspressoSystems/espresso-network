//! A light client [`Client`] served by staked peers over the cliquenet query lane.

use std::{collections::HashMap, ops::Range};

use ::light_client::{
    client::Client,
    consensus::{
        header::HeaderProof, leaf::LeafProof, namespace::NamespaceProof, payload::PayloadProof,
    },
    storage::LeafRequest,
};
use anyhow::{Context as _, Result, bail};
use espresso_types::{Certificate2, NamespaceId, SeqTypes, v0_3::StakeTableEvent};
use futures::future::try_join_all;
use hotshot_query_service::{
    availability::{LeafId, LeafQueryData},
    node::BlockId,
};
use hotshot_types::{data::EpochNumber, utils::epoch_from_block_number};

use super::{ApiState, data_source::StakeTableDataSource, state::stake_table_l1_range};
use crate::query_peers::{LeafRef, QueryRequest, QueryResponse};

/// Each call asks one staked peer over the query lane. Calls for data the lane does not carry
/// fail, and the light client verifies whatever comes back.
#[derive(Clone, Debug)]
pub(crate) struct PeerClient {
    state: ApiState,
}

impl PeerClient {
    pub(super) fn new(state: ApiState) -> PeerClient {
        PeerClient { state }
    }

    async fn ask(&self, request: QueryRequest) -> Result<QueryResponse> {
        self.state.context().await.query_peers(request).await
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
                LeafRef::Height(n as u64)
            },
            LeafRequest::Leaf(LeafId::Hash(hash)) => LeafRef::Hash(hash),
            LeafRequest::Header(BlockId::Hash(hash)) => LeafRef::BlockHash(hash),
            LeafRequest::Header(BlockId::PayloadHash(hash)) => LeafRef::PayloadHash(hash),
        };
        match self
            .ask(QueryRequest::LeafProof { leaf, finalized })
            .await?
        {
            QueryResponse::LeafProof(proof) => Ok(proof),
            response => bail!("unexpected response {response:?}"),
        }
    }

    async fn header_proof(&self, _root: u64, _id: BlockId<SeqTypes>) -> Result<HeaderProof> {
        bail!("header proofs are not served over the query lane")
    }

    async fn get_leaves_in_range(
        &self,
        start: usize,
        end: usize,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        match self
            .ask(QueryRequest::Leaves(start as u64..end as u64))
            .await?
        {
            QueryResponse::Leaves(leaves) => Ok(leaves),
            response => bail!("unexpected response {response:?}"),
        }
    }

    async fn get_leaves_for_ranges(
        &self,
        ranges: &[Range<u64>],
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        let leaves = try_join_all(
            ranges
                .iter()
                .map(|range| self.get_leaves_in_range(range.start as usize, range.end as usize)),
        )
        .await?;
        Ok(leaves.into_iter().flatten().collect())
    }

    async fn payload_proof(&self, height: u64) -> Result<PayloadProof> {
        match self.ask(QueryRequest::PayloadProof(height)).await? {
            QueryResponse::PayloadProof(proof) => Ok(proof),
            response => bail!("unexpected response {response:?}"),
        }
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

    /// Read from this node's own stake table events, the same way the light client API serves
    /// them, so verifying a peer's answer never needs another peer.
    async fn stake_table_events(&self, epoch: EpochNumber) -> Result<Vec<StakeTableEvent>> {
        let ctx = self.state.context().await;
        let node_state = ctx.node_state();
        let epoch_height = node_state.epoch_height.context("epoch height not set")?;
        let first_epoch = epoch_from_block_number(node_state.epoch_start_block, epoch_height);
        let membership = ctx.membership();
        let membership = &membership;
        let range = stake_table_l1_range(*epoch, first_epoch, move |epoch| async move {
            membership
                .load_epoch_root(EpochNumber::new(epoch))
                .await?
                .with_context(|| format!("missing epoch root for epoch {epoch}"))
        })
        .await?;
        StakeTableDataSource::stake_table_events(&self.state, range.start, range.end).await
    }

    async fn cert2(&self, height: u64) -> Result<Option<Certificate2<SeqTypes>>> {
        match self.ask(QueryRequest::Cert2(height)).await? {
            QueryResponse::Cert2(cert2) => Ok(Some(cert2)),
            response => bail!("unexpected response {response:?}"),
        }
    }
}
