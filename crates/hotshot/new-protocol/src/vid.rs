use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

use committable::Commitment;
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{VidCommitment2, VidDisperse2, VidDisperseShare2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::node_implementation::NodeType,
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
    pub payload_commitment: VidCommitment2,
    pub payload: T::BlockPayload,
    pub metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    pub tx_commitments: Vec<Commitment<T::Transaction>>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VidDisperseRequest<T: NodeType> {
    pub view: ViewNumber,
    pub vid_disperse: VidDisperse2<T>,
}

/// Accepts pre-computed VID disperse data and returns it as output.
///
/// The actual VID computation is performed by the [`BlockBuilder`] so
/// this component simply queues the result for the coordinator to pick up.
pub struct VidDisperser<T: NodeType> {
    pending: VecDeque<VidDisperseOutput<T>>,
}

impl<T: NodeType> VidDisperser<T> {
    pub fn new(_epoch_membership_coordinator: EpochMembershipCoordinator<T>) -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }

    pub fn request_vid_disperse(&mut self, req: VidDisperseRequest<T>) {
        self.pending.push_back(VidDisperseOutput {
            view: req.view,
            payload_commitment: req.vid_disperse.payload_commitment,
            disperse: req.vid_disperse,
        });
    }

    /// Returns the next queued VID disperse result.
    pub async fn next(&mut self) -> Option<Result<VidDisperseOutput<T>, ()>> {
        if self.pending.is_empty() {
            std::future::pending::<()>().await;
        }
        self.pending.pop_front().map(Ok)
    }

    pub fn gc(&mut self, _view_number: ViewNumber) {}
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
        let accumulator = self
            .accumulators
            .entry(view)
            .or_insert_with(|| VidShareAccumulator {
                shares: Vec::new(),
                accumulated_weight: 0,
                seen_keys: HashSet::new(),
                common: share.common.clone(),
                metadata: None,
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
        // Metadata comes from when we get the proposal, otherwise we can't reconstruct the payload
        let Some(metadata) = accumulator.metadata.clone() else {
            return;
        };
        let task = self.tasks.spawn_blocking(move || {
            let Ok(result) = AvidmGf2Scheme::recover(&common, &shares) else {
                // TODO: Handle error
                return Err(());
            };
            let payload = T::BlockPayload::from_bytes(&result, &metadata);
            let tx_commitments = payload.transaction_commitments(&metadata);
            Ok(VidReconstructOutput {
                view,
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
    }
}
