use std::{collections::BTreeMap, marker::PhantomData, time::Duration};

use async_trait::async_trait;
use committable::Commitment;
use hotshot::{traits::BlockPayload, types::SignatureKey};
use hotshot_example_types::storage_types::TestStorage;
use hotshot_types::{
    data::{
        DaProposal2, EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper, VidCommitment,
        VidDisperseShare, VidDisperseShare2, ViewChangeEvidence2, ViewNumber,
    },
    event::HotShotAction,
    message::Proposal as SignedProposal,
    simple_certificate::LightClientStateUpdateCertificateV2,
    traits::{EncodeBytes, node_implementation::NodeType, storage::Storage as StorageTrait},
    utils::EpochTransitionIndicator,
};
use tokio::{
    task::{AbortHandle, JoinSet},
    time::sleep,
};
use tracing::{error, info, warn};

use crate::{
    helpers::proposal_commitment,
    message::{Certificate2, Proposal},
};

const RETRY_DELAY: Duration = Duration::from_millis(300);

/// New protocol storage extension for data that is not part of the legacy HotShot storage trait.
#[async_trait]
pub trait NewProtocolStorage<T: NodeType>: StorageTrait<T> {
    async fn append_cert2(&self, view: ViewNumber, cert: Certificate2<T>) -> anyhow::Result<()>;
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
}

impl<T: NodeType> StorageOutput<T> {
    pub fn view_number(&self) -> ViewNumber {
        match self {
            Self::Proposal(view, _) | Self::Vid(view) | Self::Action(view, _) => *view,
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
}
