use std::{collections::BTreeMap, marker::PhantomData, time::Duration};

use async_trait::async_trait;
use hotshot::{traits::BlockPayload, types::SignatureKey};
use hotshot_example_types::storage_types::TestStorage;
use hotshot_types::{
    data::{
        DaProposal2, EpochNumber, QuorumProposal2, QuorumProposalWrapper, VidCommitment,
        VidDisperseShare, VidDisperseShare2, ViewChangeEvidence2, ViewNumber,
    },
    message::Proposal as SignedProposal,
    traits::{EncodeBytes, node_implementation::NodeType, storage::Storage as StorageTrait},
    utils::EpochTransitionIndicator,
};
use tokio::{spawn, task::JoinHandle, time::sleep};
use tracing::{error, warn};

use crate::message::{Certificate2, Proposal};

const RETRY_DELAY: Duration = Duration::from_millis(300);

/// New protocol storage extension for data that is not part of the legacy HotShot storage trait.
#[async_trait]
pub trait NewProtocolStorage<T: NodeType>: StorageTrait<T> {
    async fn append_cert2(&self, view: ViewNumber, cert: Certificate2<T>) -> anyhow::Result<()>;
}

pub struct Storage<T: NodeType, S: NewProtocolStorage<T>> {
    storage: S,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    handles: BTreeMap<ViewNumber, Vec<JoinHandle<()>>>,
}

impl<T: NodeType, S: NewProtocolStorage<T>> Storage<T, S> {
    pub fn new(storage: S, private_key: <T::SignatureKey as SignatureKey>::PrivateKey) -> Self {
        Self {
            storage,
            private_key,
            handles: BTreeMap::new(),
        }
    }

    pub fn append_vid(&mut self, vid_share: VidDisperseShare2<T>) {
        let view = vid_share.view_number;
        let storage = self.storage.clone();
        let private_key = self.private_key.clone();
        let handle = spawn(async move {
            let share: VidDisperseShare<T> = VidDisperseShare::V2(vid_share);
            let Some(proposal) = share.to_proposal(&private_key) else {
                error!("failed to sign VID share for storage");
                return;
            };
            loop {
                match storage.append_vid(&proposal).await {
                    Ok(()) => return,
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
        let handle = spawn(async move {
            let data = DaProposal2 {
                encoded_transactions: block_payload.encode(),
                metadata,
                view_number,
                epoch: Some(epoch),
                epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
            };
            let Ok(signature) = T::SignatureKey::sign(&private_key, &[]) else {
                error!("failed to sign DA proposal for storage");
                return;
            };
            let proposal = SignedProposal {
                data,
                signature,
                _pd: PhantomData,
            };
            loop {
                match storage.append_da2(&proposal, vid_commit).await {
                    Ok(()) => return,
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
        let handle = spawn(async move {
            loop {
                match storage.append_cert2(view, cert2.clone()).await {
                    Ok(()) => return,
                    Err(err) => {
                        warn!(%err, %view, "failed to append cert2, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view).or_default().push(handle);
    }

    pub fn append_proposal(&mut self, proposal: Proposal<T>) {
        let view = proposal.view_number;
        let storage = self.storage.clone();
        let private_key = self.private_key.clone();
        let handle = spawn(async move {
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
                return;
            };
            let signed = SignedProposal {
                data,
                signature,
                _pd: PhantomData,
            };
            loop {
                match storage.append_proposal_wrapper(&signed).await {
                    Ok(()) => return,
                    Err(err) => {
                        warn!(%err, "failed to append proposal, retrying");
                        sleep(RETRY_DELAY).await;
                    },
                }
            }
        });
        self.handles.entry(view).or_default().push(handle);
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
}

#[async_trait]
impl<T: NodeType> NewProtocolStorage<T> for TestStorage<T> {
    async fn append_cert2(&self, _view: ViewNumber, _cert: Certificate2<T>) -> anyhow::Result<()> {
        Ok(())
    }
}
