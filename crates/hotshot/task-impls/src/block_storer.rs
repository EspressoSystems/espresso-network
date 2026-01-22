use std::sync::Arc;

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use hotshot_task::task::TaskState;
use hotshot_types::{
    data::DaProposal2,
    message::Proposal,
    traits::{
        node_implementation::{NodeImplementation, NodeType},
        signature_key::SignatureKey,
        storage::Storage,
        BlockPayload, EncodeBytes,
    },
    utils::EpochTransitionIndicator,
};
use sha2::{Digest, Sha256};

use crate::events::HotShotEvent;

pub struct BlockStorerTaskState<TYPES: NodeType, I: NodeImplementation<TYPES>> {
    pub storage: I::Storage,
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,
}

impl<TYPES: NodeType, I: NodeImplementation<TYPES>> BlockStorerTaskState<TYPES, I> {
    fn create_da_proposal(
        &self,
        payload: TYPES::BlockPayload,
        metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
        view: TYPES::View,
    ) -> Proposal<TYPES, DaProposal2<TYPES>> {
        let encoded_transactions = payload.encode();
        let da_proposal = DaProposal2 {
            encoded_transactions: encoded_transactions.clone(),
            metadata,
            view_number: view,
            epoch: None,
            epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
        };
        // quick hash the encoded txns with sha256
        let encoded_transactions_hash = Sha256::digest(encoded_transactions);

        // sign the encoded transactions as opposed to the VID commitment
        let signature = TYPES::SignatureKey::sign(&self.private_key, &encoded_transactions_hash);
        let signature = signature.unwrap();
        Proposal {
            data: da_proposal,
            signature,
            _pd: Default::default(),
        }
    }
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
    ) -> hotshot_utils::anytrace::Result<()> {
        match event.as_ref() {
            HotShotEvent::BlockReconstructed(payload, metadata, commit, view) => {
                tracing::error!("Storing block reconstructed for view {view}");
                let _ = self
                    .storage
                    .append_da2(
                        &self.create_da_proposal(payload.clone(), metadata.clone(), *view),
                        *commit,
                    )
                    .await;
            },
            HotShotEvent::BlockReady(payload_with_metadata, commit, view) => {
                tracing::error!("Storing block ready for view {view}");
                let _ = self
                    .storage
                    .append_da2(
                        &self.create_da_proposal(
                            payload_with_metadata.payload.clone(),
                            payload_with_metadata.metadata.clone(),
                            *view,
                        ),
                        *commit,
                    )
                    .await;
            },
            _ => {},
        }
        Ok(())
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>> TaskState for BlockStorerTaskState<TYPES, I> {
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        _sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> hotshot_utils::anytrace::Result<()> {
        self.handle(event).await
    }

    fn cancel_subtasks(&mut self) {
        // No subtasks to cancel
    }
}
