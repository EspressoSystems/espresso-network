use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Instant,
};

use async_broadcast::{Receiver, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::{OuterConsensus, PayloadWithMetadata},
    data::{QuorumProposal2, VidCommitment, VidDisperseShare, VidDisperseShare2},
    epoch_membership::EpochMembershipCoordinator,
    simple_vote::HasEpoch,
    traits::{
        block_contents::BlockHeader,
        node_implementation::{NodeImplementation, NodeType},
        signature_key::SignatureKey,
        BlockPayload,
    },
    vid::avidm_gf2::AvidmGf2Scheme,
    vote::HasViewNumber,
};
use tokio::{spawn, sync::mpsc};

use crate::{
    events::{HotShotEvent, HotShotTaskCompleted},
    helpers::broadcast_event,
};

pub struct ReconstructTaskState<TYPES: NodeType> {
    pub id: u64,
    pub event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    pub consensus: OuterConsensus<TYPES>,
    pub calc_lock: Arc<RwLock<HashMap<TYPES::View, mpsc::Sender<()>>>>,
    pub proposals: BTreeMap<TYPES::View, QuorumProposal2<TYPES>>,
    pub vid_shares:
        Arc<RwLock<BTreeMap<(TYPES::View, TYPES::Epoch), Vec<VidDisperseShare2<TYPES>>>>>,
}

async fn try_reconstruct_block<TYPES: NodeType>(
    id: u64,
    calc_lock: Arc<RwLock<HashMap<TYPES::View, mpsc::Sender<()>>>>,
    consensus: OuterConsensus<TYPES>,
    view: TYPES::View,
    epoch: Option<TYPES::Epoch>,
    vid_shares: Arc<RwLock<BTreeMap<(TYPES::View, TYPES::Epoch), Vec<VidDisperseShare2<TYPES>>>>>,
    event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    mut signal_rx: mpsc::Receiver<()>,
) -> Option<()> {
    loop {
        let Some(()) = signal_rx.recv().await else {
            tracing::error!("Signal received, stopping reconstruction task for view {view}");
            break;
        };
        if consensus.read().await.saved_payloads().contains_key(&view) {
            if *view == 4 {
                tracing::error!(
                    "We already have the payload for view {view}, skipping reconstruction"
                );
            }

            return None;
        }
        if *view == 4 {
            tracing::error!("No payload found for view {view}, trying to reconstruct");
        }
        let Some(shares) = vid_shares.read().await.get(&(view, epoch?)).cloned() else {
            tracing::error!("No shares found for view {view}, skipping reconstruction");
            continue;
        };
        if shares.is_empty() {
            tracing::error!("No shares found for view {view}, skipping reconstruction");
            continue;
        }

        let (common, vid_commitment) = {
            let first_share = shares.first()?;
            (
                first_share.common.clone(),
                first_share.payload_commitment.clone(),
            )
        };

        let now = Instant::now();
        let reconstruct_result = tokio::task::spawn_blocking(move || {
            AvidmGf2Scheme::recover(
                &common,
                &shares.iter().map(|s| s.share.clone()).collect::<Vec<_>>(),
            )
        })
        .await
        .inspect_err(|e| {
            tracing::error!(error=?e, "spawn blocking failed for view {view}");
        })
        .ok()?;

        let payload_bytes = match reconstruct_result {
            Ok(payload_bytes) => payload_bytes,
            Err(e) => {
                if *view == 4 {
                    tracing::error!("Failed to reconstruct block for view {view}: {e}");
                }
                tracing::debug!(error=?e, "Failed to reconstruct block for view {view}");
                continue;
            },
        };

        let payload = TYPES::BlockPayload::from_bytes(&payload_bytes, &metadata);
        let elapsed = now.elapsed();
        tracing::error!("Reconstructed block for view {view} in {elapsed:?}");
        broadcast_event(
            Arc::new(HotShotEvent::BlockReconstructed(
                payload.clone(),
                metadata.clone(),
                VidCommitment::V2(vid_commitment),
                view,
            )),
            &event_stream,
        )
        .await;
        let _ = consensus
            .write()
            .await
            .update_saved_payloads(
                view,
                Arc::new(PayloadWithMetadata {
                    payload,
                    metadata: metadata.clone(),
                }),
            )
            .inspect_err(|_| tracing::error!("Failed to update saved payloads for view {view}"));
        calc_lock.write().await.remove(&view);
    }
    Some(())
}

impl<TYPES: NodeType> ReconstructTaskState<TYPES> {
    async fn spawn_reconstruct_task(
        &mut self,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
        metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) {
        // if self.id == 2 {
        //     tracing::error!("Spawning reconstruct task for view {} with epoch {}", view, epoch.unwrap());
        // }
        let tx = self.calc_lock.read().await.get(&view).cloned();
        let tx = match tx {
            Some(tx) => tx,
            None => {
                let (tx, rx) = mpsc::channel(100);
                self.calc_lock.write().await.insert(view, tx.clone());
                spawn(try_reconstruct_block(
                    self.id,
                    self.calc_lock.clone(),
                    self.consensus.clone(),
                    view,
                    epoch,
                    self.vid_shares.clone(),
                    self.event_stream.clone(),
                    metadata,
                    rx,
                ));
                tx
            },
        };
        let _ = tx.send(()).await;
    }
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
    ) -> Option<HotShotTaskCompleted> {
        match event.as_ref() {
            HotShotEvent::VidShareValidated(share) => {
                let VidDisperseShare::V2(ref share) = share.data else {
                    return None;
                };
                // if self.id == 2 {
                //     tracing::error!("Received vid share for view {} with epoch {}", share.view_number(), share.epoch().unwrap());
                // }
                let view = share.view_number();
                self.vid_shares
                    .write()
                    .await
                    .entry((view, share.epoch().unwrap()))
                    .or_default()
                    .push(share.clone());
                tracing::error!(
                    "Received vid share for view {} we now have {} shares",
                    view,
                    self.vid_shares
                        .read()
                        .await
                        .get(&(view, share.epoch().unwrap()))
                        .unwrap()
                        .len()
                );

                // if we already have the payload, no need to try to reconstruct it
                if self
                    .consensus
                    .read()
                    .await
                    .saved_payloads()
                    .contains_key(&view)
                {
                    // if self.id == 2 {
                    //     tracing::error!("We already have the payload for view {view}, skipping reconstruction");
                    // }
                    if *view == 4 {
                        tracing::error!(
                            "We already have the payload for view {view}, skipping reconstruction"
                        );
                    }
                    tracing::debug!(
                        "We already have the payload for view {view}, skipping reconstruction"
                    );
                    return None;
                }

                let proposal = self.proposals.get(&view).cloned()?;
                self.spawn_reconstruct_task(
                    view,
                    proposal.epoch(),
                    proposal.block_header.metadata().clone(),
                )
                .await;
            },
            HotShotEvent::QuorumProposalValidated(proposal, _) => {
                let view = proposal.data.view_number();
                self.proposals.insert(view, proposal.data.clone().into());
                // if self.id == 2 {
                //     tracing::error!("Received quorum proposal for view {} with epoch {}", view, proposal.data.epoch().unwrap());
                // }
                // if we already have the payload
                if self
                    .consensus
                    .read()
                    .await
                    .saved_payloads()
                    .contains_key(&view)
                {
                    tracing::debug!(
                        "We already have the payload for view {view}, skipping reconstruction"
                    );
                    return None;
                }
                self.spawn_reconstruct_task(
                    view,
                    proposal.data.epoch(),
                    proposal.data.block_header().metadata().clone(),
                )
                .await;
            },
            HotShotEvent::Shutdown => {
                return Some(HotShotTaskCompleted);
            },
            _ => {},
        }
        None
    }
}

#[async_trait]
impl<TYPES: NodeType> TaskState for ReconstructTaskState<TYPES> {
    type Event = HotShotEvent<TYPES>;

    async fn handle_event(
        &mut self,
        event: Arc<Self::Event>,
        _sender: &Sender<Arc<Self::Event>>,
        _receiver: &Receiver<Arc<Self::Event>>,
    ) -> hotshot_utils::anytrace::Result<()> {
        self.handle(event).await;
        Ok(())
    }

    fn cancel_subtasks(&mut self) {
        // No subtasks to cancel
    }
}
