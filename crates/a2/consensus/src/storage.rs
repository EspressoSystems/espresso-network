//! The single writer of the consensus store.
//!
//! Writes are queued synchronously into a batch. While one batch is being applied on a blocking
//! thread, the next one fills, so all writes made during one commit share the next commit and
//! its fsync. Each write names the [`StorageOutput`] that reports it durable, which consensus
//! waits on before sending the messages that depend on it.

use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    mem,
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use app_tables::AppWrite;
use committable::Commitment;
use consensus_api::payloads::{Acked, Acks, Offer, PayloadQueue};
use consensus_store::{Batch, Store};
use hotshot_types::{
    data::{EpochNumber, Leaf2, VidCommitment, VidCommitment2, VidDisperseShare2, ViewNumber},
    event::HotShotAction,
    message::Proposal as SignedProposal,
    simple_certificate::{LightClientStateUpdateCertificateV2, UpgradeCertificate},
    simple_vote::LockView,
    traits::{
        BlockPayload, EncodeBytes,
        metrics::{Gauge, Histogram, Metrics},
        node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    vid::avidm_gf2::AvidmGf2Common,
    vote::HasViewNumber,
};
use parking_lot::Mutex;
use tokio::{
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
    time::{Instant, sleep_until},
};
use tracing::{error, info, warn};

use crate::{
    coordinator::metrics::Measurement,
    discovery::SignedServiceRecord,
    helpers::proposal_commitment,
    message::{Certificate1, Certificate2, Proposal},
    state::StateEntry,
    store::{self, Anchor, ConsensusStore, ForeignWrite, Restored, StateChange},
};

/// The delay before the first retry of a failed batch. It doubles with each further failure, up
/// to [`MAX_RETRY_DELAY`].
const MIN_RETRY_DELAY: Duration = Duration::from_millis(300);

const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

/// How many bytes the decide log may take up, with the payloads of its entries, before this
/// node stops sending vote2, unless configured otherwise.
pub const DEFAULT_DECIDE_LOG_LIMIT: u64 = 10 << 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionKind {
    Vote,
    Propose,
    /// A timeout vote that carries the signer's lock. Recorded before it is
    /// sent, so that a restarted node does not vote2 at a view it timed out.
    Timeout,
}

impl From<ActionKind> for HotShotAction {
    fn from(kind: ActionKind) -> Self {
        match kind {
            ActionKind::Vote => HotShotAction::Vote,
            ActionKind::Propose => HotShotAction::Propose,
            ActionKind::Timeout => HotShotAction::TimeoutVote,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageOutput<T: NodeType> {
    Proposal(ViewNumber, Commitment<Leaf2<T>>),
    Action(ViewNumber, ActionKind),
    /// The locked QC at the given lock position has been durably persisted.
    HighQc(LockView),
    /// The payload of the block with this commitment, proposed in this view, is on disk.
    Payload(ViewNumber, VidCommitment2),

    /// The decide log has reached its limit, or has room again after reaching it.
    DecideLogFull(bool),
}

impl<T: NodeType> StorageOutput<T> {
    pub fn view_number(&self) -> ViewNumber {
        match self {
            Self::Proposal(view, _) | Self::Action(view, _) | Self::Payload(view, _) => *view,
            Self::HighQc(lock) => lock.view,
            Self::DecideLogFull(_) => ViewNumber::genesis(),
        }
    }
}

/// What the application keeps of the latest decided state, in its own tables of the store, and
/// what of each decided leaf's state the decide log carries.
///
/// Its writes join the batch that appends the decide to the decide log, so the tables never
/// fall behind or run ahead of the anchor. It runs where that batch is applied, off the
/// coordinator, so that its cost delays the store, not consensus.
pub trait DecidedState<T: NodeType>: Send + Sync {
    /// The writes that bring the application's tables up to date with the decided `leaf`, and
    /// the parts of its state for the decide log.
    ///
    /// Called for each leaf of a decide, oldest first. `state` is what this node holds for
    /// `leaf`: a state with its delta if this node validated `leaf`, else one without, or none.
    fn decided(&mut self, leaf: &Leaf2<T>, state: Option<&StateEntry<T>>) -> DecidedWrites;
}

/// What [`DecidedState::decided`] makes of a decided leaf.
#[derive(Debug, Default)]
pub struct DecidedWrites {
    /// The writes to the application's tables.
    pub writes: Vec<AppWrite>,

    /// The parts of the leaf's state for its decide log entry.
    pub state: Option<StateChange>,
}

/// Storage cannot go on.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The store failed in a way retrying cannot fix: it is poisoned or corrupt.
    #[error("consensus store failed: {0}")]
    Store(#[from] consensus_store::Error),

    /// A write could not be encoded. Consensus would wait for it forever.
    #[error("failed to encode a storage write: {0:#}")]
    Encoding(anyhow::Error),

    /// The blocking task applying a batch was cancelled, as at runtime shutdown.
    #[error("storage task cancelled")]
    Cancelled,
}

/// Time from a write's request until it is durable, including the wait for the batch it joined.
/// These latencies gate consensus progress: proposal release waits on `append_proposal` and
/// `record_action(Propose)`, and phase-2 votes wait on `append_da`,
/// `record_action(Vote)` and `append_high_qc2`.
///
/// [`Measurement`] records on drop, so a write lost to shutdown still records its elapsed time.
pub struct StorageMetrics {
    append_da: Arc<dyn Histogram>,
    append_high_qc: Arc<dyn Histogram>,
    append_state_cert: Arc<dyn Histogram>,
    append_proposal: Arc<dyn Histogram>,
    record_action: Arc<dyn Histogram>,
    /// Duration of one batch commit.
    apply: Arc<dyn Histogram>,

    /// The bytes the decide log takes up, and how many it may.
    decide_log_bytes: Arc<dyn Gauge>,
    decide_log_limit: Arc<dyn Gauge>,
}

impl StorageMetrics {
    pub fn new(m: &dyn Metrics) -> Self {
        let histogram = |name: &str| -> Arc<dyn Histogram> {
            m.create_histogram(format!("storage_{name}"), Some("s".into()))
                .into()
        };
        Self {
            append_da: histogram("append_da"),
            append_high_qc: histogram("append_high_qc"),
            append_state_cert: histogram("append_state_cert"),
            append_proposal: histogram("append_proposal"),
            record_action: histogram("record_action"),
            apply: histogram("apply"),
            decide_log_bytes: m
                .create_gauge("storage_decide_log_bytes".into(), Some("bytes".into()))
                .into(),
            decide_log_limit: m
                .create_gauge("storage_decide_log_limit".into(), Some("bytes".into()))
                .into(),
        }
    }
}

pub struct Storage<T: NodeType, S> {
    store: ConsensusStore<T, S>,

    /// Signature stored with proposals. It signs no data; restarted nodes serve the proposals to
    /// peers in the signed form messages use.
    signature: <T::SignatureKey as SignatureKey>::PureAssembledSignatureType,

    /// Writes not yet handed to the store.
    pending: Pending<T>,

    /// The batch being applied, if any.
    applying: Option<JoinHandle<Applied<T>>>,

    /// The writer's batch that failed, which is applied again at `retry_at`. Until then only
    /// foreign writes are applied, each batch of them on its own, so that an acknowledgement,
    /// which frees space, does not wait for a batch that fails for lack of it.
    failed: Option<Pending<T>>,
    retry_at: Instant,

    /// The delay before the next retry. Reset when a batch of the writer succeeds.
    retry_delay: Duration,

    /// A write that could not be encoded, which [`Self::next`] reports.
    fatal: Option<StorageError>,

    /// Outputs of applied batches, not yet returned by [`Self::next`].
    durable: std::collections::VecDeque<StorageOutput<T>>,

    /// The size of the decide log, and the size at which it is full.
    decide_log_bytes: watch::Receiver<u64>,
    decide_log_limit: u64,

    /// Whether the last [`StorageOutput::DecideLogFull`] reported the log full.
    decide_log_full: bool,

    /// State certificates written since this writer started and not yet decided, by the view of
    /// their epoch root. A decide finds them here even before their write is durable.
    state_certs: BTreeMap<ViewNumber, LightClientStateUpdateCertificateV2<T>>,

    decided_state: Option<Arc<Mutex<Box<dyn DecidedState<T>>>>>,

    /// Writes from outside the writer, such as acknowledgements of the decide log, which join
    /// the next batch.
    foreign: mpsc::UnboundedReceiver<ForeignWrite>,

    /// Where payloads go instead of the store, if anywhere: the deployment's storage instances,
    /// whose acknowledgements report them durable.
    payload_sink: Option<PayloadSink>,

    /// Payloads written since this writer started, until garbage collection.
    written_payloads: BTreeSet<(ViewNumber, VidCommitment2)>,

    /// Blocks decided without their payload stored, with their height, until garbage
    /// collection. A payload stored for one of them in that time joins its decide log entry.
    awaited_payloads: BTreeMap<(ViewNumber, VidCommitment2), u64>,

    /// The view garbage was last collected before; no payload is awaited for earlier views.
    collected_before: ViewNumber,

    /// The latest of each value the store may only raise, as written or queued.
    marks: Marks,

    metrics: Option<StorageMetrics>,
}

/// The latest of each value the store may only raise: the lock, the boundary certificate, the
/// acted and restart views, and the height of the anchor. The writer compares against them
/// before it queues a write, so a value is never lowered.
#[derive(Clone, Copy, Debug, Default)]
struct Marks {
    locked: Option<LockView>,
    boundary: Option<LockView>,
    acted: Option<ViewNumber>,
    restart: Option<ViewNumber>,
    anchor: Option<u64>,
}

impl Marks {
    /// The values a restarted node finds in its store.
    fn of<T: NodeType>(restored: &Restored<T>) -> Self {
        Self {
            locked: restored.locked_qc.as_ref().map(LockView::of),
            boundary: restored.boundary_qc.as_ref().map(LockView::of),
            acted: restored.acted_view,
            restart: restored.restart_view,
            anchor: restored.anchor.as_ref().map(|anchor| anchor.leaf.height()),
        }
    }
}

/// A batch and what to report once it is durable.
struct Pending<T: NodeType> {
    batch: Batch,
    outputs: Vec<StorageOutput<T>>,
    timers: Vec<Measurement>,

    /// The writes from outside the writer, applied after `batch` in the same commit, and where
    /// to report them. They are kept apart so that a failing one can be dropped without
    /// holding up the writer's own.
    foreign_batch: Batch,
    foreign: Vec<oneshot::Sender<anyhow::Result<()>>>,

    /// Decides whose decide log entries and application writes are added to `batch` where it
    /// is applied, in this order, after the rest of it.
    decided: Vec<Decide<T>>,

    /// Whether the batch adds to the decide log, which its readers then hear of, and whether
    /// it or the foreign writes change the decide log's size, which is then measured.
    adds_to_decide_log: bool,
    resizes_decide_log: bool,
    foreign_resizes_decide_log: bool,
}

impl<T: NodeType> Default for Pending<T> {
    fn default() -> Self {
        Self {
            batch: Batch::new(),
            outputs: Vec::new(),
            timers: Vec::new(),
            foreign_batch: Batch::new(),
            foreign: Vec::new(),
            decided: Vec::new(),
            adds_to_decide_log: false,
            resizes_decide_log: false,
            foreign_resizes_decide_log: false,
        }
    }
}

impl<T: NodeType> Pending<T> {
    /// Whether there is nothing to write and nothing to report. Outputs can wait without a
    /// write: those of writes that would not raise a stored value.
    fn is_empty(&self) -> bool {
        self.writes_nothing() && self.outputs.is_empty() && self.foreign.is_empty()
    }

    fn writes_nothing(&self) -> bool {
        self.batch.is_empty() && self.foreign_batch.is_empty() && self.decided.is_empty()
    }

    /// The writer's batch with the foreign writes after it.
    fn combined(&self) -> Batch {
        let mut batch = self.batch.clone();
        batch.append(self.foreign_batch.clone());
        batch
    }

    /// The foreign writes alone, as a batch of their own. The writer's stay.
    fn split_foreign(&mut self) -> Self {
        Self {
            batch: mem::take(&mut self.foreign_batch),
            foreign: mem::take(&mut self.foreign),
            resizes_decide_log: mem::take(&mut self.foreign_resizes_decide_log),
            ..Self::default()
        }
    }

    /// Reports the foreign writes failed with `error` and drops them. A foreign write may be
    /// what fails, so it must not hold up the writes consensus waits on.
    fn drop_foreign(&mut self, error: &str) {
        self.report(Some(error));
        self.foreign_batch = Batch::new();
        self.foreign_resizes_decide_log = false;
    }

    /// Reports the foreign writes as durable, or as failed with `error`.
    fn report(&mut self, error: Option<&str>) {
        for done in self.foreign.drain(..) {
            let _ = done.send(match error {
                None => Ok(()),
                Some(error) => Err(anyhow::anyhow!("{error}")),
            });
        }
    }

    /// Adds the decide log entries of `decided`, and the application's writes for them, to the
    /// batch, so that a retry applies them as they are.
    fn add_decided(
        &mut self,
        decided_state: Option<&Mutex<Box<dyn DecidedState<T>>>>,
    ) -> anyhow::Result<()> {
        for decide in mem::take(&mut self.decided) {
            let mut decided: Vec<DecidedWrites> = Vec::new();
            if let Some(decided_state) = decided_state {
                let mut decided_state = decided_state.lock();
                decided.resize_with(decide.leaves.len(), DecidedWrites::default);
                for (i, leaf) in decide.leaves.iter().enumerate().rev() {
                    decided[i] =
                        decided_state.decided(leaf, decide.states.get(i).and_then(Option::as_ref));
                }
            }
            let states: Vec<_> = decided.iter_mut().map(|d| d.state.take()).collect();
            let view = decide.cert1.view_number();
            store::put_decided(
                &mut self.batch,
                &decide.leaves,
                &decide.cert1,
                decide.cert2.as_ref(),
                &decide.vid_common,
                &decide.state_certs,
                &states,
            )
            .with_context(|| format!("leaves decided in view {view}"))?;
            for d in decided.iter().rev() {
                store::put_app(&mut self.batch, &d.writes);
            }
        }
        Ok(())
    }
}

struct Applied<T: NodeType> {
    pending: Pending<T>,
    result: Result<(), consensus_store::Error>,

    /// Why the batch was not applied: a decide log entry could not be encoded.
    encoding: Option<anyhow::Error>,
}

/// A decide as [`Storage::append_decided`] takes it, for its decide log entries.
struct Decide<T: NodeType> {
    leaves: Vec<Leaf2<T>>,
    cert1: Certificate1<T>,
    cert2: Option<Certificate2<T>>,
    vid_common: Vec<Option<AvidmGf2Common>>,
    state_certs: Vec<Option<LightClientStateUpdateCertificateV2<T>>>,
    states: Vec<Option<StateEntry<T>>>,
}

impl<T: NodeType, S: Store> Storage<T, S> {
    /// The writer of `store`, which holds `restored`, as [`ConsensusStore::load`] reads it. The
    /// writer never lowers the values that may only rise, starting from those in `restored`.
    pub fn new(
        store: ConsensusStore<T, S>,
        private_key: &<T::SignatureKey as SignatureKey>::PrivateKey,
        restored: &Restored<T>,
    ) -> Self {
        let marks = Marks::of(restored);
        let signature =
            T::SignatureKey::sign(private_key, &[]).expect("signing empty data cannot fail");
        let decide_log_bytes = store.decide_log_bytes();
        let (writer, foreign) = mpsc::unbounded_channel();
        store.attach_writer(writer);
        Self {
            store,
            signature,
            pending: Pending::default(),
            applying: None,
            failed: None,
            retry_at: Instant::now(),
            retry_delay: MIN_RETRY_DELAY,
            fatal: None,
            durable: Default::default(),
            decide_log_bytes,
            decide_log_limit: DEFAULT_DECIDE_LOG_LIMIT,
            decide_log_full: false,
            state_certs: BTreeMap::new(),
            decided_state: None,
            foreign,
            payload_sink: None,
            written_payloads: BTreeSet::new(),
            awaited_payloads: BTreeMap::new(),
            collected_before: ViewNumber::genesis(),
            marks,
            metrics: None,
        }
    }

    pub fn with_metrics(mut self, m: &dyn Metrics) -> Self {
        if m.is_recording() {
            let metrics = StorageMetrics::new(m);
            metrics
                .decide_log_limit
                .set(usize::try_from(self.decide_log_limit).unwrap_or(usize::MAX));
            self.metrics = Some(metrics);
        }
        self
    }

    /// Sets how many bytes the decide log may take up, with the payloads of its entries.
    ///
    /// Once it is full, [`Self::next`] reports [`StorageOutput::DecideLogFull`], and consensus
    /// holds its vote2 until the store acknowledges entries.
    pub fn with_decide_log_limit(mut self, limit: u64) -> Self {
        self.decide_log_limit = limit;
        if let Some(m) = &self.metrics {
            m.decide_log_limit
                .set(usize::try_from(limit).unwrap_or(usize::MAX));
        }
        self
    }

    /// Keeps the application's decided state with every decide.
    pub fn with_decided_state(mut self, decided_state: Box<dyn DecidedState<T>>) -> Self {
        self.decided_state = Some(Arc::new(Mutex::new(decided_state)));
        self
    }

    /// Hands payloads to the storage instances that take them from `queue`, instead of storing
    /// them: a payload is durable once a storage instance acknowledges it on `acks`. Consensus
    /// then keeps payloads only in memory, and the decide log carries none.
    pub fn with_payload_sink(mut self, queue: PayloadQueue, acks: Acks) -> Self {
        self.payload_sink = Some(PayloadSink {
            queue,
            acks,
            offered: BTreeMap::new(),
        });
        self
    }

    /// Persists the payload of a block. Vote2 on the block waits for it.
    pub fn append_da(
        &mut self,
        view: ViewNumber,
        _epoch: EpochNumber,
        block_payload: T::BlockPayload,
        metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
        vid_commit: VidCommitment,
    ) {
        let VidCommitment::V2(commitment) = vid_commit else {
            warn!(%view, "payload with a non-V2 VID commitment; not stored");
            return;
        };
        let encoded = block_payload.encode();
        if let Some(sink) = &mut self.payload_sink {
            let name = vid_commit.to_string();
            sink.offered
                .insert((*view, name.clone()), (view, commitment));
            sink.queue.offer(Offer {
                view: *view,
                commitment: name,
                ns_table: metadata.encode().to_vec().into(),
                payload: encoded.to_vec().into(),
            });
            return;
        }
        self.written_payloads.insert((view, commitment));
        let mut written = store::put_payload(&mut self.pending.batch, view, &commitment, encoded);
        if let Some(height) = self.awaited_payloads.remove(&(view, commitment)) {
            // After the payload's own write, so that the copy finds it.
            written = written.and_then(|()| {
                store::put_decided_payload(&mut self.pending.batch, height, view, &commitment)
            });
            self.pending.adds_to_decide_log = true;
            self.pending.resizes_decide_log = true;
        }
        self.queued(written, StorageOutput::Payload(view, commitment), |m| {
            &m.append_da
        });
    }

    /// Persists the own certificate of the last block of an epoch. Nothing waits on it.
    pub fn append_boundary_qc2(&mut self, qc: Certificate1<T>) {
        let lock = LockView::of(&qc);
        if self.marks.boundary >= Some(lock) {
            return;
        }
        match store::put_boundary_qc(&mut self.pending.batch, &qc) {
            Ok(()) => self.marks.boundary = Some(lock),
            Err(err) => self.fail(err.context(format!("boundary qc of view {}", qc.view_number()))),
        }
    }

    /// Persists the locked QC; once durable, emits [`StorageOutput::HighQc`], which gates the
    /// matching phase-2 vote.
    pub fn append_high_qc2(&mut self, high_qc: Certificate1<T>) {
        let lock = LockView::of(&high_qc);
        // An earlier lock leaves the stored one, and is durable as soon as that is.
        let written = match self.marks.locked < Some(lock) {
            true => store::put_locked_qc(&mut self.pending.batch, &high_qc)
                .inspect(|()| self.marks.locked = Some(lock)),
            false => Ok(()),
        };
        self.queued(written, StorageOutput::HighQc(lock), |m| &m.append_high_qc);
    }

    /// Persists the state certificate of the epoch root proposed in `view`. It counts as
    /// decided once [`Self::decided`] passes `view`.
    pub fn append_state_cert(
        &mut self,
        view: ViewNumber,
        state_cert: LightClientStateUpdateCertificateV2<T>,
    ) {
        let timer = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.append_state_cert.clone()));
        match store::put_pending_state_cert(&mut self.pending.batch, view, &state_cert) {
            Ok(()) => {
                self.pending.timers.extend(timer);
                self.state_certs.insert(view, state_cert);
            },
            Err(err) => self.fail(err.context(format!("state certificate of view {view}"))),
        }
    }

    /// Persists the newest record of a validator's API service. Nothing waits on it.
    pub fn append_service_record(&mut self, record: &SignedServiceRecord<T>) {
        if let Err(err) = store::put_service_record(&mut self.pending.batch, record) {
            self.fail(err.context(format!("service record of {}", record.signer)));
        }
    }

    /// Deletes the records of the epochs before `epoch`.
    pub fn drop_service_records(&mut self, epoch: EpochNumber) {
        store::drop_service_records(&mut self.pending.batch, epoch);
    }

    pub fn update_decided_upgrade_certificate(&mut self, cert: UpgradeCertificate<T>) {
        if let Err(err) = store::put_upgrade_cert(&mut self.pending.batch, &cert) {
            self.fail(err.context("upgrade certificate"));
        }
    }

    /// `proposal` must already be validated: `state_cert`'s presence is not re-checked here,
    /// only gated at write time by `state_cert_matches_parent` (received proposals) and
    /// `maybe_propose` (self-proposed ones). A restart reads the stored proposals back without
    /// re-checking, so that write-time gate is the only thing enforcing this.
    pub fn append_proposal(&mut self, proposal: Proposal<T>) {
        let view = proposal.view_number;
        let commitment = proposal_commitment(&proposal);
        let signed = SignedProposal {
            data: proposal,
            signature: self.signature.clone(),
            _pd: PhantomData,
        };
        let written = store::put_proposal(&mut self.pending.batch, &signed);
        self.queued(written, StorageOutput::Proposal(view, commitment), |m| {
            &m.append_proposal
        });
    }

    /// Records that this node acted in `view`: proposals, votes and timeout votes raise the
    /// acted view; votes also raise the restart view past `view`.
    pub fn record_action(&mut self, view: ViewNumber, kind: ActionKind) {
        if self.marks.acted < Some(view) {
            store::put_acted_view(&mut self.pending.batch, view);
            self.marks.acted = Some(view);
        }
        if kind == ActionKind::Vote && self.marks.restart < Some(view + 1) {
            store::put_restart_view(&mut self.pending.batch, view + 1);
            self.marks.restart = Some(view + 1);
        }
        self.queued(Ok(()), StorageOutput::Action(view, kind), |m| {
            &m.record_action
        });
    }

    /// Appends a decide to the decide log and moves the anchor forward to its newest leaf:
    /// `leaves` newest first, with
    /// the certificates that decided the newest, and this node's VID share and state of each.
    pub fn append_decided(
        &mut self,
        leaves: &[Leaf2<T>],
        cert1: &Certificate1<T>,
        cert2: Option<&Certificate2<T>>,
        vid_shares: &[Option<SignedProposal<T, VidDisperseShare2<T>>>],
        states: &[Option<StateEntry<T>>],
    ) {
        let vid_common: Vec<_> = vid_shares
            .iter()
            .map(|share| share.as_ref().map(|s| s.data.common.clone()))
            .collect();
        let state_certs: Vec<_> = leaves
            .iter()
            .map(|leaf| self.state_cert(leaf.view_number()))
            .collect();
        // The copies go ahead of the collection of garbage, which deletes what they share; the
        // entries, which carry what the application makes of each leaf's state, are built where
        // the batch is applied.
        if let Err(err) = store::copy_decided_payloads(&mut self.pending.batch, leaves) {
            let view = cert1.view_number();
            self.fail(err.context(format!("leaves decided in view {view}")));
            return;
        }
        self.pending.decided.push(Decide {
            leaves: leaves.to_vec(),
            cert1: cert1.clone(),
            cert2: cert2.cloned(),
            vid_common,
            state_certs,
            states: states.to_vec(),
        });
        self.pending.adds_to_decide_log = true;
        self.pending.resizes_decide_log = true;
        self.await_payloads(leaves);
        self.advance_anchor(leaves, cert1, cert2);
    }

    /// Deletes the stored proposals and payloads of views before `view`.
    pub fn gc(&mut self, view: ViewNumber) {
        store::collect_garbage(&mut self.pending.batch, view);
        self.collected_before = self.collected_before.max(view);
        self.written_payloads.retain(|(v, _)| *v >= view);
        self.awaited_payloads.retain(|(v, _), _| *v >= view);
        if let Some(sink) = &mut self.payload_sink {
            sink.offered.retain(|_, (v, _)| *v >= view);
        }
    }

    /// Marks the state certificates of views up to `view` as decided.
    pub fn decided(&mut self, view: ViewNumber) {
        if let Err(err) =
            self.store
                .finalize_state_certs(&mut self.pending.batch, view, &self.state_certs)
        {
            warn!(%err, %view, "failed to finalize state certificates");
        }
        self.state_certs = self.state_certs.split_off(&(view + 1));
    }

    /// Moves the anchor to the newest of `leaves`, unless a decide that fills a gap leaves it
    /// behind the anchor.
    fn advance_anchor(
        &mut self,
        leaves: &[Leaf2<T>],
        cert1: &Certificate1<T>,
        cert2: Option<&Certificate2<T>>,
    ) {
        let Some(newest) = leaves.first() else {
            return;
        };
        if self.marks.anchor >= Some(newest.height()) {
            return;
        }
        let mut leaf = newest.clone();
        leaf.unfill_block_payload();
        let anchor = Anchor {
            leaf,
            cert1: cert1.clone(),
            cert2: cert2.cloned(),
        };
        match store::put_anchor(&mut self.pending.batch, &anchor) {
            Ok(()) => self.marks.anchor = Some(newest.height()),
            Err(err) => self.fail(err.context(format!("anchor at height {}", newest.height()))),
        }
    }

    /// Remembers the decided `leaves` whose payloads are not stored, so that a payload stored
    /// before garbage collection joins the decide log.
    fn await_payloads(&mut self, leaves: &[Leaf2<T>]) {
        // Payloads that go to the storage instances never join the decide log.
        if self.payload_sink.is_some() {
            return;
        }
        for leaf in leaves {
            let VidCommitment::V2(commitment) = leaf.payload_commitment() else {
                continue;
            };
            let key = (leaf.view_number(), commitment);
            if key.0 < self.collected_before || self.written_payloads.contains(&key) {
                continue;
            }
            match self.store.has_payload(key.0, &commitment) {
                Ok(true) => {},
                Ok(false) => {
                    self.awaited_payloads.insert(key, leaf.height());
                },
                Err(err) => warn!(%err, view = %key.0, "failed to look up a stored payload"),
            }
        }
    }

    /// The undecided state certificate of the epoch root proposed in `view`, if any.
    fn state_cert(&self, view: ViewNumber) -> Option<LightClientStateUpdateCertificateV2<T>> {
        if let Some(cert) = self.state_certs.get(&view) {
            return Some(cert.clone());
        }
        self.store
            .pending_state_cert(view)
            .inspect_err(|err| warn!(%err, %view, "failed to read a state certificate"))
            .ok()
            .flatten()
    }

    /// The next write that has become durable, or a change of whether the decide log is full.
    ///
    /// Starts applying the queued writes whenever no batch is being applied. Cancel-safe: a
    /// batch in progress stays in progress.
    pub async fn next(&mut self) -> Result<StorageOutput<T>, StorageError> {
        loop {
            if let Some(err) = self.fatal.take() {
                return Err(err);
            }
            if let Some(output) = self.durable.pop_front() {
                return Ok(output);
            }
            if let Some(full) = self.decide_log_full_changed() {
                return Ok(StorageOutput::DecideLogFull(full));
            }
            // Writes from outside that arrived while a batch was applied join the next one.
            while let Ok(write) = self.foreign.try_recv() {
                self.take_foreign(write);
            }
            if let Some(acked) = self
                .payload_sink
                .as_mut()
                .and_then(|s| s.acks.try_recv().ok())
            {
                self.payload_acked(acked);
                continue;
            }
            self.start();
            if !self.durable.is_empty() {
                continue;
            }
            // A clone, since `finish` borrows `self`. It has seen what the original has.
            let mut decide_log_bytes = self.decide_log_bytes.clone();
            // This future is dropped whenever the caller's other branches win, so every await
            // below only borrows fields, and nothing is moved out of the writer meanwhile.
            // Acknowledgements that arrive while a batch is applied are taken at the top of the
            // next round.
            if self.applying.is_some() {
                tokio::select! {
                    applied = self.finish() => applied?,
                    _ = decide_log_bytes.changed() => {},
                }
            } else {
                let retry_at = self.failed.as_ref().map(|_| self.retry_at);
                let acks = self.payload_sink.as_mut().map(|sink| &mut sink.acks);
                let acked = async {
                    match acks {
                        Some(acks) => acks.recv().await,
                        None => std::future::pending().await,
                    }
                };
                tokio::select! {
                    // The sender lives as long as the store, which this writer holds.
                    _ = decide_log_bytes.changed() => {},
                    Some(write) = self.foreign.recv() => self.take_foreign(write),
                    Some(acked) = acked => self.payload_acked(acked),
                    _ = sleep_until(retry_at.unwrap_or_else(Instant::now)), if retry_at.is_some() => {},
                }
            }
        }
    }

    /// Reports the payload a storage instance acknowledged durable, if it was offered and not yet
    /// collected.
    fn payload_acked(&mut self, acked: Acked) {
        let output = acked_output(&mut self.payload_sink, acked);
        self.durable.extend(output);
    }

    /// Adds a write from outside the writer to the next batch, so that it shares its commit.
    fn take_foreign(&mut self, write: ForeignWrite) {
        self.pending.foreign_batch.append(write.batch);
        self.pending.foreign_resizes_decide_log |= write.resizes_decide_log;
        self.pending.foreign.push(write.done);
    }

    /// Applies every queued write, for shutdown.
    pub async fn flush(mut self) {
        info!("flushing storage during shutdown");
        while let Ok(write) = self.foreign.try_recv() {
            self.take_foreign(write);
        }
        loop {
            self.start();
            if self.applying.is_some() {
                if let Err(err) = self.finish().await {
                    error!(%err, "storage failed during shutdown");
                    return;
                }
            } else if self.failed.is_some() {
                sleep_until(self.retry_at).await;
            } else if self.pending.is_empty() {
                break;
            }
        }
        info!("storage flush complete");
    }

    /// Starts applying a batch unless one is being applied: while the writer's batch is failing,
    /// the foreign writes on their own, and the failed batch once its retry is due; otherwise
    /// the queued writes.
    fn start(&mut self) {
        if self.applying.is_some() {
            return;
        }
        if self.failed.is_some() {
            if !self.pending.foreign.is_empty() {
                let foreign = self.pending.split_foreign();
                self.applying = Some(self.spawn(foreign));
            } else if Instant::now() >= self.retry_at {
                let failed = self.failed.take().expect("checked above");
                self.applying = Some(self.spawn(failed));
            }
            return;
        }
        if self.pending.is_empty() {
            return;
        }
        let mut pending = mem::take(&mut self.pending);
        if pending.writes_nothing() {
            // Nothing to write, and everything queued before is durable.
            pending.report(None);
            self.durable.extend(pending.outputs);
            return;
        }
        self.applying = Some(self.spawn(pending));
    }

    /// Waits for the batch being applied and moves its outputs to `durable`. A failed batch of
    /// the writer is kept for a retry, with a delay that doubles with each failure.
    ///
    /// Cancel-safe: a batch in progress stays in progress.
    async fn finish(&mut self) -> Result<(), StorageError> {
        let applying = self.applying.as_mut().expect("a batch is being applied");
        let applied = match applying.await {
            Ok(applied) => applied,
            Err(err) if err.is_panic() => std::panic::resume_unwind(err.into_panic()),
            Err(_) => {
                self.applying = None;
                return Err(StorageError::Cancelled);
            },
        };
        self.applying = None;
        let Applied {
            mut pending,
            result,
            encoding,
        } = applied;
        if let Some(err) = encoding {
            self.fail(err);
            return Ok(());
        }
        // While the writer's batch is failing, only foreign writes are applied.
        let writers = self.failed.is_none();
        match result {
            Ok(()) => {
                pending.report(None);
                if pending.adds_to_decide_log {
                    self.store.announce();
                }
                self.durable.extend(pending.outputs);
                if writers {
                    self.retry_delay = MIN_RETRY_DELAY;
                }
            },
            Err(
                err
                @ (consensus_store::Error::Poisoned(_) | consensus_store::Error::Corrupt { .. }),
            ) => {
                pending.report(Some(&err.to_string()));
                return Err(err.into());
            },
            Err(err) => {
                pending.drop_foreign(&err.to_string());
                if writers {
                    warn!(%err, delay = ?self.retry_delay, "failed to apply storage batch, retrying");
                    self.retry_at = Instant::now() + self.retry_delay;
                    self.retry_delay = (self.retry_delay * 2).min(MAX_RETRY_DELAY);
                    self.failed = Some(pending);
                } else {
                    warn!(%err, "failed to apply foreign writes while the writer's batch is failing");
                }
            },
        }
        Ok(())
    }

    /// Stops the writer, since consensus would wait forever for a write that cannot be encoded.
    fn fail(&mut self, err: anyhow::Error) {
        error!(err = format!("{err:#}"), "failed to encode a storage write");
        self.fatal.get_or_insert(StorageError::Encoding(err));
    }

    /// Whether the decide log is full, if that differs from what was last reported.
    fn decide_log_full_changed(&mut self) -> Option<bool> {
        let bytes = *self.decide_log_bytes.borrow_and_update();
        if let Some(m) = &self.metrics {
            m.decide_log_bytes
                .set(usize::try_from(bytes).unwrap_or(usize::MAX));
        }
        let full = bytes >= self.decide_log_limit;
        if full == self.decide_log_full {
            return None;
        }
        self.decide_log_full = full;
        if full {
            warn!(
                bytes,
                limit = self.decide_log_limit,
                "the decide log is full; holding vote2 until the store acknowledges entries"
            );
        } else {
            info!(bytes, "the decide log has room again");
        }
        Some(full)
    }

    fn queued(
        &mut self,
        written: anyhow::Result<()>,
        output: StorageOutput<T>,
        histogram: impl FnOnce(&StorageMetrics) -> &Arc<dyn Histogram>,
    ) {
        if let Err(err) = written {
            self.fail(err.context(format!("{output:?}")));
            return;
        }
        if let Some(m) = &self.metrics {
            self.pending
                .timers
                .push(Measurement::start(histogram(m).clone()));
        }
        self.pending.outputs.push(output);
    }

    fn spawn(&self, mut pending: Pending<T>) -> JoinHandle<Applied<T>> {
        let store = self.store.clone();
        let decided_state = self.decided_state.clone();
        let apply_histogram = self.metrics.as_ref().map(|m| m.apply.clone());
        tokio::task::spawn_blocking(move || {
            if let Err(err) = pending.add_decided(decided_state.as_deref()) {
                return Applied {
                    pending,
                    result: Ok(()),
                    encoding: Some(err),
                };
            }
            let timer = apply_histogram.map(Measurement::start);
            let result = store.store().apply(&pending.combined());
            drop(timer);
            if result.is_ok()
                && (pending.resizes_decide_log || pending.foreign_resizes_decide_log)
                && let Err(err) = store.measure_decide_log()
            {
                warn!(%err, "failed to measure the decide log");
            }
            Applied {
                pending,
                result,
                encoding: None,
            }
        })
    }
}

/// Where payloads go instead of the store.
struct PayloadSink {
    queue: PayloadQueue,
    acks: Acks,

    /// Payloads offered and not yet acknowledged, by view and commitment as offered.
    offered: BTreeMap<Acked, (ViewNumber, VidCommitment2)>,
}

/// The output that reports the payload `acked` durable, if it was offered.
fn acked_output<T: NodeType>(
    sink: &mut Option<PayloadSink>,
    acked: Acked,
) -> Option<StorageOutput<T>> {
    let (view, commitment) = sink.as_mut()?.offered.remove(&acked)?;
    Some(StorageOutput::Payload(view, commitment))
}
