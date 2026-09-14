//! The API's view of the node it runs on.
//!
//! [`ApiState`](super::ApiState) reaches consensus and node-wide state only through
//! [`ApiContext`], so the same API modules can be served by a validator ([`SequencerContext`])
//! or by a node that follows the chain without taking part in consensus.

use std::{collections::HashMap, sync::Arc, time::Duration};

use ::light_client::{
    LightClient,
    client::{FallbackClient, QueryServiceClient},
    storage::SqliteStorage,
};
use async_lock::RwLock;
use async_trait::async_trait;
use espresso_types::{
    Leaf2, NodeState, PubKey, SeqTypes, Transaction, ValidatedState,
    v0::traits::SequencerPersistence,
};
use futures::future::BoxFuture;
use hotshot::traits::NodeImplementation;
use hotshot_events_service::events_source::EventsStreamer;
use hotshot_new_protocol::storage::NewProtocolStorage;
use hotshot_query_service::availability::VidCommonQueryData;
use hotshot_types::{
    ValidatorConfig,
    data::{EpochNumber, VidShare, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    network::NetworkConfig,
    traits::network::ConnectedNetwork,
    utils::StateAndDelta,
};

use crate::{
    SequencerApiVersion, SequencerContext, consensus_handle::ConsensusHandle, context::TaskList,
    state_signature::StateSigner,
};

/// The light client the node fetches missing data with.
pub type NodeLightClient = LightClient<SqliteStorage, FallbackClient<QueryServiceClient>>;

/// The state delta of a block, as consensus tracks it.
pub type Delta = <ValidatedState as hotshot_types::traits::ValidatedState<SeqTypes>>::Delta;

/// What the API needs from consensus.
///
/// Reads, except for `submit_transaction` and `update_leaf`, which write: the latter is how
/// catchup pushes a state it recovered from storage back into memory.
#[async_trait]
pub trait ConsensusSource: Send + Sync + 'static {
    async fn decided_leaf(&self) -> Leaf2;
    async fn decided_state(&self) -> Option<Arc<ValidatedState>>;
    async fn state(&self, view: ViewNumber) -> Option<Arc<ValidatedState>>;
    async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<SeqTypes>;
    async fn undecided_leaves(&self) -> Vec<Leaf2>;
    async fn current_epoch(&self) -> Option<EpochNumber>;
    async fn membership_coordinator(&self) -> EpochMembershipCoordinator<SeqTypes>;
    async fn upgrade_lock(&self) -> UpgradeLock<SeqTypes>;
    async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<()>;
    /// Remember a state recovered from storage for the view of `leaf`.
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

/// Everything the API needs from the node it runs on.
///
/// The API holds a clone of the context, so `Clone` has to be cheap and the clones have to
/// agree: see [`with_task_list`](Self::with_task_list).
pub trait ApiContext: Clone + Send + Sync + 'static {
    type Persistence: SequencerPersistence;

    fn consensus(&self) -> Arc<dyn ConsensusSource>;
    fn persistence(&self) -> Arc<Self::Persistence>;
    fn node_state(&self) -> NodeState;
    fn network_config(&self) -> NetworkConfig<SeqTypes>;
    /// The node's own keys; `None` on a node without stake.
    fn validator_config(&self) -> Option<&ValidatorConfig<SeqTypes>>;
    /// Signs light client states; `None` on a node without stake.
    fn state_signer(&self) -> Option<Arc<RwLock<StateSigner<SequencerApiVersion>>>>;
    /// Streams consensus events; `None` on a node without a consensus event stream.
    fn event_streamer(&self) -> Option<Arc<RwLock<EventsStreamer<SeqTypes>>>>;
    /// A light client the node already runs, for the query service to fetch through.
    fn light_client(&self) -> Option<Arc<NodeLightClient>>;
    /// Collect VID shares for a block from peers, verifying each against `vid_common`.
    fn request_vid_shares(
        &self,
        block_number: u64,
        vid_common: VidCommonQueryData<SeqTypes>,
        timeout: Duration,
    ) -> BoxFuture<'static, anyhow::Result<Vec<VidShare>>>;
    /// Attach background tasks to shut down with the node.
    ///
    /// [`Options::serve`](super::Options::serve) hands a clone of the context to the API before
    /// calling this, so the task list must be shared between clones: tasks attached here have to
    /// be visible to, and shut down by, the clone the API already holds. [`TaskList`] is that
    /// shared handle: dropping any clone of it aborts every task.
    fn with_task_list(self, tasks: TaskList) -> Self
    where
        Self: Sized;
}

#[async_trait]
impl<I> ConsensusSource for ConsensusHandle<SeqTypes, I>
where
    I: NodeImplementation<SeqTypes>,
    I::Storage: NewProtocolStorage<SeqTypes>,
{
    async fn decided_leaf(&self) -> Leaf2 {
        ConsensusHandle::decided_leaf(self).await
    }

    async fn decided_state(&self) -> Option<Arc<ValidatedState>> {
        ConsensusHandle::decided_state(self).await
    }

    async fn state(&self, view: ViewNumber) -> Option<Arc<ValidatedState>> {
        ConsensusHandle::state(self, view).await
    }

    async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<SeqTypes> {
        ConsensusHandle::state_and_delta(self, view).await
    }

    async fn undecided_leaves(&self) -> Vec<Leaf2> {
        ConsensusHandle::undecided_leaves(self).await
    }

    async fn current_epoch(&self) -> Option<EpochNumber> {
        ConsensusHandle::current_epoch(self).await
    }

    async fn membership_coordinator(&self) -> EpochMembershipCoordinator<SeqTypes> {
        ConsensusHandle::membership_coordinator(self).await
    }

    async fn upgrade_lock(&self) -> UpgradeLock<SeqTypes> {
        ConsensusHandle::upgrade_lock(self).await
    }

    async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<()> {
        ConsensusHandle::submit_transaction(self, tx).await
    }

    async fn update_leaf(
        &self,
        leaf: Leaf2,
        state: Arc<ValidatedState>,
        delta: Option<Arc<Delta>>,
    ) -> anyhow::Result<()> {
        ConsensusHandle::update_leaf(self, leaf, state, delta).await
    }

    async fn current_proposal_participation(&self) -> HashMap<PubKey, f64> {
        ConsensusHandle::current_proposal_participation(self).await
    }

    async fn proposal_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64> {
        ConsensusHandle::proposal_participation(self, epoch).await
    }

    async fn current_vote_participation(&self) -> HashMap<PubKey, f64> {
        ConsensusHandle::current_vote_participation(self).await
    }

    async fn vote_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64> {
        ConsensusHandle::vote_participation(self, epoch).await
    }
}

impl<N, P> ApiContext for SequencerContext<N, P>
where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
{
    type Persistence = P;

    fn consensus(&self) -> Arc<dyn ConsensusSource> {
        self.consensus_handle()
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
