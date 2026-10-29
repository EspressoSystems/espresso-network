//! The LC contract's voting stake table and the candidate table that matches it.

use std::ops::RangeInclusive;

use hotshot_types::{
    PeerConfig, data::EpochNumber, epoch_membership::EpochMembershipCoordinator,
    stake_table::HSStakeTable,
};
use indexmap::IndexMap;
use thiserror::Error;
use vbs::version::Version;

use super::AuthenticatedValidatorMap;
use crate::{
    PubKey, SeqTypes,
    v0_3::{EventKey, StakeTableError, StakeTableEvent},
};

/// The circuit commitment of a stake table. Aliased because `StakeTableState` in this crate is the
/// L1 validator state.
pub type StakeTableCommitment = hotshot_types::light_client::StakeTableState;
/// The LC contract's voting stake table, read at one L1 block.
#[derive(Clone, Debug)]
pub struct LcContractVotingStakeTable {
    /// L1 block the getters were called at.
    pub l1_block: u64,
    /// `finalizedState.blockHeight`.
    pub finalized_block_height: u64,
    /// `E'`: epoch whose table `commitment` is.
    pub voting_epoch: EpochNumber,
    /// `votingStakeTableState`: `comm(ST(voting_epoch))`.
    pub commitment: StakeTableCommitment,
}

impl LcContractVotingStakeTable {
    /// `E'`: `epoch(height)`, plus 1 when `height` is an epoch root; `first_epoch` before the first
    /// root. Pure.
    pub fn voting_epoch(
        finalized_block_height: u64,
        epoch_height: u64,
        first_epoch: EpochNumber,
    ) -> EpochNumber {
        let _ = (finalized_block_height, epoch_height, first_epoch);
        todo!()
    }
}

/// Candidate: active set whose commitment equals the LC contract's for `epoch`. Used only to verify
/// the anchor leaf and to dial; never installed.
#[derive(Clone, Debug)]
pub struct LcContractCandidateStakeTable {
    /// Epoch whose table the commitment is.
    pub epoch: EpochNumber,
    /// Latest L1 block at or below the read block whose events give this set.
    pub l1_block: u64,
    /// Version whose selection rules produced the match.
    pub version: Version,
    /// Active validators with connect info as of `l1_block`.
    pub validators: AuthenticatedValidatorMap,
}

impl LcContractCandidateStakeTable {
    /// The latest event block at or below `lc_contract.l1_block` whose active set, under one of
    /// `candidate_versions`, commits to `lc_contract.commitment`. Pure. Uses
    /// `StakeTableState::apply_event` and `select_active_validator_set`, both visible here.
    pub fn find(
        lc_contract: &LcContractVotingStakeTable,
        events: &[(EventKey, StakeTableEvent)],
        candidate_versions: &[Version],
        stake_table_capacity: usize,
    ) -> Result<Self, LcContractStakeTableMatchError> {
        let _ = (
            lc_contract,
            events,
            candidate_versions,
            stake_table_capacity,
        );
        todo!()
    }

    /// Entries in committee order.
    pub fn peer_configs(&self) -> IndexMap<PubKey, PeerConfig<SeqTypes>> {
        todo!()
    }

    /// Stake table of `epoch` that verifies its certificates: the installed table, else this
    /// candidate when `epoch == self.epoch`. Pure given its inputs.
    ///
    /// Reads installed tables through `membership().snapshot(epoch)`, which is public and never
    /// starts catchup; never `stake_table_for_epoch` or `membership_for_epoch`, which start HTTP
    /// catchup. The caller derives entries with `StakeTableEntries::from` and the threshold with
    /// `supermajority_threshold(total_stakes())`.
    pub fn quorum_for(
        &self,
        epoch: EpochNumber,
        membership: &EpochMembershipCoordinator<SeqTypes>,
    ) -> Option<HSStakeTable<SeqTypes>> {
        let _ = (epoch, membership);
        todo!()
    }
}

/// Why `find` produced no candidate.
#[derive(Debug, Error)]
pub enum LcContractStakeTableMatchError {
    /// No L1 block and version reproduce the LC contract commitment.
    #[error(
        "no stake table for epoch {epoch} matches the LC contract commitment in L1 blocks \
         {searched_l1_blocks:?}"
    )]
    NoLcContractCommitmentMatch {
        epoch: EpochNumber,
        searched_l1_blocks: RangeInclusive<u64>,
    },
    /// Applying an L1 event or selecting the active set failed fatally.
    #[error(transparent)]
    StakeTable(#[from] StakeTableError),
    /// The commitment of a candidate could not be computed (over capacity).
    #[error("failed to compute the stake table commitment")]
    Commitment { source: anyhow::Error },
}
