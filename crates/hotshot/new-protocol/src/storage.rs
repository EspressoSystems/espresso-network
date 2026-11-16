//! The single writer of the consensus store.
//!
//! Writes are queued synchronously into a batch. While one batch is being applied on a blocking
//! thread, the next one fills, so all writes made during one commit share the next commit and
//! its fsync. Each write names the [`StorageOutput`] that reports it durable, which consensus
//! waits on before sending the messages that depend on it.

use std::{marker::PhantomData, mem, sync::Arc, time::Duration};

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
        metrics::{Histogram, Metrics},
        node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    vote::HasViewNumber,
};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::{
    coordinator::metrics::Measurement,
    helpers::proposal_commitment,
    message::{Certificate1, Certificate2, Proposal},
    store::{self, ConsensusStore, StoredPayload},
};

const RETRY_DELAY: Duration = Duration::from_millis(300);

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
}

impl<T: NodeType> StorageOutput<T> {
    pub fn view_number(&self) -> ViewNumber {
        match self {
            Self::Proposal(view, _) | Self::Action(view, _) | Self::Payload(view, _) => *view,
            Self::HighQc(lock) => lock.view,
        }
    }
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
    metrics: Option<StorageMetrics>,
}

/// A batch and what to report once it is durable.
struct Pending<T: NodeType> {
    batch: Batch,
    outputs: Vec<StorageOutput<T>>,
    timers: Vec<Measurement>,

    /// The height after the newest decide log entry the batch writes.
    next_height: Option<u64>,

    /// The payloads the batch writes.
    payloads: Vec<StoredPayload>,
}

impl<T: NodeType> Default for Pending<T> {
    fn default() -> Self {
        Self {
            batch: Batch::new(),
            outputs: Vec::new(),
            timers: Vec::new(),
            next_height: None,
            payloads: Vec::new(),
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
        Self {
            store,
            signature,
            pending: Pending::default(),
            applying: None,
            durable: Default::default(),
            metrics: None,
        }
    }

    pub fn with_metrics(mut self, m: &dyn Metrics) -> Self {
        if m.is_recording() {
            self.metrics = Some(StorageMetrics::new(m));
        }
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
        let size = encoded.len() as u64;
        let written = store::put_payload(&mut self.pending.batch, view, &commitment, encoded);
        if written.is_ok() {
            self.pending.payloads.push(StoredPayload {
                view,
                commitment,
                size,
            });
        }
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
            Ok(()) => self.pending.timers.extend(timer),
            Err(err) => error!(%err, %view, "failed to encode state certificate"),
        }
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
    /// the certificates that decided the newest, and this node's VID share of each.
    pub fn append_decided(
        &mut self,
        leaves: &[Leaf2<T>],
        cert1: &Certificate1<T>,
        cert2: Option<&Certificate2<T>>,
        vid_shares: &[Option<SignedProposal<T, VidDisperseShare2<T>>>],
    ) {
        let vid_common: Vec<_> = vid_shares
            .iter()
            .map(|share| share.as_ref().map(|s| s.data.common.clone()))
            .collect();
        match store::put_decided(&mut self.pending.batch, leaves, cert1, cert2, &vid_common) {
            Ok(()) => {
                let next = leaves.iter().map(|l| l.height() + 1).max();
                self.pending.next_height = self.pending.next_height.max(next);
            },
            Err(err) => {
                error!(%err, view = %cert1.view_number(), "failed to encode decided leaves")
            },
        }
    }

    /// Deletes the stored proposals and payloads of views before `view`.
    pub fn gc(&mut self, view: ViewNumber) {
        store::collect_garbage(&mut self.pending.batch, view);
    }

    /// Marks the state certificates of views up to `view` as decided.
    pub fn decided(&mut self, view: ViewNumber) {
        if let Err(err) = self
            .store
            .finalize_state_certs(&mut self.pending.batch, view)
        {
            warn!(%err, %view, "failed to finalize state certificates");
        }
    }

    /// The next write that has become durable.
    ///
    /// Starts applying the queued writes whenever no batch is being applied. Cancel-safe: a
    /// batch in progress stays in progress. Pends forever while there is nothing to write.
    pub async fn next(&mut self) -> Result<StorageOutput<T>, StorageError> {
        loop {
            if let Some(output) = self.durable.pop_front() {
                return Ok(output);
            }
            if self.applying.is_none() && self.pending.is_empty() {
                return std::future::pending().await;
            }
            self.apply_next().await?;
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
                    payloads,
                    ..
                } = applied.pending;
                self.store.announce(next_height, payloads);
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
        let store = self.store.store().clone();
        let apply_histogram = self.metrics.as_ref().map(|m| m.apply.clone());
        tokio::task::spawn_blocking(move || {
            std::thread::sleep(delay);
            let timer = apply_histogram.map(Measurement::start);
            let result = store.apply(&pending.batch);
            drop(timer);
            Applied { pending, result }
        })
    }
}
