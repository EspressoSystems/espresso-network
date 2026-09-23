//! [`ApiState`](super::ApiState) reaches consensus and node-wide state only through
//! [`ApiContext`], so the same API modules can be served by a validator ([`SequencerContext`])
//! or by a node that follows the chain without taking part in consensus.

use std::{collections::HashMap, sync::Arc, time::Duration};

use ::light_client::{
    LightClient,
    client::{FallbackClient, QueryServiceClient},
    storage::SqliteStorage,
};
use anyhow::Context as _;
use async_lock::RwLock;
use async_trait::async_trait;
use espresso_types::{
    Leaf2, NodeState, PubKey, SeqTypes, Transaction, ValidatedState,
    v0::traits::SequencerPersistence,
};
use futures::future::BoxFuture;
use hotshot_events_service::events_source::EventsStreamer;
use hotshot_new_protocol::{client::ClientApi, state::UpdateLeaf};
use hotshot_query_service::availability::VidCommonQueryData;
use hotshot_types::{
    ValidatorConfig,
    data::{EpochNumber, VidShare, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    network::NetworkConfig,
    utils::StateAndDelta,
};
use tracing::warn;

use crate::{
    SequencerApiVersion, SequencerContext, context::TaskList, state_signature::StateSigner,
};

pub type NodeLightClient = LightClient<SqliteStorage, FallbackClient<QueryServiceClient>>;

pub type Delta = <ValidatedState as hotshot_types::traits::ValidatedState<SeqTypes>>::Delta;

#[async_trait]
pub trait ConsensusSource: Send + Sync + 'static {
    async fn decided_leaf(&self) -> anyhow::Result<Leaf2>;
    async fn decided_state(&self) -> Option<Arc<ValidatedState>>;
    async fn state(&self, view: ViewNumber) -> Option<Arc<ValidatedState>>;
    async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<SeqTypes>;
    async fn undecided_leaves(&self) -> Vec<Leaf2>;
    async fn current_epoch(&self) -> Option<EpochNumber>;
    async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<()>;
    /// How catchup pushes a state it recovered from storage back into memory.
    async fn update_leaf(
        &self,
        leaf: Leaf2,
        state: Arc<ValidatedState>,
        delta: Option<Arc<Delta>>,
    ) -> anyhow::Result<()>;
    async fn current_proposal_participation(&self) -> HashMap<PubKey, f64>;
    async fn proposal_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64>;
    async fn current_vote_participation(&self) -> HashMap<PubKey, f64>;
    async fn vote_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64>;
}

pub trait ApiContext: Clone + Send + Sync + 'static {
    type Persistence: SequencerPersistence;

    fn consensus(&self) -> Arc<dyn ConsensusSource>;
    fn membership_coordinator(&self) -> EpochMembershipCoordinator<SeqTypes>;
    fn upgrade_lock(&self) -> UpgradeLock<SeqTypes>;
    fn persistence(&self) -> Arc<Self::Persistence>;
    fn node_state(&self) -> NodeState;
    fn network_config(&self) -> NetworkConfig<SeqTypes>;
    fn validator_config(&self) -> Option<&ValidatorConfig<SeqTypes>>;
    fn state_signer(&self) -> Option<Arc<RwLock<StateSigner<SequencerApiVersion>>>>;
    fn event_streamer(&self) -> Option<Arc<RwLock<EventsStreamer<SeqTypes>>>>;
    /// A light client the node already runs, for the query service to fetch through.
    fn light_client(&self) -> Option<Arc<NodeLightClient>>;
    fn request_vid_shares(
        &self,
        block_number: u64,
        vid_common: VidCommonQueryData<SeqTypes>,
        timeout: Duration,
    ) -> BoxFuture<'static, anyhow::Result<Vec<VidShare>>>;
    /// [`Options::serve`](super::Options::serve) hands a clone of the context to the API before
    /// calling this, so the tasks attached here must be shared with that clone: [`TaskList`] is
    /// that shared handle, and dropping any clone of it aborts every task.
    fn with_task_list(self, tasks: TaskList) -> Self
    where
        Self: Sized;
}

/// Reads that fail because the coordinator stopped are logged and answered as if the value were
/// unknown, except [`ConsensusSource::decided_leaf`], which always has an answer while consensus
/// runs and so reports the failure.
#[async_trait]
impl ConsensusSource for ClientApi<SeqTypes> {
    async fn decided_leaf(&self) -> anyhow::Result<Leaf2> {
        ClientApi::decided_leaf(self)
            .await
            .context("failed to read the decided leaf from the coordinator")
    }

    async fn decided_state(&self) -> Option<Arc<ValidatedState>> {
        ClientApi::decided_state(self)
            .await
            .inspect_err(|err| warn!(%err, "coordinator unavailable for decided_state"))
            .ok()
            .flatten()
    }

    async fn state(&self, view: ViewNumber) -> Option<Arc<ValidatedState>> {
        ClientApi::state(self, view)
            .await
            .inspect_err(|err| warn!(%view, %err, "coordinator unavailable for state"))
            .ok()
            .flatten()
    }

    async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<SeqTypes> {
        ClientApi::state_and_delta(self, view)
            .await
            .inspect_err(|err| warn!(%view, %err, "coordinator unavailable for state_and_delta"))
            .unwrap_or((None, None))
    }

    async fn undecided_leaves(&self) -> Vec<Leaf2> {
        ClientApi::undecided_leaves(self)
            .await
            .inspect_err(|err| warn!(%err, "coordinator unavailable for undecided_leaves"))
            .unwrap_or_default()
    }

    async fn current_epoch(&self) -> Option<EpochNumber> {
        ClientApi::current_epoch(self)
            .await
            .inspect_err(|err| warn!(%err, "coordinator unavailable for current_epoch"))
            .ok()
            .flatten()
    }

    async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<()> {
        ClientApi::submit_transaction(self, tx)
            .await
            .context("failed to submit transaction to the coordinator")
    }

    async fn update_leaf(
        &self,
        leaf: Leaf2,
        state: Arc<ValidatedState>,
        delta: Option<Arc<Delta>>,
    ) -> anyhow::Result<()> {
        let update = UpdateLeaf {
            view: leaf.view_number(),
            leaf,
            state,
            delta,
        };
        ClientApi::update_leaf(self, update)
            .await
            .context("failed to update a leaf in the coordinator")
    }

    async fn current_proposal_participation(&self) -> HashMap<PubKey, f64> {
        ClientApi::proposal_participation(self, None)
            .await
            .inspect_err(|err| warn!(%err, "coordinator unavailable for proposal participation"))
            .unwrap_or_default()
    }

    async fn proposal_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64> {
        ClientApi::proposal_participation(self, Some(epoch))
            .await
            .inspect_err(|err| warn!(%err, "coordinator unavailable for proposal participation"))
            .unwrap_or_default()
    }

    async fn current_vote_participation(&self) -> HashMap<PubKey, f64> {
        ClientApi::vote_participation(self, None)
            .await
            .inspect_err(|err| warn!(%err, "coordinator unavailable for vote participation"))
            .unwrap_or_default()
    }

    async fn vote_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64> {
        ClientApi::vote_participation(self, Some(epoch))
            .await
            .inspect_err(|err| warn!(%err, "coordinator unavailable for vote participation"))
            .unwrap_or_default()
    }
}

impl<P> ApiContext for SequencerContext<P>
where
    P: SequencerPersistence,
{
    type Persistence = P;

    fn consensus(&self) -> Arc<dyn ConsensusSource> {
        Arc::new(self.client_api().clone())
    }

    fn membership_coordinator(&self) -> EpochMembershipCoordinator<SeqTypes> {
        self.node_state().coordinator
    }

    fn upgrade_lock(&self) -> UpgradeLock<SeqTypes> {
        SequencerContext::upgrade_lock(self).clone()
    }

    fn persistence(&self) -> Arc<P> {
        SequencerContext::persistence(self)
    }

    fn node_state(&self) -> NodeState {
        SequencerContext::node_state(self)
    }

    fn network_config(&self) -> NetworkConfig<SeqTypes> {
        SequencerContext::network_config(self)
    }

    fn validator_config(&self) -> Option<&ValidatorConfig<SeqTypes>> {
        Some(SequencerContext::validator_config(self))
    }

    fn state_signer(&self) -> Option<Arc<RwLock<StateSigner<SequencerApiVersion>>>> {
        Some(SequencerContext::state_signer(self))
    }

    fn event_streamer(&self) -> Option<Arc<RwLock<EventsStreamer<SeqTypes>>>> {
        Some(SequencerContext::event_streamer(self))
    }

    fn light_client(&self) -> Option<Arc<NodeLightClient>> {
        None
    }

    fn request_vid_shares(
        &self,
        block_number: u64,
        vid_common: VidCommonQueryData<SeqTypes>,
        timeout: Duration,
    ) -> BoxFuture<'static, anyhow::Result<Vec<VidShare>>> {
        super::request_vid_shares(
            self.request_response_protocol.clone(),
            block_number,
            vid_common,
            timeout,
        )
    }

    fn with_task_list(self, tasks: TaskList) -> Self {
        SequencerContext::with_task_list(self, tasks)
    }
}
