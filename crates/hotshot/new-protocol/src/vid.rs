use std::collections::{BTreeMap, BTreeSet, HashSet};

use committable::Commitment;
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{EpochNumber, VidCommitment2, VidDisperse2, VidDisperseShare2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Scheme, AvidmGf2Share},
};
use tokio::task::{AbortHandle, JoinSet};

pub struct VidDisperseOutput<T: NodeType> {
    pub view: ViewNumber,
    pub payload_commitment: VidCommitment2,
    pub disperse: VidDisperse2<T>,
}

pub struct VidReconstructOutput<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub payload_commitment: VidCommitment2,
    pub payload: T::BlockPayload,
    pub metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    /// Header of the block this payload belongs to, captured from the proposal. Carried
    /// through reconstruction so consumers don't depend on the proposal still being in
    /// consensus state (it may have been garbage collected by the time we finish).
    pub header: T::BlockHeader,
    pub tx_commitments: Vec<Commitment<T::Transaction>>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VidDisperseRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub block: T::BlockPayload,
    pub metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    pub payload_commitment: VidCommitment2,
}

pub struct VidDisperser<T: NodeType> {
    calculations: BTreeMap<(ViewNumber, VidCommitment2), AbortHandle>,
    epoch_membership_coordinator: EpochMembershipCoordinator<T>,
    tasks: JoinSet<Result<VidDisperseOutput<T>, ()>>,
}

impl<T: NodeType> VidDisperser<T> {
    pub fn new(epoch_membership_coordinator: EpochMembershipCoordinator<T>) -> Self {
        Self {
            calculations: BTreeMap::new(),
            epoch_membership_coordinator,
            tasks: JoinSet::new(),
        }
    }

    pub fn request_vid_disperse(&mut self, vid_disperse_request: VidDisperseRequest<T>) {
        let key = (
            vid_disperse_request.view,
            vid_disperse_request.payload_commitment,
        );
        if self.calculations.contains_key(&key) {
            return;
        }
        let handle = self.tasks.spawn(Self::handle_vid_disperse_request(
            self.epoch_membership_coordinator.clone(),
            vid_disperse_request,
        ));
        self.calculations.insert(key, handle);
    }

    pub async fn next(&mut self) -> Option<Result<VidDisperseOutput<T>, ()>> {
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
    ) -> Result<VidDisperseOutput<T>, ()> {
        let Ok((disperse, _duration)) = VidDisperse2::calculate_vid_disperse(
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
        Ok(VidDisperseOutput {
            view: vid_disperse_request.view,
            payload_commitment: disperse.payload_commitment,
            disperse,
        })
    }
    pub fn gc(&mut self, view_number: ViewNumber) {
        let keep = self
            .calculations
            .split_off(&(view_number, VidCommitment2::default()));
        for handle in self.calculations.values_mut() {
            handle.abort();
        }
        self.calculations = keep;
    }
}

pub(crate) struct VidShareAccumulator<T: NodeType> {
    shares: Vec<AvidmGf2Share>,
    accumulated_weight: usize,
    seen_keys: HashSet<T::SignatureKey>,
    common: AvidmGf2Common,
    /// Block header from the proposal for this view. Required for reconstruction (it
    /// provides the payload metadata) and carried into the output for consumers.
    header: Option<T::BlockHeader>,
    epoch: Option<EpochNumber>,
}

impl<T: NodeType> VidShareAccumulator<T> {
    fn has_enough_shares(&self) -> bool {
        self.accumulated_weight >= self.common.param.recovery_threshold
    }
}

/// Number of views below the GC view for which in-flight reconstructions and share
/// accumulators are kept alive, so that payloads for just-decided views can still be
/// reconstructed and delivered to the decide pipeline / query service.
pub(crate) const RECONSTRUCT_KEEP_HORIZON: u64 = 5;

#[derive(Default)]
pub struct VidReconstructor<T: NodeType> {
    accumulators: BTreeMap<ViewNumber, VidShareAccumulator<T>>,
    reconstructed: BTreeSet<ViewNumber>,
    tasks: JoinSet<Result<VidReconstructOutput<T>, ()>>,
    calculations: BTreeMap<ViewNumber, AbortHandle>,
}

impl<T: NodeType> VidReconstructor<T> {
    pub fn new() -> Self {
        Self {
            accumulators: BTreeMap::new(),
            reconstructed: BTreeSet::new(),
            tasks: JoinSet::new(),
            calculations: BTreeMap::new(),
        }
    }

    pub(crate) fn handle_vid_share<H>(&mut self, share: VidDisperseShare2<T>, header: H)
    where
        H: Into<Option<T::BlockHeader>>,
    {
        let view = share.view_number;
        if self.reconstructed.contains(&view) {
            return;
        }
        let payload_commitment = share.payload_commitment;
        let recipient_key = share.recipient_key.clone();
        let weight = share.share.weight();
        let header = header.into();
        let share_epoch = share.epoch;
        let accumulator = self
            .accumulators
            .entry(view)
            .or_insert_with(|| VidShareAccumulator {
                shares: Vec::new(),
                accumulated_weight: 0,
                seen_keys: HashSet::new(),
                common: share.common.clone(),
                header: None,
                epoch: share_epoch,
            });
        if accumulator.header.is_none()
            && let Some(h) = header
        {
            accumulator.header = Some(h)
        }
        if accumulator.seen_keys.insert(recipient_key) {
            accumulator.accumulated_weight += weight;
            accumulator.shares.push(share.share);
        }
        if accumulator.has_enough_shares() {
            self.try_reconstruct(view, payload_commitment);
        }
    }

    pub async fn next(&mut self) -> Option<Result<VidReconstructOutput<T>, ()>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(Ok(out))) => {
                    self.calculations.remove(&out.view);
                    self.accumulators.remove(&out.view);
                    self.reconstructed.insert(out.view);
                    return Some(Ok(out));
                },
                Some(Ok(Err(()))) => {
                    // TODO: Handle error
                    return Some(Err(()));
                },
                Some(Err(_)) => continue,
                None => return None,
            }
        }
    }

    fn try_reconstruct(&mut self, view: ViewNumber, payload_commitment: VidCommitment2) {
        if self.calculations.contains_key(&view) {
            return;
        }
        let Some(accumulator) = self.accumulators.get(&view) else {
            return;
        };
        let shares = accumulator.shares.clone();
        let common = accumulator.common.clone();
        // The header comes from the proposal; without it we have no payload metadata and
        // can't reconstruct the payload.
        let Some(header) = accumulator.header.clone() else {
            return;
        };
        let epoch = accumulator.epoch.unwrap_or(EpochNumber::genesis());
        let task = self.tasks.spawn_blocking(move || {
            let Ok(result) = AvidmGf2Scheme::recover(&common, &shares) else {
                // TODO: Handle error
                return Err(());
            };
            let metadata = header.metadata().clone();
            let payload = T::BlockPayload::from_bytes(&result, &metadata);
            let tx_commitments = payload.transaction_commitments(&metadata);
            Ok(VidReconstructOutput {
                view,
                epoch,
                payload_commitment,
                payload,
                metadata,
                header,
                tx_commitments,
            })
        });
        self.calculations.insert(view, task);
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        // GC runs when views are decided, but the decided views' payloads are exactly what
        // the decide pipeline still needs: a multi-leaf decide (e.g. after a timeout)
        // would otherwise abort the reconstructions for the older leaves in the batch and
        // lose their payloads. Keep a small horizon of views alive below the GC view; far
        // below it, accumulators can no longer make progress anyway (Vote1 messages
        // carrying shares stop arriving once the network moves on).
        let horizon = ViewNumber::new(view_number.saturating_sub(RECONSTRUCT_KEEP_HORIZON));
        let keep = self.calculations.split_off(&horizon);
        for handle in self.calculations.values_mut() {
            handle.abort();
        }
        self.calculations = keep;
        self.accumulators = self.accumulators.split_off(&horizon);
        // Forget completed views below the horizon; their accumulators are gone, so late
        // shares can no longer trigger duplicate reconstructions.
        self.reconstructed = self.reconstructed.split_off(&horizon);
    }

    /// Mark `view` as already-reconstructed: drop accumulated shares, abort any
    /// in-flight reconstruction task, and ignore later shares for this view.
    pub fn mark_reconstructed(&mut self, view: ViewNumber) {
        self.reconstructed.insert(view);
        self.accumulators.remove(&view);
        if let Some(handle) = self.calculations.remove(&view) {
            handle.abort();
        }
    }
}
