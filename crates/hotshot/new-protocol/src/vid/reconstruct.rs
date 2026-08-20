use std::{
    collections::{BTreeMap, BTreeSet, HashSet, btree_map::Entry},
    ops::Range,
};

use committable::Commitment;
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{EpochNumber, VidCommitment2, VidDisperseShare2, ViewNumber, ns_table::parse_ns_table},
    traits::{block_contents::EncodeBytes, node_implementation::NodeType},
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Param, AvidmGf2Scheme, AvidmGf2Share},
};
use tokio::task::{AbortHandle, Id, JoinSet};
use tracing::{error, warn};

type Metadata<T> = <<T as NodeType>::BlockPayload as BlockPayload<T>>::Metadata;

type ReconstructResult<T> =
    Result<VidReconstructOutput<T>, VidReconstructError<<T as NodeType>::SignatureKey>>;

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
    /// Unverifiable shares were weeded out; reconstruction retries once the
    /// remaining shares cover the recovery threshold again.
    #[error("awaiting more shares after weeding out unverifiable ones")]
    AwaitingShares,
    /// Every share verified yet the payload still does not re-commit: the
    /// disperser committed to a non-codeword, so no subset can ever recover it.
    #[error("unrecoverable: verified shares cannot decode to a payload matching the commitment")]
    Unrecoverable,
    /// A payload obtained whole from a peer does not re-commit. Says nothing
    /// about the view's shares — share-based reconstruction continues — only
    /// that this response was bad.
    #[error("fetched payload does not match the commitment")]
    FetchedPayloadMismatch,
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

pub(crate) struct VidShareAccumulator<T: NodeType> {
    /// The payload commitment claimed by the view's validated proposal.
    payload_commitment: VidCommitment2,
    metadata: Metadata<T>,
    epoch: EpochNumber,
    /// The VID erasure parameters the committee fixes for this view, used to
    /// reject shares carrying a forged `common.param` (see [`Self::accept`]).
    /// `None` if the committee could not be resolved; the param check is then
    /// skipped, matching the previously unchecked path.
    expected_param: Option<AvidmGf2Param>,
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
        expected_param: Option<AvidmGf2Param>,
    ) {
        if self.reconstructed.contains(&view) {
            return;
        }
        let accumulator = match self.accumulators.entry(view) {
            Entry::Occupied(existing) => {
                // The first proposal wins: an equivocating leader cannot re-pin
                // the view to another commitment.
                if existing.get().payload_commitment != payload_commitment {
                    warn!(%view, "conflicting proposal for a view pinned to another commitment");
                }
                return;
            },
            Entry::Vacant(slot) => slot.insert(VidShareAccumulator::new(
                payload_commitment,
                metadata,
                epoch,
                expected_param,
            )),
        };
        for (sender, share) in self.pending.remove(&view).into_iter().flatten() {
            accumulator.accept(view, sender, share);
        }
        self.try_reconstruct(view);
    }

    pub(crate) fn handle_vid_share(
        &mut self,
        sender: T::SignatureKey,
        share: VidDisperseShare2<T>,
    ) {
        let view = share.view_number;
        // A share carries the voter it belongs to; only the authenticated
        // sender may contribute its own. This cheap check bounds each node to
        // one slot and guards the pre-proposal `pending` window, where a share
        // cannot yet be verified against a commitment.
        if share.recipient_key != sender {
            warn!(%view, ?sender, "VID share recipient key does not match its sender");
            return;
        }
        if self.reconstructed.contains(&view) {
            return;
        }
        let Some(accumulator) = self.accumulators.get_mut(&view) else {
            // No validated proposal yet: hold the voter's share until
            // `handle_proposal` pins the view's commitment.
            self.pending
                .entry(view)
                .or_default()
                .entry(sender)
                .or_insert(share);
            return;
        };
        accumulator.accept(view, sender, share);
        self.try_reconstruct(view);
    }

    /// Verify a payload obtained whole from a peer instead of decoded from
    /// shares, and surface it through [`Self::next`] like any reconstruction.
    ///
    /// Nothing in the response is trusted: the bytes count only if they
    /// re-commit to `payload_commitment` under the committee's `param` — the
    /// same binding [`decode_and_recommit`] enforces for shares. The caller
    /// pins commitment and metadata from its own validated proposal, so a
    /// peer can at worst waste the verification.
    ///
    /// Runs outside `calculations` on purpose: a share-based attempt may be
    /// in flight for the same view, and neither should wait for the other.
    /// [`Self::next`] keeps the first success and drops the loser.
    pub(crate) fn handle_fetched_payload(
        &mut self,
        view: ViewNumber,
        epoch: EpochNumber,
        payload_commitment: VidCommitment2,
        metadata: Metadata<T>,
        param: AvidmGf2Param,
        payload: Vec<u8>,
    ) {
        if self.reconstructed.contains(&view) {
            return;
        }
        self.tasks.spawn_blocking(move || {
            let ns_table = parse_ns_table(payload.len(), &metadata.encode());
            match AvidmGf2Scheme::commit(&param, &payload, ns_table) {
                Ok((recomputed, _)) if recomputed == payload_commitment => {
                    let payload = T::BlockPayload::from_bytes(&payload, &metadata);
                    let tx_commitments = payload.transaction_commitments(&metadata);
                    Ok(VidReconstructOutput {
                        view,
                        epoch,
                        payload_commitment,
                        payload,
                        metadata,
                        tx_commitments,
                    })
                },
                Ok((recomputed, _)) => {
                    warn!(
                        %view,
                        expected = %payload_commitment,
                        %recomputed,
                        "fetched payload does not match the payload commitment"
                    );
                    Err(VidReconstructError {
                        view,
                        payload_commitment,
                        kind: VidReconstructErrorKind::FetchedPayloadMismatch,
                        bad_share_keys: Vec::new(),
                    })
                },
                Err(err) => {
                    warn!(%view, %err, "failed to commit fetched payload");
                    Err(VidReconstructError {
                        view,
                        payload_commitment,
                        kind: VidReconstructErrorKind::FetchedPayloadMismatch,
                        bad_share_keys: Vec::new(),
                    })
                },
            }
        });
    }

    pub async fn next(&mut self) -> Option<ReconstructResult<T>> {
        loop {
            match self.tasks.join_next_with_id().await? {
                Ok((id, Ok(out))) => {
                    self.forget_calculation(out.view, id);
                    // A share-based attempt and a fetched-payload verification
                    // may race; the first one out wins, the loser is dropped.
                    if !self.reconstructed.insert(out.view) {
                        continue;
                    }

                    self.accumulators.remove(&out.view);

                    // Nothing left to decode: try to stop the attempt that
                    // lost. Best effort — aborting a blocking task that has
                    // already started does nothing, and its result is dropped
                    // by the `reconstructed` check above either way.
                    if let Some(loser) = self.calculations.remove(&out.view) {
                        loser.abort();
                    }

                    return Some(Ok(out));
                },
                Ok((id, Err(err))) => {
                    self.forget_calculation(err.view, id);
                    self.handle_failed_attempt(&err);
                    return Some(Err(err));
                },
                Err(err) => {
                    if err.is_panic() {
                        error!(%err, "VID reconstruction task panicked");
                    }
                    let view = self
                        .calculations
                        .iter()
                        .find(|(_, task)| task.id() == err.id())
                        .map(|(view, _)| *view);
                    if let Some(view) = view {
                        self.calculations.remove(&view);
                    }
                },
            }
        }
    }

    /// Drop the view's calculation handle, but only if `id` is the calculation
    /// it tracks.
    ///
    /// Fetched-payload verifications run in the same [`JoinSet`] and carry the
    /// same view, so a finished task is not necessarily the tracked one.
    /// Dropping the handle of a calculation that is still running would defeat
    /// the [`Self::try_reconstruct`] guard against a second attempt for the
    /// same view, and leave the first one unabortable.
    fn forget_calculation(&mut self, view: ViewNumber, id: Id) {
        if self
            .calculations
            .get(&view)
            .is_some_and(|task| task.id() == id)
        {
            self.calculations.remove(&view);
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
            VidReconstructErrorKind::FetchedPayloadMismatch => {
                // A bad response indicts the responder, not the view's shares.
            },
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
            reconstruct::<T>(view, epoch, payload_commitment, common, shares, metadata)
        });
        self.calculations.insert(view, task);
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

impl<T: NodeType> VidShareAccumulator<T> {
    fn new(
        payload_commitment: VidCommitment2,
        metadata: Metadata<T>,
        epoch: EpochNumber,
        expected_param: Option<AvidmGf2Param>,
    ) -> Self {
        Self {
            payload_commitment,
            metadata,
            epoch,
            expected_param,
            common: None,
            shares: BTreeMap::new(),
            seen_keys: HashSet::new(),
            exhausted: false,
        }
    }

    /// Admit `share` from the authenticated `sender`, dropping it if it fails
    /// any intake check. The non-overlapping path stays crypto-free; a
    /// shard-range overlap is the only case that triggers per-share
    /// verification, via [`Self::resolve_conflict`].
    fn accept(&mut self, view: ViewNumber, sender: T::SignatureKey, share: VidDisperseShare2<T>) {
        if self.exhausted {
            return;
        }
        if share.payload_commitment != self.payload_commitment {
            warn!(%view, ?sender, "VID share commitment differs from the proposal's");
            return;
        }
        // The commitment binds a share's `ns_commits` but not its `param`, so a
        // Byzantine voter can pair real `ns_commits` with a forged `param` (e.g.
        // an inflated `recovery_threshold`). Pinning that common as the
        // verification oracle would reject every honest share, so reject it now.
        if let Some(expected) = &self.expected_param
            && share.common.param != *expected
        {
            warn!(%view, ?sender, "VID share common param differs from the committee's");
            return;
        }
        // The commitment hash-binds the common, so trust it as the verification
        // oracle only after that check; later shares must carry the same common.
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
        // Honest dispersal assigns disjoint ranges, so an overlap with an
        // admitted share proves a squat; resolve it (needs verification) below.
        let conflicts: Vec<T::SignatureKey> = self
            .shares
            .iter()
            .filter(|(_, admitted)| {
                admitted
                    .range()
                    .is_some_and(|covered| covered.start < range.end && range.start < covered.end)
            })
            .map(|(key, _)| key.clone())
            .collect();
        if conflicts.is_empty() {
            self.seen_keys.insert(sender.clone());
            self.shares.insert(sender, share.share);
            return;
        }
        self.resolve_conflict(view, sender, share, conflicts);
    }

    /// Resolve a shard-range collision between the incoming `share` and the
    /// already-admitted `conflicts`: verify each against the commitment-bound
    /// common, evict those that fail, and admit the newcomer only if it
    /// verifies and no surviving share still covers its range.
    fn resolve_conflict(
        &mut self,
        view: ViewNumber,
        sender: T::SignatureKey,
        share: VidDisperseShare2<T>,
        conflicts: Vec<T::SignatureKey>,
    ) {
        // A conflict implies a prior admission, which pinned the common.
        let Some(common) = self.common.clone() else {
            return;
        };
        // The sender has used its one slot regardless of the outcome.
        self.seen_keys.insert(sender.clone());
        let mut survivor = false;
        for key in conflicts {
            let verified = self
                .shares
                .get(&key)
                .is_some_and(|admitted| share_verifies(&common, admitted));
            if verified {
                survivor = true;
            } else {
                warn!(%view, ?key, "evicting unverifiable VID share squatting a shard range");
                self.shares.remove(&key);
            }
        }
        // A verified share still covers the contested range: the newcomer would
        // double-cover it, so drop it.
        if survivor {
            return;
        }
        if share_verifies(&common, &share.share) {
            self.shares.insert(sender, share.share);
        } else {
            warn!(%view, ?sender, "dropping unverifiable VID share at intake conflict");
        }
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

/// Decode the shares and accept the result only if it re-commits to
/// `payload_commitment`. On failure, report the shares that fail
/// verification against the commitment (each share is self-authenticating
/// via its merkle proofs) so they can be weeded out. If every share
/// verifies, the payload is unrecoverable: the shares cover the recovery
/// threshold with disjoint ranges, so the disperser committed to a
/// non-codeword and no share subset can ever succeed.
fn reconstruct<T: NodeType>(
    view: ViewNumber,
    epoch: EpochNumber,
    payload_commitment: VidCommitment2,
    common: AvidmGf2Common,
    shares: Vec<(T::SignatureKey, AvidmGf2Share)>,
    metadata: Metadata<T>,
) -> ReconstructResult<T> {
    let (keys, shares): (Vec<_>, Vec<_>) = shares.into_iter().unzip();
    if let Some(bytes) =
        decode_and_recommit::<T>(view, &common, &shares, &payload_commitment, &metadata)
    {
        let payload = T::BlockPayload::from_bytes(&bytes, &metadata);
        let tx_commitments = payload.transaction_commitments(&metadata);
        let output = VidReconstructOutput {
            view,
            epoch,
            payload_commitment,
            payload,
            metadata,
            tx_commitments,
        };
        return Ok(output);
    }

    let bad_share_keys: Vec<_> = keys
        .into_iter()
        .zip(&shares)
        .filter(|(_, share)| !share_verifies(&common, share))
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
fn decode_and_recommit<T: NodeType>(
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

/// Whether `share` verifies against a `common` already known to be hash-bound
/// to the commitment (a `verify_with_verified_common` success).
fn share_verifies(common: &AvidmGf2Common, share: &AvidmGf2Share) -> bool {
    matches!(
        AvidmGf2Scheme::verify_share_with_verified_common(common, share),
        Ok(Ok(()))
    )
}

#[cfg(test)]
mod tests {
    use hotshot_example_types::node_types::TestTypes;

    use super::*;

    /// A finished task clears only the calculation it *is*.
    ///
    /// Fetched-payload verifications share the [`JoinSet`] and carry the same
    /// view as the share-based calculation they race, so clearing by view
    /// alone would drop a running calculation's handle: the guard against a
    /// second attempt for that view would stop holding, and the first one
    /// could no longer be aborted.
    #[tokio::test]
    async fn a_finished_task_clears_only_its_own_calculation() {
        let mut reconstructor = VidReconstructor::<TestTypes>::new();
        let view = ViewNumber::new(1);

        let calculation = reconstructor.tasks.spawn(std::future::pending());
        let other = reconstructor.tasks.spawn(std::future::pending());
        let calculation_id = calculation.id();
        reconstructor.calculations.insert(view, calculation);

        reconstructor.forget_calculation(view, other.id());
        assert!(
            reconstructor.calculations.contains_key(&view),
            "another task finishing must leave the calculation in place"
        );

        reconstructor.forget_calculation(view, calculation_id);
        assert!(!reconstructor.calculations.contains_key(&view));
    }

    /// A panicking calculation releases its view.
    ///
    /// A panic yields a [`JoinError`] carrying only the task id, so the view
    /// has to be recovered from `calculations`. Leaving the handle behind
    /// would make [`VidReconstructor::try_reconstruct`] refuse that view for
    /// good, and the share path would never retry it.
    #[tokio::test]
    async fn a_panicking_calculation_releases_its_view() {
        let mut reconstructor = VidReconstructor::<TestTypes>::new();
        let view = ViewNumber::new(1);

        let calculation = reconstructor
            .tasks
            .spawn(async { panic!("decode blew up") });
        reconstructor.calculations.insert(view, calculation);

        // The panic is swallowed; with no other task the stream then ends.
        assert!(reconstructor.next().await.is_none());
        assert!(
            !reconstructor.calculations.contains_key(&view),
            "a panicked calculation must not block its view forever"
        );
    }
}
