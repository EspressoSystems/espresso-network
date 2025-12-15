// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::OuterConsensus,
    data::{null_block, PackedBundle, VidCommitment},
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, EventType},
    message::UpgradeLock,
    traits::{
        block_contents::{BlockHeader, BuilderFee},
        node_implementation::{ConsensusTime, NodeType, Versions},
        signature_key::SignatureKey,
        BlockPayload, EncodeBytes,
    },
    utils::{is_epoch_transition, is_last_block, ViewInner},
    vote::HasViewNumber,
};
use hotshot_utils::anytrace::*;
use tracing::instrument;
use vbs::version::{StaticVersionType, Version};

use crate::{
    builder::v0_1::BuilderClient as BuilderClientBase,
    events::{HotShotEvent, HotShotTaskCompleted},
    helpers::broadcast_event,
};

/// Builder Provided Responses
pub struct BuilderResponse<TYPES: NodeType> {
    /// Fee information
    pub fee: BuilderFee<TYPES>,

    /// Block payload
    pub block_payload: TYPES::BlockPayload,

    /// Block metadata
    pub metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
}

#[derive(Default)]
pub struct Mempool<TYPES: NodeType> {
    transactions: Vec<TYPES::Transaction>,
    recently_decided_transactions: HashSet<(TYPES::Transaction, TYPES::View)>,
    recently_proposed_blocks: HashMap<TYPES::View, TYPES::BlockPayload>,
}

impl<TYPES: NodeType> Mempool<TYPES> {
    fn insert_transaction(&mut self, transaction: TYPES::Transaction) {
        self.transactions.push(transaction);
    }

    fn decide_block(
        &mut self,
        view: TYPES::View,
        block_payload: TYPES::BlockPayload,
        metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) {
        for transaction in block_payload.transactions(metadata) {
            self.recently_decided_transactions
                .insert((transaction, view));
        }
        self.recently_proposed_blocks.remove(&view);
    }
}

/// Tracks state of a Transaction task
pub struct BlockTaskState<TYPES: NodeType, V: Versions> {
    /// Output events to application
    pub output_event_stream: async_broadcast::Sender<Event<TYPES>>,

    /// View number this view is executing in.
    pub cur_view: TYPES::View,

    /// Epoch number this node is executing in.
    pub cur_epoch: Option<TYPES::Epoch>,

    /// Reference to consensus. Leader will require a read lock on this.
    pub consensus: OuterConsensus<TYPES>,

    /// Membership for the quorum
    pub membership_coordinator: EpochMembershipCoordinator<TYPES>,

    /// Builder 0.1 API clients
    pub builder_clients: Vec<BuilderClientBase<TYPES>>,

    /// This Nodes Public Key
    pub public_key: TYPES::SignatureKey,

    /// Our Private Key
    pub private_key: <TYPES::SignatureKey as SignatureKey>::PrivateKey,

    /// InstanceState
    pub instance_state: Arc<TYPES::InstanceState>,

    /// This state's ID
    pub id: u64,

    /// Lock for a decided upgrade
    pub upgrade_lock: UpgradeLock<TYPES, V>,

    /// Number of blocks in an epoch, zero means there are no epochs
    pub epoch_height: u64,

    /// Mempool for the block task
    pub mempool: Mempool<TYPES>,
}

async fn vid_from_high_qc<TYPES: NodeType>(
    consensus: &OuterConsensus<TYPES>,
) -> Option<VidCommitment> {
    let consensus_reader = consensus.read().await;
    let high_qc = consensus_reader.high_qc();
    let view_number = high_qc.view_number();
    let view_data = consensus_reader.validated_state_map().get(&view_number)?;
    match &view_data.view_inner {
        ViewInner::Da {
            payload_commitment, ..
        } => Some(*payload_commitment),
        ViewInner::Leaf {
            leaf: leaf_commitment,
            ..
        } => {
            let leaf = consensus_reader.saved_leaves().get(leaf_commitment)?;
            Some(leaf.payload_commitment())
        },
        ViewInner::Failed => None,
    }
}

impl<TYPES: NodeType, V: Versions> BlockTaskState<TYPES, V> {
    /// legacy view change handler
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view), name = "Transaction task", level = "error", target = "TransactionTaskState")]
    pub async fn handle_view_change(
        &mut self,
        event_stream: &Sender<Arc<HotShotEvent<TYPES>>>,
        block_view: TYPES::View,
        block_epoch: Option<TYPES::Epoch>,
        vid: Option<VidCommitment>,
    ) -> Option<HotShotTaskCompleted> {
        let version = match self.upgrade_lock.version(block_view).await {
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
            let Some(epoch) = block_epoch else {
                tracing::error!("Epoch is required for epoch-based view change");
                return None;
            };
            let high_qc = self.consensus.read().await.high_qc().clone();
            let mut high_qc_block_number = if let Some(bn) = high_qc.data.block_number {
                bn
            } else {
                // If it's the first view after the upgrade the high QC won't have a block number
                // So just use the highest_block number we've stored
                if block_view
                    > self
                        .upgrade_lock
                        .upgrade_view()
                        .await
                        .unwrap_or(TYPES::View::new(0))
                        + 1
                {
                    tracing::warn!("High QC in epoch version and not the first QC after upgrade");
                    self.send_empty_block(event_stream, block_view, block_epoch, version)
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
                        "Sending empty block event. View number: {block_view}. Parent Block \
                         number: {high_qc_block_number}"
                    );
                    self.send_empty_block(event_stream, block_view, block_epoch, version)
                        .await;
                    return None;
                }
            }
        }

        let vid = match vid {
            Some(vid) => vid,
            None => {
                let Some(vid) = vid_from_high_qc(&self.consensus).await else {
                    self.send_empty_block(event_stream, block_view, block_epoch, version)
                        .await;
                    return None;
                };
                vid
            },
        };

        // Request a block from the builder unless we are between versions.
        let block = {
            if self
                .upgrade_lock
                .decided_upgrade_certificate
                .read()
                .await
                .as_ref()
                .is_some_and(|cert| cert.upgrading_in(block_view))
            {
                None
            } else {
                self.wait_for_block(block_view, vid).await
            }
        };

        if let Some(BuilderResponse {
            block_payload,
            metadata,
            fee,
        }) = block
        {
            broadcast_event(
                Arc::new(HotShotEvent::BlockRecv(PackedBundle::new(
                    block_payload.encode(),
                    metadata,
                    block_view,
                    block_epoch,
                    vec1::vec1![fee],
                ))),
                event_stream,
            )
            .await;
        } else {
            self.send_empty_block(event_stream, block_view, block_epoch, version)
                .await;
        };

        return None;
    }

    async fn wait_for_block(
        &self,
        block_view: TYPES::View,
        parent_vid: VidCommitment,
    ) -> Option<BuilderResponse<TYPES>> {
        let _previous_block = self
            .wait_for_previous_block(block_view - 1, parent_vid)
            .await;
        None
    }

    async fn wait_for_previous_block(
        &self,
        _parent_view: TYPES::View,
        _parent_vid: VidCommitment,
    ) -> Result<TYPES::BlockPayload> {
        todo!()
    }

    /// Send the event to the event stream that we are proposing an empty block
    async fn send_empty_block(
        &self,
        event_stream: &Sender<Arc<HotShotEvent<TYPES>>>,
        block_view: TYPES::View,
        block_epoch: Option<TYPES::Epoch>,
        version: Version,
    ) {
        // If we couldn't get a block, send an empty block
        tracing::info!("Failed to get a block for view {block_view}, proposing empty block");

        // Increment the metric for number of empty blocks proposed
        self.consensus
            .write()
            .await
            .metrics
            .number_of_empty_blocks_proposed
            .add(1);

        let num_storage_nodes = match self
            .membership_coordinator
            .stake_table_for_epoch(block_epoch)
            .await
        {
            Ok(epoch_stake_table) => epoch_stake_table.total_nodes().await,
            Err(e) => {
                tracing::warn!("Failed to get num_storage_nodes for epoch {block_epoch:?}: {e}");
                return;
            },
        };

        let Some(null_fee) = null_block::builder_fee::<TYPES, V>(num_storage_nodes, version) else {
            tracing::error!("Failed to get null fee");
            return;
        };

        // Create an empty block payload and metadata
        let (_, metadata) = <TYPES as NodeType>::BlockPayload::empty();

        // Broadcast the empty block
        broadcast_event(
            Arc::new(HotShotEvent::BlockRecv(PackedBundle::new(
                vec![].into(),
                metadata,
                block_view,
                block_epoch,
                vec1::vec1![null_fee],
            ))),
            event_stream,
        )
        .await;
    }

    /// Produce a null block
    pub async fn null_block(
        &self,
        block_view: TYPES::View,
        block_epoch: Option<TYPES::Epoch>,
        version: Version,
        num_storage_nodes: usize,
    ) -> Option<PackedBundle<TYPES>> {
        let Some(null_fee) = null_block::builder_fee::<TYPES, V>(num_storage_nodes, version) else {
            tracing::error!("Failed to calculate null block fee.");
            return None;
        };

        // Create an empty block payload and metadata
        let (_, metadata) = <TYPES as NodeType>::BlockPayload::empty();

        Some(PackedBundle::new(
            vec![].into(),
            metadata,
            block_view,
            block_epoch,
            vec1::vec1![null_fee],
        ))
    }

    /// main task event handler
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view, epoch = self.cur_epoch.map(|x| *x)), name = "Transaction task", level = "error", target = "TransactionTaskState")]
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<()> {
        match event.as_ref() {
            HotShotEvent::TransactionsRecv(transactions) => {
                broadcast_event(
                    Event {
                        view_number: self.cur_view,
                        event: EventType::Transactions {
                            transactions: transactions.clone(),
                        },
                    },
                    &self.output_event_stream,
                )
                .await;
            },
            HotShotEvent::ViewChange(view, epoch) => {
                let view = TYPES::View::new(std::cmp::max(1, **view));
                ensure!(
                    *view > *self.cur_view && *epoch >= self.cur_epoch,
                    debug!(
                        "Received a view change to an older view and epoch: tried to change view \
                         to {view}and epoch {epoch:?} though we are at view {} and epoch {:?}",
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
                    self.handle_view_change(&event_stream, view, *epoch, None)
                        .await;
                    return Ok(());
                }
            },
            HotShotEvent::QuorumProposalValidated(proposal, _leaf) => {
                let view_number = proposal.data.view_number();
                let next_view = view_number + 1;

                let version = match self.upgrade_lock.version(next_view).await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("Failed to calculate version: {e:?}");
                        return Ok(());
                    },
                };

                if version < V::DrbAndHeaderUpgrade::VERSION {
                    return Ok(());
                }

                let vid = proposal.data.block_header().payload_commitment();
                let block_height = proposal.data.block_header().block_number();
                if is_epoch_transition(block_height, self.epoch_height) {
                    return Ok(());
                }
                if next_view <= self.cur_view {
                    return Ok(());
                }
                // move to next view for this task only
                self.cur_view = next_view;

                let leader = self
                    .membership_coordinator
                    .membership_for_epoch(self.cur_epoch)
                    .await?
                    .leader(next_view)
                    .await?;
                if leader == self.public_key {
                    self.handle_view_change(&event_stream, next_view, self.cur_epoch, Some(vid))
                        .await;
                    return Ok(());
                }
            },
            _ => {},
        }
        Ok(())
    }
}

#[async_trait]
/// task state implementation for Transactions Task
impl<TYPES: NodeType, V: Versions> TaskState for BlockTaskState<TYPES, V> {
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
