use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::{NonZeroU64, NonZeroUsize},
    panic::resume_unwind,
    sync::Arc,
    time::Duration,
};

use committable::{Commitment, Committable};
use hotshot_types::{
    PeerConfig,
    consensus::PayloadWithMetadata,
    data::{EpochNumber, Leaf2, VidCommitment, ViewNumber},
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
use rayon::prelude::{IntoParallelRefIterator as _, ParallelIterator as _};
use tokio::{
    task::{AbortHandle, JoinSet, spawn_blocking},
    time::{Instant, sleep_until},
};
use tracing::{debug_span, error, warn};
use versions::{NEW_PROTOCOL_VERSION, NO_BUILDER_COMMITMENT_VERSION, Version};

use crate::{
    consensus::ConsensusInput,
    coordinator::metrics::{Measurement, finish_measurement, ignore_measurement},
    helpers::proposal_commitment,
    message::{DedupManifest, Proposal, TransactionMessage},
    network::message_limit,
    state::HeaderRequest,
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
    pub payload_commitment: VidCommitment,
    pub manifest: DedupManifest<T>,
    /// The shares behind `payload_commitment`, until the coordinator takes them for the
    /// disperser.
    pub dispersal: Option<BuiltDispersal<T>>,
}

/// Commitments the leader computes for a block it built.
pub struct BlockCommitments<T: NodeType> {
    pub block_size: u64,
    pub payload_commitment: VidCommitment,
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
                // Named after the step it replaced, so the `block_build` bench compares refs.
                || {
                    debug_span!("vid_commitment").in_scope(|| {
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
        payload_commitment: VidCommitment::V2(dispersal.payload_commitment()?),
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

pub struct BlockBuilderConfig {
    pub max_retry_bytes: u64,
    /// `max_block_size` per protocol version; a missing version inherits the previous one.
    pub block_sizes: BTreeMap<Version, u64>,
    pub ttl: u64,
    /// Views a leader remembers included transactions. Keep it at least `ttl`, or a node still
    /// resending a transaction gets it included again once leaders have forgotten it.
    pub dedup_window_size: u64,
    /// How long a leader whose pool is under `min_pool_fraction` of a block, or empty, waits
    /// for transactions before it builds.
    pub empty_block_delay: Duration,
    /// The share of a block the pool must hold for a leader to build at once. Under it the
    /// leader waits for forwards, until `empty_block_delay` passes or Cert1 of the previous
    /// view forms, and builds with what arrived. 0 waits only on an empty pool.
    pub min_pool_fraction: f64,
    /// Views between a send and the first view it targets. At 2 the next view's leader has
    /// taken its block by the time most of this view's submissions reach it, so a copy sent
    /// to it would mostly wait in its pool for a later turn. 1 sends to it as well, for a
    /// leader that holds its block for the pool to fill (`min_pool_fraction`).
    pub send_lead: u64,
    /// How many upcoming leaders each transaction is sent to. A leader holds up to
    /// `fanout + 1` blocks of transactions for its later views.
    pub fanout: NonZeroU64,
}

impl Default for BlockBuilderConfig {
    fn default() -> Self {
        let ttl = 10;
        Self {
            max_retry_bytes: 4 * 1024 * 1024 * 1024,
            block_sizes: BTreeMap::from([(versions::version(0, 0), 1024 * 1024 * 1024)]),
            ttl,
            dedup_window_size: ttl,
            empty_block_delay: Duration::from_millis(500),
            min_pool_fraction: 0.0,
            send_lead: 2,
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

/// Block requests for one view, parked until the pool has transactions or `deadline` passes.
struct DeferredBuild<T: NodeType> {
    deadline: Instant,
    requests: Vec<BlockAndHeaderRequest<T>>,
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
    /// Requests waiting for the pool to fill, by view.
    deferred: BTreeMap<ViewNumber, DeferredBuild<T>>,
    /// The latest view whose Cert1 formed here.
    cert1_view: Option<ViewNumber>,
    tasks: JoinSet<Result<BlockBuilderOutput<T>, BlockError>>,
    tracer: Option<crate::leader_trace::LeaderTracerHandle>,
    commitments_duration_metric: Option<Arc<dyn Histogram>>,
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
            deferred: BTreeMap::new(),
            cert1_view: None,
            tasks: JoinSet::new(),
            tracer: None,
            commitments_duration_metric: None,
        }
    }

    pub fn with_metrics(mut self, commitments_duration: Option<Arc<dyn Histogram>>) -> Self {
        self.commitments_duration_metric = commitments_duration;
        self
    }

    pub fn set_tracer(&mut self, tracer: Option<crate::leader_trace::LeaderTracerHandle>) {
        self.tracer = tracer;
    }

    /// Builds a block for `request` from the pool, or when the pool holds less than
    /// `min_pool_fraction` of a block, waits for transactions to arrive, up to
    /// `empty_block_delay` or until [`Self::on_certificate1`] ends the wait, and builds from
    /// those. Without the wait an idle network produces empty blocks as fast as consensus can
    /// run them, flooding the coordinator's event queue, and a leader whose forwards are a
    /// moment late proposes a near-empty block instead of them.
    pub fn request_block(&mut self, request: BlockAndHeaderRequest<T>) {
        let view = request.view;
        if let Some(deferred) = self.deferred.get_mut(&view) {
            deferred.requests.push(request);
            return;
        }
        if self.view_transactions.contains_key(&view)
            || self.pool_is_ready(view)
            || self.can_propose(view)
        {
            self.build(request);
            return;
        }
        self.deferred.insert(
            view,
            DeferredBuild {
                deadline: Instant::now() + self.config.empty_block_delay,
                requests: Vec::from([request]),
            },
        );
    }

    /// Whether the pool holds enough for a block of `view` to be built at once.
    fn pool_is_ready(&self, view: ViewNumber) -> bool {
        let min_bytes = (self.block_size(view) as f64 * self.config.min_pool_fraction) as u64;
        self.leader_total_bytes > 0 && self.leader_total_bytes >= min_bytes
    }

    /// Whether the leader of `view` could propose as soon as it has a block, so holding one
    /// for the pool to fill only delays it: Cert1 of the previous view has formed and the
    /// pool is not empty.
    fn can_propose(&self, view: ViewNumber) -> bool {
        self.cert1_view.is_some_and(|cert1| cert1 + 1 >= view) && self.leader_total_bytes > 0
    }

    /// Builds the parked requests of every view whose pool is ready, earliest view first.
    fn build_ready(&mut self) {
        while let Some(view) = self
            .deferred
            .keys()
            .find(|view| self.pool_is_ready(**view))
            .copied()
        {
            self.build_deferred(view);
        }
    }

    /// Builds the parked requests of every view whose wait is over.
    fn build_due(&mut self) {
        let now = Instant::now();
        let due: Vec<ViewNumber> = self
            .deferred
            .iter()
            .filter(|(_, d)| d.deadline <= now)
            .map(|(view, _)| *view)
            .collect();
        for view in due {
            self.build_deferred(view);
        }
    }

    /// Cert1 of `view` formed, so the leader of the view after it can propose as soon as it
    /// has a block: a request parked for the pool to fill builds now with what arrived, and
    /// one that arrives later builds at once. An empty pool keeps waiting, since an empty
    /// block would only pass the turn.
    pub fn on_certificate1(&mut self, view: ViewNumber) {
        self.cert1_view = self.cert1_view.max(Some(view));
        let next = view + 1;
        if self.deferred.contains_key(&next) && self.can_propose(next) {
            self.build_deferred(next);
        }
    }

    fn build_deferred(&mut self, view: ViewNumber) {
        let Some(deferred) = self.deferred.remove(&view) else {
            return;
        };
        for request in deferred.requests {
            self.build(request);
        }
    }

    fn build(&mut self, request: BlockAndHeaderRequest<T>) {
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

        let duration_metric = self.commitments_duration_metric.clone();

        let handle = self.tasks.spawn(async move {
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
                let commitments =
                    block_commitments(&payload, &pooled_hashes, &stake_table, version);
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
        crate::trace_leader_event!(
            self.tracer,
            view,
            crate::leader_trace::LeaderEvent::BlockTaken
        );
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

    /// The next built block. Resolves to `None` once nothing is building or waiting.
    pub async fn next(&mut self) -> Option<Result<BlockBuilderOutput<T>, BlockError>> {
        loop {
            let deadline = self.deferred.values().map(|d| d.deadline).min();
            tokio::select! {
                biased;
                joined = self.tasks.join_next(), if !self.tasks.is_empty() => match joined {
                    Some(Ok(result)) => return Some(result),
                    Some(Err(err)) if err.is_panic() => {
                        error!(%err, "block builder task panicked");
                    },
                    Some(Err(_)) | None => {},
                },
                _ = sleep_until(deadline.unwrap_or_else(Instant::now)), if deadline.is_some() => {
                    self.build_due();
                },
                else => return None,
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
        self.deferred = self.deferred.split_off(&view_number);
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

    /// Returns the message for the upcoming leaders, addressed to the view `send_lead` ahead,
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
            .max(self.block_size(self.current_view + self.config.send_lead));
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
        let first_target = self.current_view + self.config.send_lead;
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
        // A forward carries up to a block of transactions, each hashed in full, and this runs
        // on the coordinator before the leader can build. Hashing them one after another
        // stalls all of consensus for the whole batch.
        let hashes = msg
            .transactions
            .par_iter()
            .map(Committable::commit)
            .collect::<Vec<_>>();
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
        self.build_ready();
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
            view: view + self.config.send_lead,
            transactions: batch,
        })
    }

    fn resend_batch(&mut self, view: ViewNumber) -> Vec<T::Transaction> {
        let first_target = view + self.config.send_lead;
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
        self.take_block(view).0
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
