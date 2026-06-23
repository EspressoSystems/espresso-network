use std::{collections::BTreeMap, marker::PhantomData, time::Duration};

use anyhow::anyhow;
use async_trait::async_trait;
use committable::Commitment;
use hotshot::{traits::BlockPayload, types::SignatureKey};
use hotshot_example_types::storage_types::TestStorage;
use hotshot_types::{
    data::{
        DaProposal, DaProposal2, EpochNumber, Leaf2, QuorumProposal, QuorumProposal2,
        QuorumProposalWrapper, VidCommitment, VidDisperseShare, VidDisperseShare2,
        ViewChangeEvidence2, ViewNumber,
    },
    drb::{DrbInput, DrbResult},
    event::HotShotAction,
    message::Proposal as SignedProposal,
    simple_certificate::{
        LightClientStateUpdateCertificateV2, NextEpochQuorumCertificate2, QuorumCertificate,
        QuorumCertificate2, UpgradeCertificate,
    },
    traits::{EncodeBytes, node_implementation::NodeType, storage::Storage as StorageTrait},
    utils::EpochTransitionIndicator,
    vote::HasViewNumber,
};
use tokio::{
    task::{AbortHandle, JoinSet},
    time::sleep,
};
use tracing::{error, info, warn};

use crate::{
    helpers::proposal_commitment,
    message::{Certificate1, Certificate2, Proposal},
};

const RETRY_DELAY: Duration = Duration::from_millis(300);

/// New protocol storage extension for data that is not part of the legacy HotShot storage trait.
#[async_trait]
pub trait NewProtocolStorage<T: NodeType>: StorageTrait<T> {
    async fn append_cert2(&self, view: ViewNumber, cert: Certificate2<T>) -> anyhow::Result<()>;

    /// Persist the locked QC, written before each phase-2 vote so the lock
    /// survives a restart instead of regressing to the decided-anchor QC.
    async fn append_high_qc2(&self, high_qc: Certificate1<T>) -> anyhow::Result<()>;

    /// Load the persisted locked QC, if any.
    async fn load_high_qc2(&self) -> anyhow::Result<Option<Certificate1<T>>>;
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

pub struct Storage<T: NodeType, S> {
    storage: S,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    tasks: JoinSet<Option<StorageOutput<T>>>,
    handles: BTreeMap<ViewNumber, Vec<AbortHandle>>,
}

impl<T: NodeType, S: NewProtocolStorage<T>> Storage<T, S> {
    pub fn new(storage: S, private_key: <T::SignatureKey as SignatureKey>::PrivateKey) -> Self {
        Self {
            storage,
            private_key,
            tasks: JoinSet::new(),
            handles: BTreeMap::new(),
        }
    }

    pub fn append_vid(&mut self, vid_share: VidDisperseShare2<T>) {
        let view = vid_share.view_number;
        let storage = self.storage.clone();
        let private_key = self.private_key.clone();
        let handle = self.tasks.spawn(async move {
            let share: VidDisperseShare<T> = VidDisperseShare::V2(vid_share);
            let Some(proposal) = share.to_proposal(&private_key) else {
                error!("failed to sign VID share for storage");
                return None;
            };
            loop {
                match storage.append_vid(&proposal).await {
                    Ok(()) => return Some(StorageOutput::Vid(view)),
                    Err(err) => {
                        warn!(%err, "failed to append VID share, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view).or_default().push(handle);
    }

    pub fn append_da(
        &mut self,
        view_number: ViewNumber,
        epoch: EpochNumber,
        block_payload: T::BlockPayload,
        metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
        vid_commit: VidCommitment,
    ) {
        let storage = self.storage.clone();
        let private_key = self.private_key.clone();
        let handle = self.tasks.spawn(async move {
            let data = DaProposal2 {
                encoded_transactions: block_payload.encode(),
                metadata,
                view_number,
                epoch: Some(epoch),
                epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
            };
            let Ok(signature) = T::SignatureKey::sign(&private_key, &[]) else {
                error!("failed to sign DA proposal for storage");
                return None;
            };
            let proposal = SignedProposal {
                data,
                signature,
                _pd: PhantomData,
            };
            loop {
                match storage.append_da2(&proposal, vid_commit).await {
                    Ok(()) => return None,
                    Err(err) => {
                        warn!(%err, "failed to append DA proposal, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view_number).or_default().push(handle);
    }

    pub fn append_cert2(&mut self, view: ViewNumber, cert2: Certificate2<T>) {
        let storage = self.storage.clone();
        let handle = self.tasks.spawn(async move {
            loop {
                match storage.append_cert2(view, cert2.clone()).await {
                    Ok(()) => return None,
                    Err(err) => {
                        warn!(%err, %view, "failed to append cert2, retrying");
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
        let handle = self.tasks.spawn(async move {
            loop {
                match storage.append_high_qc2(high_qc.clone()).await {
                    Ok(()) => return Some(StorageOutput::HighQc(view)),
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
        let handle = self.tasks.spawn(async move {
            loop {
                match storage.update_state_cert(state_cert.clone()).await {
                    Ok(()) => return None,
                    Err(err) => {
                        warn!(%err, epoch = %state_cert.epoch, "failed to append state cert, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view).or_default().push(handle);
    }

    pub fn append_proposal(&mut self, proposal: Proposal<T>) {
        let view = proposal.view_number;
        let commitment = proposal_commitment(&proposal);
        let storage = self.storage.clone();
        let private_key = self.private_key.clone();
        let handle = self.tasks.spawn(async move {
            let data = QuorumProposalWrapper {
                proposal: QuorumProposal2 {
                    block_header: proposal.block_header,
                    view_number: proposal.view_number,
                    epoch: Some(proposal.epoch),
                    justify_qc: proposal.justify_qc,
                    next_epoch_justify_qc: None,
                    upgrade_certificate: proposal.upgrade_certificate,
                    view_change_evidence: proposal
                        .view_change_evidence
                        .map(ViewChangeEvidence2::Timeout),
                    next_drb_result: proposal.next_drb_result,
                    state_cert: proposal.state_cert,
                },
            };
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
                    Ok(()) => return Some(StorageOutput::Proposal(view, commitment)),
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
        let handle = self.tasks.spawn(async move {
            loop {
                match storage.record_action(view, epoch, kind.into()).await {
                    Ok(()) => return Some(StorageOutput::Action(view, kind)),
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
    async fn append_cert2(&self, _view: ViewNumber, _cert: Certificate2<T>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn append_high_qc2(&self, high_qc: Certificate1<T>) -> anyhow::Result<()> {
        // `Certificate1<T>` is a `QuorumCertificate2<T>`; reuse the monotonic legacy slot.
        StorageTrait::update_high_qc2(self, high_qc).await
    }

    async fn load_high_qc2(&self) -> anyhow::Result<Option<Certificate1<T>>> {
        Ok(self.high_qc_cloned().await)
    }
}

/// Drop-on-the-floor implementation of [`StorageTrait`] + [`NewProtocolStorage`].
///
/// Every `append_*` / `update_*` / `store_*` call returns `Ok(())` without
/// keeping the data, and every `load_*` call returns an error.  This trades
/// crash-recovery for **bounded memory**: the bench's previous use of
/// `TestStorage` accumulated every dispersed payload, recovered proposal,
/// and cert in process memory, leaking at roughly one block-payload per view
/// (`memory_leak.py` measured +6 GB/min on 50 MB blocks).  `NullStorage`
/// keeps RSS flat by simply throwing the data away — appropriate for the
/// throughput/latency bench where persistence is irrelevant.
///
/// Do NOT use in production paths or any test that exercises restart logic.
#[derive(Clone, Default)]
pub struct NullStorage<T: NodeType> {
    _pd: PhantomData<T>,
}

impl<T: NodeType> NullStorage<T> {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl<T: NodeType> StorageTrait<T> for NullStorage<T> {
    async fn append_vid(
        &self,
        _proposal: &SignedProposal<T, VidDisperseShare<T>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn append_da(
        &self,
        _proposal: &SignedProposal<T, DaProposal<T>>,
        _vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn append_proposal(
        &self,
        _proposal: &SignedProposal<T, QuorumProposal<T>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn append_proposal2(
        &self,
        _proposal: &SignedProposal<T, QuorumProposal2<T>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn record_action(
        &self,
        _view: ViewNumber,
        _epoch: Option<EpochNumber>,
        _action: HotShotAction,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn update_high_qc(&self, _high_qc: QuorumCertificate<T>) -> anyhow::Result<()> {
        Ok(())
    }
    async fn update_state_cert(
        &self,
        _state_cert: LightClientStateUpdateCertificateV2<T>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn update_next_epoch_high_qc2(
        &self,
        _next_epoch_high_qc: NextEpochQuorumCertificate2<T>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn update_eqc(
        &self,
        _high_qc: QuorumCertificate2<T>,
        _next_epoch_high_qc: NextEpochQuorumCertificate2<T>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn update_decided_upgrade_certificate(
        &self,
        _decided_upgrade_certificate: Option<UpgradeCertificate<T>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn store_drb_result(
        &self,
        _epoch: EpochNumber,
        _drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn store_epoch_root(
        &self,
        _epoch: EpochNumber,
        _block_header: T::BlockHeader,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    async fn store_drb_input(&self, _drb_input: DrbInput) -> anyhow::Result<()> {
        Ok(())
    }
    async fn load_drb_input(&self, _epoch: u64) -> anyhow::Result<DrbInput> {
        Err(anyhow!(
            "NullStorage discards data — load_drb_input is not supported"
        ))
    }
}

#[async_trait]
impl<T: NodeType> NewProtocolStorage<T> for NullStorage<T> {
    async fn append_cert2(&self, _view: ViewNumber, _cert: Certificate2<T>) -> anyhow::Result<()> {
        Ok(())
    }
    async fn append_high_qc2(&self, _high_qc: Certificate1<T>) -> anyhow::Result<()> {
        Ok(())
    }
    async fn load_high_qc2(&self) -> anyhow::Result<Option<Certificate1<T>>> {
        Ok(None)
    }
}
