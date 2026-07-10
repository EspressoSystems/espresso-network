use std::{collections::HashMap, sync::Arc};

use async_broadcast::{InactiveReceiver, Sender};
use async_lock::RwLock;
use committable::Commitment;
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::BoxStream};
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_new_protocol::{
    client::ClientApi,
    consensus::{ConsensusInput, ConsensusOutput, PreCutoverSeed},
    coordinator::{
        Coordinator, CoordinatorOutput,
        error::{CoordinatorError, Severity},
    },
    cutover::{
        CutoverGate, crossed, extract_pre_cutover_seed, forward_legacy_epoch_changes,
        forward_legacy_high_qc, forward_legacy_timeout_votes,
    },
    state::UpdateLeaf,
    storage::NewProtocolStorage,
};
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposalWrapper, VidDisperseShare, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, LeafInfo},
    message::{Proposal as SignedProposal, UpgradeLock, convert_proposal},
    new_protocol::CoordinatorEvent,
    traits::{ValidatedState, node_implementation::NodeType, signature_key::SignatureKey},
    utils::StateAndDelta,
};
use tokio::{select, spawn, sync::watch};
use tokio_util::{sync::CancellationToken, task::AbortOnDropHandle};
use versions::NEW_PROTOCOL_VERSION;

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

fn coordinator_event<T, S>(
    coordinator: &Coordinator<T, S>,
    output: &CoordinatorOutput<T>,
) -> Option<CoordinatorEvent<T>>
where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    match output {
        CoordinatorOutput::Consensus(inner) => consensus_event(coordinator, inner),
        CoordinatorOutput::ExternalMessageReceived { sender, data } => {
            Some(CoordinatorEvent::ExternalMessageReceived {
                sender: sender.clone(),
                data: data.clone(),
            })
        },
    }
}

pub struct ConsensusHandle<T: NodeType, I: NodeImplementation<T>> {
    legacy_handle: Arc<RwLock<SystemContextHandle<T, I>>>,
    client_api: Option<ClientApi<T>>,
    /// Safety net: aborts the coordinator task on drop if `shut_down()` was never called.
    #[allow(dead_code)]
    coordinator_task: Option<AbortOnDropHandle<()>>,
    /// Signals the coordinator loop to stop
    shutdown: CancellationToken,
    /// Cancelled by the coordinator loop once it has stopped
    shutdown_complete: CancellationToken,
    /// Set to `true` when the new protocol is active. This wakes the coordinator loop.
    activated: watch::Sender<bool>,
    epoch_height: u64,
    cutover_gate: CutoverGate,
    legacy_event_rx: InactiveReceiver<Event<T>>,
    event_rx: InactiveReceiver<CoordinatorEvent<T>>,
}

impl<T, I> ConsensusHandle<T, I>
where
    T: NodeType,
    I: NodeImplementation<T>,
{
    /// `coordinator` is `None` if the base version or upgrade version is not set to new protocol version
    pub fn new(
        legacy_handle: Arc<RwLock<SystemContextHandle<T, I>>>,
        coordinator: Option<Coordinator<T, I::Storage>>,
        new_protocol_active: bool,
        epoch_height: u64,
        legacy_event_rx: InactiveReceiver<Event<T>>,
        event_channel_capacity: usize,
    ) -> Self
    where
        I::Storage: NewProtocolStorage<T>,
    {
        let (mut event_tx, mut event_rx) =
            async_broadcast::broadcast::<CoordinatorEvent<T>>(event_channel_capacity);
        event_tx.set_await_active(false);
        event_rx.set_overflow(true);

        let shutdown = CancellationToken::new();
        let shutdown_complete = CancellationToken::new();
        let (activated, activated_rx) = watch::channel(new_protocol_active);

        let (client_api, coordinator_task) = match coordinator {
            Some(coordinator) => {
                let client_api = coordinator.client_api().clone();

                let coordinator_task = AbortOnDropHandle::new(spawn(run_coordinator(
                    coordinator,
                    event_tx,
                    activated_rx,
                    shutdown.clone(),
                    shutdown_complete.clone(),
                )));

                spawn(forward_legacy_timeout_votes(
                    legacy_event_rx.clone(),
                    client_api.clone(),
                ));
                spawn(forward_legacy_high_qc(
                    legacy_event_rx.clone(),
                    client_api.clone(),
                ));
                spawn(forward_legacy_epoch_changes(
                    legacy_event_rx.clone(),
                    client_api.clone(),
                    epoch_height,
                ));

                (Some(client_api), Some(coordinator_task))
            },
            None => {
                shutdown_complete.cancel();
                (None, None)
            },
        };

        Self {
            legacy_handle,
            client_api,
            coordinator_task,
            shutdown,
            shutdown_complete,
            activated,
            epoch_height,
            cutover_gate: CutoverGate::new(),
            legacy_event_rx,
            event_rx: event_rx.deactivate(),
        }
    }

    pub async fn extract_pre_cutover_seed(&self) -> Option<PreCutoverSeed<T>> {
        let legacy = self.legacy_handle.read().await;
        extract_pre_cutover_seed(&legacy).await
    }

    pub fn legacy_consensus(&self) -> Arc<RwLock<SystemContextHandle<T, I>>> {
        self.legacy_handle.clone()
    }

    fn client_api(&self) -> &ClientApi<T> {
        self.client_api
            .as_ref()
            .expect("cutover active without a coordinator")
    }

    /// Whether `view` is at or past the new-protocol upgrade boundary,
    /// according to the legacy upgrade lock. This is a stateless version
    /// check — use it when routing per-view queries like `state(view)`.
    /// For "should we route to the coordinator?" use [`cutover_active`](Self::cutover_active).
    async fn at_or_past_cutover(&self, view: ViewNumber) -> bool {
        self.legacy_handle
            .read()
            .await
            .hotshot
            .upgrade_lock
            .version_infallible(view)
            >= NEW_PROTOCOL_VERSION
    }

    /// Whether the cutover has happened — the gate has latched on this
    /// node. Stateful: the first call after legacy crosses the cutover
    /// view triggers seed extraction + dispatch. Use this for "should
    /// we route to the coordinator?" decisions.
    pub async fn cutover_active(&self) -> bool {
        let Some(client_api) = &self.client_api else {
            return false;
        };
        if self.cutover_gate.is_active() {
            return true;
        }
        let legacy = self.legacy_handle.read().await;
        if !crossed(&legacy).await {
            return false;
        }
        // Wake the coordinator loop
        self.activated.send_replace(true);
        self.cutover_gate.check(&legacy, client_api).await
    }

    pub fn event_stream(&self) -> BoxStream<'static, CoordinatorEvent<T>> {
        let old_stream = self
            .legacy_event_rx
            .activate_cloned()
            .map(CoordinatorEvent::LegacyEvent);

        let new_stream = self.event_rx.activate_cloned();

        futures::stream::select(old_stream, new_stream).boxed()
    }

    pub async fn current_view(&self) -> ViewNumber {
        if self.cutover_active().await {
            return self
                .client_api()
                .current_view()
                .await
                .expect("coordinator channel closed");
        }
        self.legacy_handle.read().await.cur_view().await
    }

    pub async fn decided_leaf(&self) -> Leaf2<T> {
        if self.cutover_active().await {
            return self
                .client_api()
                .decided_leaf()
                .await
                .expect("coordinator channel closed");
        }
        self.legacy_handle.read().await.decided_leaf().await
    }

    pub async fn decided_state(&self) -> Option<Arc<T::ValidatedState>> {
        if self.cutover_active().await {
            return match self.client_api().decided_state().await {
                Ok(state) => state,
                Err(err) => {
                    tracing::warn!("coordinator unavailable for decided_state: {err:#}");
                    None
                },
            };
        }
        Some(self.legacy_handle.read().await.decided_state().await)
    }

    pub async fn state(&self, view: ViewNumber) -> Option<Arc<T::ValidatedState>> {
        if self.at_or_past_cutover(view).await {
            return match self.client_api().state(view).await {
                Ok(state) => state,
                Err(err) => {
                    tracing::warn!(%view, "coordinator unavailable for state: {err:#}");
                    None
                },
            };
        }
        self.legacy_handle.read().await.state(view).await
    }

    pub async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<T> {
        if self.at_or_past_cutover(view).await {
            return match self.client_api().state_and_delta(view).await {
                Ok(state_and_delta) => state_and_delta,
                Err(err) => {
                    tracing::warn!(%view, "coordinator unavailable for state_and_delta: {err:#}");
                    (None, None)
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
            .state_and_delta(view)
    }

    pub async fn undecided_leaves(&self) -> Vec<Leaf2<T>> {
        if self.cutover_active().await {
            return match self.client_api().undecided_leaves().await {
                Ok(leaves) => leaves,
                Err(err) => {
                    tracing::warn!("coordinator unavailable for undecided_leaves: {err:#}");
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
        if self.cutover_active().await {
            return match self.client_api().current_epoch().await {
                Ok(epoch) => epoch,
                Err(err) => {
                    tracing::warn!("coordinator unavailable for current_epoch: {err:#}");
                    None
                },
            };
        }
        self.legacy_handle.read().await.cur_epoch().await
    }

    pub async fn epoch_height(&self) -> u64 {
        if self.cutover_active().await {
            return self.epoch_height;
        }
        self.legacy_handle.read().await.epoch_height
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
        if self.at_or_past_cutover(view).await {
            let client_api = self.client_api().clone();
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
        Ok(async move { future.await.map_err(|e| anyhow::anyhow!("{e}")) }.boxed())
    }

    pub async fn submit_transaction(&self, tx: T::Transaction) -> anyhow::Result<()> {
        let view = self.current_view().await;
        if self.at_or_past_cutover(view).await {
            return self
                .client_api()
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
        if self.at_or_past_cutover(view).await {
            return self
                .client_api()
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
        if self.cutover_active().await {
            // New protocol consensus is already running via the coordinator task.
            // Don't start legacy HotShot consensus tasks.
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
        self.shutdown.cancel();
        self.shutdown_complete.cancelled().await;
        self.legacy_handle.write().await.shut_down().await;
    }
}

async fn run_coordinator<T, S>(
    mut coord: Coordinator<T, S>,
    tx: Sender<CoordinatorEvent<T>>,
    mut activated: watch::Receiver<bool>,
    shutdown: CancellationToken,
    shutdown_complete: CancellationToken,
) where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    let _done = shutdown_complete.drop_guard();

    if *activated.borrow() {
        // Already on the new protocol
        coord.start();
    } else {
        select! {
            () = shutdown.cancelled() => {
                tracing::info!("shutdown before new protocol activation");
                return;
            },
            res = activated.wait_for(|active| *active) => {
                if res.is_err() {
                    tracing::info!("activation channel closed without shutdown");
                    return;
                }
            },
        }
    }

    loop {
        let input = select! {
            () = shutdown.cancelled() => break,
            input = coord.next_consensus_input() => input,
        };
        if let Err(err) = apply_input(&mut coord, &tx, input).await {
            tracing::error!(%err, "coordinator: critical error");
            break;
        }
    }
    coord.stop().await;
}

async fn apply_input<T, S>(
    coord: &mut Coordinator<T, S>,
    tx: &Sender<CoordinatorEvent<T>>,
    input: Result<ConsensusInput<T>, CoordinatorError>,
) -> Result<(), CoordinatorError>
where
    T: NodeType,
    S: NewProtocolStorage<T>,
{
    match input {
        Ok(input) => coord.apply_consensus(input),
        Err(err) if err.severity == Severity::Critical => return Err(err),
        Err(err) => {
            tracing::warn!(%err, "coordinator: non-critical error");
            return Ok(());
        },
    }

    while let Some(output) = coord.outbox_mut().pop_front() {
        if let Some(event) = consensus_event(coord, &output) {
            broadcast_event(tx, event).await;
        }
        if let Err(err) = coord.process_consensus_output(output) {
            if err.severity == Severity::Critical {
                return Err(err.context("processing consensus output"));
            }
            tracing::warn!(%err, "coordinator: error processing output");
        }
    }

    while let Some(output) = coord.coordinator_outbox_mut().pop_front() {
        if let Some(event) = coordinator_event(coord, &output) {
            broadcast_event(tx, event).await;
        }
    }

    Ok(())
}

async fn broadcast_event<T>(sender: &Sender<CoordinatorEvent<T>>, event: CoordinatorEvent<T>)
where
    T: NodeType,
{
    match sender.broadcast_direct(event).await {
        Ok(None) => {},
        Ok(Some(overflowed)) => {
            tracing::warn!(
                %overflowed,
                "coordinator event channel overflow, oldest event dropped"
            );
        },
        Err(err) => {
            tracing::warn!(%err, "failed to broadcast consensus event");
        },
    }
}
