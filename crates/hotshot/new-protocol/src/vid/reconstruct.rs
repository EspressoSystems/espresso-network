use std::{
    collections::{BTreeMap, BTreeSet, HashSet, btree_map::Entry},
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use committable::Commitment;
use hotshot_types::{
    consensus::ConsensusMetricsValue,
    data::{
        EpochNumber, VidCommitment2, VidDisperseShare2, ViewNumber,
        ns_table::{ns_table_payload_byte_len, parse_ns_table},
    },
    traits::{
        BlockPayload,
        block_contents::EncodeBytes,
        metrics::{Counter, Histogram},
        node_implementation::NodeType,
    },
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Param, AvidmGf2Scheme, AvidmGf2Share},
};
use tokio::task::{AbortHandle, Id, JoinSet};
use tracing::{debug, error, warn};

use super::ns_lens_match_metadata;

pub(crate) type Metadata<T> = <<T as NodeType>::BlockPayload as BlockPayload<T>>::Metadata;

type ReconstructResult<T> =
    Result<ObtainedPayload<T>, VidReconstructError<<T as NodeType>::SignatureKey>>;

/// A block's payload in hand, with what the node needs to file it.
///
/// Produced from a view's shares and, on the recovery path, from a payload
/// a peer sent whole. Either way the bytes are bound to `payload_commitment`
/// before this is built.
pub struct ObtainedPayload<T: NodeType> {
    pub view: ViewNumber,
    pub epoch: EpochNumber,
    pub payload_commitment: VidCommitment2,
    pub payload: T::BlockPayload,
    pub metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
    pub tx_commitments: Vec<Commitment<T::Transaction>>,
    pub source: PayloadSource,
}

/// How a payload came into hand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadSource {
    /// Every systematic share was in hand: the payload is their concatenation.
    Systematic,
    /// A threshold of shares was erasure-decoded.
    Decoded,
    /// A peer sent the payload whole.
    Fetched,
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

/// What vouches for the proposal a view is pinned to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PinnedBy {
    /// The leader alone: the proposal validated and paired with this node's
    /// share. An equivocating leader may have given the quorum another one.
    Leader,
    /// A certificate: the quorum voted for it, so the view decides on it if
    /// it decides at all.
    Certificate,
}

#[derive(Default)]
pub struct VidReconstructor<T: NodeType> {
    /// Shares held for a commitment no proposal has pinned their view to,
    /// one per voter: they arrived before the proposal, or while the leader's
    /// pin holds another commitment that a certificate may yet replace.
    pending: BTreeMap<ViewNumber, BTreeMap<T::SignatureKey, VidDisperseShare2<T>>>,
    /// One accumulator per view, created when a proposal pins it:
    /// reconstruction needs the proposal's metadata, so only the pinned
    /// commitment is worth accumulating.
    accumulators: BTreeMap<ViewNumber, VidShareAccumulator<T>>,
    reconstructed: BTreeSet<ViewNumber>,
    tasks: JoinSet<Attempt<T>>,
    calculations: BTreeMap<ViewNumber, Attempts>,
    metrics: Option<VidReconstructMetrics>,
}

/// Per-attempt metrics: run time by path, and how many attempts failed or
/// lost the race to the other path.
pub struct VidReconstructMetrics {
    systematic_duration: Arc<dyn Histogram>,
    decode_duration: Arc<dyn Histogram>,
    failures: Arc<dyn Counter>,
    lost: Arc<dyn Counter>,
}

impl VidReconstructMetrics {
    pub fn new(consensus: &ConsensusMetricsValue) -> Self {
        Self {
            systematic_duration: consensus.vid_reconstruct_systematic_duration.clone().into(),
            decode_duration: consensus.vid_reconstruct_decode_duration.clone().into(),
            failures: consensus.vid_reconstruct_failures.clone().into(),
            lost: consensus.vid_reconstruct_lost.clone().into(),
        }
    }

    fn record<T: NodeType>(&self, attempt: &Attempt<T>) {
        let duration = match attempt.path {
            ReconstructPath::Systematic => &self.systematic_duration,
            ReconstructPath::Decode => &self.decode_duration,
        };
        duration.add_point(attempt.elapsed.as_secs_f64());
        match &attempt.outcome {
            Some(Ok(_)) => {},
            Some(Err(_)) => self.failures.add(1),
            None => self.lost.add(1),
        }
    }
}

/// A reconstruction path: which shares it needs and what it does with them.
///
/// The code is systematic, so the shares holding the shard positions below
/// the recovery threshold hold the payload itself. That set is specific, so
/// it may complete after any threshold of mixed shares has: the paths race,
/// and a decode attempt started on mixed shares keeps running when the
/// systematic set completes behind it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReconstructPath {
    /// Every systematic shard in hand; the payload is their concatenation.
    Systematic,
    /// A threshold of shards, erasure-decoded.
    Decode,
}

impl From<ReconstructPath> for PayloadSource {
    fn from(path: ReconstructPath) -> Self {
        match path {
            ReconstructPath::Systematic => Self::Systematic,
            ReconstructPath::Decode => Self::Decoded,
        }
    }
}

/// A view's in-flight attempts, at most one per path. `won` is set by the
/// first to re-commit, so the other skips its own re-commit: a blocking task
/// cannot be aborted once it has started.
#[derive(Default)]
struct Attempts {
    systematic: Option<AbortHandle>,
    decode: Option<AbortHandle>,
    won: Arc<AtomicBool>,
}

impl Attempts {
    fn slot(&mut self, path: ReconstructPath) -> &mut Option<AbortHandle> {
        match path {
            ReconstructPath::Systematic => &mut self.systematic,
            ReconstructPath::Decode => &mut self.decode,
        }
    }

    fn is_empty(&self) -> bool {
        self.systematic.is_none() && self.decode.is_none()
    }

    fn abort(&self) {
        for handle in [&self.systematic, &self.decode].into_iter().flatten() {
            handle.abort();
        }
    }

    fn forget(&mut self, id: Id) {
        for slot in [&mut self.systematic, &mut self.decode] {
            if slot.as_ref().is_some_and(|handle| handle.id() == id) {
                *slot = None;
            }
        }
    }
}

/// What a reconstruction task returns.
struct Attempt<T: NodeType> {
    view: ViewNumber,
    path: ReconstructPath,
    elapsed: Duration,
    /// `None` if the other path won while this one ran.
    outcome: Option<ReconstructResult<T>>,
}

pub(crate) struct VidShareAccumulator<T: NodeType> {
    /// The payload commitment claimed by the proposal the view is pinned to.
    payload_commitment: VidCommitment2,
    /// What vouches for that proposal; only a certificate's pin is final.
    pinned_by: PinnedBy,
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
            metrics: None,
        }
    }

    pub fn with_metrics(mut self, metrics: Option<VidReconstructMetrics>) -> Self {
        self.metrics = metrics;
        self
    }

    /// Pin `view` to a proposal's payload commitment and metadata, and admit
    /// the held shares for that commitment.
    ///
    /// The first pin holds against later ones, except that a certificate's
    /// replaces the leader's: an equivocating leader may have paired this
    /// node with a proposal the quorum did not certify.
    pub(crate) fn handle_proposal(
        &mut self,
        view: ViewNumber,
        payload_commitment: VidCommitment2,
        metadata: Metadata<T>,
        epoch: EpochNumber,
        expected_param: Option<AvidmGf2Param>,
        pinned_by: PinnedBy,
    ) {
        if self.reconstructed.contains(&view) {
            return;
        }
        let accumulator = match self.accumulators.entry(view) {
            Entry::Occupied(mut existing) => {
                let pinned = existing.get_mut();
                if pinned.payload_commitment == payload_commitment {
                    if pinned_by == PinnedBy::Certificate {
                        pinned.pinned_by = PinnedBy::Certificate;
                    }
                    return;
                }
                if pinned_by == PinnedBy::Leader || pinned.pinned_by == PinnedBy::Certificate {
                    warn!(%view, "conflicting proposal for a view pinned to another commitment");
                    return;
                }
                warn!(%view, "leader equivocated; re-pinning to the certified proposal");
                if let Some(attempts) = self.calculations.remove(&view) {
                    attempts.abort();
                }
                existing.insert(VidShareAccumulator::new(
                    payload_commitment,
                    metadata,
                    epoch,
                    expected_param,
                    pinned_by,
                ));
                existing.into_mut()
            },
            Entry::Vacant(slot) => slot.insert(VidShareAccumulator::new(
                payload_commitment,
                metadata,
                epoch,
                expected_param,
                pinned_by,
            )),
        };
        // Shares for other commitments stay held: a certificate may yet re-pin
        // the view to one of them.
        let held = self.pending.remove(&view).unwrap_or_default();
        let (admitted, held): (BTreeMap<_, _>, BTreeMap<_, _>) = held
            .into_iter()
            .partition(|(_, share)| share.payload_commitment == payload_commitment);
        if !held.is_empty() {
            self.pending.insert(view, held);
        }
        for (sender, share) in admitted {
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
        // one slot and guards the `pending` window, where a share cannot yet
        // be verified against a commitment.
        if share.recipient_key != sender {
            warn!(%view, ?sender, "VID share recipient key does not match its sender");
            return;
        }
        if self.reconstructed.contains(&view) {
            return;
        }
        let Some(accumulator) = self.accumulators.get_mut(&view) else {
            self.hold(view, sender, share);
            return;
        };
        if share.payload_commitment == accumulator.payload_commitment {
            accumulator.accept(view, sender, share);
            self.try_reconstruct(view);
            return;
        }
        match accumulator.pinned_by {
            // A certificate may yet re-pin the view to the share's commitment.
            PinnedBy::Leader => self.hold(view, sender, share),
            PinnedBy::Certificate => {
                warn!(%view, ?sender, "VID share commitment differs from the certified proposal's");
            },
        }
    }

    /// Hold a voter's share until a proposal pins the view to its commitment.
    /// One slot per voter: the first share it sends for the view stays.
    fn hold(&mut self, view: ViewNumber, sender: T::SignatureKey, share: VidDisperseShare2<T>) {
        self.pending
            .entry(view)
            .or_default()
            .entry(sender)
            .or_insert(share);
    }

    pub async fn next(&mut self) -> Option<ReconstructResult<T>> {
        loop {
            let attempt = match self.tasks.join_next().await? {
                Ok(attempt) => attempt,
                Err(err) => {
                    if err.is_panic() {
                        error!(%err, "VID reconstruction task panicked");
                    }
                    self.calculations.retain(|_, attempts| {
                        attempts.forget(err.id());
                        !attempts.is_empty()
                    });
                    continue;
                },
            };
            if let Some(metrics) = &self.metrics {
                metrics.record(&attempt);
            }
            let Attempt {
                view,
                path,
                outcome,
                ..
            } = attempt;
            self.finish_attempt(view, path);
            let Some(outcome) = outcome else {
                continue;
            };
            let payload_commitment = match &outcome {
                Ok(out) => out.payload_commitment,
                Err(err) => err.payload_commitment,
            };
            if self.is_superseded(view, payload_commitment) {
                continue;
            }
            match outcome {
                Ok(out) => {
                    if let Some(attempts) = self.calculations.remove(&view) {
                        attempts.abort();
                    }
                    self.accumulators.remove(&view);
                    self.reconstructed.insert(view);
                    return Some(Ok(out));
                },
                Err(err) => {
                    self.handle_failed_attempt(&err);
                    return Some(Err(err));
                },
            }
        }
    }

    /// Clear a finished attempt's slot, dropping the view's entry once both
    /// paths are idle.
    fn finish_attempt(&mut self, view: ViewNumber, path: ReconstructPath) {
        if let Entry::Occupied(mut entry) = self.calculations.entry(view) {
            *entry.get_mut().slot(path) = None;
            if entry.get().is_empty() {
                entry.remove();
            }
        }
    }

    /// Whether an attempt's result is stale: its view was retired, or its pin
    /// was since replaced by a certificate's. The attempt was aborted, but a
    /// blocking task that had already started still runs to completion.
    fn is_superseded(&self, view: ViewNumber, payload_commitment: VidCommitment2) -> bool {
        self.reconstructed.contains(&view)
            || self
                .accumulators
                .get(&view)
                .is_some_and(|accumulator| accumulator.payload_commitment != payload_commitment)
    }

    /// Apply the outcome of a failed attempt: weed the bad shares out of the
    /// accumulator, then either mark the payload as unrecoverable or retry
    /// (`try_reconstruct` re-checks coverage, which shares that arrived while
    /// the attempt ran may already restore).
    fn handle_failed_attempt(&mut self, err: &VidReconstructError<T::SignatureKey>) {
        let Some(accumulator) = self.accumulators.get_mut(&err.view) else {
            return;
        };
        for key in &err.bad_share_keys {
            accumulator.shares.remove(key);
        }
        match err.kind {
            VidReconstructErrorKind::Unrecoverable => accumulator.exhausted = true,
            VidReconstructErrorKind::AwaitingShares => self.try_reconstruct(err.view),
        }
    }

    fn try_reconstruct(&mut self, view: ViewNumber) {
        let Some(accumulator) = self.accumulators.get(&view) else {
            return;
        };
        if accumulator.exhausted {
            return;
        }
        let Some(path) = accumulator.ready_path() else {
            return;
        };
        let attempts = self.calculations.entry(view).or_default();
        if attempts.slot(path).is_some() {
            return;
        }
        // A ready path implies an admitted share, which pinned the common.
        let Some(common) = accumulator.common.clone() else {
            return;
        };
        debug!(%view, ?path, "spawning VID reconstruction attempt");
        let job = ReconstructJob {
            view,
            epoch: accumulator.epoch,
            payload_commitment: accumulator.payload_commitment,
            common,
            shares: accumulator.shares_for(path),
            metadata: accumulator.metadata.clone(),
            path,
            won: attempts.won.clone(),
        };
        let task = self.tasks.spawn_blocking(move || job.run());
        *attempts.slot(path) = Some(task);
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        let keep = self.calculations.split_off(&view_number);
        for attempts in self.calculations.values() {
            attempts.abort();
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
        if let Some(attempts) = self.calculations.remove(&view) {
            attempts.abort();
        }
    }
}

impl<T: NodeType> VidShareAccumulator<T> {
    fn new(
        payload_commitment: VidCommitment2,
        metadata: Metadata<T>,
        epoch: EpochNumber,
        expected_param: Option<AvidmGf2Param>,
        pinned_by: PinnedBy,
    ) -> Self {
        Self {
            payload_commitment,
            pinned_by,
            metadata,
            epoch,
            expected_param,
            common: None,
            shares: BTreeMap::new(),
            seen_keys: HashSet::new(),
            exhausted: false,
        }
    }

    /// Admit `share`, which claims this accumulator's commitment, from the
    /// authenticated `sender`, dropping it if it fails any intake check. The
    /// non-overlapping path stays crypto-free; a shard-range overlap is the
    /// only case that triggers per-share verification, via
    /// [`Self::resolve_conflict`].
    fn accept(&mut self, view: ViewNumber, sender: T::SignatureKey, share: VidDisperseShare2<T>) {
        debug_assert_eq!(share.payload_commitment, self.payload_commitment);
        if self.exhausted {
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
        // The commitment hash-binds the common's `ns_commits` but not its
        // `ns_lens`, which a forwarded share could shuffle against the
        // proposal's table. Trust a common as the verification oracle only
        // after both checks; later shares must carry the same common.
        if let Some(common) = &self.common {
            if share.common != *common {
                warn!(%view, ?sender, "VID share common differs from the accumulator's");
                return;
            }
        } else if !AvidmGf2Scheme::is_consistent(&self.payload_commitment, &share.common) {
            warn!(%view, ?sender, "VID share common is inconsistent with its commitment");
            return;
        } else if !ns_lens_match_metadata(&share.common, &self.metadata.encode()) {
            warn!(
                %view, ?sender,
                "VID share namespace lengths disagree with the proposal's namespace table"
            );
            return;
        } else {
            self.common = Some(share.common.clone());
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

    /// Number of systematic shard positions, those below `threshold`, covered
    /// by the admitted shares.
    fn systematic_coverage(&self, threshold: usize) -> usize {
        self.ranges()
            .map(|range| range.end.min(threshold).saturating_sub(range.start))
            .sum()
    }

    /// The path the admitted shares are ready for, if any. Systematic first:
    /// with every systematic shard in hand there is nothing to decode.
    fn ready_path(&self) -> Option<ReconstructPath> {
        let threshold = self.common.as_ref()?.param.recovery_threshold;
        if self.systematic_coverage(threshold) >= threshold {
            Some(ReconstructPath::Systematic)
        } else if self.coverage() >= threshold {
            Some(ReconstructPath::Decode)
        } else {
            None
        }
    }

    /// The admitted shares an attempt down `path` works from. The systematic
    /// path takes only the shares holding systematic shards, so a bad share
    /// elsewhere cannot poison it.
    fn shares_for(&self, path: ReconstructPath) -> Vec<(T::SignatureKey, AvidmGf2Share)> {
        let threshold = self
            .common
            .as_ref()
            .map_or(0, |common| common.param.recovery_threshold);
        self.shares
            .iter()
            .filter(|(_, share)| match path {
                ReconstructPath::Decode => true,
                ReconstructPath::Systematic => {
                    share.range().is_some_and(|range| range.start < threshold)
                },
            })
            .map(|(key, share)| (key.clone(), share.clone()))
            .collect()
    }
}

/// One reconstruction attempt's inputs.
struct ReconstructJob<T: NodeType> {
    view: ViewNumber,
    epoch: EpochNumber,
    payload_commitment: VidCommitment2,
    common: AvidmGf2Common,
    shares: Vec<(T::SignatureKey, AvidmGf2Share)>,
    metadata: Metadata<T>,
    path: ReconstructPath,
    won: Arc<AtomicBool>,
}

impl<T: NodeType> ReconstructJob<T> {
    /// Recover the payload from the shares and accept it only if it
    /// re-commits to `payload_commitment`: recovery alone does not bind the
    /// bytes to the commitment, since a Byzantine disperser can commit to a
    /// non-codeword and a bad share poisons the erasure decoding.
    ///
    /// On failure, report the shares that fail verification against the
    /// commitment (each share is self-authenticating via its merkle proofs) so
    /// they can be weeded out. If every share verifies, the payload is
    /// unrecoverable: the shares cover the recovery threshold with disjoint
    /// ranges, so the disperser committed to a non-codeword and no share
    /// subset can ever succeed.
    fn run(self) -> Attempt<T> {
        let Self {
            view,
            epoch,
            payload_commitment,
            common,
            shares,
            metadata,
            path,
            won,
        } = self;
        let started = Instant::now();
        let (keys, shares): (Vec<_>, Vec<_>) = shares.into_iter().unzip();
        let bytes = recover(view, &common, &shares);
        // The other path won meanwhile: its re-commit already bound the
        // payload, so this one would only confirm it.
        if won.load(Ordering::Acquire) {
            return Attempt {
                view,
                path,
                elapsed: started.elapsed(),
                outcome: None,
            };
        }
        let outcome = match bytes {
            Some(bytes)
                if matches_commitment(
                    view,
                    &common.param,
                    &metadata.encode(),
                    &bytes,
                    &payload_commitment,
                ) =>
            {
                won.store(true, Ordering::Release);
                let payload = T::BlockPayload::from_bytes(&bytes, &metadata);
                let tx_commitments = payload.transaction_commitments(&metadata);
                Ok(ObtainedPayload {
                    view,
                    epoch,
                    payload_commitment,
                    payload,
                    metadata,
                    tx_commitments,
                    source: path.into(),
                })
            },
            _ => Err(weed::<T>(view, payload_commitment, &common, keys, &shares)),
        };
        Attempt {
            view,
            path,
            elapsed: started.elapsed(),
            outcome: Some(outcome),
        }
    }
}

/// Recover the payload bytes from `shares`, unbound to any commitment.
fn recover(view: ViewNumber, common: &AvidmGf2Common, shares: &[AvidmGf2Share]) -> Option<Vec<u8>> {
    match AvidmGf2Scheme::recover(common, shares) {
        Ok(bytes) => Some(bytes),
        Err(err) => {
            warn!(%view, %err, "VID recovery failed");
            None
        },
    }
}

/// The error for an attempt whose recovered bytes did not re-commit, blaming
/// the shares that fail verification.
fn weed<T: NodeType>(
    view: ViewNumber,
    payload_commitment: VidCommitment2,
    common: &AvidmGf2Common,
    keys: Vec<T::SignatureKey>,
    shares: &[AvidmGf2Share],
) -> VidReconstructError<T::SignatureKey> {
    let bad_share_keys: Vec<_> = keys
        .into_iter()
        .zip(shares)
        .filter(|(_, share)| !share_verifies(common, share))
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
    VidReconstructError {
        view,
        payload_commitment,
        kind,
        bad_share_keys,
    }
}

/// Whether `payload` is what `payload_commitment` commits to, sliced by the
/// header's namespace table `ns_table`.
///
/// The commitment covers each namespace's bytes but not the table, and
/// `parse_ns_table` substitutes one namespace spanning the payload for a table
/// sized for another payload. For a one-namespace block that substitute is the
/// honest table, so a header claiming a shorter or longer payload would
/// recommit to the same bytes and decide with a table that misdescribes them.
/// Refuse such a table instead: it must be sized for `payload`, or be no
/// table at all, which the disperser also reads as one namespace.
pub(crate) fn matches_commitment(
    view: ViewNumber,
    param: &AvidmGf2Param,
    ns_table: &[u8],
    payload: &[u8],
    payload_commitment: &VidCommitment2,
) -> bool {
    if let Some(claimed) = ns_table_payload_byte_len(ns_table)
        && claimed != payload.len()
    {
        warn!(
            %view,
            claimed,
            actual = payload.len(),
            "namespace table is sized for another payload"
        );
        return false;
    }
    let ns_table = parse_ns_table(payload.len(), ns_table);
    match AvidmGf2Scheme::commit(param, payload, ns_table) {
        Ok((recomputed, _)) if recomputed == *payload_commitment => true,
        Ok((recomputed, _)) => {
            warn!(
                %view,
                expected = %payload_commitment,
                %recomputed,
                "payload does not match the payload commitment"
            );
            false
        },
        Err(err) => {
            warn!(%view, %err, "failed to recommit VID payload");
            false
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
