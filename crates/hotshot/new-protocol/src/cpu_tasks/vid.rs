use std::collections::{BTreeMap, BTreeSet, HashSet};

use hotshot_types::{
    data::{VidCommitment2, VidDisperse2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{BlockPayload, node_implementation::NodeType},
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Scheme, AvidmGf2Share},
};
use tokio::{
    spawn,
    sync::mpsc::{self},
    task::JoinHandle,
};

use crate::{
    coordinator::handle::CoordinatorHandle,
    events::{VidDisperseRequest, VidShareInput},
};

pub(super) struct VidDisperseTask<TYPES: NodeType> {
    calculations: BTreeMap<ViewNumber, JoinHandle<()>>,
    epoch_membership_coordinator: EpochMembershipCoordinator<TYPES>,
    rx: mpsc::Receiver<VidDisperseRequest<TYPES>>,
    coordinator_handle: CoordinatorHandle<TYPES>,
    internal_tx: mpsc::Sender<(ViewNumber, VidCommitment2, VidDisperse2<TYPES>)>,
    internal_rx: mpsc::Receiver<(ViewNumber, VidCommitment2, VidDisperse2<TYPES>)>,
}

impl<TYPES: NodeType> VidDisperseTask<TYPES> {
    pub fn new(
        rx: mpsc::Receiver<VidDisperseRequest<TYPES>>,
        coordinator_handle: CoordinatorHandle<TYPES>,
        epoch_membership_coordinator: EpochMembershipCoordinator<TYPES>,
    ) -> Self {
        let (internal_tx, internal_rx) = mpsc::channel(100);
        Self {
            calculations: BTreeMap::new(),
            epoch_membership_coordinator,
            rx,
            coordinator_handle,
            internal_tx,
            internal_rx,
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(request) = self.rx.recv() => {
                if self.calculations.contains_key(&request.view) {
                    continue;
                }
                let view = request.view;
                let handle = self.handle_vid_disperse_request(request);
                self.calculations.insert(view, handle);
            },
            Some(response) = self.internal_rx.recv() => {
                let (view, payload_commitment, disperse) = response;
                self.calculations.remove(&view);
                let _ = self.coordinator_handle.respond_vid_disperse(payload_commitment, disperse).await;
            },
            else => break,
            }
        }
    }

    fn handle_vid_disperse_request(
        &self,
        vid_disperse_request: VidDisperseRequest<TYPES>,
    ) -> JoinHandle<()> {
        let tx = self.internal_tx.clone();
        let epoch_membership_coordinator = self.epoch_membership_coordinator.clone();
        spawn(async move {
            let Ok((disperse, duration)) = VidDisperse2::calculate_vid_disperse(
                &vid_disperse_request.block,
                &epoch_membership_coordinator,
                vid_disperse_request.view,
                Some(vid_disperse_request.epoch),
                Some(vid_disperse_request.epoch),
                &vid_disperse_request.metadata,
            )
            .await
            else {
                // TODO: Handle error
                return;
            };
            let _ = tx
                .send((
                    vid_disperse_request.view,
                    disperse.payload_commitment,
                    disperse,
                ))
                .await;
        })
    }
}

struct VidShareAccumulator<TYPES: NodeType> {
    shares: Vec<AvidmGf2Share>,
    accumulated_weight: usize,
    seen_keys: HashSet<TYPES::SignatureKey>,
    common: AvidmGf2Common,
    metadata: Option<<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata>,
}

impl<TYPES: NodeType> VidShareAccumulator<TYPES> {
    fn has_enough_shares(&self) -> bool {
        self.accumulated_weight >= self.common.param.recovery_threshold
    }
}

pub(super) struct VidShareTask<TYPES: NodeType> {
    accumulators: BTreeMap<ViewNumber, VidShareAccumulator<TYPES>>,
    reconstructed: BTreeSet<ViewNumber>,
    rx: mpsc::Receiver<VidShareInput<TYPES>>,
    coordinator_handle: CoordinatorHandle<TYPES>,
    internal_tx: mpsc::Sender<(ViewNumber, VidCommitment2, TYPES::BlockPayload)>,
    internal_rx: mpsc::Receiver<(ViewNumber, VidCommitment2, TYPES::BlockPayload)>,
}

impl<TYPES: NodeType> VidShareTask<TYPES> {
    pub fn new(
        rx: mpsc::Receiver<VidShareInput<TYPES>>,
        coordinator_handle: CoordinatorHandle<TYPES>,
    ) -> Self {
        let (internal_tx, internal_rx) = mpsc::channel(100);
        Self {
            accumulators: BTreeMap::new(),
            reconstructed: BTreeSet::new(),
            rx,
            coordinator_handle,
            internal_tx,
            internal_rx,
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                Some(input) = self.rx.recv() => {
                    let view = input.share.view_number;
                    if self.reconstructed.contains(&view) {
                        continue;
                    }
                    let payload_commitment = input.share.payload_commitment;
                    let recipient_key = input.share.recipient_key.clone();
                    let weight = input.share.share.weight();
                    let accumulator = self.accumulators.entry(view).or_insert_with(|| {
                        VidShareAccumulator {
                            shares: Vec::new(),
                            accumulated_weight: 0,
                            seen_keys: HashSet::new(),
                            common: input.share.common.clone(),
                            metadata: input.metadata.clone(),
                        }
                    });
                    if !accumulator.seen_keys.insert(recipient_key) {
                        // Already have a share from this key, skip duplicate
                        continue;
                    }
                    accumulator.accumulated_weight += weight;
                    accumulator.shares.push(input.share.share);
                    if accumulator.has_enough_shares() {
                        self.try_reconstruct(view, payload_commitment);
                    }
                },
                Some((view, vid_commitment, payload)) = self.internal_rx.recv() => {
                    self.accumulators.remove(&view);
                    self.reconstructed.insert(view);
                    let _ = self.coordinator_handle.respond_block_reconstructed(view, payload, vid_commitment).await;
                },
                else => break,
            }
        }
    }

    fn try_reconstruct(&self, view: ViewNumber, payload_commitment: VidCommitment2) {
        let Some(accumulator) = self.accumulators.get(&view) else {
            return;
        };
        let shares = accumulator.shares.clone();
        let common = accumulator.common.clone();
        // Metadata comes from when we get the proposal, otherwise we can't reconstruct the payload
        let Some(metadata) = accumulator.metadata.clone() else {
            return;
        };
        let tx = self.internal_tx.clone();
        spawn(async move {
            let result =
                tokio::task::spawn_blocking(move || AvidmGf2Scheme::recover(&common, &shares))
                    .await;
            let Ok(Ok(bytes)) = result else {
                return;
            };
            let payload = TYPES::BlockPayload::from_bytes(&bytes, &metadata);
            let _ = tx.send((view, payload_commitment, payload)).await;
        });
    }
}
