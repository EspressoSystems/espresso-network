use std::{any::type_name, mem};

use committable::Committable;
use hotshot::types::SignatureKey;
use hotshot_types::{
    epoch_membership::EpochMembership,
    message::UpgradeLock,
    simple_vote::{HasEpoch, VersionedVoteData},
    stake_table::StakeTableEntries,
    traits::node_implementation::NodeType,
    vote::{Certificate, Vote, VoteAccumulator},
};
use hotshot_utils::anytrace;
use tracing::{info, warn};

/// A [`VoteAccumulator`] that validates certificates before emitting them.
///
/// Votes are accumulated without checking their signatures. If the resulting
/// certificate is invalid, votes with invalid signatures are discarded, the
/// remaining ones are accumulated again, and every subsequent vote is verified
/// before it is accumulated.
pub struct CheckedAccumulator<T, V, C>
where
    T: NodeType,
    V: Vote<T>,
    C: Certificate<T, V::Commitment, Voteable = V::Commitment> + HasEpoch,
{
    accumulator: VoteAccumulator<T, V, C>,

    /// Votes accumulated so far, kept for recovery from an invalid certificate.
    votes: Vec<V>,

    /// Verify votes before accumulating them?
    ///
    /// Set after an invalid certificate.
    verify_votes: bool,

    membership: EpochMembership<T>,

    upgrade_lock: UpgradeLock<T>,
}

impl<T, V, C> CheckedAccumulator<T, V, C>
where
    T: NodeType,
    V: Vote<T>,
    C: Certificate<T, V::Commitment, Voteable = V::Commitment> + HasEpoch,
{
    pub fn new(membership: EpochMembership<T>, lock: UpgradeLock<T>) -> Self {
        Self {
            accumulator: VoteAccumulator::new(lock.clone()),
            votes: Vec::new(),
            verify_votes: false,
            membership,
            upgrade_lock: lock,
        }
    }

    /// Accumulate a vote.
    ///
    /// Returns a valid certificate once enough votes have been collected.
    pub fn add(&mut self, vote: V) -> Option<C> {
        if self.verify_votes {
            if !self.is_valid_vote(&vote) {
                return None;
            }
            let cert = self
                .accumulator
                .accumulate(&vote, self.membership.clone())?;
            debug_assert!(self.validate(&cert).is_ok());
            info!(view = %cert.view_number(), cert = type_name::<C>(), "certificate formed");
            return Some(cert);
        }

        let Some(cert) = self.accumulator.accumulate(&vote, self.membership.clone()) else {
            self.votes.push(vote);
            return None;
        };

        match self.validate(&cert) {
            Ok(()) => {
                info!(view = %cert.view_number(), cert = type_name::<C>(), "certificate formed");
                Some(cert)
            },
            Err(err) => {
                warn!(view = %cert.view_number(), %err, "invalid certificate formed");
                self.votes.push(vote);
                self.recover()
            },
        }
    }

    /// Discard votes with invalid signatures and accumulate the rest again.
    fn recover(&mut self) -> Option<C> {
        self.verify_votes = true;
        self.accumulator.clear();
        for vote in mem::take(&mut self.votes) {
            if !self.is_valid_vote(&vote) {
                continue;
            }
            if let Some(cert) = self.accumulator.accumulate(&vote, self.membership.clone()) {
                debug_assert!(self.validate(&cert).is_ok());
                info!(view = %cert.view_number(), cert = type_name::<C>(), "certificate formed");
                return Some(cert);
            }
        }
        None
    }

    /// Check the certificate's aggregate signature against the stake table.
    fn validate(&self, cert: &C) -> anytrace::Result<()> {
        let table = StakeTableEntries::from(C::stake_table(&self.membership));
        let thresh = C::threshold(&self.membership);
        cert.is_valid_cert(&table.0, thresh, &self.upgrade_lock)
    }

    /// Check the vote's signature.
    fn is_valid_vote(&self, vote: &V) -> bool {
        let commit = match VersionedVoteData::new(
            vote.date().clone(),
            vote.view_number(),
            &self.upgrade_lock,
        ) {
            Ok(data) => data.commit(),
            Err(err) => {
                warn!(%err, "failed to generate versioned vote data");
                return false;
            },
        };
        let valid = vote
            .signing_key()
            .validate(&vote.signature(), commit.as_ref());
        if !valid {
            warn!(
                view = %vote.view_number(),
                cert = type_name::<C>(),
                signer = %vote.signing_key(),
                "invalid vote"
            );
        }
        valid
    }
}
