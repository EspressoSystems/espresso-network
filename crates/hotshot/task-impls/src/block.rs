// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{HashMap, HashSet},
    num::NonZero,
    sync::Arc,
    time::{Duration, Instant},
};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use committable::{Commitment, Committable};
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::{OuterConsensus, PayloadWithMetadata},
    data::{null_block, PackedBundle},
    epoch_membership::EpochMembershipCoordinator,
    event::Event,
    message::UpgradeLock,
    traits::{
        block_contents::{BlockHeader, BuilderFee, Transaction},
        node_implementation::{ConsensusTime, NodeType, Versions},
        signature_key::{BuilderSignatureKey, SignatureKey},
        BlockPayload, EncodeBytes,
    },
    utils::{is_epoch_transition, is_last_block},
    vote::HasViewNumber,
};
use hotshot_utils::anytrace::*;
use tokio::time::timeout;
use tracing::instrument;
use vbs::version::{StaticVersionType, Version};

use crate::{
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

#[derive(Clone)]
struct ReceivedTransaction<TYPES: NodeType> {
    tx: TYPES::Transaction,
    len: u64,
    commit: Commitment<TYPES::Transaction>,
    view: TYPES::View,
}

pub struct Mempool<TYPES: NodeType> {
    max_block_size: u64,
    transactions: Vec<ReceivedTransaction<TYPES>>,
    recently_decided_transactions: lru::LruCache<Commitment<TYPES::Transaction>, bool>,
    recently_proposed_blocks: HashMap<TYPES::View, Vec<TYPES::Transaction>>,
}

impl<TYPES: NodeType> Mempool<TYPES> {
    pub fn new(max_block_size: u64) -> Self {
        Self {
            max_block_size,
            transactions: Vec::new(),
            recently_decided_transactions: lru::LruCache::new(NonZero::new(1000).unwrap()),
            recently_proposed_blocks: HashMap::new(),
        }
    }
    fn receive_transaction(&mut self, transaction: TYPES::Transaction, view: TYPES::View) {
        let now = Instant::now();
        let commit = transaction.commit();
        if self
            .recently_decided_transactions
            .contains(&transaction.commit())
        {
            return;
        }
        let len = transaction.minimum_block_size();
        if len > self.max_block_size {
            return;
        }

        self.transactions.push(ReceivedTransaction {
            tx: transaction,
            len,
            commit,
            view,
        });
        let elapsed = now.elapsed();
        tracing::error!("Received transactions in {elapsed:?}");
    }

    fn decide_block(
        &mut self,
        view: TYPES::View,
        block_payload: &TYPES::BlockPayload,
        metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) {
        let now = Instant::now();
        let txn_set: HashSet<Commitment<TYPES::Transaction>> = block_payload
            .transactions(metadata)
            .map(|tx| tx.commit())
            .collect();
        for txn_commit in &txn_set {
            self.recently_decided_transactions
                .put(txn_commit.clone(), true);
        }
        self.transactions
            .retain(|tx| !txn_set.contains(&tx.commit) && tx.view >= view);
        self.recently_proposed_blocks.remove(&view);
        let elapsed = now.elapsed();
        tracing::error!("Mempool processed block in {elapsed:?}");
    }

    fn receive_block(
        &mut self,
        view: TYPES::View,
        block_payload: TYPES::BlockPayload,
        metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) {
        self.recently_proposed_blocks
            .insert(view, block_payload.transactions(metadata).collect());
    }
}

impl<TYPES: NodeType> Default for Mempool<TYPES> {
    fn default() -> Self {
        Self::new(1 * 1024 * 1024) // 1MB
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

    /// Base fee for the block task
    pub base_fee: u64,

    /// Builder key for the block task
    pub builder_key: TYPES::BuilderSignatureKey,

    /// Builder private key for the block task
    pub builder_private_key: <TYPES::BuilderSignatureKey as BuilderSignatureKey>::BuilderPrivateKey,

    pub max_block_size: u64,
}

impl<TYPES: NodeType, V: Versions> BlockTaskState<TYPES, V> {
    /// legacy view change handler
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view), name = "Transaction task", level = "error", target = "BlockTaskState")]
    pub async fn handle_view_change(
        &mut self,
        event_stream: &Sender<Arc<HotShotEvent<TYPES>>>,
        block_view: TYPES::View,
        block_epoch: Option<TYPES::Epoch>,
        receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
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

        // Build a block unless we are between versions
        let upgrade = self
            .upgrade_lock
            .decided_upgrade_certificate
            .read()
            .await
            .as_ref()
            .is_some_and(|cert| cert.upgrading_in(block_view));
        let block = {
            if upgrade {
                None
            } else {
                self.wait_for_block(block_view, receiver).await
            }
        };

        if let Some(BuilderResponse {
            block_payload,
            metadata,
            fee,
        }) = block
        {
            tracing::error!("broadcasting block to consensus");
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

    async fn handle_block(
        &mut self,
        view: TYPES::View,
        block_payload: TYPES::BlockPayload,
        metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) {
        let _ = self.consensus.write().await.update_saved_payloads(
            view,
            Arc::new(PayloadWithMetadata {
                payload: block_payload.clone(),
                metadata: metadata.clone(),
            }),
        );
        self.mempool.receive_block(view, block_payload, &metadata);
    }
    async fn wait_for_block(
        &mut self,
        block_view: TYPES::View,
        receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) -> Option<BuilderResponse<TYPES>> {
        let now = Instant::now();
        match timeout(
            Duration::from_secs(1),
            self.wait_for_previous_block(block_view - 1, receiver),
        )
        .await
        {
            Ok(Ok((previous_block, metadata))) => {
                let elapsed = now.elapsed();
                tracing::debug!("Waited for previous block in {elapsed:?}");
                self.handle_block(block_view - 1, previous_block, metadata)
                    .await;
            },
            Ok(Err(e)) => {
                tracing::warn!("Failed to get previous block: {e}");
            },
            Err(_) => {
                tracing::warn!(
                    "Timeout waiting for previous block (view {}), proceeding without it",
                    block_view - 1
                );
            },
        }

        let now = Instant::now();
        let PayloadWithMetadata { payload, metadata } = self.build_block(block_view).await?;
        let elapsed = now.elapsed();
        tracing::error!("Built block in {elapsed:?}");

        let now = Instant::now();
        let encoded_payload = payload.encode();
        let encoded_txns: Vec<u8> = encoded_payload.to_vec();
        let block_size: u64 = encoded_txns.len() as u64;
        let offered_fee: u64 = self.base_fee * block_size;

        let Some(signature_over_fee_info) =
            TYPES::BuilderSignatureKey::sign_fee(&self.builder_private_key, offered_fee, &metadata)
                .ok()
        else {
            tracing::error!("Failed to sign fee, sending empty block");
            return None;
        };
        let builder_fee = BuilderFee {
            fee_amount: offered_fee,
            fee_account: self.builder_key.clone(),
            fee_signature: signature_over_fee_info,
        };
        let elapsed = now.elapsed();
        tracing::error!("Created Builder Response in {elapsed:?}");
        Some(BuilderResponse {
            block_payload: payload,
            metadata,
            fee: builder_fee,
        })
    }

    async fn build_block(&mut self, block_view: TYPES::View) -> Option<PayloadWithMetadata<TYPES>> {
        let mut transactions = self.mempool.transactions.clone();
        let mut view = block_view - 1;
        let mut in_flight_txns = HashSet::new();
        while let Some(payload) = self.mempool.recently_proposed_blocks.get(&view) {
            in_flight_txns.extend(payload);
            let Some(proposal) = self
                .consensus
                .read()
                .await
                .last_proposals()
                .get(&view)
                .cloned()
            else {
                break;
            };
            view = proposal.data.justify_qc().view_number();
        }
        transactions.retain(|transaction| !in_flight_txns.contains(&transaction.tx));
        let transactions_to_include = transactions.iter().scan(0, |total_size, tx| {
            let prev_size = *total_size;
            *total_size += tx.len;
            // We will include one transaction over our target block length
            // if it's the first transaction in queue, otherwise we'd have a possible failure
            // state where a single transaction larger than target block state is stuck in
            // queue and we just build empty blocks forever
            if *total_size >= self.max_block_size && prev_size != 0 {
                None
            } else {
                Some(tx.tx.clone())
            }
        });
        let validated_state = self
            .consensus
            .read()
            .await
            .validated_state_map()
            .get(&(block_view - 1))?
            .leaf_and_state()?
            .1
            .clone();
        let (payload, metadata) = <TYPES::BlockPayload as BlockPayload<TYPES>>::from_transactions(
            transactions_to_include,
            &validated_state,
            &self.instance_state,
        )
        .await
        .ok()?;
        Some(PayloadWithMetadata { payload, metadata })
    }

    async fn wait_for_previous_block(
        &mut self,
        parent_view: TYPES::View,
        mut receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<(
        TYPES::BlockPayload,
        <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    )> {
        if let Some(payload_with_metadata) = self
            .consensus
            .read()
            .await
            .saved_payloads()
            .get(&parent_view)
        {
            return Ok((
                payload_with_metadata.payload.clone(),
                payload_with_metadata.metadata.clone(),
            ));
        }
        // TODO: Handle the case where the block is received before this call
        while let Ok(event) = receiver.recv_direct().await {
            match event.as_ref() {
                HotShotEvent::BlockDirectlyRecv(payload_with_metadata, view)
                    if *view == parent_view =>
                {
                    tracing::debug!(
                        "Received block directly for parent view {parent_view}, building block"
                    );
                    return Ok((
                        payload_with_metadata.payload.clone(),
                        payload_with_metadata.metadata.clone(),
                    ));
                },
                HotShotEvent::BlockReconstructed(block, metadata, _, view)
                    if *view == parent_view =>
                {
                    tracing::debug!(
                        "Reconstructed block for parent view {parent_view}, building block"
                    );
                    return Ok((block.clone(), metadata.clone()));
                },
                _ => continue,
            }
        }
        Err(hotshot_utils::anytrace::error!("No block received"))
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
        tracing::error!("Failed to get a block for view {block_view}, proposing empty block");

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
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view, epoch = self.cur_epoch.map(|x| *x)), name = "Block task", level = "error", target = "BlockTaskState")]
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
        receiver: Receiver<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<()> {
        match event.as_ref() {
            HotShotEvent::TransactionsRecv(transactions) => {
                for transaction in transactions {
                    self.mempool.receive_transaction(
                        transaction.clone(),
                        self.consensus.read().await.cur_view(),
                    );
                }
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
                    self.handle_view_change(&event_stream, view, *epoch, receiver)
                        .await;
                    return Ok(());
                }
            },
            HotShotEvent::BlockReconstructed(block, metadata, _, view) => {
                self.handle_block(*view, block.clone(), metadata.clone())
                    .await;
            },
            HotShotEvent::BlockDirectlyRecv(payload_with_metadata, view) => {
                tracing::info!("Received block directly from leader for view {view}");
                self.handle_block(
                    *view,
                    payload_with_metadata.payload.clone(),
                    payload_with_metadata.metadata.clone(),
                )
                .await;
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
                    self.handle_view_change(&event_stream, next_view, self.cur_epoch, receiver)
                        .await;
                    return Ok(());
                }
            },
            HotShotEvent::LeavesDecided(leaves) => {
                for leaf in leaves {
                    if let Some(payload) = self
                        .consensus
                        .read()
                        .await
                        .saved_payloads()
                        .get(&leaf.view_number())
                    {
                        self.mempool.decide_block(
                            leaf.view_number(),
                            &payload.payload,
                            &payload.metadata,
                        );
                    }
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
        receiver: &Receiver<Arc<Self::Event>>,
    ) -> Result<()> {
        self.handle(event, sender.clone(), receiver.clone()).await
    }

    fn cancel_subtasks(&mut self) {}
}
