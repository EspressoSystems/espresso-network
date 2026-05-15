use std::collections::{BTreeMap, BTreeSet, HashSet};

use committable::Commitment;
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{
        EpochNumber, VidCommitment, VidCommitment2, VidDisperse2, VidDisperseShare2, ViewNumber,
        ns_table::parse_ns_table, vid_disperse::vid_total_weight,
    },
    epoch_membership::EpochMembershipCoordinator,
    traits::{block_contents::EncodeBytes, node_implementation::NodeType},
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Scheme, AvidmGf2Share, init_avidm_gf2_param},
};
use tokio::task::{AbortHandle, JoinSet};
use tracing::warn;

use crate::message::BlockPushMessage;

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
    pub tx_commitments: Vec<Commitment<T::Transaction>>,
}

#[derive(Debug, thiserror::Error)]
pub enum VidReconstructError {
    /// `AvidmGf2::recover` (share-driven reconstruction) failed.
    #[error("share-based reconstruction failed for view {0}")]
    Reconstruct(ViewNumber),
    /// `verify_block` failed (commit mismatch / param init / commit error).
    #[error("block push verification failed for view {0}")]
    VerifyBlock(ViewNumber),
}

impl VidReconstructError {
    pub fn view(&self) -> ViewNumber {
        match self {
            Self::Reconstruct(v) | Self::VerifyBlock(v) => *v,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VidDisperseRequest<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub block: T::BlockPayload,
    pub metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
}

pub struct VidDisperser<T: NodeType> {
    calculations: BTreeMap<ViewNumber, AbortHandle>,
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
        let keep = self.calculations.split_off(&view_number);
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
    metadata: Option<<T::BlockPayload as BlockPayload<T>>::Metadata>,
    epoch: Option<EpochNumber>,
}

impl<T: NodeType> VidShareAccumulator<T> {
    fn has_enough_shares(&self) -> bool {
        self.accumulated_weight >= self.common.param.recovery_threshold
    }
}

#[derive(Default)]
pub struct VidReconstructor<T: NodeType> {
    accumulators: BTreeMap<ViewNumber, VidShareAccumulator<T>>,
    pub(crate) reconstructed: BTreeSet<ViewNumber>,
    tasks: JoinSet<Result<VidReconstructOutput<T>, VidReconstructError>>,
    calculations: BTreeMap<ViewNumber, AbortHandle>,
    block_verifications: BTreeMap<ViewNumber, AbortHandle>,
    /// Wall-clock unix nanoseconds at which `try_reconstruct` first fired for a view —
    /// i.e., when 2N/3+1 shares had been accumulated and `AvidmGf2::recover` was about
    /// to start. Subtract this from `block_reconstructed_ns` to isolate recover CPU time
    /// from share-collection time. Cleared by `gc`.
    threshold_reached_ns: BTreeMap<ViewNumber, i128>,
}

impl<T: NodeType> VidReconstructor<T> {
    pub fn new() -> Self {
        Self {
            accumulators: BTreeMap::new(),
            reconstructed: BTreeSet::new(),
            tasks: JoinSet::new(),
            calculations: BTreeMap::new(),
            block_verifications: BTreeMap::new(),
            threshold_reached_ns: BTreeMap::new(),
        }
    }

    /// Wall-clock unix nanoseconds at which the share threshold for `view` was first
    /// reached, if it has been reached. Returns `None` for views that haven't yet
    /// triggered `try_reconstruct`.
    pub fn threshold_reached_ns(&self, view: ViewNumber) -> Option<i128> {
        self.threshold_reached_ns.get(&view).copied()
    }

    pub(crate) fn handle_vid_share<M>(&mut self, share: VidDisperseShare2<T>, metadata: M)
    where
        M: Into<Option<<T::BlockPayload as BlockPayload<T>>::Metadata>>,
    {
        let view = share.view_number;
        if self.reconstructed.contains(&view) {
            return;
        }
        let payload_commitment = share.payload_commitment;
        let recipient_key = share.recipient_key.clone();
        let weight = share.share.weight();
        let metadata = metadata.into();
        let share_epoch = share.epoch;
        let accumulator = self
            .accumulators
            .entry(view)
            .or_insert_with(|| VidShareAccumulator {
                shares: Vec::new(),
                accumulated_weight: 0,
                seen_keys: HashSet::new(),
                common: share.common.clone(),
                metadata: None,
                epoch: share_epoch,
            });
        if accumulator.metadata.is_none()
            && let Some(m) = metadata
        {
            accumulator.metadata = Some(m)
        }
        if accumulator.seen_keys.insert(recipient_key) {
            accumulator.accumulated_weight += weight;
            accumulator.shares.push(share.share);
        }
        if accumulator.has_enough_shares() {
            self.try_reconstruct(view, payload_commitment);
        }
    }

    pub async fn next(&mut self) -> Option<Result<VidReconstructOutput<T>, VidReconstructError>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(Ok(out))) => {
                    let view = out.view;
                    if self.reconstructed.contains(&view) {
                        self.calculations.remove(&view);
                        self.block_verifications.remove(&view);
                        continue;
                    }
                    if let Some(h) = self.calculations.remove(&view) {
                        h.abort();
                    }
                    if let Some(h) = self.block_verifications.remove(&view) {
                        h.abort();
                    }
                    self.accumulators.remove(&view);
                    self.reconstructed.insert(view);
                    return Some(Ok(out));
                },
                Some(Ok(Err(err))) => {
                    match &err {
                        VidReconstructError::Reconstruct(view) => {
                            self.calculations.remove(view);
                        },
                        VidReconstructError::VerifyBlock(view) => {
                            self.block_verifications.remove(view);
                        },
                    }
                    return Some(Err(err));
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
        // Metadata comes from when we get the proposal, otherwise we can't reconstruct the payload
        let Some(metadata) = accumulator.metadata.clone() else {
            return;
        };
        let epoch = accumulator.epoch.unwrap_or(EpochNumber::genesis());
        // Record the moment we have enough shares to start recovery. Done before
        // `spawn_blocking` so the timestamp captures the actual handoff to the
        // CPU-bound recover task, not the recover finish time.
        self.threshold_reached_ns.entry(view).or_insert_with(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as i128)
                .unwrap_or(0)
        });
        let task = self.tasks.spawn_blocking(move || {
            let Ok(result) = AvidmGf2Scheme::recover(&common, &shares) else {
                return Err(VidReconstructError::Reconstruct(view));
            };
            let payload = T::BlockPayload::from_bytes(&result, &metadata);
            let tx_commitments = payload.transaction_commitments(&metadata);
            Ok(VidReconstructOutput {
                view,
                epoch,
                payload_commitment,
                payload,
                metadata,
                tx_commitments,
            })
        });
        self.calculations.insert(view, task);
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        let keep = self.calculations.split_off(&view_number);
        for handle in self.calculations.values_mut() {
            handle.abort();
        }
        self.calculations = keep;
        let keep = self.block_verifications.split_off(&view_number);
        for handle in self.block_verifications.values_mut() {
            handle.abort();
        }
        self.block_verifications = keep;
        self.accumulators = self.accumulators.split_off(&view_number);
        self.threshold_reached_ns = self.threshold_reached_ns.split_off(&view_number);
    }

    /// Verify a `BlockPushMessage` off-thread.
    pub async fn spawn_verify_block_task(
        &mut self,
        block: BlockPushMessage<T>,
        membership_coordinator: &EpochMembershipCoordinator<T>,
    ) {
        let view = block.view;
        if self.reconstructed.contains(&view) || self.block_verifications.contains_key(&view) {
            return;
        }
        let VidCommitment::V2(expected_v2) = block.payload_commitment else {
            warn!(%view, "block push has non-V2 commit; dropping");
            return;
        };
        let Ok(membership) = membership_coordinator.stake_table_for_epoch(Some(block.epoch)) else {
            warn!(%view, epoch = %block.epoch, "block push verify: stake table unavailable");
            return;
        };
        let total_weight = vid_total_weight(membership.stake_table(), Some(block.epoch));
        let epoch = block.epoch;
        let task = self.tasks.spawn_blocking(move || {
            let payload_bytes = block.payload.encode();
            let metadata_bytes = block.metadata.encode();
            let Ok(param) = init_avidm_gf2_param(total_weight) else {
                warn!(%view, "block push verify: failed to init avidm param");
                return Err(VidReconstructError::VerifyBlock(view));
            };
            let ns_table = parse_ns_table(payload_bytes.len(), metadata_bytes.as_ref());
            let Ok((computed, _common)) =
                AvidmGf2Scheme::commit(&param, payload_bytes.as_ref(), ns_table)
            else {
                warn!(%view, "block push verify: AvidmGf2Scheme::commit failed");
                return Err(VidReconstructError::VerifyBlock(view));
            };
            if computed != expected_v2 {
                warn!(%view, "block push commit mismatch; falling back to share-based recover");
                return Err(VidReconstructError::VerifyBlock(view));
            }
            let tx_commitments = block.payload.transaction_commitments(&block.metadata);
            Ok(VidReconstructOutput {
                view,
                epoch,
                payload_commitment: expected_v2,
                payload: block.payload,
                metadata: block.metadata,
                tx_commitments,
            })
        });
        self.block_verifications.insert(view, task);
    }
}
