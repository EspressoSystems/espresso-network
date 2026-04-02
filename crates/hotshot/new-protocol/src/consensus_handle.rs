use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use async_broadcast::{InactiveReceiver, Sender};
use async_lock::{Mutex, RwLock};
use futures::{StreamExt, stream::BoxStream};
use hotshot::types::SystemContextHandle;
use committable::Commitment;
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposalWrapper, ViewNumber},
    message::Proposal,
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, EventType},
    message::UpgradeLock,
    traits::{
        ValidatedState,
        node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    utils::StateAndDelta,
};

use versions::version;

use crate::{
    consensus::{Consensus, ConsensusOutput},
    coordinator::Coordinator,
    message::Certificate2,
    state::StateManager,
};

#[derive(Clone, Debug)]
pub struct NewDecideEvent<T: NodeType> {
    pub view_number: ViewNumber,
    pub leaves: Vec<Leaf2<T>>,
    pub cert2: Certificate2<T>,
}

#[derive(Clone, Debug)]
pub enum ConsensusEvent<T: NodeType> {
    LegacyEvent(Event<T>),
    NewDecide(NewDecideEvent<T>),
    ViewChanged { view_number: ViewNumber },
    QuorumProposal {
        proposal: Proposal<T, QuorumProposalWrapper<T>>,
        sender: T::SignatureKey,
    },
    ExternalMessageReceived {
        sender: T::SignatureKey,
        data: Vec<u8>,
    },
}

pub fn event_from_output<T: NodeType>(
    output: &ConsensusOutput<T>,
) -> Option<ConsensusEvent<T>> {
    match output {
        ConsensusOutput::LeafDecided { leaves, cert2 } => leaves.first().map(|first_leaf| {
            ConsensusEvent::NewDecide(NewDecideEvent {
                view_number: first_leaf.view_number(),
                leaves: leaves.clone(),
                cert2: cert2.clone(),
            })
        }),
        ConsensusOutput::ViewChanged(view, _epoch) => {
            Some(ConsensusEvent::ViewChanged { view_number: *view })
        },
        ConsensusOutput::SendProposal(..) => None,
        ConsensusOutput::ProposalReceived { proposal, sender } => {
            Some(ConsensusEvent::QuorumProposal {
                proposal: Proposal {
                    data: QuorumProposalWrapper::from(proposal.data.clone()),
                    signature: proposal.signature.clone(),
                    _pd: std::marker::PhantomData,
                },
                sender: sender.clone(),
            })
        },
        ConsensusOutput::ExternalMessageReceived { sender, data } => {
            Some(ConsensusEvent::ExternalMessageReceived {
                sender: sender.clone(),
                data: data.clone(),
            })
        },
        _ => None,
    }
}

pub struct ConsensusHandle<T: NodeType, I: hotshot::traits::NodeImplementation<T>> {
    handle: Arc<RwLock<SystemContextHandle<T, I>>>,
    coordinator_consensus: Arc<RwLock<Consensus<T>>>,
    coordinator_state_manager: Arc<Mutex<StateManager<T>>>,
    coordinator_epoch_height: u64,
    new_protocol_active: AtomicBool,
    event_rx: InactiveReceiver<ConsensusEvent<T>>,
}

impl<T: NodeType, I: hotshot::traits::NodeImplementation<T>> ConsensusHandle<T, I> {
    pub fn new(
        handle: Arc<RwLock<SystemContextHandle<T, I>>>,
        coordinator: &Coordinator<T, I>,
        coordinator_epoch_height: u64,
        event_channel_capacity: usize,
    ) -> (Self, Sender<ConsensusEvent<T>>) {
        let (event_tx, event_rx) = async_broadcast::broadcast(event_channel_capacity);

        let adapter = Self {
            handle,
            coordinator_consensus: coordinator.consensus(),
            coordinator_state_manager: coordinator.state_manager(),
            coordinator_epoch_height,
            new_protocol_active: AtomicBool::new(false),
            event_rx: event_rx.deactivate(),
        };

        (adapter, event_tx)
    }

    pub fn hotshot(&self) -> Arc<RwLock<SystemContextHandle<T, I>>> {
        self.handle.clone()
    }

    pub fn coordinator_consensus(&self) -> Arc<RwLock<Consensus<T>>> {
        self.coordinator_consensus.clone()
    }

    pub fn coordinator_state_manager(&self) -> Arc<Mutex<StateManager<T>>> {
        self.coordinator_state_manager.clone()
    }

    async fn new_protocol_at(&self, view: ViewNumber) -> bool {
        if self.new_protocol_active.load(Ordering::Relaxed) {
            return true;
        }

        // TODO: is this the correct way to check version?
        let active = self
            .handle
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
        let view = self.handle.read().await.cur_view().await;
        self.new_protocol_at(view).await
    }

    pub fn event_stream(&self) -> BoxStream<'static, ConsensusEvent<T>> {
        let handle = self.handle.clone();

        let old_stream = futures::stream::once(async move { handle.read().await.event_stream() })
            .flatten()
            .filter_map(|event| {
                let adapter_event = match &event.event {
                    EventType::Decide { .. } => Some(ConsensusEvent::LegacyEvent(event)),
                    EventType::ViewFinished { view_number } => Some(ConsensusEvent::ViewChanged {
                        view_number: *view_number,
                    }),
                    EventType::QuorumProposal { proposal, sender } => {
                        Some(ConsensusEvent::QuorumProposal {
                            proposal: proposal.clone(),
                            sender: sender.clone(),
                        })
                    },
                    EventType::ExternalMessageReceived { sender, data } => {
                        Some(ConsensusEvent::ExternalMessageReceived {
                            sender: sender.clone(),
                            data: data.clone(),
                        })
                    },
                    _ => None,
                };
                futures::future::ready(adapter_event)
            });

        let new_stream = self.event_rx.activate_cloned();

        futures::stream::select(old_stream, new_stream).boxed()
    }

    pub async fn cur_view(&self) -> ViewNumber {
        if self.new_protocol().await {
            return self.coordinator_consensus.read().await.cur_view();
        }
        self.handle.read().await.cur_view().await
    }

    pub async fn decided_leaf(&self) -> Leaf2<T> {
        if self.new_protocol().await {
            if let Some(leaf) = self.coordinator_consensus.read().await.last_decided_leaf() {
                return leaf.clone();
            }
        }
        self.handle.read().await.decided_leaf().await
    }

    pub async fn decided_state(&self) -> Arc<T::ValidatedState> {
        if self.new_protocol().await {
            let view = self
                .coordinator_consensus
                .read()
                .await
                .last_decided_leaf()
                .map(|leaf| leaf.view_number());
            if let Some(view) = view {
                if let Some(state) = self.coordinator_state_manager.lock().await.get_state(&view) {
                    return state;
                }
            }
        }
        self.handle.read().await.decided_state().await
    }

    pub async fn state(&self, view: ViewNumber) -> Option<Arc<T::ValidatedState>> {
        if self.new_protocol_at(view).await {
            return self.coordinator_state_manager.lock().await.get_state(&view);
        }
        self.handle.read().await.state(view).await
    }

    pub async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<T> {
        if self.new_protocol_at(view).await {
            return self
                .coordinator_state_manager
                .lock()
                .await
                .get_state_and_delta(&view);
        }
        self.handle
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
            return self.coordinator_consensus.read().await.undecided_leaves();
        }
        self.handle
            .read()
            .await
            .hotshot
            .consensus()
            .read()
            .await
            .undecided_leaves()
    }

    pub async fn cur_epoch(&self) -> Option<EpochNumber> {
        if self.new_protocol().await {
            return self.coordinator_consensus.read().await.cur_epoch();
        }
        self.handle.read().await.cur_epoch().await
    }

    pub async fn epoch_height(&self) -> u64 {
        if self.new_protocol().await {
            return self.coordinator_epoch_height;
        }
        self.handle.read().await.epoch_height
    }

    // TODO: implement for new protocol
    pub async fn membership_coordinator(&self) -> EpochMembershipCoordinator<T> {
        self.handle.read().await.membership_coordinator.clone()
    }

    // TODO: implement for new protocol
    pub async fn upgrade_lock(&self) -> UpgradeLock<T> {
        self.handle.read().await.hotshot.upgrade_lock.clone()
    }

    // TODO: implement for new protocol
    pub async fn storage(&self) -> I::Storage {
        self.handle.read().await.storage()
    }

    // TODO: implement for new protocol
    pub async fn current_proposal_participation(&self) -> HashMap<T::SignatureKey, f64> {
        self.handle
            .read()
            .await
            .consensus()
            .read()
            .await
            .current_proposal_participation()
    }

    //TODO: 
    pub async fn proposal_participation(
        &self,
        epoch: EpochNumber,
    ) -> HashMap<T::SignatureKey, f64> {
        self.handle
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
        self.handle
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
        self.handle
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
        impl futures::Future<Output = anyhow::Result<Proposal<T, QuorumProposalWrapper<T>>>>,
    > {
        let future = self
            .handle
            .read()
            .await
            .request_proposal(view, leaf_commitment)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(async move { future.await.map_err(|e| anyhow::anyhow!("{e}")) })
    }

    // TODO: implement for new protocol
    pub async fn submit_transaction(&self, tx: T::Transaction) -> anyhow::Result<()> {
        self.handle
            .read()
            .await
            .submit_transaction(tx)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    // TODO: implement for new protocol
    pub async fn update_leaf(
        &self,
        leaf: Leaf2<T>,
        state: Arc<T::ValidatedState>,
        delta: Option<Arc<<T::ValidatedState as ValidatedState<T>>::Delta>>,
    ) -> anyhow::Result<()> {
        self.handle
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
        self.handle.read().await.hotshot.start_consensus().await;
    }

    // TODO: implement for new protocol
    pub async fn shut_down(&self) {
        self.handle.write().await.shut_down().await;
    }
}
