use std::{
    collections::{BTreeMap, HashMap, HashSet},
    panic::resume_unwind,
    sync::Arc,
    time::Duration,
};

use committable::{Commitment, Committable};
use hotshot::traits::{BlockPayload, ValidatedState as _};
use hotshot_types::{
    consensus::PayloadWithMetadata,
    data::{
        EpochNumber, Leaf2, VidCommitment, ViewNumber, vid_commitment,
        vid_disperse::vid_total_weight,
    },
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    traits::{
        EncodeBytes,
        block_contents::{BuilderFee, Transaction},
        node_implementation::NodeType,
        signature_key::BuilderSignatureKey,
    },
    utils::BuilderCommitment,
};
use tokio::{
    task::{AbortHandle, JoinSet, spawn_blocking},
    time::sleep,
};
use tracing::{error, warn};

use crate::{
    consensus::ConsensusInput,
    helpers::proposal_commitment,
    message::{DedupManifest, Proposal, TransactionMessage},
    state::HeaderRequest,
};

#[derive(Debug, thiserror::Error)]
pub enum BlockError {
    #[error("payload construction failed: {0}")]
    PayloadConstruction(String),

    #[error("stake table unavailable")]
    StakeTableUnavailable,

    #[error("builder signature failed")]
    BuilderSignature,

    #[error("block builder task cancelled")]
    Cancelled,
    #[error("mempool full: {pending} of {limit} bytes pending")]
    MempoolFull { pending: u64, limit: u64 },
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BlockAndHeaderRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub parent_proposal: Proposal<T>,
}

pub struct BlockBuilderOutput<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub payload: PayloadWithMetadata<T>,
    pub parent_proposal: Proposal<T>,
    pub builder_commitment: BuilderCommitment,
    pub builder_fee: BuilderFee<T>,
    pub payload_commitment: VidCommitment,
    pub manifest: DedupManifest<T>,
}

/// Blocks' worth of its own submissions a node buffers: they stay pending until their block is
/// built on, so this covers the blocks in flight plus the next one.
const MEMPOOL_BLOCKS: u64 = 4;

pub struct BlockBuilderConfig {
    /// The chain's largest block, over every configured upgrade. A leader collects up to this
    /// many bytes of transactions for its next block, from all peers first come first served; a
    /// node forwards at most this much per view and holds `MEMPOOL_BLOCKS` times this of its own
    /// submissions.
    pub max_block_size: u64,
    /// Views to wait before forwarding a pending transaction to a leader again.
    pub forward_interval: u64,
    pub ttl: u64,
    pub dedup_window_size: u64,
    pub empty_block_delay: Duration,
}

impl BlockBuilderConfig {
    fn max_mempool_bytes(&self) -> u64 {
        MEMPOOL_BLOCKS * self.max_block_size
    }
}

impl Default for BlockBuilderConfig {
    fn default() -> Self {
        Self {
            max_block_size: 2 * 1024 * 1024,
            forward_interval: 5,
            ttl: 50,
            dedup_window_size: 10,
            empty_block_delay: Duration::from_millis(500),
        }
    }
}

struct RetryEntry<T: NodeType> {
    tx: T::Transaction,
    valid_until: ViewNumber,
    size: u64,
    last_forwarded: Option<ViewNumber>,
}

impl<T: NodeType> RetryEntry<T> {
    fn due_for_forward(&self, view: ViewNumber, interval: u64) -> bool {
        self.last_forwarded
            .is_none_or(|last| view >= last + interval)
    }
}

pub struct BlockBuilder<T: NodeType> {
    instance: Arc<T::InstanceState>,
    membership: EpochMembershipCoordinator<T>,
    retry_pending: HashMap<Commitment<T::Transaction>, RetryEntry<T>>,
    retry_total_bytes: u64,
    leader_buffer: HashMap<Commitment<T::Transaction>, T::Transaction>,
    leader_total_bytes: u64,
    dedups: BTreeMap<ViewNumber, HashSet<Commitment<T::Transaction>>>,
    config: BlockBuilderConfig,
    upgrade_lock: UpgradeLock<T>,
    current_view: ViewNumber,
    // Keyed by (view, parent_proposal commitment) so that two requests for
    // the same view but different parents (e.g. one from
    // `handle_proposal_with_vid_share` and one from
    // `handle_timeout_certificate`) don't dedup against each other.
    calculations: BTreeMap<(ViewNumber, Commitment<Leaf2<T>>), AbortHandle>,
    tasks: JoinSet<Result<BlockBuilderOutput<T>, BlockError>>,
}

impl<T: NodeType> BlockBuilder<T> {
    pub fn new(
        instance: Arc<T::InstanceState>,
        membership: EpochMembershipCoordinator<T>,
        config: BlockBuilderConfig,
        upgrade_lock: UpgradeLock<T>,
    ) -> Self {
        Self {
            instance,
            membership,
            config,
            upgrade_lock,
            retry_pending: HashMap::new(),
            retry_total_bytes: 0,
            leader_buffer: HashMap::new(),
            leader_total_bytes: 0,
            dedups: BTreeMap::new(),
            current_view: ViewNumber::genesis(),
            calculations: BTreeMap::new(),
            tasks: JoinSet::new(),
        }
    }

    pub fn request_block(&mut self, request: BlockAndHeaderRequest<T>) {
        let view = request.view;
        let parent_commitment = proposal_commitment(&request.parent_proposal);
        if self.calculations.contains_key(&(view, parent_commitment)) {
            return;
        }
        let Ok(version) = self.upgrade_lock.version(view) else {
            warn!(%view, "unsupported version");
            return;
        };
        let epoch = request.epoch;
        let buffer = std::mem::take(&mut self.leader_buffer);
        self.leader_total_bytes = 0;
        let instance = self.instance.clone();
        let membership = self.membership.clone();

        let empty_block_delay = self.config.empty_block_delay;

        let handle = self.tasks.spawn(async move {
            // Without this an idle network produces empty blocks as fast as consensus can run
            // them, flooding the coordinator's event queue.
            if buffer.is_empty() {
                sleep(empty_block_delay).await;
            }
            let (hashes, txs): (Vec<_>, Vec<_>) = buffer.into_iter().unzip();
            let manifest = DedupManifest {
                view,
                epoch,
                hashes,
            };

            let validated_state =
                T::ValidatedState::from_header(&request.parent_proposal.block_header);
            let (payload, metadata) =
                T::BlockPayload::from_transactions(txs, &validated_state, &instance)
                    .await
                    .map_err(|e| BlockError::PayloadConstruction(e.to_string()))?;
            let payload: PayloadWithMetadata<T> = PayloadWithMetadata { payload, metadata };

            let total_weight = {
                let target_mem = membership
                    .stake_table_for_epoch(Some(epoch))
                    .map_err(|_| BlockError::StakeTableUnavailable)?;
                vid_total_weight(target_mem.stake_table(), Some(epoch))
            };
            let commitments = spawn_blocking(move || {
                let payload_bytes = payload.payload.encode();
                let metadata_bytes = payload.metadata.encode();
                // The two commitments are independent, and neither can be split:
                // `vid_commitment` erasure-codes the payload (parallel over
                // namespaces internally) and `builder_commitment` is a serial
                // SHA-256 over every transaction. Running them sequentially made the
                // leader pay both in turn on the path that gates its proposal, so
                // the hash rides alongside the erasure code instead, occupying one
                // worker for its duration rather than adding its full wall time.
                let (payload_commitment, builder_commitment) = rayon::join(
                    || {
                        vid_commitment(
                            payload_bytes.as_ref(),
                            metadata_bytes.as_ref(),
                            total_weight,
                            version,
                        )
                    },
                    || payload.payload.builder_commitment(&payload.metadata),
                );
                let block_size = payload_bytes.len() as u64;
                (payload, block_size, payload_commitment, builder_commitment)
            });
            let (payload, block_size, payload_commitment, builder_commitment) =
                match commitments.await {
                    Ok(out) => out,
                    Err(e) if e.is_panic() => resume_unwind(e.into_panic()),
                    Err(_) => return Err(BlockError::Cancelled),
                };
            let (builder_key, builder_private_key) =
                T::BuilderSignatureKey::generated_from_seed_indexed([0u8; 32], 0);
            let offered_fee = block_size;
            let builder_fee = BuilderFee {
                fee_amount: offered_fee,
                fee_account: builder_key,
                fee_signature: T::BuilderSignatureKey::sign_fee(
                    &builder_private_key,
                    offered_fee,
                    &payload.metadata,
                )
                .map_err(|_| BlockError::BuilderSignature)?,
            };
            Ok(BlockBuilderOutput {
                view,
                epoch,
                payload,
                parent_proposal: request.parent_proposal,
                builder_commitment,
                builder_fee,
                payload_commitment,
                manifest,
            })
        });
        self.calculations.insert((view, parent_commitment), handle);
    }

    pub async fn next(&mut self) -> Option<Result<BlockBuilderOutput<T>, BlockError>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => return Some(result),
                Some(Err(err)) => {
                    if err.is_panic() {
                        error!(%err, "block builder task panicked");
                    }
                },
                None => return None,
            }
        }
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        self.calculations.retain(|(view, _), handle| {
            if *view < view_number {
                handle.abort();
                false
            } else {
                true
            }
        });
    }

    pub fn outstanding_transactions(&self) -> (usize, usize) {
        (self.retry_pending.len(), self.retry_total_bytes as usize)
    }

    pub fn on_submit_transaction(&mut self, tx: T::Transaction) -> Result<(), BlockError> {
        let hash = tx.commit();

        if self.retry_pending.contains_key(&hash) {
            return Ok(());
        }

        let size = tx.minimum_block_size();
        if self.retry_total_bytes + size > self.config.max_mempool_bytes() {
            return Err(BlockError::MempoolFull {
                pending: self.retry_total_bytes,
                limit: self.config.max_mempool_bytes(),
            });
        }

        let valid_until = self.current_view + self.config.ttl;

        self.retry_total_bytes += size;
        self.retry_pending.insert(
            hash,
            RetryEntry {
                tx,
                valid_until,
                size,
                last_forwarded: None,
            },
        );
        Ok(())
    }

    pub fn on_transactions(&mut self, msg: TransactionMessage<T>) {
        for tx in msg.transactions {
            let hash = tx.commit();

            if self.dedups.values().any(|hs| hs.contains(&hash)) {
                continue;
            }

            if self.leader_buffer.contains_key(&hash) {
                continue;
            }

            let size = tx.minimum_block_size();
            if self.leader_total_bytes + size > self.config.max_block_size {
                continue;
            }

            self.leader_total_bytes += size;
            self.leader_buffer.insert(hash, tx);
        }
    }

    pub fn on_dedup_manifest(&mut self, manifest: DedupManifest<T>) {
        let DedupManifest { view, hashes, .. } = manifest;
        self.mark_included(view, hashes);
    }

    /// Advances the view and returns the transactions to forward to the next leader. The caller
    /// reports what it sent through [`Self::on_forwarded`].
    pub fn on_view_changed(&mut self, view: ViewNumber) -> Vec<T::Transaction> {
        self.current_view = view;
        self.expire_pending(view);
        self.forward_batch(view)
            .iter()
            .map(|hash| self.retry_pending[hash].tx.clone())
            .collect()
    }

    fn expire_pending(&mut self, view: ViewNumber) {
        let mut expired_bytes = 0u64;
        self.retry_pending.retain(|_, entry| {
            if view > entry.valid_until {
                expired_bytes += entry.size;
                false
            } else {
                true
            }
        });
        self.retry_total_bytes -= expired_bytes;
    }

    /// The transactions to forward to the next leader, at most one block's worth, since the leader
    /// keeps no more than that. A transaction larger than a block is forwarded alone rather than
    /// never.
    ///
    /// Never-forwarded transactions come first, then least recently forwarded, so no transaction
    /// can be crowded out of every batch until its ttl.
    fn forward_batch(&self, view: ViewNumber) -> Vec<Commitment<T::Transaction>> {
        let mut due: Vec<(Option<ViewNumber>, ViewNumber, Commitment<T::Transaction>)> = self
            .retry_pending
            .iter()
            .filter(|(_, entry)| entry.due_for_forward(view, self.config.forward_interval))
            .map(|(hash, entry)| (entry.last_forwarded, entry.valid_until, *hash))
            .collect();
        due.sort_unstable();

        let mut batch = Vec::new();
        let mut bytes = 0u64;
        for (_, _, hash) in due {
            let size = self.retry_pending[&hash].size;
            if bytes > 0 && bytes + size > self.config.max_block_size {
                break;
            }
            bytes += size;
            batch.push(hash);
        }
        batch
    }

    /// Record that `batch` was handed to the network for a leader. Until this is called the
    /// transactions stay due, so a batch with no leader to go to is retried on the next view rather
    /// than after the interval.
    pub fn on_forwarded(&mut self, view: ViewNumber, batch: &[Commitment<T::Transaction>]) {
        for hash in batch {
            if let Some(entry) = self.retry_pending.get_mut(hash) {
                entry.last_forwarded = Some(view);
            }
        }
    }

    /// Call for every block this node proposes or reconstructs, so it stops forwarding the
    /// block's transactions and drops copies that reach it later.
    pub fn on_block_reconstructed(
        &mut self,
        view: ViewNumber,
        tx_commitments: Vec<Commitment<T::Transaction>>,
    ) {
        for hash in &tx_commitments {
            if let Some(entry) = self.retry_pending.remove(hash) {
                self.retry_total_bytes = self.retry_total_bytes.saturating_sub(entry.size);
            }
        }
        self.mark_included(view, tx_commitments);
    }

    fn mark_included(&mut self, view: ViewNumber, hashes: Vec<Commitment<T::Transaction>>) {
        for hash in &hashes {
            if let Some(tx) = self.leader_buffer.remove(hash) {
                self.leader_total_bytes -= tx.minimum_block_size();
            }
        }

        let lower_bound: ViewNumber = self
            .current_view
            .saturating_sub(self.config.dedup_window_size)
            .into();

        if view >= lower_bound {
            self.dedups.entry(view).or_default().extend(hashes);
        }

        self.dedups = self.dedups.split_off(&lower_bound);
    }

    #[cfg(test)]
    pub(crate) fn drain(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
    ) -> (Vec<T::Transaction>, DedupManifest<T>) {
        let (hashes, txs) = self.leader_buffer.drain().unzip();
        self.leader_total_bytes = 0;

        let manifest = DedupManifest {
            view,
            epoch,
            hashes,
        };

        (txs, manifest)
    }
}

impl<T: NodeType> From<&BlockBuilderOutput<T>> for HeaderRequest<T> {
    fn from(output: &BlockBuilderOutput<T>) -> Self {
        HeaderRequest {
            view: output.view,
            epoch: output.epoch,
            parent_proposal: output.parent_proposal.clone(),
            payload_commitment: output.payload_commitment,
            builder_commitment: output.builder_commitment.clone(),
            metadata: output.payload.metadata.clone(),
            builder_fee: output.builder_fee.clone(),
        }
    }
}

impl<T: NodeType> From<BlockBuilderOutput<T>> for ConsensusInput<T> {
    fn from(output: BlockBuilderOutput<T>) -> Self {
        ConsensusInput::BlockBuilt {
            view: output.view,
            epoch: output.epoch,
            payload: output.payload.payload,
            metadata: output.payload.metadata,
            payload_commitment: output.payload_commitment,
        }
    }
}
