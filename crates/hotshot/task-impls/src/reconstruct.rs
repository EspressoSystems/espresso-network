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
        node_implementation::{ConsensusTime, NodeType},
        BlockPayload,
    },
    vid::avidm_gf2::AvidmGf2Scheme,
    vote::HasViewNumber,
};
use tokio::{spawn, sync::mpsc};

use crate::{
    events::{HotShotEvent, HotShotTaskCompleted},
    helpers::{broadcast_event, validate_vid_share},
};

pub struct ReconstructTaskState<TYPES: NodeType> {
    pub id: u64,
    pub event_stream: Sender<Arc<HotShotEvent<TYPES>>>,
    pub consensus: OuterConsensus<TYPES>,
    pub membership: EpochMembershipCoordinator<TYPES>,
    pub public_key: TYPES::SignatureKey,
    pub calc_lock: Arc<RwLock<HashMap<TYPES::View, mpsc::Sender<()>>>>,
    pub proposals: BTreeMap<TYPES::View, QuorumProposal2<TYPES>>,
    #[allow(clippy::type_complexity)]
    pub vid_shares:
        Arc<RwLock<BTreeMap<(TYPES::View, TYPES::Epoch), Vec<VidDisperseShare2<TYPES>>>>>,
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
async fn try_reconstruct_block<TYPES: NodeType>(
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
            tracing::debug!("We already have the payload for view {view}, skipping reconstruction");

            return None;
        }
        tracing::debug!("No payload found for view {view}, trying to reconstruct");
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
            (first_share.common.clone(), first_share.payload_commitment)
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
        if let Some(epoch) = epoch {
            vid_shares.write().await.remove(&(view, epoch));
        }
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

    async fn handle_validated_share(
        &mut self,
        share: &VidDisperseShare2<TYPES>,
        view: TYPES::View,
    ) {
        self.vid_shares
            .write()
            .await
            .entry((view, share.epoch().unwrap()))
            .or_default()
            .push(share.clone());
        tracing::info!(
            "Received vid share for view {} we now have {} shares",
            view,
            self.vid_shares
                .read()
                .await
                .get(&(view, share.epoch().unwrap()))
                .unwrap()
                .len()
        );

        let Some(proposal) = self.proposals.get(&view).cloned() else {
            return;
        };
        self.spawn_reconstruct_task(
            view,
            proposal.epoch(),
            proposal.block_header.metadata().clone(),
        )
        .await;
    }
    async fn is_old_view(&self, view: TYPES::View) -> bool {
        let locked_view = self.consensus.read().await.locked_view();
        view < locked_view
    }
    pub async fn handle(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
    ) -> Option<HotShotTaskCompleted> {
        match event.as_ref() {
            HotShotEvent::VidShareRecv(sender, share) => {
                let view = share.data.view_number();
                if self.is_old_view(view).await {
                    return None;
                }
                if self
                    .consensus
                    .read()
                    .await
                    .saved_payloads()
                    .contains_key(&view)
                {
                    return None;
                }
                // Only handle non-self shares; self-shares are handled by the quorum vote task
                if *share.data.recipient_key() == self.public_key {
                    return None;
                }

                if let Err(e) = validate_vid_share::<TYPES>(
                    sender,
                    share,
                    &self.membership,
                    self.consensus.clone(),
                )
                .await
                {
                    tracing::warn!("VID share validation failed for view {view}: {e}");
                    return None;
                }

                self.consensus
                    .write()
                    .await
                    .update_vid_shares(view, share.clone());

                let VidDisperseShare::V2(ref inner_share) = share.data else {
                    return None;
                };
                self.handle_validated_share(inner_share, view).await;
            },
            HotShotEvent::VidShareValidated(share) => {
                let VidDisperseShare::V2(ref share) = share.data else {
                    return None;
                };
                let view = share.view_number();

                if self.is_old_view(view).await {
                    return None;
                }

                if self
                    .consensus
                    .read()
                    .await
                    .saved_payloads()
                    .contains_key(&view)
                {
                    tracing::debug!(
                        "We already have the payload for view {view}, dropping VID share"
                    );
                    return None;
                }

                self.handle_validated_share(share, view).await;
            },
            HotShotEvent::QuorumProposalValidated(proposal, _) => {
                let view = proposal.data.view_number();
                self.proposals.insert(view, proposal.data.clone().into());

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
            HotShotEvent::LeavesDecided(leaves) => {
                if let Some(max_view) = leaves.iter().map(|l| l.view_number()).max() {
                    let gc_view = TYPES::View::new(max_view.saturating_sub(1));
                    let mut shares = self.vid_shares.write().await;
                    let calc_lock_len = self.calc_lock.read().await.len();
                    let shares_total: usize = shares.values().map(|v| v.len()).sum();
                    tracing::warn!(
                        "reconstruct GC before: id={} gc_view={gc_view} vid_shares_keys={} \
                         vid_shares_total={shares_total} proposals={} calc_lock={calc_lock_len}",
                        self.id,
                        shares.len(),
                        self.proposals.len(),
                    );
                    *shares = shares.split_off(&(gc_view, TYPES::Epoch::genesis()));
                    self.proposals = self.proposals.split_off(&gc_view);
                    let shares_total_after: usize = shares.values().map(|v| v.len()).sum();
                    tracing::warn!(
                        "reconstruct GC after: id={} vid_shares_keys={} \
                         vid_shares_total={shares_total_after} proposals={}",
                        self.id,
                        shares.len(),
                        self.proposals.len(),
                    );
                }
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
