// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! The election trait, used to decide which node is the leader and determine if a vote is valid.
//!
//! Reads of per-epoch state go through a [`MembershipSnapshot`] obtained from
//! [`Membership::snapshot`]. Reads of pre-epoch state go through a
//! [`NonEpochMembershipSnapshot`] obtained from [`Membership::non_epoch_snapshot`].
//! Each snapshot is a consistent point-in-time view; its accessors observe
//! one moment of the membership state, so derived values from the same
//! snapshot are guaranteed to come from one logical instant.

use std::fmt::Debug;

use alloy::primitives::U256;
use committable::{Commitment, Committable};
use hotshot_utils::anytrace;

use super::node_implementation::NodeType;
use crate::{
    PeerConfig,
    data::{EpochNumber, Leaf2, ViewNumber},
    drb::DrbResult,
    stake_table::supermajority_threshold,
    traits::signature_key::StakeTableEntryType,
};

pub struct NoStakeTableHash;

impl Committable for NoStakeTableHash {
    fn commit(&self) -> Commitment<Self> {
        Commitment::from_raw([0u8; 32])
    }
}

/// A consistent per-epoch view of a [`Membership`].
pub trait MembershipSnapshot<T: NodeType>: Clone + Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;
    type StakeTableHash: Committable;

    /// The epoch this snapshot is bound to.
    fn epoch(&self) -> EpochNumber;

    /// The first epoch known to the membership (cached at snapshot creation).
    fn first_epoch(&self) -> Option<EpochNumber>;

    /// Whether a randomized stake table (DRB result) was available for this
    /// epoch at the time the snapshot was captured.
    fn has_drb(&self) -> bool;

    /// The (non-DA) stake table for this epoch.
    ///
    /// Iteration order is the stake-table position order — the position
    /// of a key in this iterator equals the value returned by the
    /// implementation's `get_validator_index` (where applicable) and is
    /// part of the per-epoch consensus contract.
    fn stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<T>> + Send;

    /// The DA stake table for this epoch.
    fn da_stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<T>> + Send;

    /// The set of public keys with stake in this epoch, in stake-table order.
    fn committee_members(
        &self,
        view: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &T::SignatureKey> + Send;

    /// The set of public keys in the DA committee for this epoch.
    fn da_committee_members(
        &self,
        view: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &T::SignatureKey> + Send;

    /// The stake-table entry for `key`, or `None` if the key is not in the
    /// committee for this epoch.
    fn stake(&self, key: &T::SignatureKey) -> Option<PeerConfig<T>>;

    /// The DA stake-table entry for `key`, or `None`.
    fn da_stake(&self, key: &T::SignatureKey) -> Option<PeerConfig<T>>;

    /// Whether `key` has stake in this epoch.
    fn has_stake(&self, key: &T::SignatureKey) -> bool;

    /// Whether `key` has DA stake in this epoch.
    fn has_da_stake(&self, key: &T::SignatureKey) -> bool;

    /// The leader for `view` in this epoch.
    ///
    /// # Errors
    ///
    /// Returns an error if the leader cannot be calculated.
    fn lookup_leader(&self, view: ViewNumber) -> Result<T::SignatureKey, Self::Error>;

    /// The commitment of the stake table for this epoch, if available.
    fn stake_table_hash(&self) -> Option<Commitment<Self::StakeTableHash>> {
        None
    }

    /// Number of members in this epoch's stake table.
    fn total_nodes(&self) -> usize {
        self.stake_table().len()
    }

    /// Number of members in this epoch's DA committee.
    fn da_total_nodes(&self) -> usize {
        self.da_stake_table().len()
    }

    /// Sum of all stake in this epoch.
    fn total_stake(&self) -> U256 {
        self.stake_table()
            .fold(U256::ZERO, |acc, e| acc + e.stake_table_entry.stake())
    }

    /// Sum of all DA stake in this epoch.
    fn total_da_stake(&self) -> U256 {
        self.da_stake_table()
            .fold(U256::ZERO, |acc, e| acc + e.stake_table_entry.stake())
    }

    /// Quorum (supermajority) threshold for this epoch.
    fn success_threshold(&self) -> U256 {
        supermajority_threshold(self.total_stake())
    }

    /// DA quorum threshold for this epoch.
    fn da_success_threshold(&self) -> U256 {
        let total = self.total_da_stake();
        let one = U256::ONE;
        let two = U256::from(2);
        let three = U256::from(3);
        if total < U256::MAX / two {
            ((total * two) / three) + one
        } else {
            ((total / three) * two) + two
        }
    }

    /// Failure threshold (1/3 + 1) for this epoch.
    fn failure_threshold(&self) -> U256 {
        let total = self.total_stake();
        (total / U256::from(3)) + U256::ONE
    }

    /// Threshold required for a protocol upgrade.
    fn upgrade_threshold(&self) -> U256 {
        let total = self.total_stake();
        let nine = U256::from(9);
        let ten = U256::from(10);
        let normal = self.success_threshold();
        let higher = if total < U256::MAX / nine {
            (total * nine) / ten
        } else {
            (total / ten) * nine
        };
        std::cmp::max(higher, normal)
    }

    /// The leader for `view` in this epoch, returning a HotShot-internal
    /// error type. Default impl wraps [`Self::lookup_leader`].
    fn leader(&self, view: ViewNumber) -> anytrace::Result<T::SignatureKey> {
        use hotshot_utils::anytrace::*;
        let epoch = self.epoch();
        self.lookup_leader(view).wrap().context(info!(
            "Failed to get leader for view {view} in epoch {epoch}"
        ))
    }
}

/// A consistent view of the pre-epoch [`Membership`] state.
///
/// Used when consensus is operating before epochs are enabled (the
/// `epoch == None` path in the legacy API).
pub trait NonEpochMembershipSnapshot<T: NodeType>: Clone + Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    fn stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<T>> + Send + '_;
    fn da_stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<T>> + Send + '_;
    fn committee_members(
        &self,
        view: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &T::SignatureKey> + Send + '_;
    fn da_committee_members(
        &self,
        view: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &T::SignatureKey> + Send + '_;
    fn stake(&self, key: &T::SignatureKey) -> Option<PeerConfig<T>>;
    fn da_stake(&self, key: &T::SignatureKey) -> Option<PeerConfig<T>>;
    fn has_stake(&self, key: &T::SignatureKey) -> bool;
    fn has_da_stake(&self, key: &T::SignatureKey) -> bool;
    fn lookup_leader(&self, view: ViewNumber) -> Result<T::SignatureKey, Self::Error>;

    fn total_nodes(&self) -> usize {
        self.stake_table().len()
    }

    fn da_total_nodes(&self) -> usize {
        self.da_stake_table().len()
    }

    fn total_stake(&self) -> U256 {
        self.stake_table()
            .fold(U256::ZERO, |acc, e| acc + e.stake_table_entry.stake())
    }

    fn total_da_stake(&self) -> U256 {
        self.da_stake_table()
            .fold(U256::ZERO, |acc, e| acc + e.stake_table_entry.stake())
    }

    fn success_threshold(&self) -> U256 {
        supermajority_threshold(self.total_stake())
    }

    fn da_success_threshold(&self) -> U256 {
        let total = self.total_da_stake();
        let one = U256::ONE;
        let two = U256::from(2);
        let three = U256::from(3);
        if total < U256::MAX / two {
            ((total * two) / three) + one
        } else {
            ((total / three) * two) + two
        }
    }

    fn failure_threshold(&self) -> U256 {
        let total = self.total_stake();
        (total / U256::from(3)) + U256::ONE
    }

    fn upgrade_threshold(&self) -> U256 {
        let total = self.total_stake();
        let nine = U256::from(9);
        let ten = U256::from(10);
        let normal = self.success_threshold();
        let higher = if total < U256::MAX / nine {
            (total * nine) / ten
        } else {
            (total / ten) * nine
        };
        std::cmp::max(higher, normal)
    }

    fn leader(&self, view: ViewNumber) -> anytrace::Result<T::SignatureKey> {
        use hotshot_utils::anytrace::*;
        self.lookup_leader(view)
            .wrap()
            .context(info!("Failed to get leader for view {view} (non-epoch)"))
    }
}

/// A protocol for determining membership in and participating in a committee.
///
/// All read access goes through one of two snapshot types:
/// - [`Self::snapshot`] for per-epoch reads
/// - [`Self::non_epoch_snapshot`] for pre-epoch reads
///
/// Each snapshot is a consistent point-in-time view; derived values read from
/// the same snapshot are guaranteed to come from one logical moment.
pub trait Membership<T: NodeType>: Debug + Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// A consistent per-epoch view, returned by [`Self::snapshot`].
    type Snapshot: MembershipSnapshot<T, Error = Self::Error>;

    /// A consistent pre-epoch view, returned by [`Self::non_epoch_snapshot`].
    type NonEpochSnapshot: NonEpochMembershipSnapshot<T, Error = Self::Error>;

    /// Capture a consistent per-epoch view.
    ///
    /// Returns `None` if no committee is loaded for `epoch`.
    fn snapshot(&self, epoch: EpochNumber) -> Option<Self::Snapshot>;

    /// Capture a consistent pre-epoch view.
    fn non_epoch_snapshot(&self) -> Self::NonEpochSnapshot;

    /// Get first epoch if epochs are enabled, `None` otherwise.
    fn first_epoch(&self) -> Option<EpochNumber>;

    /// Get the highest epoch for which a stake table is currently in memory,
    /// or `None` if no stake tables are loaded. Used at startup to find the
    /// point from which to walk forward catching up missing epochs.
    fn highest_known_epoch(&self) -> Option<EpochNumber> {
        None
    }

    /// Gets the validated block header and epoch number of the epoch root
    /// at the given block height.
    fn get_epoch_root(
        &self,
        e: EpochNumber,
    ) -> impl Future<Output = Result<Leaf2<T>, Self::Error>> + Send;

    /// Gets the DRB result for the given epoch.
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
    fn add_drb_result(&self, e: EpochNumber, d: DrbResult);

    /// Called to notify the Membership that Epochs are enabled.
    /// Implementations should copy the pre-epoch stake table into epoch and epoch+1
    /// when this is called. The value of initial_drb_result should be used for DRB
    /// calculations for epochs (epoch+1) and earlier.
    fn set_first_epoch(&self, e: EpochNumber, r: DrbResult);

    /// Register a DA committee that takes effect starting at `first_epoch`.
    fn add_da_committee(&self, first_epoch: EpochNumber, da_committee: Vec<PeerConfig<T>>);
}
