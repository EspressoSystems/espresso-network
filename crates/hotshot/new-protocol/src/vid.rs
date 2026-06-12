use std::collections::{BTreeMap, BTreeSet, HashSet, btree_map::Entry};

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

/// Why a reconstruction attempt failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum VidReconstructErrorKind {
    /// Decoding failed or the recovered payload did not re-commit to the
    /// expected commitment, and the shares that failed verification were
    /// weeded out. Reconstruction retries once enough verified weight
    /// accumulates.
    #[error("awaiting more shares after weeding out unverifiable ones")]
    AwaitingShares,
    /// Every share verified against the commitment and the decoded payload
    /// still does not re-commit to it: the disperser committed to a
    /// non-codeword, so no subset of shares can ever recover this payload.
    #[error("unrecoverable: verified shares cannot decode to a payload matching the commitment")]
    Unrecoverable,
}

/// A failed reconstruction attempt for one view and claimed commitment.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("VID reconstruction failed for view {view}: {kind}")]
pub struct VidReconstructError<K> {
    pub view: ViewNumber,
    pub payload_commitment: VidCommitment2,
    pub kind: VidReconstructErrorKind,
    /// Voters whose shares failed verification against the commitment.
    /// Provably bad and attributable: each share arrived in a message
    /// signed by its voter.
    pub bad_share_keys: Vec<K>,
}

type Metadata<T> = <<T as NodeType>::BlockPayload as BlockPayload<T>>::Metadata;
type ReconstructResult<T> =
    Result<VidReconstructOutput<T>, VidReconstructError<<T as NodeType>::SignatureKey>>;

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
    shares: BTreeMap<T::SignatureKey, AvidmGf2Share>,
    accumulated_weight: usize,
    /// Every key that ever contributed, including weeded ones: a voter whose
    /// share failed verification doesn't get a second submission.
    seen_keys: HashSet<T::SignatureKey>,
    common: AvidmGf2Common,
    metadata: Option<Metadata<T>>,
    epoch: Option<EpochNumber>,
    /// Set when a fully verified share set failed to decode to a payload
    /// matching the commitment: the disperser is provably faulty and further
    /// attempts are pointless.
    exhausted: bool,
}

impl<T: NodeType> VidShareAccumulator<T> {
    fn has_enough_shares(&self) -> bool {
        self.accumulated_weight >= self.common.param.recovery_threshold
    }
}

#[derive(Default)]
pub struct VidReconstructor<T: NodeType> {
    accumulators: BTreeMap<(ViewNumber, VidCommitment2), VidShareAccumulator<T>>,
    reconstructed: BTreeSet<ViewNumber>,
    tasks: JoinSet<ReconstructResult<T>>,
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
        M: Into<Option<Metadata<T>>>,
    {
        let view = share.view_number;
        if self.reconstructed.contains(&view) {
            return;
        }
        let payload_commitment = share.payload_commitment;
        let accumulator = match self.accumulators.entry((view, payload_commitment)) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                // The commitment hash-binds the common data; check that
                // before trusting the common as the verification oracle
                // used for weeding out bad shares.
                if !AvidmGf2Scheme::is_consistent(&payload_commitment, &share.common) {
                    warn!(
                        %view,
                        sender = ?share.recipient_key,
                        "VID share common is inconsistent with its commitment"
                    );
                    return;
                }
                entry.insert(VidShareAccumulator {
                    shares: BTreeMap::new(),
                    accumulated_weight: 0,
                    seen_keys: HashSet::new(),
                    common: share.common.clone(),
                    metadata: None,
                    epoch: share.epoch,
                    exhausted: false,
                })
            },
        };
        if accumulator.exhausted {
            return;
        }
        // `is_consistent` pins everything the commitment hashes; comparing
        // with the accumulator's common catches shares smuggling a different
        // param alongside the same commitment.
        if share.common != accumulator.common {
            warn!(
                %view,
                sender = ?share.recipient_key,
                "VID share common differs from the accumulator's"
            );
            return;
        }
        // A share whose namespaces disagree on the shard range is malformed;
        // its weight is meaningless.
        if share.share.range().is_none() {
            warn!(
                %view,
                sender = ?share.recipient_key,
                "VID share has an inconsistent shard range"
            );
            return;
        }
        if accumulator.metadata.is_none()
            && let Some(m) = metadata.into()
        {
            accumulator.metadata = Some(m)
        }
        if accumulator.seen_keys.insert(share.recipient_key.clone()) {
            accumulator.accumulated_weight += share.share.weight();
            accumulator.shares.insert(share.recipient_key, share.share);
        }
        if accumulator.has_enough_shares() {
            self.try_reconstruct(view, payload_commitment);
        }
    }

    pub async fn next(&mut self) -> Option<ReconstructResult<T>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(Ok(out))) => {
                    self.calculations.remove(&out.view);
                    self.accumulators.retain(|(view, _), _| *view != out.view);
                    self.reconstructed.insert(out.view);
                    return Some(Ok(out));
                },
                Some(Ok(Err(err))) => {
                    self.calculations.remove(&err.view);
                    self.handle_failed_attempt(&err);
                    return Some(Err(err));
                },
                Some(Err(_)) => continue,
                None => return None,
            }
        }
    }

    /// Apply the outcome of a failed attempt: remove the weeded shares and
    /// either retry right away (shares that arrived while the attempt ran
    /// may already provide enough verified weight) or mark the payload as
    /// unrecoverable.
    fn handle_failed_attempt(&mut self, err: &VidReconstructError<T::SignatureKey>) {
        let mut retry = false;
        if let Some(accumulator) = self
            .accumulators
            .get_mut(&(err.view, err.payload_commitment))
        {
            for key in &err.bad_share_keys {
                if let Some(share) = accumulator.shares.remove(key) {
                    accumulator.accumulated_weight -= share.weight();
                }
            }
            match err.kind {
                VidReconstructErrorKind::Unrecoverable => accumulator.exhausted = true,
                VidReconstructErrorKind::AwaitingShares => {
                    // Only retry when weeding changed the share set, else an
                    // attempt over the same shares would fail the same way in
                    // a loop. If shares arrived while the attempt ran, the
                    // next arriving share triggers the retry instead.
                    retry = !err.bad_share_keys.is_empty() && accumulator.has_enough_shares();
                },
            }
        }
        if retry {
            self.try_reconstruct(err.view, err.payload_commitment);
        }
    }

    fn try_reconstruct(&mut self, view: ViewNumber, payload_commitment: VidCommitment2) {
        if self.calculations.contains_key(&view) {
            return;
        }
        let Some(accumulator) = self.accumulators.get(&(view, payload_commitment)) else {
            return;
        };
        if accumulator.exhausted {
            return;
        }
        // Metadata comes from when we get the proposal, otherwise we can't reconstruct the payload
        let Some(metadata) = accumulator.metadata.clone() else {
            return;
        };
        let shares: Vec<(T::SignatureKey, AvidmGf2Share)> = accumulator
            .shares
            .iter()
            .map(|(key, share)| (key.clone(), share.clone()))
            .collect();
        let common = accumulator.common.clone();
        let epoch = accumulator.epoch.unwrap_or(EpochNumber::genesis());
        let task = self.tasks.spawn_blocking(move || {
            Self::reconstruct(view, epoch, payload_commitment, common, shares, metadata)
        });
        self.calculations.insert(view, task);
    }

    /// Decode the shares and accept the result only if it re-commits to
    /// `payload_commitment`. On failure, weed out the shares that fail
    /// verification against the commitment (each share is self-authenticating
    /// via its merkle proofs) and retry once with the verified remainder.
    fn reconstruct(
        view: ViewNumber,
        epoch: EpochNumber,
        payload_commitment: VidCommitment2,
        common: AvidmGf2Common,
        shares: Vec<(T::SignatureKey, AvidmGf2Share)>,
        metadata: Metadata<T>,
    ) -> ReconstructResult<T> {
        let decode_set = Self::dedup_by_range(view, &shares);
        if let Some(bytes) =
            Self::decode_and_recommit(view, &common, &decode_set, &payload_commitment, &metadata)
        {
            return Ok(Self::output(
                view,
                epoch,
                payload_commitment,
                &bytes,
                metadata,
            ));
        }
        let (good, bad): (Vec<_>, Vec<_>) = shares.into_iter().partition(|(_, share)| {
            matches!(
                AvidmGf2Scheme::verify_share_with_verified_common(&common, share),
                Ok(Ok(()))
            )
        });
        let bad_share_keys: Vec<_> = bad.into_iter().map(|(key, _)| key).collect();
        if !bad_share_keys.is_empty() {
            warn!(
                %view,
                %payload_commitment,
                ?bad_share_keys,
                "weeded out VID shares that failed verification"
            );
        }
        // Weight that provably backs this commitment: verified shares,
        // counting overlapping (e.g. replayed) shard ranges once.
        let verified_set = Self::dedup_by_range(view, &good);
        let verified_weight: usize = verified_set.iter().map(AvidmGf2Share::weight).sum();
        let kind = if verified_weight < common.param.recovery_threshold {
            VidReconstructErrorKind::AwaitingShares
        } else {
            if !bad_share_keys.is_empty()
                && let Some(bytes) = Self::decode_and_recommit(
                    view,
                    &common,
                    &verified_set,
                    &payload_commitment,
                    &metadata,
                )
            {
                return Ok(Self::output(
                    view,
                    epoch,
                    payload_commitment,
                    &bytes,
                    metadata,
                ));
            }
            // A deduplicated, fully verified set with enough weight cannot
            // decode to a payload matching the commitment: the disperser
            // committed to a non-codeword, no share subset can ever succeed.
            // (With nothing weeded, the failed first attempt already used
            // exactly this set.)
            warn!(
                %view,
                %payload_commitment,
                "verified shares cannot decode to a payload matching the commitment"
            );
            VidReconstructErrorKind::Unrecoverable
        };
        Err(VidReconstructError {
            view,
            payload_commitment,
            kind,
            bad_share_keys,
        })
    }

    /// Keep the first share covering each shard position, in input order.
    ///
    /// A replayed share verifies against the commitment (merkle proofs don't
    /// bind the recipient), but feeding the same shard position twice breaks
    /// the decoder, and double-counting its weight would fake quorum.
    fn dedup_by_range(
        view: ViewNumber,
        shares: &[(T::SignatureKey, AvidmGf2Share)],
    ) -> Vec<AvidmGf2Share> {
        let mut covered = BTreeSet::new();
        let mut out = Vec::with_capacity(shares.len());
        for (key, share) in shares {
            // Intake rejects shares without a consistent range.
            let Some(range) = share.range() else {
                continue;
            };
            if range.clone().any(|position| covered.contains(&position)) {
                warn!(%view, sender = ?key, "dropped VID share with an already-covered shard range");
                continue;
            }
            covered.extend(range.clone());
            out.push(share.clone());
        }
        out
    }

    /// Decode `shares` and return the payload bytes only if they re-commit to
    /// `payload_commitment`. Recovery alone does not bind the decoded bytes
    /// to the commitment: a Byzantine disperser can commit to a non-codeword,
    /// and a bad share poisons the erasure decoding.
    fn decode_and_recommit(
        view: ViewNumber,
        common: &AvidmGf2Common,
        shares: &[AvidmGf2Share],
        payload_commitment: &VidCommitment2,
        metadata: &Metadata<T>,
    ) -> Option<Vec<u8>> {
        let bytes = match AvidmGf2Scheme::recover(common, shares) {
            Ok(bytes) => bytes,
            Err(err) => {
                warn!(%view, %err, "VID recovery failed");
                return None;
            },
        };
        let ns_table = parse_ns_table(bytes.len(), &metadata.encode());
        match AvidmGf2Scheme::commit(&common.param, &bytes, ns_table) {
            Ok((recomputed, _)) if recomputed == *payload_commitment => Some(bytes),
            Ok((recomputed, _)) => {
                warn!(
                    %view,
                    expected = %payload_commitment,
                    %recomputed,
                    "reconstructed payload does not match the payload commitment"
                );
                None
            },
            Err(err) => {
                warn!(%view, %err, "failed to recommit reconstructed VID payload");
                None
            },
        }
    }

    fn output(
        view: ViewNumber,
        epoch: EpochNumber,
        payload_commitment: VidCommitment2,
        bytes: &[u8],
        metadata: Metadata<T>,
    ) -> VidReconstructOutput<T> {
        let payload = T::BlockPayload::from_bytes(bytes, &metadata);
        let tx_commitments = payload.transaction_commitments(&metadata);
        VidReconstructOutput {
            view,
            epoch,
            payload_commitment,
            payload,
            metadata,
            tx_commitments,
        }
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        let keep = self.calculations.split_off(&view_number);
        for handle in self.calculations.values_mut() {
            handle.abort();
        }
        self.calculations = keep;
        self.accumulators = self
            .accumulators
            .split_off(&(view_number, VidCommitment2::default()));
        self.reconstructed = self.reconstructed.split_off(&view_number);
    }

    /// Stop tracking `view`.
    ///
    /// Either because its payload was reconstructed (or obtained elsewhere)
    /// or because it timed out and will never be decided: record it so
    /// `handle_vid_share` ignores later shares, drop its accumulators, and
    /// abort any in-flight reconstruction task.
    pub fn retire_view(&mut self, view: ViewNumber) {
        self.reconstructed.insert(view);
        self.accumulators.retain(|(v, _), _| *v != view);
        if let Some(handle) = self.calculations.remove(&view) {
            handle.abort();
        }
    }
}
