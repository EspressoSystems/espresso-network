use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use async_broadcast::InactiveReceiver;
use async_lock::RwLock;
use committable::Commitment;
pub use espresso_types::{CoordinatorEvent, NewDecideEvent};
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::BoxStream};
use hotshot::types::SystemContextHandle;
use hotshot_new_protocol::{
    client::ClientApi,
    consensus::ConsensusOutput,
    coordinator::{Coordinator, CoordinatorOutput, error::Severity},
    state::UpdateLeaf,
    storage::NewProtocolStorage,
};
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposalWrapper, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    event::Event,
    message::{Proposal as SignedProposal, UpgradeLock, convert_proposal},
    traits::{
        ValidatedState, network::ConnectedNetwork, node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    utils::StateAndDelta,
};
use tokio::spawn;
use tokio_util::task::AbortOnDropHandle;
use versions::version;

fn consensus_event<T: NodeType>(
    output: &ConsensusOutput<T>,
    cur_view: &mut ViewNumber,
) -> Option<CoordinatorEvent<T>> {
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
            Some(CoordinatorEvent::NewDecide(NewDecideEvent {
                leaves: leaves.clone(),
                cert1: cert1.clone(),
                cert2: cert2.clone(),
                vid_shares: vid_shares.clone(),
            }))
        },
        ConsensusOutput::ViewChanged(view, _epoch) if *view > *cur_view => {
            *cur_view = *view;
            Some(CoordinatorEvent::ViewChanged { view_number: *view })
        },
        ConsensusOutput::ProposalValidated { proposal, sender } => {
            Some(CoordinatorEvent::QuorumProposal {
                proposal: proposal.clone(),
                sender: sender.clone(),
            })
        },
        _ => None,
    }
}

fn coordinator_event<T: NodeType>(
    output: &CoordinatorOutput<T>,
    cur_view: &mut ViewNumber,
) -> Option<CoordinatorEvent<T>> {
    match output {
        CoordinatorOutput::Consensus(inner) => consensus_event(inner, cur_view),
        CoordinatorOutput::ExternalMessageReceived { sender, data } => {
            Some(CoordinatorEvent::ExternalMessageReceived {
                sender: sender.clone(),
                data: data.clone(),
            })
        },
    }
}

pub struct ConsensusHandle<T: NodeType, I: hotshot::traits::NodeImplementation<T>> {
    legacy_handle: Arc<RwLock<SystemContextHandle<T, I>>>,
    client_api: ClientApi<T>,
    coordinator_task: AbortOnDropHandle<()>,
    epoch_height: u64,
    new_protocol_active: AtomicBool,
    legacy_event_rx: InactiveReceiver<Event<T>>,
    event_rx: InactiveReceiver<CoordinatorEvent<T>>,
}

impl<T: NodeType, I: hotshot::traits::NodeImplementation<T>> ConsensusHandle<T, I> {
    pub fn new<CN: ConnectedNetwork<T::SignatureKey>>(
        legacy_handle: Arc<RwLock<SystemContextHandle<T, I>>>,
        coordinator: Coordinator<T, CN, I::Storage>,
        epoch_height: u64,
        legacy_event_rx: InactiveReceiver<Event<T>>,
        event_channel_capacity: usize,
    ) -> Self
    where
        I::Storage: NewProtocolStorage<T>,
    {
        let client_api = coordinator.client_api().clone();

        let (mut event_tx, mut event_rx) =
            async_broadcast::broadcast::<CoordinatorEvent<T>>(event_channel_capacity);
        event_tx.set_await_active(false);
        event_rx.set_overflow(true);

        let coordinator_task =
            AbortOnDropHandle::new(spawn(run_coordinator(coordinator, event_tx)));

        Self {
            legacy_handle,
            client_api,
            coordinator_task,
            epoch_height,
            new_protocol_active: AtomicBool::new(false),
            legacy_event_rx,
            event_rx: event_rx.deactivate(),
        }
    }

    pub fn legacy_consensus(&self) -> Arc<RwLock<SystemContextHandle<T, I>>> {
        self.legacy_handle.clone()
    }

    pub fn client_api(&self) -> &ClientApi<T> {
        &self.client_api
    }

    async fn new_protocol_at(&self, view: ViewNumber) -> bool {
        if self.new_protocol_active.load(Ordering::Relaxed) {
            return true;
        }
        let active = self
            .legacy_handle
            .read()
            .await
            .hotshot
            .upgrade_lock
            .version_infallible(view)
            >= version(0, 8);
        if active {
            self.new_protocol_active.store(true, Ordering::Relaxed);
        }
        active
    }

    async fn new_protocol(&self) -> bool {
        if self.new_protocol_active.load(Ordering::Relaxed) {
            return true;
        }
        let view = self.legacy_handle.read().await.cur_view().await;
        self.new_protocol_at(view).await
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
        if self.new_protocol().await {
            return self
                .client_api
                .current_view()
                .await
                .expect("coordinator channel closed");
        }
        self.legacy_handle.read().await.cur_view().await
    }

    pub async fn decided_leaf(&self) -> Leaf2<T> {
        if self.new_protocol().await {
            return self
                .client_api
                .decided_leaf()
                .await
                .expect("coordinator channel closed");
        }
        self.legacy_handle.read().await.decided_leaf().await
    }

    pub async fn decided_state(&self) -> Arc<T::ValidatedState> {
        if self.new_protocol().await {
            return self
                .client_api
                .decided_state()
                .await
                .expect("coordinator channel closed")
                .expect("decided state must exist");
        }
        self.legacy_handle.read().await.decided_state().await
    }

    pub async fn state(&self, view: ViewNumber) -> Option<Arc<T::ValidatedState>> {
        if self.new_protocol_at(view).await {
            return self
                .client_api
                .state(view)
                .await
                .expect("coordinator channel closed");
        }
        self.legacy_handle.read().await.state(view).await
    }

    pub async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<T> {
        if self.new_protocol_at(view).await {
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
        if self.new_protocol().await {
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
        if self.new_protocol().await {
            return self
                .client_api
                .current_epoch()
                .await
                .expect("coordinator channel closed");
        }
        self.legacy_handle.read().await.cur_epoch().await
    }

    pub async fn epoch_height(&self) -> u64 {
        if self.new_protocol().await {
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
        if self.new_protocol_at(view).await {
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
        if self.new_protocol_at(view).await {
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
        if self.new_protocol_at(view).await {
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
        if self.new_protocol().await {
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

async fn run_coordinator<
    T: NodeType,
    CN: ConnectedNetwork<T::SignatureKey>,
    S: NewProtocolStorage<T>,
>(
    mut coordinator: Coordinator<T, CN, S>,
    event_sender: async_broadcast::Sender<CoordinatorEvent<T>>,
) {
    coordinator.start().await;
    //TODO:
    let mut cur_view = ViewNumber::new(0);
    loop {
        match coordinator.next_consensus_input().await {
            Ok(input) => coordinator.apply_consensus(input).await,
            Err(err) if err.severity == Severity::Critical => {
                tracing::error!(%err, "coordinator: critical error");
                return;
            },
            Err(err) => {
                tracing::warn!(%err, "coordinator: non-critical error");
            },
        }
        while let Some(output) = coordinator.outbox_mut().pop_front() {
            if let Some(event) = consensus_event(&output, &mut cur_view) {
                broadcast_event(&event_sender, event).await;
            }
            if let Err(err) = coordinator.process_consensus_output(output).await {
                if err.severity == Severity::Critical {
                    tracing::error!(%err, "coordinator: critical error processing output");
                    return;
                } else {
                    tracing::warn!(%err, "coordinator: error processing output");
                }
            }
        }
        while let Some(output) = coordinator.coordinator_outbox_mut().pop_front() {
            if let Some(event) = coordinator_event(&output, &mut cur_view) {
                broadcast_event(&event_sender, event).await;
            }
        }
    }
}

async fn broadcast_event<T: NodeType>(
    sender: &async_broadcast::Sender<CoordinatorEvent<T>>,
    event: CoordinatorEvent<T>,
) {
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
