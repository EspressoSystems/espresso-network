use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    mem,
    sync::mpsc,
};

use hotshot_types::{
    data::ViewNumber,
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::UpgradeLock,
    simple_certificate::{LightClientStateUpdateCertificateV2, QuorumCertificate2},
    simple_vote::{HasEpoch, QuorumVote2},
    traits::node_implementation::NodeType,
    vote::{HasViewNumber, LightClientStateUpdateVoteAccumulator, Vote},
};
use tokio_util::task::JoinMap;
use tracing::{error, info};

use crate::{message::Vote1, vote::CheckedAccumulator};

/// The pair of certificates formed at an epoch-root view.
type EpochRootCerts<T> = (
    QuorumCertificate2<T>,
    LightClientStateUpdateCertificateV2<T>,
);

/// Collects epoch-root votes and forms the certificate pair for such views.
///
/// An epoch-root [`Vote1`] carries a quorum vote and a light-client state
/// update vote. Per view, both are tallied from the same vote stream — quorum
/// votes into a [`QuorumCertificate2`], state votes into a
/// [`LightClientStateUpdateCertificateV2`] — and the pair is emitted only once
/// both cross their threshold, so consensus never sees an epoch-root quorum
/// certificate without the matching state certificate.
pub struct EpochRootVoteCollector<T: NodeType> {
    /// Tasks collecting votes and verifying certificates.
    accumulators: JoinMap<ViewNumber, Option<EpochRootCerts<T>>>,

    /// Where callers submit their votes.
    ballot_boxes: BTreeMap<ViewNumber, mpsc::Sender<Vote1<T>>>,

    /// Votes for epochs we have yet to resolve.
    pending: BTreeMap<ViewNumber, Vec<Vote1<T>>>,

    /// Views that had valid certificates already.
    completed: BTreeSet<ViewNumber>,

    /// The signers per view.
    signers: BTreeMap<ViewNumber, HashSet<T::SignatureKey>>,

    /// The GC threshold.
    lower_bound: ViewNumber,

    membership: EpochMembershipCoordinator<T>,

    upgrade_lock: UpgradeLock<T>,
}

impl<T: NodeType> EpochRootVoteCollector<T> {
    pub fn new(mc: EpochMembershipCoordinator<T>, lock: UpgradeLock<T>) -> Self {
        Self {
            accumulators: JoinMap::new(),
            ballot_boxes: BTreeMap::new(),
            pending: BTreeMap::new(),
            completed: BTreeSet::new(),
            signers: BTreeMap::new(),
            lower_bound: ViewNumber::genesis(),
            membership: mc,
            upgrade_lock: lock,
        }
    }

    pub async fn next(&mut self) -> Option<EpochRootCerts<T>> {
        loop {
            match self.accumulators.join_next().await {
                Some((view, Ok(Some(certs)))) => {
                    self.ballot_boxes.remove(&view);
                    if view >= self.lower_bound {
                        self.completed.insert(view);
                        return Some(certs);
                    }
                },
                Some((_, Ok(None))) => {},
                Some((view, Err(err))) => {
                    if err.is_panic() {
                        error!(%view, %err, "epoch-root vote collection task panic");
                    }
                },
                None => return None,
            }
        }
    }

    /// Accumulate a `Vote1` for an epoch-root view.
    ///
    /// Caller should have verified `vote1.state_vote.is_some()`.
    pub fn accumulate_vote(&mut self, vote1: Vote1<T>) {
        if vote1.state_vote.is_none() {
            return;
        }

        let view = vote1.view_number();

        if view < self.lower_bound || self.completed.contains(&view) {
            return;
        }

        let Some(membership) = self.resolve_membership(&vote1.vote) else {
            self.pending.entry(view).or_default().push(vote1);
            return;
        };

        // Check that we have not received a vote from this signer already.
        if !self
            .signers
            .entry(view)
            .or_default()
            .insert(vote1.vote.signing_key())
        {
            return;
        }

        if let Some(tx) = self.ballot_boxes.get(&view) {
            let _ = tx.send(vote1);
            return;
        }

        let (tx, rx) = mpsc::channel();

        let _ = tx.send(vote1);
        self.ballot_boxes.insert(view, tx);

        let lock = self.upgrade_lock.clone();
        self.accumulators
            .spawn_blocking(view, move || accumulate_votes(rx, membership, lock));
    }

    pub fn retry_pending_votes(&mut self) {
        for vote in mem::take(&mut self.pending).into_values().flatten() {
            self.accumulate_vote(vote)
        }
    }

    pub fn gc(&mut self, view: ViewNumber) {
        self.ballot_boxes = self.ballot_boxes.split_off(&view);
        self.completed = self.completed.split_off(&view);
        self.pending = self.pending.split_off(&view);
        self.signers = self.signers.split_off(&view);
        self.lower_bound = view;
    }

    fn resolve_membership(&mut self, vote: &QuorumVote2<T>) -> Option<EpochMembership<T>> {
        let epoch = vote.epoch()?;
        self.membership.membership_for_epoch(Some(epoch)).ok()
    }
}

fn accumulate_votes<T: NodeType>(
    rx: mpsc::Receiver<Vote1<T>>,
    membership: EpochMembership<T>,
    lock: UpgradeLock<T>,
) -> Option<EpochRootCerts<T>> {
    let mut quorum_accumulator =
        CheckedAccumulator::<T, QuorumVote2<T>, QuorumCertificate2<T>>::new(
            membership.clone(),
            lock.clone(),
        );
    let mut state_accumulator = LightClientStateUpdateVoteAccumulator::<T> {
        vote_outcomes: HashMap::new(),
        upgrade_lock: lock,
    };

    let mut quorum_cert = None;
    let mut state_cert = None;

    while let Ok(vote1) = rx.recv() {
        let Some(state_vote) = vote1.state_vote else {
            error!(view = %vote1.vote.view_number(), "epoch-root vote1 without state vote");
            continue;
        };
        let bls_key = vote1.vote.signing_key();

        if quorum_cert.is_none() {
            quorum_cert = quorum_accumulator.add(vote1.vote);
        }

        // Unlike quorum votes, state votes are fully checked, including their
        // signatures, by the accumulator, so the certificate does not need to
        // be validated again.
        if state_cert.is_none() {
            state_cert = state_accumulator.accumulate(&bls_key, &state_vote, &membership);
        }

        if let (Some(q), Some(s)) = (&quorum_cert, &state_cert) {
            info!(
                view = %q.view_number(),
                epoch = %s.epoch,
                "epoch-root certificates formed"
            );
            return Some((q.clone(), s.clone()));
        }
    }
    None
}
