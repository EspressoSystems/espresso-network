use std::collections::{BTreeMap, BTreeSet};

use hotshot_types::{
    data::{
        EpochNumber, VidCommitment2, VidDisperseShare2, ViewNumber,
        vid_disperse::{AvidmGf2DisperseShareFragment, AvidmGf2NamespacePiece},
    },
    traits::node_implementation::NodeType,
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Param},
};

pub struct VidFragmentAccumulator<T: NodeType> {
    /// Partial shares per view, keyed within a view by the disperser that sent
    /// the fragments. Keying by disperser is what keeps one sender's stream
    /// from displacing another's; see [`Self::accept`].
    pending: BTreeMap<ViewNumber, BTreeMap<T::SignatureKey, PendingShare<T>>>,
    completed: BTreeMap<ViewNumber, BTreeSet<T::SignatureKey>>,
    lower_bound: ViewNumber,
}

#[derive(Debug, thiserror::Error)]
pub enum VidFragmentError {
    #[error("fragment disagrees with the view's pinned metadata")]
    Inconsistent,

    #[error("namespace index {index} out of range for {num_namespaces} namespaces")]
    IndexOutOfRange { index: usize, num_namespaces: usize },

    #[error("duplicate fragment for namespace index {0}")]
    DuplicateIndex(usize),

    #[error("fragment contains no namespaces")]
    Empty,

    #[error("fragment names no epoch")]
    MissingEpoch,

    #[error("fragment target epoch {target_epoch:?} does not match its epoch {epoch:?}")]
    TargetEpochMismatch {
        epoch: Option<EpochNumber>,
        target_epoch: Option<EpochNumber>,
    },
}

impl<T: NodeType> Default for VidFragmentAccumulator<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: NodeType> VidFragmentAccumulator<T> {
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            completed: BTreeMap::new(),
            lower_bound: ViewNumber::genesis(),
        }
    }

    /// Buffer a `fragment` addressed to this node, sent by `disperser`.
    ///
    /// Returns `Ok(None)` while namespaces are still outstanding,
    /// `Ok(Some(share))` once the final namespace completes the view, and
    /// `Err` if the fragment is malformed or inconsistent with what
    /// `disperser` already pinned for the view.
    ///
    /// `disperser` is the authenticated message sender, not a field of the
    /// fragment. Each disperser gets its own buffer: a fragment stream commits to
    /// nothing beyond its payload commitment, so a stream that pinned or
    /// completed a view for one sender must not lock out the view's honest
    /// leader.
    pub fn accept(
        &mut self,
        disperser: &T::SignatureKey,
        fragment: AvidmGf2DisperseShareFragment<T>,
    ) -> Result<Option<VidDisperseShare2<T>>, VidFragmentError> {
        let view = fragment.view_number;
        if self.is_retired(view, disperser) {
            return Ok(None);
        }
        let epoch = well_formed(&fragment)?;
        let complete = {
            let pending = self.pending_for(view, disperser, epoch, &fragment)?;
            pending.insert_pieces(fragment.namespaces)?;
            pending.is_complete()
        };
        if !complete {
            return Ok(None);
        }
        let pending = self.take_pending(view, disperser);
        self.completed
            .entry(view)
            .or_default()
            .insert(disperser.clone());
        Ok(Some(pending.into_share(view)))
    }

    pub fn gc(&mut self, view_number: ViewNumber) {
        self.pending = self.pending.split_off(&view_number);
        self.completed = self.completed.split_off(&view_number);
        self.lower_bound = view_number;
    }

    /// Has `view` been GCed, or `disperser` already completed a share for it?
    fn is_retired(&self, view: ViewNumber, disperser: &T::SignatureKey) -> bool {
        view < self.lower_bound
            || self
                .completed
                .get(&view)
                .is_some_and(|keys| keys.contains(disperser))
    }

    /// `disperser`'s buffer for `view`, opening it on the first fragment and
    /// pinning the metadata every later fragment of the stream must repeat.
    fn pending_for<'a>(
        &'a mut self,
        view: ViewNumber,
        disperser: &T::SignatureKey,
        epoch: EpochNumber,
        fragment: &AvidmGf2DisperseShareFragment<T>,
    ) -> Result<&'a mut PendingShare<T>, VidFragmentError> {
        let pending = self
            .pending
            .entry(view)
            .or_default()
            .entry(disperser.clone())
            .or_insert_with(|| PendingShare {
                epoch,
                payload_commitment: fragment.payload_commitment,
                recipient_key: fragment.recipient_key.clone(),
                param: fragment.param.clone(),
                num_namespaces: fragment.num_namespaces,
                pieces: BTreeMap::new(),
            });
        if pending.num_namespaces != fragment.num_namespaces
            || pending.epoch != epoch
            || pending.payload_commitment != fragment.payload_commitment
            || pending.recipient_key != fragment.recipient_key
            || pending.param != fragment.param
        {
            return Err(VidFragmentError::Inconsistent);
        }
        Ok(pending)
    }

    /// Remove `disperser`'s buffer for `view`, dropping the view's map once it
    /// holds no other disperser.
    fn take_pending(&mut self, view: ViewNumber, disperser: &T::SignatureKey) -> PendingShare<T> {
        let by_disperser = self
            .pending
            .get_mut(&view)
            .expect("buffer just opened above");
        let pending = by_disperser
            .remove(disperser)
            .expect("buffer just opened above");
        if by_disperser.is_empty() {
            self.pending.remove(&view);
        }
        pending
    }
}

/// The fragment's epoch, if the fragment is structurally sound.
///
/// An honest disperser sends `epoch` and `target_epoch` equal. They are
/// separately chosen by whoever sent the fragment and select committees on
/// different paths downstream -- `epoch` the leader and share verification,
/// `target_epoch` the erasure parameters the reconstructor holds every peer's
/// share to -- so a fragment that lets them diverge is malformed.
fn well_formed<T: NodeType>(
    fragment: &AvidmGf2DisperseShareFragment<T>,
) -> Result<EpochNumber, VidFragmentError> {
    if fragment.num_namespaces == 0 {
        return Err(VidFragmentError::Empty);
    }
    if fragment.target_epoch != fragment.epoch {
        return Err(VidFragmentError::TargetEpochMismatch {
            epoch: fragment.epoch,
            target_epoch: fragment.target_epoch,
        });
    }
    fragment.epoch.ok_or(VidFragmentError::MissingEpoch)
}

/// A view's partially-collected namespace pieces, keyed by namespace index.
struct PendingShare<T: NodeType> {
    epoch: EpochNumber,
    payload_commitment: VidCommitment2,
    recipient_key: T::SignatureKey,
    param: AvidmGf2Param,
    num_namespaces: usize,
    pieces: BTreeMap<usize, AvidmGf2NamespacePiece>,
}

impl<T: NodeType> PendingShare<T> {
    fn insert_pieces(
        &mut self,
        pieces: Vec<AvidmGf2NamespacePiece>,
    ) -> Result<(), VidFragmentError> {
        let mut incoming = BTreeSet::new();
        for piece in &pieces {
            let ns_index = piece.ns_index;
            if ns_index >= self.num_namespaces {
                return Err(VidFragmentError::IndexOutOfRange {
                    index: ns_index,
                    num_namespaces: self.num_namespaces,
                });
            }
            if self.pieces.contains_key(&ns_index) || !incoming.insert(ns_index) {
                return Err(VidFragmentError::DuplicateIndex(ns_index));
            }
        }
        for piece in pieces {
            self.pieces.insert(piece.ns_index, piece);
        }
        Ok(())
    }

    fn is_complete(&self) -> bool {
        self.pieces.len() == self.num_namespaces
    }

    /// Assemble the completed share.
    ///
    /// Every namespace is present and indices are distinct and in range, so
    /// they cover `0..num_namespaces` exactly; the `BTreeMap` yields them in
    /// that order.
    fn into_share(self, view: ViewNumber) -> VidDisperseShare2<T> {
        let mut ns_commits = Vec::with_capacity(self.num_namespaces);
        let mut ns_lens = Vec::with_capacity(self.num_namespaces);
        let mut ns_shares = Vec::with_capacity(self.num_namespaces);
        for piece in self.pieces.into_values() {
            ns_commits.push(piece.ns_commit);
            ns_lens.push(piece.ns_payload_byte_len);
            ns_shares.push(piece.ns_share);
        }
        VidDisperseShare2 {
            view_number: view,
            epoch: Some(self.epoch),
            // Equal to `epoch` by `well_formed`; reusing it is what keeps the
            // two from diverging in the assembled share.
            target_epoch: Some(self.epoch),
            payload_commitment: self.payload_commitment,
            share: ns_shares.into(),
            recipient_key: self.recipient_key,
            common: AvidmGf2Common {
                param: self.param,
                ns_commits,
                ns_lens,
            },
        }
    }
}
