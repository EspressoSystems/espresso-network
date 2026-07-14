use std::{collections::HashMap, mem, sync::Arc};

use async_broadcast::{InactiveReceiver, Sender, broadcast};
use async_lock::RwLock as AsyncRwLock;
use committable::Commitment;
use futures::{
    FutureExt, StreamExt,
    future::BoxFuture,
    stream::{self, BoxStream},
};
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_new_protocol::{
    client::ClientApi,
    consensus::{ConsensusInput, ConsensusOutput, PreCutoverSeed},
    coordinator::{
        Coordinator,
        error::{CoordinatorError, Severity},
    },
    cutover::{
        extract_pre_cutover_seed, forward_legacy_epoch_changes, forward_legacy_high_qc,
        forward_legacy_timeout_votes,
    },
    state::UpdateLeaf,
    storage::NewProtocolStorage,
};
use hotshot_types::{
    data::{BlockNumber, EpochNumber, Leaf2, QuorumProposalWrapper, VidDisperseShare, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, LeafInfo},
    message::{Proposal as SignedProposal, UpgradeLock, convert_proposal},
    new_protocol::CoordinatorEvent,
    traits::{ValidatedState, node_implementation::NodeType, signature_key::SignatureKey},
    utils::StateAndDelta,
};
use parking_lot::RwLock;
use tokio::{select, spawn};
use tokio_util::{sync::CancellationToken, task::AbortOnDropHandle};
use tracing::{error, warn};

type StartFn<T> = Box<
    dyn FnOnce(Option<PreCutoverSeed<T>>, CancellationToken) -> AbortOnDropHandle<()> + Send + Sync,
>;

pub struct ConsensusHandle<T: NodeType, I: NodeImplementation<T>> {
    legacy_handle: Arc<AsyncRwLock<SystemContextHandle<T, I>>>,
    epoch_height: BlockNumber,
    legacy_event_rx: InactiveReceiver<Event<T>>,
    event_rx: InactiveReceiver<CoordinatorEvent<T>>,
    upgrade_lock: UpgradeLock<T>,
    new_proto: RwLock<NewProtocol<T>>,
    tasks: Vec<AbortOnDropHandle<()>>,
}

enum NewProtocol<T: NodeType> {
    Empty,
    Init {
        client_api: ClientApi<T>,
        start: StartFn<T>,
    },
    Running {
        coordinator: AbortOnDropHandle<()>,
        client_api: ClientApi<T>,
        shutdown: CancellationToken,
    },
}

impl<T: NodeType> NewProtocol<T> {
    fn take(&mut self) -> Self {
        mem::replace(self, Self::Empty)
    }
}

impl<T, I> ConsensusHandle<T, I>
where
    T: NodeType,
    I: NodeImplementation<T>,
{
    pub async fn new(
        ctx: Arc<AsyncRwLock<SystemContextHandle<T, I>>>,
        coordinator: Coordinator<T, I::Storage>,
        epoch_height: BlockNumber,
        rx: InactiveReceiver<Event<T>>,
        event_channel_capacity: usize,
    ) -> Self
    where
        I::Storage: NewProtocolStorage<T>,
    {
        let (mut event_tx, mut event_rx) = broadcast(event_channel_capacity);
        event_tx.set_await_active(false);
        event_rx.set_overflow(true);

        let upgrade_lock = ctx.read().await.hotshot.upgrade_lock.clone();

        let client_api = coordinator.client_api();

        let tasks = vec![
            AbortOnDropHandle::new(spawn(forward_legacy_timeout_votes(
                rx.clone(),
                client_api.clone(),
            ))),
            AbortOnDropHandle::new(spawn(forward_legacy_high_qc(
                rx.clone(),
                client_api.clone(),
            ))),
            AbortOnDropHandle::new(spawn(forward_legacy_epoch_changes(
                rx.clone(),
                client_api.clone(),
                epoch_height.into(),
            ))),
        ];

        Self {
            upgrade_lock,
            legacy_handle: ctx,
            epoch_height,
            legacy_event_rx: rx,
            event_rx: event_rx.deactivate(),
            new_proto: RwLock::new(NewProtocol::Init {
                client_api: coordinator.client_api().clone(),
                start: Box::new(move |seed, shutdown| {
                    AbortOnDropHandle::new(spawn(run_coordinator(
                        coordinator,
                        event_tx,
                        seed,
                        shutdown,
                    )))
                }),
            }),
            tasks,
        }
    }

    pub async fn activate(&self) {
        if matches!(*self.new_proto.read(), NewProtocol::Running { .. }) {
            return;
        }

        let view = self.legacy_handle.read().await.cur_view().await;

        if !self.upgrade_lock.new_protocol_active(view) {
            return;
        }

        let seed = {
            let legacy = self.legacy_handle.read().await;
            extract_pre_cutover_seed(&legacy).await
        };

        if seed.is_none() {
            warn!("seed extraction returned None; coordinator will not be seeded");
        }

        let mut new_proto = self.new_proto.write();

        match new_proto.take() {
            NewProtocol::Init { client_api, start } => {
                let shutdown = CancellationToken::new();
                *new_proto = NewProtocol::Running {
                    coordinator: start(seed, shutdown.clone()),
                    client_api,
                    shutdown,
                };
            },
            other => *new_proto = other,
        }
    }

    pub fn legacy_consensus(&self) -> Arc<AsyncRwLock<SystemContextHandle<T, I>>> {
        self.legacy_handle.clone()
    }

    pub fn event_stream(&self) -> BoxStream<'static, CoordinatorEvent<T>> {
        let old_stream = self
            .legacy_event_rx
            .activate_cloned()
            .map(CoordinatorEvent::LegacyEvent);
        let new_stream = self.event_rx.activate_cloned();
        stream::select(old_stream, new_stream).boxed()
    }

    pub async fn current_view(&self) -> ViewNumber {
        if let Some(client_api) = self.client_api().await {
            return client_api
                .current_view()
                .await
                .expect("coordinator channel closed"); // FIXME
        }
        self.legacy_handle.read().await.cur_view().await
    }

    pub async fn decided_leaf(&self) -> Leaf2<T> {
        if let Some(client_api) = self.client_api().await {
            return client_api
                .decided_leaf()
                .await
                .expect("coordinator channel closed"); // FIXME
        }
        self.legacy_handle.read().await.decided_leaf().await
    }

    pub async fn decided_state(&self) -> Option<Arc<T::ValidatedState>> {
        if let Some(client_api) = self.client_api().await {
            return match client_api.decided_state().await {
                Ok(state) => state,
                Err(err) => {
                    warn!(%err, "coordinator unavailable for decided_state");
                    None
                },
            };
        }
        Some(self.legacy_handle.read().await.decided_state().await)
    }

    pub async fn state(&self, view: ViewNumber) -> Option<Arc<T::ValidatedState>> {
        if !self.upgrade_lock.new_protocol_active(view) {
            return self.legacy_handle.read().await.state(view).await;
        }
        match self.client_api().await?.state(view).await {
            Ok(state) => state,
            Err(err) => {
                warn!(%view, %err, "coordinator unavailable for state");
                None
            },
        }
    }

    pub async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<T> {
        if !self.upgrade_lock.new_protocol_active(view) {
            return self
                .legacy_handle
                .read()
                .await
                .hotshot
                .consensus()
                .read()
                .await
                .state_and_delta(view);
        }
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
        if let Some(client_api) = self.client_api().await {
            return match client_api.undecided_leaves().await {
                Ok(leaves) => leaves,
                Err(err) => {
                    warn!(%err, "coordinator unavailable for undecided_leaves");
                    Vec::new()
                },
            };
        }
        self.legacy_handle
            .read()
            .await
            .hotshot
            .consensus()
            .read()
            .await
            .undecided_leaves()
    }

    pub async fn current_epoch(&self) -> Option<EpochNumber> {
        if let Some(client_api) = self.client_api().await {
            return match client_api.current_epoch().await {
                Ok(epoch) => epoch,
                Err(err) => {
                    warn!(%err, "coordinator unavailable for current_epoch");
                    None
                },
            };
        }
        self.legacy_handle.read().await.cur_epoch().await
    }

    pub async fn epoch_height(&self) -> BlockNumber {
        if self.is_new_proto_running() {
            return self.epoch_height;
        }
        self.legacy_handle.read().await.epoch_height.into()
    }

    // TODO: implement for new protocol
    pub async fn membership_coordinator(&self) -> EpochMembershipCoordinator<T> {
        self.legacy_handle
            .read()
            .await
            .membership_coordinator
            .clone()
    }

    // TODO: implement for new protocol
    pub async fn upgrade_lock(&self) -> UpgradeLock<T> {
        self.legacy_handle.read().await.hotshot.upgrade_lock.clone()
    }

    // TODO: implement for new protocol
    pub async fn storage(&self) -> I::Storage {
        self.legacy_handle.read().await.storage()
    }

    // TODO: implement for new protocol
    pub async fn current_proposal_participation(&self) -> HashMap<T::SignatureKey, f64> {
        self.legacy_handle
            .read()
            .await
            .consensus()
            .read()
            .await
            .current_proposal_participation()
    }

    pub async fn proposal_participation(
        &self,
        epoch: EpochNumber,
    ) -> HashMap<T::SignatureKey, f64> {
        self.legacy_handle
            .read()
            .await
            .consensus()
            .read()
            .await
            .proposal_participation(epoch)
    }

    pub async fn current_vote_participation(
        &self,
    ) -> HashMap<<T::SignatureKey as SignatureKey>::VerificationKeyType, f64> {
        self.legacy_handle
            .read()
            .await
            .consensus()
            .read()
            .await
            .current_vote_participation()
    }

    pub async fn vote_participation(
        &self,
        epoch: Option<EpochNumber>,
    ) -> HashMap<<T::SignatureKey as SignatureKey>::VerificationKeyType, f64> {
        self.legacy_handle
            .read()
            .await
            .consensus()
            .read()
            .await
            .vote_participation(epoch)
    }

    pub async fn request_proposal(
        &self,
        view: ViewNumber,
        leaf_commitment: Commitment<Leaf2<T>>,
    ) -> anyhow::Result<
        BoxFuture<'static, anyhow::Result<SignedProposal<T, QuorumProposalWrapper<T>>>>,
    > {
        if self.upgrade_lock.new_protocol_active(view)
            && let Some(client_api) = self.client_api().await
        {
            return Ok(async move {
                client_api
                    .request_proposal(view, leaf_commitment)
                    .await
                    .map(convert_proposal)
                    .map_err(|err| anyhow::anyhow!("{err}"))
            }
            .boxed());
        }

        let future = self
            .legacy_handle
            .read()
            .await
            .request_proposal(view, leaf_commitment)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(future.boxed())
    }

    pub async fn submit_transaction(&self, tx: T::Transaction) -> anyhow::Result<()> {
        if let Some(client_api) = self.client_api().await {
            return client_api
                .submit_transaction(tx)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"));
        }
        self.legacy_handle
            .read()
            .await
            .submit_transaction(tx)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub async fn update_leaf(
        &self,
        leaf: Leaf2<T>,
        state: Arc<T::ValidatedState>,
        delta: Option<Arc<<T::ValidatedState as ValidatedState<T>>::Delta>>,
    ) -> anyhow::Result<()> {
        let view = leaf.view_number();
        if self.upgrade_lock.new_protocol_active(view)
            && let Some(client_api) = self.client_api().await
        {
            return client_api
                .update_leaf(UpdateLeaf {
                    view,
                    leaf,
                    state,
                    delta,
                })
                .await
                .map_err(|e| anyhow::anyhow!("{e}"));
        }
        self.legacy_handle
            .read()
            .await
            .hotshot
            .consensus()
            .write()
            .await
            .update_leaf(leaf, state, delta)
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub async fn start_consensus(&self) {
        self.activate().await;
        if self.is_new_proto_running() {
            return;
        }
        self.legacy_handle
            .read()
            .await
            .hotshot
            .start_consensus()
            .await;
    }

    pub async fn shut_down(&self) {
        for t in &self.tasks {
            t.abort()
        }
        // Release the cliquenet listener port before the slow legacy
        // shutdown; `drop(other)` because a non-binding pattern would not
        // move the value.
        let new_proto = self.new_proto.write().take();
        match new_proto {
            NewProtocol::Running {
                shutdown,
                coordinator,
                ..
            } => {
                shutdown.cancel();
                let _ = coordinator.await;
            },
            other => drop(other),
        }
        self.legacy_handle.write().await.shut_down().await;
    }

    fn is_new_proto_running(&self) -> bool {
        matches!(*self.new_proto.read(), NewProtocol::Running { .. })
    }

    async fn client_api(&self) -> Option<ClientApi<T>> {
        if let NewProtocol::Running { client_api, .. } = &*self.new_proto.read() {
            return Some(client_api.clone());
        }
        self.activate().await;
        if let NewProtocol::Running { client_api, .. } = &*self.new_proto.read() {
            Some(client_api.clone())
        } else {
            None
        }
    }
}

async fn run_coordinator<T, S>(
    mut coord: Coordinator<T, S>,
    tx: Sender<CoordinatorEvent<T>>,
    seed: Option<PreCutoverSeed<T>>,
    shutdown: CancellationToken,
) where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    coord.start(seed);

    loop {
        select! {
            () = shutdown.cancelled() => break,
            it = coord.next_consensus_input() => {
                if let Err(err) = apply_input(&mut coord, &tx, it).await {
                    error!(%err, "coordinator: critical error");
                    break;
                }
            }
        }
    }

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

    while let Some(m) = coord.coordinator_outbox_mut().pop_front() {
        let e = CoordinatorEvent::ExternalMessageReceived {
            sender: m.sender,
            data: m.data,
        };
        broadcast_event(tx, e).await;
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
                proposal: proposal.clone(),
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
            payload: payload.clone(),
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
