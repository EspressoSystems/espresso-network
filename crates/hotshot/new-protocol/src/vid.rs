use std::{
    collections::{BTreeMap, BTreeSet, HashSet, btree_map::Entry},
    mem,
    ops::Range,
};

use committable::Commitment;
use hotshot::traits::BlockPayload;
use hotshot_types::{
    data::{
        EpochNumber, VidCommitment2, VidDisperse2, VidDisperseShare2, ViewNumber,
        ns_table::parse_ns_table,
        vid_disperse::{AvidmGf2DisperseShareFragment, AvidmGf2NamespacePiece},
    },
    epoch_membership::EpochMembershipCoordinator,
    message::Proposal as SignedProposal,
    traits::{
        block_contents::EncodeBytes, node_implementation::NodeType, signature_key::SignatureKey,
    },
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Param, AvidmGf2Scheme, AvidmGf2Share},
};
use hotshot_utils::anytrace::{self, Wrap};
use rayon::prelude::*;
use tokio::task::{AbortHandle, JoinSet};
use tracing::{error, warn};

use crate::{
    message::{ConsensusMessage, Message, MessageType},
    network::{NetworkError, Sender},
};

pub struct VidDisperseOutput {
    pub view: ViewNumber,
    pub payload_commitment: VidCommitment2,
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

type Metadata<T> = <<T as NodeType>::BlockPayload as BlockPayload<T>>::Metadata;
type ReconstructResult<T> =
    Result<VidReconstructOutput<T>, VidReconstructError<<T as NodeType>::SignatureKey>>;

/// Whether `share` verifies against a `common` already known to be hash-bound
/// to the commitment (a `verify_with_verified_common` success).
fn share_verifies(common: &AvidmGf2Common, share: &AvidmGf2Share) -> bool {
    matches!(
        AvidmGf2Scheme::verify_share_with_verified_common(common, share),
        Ok(Ok(()))
    )
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
    network: Sender<T>,
    public_key: T::SignatureKey,
    private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    bucket_threshold: usize,
    tasks: JoinSet<Result<VidDisperseOutput, VidDisperseError>>,
}

impl<T: NodeType> VidDisperser<T> {
    pub fn new(
        epoch_membership_coordinator: EpochMembershipCoordinator<T>,
        network: Sender<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
    ) -> Self {
        Self {
            calculations: BTreeMap::new(),
            epoch_membership_coordinator,
            network,
            public_key,
            private_key,
            bucket_threshold: 128 * 1024,
            tasks: JoinSet::new(),
        }
    }

    pub fn with_bucket_threshold(mut self, len: usize) -> Self {
        self.bucket_threshold = len;
        self
    }

    pub fn request_vid_disperse(&mut self, vid_disperse_request: VidDisperseRequest<T>) {
        let key = (
            vid_disperse_request.view,
            vid_disperse_request.payload_commitment,
        );
        if self.calculations.contains_key(&key) {
            return;
        }
        let membership = self.epoch_membership_coordinator.clone();
        let network = self.network.clone();
        let public_key = self.public_key.clone();
        let private_key = self.private_key.clone();
        let bucket_threshold = self.bucket_threshold;
        let handle = self.tasks.spawn_blocking(move || {
            Self::handle_vid_disperse_request(
                membership,
                network,
                public_key,
                private_key,
                vid_disperse_request,
                bucket_threshold,
            )
        });
        self.calculations.insert(key, handle);
    }

    pub async fn next(&mut self) -> Option<Result<VidDisperseOutput, VidDisperseError>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(result)) => return Some(result),
                Some(Err(err)) => {
                    if err.is_panic() {
                        error!(%err, "vid disperse task panic");
                    }
                    continue;
                },
                None => return None,
            }
        }
    }

    fn handle_vid_disperse_request(
        epoch_membership_coordinator: EpochMembershipCoordinator<T>,
        network: Sender<T>,
        public_key: T::SignatureKey,
        private_key: <T::SignatureKey as SignatureKey>::PrivateKey,
        vid_disperse_request: VidDisperseRequest<T>,
        bucket_threshold: usize,
    ) -> Result<VidDisperseOutput, VidDisperseError> {
        let view = vid_disperse_request.view;
        let epoch = vid_disperse_request.epoch;
        let payload_commitment = vid_disperse_request.payload_commitment;

        let params = VidDisperse2::<T>::disperse_params(
            &vid_disperse_request.block,
            &epoch_membership_coordinator,
            Some(epoch),
            &vid_disperse_request.metadata,
        )
        .map_err(VidDisperseError::Vid)?;

        let signature = T::SignatureKey::sign(&private_key, payload_commitment.as_ref())
            .map_err(|err| VidDisperseError::Sign(err.into()))?;

        let num_namespaces = params.ns_table.len();

        // Coalesce small namespaces into balanced buckets so a block of many
        // tiny namespaces is sent as a few messages per recipient rather than
        // one tiny message each. Each bucket is one parallel unit and one
        // message per recipient.
        let buckets: Vec<Vec<usize>> = bucketize(&params.ns_table, bucket_threshold);

        buckets.par_iter().try_for_each_with(
            network,
            |network, bucket| -> Result<(), VidDisperseError> {
                let mut pieces = vec![Vec::new(); params.recipients.len()];

                for &ns_index in bucket {
                    let dispersal = AvidmGf2Scheme::ns_disperse_one(
                        &params.param,
                        &params.weights,
                        &params.payload[params.ns_table[ns_index].clone()],
                        ns_index,
                    )
                    .wrap()
                    .map_err(VidDisperseError::Vid)?;

                    let ns_payload_byte_len = dispersal.payload_byte_len;
                    let ns_commit = dispersal.commit;
                    for (pieces, ns_share) in pieces.iter_mut().zip(dispersal.shares) {
                        pieces.push(AvidmGf2NamespacePiece {
                            ns_index: dispersal.ns_index,
                            ns_payload_byte_len,
                            ns_commit,
                            ns_share,
                        });
                    }
                }

                for (recipient, pieces) in params.recipients.iter().zip(pieces) {
                    let fragment = AvidmGf2DisperseShareFragment {
                        view_number: view,
                        epoch: Some(epoch),
                        target_epoch: Some(epoch),
                        payload_commitment,
                        recipient_key: recipient.clone(),
                        param: params.param.clone(),
                        num_namespaces,
                        namespaces: pieces,
                    };
                    let message = Message {
                        sender: public_key.clone(),
                        message_type: MessageType::Consensus(ConsensusMessage::VidShareFragment(
                            SignedProposal::new(fragment, signature.clone()),
                        )),
                    };
                    if let Err(err) = network.unicast(view, recipient, &message) {
                        if err.is_critical() {
                            return Err(err.into());
                        }
                        warn!(%err, "network error while sending vid share fragment");
                    }
                }
                Ok(())
            },
        )?;

        Ok(VidDisperseOutput {
            view,
            payload_commitment,
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

/// Group namespace indices into buckets whose payload sizes are each at least
/// `threshold` bytes (except possibly the last), coalescing small namespaces so
/// the disperser sends one message per bucket rather than one per namespace.
///
/// A `threshold` of 0 puts every namespace in its own bucket (no coalescing); a
/// threshold larger than the whole payload yields a single bucket. A namespace
/// at least `threshold` bytes on its own seals its bucket immediately, so large
/// namespaces stay separate while small ones accumulate.
fn bucketize(ns_table: &[Range<usize>], threshold: usize) -> Vec<Vec<usize>> {
    let mut buckets = Vec::new();
    let mut current = Vec::new();
    let mut current_bytes = 0usize;
    for (ns_index, range) in ns_table.iter().enumerate() {
        current.push(ns_index);
        current_bytes += range.len();
        if current_bytes >= threshold {
            buckets.push(mem::take(&mut current));
            current_bytes = 0;
        }
    }
    if !current.is_empty() {
        buckets.push(current);
    }
    buckets
}

#[derive(Debug, thiserror::Error)]
pub enum VidFragmentError {
    #[error("fragment disagrees with the view's pinned metadata")]
    Inconsistent,
    #[error("namespace index {index} out of range for {num_namespaces} namespaces")]
    IndexOutOfRange { index: usize, num_namespaces: usize },
    #[error("duplicate fragment for namespace index {0}")]
    DuplicateIndex(usize),
}

/// A view's partially-collected namespace pieces, keyed by namespace index.
struct PendingShare<T: NodeType> {
    epoch: Option<EpochNumber>,
    target_epoch: Option<EpochNumber>,
    payload_commitment: VidCommitment2,
    recipient_key: T::SignatureKey,
    param: AvidmGf2Param,
    num_namespaces: usize,
    pieces: BTreeMap<usize, AvidmGf2NamespacePiece>,
}

#[derive(Default)]
pub struct VidFragmentAccumulator<T: NodeType> {
    pending: BTreeMap<ViewNumber, PendingShare<T>>,
    completed: BTreeSet<ViewNumber>,
}

impl<T: NodeType> VidFragmentAccumulator<T> {
    /// Buffer a `fragment` addressed to this node.
    ///
    /// Returns `Ok(None)` while namespaces are still outstanding,
    /// `Ok(Some(share))` once the final namespace completes the view, and
    /// `Err` if the fragment is malformed or inconsistent with the view's
    /// already-pinned metadata.
    pub(crate) fn accept(
        &mut self,
        fragment: AvidmGf2DisperseShareFragment<T>,
    ) -> Result<Option<VidDisperseShare2<T>>, VidFragmentError> {
        let view = fragment.view_number;
        if self.completed.contains(&view) {
            return Ok(None);
        }
        if fragment.num_namespaces == 0 {
            return Err(VidFragmentError::IndexOutOfRange {
                index: 0,
                num_namespaces: 0,
            });
        }
        let pending = match self.pending.entry(view) {
            Entry::Vacant(slot) => slot.insert(PendingShare {
                epoch: fragment.epoch,
                target_epoch: fragment.target_epoch,
                payload_commitment: fragment.payload_commitment,
                recipient_key: fragment.recipient_key.clone(),
                param: fragment.param.clone(),
                num_namespaces: fragment.num_namespaces,
                pieces: BTreeMap::new(),
            }),
            Entry::Occupied(slot) => {
                let pending = slot.into_mut();
                if pending.num_namespaces != fragment.num_namespaces
                    || pending.epoch != fragment.epoch
                    || pending.target_epoch != fragment.target_epoch
                    || pending.payload_commitment != fragment.payload_commitment
                    || pending.recipient_key != fragment.recipient_key
                    || pending.param != fragment.param
                {
                    return Err(VidFragmentError::Inconsistent);
                }
                pending
            },
        };
        for piece in fragment.namespaces {
            let ns_index = piece.ns_index;
            if ns_index >= pending.num_namespaces {
                return Err(VidFragmentError::IndexOutOfRange {
                    index: ns_index,
                    num_namespaces: pending.num_namespaces,
                });
            }
            if pending.pieces.contains_key(&ns_index) {
                return Err(VidFragmentError::DuplicateIndex(ns_index));
            }
            pending.pieces.insert(ns_index, piece);
        }
        if pending.pieces.len() != pending.num_namespaces {
            return Ok(None);
        }
        // Every namespace is present and indices are distinct and in range, so
        // they cover `0..num_namespaces` exactly; the `BTreeMap` yields them in
        // that order.
        let pending = self.pending.remove(&view).expect("just inserted above");
        self.completed.insert(view);
        let mut ns_commits = Vec::with_capacity(pending.num_namespaces);
        let mut ns_lens = Vec::with_capacity(pending.num_namespaces);
        let mut ns_shares = Vec::with_capacity(pending.num_namespaces);
        for piece in pending.pieces.into_values() {
            ns_commits.push(piece.ns_commit);
            ns_lens.push(piece.ns_payload_byte_len);
            ns_shares.push(piece.ns_share);
        }
        Ok(Some(VidDisperseShare2 {
            view_number: view,
            epoch: pending.epoch,
            target_epoch: pending.target_epoch,
            payload_commitment: pending.payload_commitment,
            share: ns_shares.into(),
            recipient_key: pending.recipient_key,
            common: AvidmGf2Common {
                param: pending.param,
                ns_commits,
                ns_lens,
            },
        }))
    }

    pub(crate) fn gc(&mut self, view_number: ViewNumber) {
        self.pending = self.pending.split_off(&view_number);
        self.completed = self.completed.split_off(&view_number);
    }
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
        return Ok(output(view, epoch, payload_commitment, &bytes, metadata));
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

fn output<T: NodeType>(
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

#[derive(Debug, thiserror::Error)]
pub enum VidDisperseError {
    #[error("network error: {0}")]
    Net(#[from] NetworkError),

    #[error("vid error: {0}")]
    Vid(#[source] anytrace::Error),

    #[error("sign error: {0}")]
    Sign(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl VidDisperseError {
    pub fn is_critical(&self) -> bool {
        match self {
            Self::Net(e) => e.is_critical(),
            Self::Vid(_) | Self::Sign(_) => false,
        }
    }
}
