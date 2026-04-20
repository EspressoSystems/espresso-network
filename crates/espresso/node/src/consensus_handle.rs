use std::{
    collections::HashMap,
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use async_broadcast::InactiveReceiver;
use async_lock::RwLock;
use committable::Commitment;
use futures::{StreamExt, stream::BoxStream};
use hotshot::types::SystemContextHandle;
use hotshot_new_protocol::{
    client::ClientApi,
    consensus::ConsensusOutput,
    coordinator::{Coordinator, CoordinatorOutput, error::Severity},
    message::{Certificate2, Proposal as NewProposal},
    state::UpdateLeaf,
};
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposalWrapper, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    event::Event,
    message::{Proposal as SignedProposal, UpgradeLock},
    traits::{
        ValidatedState, network::ConnectedNetwork, node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    utils::StateAndDelta,
};
use tokio::spawn;
use tokio_util::task::AbortOnDropHandle;
use versions::version;

#[derive(Clone, Debug)]
pub struct NewDecideEvent<T: NodeType> {
    pub leaves: Vec<Leaf2<T>>,
    pub cert2: Certificate2<T>,
}

#[derive(Clone, Debug)]
pub enum CoordinatorEvent<T: NodeType> {
    LegacyEvent(Event<T>),
    NewDecide(NewDecideEvent<T>),
    ViewChanged {
        view_number: ViewNumber,
    },
    QuorumProposal {
        proposal: SignedProposal<T, NewProposal<T>>,
        sender: T::SignatureKey,
    },
    ExternalMessageReceived {
        sender: T::SignatureKey,
        data: Vec<u8>,
    },
}

impl<T: NodeType> fmt::Display for CoordinatorEvent<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LegacyEvent(event) => {
                write!(f, "Legacy: {} view={}", event.event, event.view_number)
            },
            Self::NewDecide(event) => {
                write!(f, "NewDecide: view={}", event.leaves[0].view_number())
            },
            Self::ViewChanged { view_number } => {
                write!(f, "ViewChanged: view={view_number}")
            },
            Self::QuorumProposal { proposal, .. } => {
                write!(
                    f,
                    "QuorumProposal: view={} epoch={}",
                    proposal.data.view_number, proposal.data.epoch
                )
            },
            Self::ExternalMessageReceived { .. } => {
                write!(f, "ExternalMessageReceived")
            },
        }
    }
}

fn consensus_event<T: NodeType>(output: &ConsensusOutput<T>) -> Option<CoordinatorEvent<T>> {
    match output {
        ConsensusOutput::LeafDecided { leaves, cert2 } => {
            Some(CoordinatorEvent::NewDecide(NewDecideEvent {
                leaves: leaves.clone(),
                cert2: cert2.clone(),
            }))
        },
        ConsensusOutput::ViewChanged(view, _epoch) => {
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

fn coordinator_event<T: NodeType>(output: &CoordinatorOutput<T>) -> Option<CoordinatorEvent<T>> {
    match output {
        CoordinatorOutput::Consensus(inner) => consensus_event(inner),
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
        coordinator: Coordinator<T, CN>,
        epoch_height: u64,
        legacy_event_rx: InactiveReceiver<Event<T>>,
        event_channel_capacity: usize,
    ) -> Self {
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

    // TODO: implement for new protocol
    pub async fn request_proposal(
        &self,
        view: ViewNumber,
        leaf_commitment: Commitment<Leaf2<T>>,
    ) -> anyhow::Result<
        impl futures::Future<Output = anyhow::Result<SignedProposal<T, QuorumProposalWrapper<T>>>>,
    > {
        let future = self
            .legacy_handle
            .read()
            .await
            .request_proposal(view, leaf_commitment)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(async move { future.await.map_err(|e| anyhow::anyhow!("{e}")) })
    }

    // TODO: implement for new protocol
    pub async fn submit_transaction(&self, tx: T::Transaction) -> anyhow::Result<()> {
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

    // TODO: implement for new protocol
    pub async fn start_consensus(&self) {
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

async fn run_coordinator<T: NodeType, CN: ConnectedNetwork<T::SignatureKey>>(
    mut coordinator: Coordinator<T, CN>,
    event_sender: async_broadcast::Sender<CoordinatorEvent<T>>,
) {
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
            if let Some(event) = consensus_event(&output) {
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
            if let Some(event) = coordinator_event(&output) {
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
