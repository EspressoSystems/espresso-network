// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use async_broadcast::{Receiver, Sender};
use async_trait::async_trait;
use hotshot_builder_api::v0_1::block_info::AvailableBlockInfo;
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::OuterConsensus,
    data::{null_block, PackedBundle, VidCommitment},
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, EventType},
    message::UpgradeLock,
    traits::{
        block_contents::{BlockHeader, BuilderFee, EncodeBytes},
        node_implementation::{ConsensusTime, NodeType, Versions},
        signature_key::{BuilderSignatureKey, SignatureKey},
        BlockPayload,
    },
    utils::{is_epoch_transition, is_last_block, ViewInner},
};
use hotshot_utils::anytrace::*;
use tokio::{
    spawn,
    task::JoinSet,
    time::{sleep, timeout},
};
use tokio_util::task::AbortOnDropHandle;
use tracing::instrument;
use vbs::version::{StaticVersionType, Version};

use crate::{
    builder::v0_1::BuilderClient as BuilderClientBase,
    events::{HotShotEvent, HotShotTaskCompleted},
    helpers::broadcast_event,
};

// Parameters for builder querying algorithm

/// Delay between re-tries on unsuccessful calls
const RETRY_DELAY: Duration = Duration::from_millis(100);

/// Builder Provided Responses
pub struct BuilderResponse<TYPES: NodeType> {
    /// Fee information
    pub fee: BuilderFee<TYPES>,

    /// Block payload
    pub block_payload: TYPES::BlockPayload,

    /// Block metadata
    pub metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
}

/// Tracks state of a Transaction task
pub struct TransactionTaskState<TYPES: NodeType, V: Versions> {
    /// The state's api
    pub builder_timeout: Duration,

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
}

impl<TYPES: NodeType, V: Versions> TransactionTaskState<TYPES, V> {
    /// handle view change decide legacy or not
    pub async fn handle_view_change(
        &mut self,
        event_stream: &Sender<Arc<HotShotEvent<TYPES>>>,
        block_view: TYPES::View,
        block_epoch: Option<TYPES::Epoch>,
        vid: Option<VidCommitment>,
    ) -> Option<HotShotTaskCompleted> {
        self.handle_view_change_legacy(event_stream, block_view, block_epoch, vid)
            .await
    }

    /// legacy view change handler
    #[instrument(skip_all, fields(id = self.id, view = *self.cur_view), name = "Transaction task", level = "error", target = "TransactionTaskState")]
    pub async fn handle_view_change_legacy(
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

    /// Get VID commitment for the last successful view before `block_view`.
    /// Returns None if we don't have said commitment recorded.
    #[instrument(skip_all, target = "TransactionTaskState", fields(id = self.id, cur_view = *self.cur_view, block_view = *block_view))]
    async fn last_vid_commitment_retry(
        &self,
        block_view: TYPES::View,
        task_start_time: Instant,
    ) -> Result<(TYPES::View, VidCommitment)> {
        loop {
            match self.last_vid_commitment(block_view).await {
                Ok((view, comm)) => break Ok((view, comm)),
                Err(e) if task_start_time.elapsed() >= self.builder_timeout => break Err(e),
                _ => {
                    // We still have time, will re-try in a bit
                    sleep(RETRY_DELAY).await;
                    continue;
                },
            }
        }
    }

    /// Get VID commitment for the last successful view before `block_view`.
    /// Returns None if we don't have said commitment recorded.
    #[instrument(skip_all, target = "TransactionTaskState", fields(id = self.id, cur_view = *self.cur_view, block_view = *block_view))]
    async fn last_vid_commitment(
        &self,
        block_view: TYPES::View,
    ) -> Result<(TYPES::View, VidCommitment)> {
        let consensus_reader = self.consensus.read().await;
        let mut target_view = TYPES::View::new(block_view.saturating_sub(1));

        loop {
            let view_data = consensus_reader
                .validated_state_map()
                .get(&target_view)
                .context(info!(
                    "Missing record for view {target_view} in validated state",
                ))?;

            match &view_data.view_inner {
                ViewInner::Da {
                    payload_commitment, ..
                } => return Ok((target_view, *payload_commitment)),
                ViewInner::Leaf {
                    leaf: leaf_commitment,
                    ..
                } => {
                    let leaf = consensus_reader
                        .saved_leaves()
                        .get(leaf_commitment)
                        .context(info!(
                            "Missing leaf with commitment {leaf_commitment} for view \
                             {target_view} in saved_leaves",
                        ))?;
                    return Ok((target_view, leaf.payload_commitment()));
                },
                ViewInner::Failed => {
                    // For failed views, backtrack
                    target_view = TYPES::View::new(target_view.checked_sub(1).context(warn!(
                        "Reached genesis. Something is wrong -- have we not decided any blocks \
                         since genesis?"
                    ))?);
                    continue;
                },
            }
        }
    }

    #[instrument(skip_all, fields(id = self.id, cur_view = *self.cur_view, block_view = *block_view), name = "wait_for_block", level = "error")]
    async fn wait_for_block(
        &self,
        block_view: TYPES::View,
        vid: Option<VidCommitment>,
    ) -> Option<BuilderResponse<TYPES>> {
        let task_start_time = Instant::now();

        // Find commitment to the block we want to build upon
        let (parent_view, parent_comm) = if let Some(vid) = vid {
            (block_view - 1, vid)
        } else {
            match self
                .last_vid_commitment_retry(block_view, task_start_time)
                .await
            {
                Ok((v, c)) => (v, c),
                Err(e) => {
                    tracing::warn!("Failed to find last vid commitment in time: {e}");
                    return None;
                },
            }
        };

        let parent_comm_sig = match <<TYPES as NodeType>::SignatureKey as SignatureKey>::sign(
            &self.private_key,
            parent_comm.as_ref(),
        ) {
            Ok(sig) => sig,
            Err(err) => {
                tracing::error!(%err, "Failed to sign block hash");
                return None;
            },
        };

        while task_start_time.elapsed() < self.builder_timeout {
            match timeout(
                self.builder_timeout
                    .saturating_sub(task_start_time.elapsed()),
                self.get_block(parent_comm, parent_view, &parent_comm_sig),
            )
            .await
            {
                // We got a block
                Ok(Ok(block)) => {
                    return Some(block);
                },

                // We failed to get a block
                Ok(Err(err)) => {
                    tracing::info!("Couldn't get a block: {err:#}");
                    // pause a bit
                    sleep(RETRY_DELAY).await;
                    continue;
                },

                // We timed out while getting available blocks
                Err(err) => {
                    tracing::info!(%err, "Timeout while getting available blocks");
                    return None;
                },
            }
        }

        tracing::warn!("could not get a block from the builder in time");
        None
    }

    async fn get_block(
        &self,
        parent_comm: VidCommitment,
        view_number: TYPES::View,
        parent_comm_sig: &<<TYPES as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> anyhow::Result<BuilderResponse<TYPES>> {
        // Create a `JoinSet` that joins tasks to get block information from all of the builder clients
        let mut join_set = JoinSet::new();

        // Create a map so we can later re-associate a task with its builder client
        let mut task_to_client = HashMap::new();

        // Spawn tasks to get block information from all of the builder clients simultaneously
        for client in self.builder_clients.iter() {
            // Clone the things we need in the closure
            let public_key = self.public_key.clone();
            let parent_comm_sig = parent_comm_sig.clone();
            let client = client.clone();
            let client_clone = client.clone();

            // Spawn the task to get block information from the builder client
            let id = join_set
                .spawn(async move {
                    Self::get_block_info_from_builder(
                        &client,
                        &public_key,
                        &parent_comm,
                        view_number,
                        &parent_comm_sig,
                    )
                    .await
                })
                .id();

            // Add the task id to builder client mapping
            task_to_client.insert(id, client_clone);
        }

        // We need this channel to deal with responses as they become completed. This is because the `JoinSet` doesn't
        // return tasks in the order in which they completed if more than one was ready. In our scenario, if one fails,
        // we still want to use the result from the next least latent builder.
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
        let _join_task = AbortOnDropHandle::new(spawn(async move {
            while let Some(result) = join_set.join_next_with_id().await {
                let _ = sender.send(result);
            }
        }));

        // The first builder to return block information should be the closest/least latent. It doesn't include
        // the actual block itself, so we need to ask for it
        while let Some(result) = receiver.recv().await {
            // Match on the result to get the block information
            let (task_id, block_info) = match result {
                Ok((task_id, Ok(block_info))) => (task_id, block_info),
                Ok((_, Err(err))) => {
                    tracing::warn!("Failed to get block info from builder: {err:#}");
                    continue;
                },
                Err(err) => {
                    tracing::warn!("Failed to join task: {err:#}");
                    continue;
                },
            };

            // Get the builder client from the map
            let client = task_to_client
                .get(&task_id)
                .ok_or_else(|| anyhow::anyhow!("missing builder client for task"))?;

            // For each block info,
            for block_info in block_info {
                // Get the actual block from the builder
                let block = match Self::get_block_from_builder(
                    client,
                    &self.public_key,
                    &self.private_key,
                    view_number,
                    &block_info,
                )
                .await
                {
                    Ok(block) => block,
                    Err(err) => {
                        tracing::warn!("Failed to get block from builder: {err:#}");
                        continue;
                    },
                };

                // If we got here, we successfully claimed a valid block
                return Ok(block);
            }
        }

        Err(anyhow::anyhow!("no blocks were successfully claimed"))
    }

    /// Get a block from the specified builder client. These blocks are validated and sorted by size in descending
    /// order (so the largest block is first)
    async fn get_block_info_from_builder(
        client: &BuilderClientBase<TYPES>,
        public_key: &TYPES::SignatureKey,
        parent_comm: &VidCommitment,
        view_number: TYPES::View,
        parent_comm_sig: &<<TYPES as NodeType>::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ) -> anyhow::Result<Vec<AvailableBlockInfo<TYPES>>> {
        // Get the available blocks from the builder
        let mut available_blocks = client
            .available_blocks(
                *parent_comm,
                view_number.u64(),
                public_key.clone(),
                parent_comm_sig,
            )
            .await
            .map_err(|e| anyhow::anyhow!("failed to get available blocks: {e:#}"))?;

        // Return early if no blocks were available
        if available_blocks.is_empty() {
            return Err(anyhow::anyhow!("no blocks were available"));
        }

        // Retain only block info with valid signatures
        available_blocks.retain(|block_info| {
            // Validate the signature over the block info
            if !block_info.sender.validate_block_info_signature(
                &block_info.signature,
                block_info.block_size,
                block_info.offered_fee,
                &block_info.block_hash,
            ) {
                tracing::warn!("Block info signature was invalid");
                return false;
            }
            true
        });

        // Return early if none of them had valid signatures
        if available_blocks.is_empty() {
            anyhow::bail!("no valid block info was received");
        }

        // Sort the blocks by size in descending order so that larger blocks are first
        available_blocks.sort_by(|a, b| b.block_size.cmp(&a.block_size));

        // Return the information about the (first) largest block
        Ok(available_blocks)
    }

    /// Get the actual block from the given builder
    async fn get_block_from_builder(
        client: &BuilderClientBase<TYPES>,
        public_key: &TYPES::SignatureKey,
        private_key: &<TYPES::SignatureKey as SignatureKey>::PrivateKey,
        view_number: TYPES::View,
        block_info: &AvailableBlockInfo<TYPES>,
    ) -> anyhow::Result<BuilderResponse<TYPES>> {
        // Sign the block hash that we're requesting
        let request_signature = <<TYPES as NodeType>::SignatureKey as SignatureKey>::sign(
            private_key,
            block_info.block_hash.as_ref(),
        )
        .map_err(|err| anyhow::anyhow!("failed to sign block hash for claim request: {err:#}"))?;

        // Claim both the block and the block header input
        let (block, header_input) = futures::join! {
            client.claim_block(block_info.block_hash.clone(), view_number.u64(), public_key.clone(), &request_signature),
            client.claim_either_block_header_input(block_info.block_hash.clone(), view_number.u64(), public_key.clone(), &request_signature)
        };

        // Get the block
        let block = block.map_err(|err| anyhow::anyhow!("failed to claim block: {err:#}"))?;

        // Get the block header input
        let header_input = header_input
            .map_err(|err| anyhow::anyhow!("failed to claim block header input: {err:#}"))?;

        // Validate the signature of the header input
        let Some(header_input) =
            header_input.validate_signature_and_get_input(block_info.offered_fee, &block.metadata)
        else {
            anyhow::bail!("failed to validate header input signature");
        };

        // Validate the block's signature
        if !block.validate_signature() {
            anyhow::bail!("failed to validate block signature");
        }

        // Create the builder fee
        let fee = BuilderFee {
            fee_amount: block_info.offered_fee,
            fee_account: header_input.sender,
            fee_signature: header_input.fee_signature,
        };

        // Create and return the response
        Ok(BuilderResponse {
            fee,
            block_payload: block.block_payload,
            metadata: block.metadata,
        })
    }
}

#[async_trait]
/// task state implementation for Transactions Task
impl<TYPES: NodeType, V: Versions> TaskState for TransactionTaskState<TYPES, V> {
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
