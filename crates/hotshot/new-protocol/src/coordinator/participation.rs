use std::collections::HashMap;

use hotshot_types::{
    consensus::{
        ValidatorParticipation, VoteParticipation, resolve_participation_stake_table,
        track_decided_qc_participation,
    },
    data::{EpochNumber, Leaf2},
    epoch_membership::EpochMembershipCoordinator,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
};
use tracing::warn;

pub struct ParticipationTracker<T: NodeType> {
    validator: ValidatorParticipation<T>,
    vote: VoteParticipation<T>,
}

impl<T: NodeType> Default for ParticipationTracker<T> {
    fn default() -> Self {
        let (stake_table, success_threshold) = VoteParticipation::<T>::unresolved_stake_table();
        Self {
            validator: ValidatorParticipation::new(),
            vote: VoteParticipation::new(stake_table, success_threshold, None),
        }
    }
}

impl<T: NodeType> ParticipationTracker<T> {
    pub fn new(membership: &EpochMembershipCoordinator<T>, epoch: EpochNumber) -> Self {
        let (stake_table, success_threshold) =
            resolve_participation_stake_table(membership, Some(epoch));
        Self {
            validator: ValidatorParticipation::new_in_epoch(epoch),
            vote: VoteParticipation::new(stake_table, success_threshold, Some(epoch)),
        }
    }

    pub fn leader_proposed(&mut self, leader: T::SignatureKey, epoch: EpochNumber) {
        self.validator.update_participation(leader, epoch, true);
    }

    pub fn leader_missed(&mut self, leader: T::SignatureKey, epoch: EpochNumber) {
        self.validator.update_participation(leader, epoch, false);
    }

    /// Advance the proposal tracker as soon as the view enters a new epoch,
    /// which runs ahead of the decide chain that otherwise advances it.
    pub fn on_view_changed(&mut self, epoch: EpochNumber) {
        self.validator.update_participation_epoch(epoch);
    }

    /// Call with the oldest decided leaf first.
    pub fn on_leaf_decided(&mut self, leaf: &Leaf2<T>, membership: &EpochMembershipCoordinator<T>) {
        if let Err(err) = track_decided_qc_participation(
            &leaf.justify_qc(),
            membership,
            &mut self.validator,
            &mut self.vote,
        ) {
            warn!(%err, "failed to update vote participation epoch");
        }
    }

    pub fn current_proposal_participation(&self) -> HashMap<T::SignatureKey, f64> {
        self.validator.current_proposal_participation()
    }

    pub fn proposal_participation(&self, epoch: EpochNumber) -> HashMap<T::SignatureKey, f64> {
        self.validator.proposal_participation(epoch)
    }

    pub fn current_vote_participation(
        &self,
    ) -> HashMap<<T::SignatureKey as SignatureKey>::VerificationKeyType, f64> {
        self.vote.current_vote_participation()
    }

    pub fn vote_participation(
        &self,
        epoch: Option<EpochNumber>,
    ) -> HashMap<<T::SignatureKey as SignatureKey>::VerificationKeyType, f64> {
        self.vote.vote_participation(epoch)
    }
}
