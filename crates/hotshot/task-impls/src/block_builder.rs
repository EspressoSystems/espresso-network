use std::{collections::HashMap, sync::Arc, time::Duration};

use alloy::primitives::map::HashSet;
use async_broadcast::{broadcast, Receiver, Sender};
use async_trait::async_trait;
use committable::{Commitment, Committable};
use hotshot_task::{
    dependency::{Dependency, EventDependency},
    task::TaskState,
};
use hotshot_types::{
    consensus::{Consensus, OuterConsensus},
    data::{Leaf2, PackedBundle, QuorumProposalWrapper},
    epoch_membership::EpochMembershipCoordinator,
    message::{Proposal, UpgradeLock},
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
    pub public_key: TYPES::SignatureKey,

    /// This Nodes Public Key
    pub builder_public_key: TYPES::BuilderSignatureKey,

    /// Our Private Key
    pub builder_private_key: <TYPES::BuilderSignatureKey as BuilderSignatureKey>::BuilderPrivateKey,

    /// Transactions that were decided but not seen in the block builder
    pub decided_not_seen_txns: HashSet<Commitment<<TYPES as NodeType>::Transaction>>,
}

fn collect_txns<TYPES: NodeType>(
    proposed_leaf: &Leaf2<TYPES>,
    consensus: &Consensus<TYPES>,
) -> HashMap<Commitment<<TYPES as NodeType>::Transaction>, TYPES::Transaction> {
    let mut txns = HashMap::new();

    // We've reached decide, now get the leaf chain all the way back to the last decided view, not including it.
    let old_anchor_view = consensus.last_decided_view();
    let mut current_leaf = Some(proposed_leaf.clone());
    while current_leaf
        .as_ref()
        .is_some_and(|leaf| leaf.view_number() > old_anchor_view)
    {
        // unwrap is safe, we just checked that he option is some
        let leaf = &mut current_leaf.unwrap();

        // If the block payload is available for this leaf add the transactions to the set
        if let Some(payload) = consensus.saved_payloads().get(&leaf.view_number()) {
            for txn in payload.payload.transactions(leaf.block_header().metadata()) {
                txns.insert(txn.commit(), txn.clone());
            }
        }

        current_leaf = consensus
            .saved_leaves()
            .get(&leaf.justify_qc().data.leaf_commit)
            .cloned();
    }

    txns
}

impl<TYPES: NodeType, V: Versions> BlockBuilderTaskState<TYPES, V> {
    async fn wait_for_transactions(
        &mut self,
        mut receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) -> Vec<TYPES::Transaction> {
        let mut transactions = vec![];
        while let Ok(event) = receiver.try_recv() {
            if let HotShotEvent::TransactionsRecv(txns) = event.as_ref() {
                for txn in txns {
                    if !self.transactions.contains(&txn.commit())
                        && !self.decided_not_seen_txns.contains(&txn.commit())
                    {
                        tracing::warn!(
                            "BlockBuilder: Collected transaction without waiting: {:?}",
                            txn
                        );
                        transactions.push(txn.clone());
                    } else {
                        tracing::warn!(
                            "BlockBuilder: Ignoring duplicate transaction without waiting: {:?}",
                            txn
                        );
                    }
                }
            }
        }

        let start = std::time::Instant::now();

        while let Ok(Ok(event)) =
            tokio::time::timeout(Duration::from_millis(100), receiver.recv()).await
        {
            if let HotShotEvent::TransactionsRecv(txns) = event.as_ref() {
                for txn in txns {
                    if !self.transactions.contains(&txn.commit())
                        && !self.decided_not_seen_txns.contains(&txn.commit())
                    {
                        tracing::warn!(
                            "BlockBuilder: Collected transaction while waiting: {:?}",
                            txn
                        );
                        transactions.push(txn.clone());
                    } else {
                        tracing::warn!(
                            "BlockBuilder: Ignoring duplicate transaction while waiting: {:?}",
                            txn
                        );
                    }
                }
                if !transactions.is_empty() {
                    break;
                }
            }
            if std::time::Instant::now().duration_since(start) > Duration::from_millis(100) {
                tracing::error!("BlockBuilder: Timeout waiting for transactions");
                break;
            }
        }

        transactions
    }

    async fn wait_for_proposal(
        &self,
        view: TYPES::View,
        receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) -> Option<Proposal<TYPES, QuorumProposalWrapper<TYPES>>> {
        let (_, cancel) = broadcast(1);
        let proposal_dep = EventDependency::new(
            receiver,
            cancel,
            "Proposal".to_string(),
            Box::new(move |event| {
                let HotShotEvent::QuorumProposalValidated(proposal, _) = event.as_ref() else {
                    return false;
                };
                proposal.data.view_number() == view
            }),
        );
        let event =
            match tokio::time::timeout(Duration::from_secs(6), proposal_dep.completed()).await {
                Ok(Some(proposal)) => proposal,
                Ok(None) => return None,
                Err(_) => {
                    tracing::error!("BlockBuilder: Proposal timed out");
                    return None;
                },
            };
        if let HotShotEvent::QuorumProposalValidated(proposal, _) = event.as_ref() {
            Some(proposal.clone())
        } else {
            None
        }
    }
    pub async fn build_block(
        &mut self,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
        event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
        receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
        version: Version,
    ) -> Option<HotShotTaskCompleted> {
        tracing::warn!("BlockBuilder: Building block for view {view} and epoch {epoch:?}");

        let consensus_reader = self.consensus.read().await;
        let mut proposal = consensus_reader.last_proposals().get(&(view - 1)).cloned();
        drop(consensus_reader);
        if proposal.is_none() {
            proposal = {
                let Some(proposal) = self.wait_for_proposal(view - 1, receiver.clone()).await
                else {
                    tracing::error!(
                        "BlockBuilder: No proposal found for view {view}, sending empty block"
                    );
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
                tracing::warn!(
                    "BlockBuilder: Found proposal for view {view} after waiting: {:?}",
                    proposal
                );
                Some(proposal)
            };
        }

        let proposal = proposal?;

        let leaf = Leaf2::from_quorum_proposal(&proposal.data);
        let consensus_reader = self.consensus.read().await;
        let in_flight_txns = collect_txns(&leaf, &*consensus_reader);
        drop(consensus_reader);

        let mut block = vec![];
        for (txn_hash, txn) in self.transactions.iter().rev() {
            if !in_flight_txns.contains_key(txn_hash) {
                tracing::warn!("BlockBuilder: Adding transaction to block: {:?}", txn);
                block.push(txn.clone());
            } else {
                tracing::warn!("BlockBuilder: Ignoring in-flight transaction: {:?}", txn);
            }
        }

        if block.is_empty() {
            block = self.wait_for_transactions(receiver.clone()).await;
            tracing::warn!(
                "BlockBuilder: Collected transactions after waiting for view {}: transactions: \
                 {:?}",
                view,
                block
            );
        }

        let consensus_reader = self.consensus.read().await;

        let maybe_validated_state = match consensus_reader.validated_state_map().get(&view) {
            Some(view) => view.state().cloned(),
            None => None,
        };
        drop(consensus_reader);

        let validated_state = maybe_validated_state
            .unwrap_or_else(|| Arc::new(TYPES::ValidatedState::from_header(leaf.block_header())));

        let block_str = format!("{:?}", block);

        let Some((payload, metadata)) =
            <TYPES::BlockPayload as BlockPayload<TYPES>>::from_transactions(
                block.into_iter(),
                &validated_state,
                &self.instance_state,
            )
            .await
            .ok()
        else {
            tracing::error!("BlockBuilder: Failed to build block payload, sending empty block");
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

        let encoded_payload = payload.encode();
        const FEE_AMOUNT: u64 = 0;
        let Some(signature_over_fee_info) =
            TYPES::BuilderSignatureKey::sign_fee(&self.builder_private_key, FEE_AMOUNT, &metadata)
                .ok()
        else {
            tracing::error!("BlockBuilder: Failed to sign fee, sending empty block");
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
            fee_amount: FEE_AMOUNT,
            fee_account: self.builder_public_key.clone(),
            fee_signature: signature_over_fee_info,
        };

        tracing::warn!(
            "BlockBuilder: Broadcasting BlockRecv for view {view} and epoch {epoch:?}, payload \
             size: {}, metadata: {:?}, builder_fee: {:?}, transactions: {}",
            encoded_payload.len(),
            metadata,
            builder_fee,
            block_str
        );
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
        receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
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
                tracing::error!("BlockBuilder: Epoch is required for epoch-based view change");
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
                    tracing::warn!(
                        "BlockBuilder: High QC in epoch version and not the first QC after upgrade"
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
                        "BlockBuilder: Sending empty block event. View number: {view}. Parent \
                         Block number: {high_qc_block_number}"
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
        self.build_block(view, epoch, event_stream, receiver, version)
            .await;
        None
    }

    async fn handle_transactions(
        &mut self,
        transactions: &Vec<TYPES::Transaction>,
    ) -> Option<HotShotTaskCompleted> {
        for txn in transactions {
            // ignore decided txns
            if self.decided_not_seen_txns.remove(&txn.commit()) {
                continue;
            }
            self.transactions.push(txn.commit(), txn.clone());
        }
        None
    }

    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: Sender<Arc<HotShotEvent<TYPES>>>,
        receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<()> {
        match event.as_ref() {
            HotShotEvent::TransactionsRecv(transactions) => {
                tracing::warn!(
                    "BlockBuilder: Handling received transactions: {:?}",
                    transactions
                );
                for txn in transactions {
                    let commit = txn.commit();
                    if !self.transactions.contains(&commit)
                        && !self.decided_not_seen_txns.contains(&commit)
                    {
                        tracing::warn!("BlockBuilder: Rebroadcasting transaction {:?}", txn);
                        broadcast_event(
                            Arc::new(HotShotEvent::TransactionsRecv(transactions.clone())),
                            &sender,
                        )
                        .await;
                    }
                }
                tracing::warn!(
                    "BlockBuilder: Calling handle_transactions: {:?}",
                    transactions
                );
                self.handle_transactions(transactions).await;
            },
            HotShotEvent::ViewChange(view, epoch) => {
                tracing::warn!(
                    "BlockBuilder: Handling view change to view {view} and epoch {epoch:?}"
                );
                let view = TYPES::View::new(std::cmp::max(1, **view));
                ensure!(
                    *view > *self.cur_view && *epoch >= self.cur_epoch,
                    debug!(
                        "Received a view change to an older view and epoch: tried to change view \
                         to {view} and epoch {epoch:?} though we are at view {} and epoch {:?}",
                        self.cur_view, self.cur_epoch
                    )
                );
                self.cur_view = view;
                self.cur_epoch = *epoch;

                let leader = self
                    .membership_coordinator
                    .membership_for_epoch(*epoch)
                    .await?
                    .leader(view)
                    .await?;
                if leader == self.public_key {
                    tracing::warn!(
                        "BlockBuilder: We are the leader for view {view} and epoch {epoch:?}, \
                         calling handle_view_change"
                    );
                    self.handle_view_change(view, *epoch, sender.clone(), receiver.clone())
                        .await;
                    return Ok(());
                } else {
                    tracing::warn!(
                        "BlockBuilder: We are not the leader for view {view} and epoch {epoch:?}, \
                         ignoring"
                    )
                }
            },
            HotShotEvent::ViewDecided(leaves, txns) => {
                tracing::warn!(
                    "BlockBuilder: Handling ViewDecided for view {:?} with {txns:?}",
                    leaves.first().map(|leaf| leaf.view_number())
                );
                for txn in txns {
                    // Remove the txn from our mempool if it's in there, else store it to prevent a later duplicate
                    if self.transactions.pop(txn).is_none() {
                        self.decided_not_seen_txns.insert(*txn);
                    }
                }
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
        receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event, sender.clone(), receiver.clone()).await
    }

    fn cancel_subtasks(&mut self) {}
}
