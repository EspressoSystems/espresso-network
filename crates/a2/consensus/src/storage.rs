//! The single writer of the consensus store.
//!
//! Writes are queued synchronously into a batch. While one batch is being applied on a blocking
//! thread, the next one fills, so all writes made during one commit share the next commit and
//! its fsync. Each write names the [`StorageOutput`] that reports it durable, which consensus
//! waits on before sending the messages that depend on it.

use std::{collections::BTreeMap, marker::PhantomData, mem, sync::Arc, time::Duration};

use committable::Commitment;
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
    vote::HasViewNumber,
};
use tokio::{sync::watch, task::JoinHandle};
use tracing::{error, info, warn};

use crate::{
    coordinator::metrics::Measurement,
    discovery::SignedServiceRecord,
    helpers::proposal_commitment,
    message::{Certificate1, Certificate2, Proposal},
    state::StateEntry,
    store::{self, AppWrite, ConsensusStore},
};

const RETRY_DELAY: Duration = Duration::from_millis(300);

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

/// What the application keeps of the latest decided state, in its own tables of the store.
///
/// Its writes join the batch that appends the decide to the decide log, so the tables never
/// fall behind or run ahead of the anchor.
pub trait DecidedState<T: NodeType>: Send + Sync {
    /// The writes that bring the application's tables up to date with the decided `leaf`.
    ///
    /// Called for each leaf of a decide, oldest first. `state` is what this node holds for
    /// `leaf`: a state with its delta if this node validated `leaf`, else one without, or none.
    fn decided(&mut self, leaf: &Leaf2<T>, state: Option<&StateEntry<T>>) -> Vec<AppWrite>;
}

/// Storage cannot go on.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The store failed in a way retrying cannot fix.
    #[error("consensus store failed: {0}")]
    Store(#[from] consensus_store::Error),

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

    decided_state: Option<Box<dyn DecidedState<T>>>,

    metrics: Option<StorageMetrics>,
}

/// A batch and what to report once it is durable.
struct Pending<T: NodeType> {
    batch: Batch,
    outputs: Vec<StorageOutput<T>>,
    timers: Vec<Measurement>,

    /// The height after the newest decide log entry the batch writes.
    next_height: Option<u64>,
}

impl<T: NodeType> Default for Pending<T> {
    fn default() -> Self {
        Self {
            batch: Batch::new(),
            outputs: Vec::new(),
            timers: Vec::new(),
            next_height: None,
        }
    }
}

impl<T: NodeType> Pending<T> {
    fn is_empty(&self) -> bool {
        self.batch.is_empty()
    }
}

struct Applied<T: NodeType> {
    pending: Pending<T>,
    result: Result<(), consensus_store::Error>,
}

impl<T: NodeType, S: Store> Storage<T, S> {
    pub fn new(
        store: ConsensusStore<T, S>,
        private_key: &<T::SignatureKey as SignatureKey>::PrivateKey,
    ) -> Self {
        let signature =
            T::SignatureKey::sign(private_key, &[]).expect("signing empty data cannot fail");
        let decide_log_bytes = store.decide_log_bytes();
        Self {
            store,
            signature,
            pending: Pending::default(),
            applying: None,
            durable: Default::default(),
            decide_log_bytes,
            decide_log_limit: DEFAULT_DECIDE_LOG_LIMIT,
            decide_log_full: false,
            state_certs: BTreeMap::new(),
            decided_state: None,
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
    /// holds its vote2 until the ingester frees entries.
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
        self.decided_state = Some(decided_state);
        self
    }

    /// Persists the payload of a block. Vote2 on the block waits for it.
    pub fn append_da(
        &mut self,
        view: ViewNumber,
        _epoch: EpochNumber,
        block_payload: T::BlockPayload,
        _metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
        vid_commit: VidCommitment,
    ) {
        let VidCommitment::V2(commitment) = vid_commit else {
            warn!(%view, "payload with a non-V2 VID commitment; not stored");
            return;
        };
        let encoded = block_payload.encode();
        let written = store::put_payload(&mut self.pending.batch, view, &commitment, encoded);
        self.queued(written, StorageOutput::Payload(view, commitment), |m| {
            &m.append_da
        });
    }

    /// Persists the own certificate of the last block of an epoch. Nothing waits on it.
    pub fn append_boundary_qc2(&mut self, qc: Certificate1<T>) {
        if let Err(err) = store::put_boundary_qc(&mut self.pending.batch, &qc) {
            error!(%err, view = %qc.view_number(), "failed to encode boundary qc");
        }
    }

    /// Persists the locked QC; once durable, emits [`StorageOutput::HighQc`], which gates the
    /// matching phase-2 vote.
    pub fn append_high_qc2(&mut self, high_qc: Certificate1<T>) {
        let lock = LockView::of(&high_qc);
        let written = store::put_locked_qc(&mut self.pending.batch, &high_qc);
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
            Err(err) => error!(%err, %view, "failed to encode state certificate"),
        }
    }

    /// Persists the newest record of a validator's API service. Nothing waits on it.
    pub fn append_service_record(&mut self, record: &SignedServiceRecord<T>) {
        if let Err(err) = store::put_service_record(&mut self.pending.batch, record) {
            error!(%err, signer = %record.signer, "failed to encode service record");
        }
    }

    /// Deletes the records of the epochs before `epoch`.
    pub fn drop_service_records(&mut self, epoch: EpochNumber) {
        store::drop_service_records(&mut self.pending.batch, epoch);
    }

    pub fn update_decided_upgrade_certificate(&mut self, cert: UpgradeCertificate<T>) {
        if let Err(err) = store::put_upgrade_cert(&mut self.pending.batch, &cert) {
            error!(%err, "failed to encode upgrade certificate");
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

    pub fn record_action(&mut self, view: ViewNumber, kind: ActionKind) {
        store::record_action(&mut self.pending.batch, view, kind);
        self.queued(Ok(()), StorageOutput::Action(view, kind), |m| {
            &m.record_action
        });
    }

    /// Appends a decide to the decide log and moves the anchor: `leaves` newest first, with
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
        match store::put_decided(
            &mut self.pending.batch,
            leaves,
            cert1,
            cert2,
            &vid_common,
            &state_certs,
        ) {
            Ok(()) => {
                let next = leaves.iter().map(|l| l.height() + 1).max();
                self.pending.next_height = self.pending.next_height.max(next);
            },
            Err(err) => {
                error!(%err, view = %cert1.view_number(), "failed to encode decided leaves");
                return;
            },
        }
        let Some(decided_state) = &mut self.decided_state else {
            return;
        };
        for (i, leaf) in leaves.iter().enumerate().rev() {
            let writes = decided_state.decided(leaf, states.get(i).and_then(Option::as_ref));
            if let Err(err) = store::put_app(&mut self.pending.batch, &writes) {
                error!(%err, view = %leaf.view_number(), "failed to write the decided state");
            }
        }
    }

    /// Deletes the stored proposals and payloads of views before `view`.
    pub fn gc(&mut self, view: ViewNumber) {
        store::collect_garbage(&mut self.pending.batch, view);
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
            if let Some(output) = self.durable.pop_front() {
                return Ok(output);
            }
            if let Some(full) = self.decide_log_full_changed() {
                return Ok(StorageOutput::DecideLogFull(full));
            }
            let idle = self.applying.is_none() && self.pending.is_empty();
            // A clone, since `apply_next` borrows `self`. It has seen what the original has.
            let mut decide_log_bytes = self.decide_log_bytes.clone();
            tokio::select! {
                applied = self.apply_next(), if !idle => applied?,
                // The sender lives as long as the store, which this writer holds.
                _ = decide_log_bytes.changed() => {},
            }
        }
    }

    /// Applies every queued write, for shutdown.
    pub async fn flush(mut self) {
        info!("flushing storage during shutdown");
        while self.applying.is_some() || !self.pending.is_empty() {
            if let Err(err) = self.apply_next().await {
                error!(%err, "storage failed during shutdown");
                return;
            }
        }
        info!("storage flush complete");
    }

    /// Waits for the batch being applied, starting one from the queued writes if none is, and
    /// moves its outputs to `durable`. A failed batch is started again after a delay.
    ///
    /// Cancel-safe: a batch in progress stays in progress.
    async fn apply_next(&mut self) -> Result<(), StorageError> {
        if self.applying.is_none() {
            let pending = mem::take(&mut self.pending);
            self.applying = Some(self.spawn(pending, Duration::ZERO));
        }
        let applying = self.applying.as_mut().expect("set above");
        let applied = match applying.await {
            Ok(applied) => applied,
            Err(err) if err.is_panic() => std::panic::resume_unwind(err.into_panic()),
            Err(_) => {
                self.applying = None;
                return Err(StorageError::Cancelled);
            },
        };
        self.applying = None;
        match applied.result {
            Ok(()) => {
                let Pending {
                    outputs,
                    next_height,
                    ..
                } = applied.pending;
                if let Some(next_height) = next_height {
                    self.store.announce(next_height);
                }
                self.durable.extend(outputs);
            },
            Err(err @ consensus_store::Error::Poisoned(_)) => return Err(err.into()),
            Err(err) => {
                warn!(%err, "failed to apply storage batch, retrying");
                self.applying = Some(self.spawn(applied.pending, RETRY_DELAY));
            },
        }
        Ok(())
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
                "the decide log is full; holding vote2 until the ingester frees entries"
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
            // Encoding failed: the write and its output are dropped, and whatever waits on
            // the output waits forever, as for a write that never finishes.
            error!(%err, ?output, "failed to encode storage write");
            return;
        }
        if let Some(m) = &self.metrics {
            self.pending
                .timers
                .push(Measurement::start(histogram(m).clone()));
        }
        self.pending.outputs.push(output);
    }

    fn spawn(&self, pending: Pending<T>, delay: Duration) -> JoinHandle<Applied<T>> {
        let store = self.store.clone();
        let apply_histogram = self.metrics.as_ref().map(|m| m.apply.clone());
        tokio::task::spawn_blocking(move || {
            std::thread::sleep(delay);
            let timer = apply_histogram.map(Measurement::start);
            let result = store.store().apply(&pending.batch);
            drop(timer);
            if result.is_ok()
                && pending.next_height.is_some()
                && let Err(err) = store.measure_decide_log()
            {
                warn!(%err, "failed to measure the decide log");
            }
            Applied { pending, result }
        })
    }
}
