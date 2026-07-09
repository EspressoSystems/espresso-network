use std::{collections::HashMap, sync::Arc};

use async_broadcast::InactiveReceiver;
use async_lock::RwLock;
use committable::Commitment;
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::BoxStream};
use hotshot::{traits::NodeImplementation, types::SystemContextHandle};
use hotshot_new_protocol::storage::NewProtocolStorage;
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposalWrapper, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    event::Event,
    message::{Proposal as SignedProposal, UpgradeLock},
    new_protocol::CoordinatorEvent,
    traits::{ValidatedState, node_implementation::NodeType, signature_key::SignatureKey},
    utils::StateAndDelta,
};

pub struct ConsensusHandle<T: NodeType, I: NodeImplementation<T>> {
    legacy_handle: Arc<RwLock<SystemContextHandle<T, I>>>,
    legacy_event_rx: InactiveReceiver<Event<T>>,
    event_rx: InactiveReceiver<CoordinatorEvent<T>>,
}

impl<T, I> ConsensusHandle<T, I>
where
    T: NodeType,
    I: NodeImplementation<T>,
{
    pub fn new(
        legacy_handle: Arc<RwLock<SystemContextHandle<T, I>>>,
        _epoch_height: u64,
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

        Self {
            legacy_handle,
            legacy_event_rx,
            event_rx: event_rx.deactivate(),
        }
    }

    pub fn legacy_consensus(&self) -> Arc<RwLock<SystemContextHandle<T, I>>> {
        self.legacy_handle.clone()
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
        self.legacy_handle.read().await.cur_view().await
    }

    pub async fn decided_leaf(&self) -> Leaf2<T> {
        self.legacy_handle.read().await.decided_leaf().await
    }

    pub async fn decided_state(&self) -> Arc<T::ValidatedState> {
        self.legacy_handle.read().await.decided_state().await
    }

    pub async fn state(&self, view: ViewNumber) -> Option<Arc<T::ValidatedState>> {
        self.legacy_handle.read().await.state(view).await
    }

    pub async fn state_and_delta(&self, view: ViewNumber) -> StateAndDelta<T> {
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
        self.legacy_handle.read().await.cur_epoch().await
    }

    pub async fn epoch_height(&self) -> u64 {
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
        let future = self
            .legacy_handle
            .read()
            .await
            .request_proposal(view, leaf_commitment)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(async move { future.await.map_err(|e| anyhow::anyhow!("{e}")) }.boxed())
    }

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
        self.legacy_handle
            .read()
            .await
            .hotshot
            .start_consensus()
            .await;
    }

    pub async fn shut_down(&self) {
        self.legacy_handle.write().await.shut_down().await;
    }
}
