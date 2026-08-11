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
    pending: BTreeMap<ViewNumber, PendingShare<T>>,
    completed: BTreeSet<ViewNumber>,
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

impl<T: NodeType> PendingShare<T> {
    /// Whether `fragment` carries pieces of the same share this view is already
    /// collecting. The first fragment pins these fields; later ones must agree.
    fn describes_same_share_as(&self, fragment: &AvidmGf2DisperseShareFragment<T>) -> bool {
        self.num_namespaces == fragment.num_namespaces
            && self.epoch == fragment.epoch
            && self.target_epoch == fragment.target_epoch
            && self.payload_commitment == fragment.payload_commitment
            && self.recipient_key == fragment.recipient_key
            && self.param == fragment.param
    }
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
            completed: BTreeSet::new(),
            lower_bound: ViewNumber::genesis(),
        }
    }

    /// Buffer a `fragment` addressed to this node.
    ///
    /// Returns `Ok(None)` while namespaces are still outstanding,
    /// `Ok(Some(share))` once the final namespace completes the view, and
    /// `Err` if the fragment is malformed or inconsistent with the view's
    /// already-pinned metadata.
    ///
    /// A rejected fragment leaves the view exactly as it found it. Admitting
    /// part of a fragment before rejecting the rest would strand the pieces it
    /// did insert - and, for the view's first fragment, pin its metadata - so
    /// every honest fragment that followed would be turned away as a duplicate
    /// or as inconsistent, and the view's share could never complete.
    pub fn accept(
        &mut self,
        fragment: AvidmGf2DisperseShareFragment<T>,
    ) -> Result<Option<VidDisperseShare2<T>>, VidFragmentError> {
        let view = fragment.view_number;
        if view < self.lower_bound || self.completed.contains(&view) {
            return Ok(None);
        }
        if fragment.num_namespaces == 0 {
            return Err(VidFragmentError::Empty);
        }
        let existing = self.pending.get(&view);
        if let Some(pending) = existing
            && !pending.describes_same_share_as(&fragment)
        {
            return Err(VidFragmentError::Inconsistent);
        }
        // Validate every piece before inserting any of them.
        let mut incoming = BTreeSet::new();
        for piece in &fragment.namespaces {
            let ns_index = piece.ns_index;
            if ns_index >= fragment.num_namespaces {
                return Err(VidFragmentError::IndexOutOfRange {
                    index: ns_index,
                    num_namespaces: fragment.num_namespaces,
                });
            }
            let buffered = existing.is_some_and(|p| p.pieces.contains_key(&ns_index));
            if buffered || !incoming.insert(ns_index) {
                return Err(VidFragmentError::DuplicateIndex(ns_index));
            }
        }

        let pending = self.pending.entry(view).or_insert_with(|| PendingShare {
            epoch: fragment.epoch,
            target_epoch: fragment.target_epoch,
            payload_commitment: fragment.payload_commitment,
            recipient_key: fragment.recipient_key.clone(),
            param: fragment.param.clone(),
            num_namespaces: fragment.num_namespaces,
            pieces: BTreeMap::new(),
        });
        for piece in fragment.namespaces {
            pending.pieces.insert(piece.ns_index, piece);
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

    pub fn gc(&mut self, view_number: ViewNumber) {
        self.pending = self.pending.split_off(&view_number);
        self.completed = self.completed.split_off(&view_number);
        self.lower_bound = view_number;
    }
}
