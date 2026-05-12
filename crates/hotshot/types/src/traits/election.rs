// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! The election trait, used to decide which node is the leader and determine if a vote is valid.
use std::{collections::BTreeSet, fmt::Debug};

use alloy::primitives::U256;
use committable::{Commitment, Committable};
use hotshot_utils::anytrace;

use super::node_implementation::NodeType;
use crate::{
    PeerConfig,
    data::{EpochNumber, Leaf2, ViewNumber},
    drb::DrbResult,
    stake_table::{HSStakeTable, supermajority_threshold},
    traits::signature_key::StakeTableEntryType,
};

pub struct NoStakeTableHash;

impl Committable for NoStakeTableHash {
    fn commit(&self) -> Commitment<Self> {
        Commitment::from_raw([0u8; 32])
    }
}

/// A protocol for determining membership in and participating in a committee.
pub trait Membership<T: NodeType>: Debug + Send + Sync {
    type StakeTableHash: Committable;
    type Error: std::error::Error + Send + Sync + 'static;

    /// Get all participants in the committee (including their stake) for a specific epoch
    fn stake_table(&self, e: Option<EpochNumber>) -> HSStakeTable<T>;

    /// Get all participants in the committee (including their stake) for a specific epoch
    fn da_stake_table(&self, e: Option<EpochNumber>) -> HSStakeTable<T>;

    /// Get all participants in the committee for a specific view for a specific epoch
    fn committee_members(&self, v: ViewNumber, e: Option<EpochNumber>)
    -> BTreeSet<T::SignatureKey>;

    /// Get all participants in the committee for a specific view for a specific epoch
    fn da_committee_members(
        &self,
        v: ViewNumber,
        e: Option<EpochNumber>,
    ) -> BTreeSet<T::SignatureKey>;

    /// Get the stake table entry for a public key, returns `None` if the
    /// key is not in the table for a specific epoch
    fn stake(&self, k: &T::SignatureKey, e: Option<EpochNumber>) -> Option<PeerConfig<T>>;

    /// Get the DA stake table entry for a public key, returns `None` if the
    /// key is not in the table for a specific epoch
    fn da_stake(&self, k: &T::SignatureKey, e: Option<EpochNumber>) -> Option<PeerConfig<T>>;

    /// See if a node has stake in the committee in a specific epoch
    fn has_stake(&self, k: &T::SignatureKey, e: Option<EpochNumber>) -> bool;

    /// See if a node has stake in the committee in a specific epoch
    fn has_da_stake(&self, k: &T::SignatureKey, e: Option<EpochNumber>) -> bool;

    /// The leader of the committee for view `view_number` in `epoch`.
    ///
    /// Note: There is no such thing as a DA leader, so any consumer
    /// requiring a leader should call this.
    ///
    /// # Errors
    /// Returns an error if the leader cannot be calculated
    fn lookup_leader(
        &self,
        v: ViewNumber,
        e: Option<EpochNumber>,
    ) -> Result<T::SignatureKey, Self::Error>;

    /// Returns the number of total nodes in the committee in an epoch `epoch`
    fn total_nodes(&self, e: Option<EpochNumber>) -> usize;

    /// Returns the number of total DA nodes in the committee in an epoch `epoch`
    fn da_total_nodes(&self, e: Option<EpochNumber>) -> usize;

    /// Returns if the stake table is available for the given epoch
    fn has_stake_table(&self, e: EpochNumber) -> bool;

    /// Returns if the randomized stake table is available for the given epoch
    fn has_randomized_stake_table(&self, e: EpochNumber) -> Result<bool, Self::Error>;

    /// Gets the validated block header and epoch number of the epoch root
    /// at the given block height
    fn get_epoch_root(
        &self,
        e: EpochNumber,
    ) -> impl Future<Output = Result<Leaf2<T>, Self::Error>> + Send;

    /// Gets the DRB result for the given epoch
    fn get_epoch_drb(
        &self,
        e: EpochNumber,
    ) -> impl Future<Output = Result<DrbResult, Self::Error>> + Send;

    /// Handles notifications that a new epoch root has been created.
    fn add_epoch_root(
        &self,
        h: T::BlockHeader,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Called to notify the Membership when a new DRB result has been calculated.
    /// Observes the same semantics as add_epoch_root
    fn add_drb_result(&self, e: EpochNumber, d: DrbResult);

    /// Called to notify the Membership that Epochs are enabled.
    /// Implementations should copy the pre-epoch stake table into epoch and epoch+1
    /// when this is called. The value of initial_drb_result should be used for DRB
    /// calculations for epochs (epoch+1) and earlier.
    fn set_first_epoch(&self, e: EpochNumber, r: DrbResult);

    /// Get first epoch if epochs are enabled, `None` otherwise
    fn first_epoch(&self) -> Option<EpochNumber>;

    fn add_da_committee(&self, first_epoch: EpochNumber, da_committee: Vec<PeerConfig<T>>);

    fn total_stake(&self, e: Option<EpochNumber>) -> U256 {
        self.stake_table(e).iter().fold(U256::ZERO, |acc, entry| {
            acc + entry.stake_table_entry.stake()
        })
    }

    fn total_da_stake(&self, e: Option<EpochNumber>) -> U256 {
        self.da_stake_table(e)
            .iter()
            .fold(U256::ZERO, |acc, entry| {
                acc + entry.stake_table_entry.stake()
            })
    }

    /// Returns the threshold for a specific `Membership` implementation
    fn success_threshold(&self, e: Option<EpochNumber>) -> U256 {
        let total_stake = self.total_stake(e);
        supermajority_threshold(total_stake)
    }

    /// Returns the DA threshold for a specific `Membership` implementation
    fn da_success_threshold(&self, e: Option<EpochNumber>) -> U256 {
        let total_stake = self.total_da_stake(e);
        let one = U256::ONE;
        let two = U256::from(2);
        let three = U256::from(3);
        if total_stake < U256::MAX / two {
            ((total_stake * two) / three) + one
        } else {
            ((total_stake / three) * two) + two
        }
    }

    /// Returns the threshold for a specific `Membership` implementation
    fn failure_threshold(&self, e: Option<EpochNumber>) -> U256 {
        let total_stake = self.total_stake(e);
        let one = U256::ONE;
        let three = U256::from(3);
        (total_stake / three) + one
    }

    /// Returns the threshold required to upgrade the network protocol
    fn upgrade_threshold(&self, e: Option<EpochNumber>) -> U256 {
        let total_stake = self.total_stake(e);
        let nine = U256::from(9);
        let ten = U256::from(10);
        let normal_threshold = self.success_threshold(e);
        let higher_threshold = if total_stake < U256::MAX / nine {
            (total_stake * nine) / ten
        } else {
            (total_stake / ten) * nine
        };
        std::cmp::max(higher_threshold, normal_threshold)
    }

    /// Get the highest epoch for which a stake table is currently in memory,
    /// or `None` if no stake tables are loaded. Used at startup to find the
    /// point from which to walk forward catching up missing epochs.
    fn highest_known_epoch(&self) -> Option<EpochNumber> {
        None
    }

    /// Returns the commitment of the stake table for the given epoch,
    /// Errors if the stake table is not available for the given epoch
    fn stake_table_hash(&self, _: EpochNumber) -> Option<Commitment<Self::StakeTableHash>> {
        None
    }

    /// The leader of the committee for view `view_number` in `epoch`.
    ///
    /// Note: this function uses a HotShot-internal error type.
    /// You should implement `lookup_leader`, rather than implementing this function directly.
    ///
    /// # Errors
    /// Returns an error if the leader cannot be calculated.
    fn leader(&self, v: ViewNumber, e: Option<EpochNumber>) -> anytrace::Result<T::SignatureKey> {
        use hotshot_utils::anytrace::*;
        self.lookup_leader(v, e)
            .wrap()
            .context(info!("Failed to get leader for view {v} in epoch {e:?}"))
    }
}
