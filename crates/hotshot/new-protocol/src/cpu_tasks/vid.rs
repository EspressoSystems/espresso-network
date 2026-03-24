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
    task::{AbortHandle, JoinSet},
};

use crate::{
    coordinator::handle::CoordinatorHandle,
    events::{VidDisperseRequest, VidShareInput},
};

type VidDisperseResult<T> = Result<(ViewNumber, VidCommitment2, VidDisperse2<T>), ()>;

pub(crate) struct VidDisperseTask<T: NodeType> {
    calculations: BTreeMap<ViewNumber, AbortHandle>,
    epoch_membership_coordinator: EpochMembershipCoordinator<T>,
    tasks: JoinSet<VidDisperseResult<T>>,
}

impl<T: NodeType> VidDisperseTask<T> {
    pub fn new(epoch_membership_coordinator: EpochMembershipCoordinator<T>) -> Self {
        Self {
            calculations: BTreeMap::new(),
            epoch_membership_coordinator,
            tasks: JoinSet::new(),
        }
    }

    pub async fn request_vid_disperse(&mut self, vid_disperse_request: VidDisperseRequest<T>) {
        let view = vid_disperse_request.view;
        if self.calculations.contains_key(&view) {
            return;
        }
        let handle = self.tasks.spawn(Self::handle_vid_disperse_request(
            self.epoch_membership_coordinator.clone(),
            vid_disperse_request,
        ));
        self.calculations.insert(view, handle);
    }

    pub async fn next(
        &mut self,
    ) -> Option<Result<(ViewNumber, VidCommitment2, VidDisperse2<T>), ()>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => return Some(result),
                Some(Err(_)) => continue,
                None => return None,
            }
        }
    }

    async fn handle_vid_disperse_request(
        epoch_membership_coordinator: EpochMembershipCoordinator<T>,
        vid_disperse_request: VidDisperseRequest<T>,
    ) -> Result<(ViewNumber, VidCommitment2, VidDisperse2<T>), ()> {
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
            return Err(());
        };
        Ok((
            vid_disperse_request.view,
            disperse.payload_commitment,
            disperse,
        ))
    }
}

pub(crate) struct VidShareAccumulator<T: NodeType> {
    shares: Vec<AvidmGf2Share>,
    accumulated_weight: usize,
    seen_keys: HashSet<T::SignatureKey>,
    common: AvidmGf2Common,
    metadata: Option<<T::BlockPayload as BlockPayload<T>>::Metadata>,
}

impl<T: NodeType> VidShareAccumulator<T> {
    fn has_enough_shares(&self) -> bool {
        self.accumulated_weight >= self.common.param.recovery_threshold
    }
}

pub(super) struct VidShareTask<T: NodeType> {
    accumulators: BTreeMap<ViewNumber, VidShareAccumulator<T>>,
    reconstructed: BTreeSet<ViewNumber>,
    rx: mpsc::Receiver<VidShareInput<T>>,
    coordinator_handle: CoordinatorHandle<T>,
    internal_tx: mpsc::Sender<(ViewNumber, VidCommitment2, T::BlockPayload)>,
    internal_rx: mpsc::Receiver<(ViewNumber, VidCommitment2, T::BlockPayload)>,
}

impl<T: NodeType> VidShareTask<T> {
    pub fn new(
        rx: mpsc::Receiver<VidShareInput<T>>,
        coordinator_handle: CoordinatorHandle<T>,
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
            let payload = T::BlockPayload::from_bytes(&bytes, &metadata);
            let _ = tx.send((view, payload_commitment, payload)).await;
        });
    }
}

// TODO: add tests for vid reconstruction where we receive duplicate shares, including
// the case where we receive identical shares from multiple keys
