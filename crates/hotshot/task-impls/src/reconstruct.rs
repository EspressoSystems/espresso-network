use std::{collections::HashMap, sync::Arc, time::Instant};

use async_broadcast::{Receiver, Sender};
use async_lock::RwLock;
use async_trait::async_trait;
use hotshot_task::task::TaskState;
use hotshot_types::{
    consensus::{OuterConsensus, PayloadWithMetadata},
    data::{VidCommitment, VidDisperseShare},
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
    pub event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    pub consensus: OuterConsensus<TYPES>,
    pub calc_lock: Arc<RwLock<HashMap<TYPES::View, mpsc::Sender<()>>>>,
}

async fn try_reconstruct_block<TYPES: NodeType>(
    calc_lock: Arc<RwLock<HashMap<TYPES::View, mpsc::Sender<()>>>>,
    consensus: OuterConsensus<TYPES>,
    view: TYPES::View,
    epoch: Option<TYPES::Epoch>,
    event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    mut signal_rx: mpsc::Receiver<()>,
) -> Option<()> {
    loop {
        let Some(()) = signal_rx.recv().await else {
            break;
        };
        if consensus.read().await.saved_payloads().contains_key(&view) {
            tracing::debug!("We already have the payload for view {view}, skipping reconstruction");
            return None;
        }
        let shares = consensus.read().await.vid_shares().get(&view).cloned()?;
        if shares.is_empty() {
            continue;
        }

        let mut reconstruct_shares = vec![];
        let (common, vid_commitment) = {
            let first_share = shares.values().next()?;
            let VidDisperseShare::V2(ref share) = first_share.get(&epoch)?.data else {
                continue;
            };
            (share.common.clone(), share.payload_commitment.clone())
        };

        for share in shares.values() {
            let share = share.get(&epoch)?;
            let VidDisperseShare::V2(ref share) = share.data else {
                continue;
            };
            reconstruct_shares.push(share.share.clone());
        }

        let now = Instant::now();
        let reconstruct_result = tokio::task::spawn_blocking(move || {
            AvidmGf2Scheme::recover(&common, &reconstruct_shares)
        })
        .await
        .ok()?;

        let payload_bytes = match reconstruct_result {
            Ok(payload_bytes) => payload_bytes,
            Err(e) => {
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
        let tx = self.calc_lock.read().await.get(&view).cloned();
        let tx = match tx {
            Some(tx) => tx,
            None => {
                let (tx, rx) = mpsc::channel(100);
                self.calc_lock.write().await.insert(view, tx.clone());
                spawn(try_reconstruct_block(
                    self.calc_lock.clone(),
                    self.consensus.clone(),
                    view,
                    epoch,
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
                let view = share.view_number();
                // if we already have the payload, no need to try to reconstruct it
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

                let proposal = self
                    .consensus
                    .read()
                    .await
                    .last_proposals()
                    .get(&view)
                    .cloned()?;
                self.spawn_reconstruct_task(
                    view,
                    proposal.data.epoch(),
                    proposal.data.block_header().metadata().clone(),
                )
                .await;
            },
            HotShotEvent::QuorumProposalValidated(proposal, _) => {
                let view = proposal.data.view_number();
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
