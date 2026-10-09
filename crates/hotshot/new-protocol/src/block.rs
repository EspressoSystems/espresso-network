use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::{NonZeroU64, NonZeroUsize},
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
use tracing::{debug_span, error, warn};
use versions::{NO_BUILDER_COMMITMENT_VERSION, Version};

use crate::{
    consensus::ConsensusInput,
    helpers::proposal_commitment,
    message::{DedupManifest, Proposal, TransactionMessage},
    network::message_limit,
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
}

/// Why [`BlockBuilder::on_submit_transaction`] refused a transaction.
#[derive(Debug, thiserror::Error)]
pub enum SubmitError {
    /// Bigger than a block or a forwarded message.
    #[error("transaction of {size} bytes exceeds the {limit} byte limit")]
    TooLarge { size: u64, limit: u64 },
    /// The retry buffer is full, retrying later can succeed.
    #[error("transaction retry buffer is full")]
    RetryBufferFull,
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

/// Commitments the leader computes for a block it built.
pub struct BlockCommitments<T: NodeType> {
    pub block_size: u64,
    pub payload_commitment: VidCommitment,
    pub builder_commitment: BuilderCommitment,
    pub hashes: Vec<Commitment<T::Transaction>>,
}

/// The leader's commitment work for a built block, on the proposal's critical path. The
/// `block_build` bench in espresso-types calls this; its spans time each step.
pub fn block_commitments<T: NodeType>(
    payload: &PayloadWithMetadata<T>,
    total_weight: usize,
    version: Version,
) -> BlockCommitments<T> {
    let _span = debug_span!("block_commitments").entered();
    let (payload_bytes, metadata_bytes) =
        debug_span!("encode").in_scope(|| (payload.payload.encode(), payload.metadata.encode()));
    // Independent work, run in parallel rather than paid for in turn on the leader's proposal
    // path.
    let (hashes, (payload_commitment, builder_commitment)) = rayon::join(
        || {
            debug_span!("transaction_commitments")
                .in_scope(|| payload.payload.transaction_commitments(&payload.metadata))
        },
        || {
            rayon::join(
                || {
                    debug_span!("vid_commitment").in_scope(|| {
                        vid_commitment(
                            payload_bytes.as_ref(),
                            metadata_bytes.as_ref(),
                            total_weight,
                            version,
                        )
                    })
                },
                // From 0.7 the header carries no builder commitment, so the leader skips the
                // SHA-256 over the whole payload that produced it.
                || {
                    if version >= NO_BUILDER_COMMITMENT_VERSION {
                        BuilderCommitment::empty()
                    } else {
                        debug_span!("builder_commitment")
                            .in_scope(|| payload.payload.builder_commitment(&payload.metadata))
                    }
                },
            )
        },
    );
    BlockCommitments {
        block_size: payload_bytes.len() as u64,
        payload_commitment,
        builder_commitment,
        hashes,
    }
}

/// Room in a forwarded message for everything but the transactions.
const FORWARD_ENVELOPE_BYTES: u64 = 4096;

/// Views between a send and the first view it targets. The next view's leader takes its block
/// as soon as it pairs this view's proposal, before most of this view's submissions reach it,
/// so a copy sent to it would mostly wait in its pool for a later turn.
const SEND_LEAD: u64 = 2;

pub struct BlockBuilderConfig {
    pub max_retry_bytes: u64,
    /// `max_block_size` per protocol version; a missing version inherits the previous one.
    pub block_sizes: BTreeMap<Version, u64>,
    pub ttl: u64,
    /// Views a leader remembers included transactions. Keep it at least `ttl`, or a node still
    /// resending a transaction gets it included again once leaders have forgotten it.
    pub dedup_window_size: u64,
    pub empty_block_delay: Duration,
    /// How many upcoming leaders each transaction is sent to. A leader holds up to
    /// `fanout + 1` blocks of transactions for its later views.
    pub fanout: NonZeroU64,
}

impl Default for BlockBuilderConfig {
    fn default() -> Self {
        let ttl = 10;
        Self {
            max_retry_bytes: 100 * 1024 * 1024,
            block_sizes: BTreeMap::from([(versions::version(0, 0), 2 * 1024 * 1024)]),
            ttl,
            dedup_window_size: ttl,
            empty_block_delay: Duration::from_millis(500),
            fanout: NonZeroU64::new(2).expect("2 is non-zero"),
        }
    }
}

/// Encoded bytes of transactions that fit in one message of `max_message_size`.
pub fn forward_budget(max_message_size: NonZeroUsize) -> u64 {
    (max_message_size.get() as u64).saturating_sub(FORWARD_ENVELOPE_BYTES)
}

struct RetryEntry<T: NodeType> {
    tx: T::Transaction,
    valid_until: ViewNumber,
    size: u64,
    /// Bytes on the wire, which can exceed `size`.
    encoded_size: u64,
    /// The last view whose leader was sent this transaction.
    sent_until: ViewNumber,
}

struct PoolEntry<T: NodeType> {
    tx: T::Transaction,
    /// The view the transaction was sent for, or the view it arrived in if that is later.
    view: ViewNumber,
}

/// Memory charged per retry entry on top of its block size, so tiny
/// transactions cannot outgrow `max_retry_bytes`. Doubled for table slack.
const fn retry_entry_overhead<T: NodeType>() -> u64 {
    let map = size_of::<(Commitment<T::Transaction>, RetryEntry<T>)>();
    let order = size_of::<(ViewNumber, Commitment<T::Transaction>)>();
    2 * (map + order) as u64
}

pub struct BlockBuilder<T: NodeType> {
    instance: Arc<T::InstanceState>,
    membership: EpochMembershipCoordinator<T>,
    retry_pending: HashMap<Commitment<T::Transaction>, RetryEntry<T>>,
    retry_order: BTreeSet<(ViewNumber, Commitment<T::Transaction>)>,
    retry_total_bytes: u64,
    leader_buffer: HashMap<Commitment<T::Transaction>, PoolEntry<T>>,
    leader_order: BTreeSet<(ViewNumber, Commitment<T::Transaction>)>,
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
    /// The transactions taken for the first block of each view.
    ///
    /// Every later block for the view is built from exactly these. A leader
    /// disperses a block as soon as it is built, and peers keep one share per
    /// leader and view, so a second block for the view is votable only if its
    /// payload is the same. It is the same whenever the new parent does not
    /// change how the payload is built.
    view_transactions: BTreeMap<ViewNumber, Arc<Vec<T::Transaction>>>,
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
            retry_order: BTreeSet::new(),
            retry_total_bytes: 0,
            leader_buffer: HashMap::new(),
            leader_order: BTreeSet::new(),
            leader_total_bytes: 0,
            dedups: BTreeMap::new(),
            current_view: ViewNumber::genesis(),
            calculations: BTreeMap::new(),
            view_transactions: BTreeMap::new(),
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
        let txs = self.transactions_for(view);
        let instance = self.instance.clone();
        let membership = self.membership.clone();

        let empty_block_delay = self.config.empty_block_delay;

        let handle = self.tasks.spawn(async move {
            // Without this an idle network produces empty blocks as fast as consensus can run
            // them, flooding the coordinator's event queue.
            if txs.is_empty() {
                sleep(empty_block_delay).await;
            }

            let validated_state =
                T::ValidatedState::from_header(&request.parent_proposal.block_header);
            let (payload, metadata) =
                T::BlockPayload::from_transactions(&txs, &validated_state, &instance)
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
                let commitments = block_commitments(&payload, total_weight, version);
                (payload, commitments)
            });
            let (
                payload,
                BlockCommitments {
                    block_size,
                    payload_commitment,
                    builder_commitment,
                    hashes,
                },
            ) = match commitments.await {
                Ok(out) => out,
                Err(e) if e.is_panic() => resume_unwind(e.into_panic()),
                Err(_) => return Err(BlockError::Cancelled),
            };
            let manifest = DedupManifest {
                view,
                epoch,
                hashes,
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

    fn transactions_for(&mut self, view: ViewNumber) -> Arc<Vec<T::Transaction>> {
        if let Some(txs) = self.view_transactions.get(&view) {
            return Arc::clone(txs);
        }
        let txs = Arc::new(self.take_block(view));
        self.view_transactions.insert(view, Arc::clone(&txs));
        txs
    }

    /// Removes up to one block of pooled transactions, those sent for the earliest views first.
    fn take_block(&mut self, view: ViewNumber) -> Vec<T::Transaction> {
        let max_bytes = self.block_size(view);
        let mut bytes = 0;
        let mut taken = Vec::new();
        for (_, hash) in &self.leader_order {
            let size = self.leader_buffer[hash].tx.minimum_block_size();
            if bytes + size > max_bytes {
                continue;
            }
            bytes += size;
            taken.push(*hash);
        }
        taken
            .into_iter()
            .map(|hash| {
                self.remove_pooled(&hash)
                    .expect("hashes come from the pool's own order")
            })
            .collect()
    }

    fn remove_pooled(&mut self, hash: &Commitment<T::Transaction>) -> Option<T::Transaction> {
        let entry = self.leader_buffer.remove(hash)?;
        self.leader_order.remove(&(entry.view, *hash));
        self.leader_total_bytes -= entry.tx.minimum_block_size();
        Some(entry.tx)
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
        self.view_transactions = self.view_transactions.split_off(&view_number);
    }

    pub fn fanout(&self) -> NonZeroU64 {
        self.config.fanout
    }

    /// Starts the builder at `view` instead of genesis. Transactions can be submitted before
    /// the first `on_view_changed`, and would otherwise target the leaders of the first views
    /// and expire at the first view change.
    pub fn start_at(&mut self, view: ViewNumber) {
        self.current_view = view;
    }

    pub fn outstanding_transactions(&self) -> (usize, usize) {
        (self.retry_pending.len(), self.retry_total_bytes as usize)
    }

    /// Returns the message for the upcoming leaders, addressed to the view `SEND_LEAD` ahead,
    /// which the coordinator sends on to the leaders of the `fanout` views from there.
    /// Resubmitting a queued transaction succeeds without queueing or sending it twice.
    pub fn on_submit_transaction(
        &mut self,
        tx: T::Transaction,
    ) -> Result<Option<TransactionMessage<T>>, SubmitError> {
        let hash = tx.commit();

        if self.retry_pending.contains_key(&hash) {
            return Ok(None);
        }

        let size = tx.minimum_block_size();
        let encoded_size = bincode::serialized_size(&tx).expect("transactions serialize");
        // Forwarding uses the first target view's block size, which an upgrade can raise.
        let max_bytes = self
            .block_size(self.current_view)
            .max(self.block_size(self.current_view + SEND_LEAD));
        let budget = forward_budget(message_limit(max_bytes));
        if size > max_bytes {
            return Err(SubmitError::TooLarge {
                size,
                limit: max_bytes,
            });
        }
        if encoded_size > budget {
            return Err(SubmitError::TooLarge {
                size: encoded_size,
                limit: budget,
            });
        }
        let charge = size + retry_entry_overhead::<T>();
        if self.retry_total_bytes + charge > self.config.max_retry_bytes {
            warn!("retry buffer full, rejecting {hash}");
            return Err(SubmitError::RetryBufferFull);
        }

        let valid_until = self.current_view + self.config.ttl;
        let first_target = self.current_view + SEND_LEAD;
        let sent_until = first_target + (self.config.fanout.get() - 1);
        let message = TransactionMessage {
            view: first_target,
            transactions: Vec::from([tx.clone()]),
        };

        self.retry_total_bytes += charge;
        self.retry_order.insert((valid_until, hash));
        self.retry_pending.insert(
            hash,
            RetryEntry {
                tx,
                valid_until,
                size,
                encoded_size,
                sent_until,
            },
        );
        Ok(Some(message))
    }

    pub fn on_transactions(&mut self, msg: TransactionMessage<T>) {
        // A sender behind this node may name a view that has passed. Pooling it as of now keeps
        // it from expiring before it can be built.
        let view = msg.view.max(self.current_view);
        let block_size = self.block_size(view);
        let max_bytes = block_size.saturating_mul(self.config.fanout.get() + 1);
        for tx in msg.transactions {
            let hash = tx.commit();

            if self.dedups.values().any(|hs| hs.contains(&hash)) {
                continue;
            }

            if self.leader_buffer.contains_key(&hash) {
                continue;
            }

            let size = tx.minimum_block_size();
            // It could never be built, and would hold pool space until it expires.
            if size > block_size {
                continue;
            }
            if self.leader_total_bytes + size > max_bytes {
                continue;
            }

            self.leader_total_bytes += size;
            self.leader_order.insert((view, hash));
            self.leader_buffer.insert(hash, PoolEntry { tx, view });
        }
    }

    pub fn on_dedup_manifest(&mut self, manifest: DedupManifest<T>) {
        let DedupManifest { view, hashes, .. } = manifest;
        self.mark_included(view, hashes);
    }

    /// Returns the pending transactions to send again, within one block and one message,
    /// addressed like `on_submit_transaction`. A transaction is sent again only once every
    /// leader it went to has had its turn without including it.
    pub fn on_view_changed(&mut self, view: ViewNumber) -> Option<TransactionMessage<T>> {
        self.current_view = view;
        while let Some(&(valid_until, hash)) = self.retry_order.first() {
            if valid_until >= view {
                break;
            }
            self.remove_pending(&hash);
        }
        self.expire_pooled(view);

        let batch = self.resend_batch(view);
        if batch.is_empty() {
            return None;
        }
        Some(TransactionMessage {
            view: view + SEND_LEAD,
            transactions: batch,
        })
    }

    fn resend_batch(&mut self, view: ViewNumber) -> Vec<T::Transaction> {
        let first_target = view + SEND_LEAD;
        let max_bytes = self.block_size(first_target);
        let max_encoded = forward_budget(message_limit(max_bytes));
        let mut batch = Vec::new();
        let mut unfit = Vec::new();
        let (mut bytes, mut encoded) = (0u64, 0u64);
        for (_, hash) in &self.retry_order {
            let entry = &self.retry_pending[hash];
            if entry.size > max_bytes || entry.encoded_size > max_encoded {
                unfit.push(*hash);
                continue;
            }
            // A node sees a block's transactions as included only once it has reconstructed
            // the block, about a view after its leader built it. Resending in that view would
            // send again most of what the last leader just included.
            if entry.sent_until + 1 >= view {
                continue;
            }
            if bytes + entry.size > max_bytes || encoded + entry.encoded_size > max_encoded {
                continue;
            }
            bytes += entry.size;
            encoded += entry.encoded_size;
            batch.push(*hash);
        }
        for hash in &unfit {
            warn!(%hash, "pending transaction no longer fits a block, dropping");
            self.remove_pending(hash);
        }
        let sent_until = first_target + (self.config.fanout.get() - 1);
        batch
            .into_iter()
            .map(|hash| {
                let entry = self
                    .retry_pending
                    .get_mut(&hash)
                    .expect("batched hashes come from the retry buffer");
                entry.sent_until = sent_until;
                entry.tx.clone()
            })
            .collect()
    }

    /// Drops pooled transactions sent for views more than `ttl` behind `view`: their senders
    /// have stopped retrying them.
    fn expire_pooled(&mut self, view: ViewNumber) {
        while let Some(&(sent_for, hash)) = self.leader_order.first() {
            if sent_for + self.config.ttl >= view {
                break;
            }
            self.remove_pooled(&hash);
        }
    }

    fn remove_pending(&mut self, hash: &Commitment<T::Transaction>) {
        if let Some(entry) = self.retry_pending.remove(hash) {
            self.retry_order.remove(&(entry.valid_until, *hash));
            self.retry_total_bytes -= entry.size + retry_entry_overhead::<T>();
        }
    }

    /// The block size of the protocol version running at `view`.
    fn block_size(&self, view: ViewNumber) -> u64 {
        let version = self.upgrade_lock.version_infallible(view);
        *self
            .config
            .block_sizes
            .range(..=version)
            .next_back()
            .expect("block sizes start at or below the running version")
            .1
    }

    /// Call for every block this node proposes or reconstructs, so it stops forwarding the
    /// block's transactions and drops copies that reach it later.
    pub fn on_block_reconstructed(
        &mut self,
        view: ViewNumber,
        tx_commitments: Vec<Commitment<T::Transaction>>,
    ) {
        for hash in &tx_commitments {
            self.remove_pending(hash);
        }
        self.mark_included(view, tx_commitments);
    }

    fn mark_included(&mut self, view: ViewNumber, hashes: Vec<Commitment<T::Transaction>>) {
        for hash in &hashes {
            self.remove_pooled(hash);
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
    pub(crate) fn drain(&mut self, view: ViewNumber) -> Vec<T::Transaction> {
        self.take_block(view)
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
