use std::{collections::HashMap, sync::Arc};

use async_broadcast::{InactiveReceiver, Sender};
use async_lock::RwLock;
use committable::Commitment;
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::BoxStream};
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_new_protocol::{
    client::ClientApi,
    consensus::{ConsensusOutput, PreCutoverSeed},
    coordinator::{Coordinator, CoordinatorOutput, error::Severity},
    cutover::{
        CutoverGate, extract_pre_cutover_seed, forward_legacy_epoch_changes,
        forward_legacy_timeout_votes,
    },
    network::Network,
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
use tokio::spawn;
use tokio_util::task::AbortOnDropHandle;
use versions::NEW_PROTOCOL_VERSION;

// TODO: `ConsensusOutput::LeafDecided` still carries fields (leaves +
// vid_shares) rather than a `Vec<LeafInfo>`. This is because `Consensus` doesn't own `StateManager`
// state and delta only become available one level up, in `Coordinator`.
fn consensus_event<T, N, S>(
    coordinator: &Coordinator<T, N, S>,
    output: &ConsensusOutput<T>,
) -> Option<CoordinatorEvent<T>>
where
    T: NodeType,
    N: Network<T>,
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

fn coordinator_event<T, N, S>(
    coordinator: &Coordinator<T, N, S>,
    output: &CoordinatorOutput<T>,
) -> Option<CoordinatorEvent<T>>
where
    T: NodeType,
    N: Network<T>,
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
    client_api: ClientApi<T>,
    coordinator_task: AbortOnDropHandle<()>,
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
    pub fn new<N>(
        legacy_handle: Arc<RwLock<SystemContextHandle<T, I>>>,
        coordinator: Coordinator<T, N, I::Storage>,
        epoch_height: u64,
        legacy_event_rx: InactiveReceiver<Event<T>>,
        event_channel_capacity: usize,
    ) -> Self
    where
        N: Network<T> + Send + 'static,
        I::Storage: NewProtocolStorage<T>,
    {
        let client_api = coordinator.client_api().clone();

        let (mut event_tx, mut event_rx) =
            async_broadcast::broadcast::<CoordinatorEvent<T>>(event_channel_capacity);
        event_tx.set_await_active(false);
        event_rx.set_overflow(true);

        let coordinator_task =
            AbortOnDropHandle::new(spawn(run_coordinator(coordinator, event_tx)));

        spawn(forward_legacy_timeout_votes(
            legacy_event_rx.clone(),
            client_api.clone(),
        ));
        spawn(forward_legacy_epoch_changes(
            legacy_event_rx.clone(),
            client_api.clone(),
            epoch_height,
        ));

        Self {
            legacy_handle,
            client_api,
            coordinator_task,
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

    pub fn client_api(&self) -> &ClientApi<T> {
        &self.client_api
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
        if self.cutover_gate.is_active() {
            return true;
        }
        let legacy = self.legacy_handle.read().await;
        self.cutover_gate.check(&legacy, &self.client_api).await
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
                .client_api
                .current_view()
                .await
                .expect("coordinator channel closed");
        }
        self.legacy_handle.read().await.cur_view().await
    }

    pub async fn decided_leaf(&self) -> Leaf2<T> {
        if self.cutover_active().await {
            return self
                .client_api
                .decided_leaf()
                .await
                .expect("coordinator channel closed");
        }
        self.legacy_handle.read().await.decided_leaf().await
    }

    pub async fn decided_state(&self) -> Option<Arc<T::ValidatedState>> {
        if self.cutover_active().await {
            return self
                .client_api
                .decided_state()
                .await
                .expect("coordinator channel closed");
        }
        Some(self.legacy_handle.read().await.decided_state().await)
    }

    pub async fn state(&self, view: ViewNumber) -> Option<Arc<T::ValidatedState>> {
        if self.at_or_past_cutover(view).await {
            return self
                .client_api
                .state(view)
                .await
                .expect("coordinator channel closed");
        }
        self.legacy_handle.read().await.state(view).await
    }

    pub async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<T> {
        if self.at_or_past_cutover(view).await {
            return self
                .client_api
                .state_and_delta(view)
                .await
                .expect("coordinator channel closed");
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
            return self
                .client_api
                .undecided_leaves()
                .await
                .expect("coordinator channel closed");
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
            return self
                .client_api
                .current_epoch()
                .await
                .expect("coordinator channel closed");
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
            let client_api = self.client_api.clone();
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
                .client_api
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
                .client_api
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
        self.coordinator_task.abort();
        self.legacy_handle.write().await.shut_down().await;
    }
}

async fn run_coordinator<T, N, S>(mut coord: Coordinator<T, N, S>, tx: Sender<CoordinatorEvent<T>>)
where
    T: NodeType,
    N: Network<T>,
    S: NewProtocolStorage<T>,
{
    coord.start();

    loop {
        match coord.next_consensus_input().await {
            Ok(input) => coord.apply_consensus(input),
            Err(err) if err.severity == Severity::Critical => {
                tracing::error!(%err, "coordinator: critical error");
                return;
            },
            Err(err) => {
                tracing::warn!(%err, "coordinator: non-critical error");
            },
        }
        while let Some(output) = coord.outbox_mut().pop_front() {
            if let Some(event) = consensus_event(&coord, &output) {
                broadcast_event(&tx, event).await;
            }
            if let Err(err) = coord.process_consensus_output(output) {
                if err.severity == Severity::Critical {
                    tracing::error!(%err, "coordinator: critical error processing output");
                    return;
                } else {
                    tracing::warn!(%err, "coordinator: error processing output");
                }
            }
        }
        while let Some(output) = coord.coordinator_outbox_mut().pop_front() {
            if let Some(event) = coordinator_event(&coord, &output) {
                broadcast_event(&tx, event).await;
            }
        }
    }
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
