use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    ops::Range,
};

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
    /// weeded out. Reconstruction retries once the remaining shares cover
    /// the recovery threshold again.
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
    /// The payload commitment claimed by the view's validated proposal.
    payload_commitment: VidCommitment2,
    metadata: Metadata<T>,
    epoch: EpochNumber,
    /// Common data pinned by the first admitted share; hash-bound to
    /// `payload_commitment` (see [`Self::accept`]).
    common: Option<AvidmGf2Common>,
    /// Admitted shares by voter; their shard ranges are pairwise disjoint
    /// (see [`Self::accept`]).
    shares: BTreeMap<T::SignatureKey, AvidmGf2Share>,
    /// Every voter whose share was admitted, including weeded ones: a voter
    /// whose share failed verification doesn't get a second submission.
    seen_keys: HashSet<T::SignatureKey>,
    /// Set when a fully verified share set failed to decode to a payload
    /// matching the commitment: the disperser is provably faulty and further
    /// attempts are pointless.
    exhausted: bool,
}

impl<T: NodeType> VidShareAccumulator<T> {
    fn new(payload_commitment: VidCommitment2, metadata: Metadata<T>, epoch: EpochNumber) -> Self {
        Self {
            payload_commitment,
            metadata,
            epoch,
            common: None,
            shares: BTreeMap::new(),
            seen_keys: HashSet::new(),
            exhausted: false,
        }
    }

    /// Admit `share` if it claims the proposal's commitment, carries the
    /// common data hash-bound to that commitment, is from a new voter, and
    /// covers a well-formed shard range disjoint from every admitted
    /// share's. Deduplicating by range up front keeps coverage exact — a
    /// replayed share verifies against the commitment (merkle proofs don't
    /// bind the recipient) but cannot fake quorum — and the decoder never
    /// sees a shard position twice.
    fn accept(&mut self, view: ViewNumber, share: VidDisperseShare2<T>) {
        if self.exhausted {
            return;
        }
        let sender = share.recipient_key;
        if share.payload_commitment != self.payload_commitment {
            warn!(%view, ?sender, "VID share commitment differs from the proposal's");
            return;
        }
        // The commitment hash-binds the common data; check that before
        // trusting the common as the verification oracle used for weeding
        // out bad shares. Every later share must carry the identical common.
        if let Some(common) = &self.common {
            if share.common != *common {
                warn!(%view, ?sender, "VID share common differs from the accumulator's");
                return;
            }
        } else if AvidmGf2Scheme::is_consistent(&self.payload_commitment, &share.common) {
            self.common = Some(share.common.clone());
        } else {
            warn!(%view, ?sender, "VID share common is inconsistent with its commitment");
            return;
        }
        // A share whose namespaces disagree on the shard range is malformed.
        let Some(range) = share.share.range() else {
            warn!(%view, ?sender, "VID share has an inconsistent shard range");
            return;
        };
        // An empty range contributes nothing; positions past the end of the
        // encoded payload would inflate coverage without aiding decoding.
        if range.is_empty() || range.end > share.common.param.total_weights {
            warn!(%view, ?sender, ?range, "VID share has an empty or out-of-bounds shard range");
            return;
        }
        if self.seen_keys.contains(&sender) {
            return;
        }
        // Don't charge the sender's one submission for an overlap: their
        // share may become admissible after a squatting share is weeded out.
        if self
            .ranges()
            .any(|covered| covered.start < range.end && range.start < covered.end)
        {
            warn!(%view, ?sender, "VID share covers already-covered shard positions");
            return;
        }
        self.seen_keys.insert(sender.clone());
        self.shares.insert(sender, share.share);
    }

    fn ranges(&self) -> impl Iterator<Item = &Range<usize>> {
        // Admitted shares always have a consistent range (checked in `accept`).
        self.shares.values().filter_map(AvidmGf2Share::range)
    }

    /// Number of shard positions covered by the admitted shares; exact
    /// because their ranges are disjoint.
    fn coverage(&self) -> usize {
        self.ranges().map(ExactSizeIterator::len).sum()
    }

    fn has_enough_shares(&self) -> bool {
        self.common
            .as_ref()
            .is_some_and(|common| self.coverage() >= common.param.recovery_threshold)
    }
}

#[derive(Default)]
pub struct VidReconstructor<T: NodeType> {
    /// Shares that arrived before their view's proposal, one per voter:
    /// admitted (or dropped) once the proposal pins the view's commitment.
    pending: BTreeMap<ViewNumber, BTreeMap<T::SignatureKey, VidDisperseShare2<T>>>,
    /// One accumulator per view, created when its validated proposal
    /// arrives: reconstruction needs the proposal's metadata, so only the
    /// proposal's commitment is worth accumulating.
    accumulators: BTreeMap<ViewNumber, VidShareAccumulator<T>>,
    reconstructed: BTreeSet<ViewNumber>,
    tasks: JoinSet<ReconstructResult<T>>,
    calculations: BTreeMap<ViewNumber, AbortHandle>,
}

impl<T: NodeType> VidReconstructor<T> {
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            accumulators: BTreeMap::new(),
            reconstructed: BTreeSet::new(),
            tasks: JoinSet::new(),
            calculations: BTreeMap::new(),
        }
    }

    /// Pin `view` to its validated proposal's payload commitment and
    /// metadata, and admit any shares that arrived before the proposal.
    pub(crate) fn handle_proposal(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        metadata: Metadata<T>,
        epoch: EpochNumber,
    ) {
        if self.reconstructed.contains(&view) {
            return;
        }
        if let Some(existing) = self.accumulators.get(&view) {
            // The first proposal wins: an equivocating leader cannot re-pin
            // the view to another commitment.
            if existing.payload_commitment != payload_commitment {
                warn!(%view, "conflicting proposal for a view pinned to another commitment");
            }
            return;
        }
        let accumulator = self
            .accumulators
            .entry(view)
            .or_insert_with(|| VidShareAccumulator::new(payload_commitment, metadata, epoch));
        for (_, share) in self.pending.remove(&view).into_iter().flatten() {
            accumulator.accept(view, share);
        }
        self.try_reconstruct(view);
    }

    pub(crate) fn handle_vid_share(&mut self, share: VidDisperseShare2<T>) {
        let view = share.view_number;
        if self.reconstructed.contains(&view) {
            return;
        }
        let Some(accumulator) = self.accumulators.get_mut(&view) else {
            // No validated proposal yet: hold the voter's share until
            // `handle_proposal` pins the view's commitment.
            self.pending
                .entry(view)
                .or_default()
                .entry(share.recipient_key.clone())
                .or_insert(share);
            return;
        };
        accumulator.accept(view, share);
        self.try_reconstruct(view);
    }

    pub async fn next(&mut self) -> Option<ReconstructResult<T>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(Ok(out))) => {
                    self.calculations.remove(&out.view);
                    self.accumulators.remove(&out.view);
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

    /// Apply the outcome of a failed attempt: weed the bad shares out of the
    /// accumulator, then either mark the payload as unrecoverable or retry
    /// (`try_reconstruct` re-checks coverage, which shares that arrived while
    /// the attempt ran may already restore).
    fn handle_failed_attempt(&mut self, err: &VidReconstructError<T::SignatureKey>) {
        let Some(accumulator) = self.accumulators.get_mut(&err.view) else {
            return;
        };
        // Views are pinned to one commitment for their lifetime, so a
        // finished attempt always matches; guard anyway so a future re-pin
        // policy can't weed the wrong accumulator.
        if accumulator.payload_commitment != err.payload_commitment {
            return;
        }
        for key in &err.bad_share_keys {
            accumulator.shares.remove(key);
        }
        match err.kind {
            VidReconstructErrorKind::Unrecoverable => accumulator.exhausted = true,
            VidReconstructErrorKind::AwaitingShares => self.try_reconstruct(err.view),
        }
    }

    fn try_reconstruct(&mut self, view: ViewNumber) {
        if self.calculations.contains_key(&view) {
            return;
        }
        let Some(accumulator) = self.accumulators.get(&view) else {
            return;
        };
        if accumulator.exhausted || !accumulator.has_enough_shares() {
            return;
        }
        // Enough shares implies an admitted share, which pinned the common.
        let Some(common) = accumulator.common.clone() else {
            return;
        };
        let payload_commitment = accumulator.payload_commitment;
        let metadata = accumulator.metadata.clone();
        let shares: Vec<(T::SignatureKey, AvidmGf2Share)> = accumulator
            .shares
            .iter()
            .map(|(key, share)| (key.clone(), share.clone()))
            .collect();
        let epoch = accumulator.epoch;
        let task = self.tasks.spawn_blocking(move || {
            Self::reconstruct(view, epoch, payload_commitment, common, shares, metadata)
        });
        self.calculations.insert(view, task);
    }

    /// Decode the shares and accept the result only if it re-commits to
    /// `payload_commitment`. On failure, report the shares that fail
    /// verification against the commitment (each share is self-authenticating
    /// via its merkle proofs) so they can be weeded out. If every share
    /// verifies, the payload is unrecoverable: the shares cover the recovery
    /// threshold with disjoint ranges, so the disperser committed to a
    /// non-codeword and no share subset can ever succeed.
    fn reconstruct(
        view: ViewNumber,
        epoch: EpochNumber,
        payload_commitment: VidCommitment2,
        common: AvidmGf2Common,
        shares: Vec<(T::SignatureKey, AvidmGf2Share)>,
        metadata: Metadata<T>,
    ) -> ReconstructResult<T> {
        let (keys, shares): (Vec<_>, Vec<_>) = shares.into_iter().unzip();
        if let Some(bytes) =
            Self::decode_and_recommit(view, &common, &shares, &payload_commitment, &metadata)
        {
            return Ok(Self::output(
                view,
                epoch,
                payload_commitment,
                &bytes,
                metadata,
            ));
        }
        let bad_share_keys: Vec<_> = keys
            .into_iter()
            .zip(&shares)
            .filter(|(_, share)| {
                !matches!(
                    AvidmGf2Scheme::verify_share_with_verified_common(&common, share),
                    Ok(Ok(()))
                )
            })
            .map(|(key, _)| key)
            .collect();
        let kind = if bad_share_keys.is_empty() {
            warn!(
                %view,
                %payload_commitment,
                "verified shares cannot decode to a payload matching the commitment"
            );
            VidReconstructErrorKind::Unrecoverable
        } else {
            warn!(
                %view,
                %payload_commitment,
                ?bad_share_keys,
                "weeded out VID shares that failed verification"
            );
            VidReconstructErrorKind::AwaitingShares
        };
        Err(VidReconstructError {
            view,
            payload_commitment,
            kind,
            bad_share_keys,
        })
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
        self.pending = self.pending.split_off(&view_number);
        self.accumulators = self.accumulators.split_off(&view_number);
        self.reconstructed = self.reconstructed.split_off(&view_number);
    }

    /// Stop tracking `view`.
    ///
    /// Either because its payload was reconstructed (or obtained elsewhere)
    /// or because it timed out and will never be decided: record it so
    /// `handle_vid_share` ignores later shares, drop its accumulator and
    /// pending shares, and abort any in-flight reconstruction task.
    pub fn retire_view(&mut self, view: ViewNumber) {
        self.reconstructed.insert(view);
        self.pending.remove(&view);
        self.accumulators.remove(&view);
        if let Some(handle) = self.calculations.remove(&view) {
            handle.abort();
        }
    }
}
