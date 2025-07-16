use std::{collections::HashMap, sync::Arc};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use committable::{Commitment, Committable};
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::OuterConsensus,
    data::{Leaf2, PackedBundle},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    traits::{
        block_contents::{BlockHeader, BuilderFee},
        node_implementation::{ConsensusTime, NodeType, Versions},
        signature_key::BuilderSignatureKey,
        BlockPayload, EncodeBytes, ValidatedState,
    },
    utils::{is_epoch_transition, is_last_block},
};
use hotshot_utils::anytrace::*;
use lru::LruCache;
use vbs::version::{StaticVersionType, Version};

use crate::{
    events::{HotShotEvent, HotShotTaskCompleted},
    helpers::broadcast_event,
    transactions::send_empty_block,
};

pub struct BlockBuilderTaskState<TYPES: NodeType, V: Versions> {
    /// View number this view is executing in.
    pub cur_view: TYPES::View,

    /// Epoch number this node is executing in.
    pub cur_epoch: Option<TYPES::Epoch>,

    /// Membership for the quorum
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,

    /// The consensus state
    pub consensus: OuterConsensus<TYPES>,

    pub transactions: LruCache<Commitment<<TYPES as NodeType>::Transaction>, TYPES::Transaction>,

    /// Instance state
    pub instance_state: Arc<TYPES::InstanceState>,

    /// Base fee
    pub base_fee: u64,

    /// This Nodes Public Key
    pub public_key: TYPES::BuilderSignatureKey,

    /// Our Private Key
    pub private_key: <TYPES::BuilderSignatureKey as BuilderSignatureKey>::BuilderPrivateKey,
}

async fn collect_txns<TYPES: NodeType>(
    proposed_leaf: &Leaf2<TYPES>,
    consensus: &OuterConsensus<TYPES>,
) -> HashMap<Commitment<<TYPES as NodeType>::Transaction>, TYPES::Transaction> {
    let mut txns = HashMap::new();
    let consensus_reader = consensus.read().await;

    // We've reached decide, now get the leaf chain all the way back to the last decided view, not including it.
    let old_anchor_view = consensus_reader.last_decided_view();
    let mut current_leaf = Some(proposed_leaf.clone());
    while current_leaf
        .as_ref()
        .is_some_and(|leaf| leaf.view_number() > old_anchor_view)
    {
        // unwrap is safe, we just checked that he option is some
        let leaf = &mut current_leaf.unwrap();

        // If the block payload is available for this leaf add the transactions to the set
        if let Some(payload) = consensus_reader.saved_payloads().get(&leaf.view_number()) {
            for txn in payload.payload.transactions(leaf.block_header().metadata()) {
                txns.insert(txn.commit(), txn.clone());
            }
        }

        current_leaf = consensus_reader
            .saved_leaves()
            .get(&leaf.justify_qc().data.leaf_commit)
            .cloned();
    }

    txns
}

impl<TYPES: NodeType, V: Versions> BlockBuilderTaskState<TYPES, V> {
    pub async fn build_block(
        &mut self,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
        event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
        version: Version,
    ) -> Option<HotShotTaskCompleted> {
        let consensus_reader = self.consensus.read().await;
        let Some(proposal) = consensus_reader.last_proposals().get(&view) else {
            tracing::info!("No proposal found for view {view}, sending empty block");
            send_empty_block::<TYPES, V>(
                &self.consensus,
                &self.membership_coordinator,
                &event_stream,
                view,
                epoch,
                version,
            )
            .await;
            return None;
        };
        let leaf = Leaf2::from_quorum_proposal(&proposal.data);
        let in_flight_txns = collect_txns(&leaf, &self.consensus).await;

        let mut block = vec![];
        for (txn_hash, txn) in self.transactions.iter().rev() {
            if !in_flight_txns.contains_key(txn_hash) {
                block.push(txn.clone());
            }
        }

        let maybe_validated_state = match consensus_reader.validated_state_map().get(&view) {
            Some(view) => view.state().cloned(),
            None => None,
        };

        let validated_state = maybe_validated_state
            .unwrap_or_else(|| Arc::new(TYPES::ValidatedState::from_header(leaf.block_header())));

        let Some((payload, metadata)) =
            <TYPES::BlockPayload as BlockPayload<TYPES>>::from_transactions(
                block.into_iter(),
                &validated_state,
                &self.instance_state,
            )
            .await
            .ok()
        else {
            tracing::error!("Failed to build block payload");
            return None;
        };

        let encoded_payload = payload.encode();
        let encoded_txns: Vec<u8> = encoded_payload.to_vec();
        let block_size: u64 = encoded_txns.len() as u64;
        let offered_fee: u64 = self.base_fee * block_size;

        let Some(signature_over_fee_info) =
            TYPES::BuilderSignatureKey::sign_fee(&self.private_key, offered_fee, &metadata).ok()
        else {
            tracing::error!("Failed to sign fee");
            send_empty_block::<TYPES, V>(
                &self.consensus,
                &self.membership_coordinator,
                &event_stream,
                view,
                epoch,
                version,
            )
            .await;
            return None;
        };
        let builder_fee = BuilderFee {
            fee_amount: offered_fee,
            fee_account: self.public_key.clone(),
            fee_signature: signature_over_fee_info,
        };

        broadcast_event(
            Arc::new(HotShotEvent::BlockRecv(PackedBundle::new(
                encoded_payload,
                metadata,
                view,
                epoch,
                vec1::vec1![builder_fee],
            ))),
            &event_stream,
        )
        .await;

        None
    }
    pub async fn handle_view_change(
        &mut self,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
        event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Option<HotShotTaskCompleted> {
        let version = match self.upgrade_lock.version(view).await {
            Ok(v) => v,
            Err(err) => {
                tracing::error!(
                    "Upgrade certificate requires unsupported version, refusing to request \
                     blocks: {err}"
                );
                return None;
            },
        };

        // Short circuit if we are in epochs and we are likely proposing a transition block
        // If it's the first view of the upgrade, we don't need to check for transition blocks
        if version >= V::Epochs::VERSION {
            let Some(epoch) = epoch else {
                tracing::error!("Epoch is required for epoch-based view change");
                return None;
            };
            let high_qc = self.consensus.read().await.high_qc().clone();
            let mut high_qc_block_number = if let Some(bn) = high_qc.data.block_number {
                bn
            } else {
                // If it's the first view after the upgrade the high QC won't have a block number
                // So just use the highest_block number we've stored
                if view
                    > self
                        .upgrade_lock
                        .upgrade_view()
                        .await
                        .unwrap_or(TYPES::View::new(0))
                        + 1
                {
                    tracing::warn!("High QC in epoch version and not the first QC after upgrade");
                    send_empty_block::<TYPES, V>(
                        &self.consensus,
                        &self.membership_coordinator,
                        &event_stream,
                        view,
                        Some(epoch),
                        version,
                    )
                    .await;
                    return None;
                }
                // 0 here so we use the highest block number in the calculation below
                0
            };
            high_qc_block_number = std::cmp::max(
                high_qc_block_number,
                self.consensus.read().await.highest_block,
            );
            if self
                .consensus
                .read()
                .await
                .transition_qc()
                .is_some_and(|qc| {
                    let Some(e) = qc.0.data.epoch else {
                        return false;
                    };
                    e == epoch
                })
                || is_epoch_transition(high_qc_block_number, self.epoch_height)
            {
                // We are proposing a transition block it should be empty
                if !is_last_block(high_qc_block_number, self.epoch_height) {
                    tracing::info!(
                        "Sending empty block event. View number: {view}. Parent Block number: \
                         {high_qc_block_number}"
                    );
                    send_empty_block::<TYPES, V>(
                        &self.consensus,
                        &self.membership_coordinator,
                        &event_stream,
                        view,
                        Some(epoch),
                        version,
                    )
                    .await;
                    return None;
                }
            }
        }
        self.build_block(view, epoch, event_stream, version).await;
        None
    }

    async fn handle_transactions(
        &mut self,
        transactions: &Vec<TYPES::Transaction>,
    ) -> Option<HotShotTaskCompleted> {
        for txn in transactions {
            self.transactions.push(txn.commit(), txn.clone());
        }
        None
    }

    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<()> {
        match event.as_ref() {
            HotShotEvent::TransactionsRecv(transactions) => {
                self.handle_transactions(transactions).await;
            },
            HotShotEvent::ViewChange(view, epoch) => {
                self.handle_view_change(*view, *epoch, sender.clone()).await;
            },
            _ => {},
        }
        Ok(())
    }
}

#[async_trait]
impl<TYPES: NodeType, V: Versions> TaskState for BlockBuilderTaskState<TYPES, V> {
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event, sender.clone()).await
    }

    fn cancel_subtasks(&mut self) {}
}
