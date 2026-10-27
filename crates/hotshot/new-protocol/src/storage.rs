use std::{
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use committable::Commitment;
use hotshot_example_types::storage_types::TestStorage;
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposalWrapper, VidDisperseShare, VidDisperseShare2, ViewNumber,
    },
    event::HotShotAction,
    message::Proposal as SignedProposal,
    simple_certificate::{LightClientStateUpdateCertificateV2, UpgradeCertificate},
    traits::{
        metrics::{Histogram, Metrics},
        node_implementation::NodeType,
        signature_key::SignatureKey,
        storage::Storage as StorageTrait,
    },
    vote::HasViewNumber,
};
use tokio::{
    task::{AbortHandle, JoinSet},
    time::sleep,
};
use tracing::{error, info, warn};

use crate::{
    coordinator::metrics::{Measurement, finish_measurement},
    helpers::proposal_commitment,
    message::{Certificate1, Proposal},
};

const RETRY_DELAY: Duration = Duration::from_millis(300);

/// New protocol storage extension for data that is not part of the legacy HotShot storage trait.
#[async_trait]
pub trait NewProtocolStorage<T: NodeType>: StorageTrait<T> {
    /// Persist the locked QC, written before each phase-2 vote so the lock
    /// survives a restart instead of regressing to the decided-anchor QC.
    async fn append_high_qc2(&self, high_qc: Certificate1<T>) -> anyhow::Result<()>;

    /// Load the persisted locked QC, if any.
    async fn load_high_qc2(&self) -> anyhow::Result<Option<Certificate1<T>>>;

    /// Persist a VID share, with `stamp` the unix seconds of its proposal's header. Backends that
    /// do not age shares by header time ignore it.
    async fn append_vid_stamped(
        &self,
        proposal: &SignedProposal<T, VidDisperseShare<T>>,
        _stamp: u32,
    ) -> anyhow::Result<()> {
        StorageTrait::append_vid(self, proposal).await
    }

    /// This node's stored VID share for `view`, for serving to peers.
    async fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<SignedProposal<T, VidDisperseShare<T>>>>;

    /// Views whose VID share is stored.
    async fn held_vid_views(&self) -> anyhow::Result<BTreeSet<ViewNumber>>;

    /// Decided leaves with header time at least `min_header_ts`, newest first.
    async fn decided_leaves(&self, min_header_ts: u64) -> anyhow::Result<Vec<Leaf2<T>>>;

    /// Write the archive copies of the decided leaves at `views`, which outlive the wal and are
    /// lost with the share store.
    async fn restore_leaf_copies(&self, _views: &BTreeSet<ViewNumber>) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionKind {
    Vote,
    Propose,
}

impl From<ActionKind> for HotShotAction {
    fn from(kind: ActionKind) -> Self {
        match kind {
            ActionKind::Vote => HotShotAction::Vote,
            ActionKind::Propose => HotShotAction::Propose,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageOutput<T: NodeType> {
    Proposal(ViewNumber, Commitment<Leaf2<T>>),
    Vid(ViewNumber),
    Action(ViewNumber, ActionKind),
    /// The locked QC for the given view has been durably persisted.
    HighQc(ViewNumber),
}

impl<T: NodeType> StorageOutput<T> {
    pub fn view_number(&self) -> ViewNumber {
        match self {
            Self::Proposal(view, _)
            | Self::Vid(view)
            | Self::Action(view, _)
            | Self::HighQc(view) => *view,
        }
    }
}

/// Request-to-completion latency of each storage operation, including task
/// queueing, backend lock waits, and error-retry loops. These latencies gate
/// consensus progress: proposal release waits on `append_proposal` and
/// `record_action(Propose)`, and phase-2 votes wait on `append_vid`,
/// `record_action(Vote)`, and `append_high_qc2`.
///
/// [`Measurement`] records on drop, so an op aborted by GC or shutdown still
/// records its elapsed time up to the abort.
pub struct StorageMetrics {
    append_vid: Arc<dyn Histogram>,
    append_high_qc: Arc<dyn Histogram>,
    append_state_cert: Arc<dyn Histogram>,
    append_proposal: Arc<dyn Histogram>,
    record_action: Arc<dyn Histogram>,
}

impl StorageMetrics {
    pub fn new(m: &dyn Metrics) -> Self {
        let histogram = |name: &str| -> Arc<dyn Histogram> {
            m.create_histogram(format!("storage_{name}"), Some("s".into()))
                .into()
        };
        Self {
            append_vid: histogram("append_vid"),
            append_high_qc: histogram("append_high_qc"),
            append_state_cert: histogram("append_state_cert"),
            append_proposal: histogram("append_proposal"),
            record_action: histogram("record_action"),
        }
    }
}

pub struct Storage<T: NodeType, S> {
    storage: S,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    tasks: JoinSet<Option<StorageOutput<T>>>,
    handles: BTreeMap<ViewNumber, Vec<AbortHandle>>,
    metrics: Option<StorageMetrics>,
}

impl<T: NodeType, S: NewProtocolStorage<T>> Storage<T, S> {
    pub fn new(storage: S, private_key: <T::SignatureKey as SignatureKey>::PrivateKey) -> Self {
        Self {
            storage,
            private_key,
            tasks: JoinSet::new(),
            handles: BTreeMap::new(),
            metrics: None,
        }
    }

    pub fn with_metrics(mut self, m: &dyn Metrics) -> Self {
        if m.is_recording() {
            self.metrics = Some(StorageMetrics::new(m));
        }
        self
    }

    /// `header_timestamp` is the proposal header's timestamp in seconds.
    pub fn append_vid(&mut self, vid_share: VidDisperseShare2<T>, header_timestamp: u64) {
        let view = vid_share.view_number;
        let Ok(stamp) = u32::try_from(header_timestamp) else {
            error!(
                %view,
                header_timestamp,
                "header timestamp does not fit a u32 stamp"
            );
            return;
        };
        let storage = self.storage.clone();
        let private_key = self.private_key.clone();
        let timer = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.append_vid.clone()));
        let handle = self.tasks.spawn(async move {
            let share: VidDisperseShare<T> = VidDisperseShare::V2(vid_share);
            let Some(proposal) = share.to_proposal(&private_key) else {
                error!("failed to sign VID share for storage");
                return None;
            };
            loop {
                match storage.append_vid_stamped(&proposal, stamp).await {
                    Ok(()) => {
                        finish_measurement(timer);
                        return Some(StorageOutput::Vid(view));
                    },
                    Err(err) => {
                        warn!(%err, "failed to append VID share, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view).or_default().push(handle);
    }

    /// Persist the locked QC; on success emits [`StorageOutput::HighQc`], which
    /// gates the matching phase-2 vote.
    pub fn append_high_qc2(&mut self, high_qc: Certificate1<T>) {
        let view = high_qc.view_number();
        let storage = self.storage.clone();
        let timer = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.append_high_qc.clone()));
        let handle = self.tasks.spawn(async move {
            loop {
                match storage.append_high_qc2(high_qc.clone()).await {
                    Ok(()) => {
                        finish_measurement(timer);
                        return Some(StorageOutput::HighQc(view));
                    },
                    Err(err) => {
                        warn!(%err, %view, "failed to append high qc, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view).or_default().push(handle);
    }

    pub fn append_state_cert(
        &mut self,
        view: ViewNumber,
        state_cert: LightClientStateUpdateCertificateV2<T>,
    ) {
        let storage = self.storage.clone();
        let timer = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.append_state_cert.clone()));
        let handle = self.tasks.spawn(async move {
            loop {
                match storage.update_state_cert(state_cert.clone()).await {
                    Ok(()) => {
                        finish_measurement(timer);
                        return None;
                    },
                    Err(err) => {
                        warn!(%err, epoch = %state_cert.epoch, "failed to append state cert, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view).or_default().push(handle);
    }

    /// Persist a decided upgrade certificate. Keyed at
    /// `new_version_first_view` so the decided-view GC cannot abort the write
    /// before the certificate takes effect.
    pub fn update_decided_upgrade_certificate(&mut self, cert: UpgradeCertificate<T>) {
        let view = cert.data.new_version_first_view;
        let storage = self.storage.clone();
        let handle = self.tasks.spawn(async move {
            loop {
                match storage
                    .update_decided_upgrade_certificate(Some(cert.clone()))
                    .await
                {
                    Ok(()) => return None,
                    Err(err) => {
                        warn!(%err, "failed to persist decided upgrade certificate, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view).or_default().push(handle);
    }

    /// `proposal` must already be validated: `state_cert`'s presence is not
    /// re-checked here, only gated at write time by `state_cert_matches_parent`
    /// (received proposals) and `maybe_propose` (self-proposed ones).
    /// `seed_proposals` reads persisted rows back on restart without
    /// re-checking, so that write-time gate is the only thing enforcing this.
    pub fn append_proposal(&mut self, proposal: Proposal<T>) {
        let view = proposal.view_number;
        let commitment = proposal_commitment(&proposal);
        let storage = self.storage.clone();
        let private_key = self.private_key.clone();
        let timer = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.append_proposal.clone()));
        let handle = self.tasks.spawn(async move {
            let data = QuorumProposalWrapper::from(proposal);
            let Ok(signature) = T::SignatureKey::sign(&private_key, &[]) else {
                error!("failed to sign quorum proposal for storage");
                return None;
            };
            let signed = SignedProposal {
                data,
                signature,
                _pd: PhantomData,
            };
            loop {
                match storage.append_proposal_wrapper(&signed).await {
                    Ok(()) => {
                        finish_measurement(timer);
                        return Some(StorageOutput::Proposal(view, commitment));
                    },
                    Err(err) => {
                        warn!(%err, "failed to append proposal, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view).or_default().push(handle);
    }

    pub fn record_action(
        &mut self,
        view: ViewNumber,
        epoch: Option<EpochNumber>,
        kind: ActionKind,
    ) {
        let storage = self.storage.clone();
        let timer = self
            .metrics
            .as_ref()
            .map(|m| Measurement::start(m.record_action.clone()));
        let handle = self.tasks.spawn(async move {
            loop {
                match storage.record_action(view, epoch, kind.into()).await {
                    Ok(()) => {
                        finish_measurement(timer);
                        return Some(StorageOutput::Action(view, kind));
                    },
                    Err(err) => {
                        warn!(%err, %view, ?kind, "failed to record action, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view).or_default().push(handle);
    }

    pub async fn next(&mut self) -> Option<StorageOutput<T>> {
        loop {
            match self.tasks.join_next().await? {
                Ok(Some(output)) => return Some(output),
                Ok(None) | Err(_) => continue,
            }
        }
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        let keep = self.handles.split_off(&view_number);
        for handles in self.handles.values() {
            for handle in handles {
                handle.abort();
            }
        }
        self.handles = keep;
    }

    /// Wait for all current storage writes to complete.
    pub async fn flush(mut self) {
        info!(
            tasks = self.tasks.len(),
            "flushing storage tasks during shutdown"
        );
        while let Some(result) = self.tasks.join_next().await {
            if let Err(err) = result {
                warn!(%err, "storage task failed during shutdown");
            }
        }
        info!("storage flush complete");
    }
}

#[async_trait]
impl<T: NodeType> NewProtocolStorage<T> for TestStorage<T> {
    async fn append_high_qc2(&self, high_qc: Certificate1<T>) -> anyhow::Result<()> {
        // `Certificate1<T>` is a `QuorumCertificate2<T>`; reuse the monotonic legacy slot.
        StorageTrait::update_high_qc2(self, high_qc).await
    }

    async fn load_high_qc2(&self) -> anyhow::Result<Option<Certificate1<T>>> {
        Ok(self.high_qc_cloned().await)
    }

    async fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<SignedProposal<T, VidDisperseShare<T>>>> {
        Ok(self.vid_share(view).await)
    }

    async fn held_vid_views(&self) -> anyhow::Result<BTreeSet<ViewNumber>> {
        Ok(self.vids_cloned().await.into_keys().collect())
    }

    async fn decided_leaves(&self, min_header_ts: u64) -> anyhow::Result<Vec<Leaf2<T>>> {
        Ok(self.decided_leaves_cloned(min_header_ts).await)
    }
}
