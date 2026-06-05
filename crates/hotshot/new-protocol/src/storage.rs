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
    simple_certificate::LightClientStateUpdateCertificateV2,
    traits::{EncodeBytes, node_implementation::NodeType, storage::Storage as StorageTrait},
    utils::EpochTransitionIndicator,
};
use tokio::{spawn, task::JoinHandle, time::sleep};
use tracing::{error, warn};

use crate::message::{Certificate2, Proposal};

const RETRY_DELAY: Duration = Duration::from_millis(300);

/// Maximum number of attempts for a storage write before giving up. Together with
/// [`RETRY_DELAY`] this bounds the lifetime of a persistently failing write task to ~30s.
const MAX_APPEND_ATTEMPTS: usize = 100;

/// How many views below the GC view in-flight storage writes are allowed to keep running.
///
/// Writes for just-decided views must be allowed to complete: the decide pipeline reads
/// this data back from disk to build query-service decide events, so aborting them right
/// at the decide would lose data that was still in flight (e.g. a VID reconstruction that
/// finished just before its view was decided). Aborting below the horizon is only a
/// backstop against leaking stuck tasks; bounded retries terminate them anyway.
const GC_ABORT_HORIZON: u64 = 100;

/// New protocol storage extension for data that is not part of the legacy HotShot storage trait.
#[async_trait]
pub trait NewProtocolStorage<T: NodeType>: StorageTrait<T> {
    async fn append_cert2(&self, view: ViewNumber, cert: Certificate2<T>) -> anyhow::Result<()>;
}

pub struct Storage<T: NodeType, S> {
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
            for attempt in 1..=MAX_APPEND_ATTEMPTS {
                match storage.append_vid(&proposal).await {
                    Ok(()) => return,
                    Err(err) if attempt == MAX_APPEND_ATTEMPTS => {
                        error!(%err, "failed to append VID share after {MAX_APPEND_ATTEMPTS} attempts, giving up");
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
            for attempt in 1..=MAX_APPEND_ATTEMPTS {
                match storage.append_da2(&proposal, vid_commit).await {
                    Ok(()) => return,
                    Err(err) if attempt == MAX_APPEND_ATTEMPTS => {
                        error!(%err, "failed to append DA proposal after {MAX_APPEND_ATTEMPTS} attempts, giving up");
                    },
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
            for attempt in 1..=MAX_APPEND_ATTEMPTS {
                match storage.append_cert2(view, cert2.clone()).await {
                    Ok(()) => return,
                    Err(err) if attempt == MAX_APPEND_ATTEMPTS => {
                        error!(%err, %view, "failed to append cert2 after {MAX_APPEND_ATTEMPTS} attempts, giving up");
                    },
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
        let handle = spawn(async move {
            for attempt in 1..=MAX_APPEND_ATTEMPTS {
                match storage.update_state_cert(state_cert.clone()).await {
                    Ok(()) => return,
                    Err(err) if attempt == MAX_APPEND_ATTEMPTS => {
                        error!(%err, epoch = %state_cert.epoch, "failed to append state cert after {MAX_APPEND_ATTEMPTS} attempts, giving up");
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
            for attempt in 1..=MAX_APPEND_ATTEMPTS {
                match storage.append_proposal_wrapper(&signed).await {
                    Ok(()) => return,
                    Err(err) if attempt == MAX_APPEND_ATTEMPTS => {
                        error!(%err, "failed to append proposal after {MAX_APPEND_ATTEMPTS} attempts, giving up");
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

    pub fn gc(&mut self, view_number: ViewNumber) {
        // Reap tasks that have already completed.
        self.handles.retain(|_, handles| {
            handles.retain(|handle| !handle.is_finished());
            !handles.is_empty()
        });

        // Abort only tasks far below the GC view, as a backstop against leaks. Writes for
        // recently decided views are left running: the decide pipeline still needs to read
        // that data back from disk to build query-service decide events.
        let horizon = ViewNumber::new(view_number.saturating_sub(GC_ABORT_HORIZON));
        let keep = self.handles.split_off(&horizon);
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
