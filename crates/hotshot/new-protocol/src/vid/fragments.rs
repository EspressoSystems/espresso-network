use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use hotshot_types::{
    data::{
        EpochNumber, VidCommitment2, VidDisperseShare2, ViewNumber,
        vid_disperse::{AvidmGf2DisperseShareFragment, AvidmGf2NamespacePiece},
    },
    traits::node_implementation::NodeType,
    vid::avidm_gf2::{AvidmGf2Common, AvidmGf2Param},
};

#[derive(Default)]
pub struct VidFragmentAccumulator<T: NodeType> {
    pending: BTreeMap<ViewNumber, PendingShare<T>>,
    completed: BTreeSet<ViewNumber>,
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
            return Err(VidFragmentError::Empty);
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
