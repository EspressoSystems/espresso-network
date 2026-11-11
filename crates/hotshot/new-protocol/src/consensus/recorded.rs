//! What the trace recorder reads of consensus beyond its inputs and outputs.
//!
//! Compiled for tests and the `trace` feature only. Nothing here changes
//! consensus: it reads what consensus holds.

use std::collections::BTreeSet;

use hotshot_types::{data::EpochNumber, traits::node_implementation::NodeType};

use super::Consensus;
use crate::{
    helpers::{ViewEpoch, proposal_commitment},
    storage::{ActionKind, StorageOutput},
};

/// What an action consensus holds back is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum HeldKind {
    Vote1,
    Vote2,
    Proposal,
    ReVote,
    Timeout,
}

impl HeldKind {
    /// What a trace calls the kind. A re-vote request is a leader's proposal
    /// for its view.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Vote1 => "vote1",
            Self::Vote2 => "vote2",
            Self::Proposal | Self::ReVote => "proposal",
            Self::Timeout => "timeout",
        }
    }
}

/// An action held back until storage confirms the writes in `waits`, as the
/// trace recorder reads it.
pub(crate) struct HeldBack<T: NodeType> {
    pub(crate) kind: HeldKind,
    pub(crate) key: ViewEpoch,
    pub(crate) waits: Vec<StorageOutput<T>>,
}

impl<T: NodeType> HeldBack<T> {
    fn new(
        kind: HeldKind,
        key: ViewEpoch,
        waits: impl IntoIterator<Item = StorageOutput<T>>,
    ) -> Self {
        Self {
            kind,
            key,
            waits: waits.into_iter().collect(),
        }
    }
}

impl<T: NodeType> Consensus<T> {
    /// The vote1s, vote2s, proposals, re-vote requests and timeout votes this
    /// node holds back, each with the writes storage has yet to confirm
    /// before it is released.
    ///
    /// What each waits for is what its release reads (`release_vote1_at`,
    /// `release_vote2_at`, `release_proposal`, `release_timeout_vote`).
    pub(crate) fn held_back(&self) -> Vec<HeldBack<T>> {
        let action = |view, kind| {
            (!self.stored_actions.contains(&(view, kind)))
                .then_some(StorageOutput::Action(view, kind))
        };
        let vote1 = self.pending_vote1.iter().map(|(key, vote)| {
            let ViewEpoch(view, epoch) = *key;
            let commit = vote.vote.data.leaf_commit;
            let is_revote = self.revotes.get(&view).is_some_and(|revote| {
                revote.epoch == epoch && revote.cert1.data.leaf_commit == commit
            });
            let proposal = (!is_revote && !self.is_proposal_stored(view, &commit))
                .then_some(StorageOutput::Proposal(view, commit));
            let waits = action(view, ActionKind::Vote).into_iter().chain(proposal);
            HeldBack::new(HeldKind::Vote1, *key, waits)
        });
        let vote2 = self.pending_vote2.iter().map(|(key, pending)| {
            let view = key.0;
            let barred = view <= self.restart_barred_view;
            let vote = action(view, ActionKind::Vote).filter(|_| !barred);
            let share =
                (!barred && pending.block_view == view && !self.stored_vids.contains(&view))
                    .then_some(StorageOutput::Vid(view));
            let lock = (!self.high_qc_persisted(pending.lock))
                .then_some(StorageOutput::HighQc(pending.lock));
            HeldBack::new(
                HeldKind::Vote2,
                *key,
                vote.into_iter().chain(share).chain(lock),
            )
        });
        let proposal = self.pending_proposal.iter().map(|(key, message)| {
            let view = key.0;
            let commit = proposal_commitment(&message.data);
            let stored = (!self.is_proposal_stored(view, &commit))
                .then_some(StorageOutput::Proposal(view, commit));
            let waits = action(view, ActionKind::Propose).into_iter().chain(stored);
            HeldBack::new(HeldKind::Proposal, *key, waits)
        });
        let revote = self
            .pending_revote
            .keys()
            .map(|key| HeldBack::new(HeldKind::ReVote, *key, action(key.0, ActionKind::Propose)));
        let timeout = self
            .pending_timeout_vote
            .keys()
            .map(|key| HeldBack::new(HeldKind::Timeout, *key, action(key.0, ActionKind::Timeout)));
        vote1
            .chain(vote2)
            .chain(proposal)
            .chain(revote)
            .chain(timeout)
            .collect()
    }

    /// The epochs this node holds a light-client state certificate for.
    pub(crate) fn state_cert_epochs(&self) -> BTreeSet<EpochNumber> {
        self.state_certs.keys().copied().collect()
    }
}
