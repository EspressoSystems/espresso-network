use std::{collections::HashMap, mem, sync::Arc};

use anyhow::bail;
use async_broadcast::{InactiveReceiver, Sender, broadcast};
use committable::Commitment;
use espresso_api::error::SubmitError;
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::BoxStream};
use hotshot_new_protocol::{
    block,
    client::{ClientApi, QueryError},
    consensus::{ConsensusInput, ConsensusOutput},
    coordinator::{
        Coordinator,
        error::{CoordinatorError, Severity},
    },
    message::OpaqueMessage,
    state::UpdateLeaf,
    storage::NewProtocolStorage,
};
use hotshot_types::{
    data::{BlockNumber, EpochNumber, Leaf2, QuorumProposalWrapper, VidDisperseShare, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    event::LeafInfo,
    initializer::HotShotInitializer,
    message::{Proposal as SignedProposal, UpgradeLock, convert_proposal},
    new_protocol::CoordinatorEvent,
    traits::{
        ValidatedState,
        metrics::{Gauge, Metrics},
        node_implementation::{NodeImplementation, NodeType},
        signature_key::SignatureKey,
    },
    utils::StateAndDelta,
};
use parking_lot::RwLock;
use tokio::{select, spawn, sync::mpsc::UnboundedReceiver};
use tokio_util::{sync::CancellationToken, task::AbortOnDropHandle};
use tracing::{error, warn};

/// The node's handle on consensus: the [`Coordinator`], run on its own task from
/// [`start_consensus`](Self::start_consensus) on, and what the rest of the node reads from it.
pub struct ConsensusHandle<T: NodeType, I: NodeImplementation<T>> {
    epoch_height: BlockNumber,
    event_rx: InactiveReceiver<CoordinatorEvent<T>>,
    upgrade_lock: UpgradeLock<T>,
    membership_coordinator: EpochMembershipCoordinator<T>,
    resting: Arc<RwLock<Resting<T>>>,
    new_proto: Arc<RwLock<NewProtocol<T, I::Storage>>>,
    /// Cancelled once the coordinator task has ended, however it ended.
    exited: CancellationToken,
}

/// What the readers serve once the coordinator has stopped: what it held when it stopped, written
/// by its task on the way out, or the anchor if it never ran.
struct Resting<T: NodeType> {
    view: ViewNumber,
    leaf: Leaf2<T>,
    state: Arc<T::ValidatedState>,
}

#[allow(clippy::large_enum_variant)]
enum NewProtocol<T: NodeType, S> {
    Empty,
    Init {
        coordinator: Coordinator<T, S>,
        external_rx: UnboundedReceiver<OpaqueMessage<T::SignatureKey>>,
        event_tx: Sender<CoordinatorEvent<T>>,
        queue_len: Option<Arc<dyn Gauge>>,
    },
    Running {
        /// Held so that dropping the handle aborts the task; `shut_down` waits on `exited`.
        #[cfg_attr(not(test), allow(dead_code))]
        coordinator: AbortOnDropHandle<()>,
        client_api: ClientApi<T>,
        shutdown: CancellationToken,
    },
}

impl<T: NodeType, S> NewProtocol<T, S> {
    fn take(&mut self) -> Self {
        mem::replace(self, Self::Empty)
    }
}

impl<T, I> ConsensusHandle<T, I>
where
    T: NodeType,
    I: NodeImplementation<T>,
    I::Storage: NewProtocolStorage<T>,
{
    pub fn new(
        coordinator: Coordinator<T, I::Storage>,
        external_rx: UnboundedReceiver<OpaqueMessage<T::SignatureKey>>,
        upgrade_lock: UpgradeLock<T>,
        membership_coordinator: EpochMembershipCoordinator<T>,
        initializer: &HotShotInitializer<T>,
        event_channel_capacity: usize,
        metrics: &dyn Metrics,
    ) -> Self {
        let (mut event_tx, mut event_rx) = broadcast(event_channel_capacity);
        event_tx.set_await_active(false);
        event_rx.set_overflow(true);

        let coordinator_event_queue_len = metrics.is_recording().then(|| {
            metrics
                .create_gauge("coordinator_event_queue_len".into(), None)
                .into()
        });

        let new_proto = Arc::new(RwLock::new(NewProtocol::Init {
            coordinator,
            external_rx,
            event_tx,
            queue_len: coordinator_event_queue_len,
        }));

        Self {
            epoch_height: initializer.epoch_height().into(),
            event_rx: event_rx.deactivate(),
            upgrade_lock,
            membership_coordinator,
            resting: Arc::new(RwLock::new(Resting {
                view: initializer.anchor_leaf().view_number(),
                leaf: initializer.anchor_leaf().clone(),
                state: initializer.anchor_state().clone(),
            })),
            new_proto,
            exited: CancellationToken::new(),
        }
    }

    pub async fn start_consensus(&self) {
        if matches!(*self.new_proto.read(), NewProtocol::Running { .. }) {
            return;
        }

        let mut new_proto = self.new_proto.write();

        match new_proto.take() {
            NewProtocol::Init {
                coordinator,
                external_rx,
                event_tx,
                queue_len,
            } => {
                let client_api = coordinator.client_api().clone();
                let shutdown = CancellationToken::new();
                *new_proto = NewProtocol::Running {
                    coordinator: AbortOnDropHandle::new(spawn(run_coordinator(
                        coordinator,
                        external_rx,
                        event_tx,
                        queue_len,
                        shutdown.clone(),
                        self.exited.clone(),
                        Arc::clone(&self.resting),
                    ))),
                    client_api,
                    shutdown,
                };
            },
            other => *new_proto = other,
        }
    }

    pub fn event_stream(&self) -> BoxStream<'static, CoordinatorEvent<T>> {
        self.event_rx.activate_cloned().boxed()
    }

    /// Resolves once the coordinator task has ended, whether `shut_down` stopped it or it died
    /// on a critical error or a panic. Never resolves if consensus never started.
    pub async fn stopped(&self) {
        self.exited.cancelled().await
    }

    /// Serves the resting state once the coordinator has been told to shut down, waiting for
    /// the state it leaves on the way out if a request fails meanwhile. Panics if the
    /// coordinator died instead: the node must not answer as if the chain had stopped.
    pub async fn current_view(&self) -> ViewNumber {
        let Some(client_api) = self.client_api().await else {
            return self.resting.read().view;
        };
        match client_api.current_view().await {
            Ok(view) => view,
            Err(err) => self.resting_unless_dead(err).await.view,
        }
    }

    /// See [`Self::current_view`].
    pub async fn decided_leaf(&self) -> Leaf2<T> {
        let Some(client_api) = self.client_api().await else {
            return self.resting.read().leaf.clone();
        };
        match client_api.decided_leaf().await {
            Ok(leaf) => leaf,
            Err(err) => self.resting_unless_dead(err).await.leaf.clone(),
        }
    }

    /// See [`Self::current_view`].
    pub async fn decided_state(&self) -> Option<Arc<T::ValidatedState>> {
        let Some(client_api) = self.client_api().await else {
            return Some(self.resting.read().state.clone());
        };
        match client_api.decided_state().await {
            Ok(state) => state,
            Err(err) => Some(self.resting_unless_dead(err).await.state.clone()),
        }
    }

    /// Where a failed request leaves a reader: the coordinator was told to shut down and the
    /// resting state it leaves behind answers, or no shutdown was asked for, so it died.
    async fn resting_unless_dead(
        &self,
        err: QueryError,
    ) -> parking_lot::RwLockReadGuard<'_, Resting<T>> {
        let stopping = match &*self.new_proto.read() {
            NewProtocol::Running { shutdown, .. } => shutdown.is_cancelled(),
            _ => true,
        };
        if !stopping {
            panic!("coordinator died: {err}");
        }
        // The coordinator writes the resting state before it exits.
        self.exited.cancelled().await;
        self.resting.read()
    }

    pub async fn state(&self, view: ViewNumber) -> Option<Arc<T::ValidatedState>> {
        match self.client_api().await?.state(view).await {
            Ok(state) => state,
            Err(err) => {
                warn!(%view, %err, "coordinator unavailable for state");
                None
            },
        }
    }

    pub async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<T> {
        let Some(client_api) = self.client_api().await else {
            return (None, None);
        };
        match client_api.state_and_delta(view).await {
            Ok(state_and_delta) => state_and_delta,
            Err(err) => {
                warn!(%view, %err, "coordinator unavailable for state_and_delta");
                (None, None)
            },
        }
    }

    pub async fn undecided_leaves(&self) -> Vec<Leaf2<T>> {
        let Some(client_api) = self.client_api().await else {
            return Vec::new();
        };
        match client_api.undecided_leaves().await {
            Ok(leaves) => leaves,
            Err(err) => {
                warn!(%err, "coordinator unavailable for undecided_leaves");
                Vec::new()
            },
        }
    }

    pub async fn current_epoch(&self) -> Option<EpochNumber> {
        let Some(client_api) = self.client_api().await else {
            return self.resting.read().leaf.epoch(*self.epoch_height);
        };
        match client_api.current_epoch().await {
            Ok(epoch) => Some(epoch),
            Err(err) => {
                warn!(%err, "coordinator unavailable for current_epoch");
                None
            },
        }
    }

    pub async fn epoch_height(&self) -> BlockNumber {
        self.epoch_height
    }

    pub async fn membership_coordinator(&self) -> EpochMembershipCoordinator<T> {
        self.membership_coordinator.clone()
    }

    pub async fn upgrade_lock(&self) -> UpgradeLock<T> {
        self.upgrade_lock.clone()
    }

    pub async fn current_proposal_participation(&self) -> HashMap<T::SignatureKey, f64> {
        let Some(client_api) = self.client_api().await else {
            return HashMap::new();
        };
        client_api
            .proposal_participation(None)
            .await
            .inspect_err(|err| {
                warn!(%err, "coordinator unavailable for current_proposal_participation");
            })
            .unwrap_or_default()
    }

    pub async fn proposal_participation(
        &self,
        epoch: EpochNumber,
    ) -> HashMap<T::SignatureKey, f64> {
        let Some(client_api) = self.client_api().await else {
            return HashMap::new();
        };
        client_api
            .proposal_participation(Some(epoch))
            .await
            .inspect_err(|err| {
                warn!(%err, "coordinator unavailable for proposal_participation");
            })
            .unwrap_or_default()
    }

    pub async fn current_vote_participation(
        &self,
    ) -> HashMap<<T::SignatureKey as SignatureKey>::VerificationKeyType, f64> {
        let Some(client_api) = self.client_api().await else {
            return HashMap::new();
        };
        client_api
            .vote_participation(None)
            .await
            .inspect_err(|err| {
                warn!(%err, "coordinator unavailable for current_vote_participation");
            })
            .unwrap_or_default()
    }

    pub async fn vote_participation(
        &self,
        epoch: EpochNumber,
    ) -> HashMap<<T::SignatureKey as SignatureKey>::VerificationKeyType, f64> {
        let Some(client_api) = self.client_api().await else {
            return HashMap::new();
        };
        client_api
            .vote_participation(Some(epoch))
            .await
            .inspect_err(|err| {
                warn!(%err, "coordinator unavailable for vote_participation");
            })
            .unwrap_or_default()
    }

    pub async fn request_proposal(
        &self,
        view: ViewNumber,
        leaf_commitment: Commitment<Leaf2<T>>,
    ) -> anyhow::Result<
        BoxFuture<'static, anyhow::Result<SignedProposal<T, QuorumProposalWrapper<T>>>>,
    > {
        let Some(client_api) = self.client_api().await else {
            bail!("consensus is not running");
        };
        Ok(async move {
            client_api
                .request_proposal(view, leaf_commitment)
                .await
                .map(convert_proposal)
                .map_err(anyhow::Error::new)
        }
        .boxed())
    }

    pub async fn submit_transaction(&self, tx: T::Transaction) -> anyhow::Result<()> {
        let Some(client_api) = self.client_api().await else {
            bail!("consensus is not running");
        };
        client_api
            .submit_transaction(tx)
            .await
            .map_err(|err| match err {
                QueryError::Rejected(rejection @ block::SubmitError::TooLarge { .. }) => {
                    SubmitError::Invalid(rejection.to_string()).into()
                },
                QueryError::Rejected(rejection @ block::SubmitError::RetryBufferFull) => {
                    SubmitError::Overloaded(rejection.to_string()).into()
                },
                err => anyhow::Error::new(err),
            })
    }

    pub async fn update_leaf(
        &self,
        leaf: Leaf2<T>,
        state: Arc<T::ValidatedState>,
        delta: Option<Arc<<T::ValidatedState as ValidatedState<T>>::Delta>>,
    ) -> anyhow::Result<()> {
        let view = leaf.view_number();
        let Some(client_api) = self.client_api().await else {
            bail!("consensus is not running");
        };
        client_api
            .update_leaf(UpdateLeaf {
                view,
                leaf,
                state,
                delta,
            })
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// Stops the coordinator if it runs, and drops it if it never started so that nothing can
    /// start it afterwards; the readers serve the resting state from here on.
    pub async fn shut_down(&self) {
        self.membership_coordinator.cancel_all_drb();
        let running = match &*self.new_proto.read() {
            NewProtocol::Running { shutdown, .. } => {
                shutdown.cancel();
                true
            },
            _ => false,
        };
        if running {
            // The coordinator writes the resting state before it exits. Take it only afterwards,
            // so a reader whose request fails meanwhile waits for that state instead of taking
            // the coordinator for dead.
            self.exited.cancelled().await;
        }
        drop(self.new_proto.write().take());
    }

    /// Aborts the coordinator task without touching the handle's state, as a panic in it would.
    #[cfg(test)]
    pub(crate) fn kill_coordinator(&self) {
        if let NewProtocol::Running { coordinator, .. } = &*self.new_proto.read() {
            coordinator.abort();
        }
    }

    pub(crate) async fn client_api(&self) -> Option<ClientApi<T>> {
        if let NewProtocol::Running { client_api, .. } = &*self.new_proto.read() {
            return Some(client_api.clone());
        }
        self.start_consensus().await;
        if let NewProtocol::Running { client_api, .. } = &*self.new_proto.read() {
            Some(client_api.clone())
        } else {
            None
        }
    }
}

async fn run_coordinator<T, S>(
    mut coord: Coordinator<T, S>,
    mut external_rx: UnboundedReceiver<OpaqueMessage<T::SignatureKey>>,
    tx: Sender<CoordinatorEvent<T>>,
    queue_len: Option<Arc<dyn Gauge>>,
    shutdown: CancellationToken,
    exited: CancellationToken,
    resting: Arc<RwLock<Resting<T>>>,
) where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    let _exited = exited.drop_guard();
    coord.start();
    let _forward = AbortOnDropHandle::new(spawn({
        let tx = tx.clone();
        async move {
            while let Some(m) = external_rx.recv().await {
                let e = CoordinatorEvent::ExternalMessageReceived {
                    sender: m.sender,
                    data: m.data,
                };
                broadcast_event(&tx, e).await;
            }
        }
    }));

    loop {
        select! {
            () = shutdown.cancelled() => break,
            it = coord.next_consensus_input() => {
                if let Err(err) = apply_input(&mut coord, &tx, it).await {
                    error!(%err, "coordinator: critical error");
                    break;
                }
                if let Some(m) = &queue_len {
                    m.set(tx.len())
                }
            }
        }
    }

    // Leave the readers what the coordinator holds now. No request reaches it from here on, so
    // nothing it still has queued moves the chain past this.
    let leaf = coord.last_decided_leaf().clone();
    let state = coord
        .state(leaf.view_number())
        .map(|entry| entry.state.clone())
        .unwrap_or_else(|| Arc::new(T::ValidatedState::from_header(leaf.block_header())));
    *resting.write() = Resting {
        view: coord.current_view(),
        leaf,
        state,
    };
    coord.stop().await;
}

async fn apply_input<T, S>(
    coord: &mut Coordinator<T, S>,
    tx: &Sender<CoordinatorEvent<T>>,
    it: Result<ConsensusInput<T>, CoordinatorError>,
) -> Result<(), CoordinatorError>
where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    match it {
        Ok(it) => coord.apply_consensus(it),
        Err(err) => {
            if err.severity == Severity::Critical {
                return Err(err);
            }
            warn!(%err, "coordinator: non-critical error");
        },
    }

    while let Some(out) = coord.outbox_mut().pop_front() {
        if let Some(e) = consensus_event(coord, &out) {
            broadcast_event(tx, e).await;
        }
        if let Err(err) = coord.process_consensus_output(out) {
            if err.severity == Severity::Critical {
                return Err(err);
            }
            warn!(%err, "coordinator: error processing output");
        }
    }

    Ok(())
}

// TODO: `ConsensusOutput::LeafDecided` still carries fields (leaves +
// vid_shares) rather than a `Vec<LeafInfo>`. This is because `Consensus` doesn't own `StateManager`
// state and delta only become available one level up, in `Coordinator`.
fn consensus_event<T, S>(
    coordinator: &Coordinator<T, S>,
    output: &ConsensusOutput<T>,
) -> Option<CoordinatorEvent<T>>
where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    match output {
        ConsensusOutput::LeafDecided {
            leaves,
            cert1,
            cert2,
            vid_shares,
        } => {
            if leaves.is_empty() {
                tracing::error!("coordinator emitted LeafDecided with empty leaves");
                return None;
            }
            let leaf_infos = leaves
                .iter()
                .zip(vid_shares.iter())
                .map(|(leaf, vid_share)| {
                    let (state, delta) = match coordinator.state(leaf.view_number()) {
                        Some(s) => (s.state.clone(), s.delta.clone()),
                        None => {
                            let s = Arc::new(T::ValidatedState::from_header(leaf.block_header()));
                            (s, None)
                        },
                    };
                    let vid_share = vid_share
                        .as_ref()
                        .map(|share| VidDisperseShare::V2(share.data.clone()));
                    LeafInfo::new(leaf.clone(), state, delta, vid_share, None)
                })
                .collect();
            Some(CoordinatorEvent::NewDecide {
                leaf_infos,
                cert1: cert1.clone(),
                cert2: cert2.clone(),
            })
        },
        ConsensusOutput::ProposalValidated { proposal, sender } => {
            Some(CoordinatorEvent::QuorumProposal {
                proposal: Box::new(proposal.clone()),
                sender: sender.clone(),
            })
        },
        ConsensusOutput::BlockPayloadReconstructed {
            view,
            header,
            payload,
        } => Some(CoordinatorEvent::BlockPayloadReconstructed {
            view: *view,
            header: header.clone(),
            payload: Arc::clone(payload),
        }),
        _ => None,
    }
}

async fn broadcast_event<T>(sender: &Sender<CoordinatorEvent<T>>, event: CoordinatorEvent<T>)
where
    T: NodeType,
{
    match sender.broadcast_direct(event).await {
        Ok(None) => {},
        Ok(Some(overflowed)) => {
            warn!(%overflowed, "coordinator event channel overflow, oldest event dropped");
        },
        Err(err) => {
            warn!(%err, "failed to broadcast consensus event");
        },
    }
}
