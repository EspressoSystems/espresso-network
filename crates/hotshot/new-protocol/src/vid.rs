use std::collections::{BTreeMap, BTreeSet, HashSet};

use hotshot_types::{
    data::{EpochNumber, VidCommitment2, VidDisperse2, VidDisperseShare2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{BlockPayload, node_implementation::NodeType},
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Scheme, AvidmGf2Share},
};
use tokio::task::{AbortHandle, JoinSet};

type VidDisperseResult<T> = Result<(ViewNumber, VidCommitment2, VidDisperse2<T>), ()>;
type VidShareResult<T> = Result<(ViewNumber, VidCommitment2, <T as NodeType>::BlockPayload), ()>;

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
    tasks: JoinSet<VidDisperseResult<T>>,
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
        Ok((
            vid_disperse_request.view,
            disperse.payload_commitment,
            disperse,
        ))
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
    tasks: JoinSet<VidShareResult<T>>,
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

    pub async fn next(
        &mut self,
    ) -> Option<Result<(ViewNumber, VidCommitment2, <T as NodeType>::BlockPayload), ()>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => {
                    if let Ok((view, vid_commitment, payload)) = result {
                        self.calculations.remove(&view);
                        self.accumulators.remove(&view);
                        self.reconstructed.insert(view);
                        return Some(Ok((view, vid_commitment, payload)));
                    } else {
                        // TODO: Handle error
                        return Some(Err(()));
                    }
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
            Ok((view, payload_commitment, payload))
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
