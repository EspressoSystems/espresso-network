use std::collections::{BTreeMap, BTreeSet, HashSet};

use committable::Commitment;
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{
        EpochNumber, VidCommitment2, VidDisperse2, VidDisperseShare2, ViewNumber,
        ns_table::parse_ns_table,
    },
    epoch_membership::EpochMembershipCoordinator,
    traits::{block_contents::EncodeBytes, node_implementation::NodeType},
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Scheme, AvidmGf2Share},
};
use tokio::task::{AbortHandle, JoinSet};
use tracing::warn;

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

/// A reconstruction attempt failed: either decoding errored or the recovered
/// payload did not re-commit to the expected payload commitment.
///
/// Carries the view so the reconstructor can drop its in-flight marker; the
/// accumulated shares are kept, so a later share triggers another attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("VID reconstruction failed for view {view}")]
pub struct VidReconstructError {
    pub view: ViewNumber,
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
    reconstructed: BTreeSet<ViewNumber>,
    tasks: JoinSet<Result<VidReconstructOutput<T>, VidReconstructError>>,
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
                    self.calculations.remove(&out.view);
                    self.accumulators.remove(&out.view);
                    self.reconstructed.insert(out.view);
                    return Some(Ok(out));
                },
                Some(Ok(Err(err))) => {
                    // Drop the in-flight marker but keep the accumulated
                    // shares: a later share triggers another attempt.
                    self.calculations.remove(&err.view);
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
        let task = self.tasks.spawn_blocking(move || {
            let bytes = AvidmGf2Scheme::recover(&common, &shares).map_err(|err| {
                warn!(%view, %err, "VID recovery failed");
                VidReconstructError { view }
            })?;
            // Recovery does not bind the decoded bytes to the commitment: a
            // Byzantine disperser can commit to a non-codeword, and a bad
            // share poisons the erasure decoding. Recommit and compare before
            // trusting the payload.
            let ns_table = parse_ns_table(bytes.len(), &metadata.encode());
            let (recomputed, _) =
                AvidmGf2Scheme::commit(&common.param, &bytes, ns_table).map_err(|err| {
                    warn!(%view, %err, "failed to recommit reconstructed VID payload");
                    VidReconstructError { view }
                })?;
            if recomputed != payload_commitment {
                warn!(
                    %view,
                    expected = %payload_commitment,
                    %recomputed,
                    "reconstructed payload does not match the payload commitment"
                );
                return Err(VidReconstructError { view });
            }
            let payload = T::BlockPayload::from_bytes(&bytes, &metadata);
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
        self.accumulators = self.accumulators.split_off(&view_number);
        self.reconstructed = self.reconstructed.split_off(&view_number);
    }

    /// Stop tracking `view`.
    ///
    /// Either because its payload was reconstructed (or obtained elsewhere)
    /// or because it timed out and will never be decided: record it so
    /// `handle_vid_share` ignores later shares, drop its accumulator, and
    /// abort any in-flight reconstruction task.
    pub fn retire_view(&mut self, view: ViewNumber) {
        self.reconstructed.insert(view);
        self.accumulators.remove(&view);
        if let Some(handle) = self.calculations.remove(&view) {
            handle.abort();
        }
    }
}
