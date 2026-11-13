use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::{NonZeroU64, NonZeroUsize},
    panic::resume_unwind,
    sync::{Arc, LazyLock},
    time::Duration,
};

use committable::{Commitment, Committable};
use hotshot_types::{
    PeerConfig,
    consensus::PayloadWithMetadata,
    data::{EpochNumber, Leaf2, VidCommitment2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    traits::{
        BlockPayload, EncodeBytes, ValidatedState as _,
        block_contents::{BuilderFee, Transaction},
        metrics::Histogram,
        node_implementation::NodeType,
        signature_key::BuilderSignatureKey,
    },
    utils::BuilderCommitment,
};
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use tokio::{
    task::{AbortHandle, JoinSet, spawn_blocking},
    time::sleep,
};
use tracing::{debug_span, error, warn};
use versions::{NEW_PROTOCOL_VERSION, NO_BUILDER_COMMITMENT_VERSION, Version};

use crate::{
    consensus::ConsensusInput,
    coordinator::metrics::{Measurement, finish_measurement, ignore_measurement},
    helpers::proposal_commitment,
    leader_trace::{LeaderEvent, LeaderTracerHandle},
    message::{DedupManifest, Proposal, TransactionMessage},
    network::message_limit,
    state::HeaderRequest,
    trace_leader_event,
    vid::{BuiltDispersal, VidDisperseError},
};

#[derive(Debug, thiserror::Error)]
pub enum BlockError {
    #[error("payload construction failed: {0}")]
    PayloadConstruction(String),

    #[error("stake table unavailable")]
    StakeTableUnavailable,

    #[error("builder signature failed")]
    BuilderSignature,

    #[error("vid dispersal failed: {0}")]
    Vid(#[from] VidDisperseError),

    #[error("version {0} predates the new protocol")]
    UnsupportedVersion(Version),

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
    pub payload_commitment: VidCommitment2,
    pub manifest: DedupManifest<T>,
    /// The shares behind `payload_commitment`, until the coordinator takes them for the
    /// disperser.
    pub dispersal: Option<BuiltDispersal<T>>,
}

/// Commitments the leader computes for a block it built.
pub struct BlockCommitments<T: NodeType> {
    pub block_size: u64,
    pub payload_commitment: VidCommitment2,
    pub builder_commitment: BuilderCommitment,
    pub hashes: Vec<Commitment<T::Transaction>>,
    pub dispersal: BuiltDispersal<T>,
}

/// The leader's commitment work for a built block, on the proposal's critical path. The
/// `block_build` bench in espresso-types calls this; its spans time each step.
///
/// `pooled_hashes` are the pool's hashes of the transactions the payload was built from, and
/// `stake_table` the nodes of the block's epoch. The payload commitment is folded from the
/// block's dispersal, so the leader encodes the payload once for both. The new protocol runs
/// from `NEW_PROTOCOL_VERSION`, whose VID scheme is always AvidmGf2, so an earlier `version` is
/// refused.
pub fn block_commitments<T: NodeType>(
    payload: &PayloadWithMetadata<T>,
    pooled_hashes: &[Commitment<T::Transaction>],
    stake_table: &[PeerConfig<T>],
    version: Version,
) -> Result<BlockCommitments<T>, BlockError> {
    if version < NEW_PROTOCOL_VERSION {
        return Err(BlockError::UnsupportedVersion(version));
    }
    let _span = debug_span!("block_commitments").entered();
    let (payload_bytes, metadata_bytes) =
        debug_span!("encode").in_scope(|| (payload.payload.encode(), payload.metadata.encode()));
    let block_size = payload_bytes.len() as u64;
    // Independent work, run in parallel rather than paid for in turn on the leader's proposal
    // path.
    let (hashes, (dispersal, builder_commitment)) = rayon::join(
        || transaction_hashes(payload, pooled_hashes),
        || {
            rayon::join(
                || {
                    debug_span!("vid_encode").in_scope(|| {
                        BuiltDispersal::new(stake_table, payload_bytes, &metadata_bytes)
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
    let dispersal = dispersal?;
    Ok(BlockCommitments {
        block_size,
        payload_commitment: dispersal.payload_commitment()?,
        builder_commitment,
        hashes,
        dispersal,
    })
}

/// The payload's transaction hashes, in any order: every consumer uses them as a set.
///
/// Building a payload can drop transactions but never changes, repeats or adds one, so a payload
/// that kept as many transactions as the pool gave it holds exactly the pooled ones, and the
/// pool's hashes save hashing them again. Debug builds check this.
pub(crate) fn transaction_hashes<T: NodeType>(
    payload: &PayloadWithMetadata<T>,
    pooled_hashes: &[Commitment<T::Transaction>],
) -> Vec<Commitment<T::Transaction>> {
    if payload.payload.num_transactions(&payload.metadata) == pooled_hashes.len() {
        debug_assert_eq!(
            pooled_hashes.iter().copied().collect::<HashSet<_>>(),
            payload
                .payload
                .transaction_commitments(&payload.metadata)
                .into_iter()
                .collect::<HashSet<_>>(),
            "a payload that kept every pooled transaction holds exactly those"
        );
        pooled_hashes.to_vec()
    } else {
        debug_span!("transaction_hashes")
            .in_scope(|| payload.payload.transaction_commitments(&payload.metadata))
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

/// Transactions taken from the pool for a view, with the hashes the pool keyed them by.
#[derive(Clone)]
struct PooledBlock<T: NodeType> {
    txs: Arc<Vec<T::Transaction>>,
    hashes: Arc<Vec<Commitment<T::Transaction>>>,
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
    // Transactions of the blocks this node proposed or reconstructed that have not
    // decided yet. They leave `retry_pending` only once their block decides.
    undecided: BTreeMap<(ViewNumber, VidCommitment2), Vec<Commitment<T::Transaction>>>,
    // Blocks that decided within the retry TTL, for a reconstruction that completes
    // after the decide.
    decided: BTreeSet<(ViewNumber, VidCommitment2)>,
    config: BlockBuilderConfig,
    upgrade_lock: UpgradeLock<T>,
    current_view: ViewNumber,
    // Keyed by (view, parent_proposal commitment) so that two requests for
    // the same view but different parents (e.g. one from
    // `handle_proposal_with_vid_share` and one from
    // `handle_timeout_certificate`) don't dedup against each other.
    calculations: BTreeMap<(ViewNumber, Commitment<Leaf2<T>>), AbortHandle>,
    /// The transactions taken for the first block of each view, with their hashes.
    ///
    /// Every later block for the view is built from exactly these. A leader
    /// disperses a block as soon as it is built, and peers keep one share per
    /// leader and view, so a second block for the view is votable only if its
    /// payload is the same. It is the same whenever the new parent does not
    /// change how the payload is built.
    view_transactions: BTreeMap<ViewNumber, PooledBlock<T>>,
    tasks: JoinSet<Result<BlockBuilderOutput<T>, BlockError>>,
    commitments_duration_metric: Option<Arc<dyn Histogram>>,
    /// Records the four block-builder events (request, payload start, commitments
    /// start/end), when set.
    tracer: Option<LeaderTracerHandle>,
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
            undecided: BTreeMap::new(),
            decided: BTreeSet::new(),
            current_view: ViewNumber::genesis(),
            calculations: BTreeMap::new(),
            view_transactions: BTreeMap::new(),
            tasks: JoinSet::new(),
            commitments_duration_metric: None,
            tracer: None,
        }
    }

    /// Registers the leader tracer; builder-style like `with_metrics`.
    pub fn with_tracer(mut self, tracer: Option<LeaderTracerHandle>) -> Self {
        self.tracer = tracer;
        self
    }

    pub fn with_metrics(mut self, commitments_duration: Option<Arc<dyn Histogram>>) -> Self {
        self.commitments_duration_metric = commitments_duration;
        self
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
        let PooledBlock {
            txs,
            hashes: pooled_hashes,
        } = self.transactions_for(view);
        let instance = self.instance.clone();
        let membership = self.membership.clone();

        let empty_block_delay = self.config.empty_block_delay;
        let duration_metric = self.commitments_duration_metric.clone();
        let tracer = self.tracer.clone();

        trace_leader_event!(tracer, view, LeaderEvent::BlockRequestStarted);
        let handle = self.tasks.spawn(async move {
            // Without this an idle network produces empty blocks as fast as consensus can run
            // them, flooding the coordinator's event queue.
            if txs.is_empty() {
                sleep(empty_block_delay).await;
            }
            trace_leader_event!(tracer, view, LeaderEvent::BlockPayloadStart);

            let validated_state =
                T::ValidatedState::from_header(&request.parent_proposal.block_header);
            let (payload, metadata) =
                T::BlockPayload::from_transactions(&txs, &validated_state, &instance)
                    .await
                    .map_err(|e| BlockError::PayloadConstruction(e.to_string()))?;
            let payload: PayloadWithMetadata<T> = PayloadWithMetadata { payload, metadata };

            let stake_table: Vec<_> = membership
                .stake_table_for_epoch(Some(epoch))
                .map_err(|_| BlockError::StakeTableUnavailable)?
                .stake_table()
                .cloned()
                .collect();
            let commitments = spawn_blocking(move || {
                let measurement = duration_metric.map(Measurement::start);
                trace_leader_event!(tracer, view, LeaderEvent::BlockCommitmentsStart);
                let commitments =
                    block_commitments(&payload, &pooled_hashes, &stake_table, version);
                trace_leader_event!(tracer, view, LeaderEvent::BlockCommitmentsEnd);
                match &commitments {
                    Ok(_) => finish_measurement(measurement),
                    Err(_) => ignore_measurement(measurement),
                }
                (payload, commitments)
            });
            let (payload, commitments) = match commitments.await {
                Ok(out) => out,
                Err(e) if e.is_panic() => resume_unwind(e.into_panic()),
                Err(_) => return Err(BlockError::Cancelled),
            };
            let BlockCommitments {
                block_size,
                payload_commitment,
                builder_commitment,
                hashes,
                dispersal,
            } = commitments?;
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
                dispersal: Some(dispersal),
            })
        });
        self.calculations.insert((view, parent_commitment), handle);
    }

    fn transactions_for(&mut self, view: ViewNumber) -> PooledBlock<T> {
        if let Some(pooled) = self.view_transactions.get(&view) {
            return pooled.clone();
        }
        let (txs, hashes) = self.take_block(view);
        let pooled = PooledBlock {
            txs: Arc::new(txs),
            hashes: Arc::new(hashes),
        };
        self.view_transactions.insert(view, pooled.clone());
        pooled
    }

    /// Removes up to one block of pooled transactions, those sent for the earliest views first,
    /// and returns them with their hashes.
    fn take_block(
        &mut self,
        view: ViewNumber,
    ) -> (Vec<T::Transaction>, Vec<Commitment<T::Transaction>>) {
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
        let txs = taken
            .iter()
            .map(|hash| {
                self.remove_pooled(hash)
                    .expect("hashes come from the pool's own order")
            })
            .collect();
        (txs, taken)
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
        let hashes = forwarded_hashes::<T>(&msg.transactions);
        for (hash, tx) in hashes.into_iter().zip(msg.transactions) {
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
        self.expire_block_records(view);

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

    /// Drops the records of blocks more than `ttl` views behind `view`: everything
    /// submitted before such a block was built has expired by now, so its record can no
    /// longer clear anything from `retry_pending`.
    fn expire_block_records(&mut self, view: ViewNumber) {
        let ttl = self.config.ttl;
        self.undecided
            .retain(|(block_view, _), _| *block_view + ttl >= view);
        self.decided
            .retain(|(block_view, _)| *block_view + ttl >= view);
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

    /// Call for every block this node proposes or reconstructs, so it drops copies of the
    /// block's transactions that reach it later. It keeps forwarding its own submissions
    /// until [`Self::on_blocks_decided`] confirms the block: a proposed, or even locked,
    /// block can still be abandoned, and its transactions would otherwise be lost.
    pub fn on_block_reconstructed(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        tx_commitments: Vec<Commitment<T::Transaction>>,
    ) {
        if self.decided.contains(&(view, payload_commitment)) {
            // The decide came first, as it can when this node's shares arrive slowly.
            for hash in &tx_commitments {
                self.remove_pending(hash);
            }
        } else {
            self.undecided
                .insert((view, payload_commitment), tx_commitments.clone());
        }
        self.mark_included(view, tx_commitments);
    }

    /// Call with every decided block. Its transactions leave `retry_pending`. The records
    /// of older blocks stay: a view that decides after a newer one is a gap fill, not an
    /// abandoned branch, and records expire with the retry TTL anyway.
    pub fn on_blocks_decided(
        &mut self,
        blocks: impl IntoIterator<Item = (ViewNumber, VidCommitment2)>,
    ) {
        for key in blocks {
            for hash in self.undecided.remove(&key).unwrap_or_default() {
                self.remove_pending(&hash);
            }
            self.decided.insert(key);
        }
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
        self.take_block(view).0
    }
}

/// The hashes of a forwarded batch, in batch order.
///
/// A forward carries up to a block of transactions, and the coordinator waits for their hashes
/// before it handles anything else, so a batch that is slow to hash serially is hashed across
/// [`FORWARD_HASH_POOL`]. `par_iter` is indexed, so `collect` keeps batch order.
pub(crate) fn forwarded_hashes<T: NodeType>(
    transactions: &[T::Transaction],
) -> Vec<Commitment<T::Transaction>> {
    let serial = || transactions.iter().map(Committable::commit).collect();
    let bytes: u64 = transactions
        .iter()
        .map(Transaction::minimum_block_size)
        .sum();
    if transactions.len() < MIN_PARALLEL_TRANSACTIONS && bytes < MIN_PARALLEL_BYTES {
        return serial();
    }
    match &*FORWARD_HASH_POOL {
        Some(pool) => pool.install(|| transactions.par_iter().map(Committable::commit).collect()),
        None => serial(),
    }
}

/// Batch size from which [`forwarded_hashes`] hashes in parallel.
///
/// Set by what hashing costs the coordinator, not by when parallel hashing breaks even: a
/// batch of this size hashes serially in about 0.4 ms, and below that the handoff to the pool
/// is not worth it.
pub(crate) const MIN_PARALLEL_BYTES: u64 = 256 * 1024;

/// Transaction count from which [`forwarded_hashes`] hashes in parallel whatever the batch's
/// size: each hash has a fixed cost, and this many one-byte transactions also take about
/// 0.4 ms.
pub(crate) const MIN_PARALLEL_TRANSACTIONS: usize = 1024;

/// Threads hashing forwarded batches: a quarter of the cores, between 2 and 4.
fn forward_hash_threads() -> usize {
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    (cores / 4).clamp(2, 4)
}

/// The pool forwarded batches are hashed on, apart from rayon's global one.
///
/// Any peer can forward a batch and the sender is not checked. On the global pool its hashing
/// would take every core from the leader's VID encode, and the coordinator would wait behind an
/// encode already running there, since a busy pool takes outside jobs last. This pool bounds
/// both. `None` if its threads cannot be spawned; batches are then hashed serially.
static FORWARD_HASH_POOL: LazyLock<Option<rayon::ThreadPool>> = LazyLock::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(forward_hash_threads())
        .thread_name(|i| format!("forward-hash-{i}"))
        .build()
        .inspect_err(|err| warn!(%err, "no pool to hash forwarded transactions on"))
        .ok()
});

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
