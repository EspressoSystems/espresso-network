use std::collections::HashMap;

use alloy::primitives::U256;
use hotshot_types::{
    consensus::{ValidatorParticipation, VoteParticipation},
    data::{EpochNumber, Leaf2},
    epoch_membership::EpochMembershipCoordinator,
    simple_vote::HasEpoch,
    stake_table::HSStakeTable,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
};
use tracing::warn;

pub struct ParticipationTracker<T: NodeType> {
    validator: ValidatorParticipation<T>,
    vote: VoteParticipation<T>,
}

impl<T: NodeType> Default for ParticipationTracker<T> {
    fn default() -> Self {
        Self {
            validator: ValidatorParticipation::new(),
            vote: VoteParticipation::new(HSStakeTable::default(), U256::MAX, None),
        }
    }
}

impl<T: NodeType> ParticipationTracker<T> {
    pub fn new(membership: &EpochMembershipCoordinator<T>, epoch: EpochNumber) -> Self {
        let (stake_table, success_threshold) = stake_table_for(membership, Some(epoch));
        Self {
            validator: ValidatorParticipation::new(),
            vote: VoteParticipation::new(stake_table, success_threshold, Some(epoch)),
        }
    }

    pub fn leader_proposed(&mut self, leader: T::SignatureKey, epoch: EpochNumber) {
        self.validator.update_participation(leader, epoch, true);
    }

    pub fn leader_missed(&mut self, leader: T::SignatureKey, epoch: EpochNumber) {
        self.validator.update_participation(leader, epoch, false);
    }

    /// Call with the oldest decided leaf first.
    pub fn on_leaf_decided(&mut self, leaf: &Leaf2<T>, membership: &EpochMembershipCoordinator<T>) {
        let qc = leaf.justify_qc();
        let qc_epoch = qc.epoch();
        if let Some(epoch) = qc_epoch
            && epoch > self.validator.current_epoch()
        {
            self.validator.update_participation_epoch(epoch);
        }
        if qc_epoch > self.vote.current_epoch() {
            let (stake_table, success_threshold) = stake_table_for(membership, qc_epoch);
            if let Err(err) =
                self.vote
                    .update_participation_epoch(stake_table, success_threshold, qc_epoch)
            {
                warn!(%err, "failed to update vote participation epoch");
            }
        }
        if let Err(err) = self.vote.update_participation(qc) {
            warn!(%err, "failed to update vote participation");
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

fn stake_table_for<T: NodeType>(
    membership: &EpochMembershipCoordinator<T>,
    epoch: Option<EpochNumber>,
) -> (HSStakeTable<T>, U256) {
    match membership.stake_table_for_epoch(epoch) {
        Ok(m) => (
            HSStakeTable::from_iter(m.stake_table()),
            m.success_threshold(),
        ),
        Err(err) => {
            warn!(?epoch, %err, "no stake table for participation tracking");
            (HSStakeTable::default(), U256::MAX)
        },
    }
}
