// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! The election trait, used to decide which node is the leader and determine if a vote is valid.
//!
//! Reads of per-epoch state go through a [`MembershipSnapshot`] obtained from
//! [`Membership::snapshot`]. Each snapshot is a consistent point-in-time view;
//! its accessors observe one moment of the membership state, so derived values
//! from the same snapshot are guaranteed to come from one logical instant.
//! Reads of the views before the first epoch go through the
//! [`GenesisStakeTables`] from [`Membership::genesis_stake_tables`].

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use alloy_primitives::U256;
use committable::{Commitment, Committable};
use hotshot_utils::anytrace;

use super::node_implementation::NodeType;
use crate::{
    PeerConfig,
    data::{EpochNumber, Leaf2, ViewNumber},
    drb::DrbResult,
    epoch_membership::EpochMembershipCoordinator,
    stake_table::{one_honest_threshold, supermajority_threshold, upgrade_threshold},
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

    /// The first epoch of the membership.
    fn first_epoch(&self) -> EpochNumber;

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
        supermajority_threshold(self.total_da_stake())
    }

    /// Failure threshold (1/3 + 1) for this epoch.
    fn failure_threshold(&self) -> U256 {
        one_honest_threshold(self.total_stake())
    }

    /// Threshold required for a protocol upgrade.
    fn upgrade_threshold(&self) -> U256 {
        upgrade_threshold(self.total_stake())
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

/// The stake tables of views before the first epoch, when epochs were not
/// enabled yet: the genesis stake table and the genesis DA committee.
///
/// Clones share the tables, so cloning is cheap.
#[derive(Debug, Clone)]
pub struct GenesisStakeTables<T: NodeType> {
    tables: Arc<Tables<T>>,
}

impl<T: NodeType> GenesisStakeTables<T> {
    /// Keeps both tables as given, in order.
    ///
    /// # Errors
    ///
    /// Returns an error if a table holds a key twice.
    pub fn new(
        stake_table: Vec<PeerConfig<T>>,
        da_stake_table: Vec<PeerConfig<T>>,
    ) -> anyhow::Result<Self> {
        let stake_index = index_keys(&stake_table, "stake table")?;
        index_keys(&da_stake_table, "DA committee")?;
        Ok(Self {
            tables: Arc::new(Tables {
                stake_table,
                da_stake_table,
                stake_index,
            }),
        })
    }

    pub fn stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<T>> + Send + '_ {
        self.tables.stake_table.iter()
    }

    pub fn da_stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<T>> + Send + '_ {
        self.tables.da_stake_table.iter()
    }

    pub fn stake(&self, key: &T::SignatureKey) -> Option<PeerConfig<T>> {
        let &position = self.tables.stake_index.get(key)?;
        Some(self.tables.stake_table[position].clone())
    }

    pub fn success_threshold(&self) -> U256 {
        supermajority_threshold(
            self.tables
                .stake_table
                .iter()
                .map(|p| p.stake_table_entry.stake())
                .sum(),
        )
    }
}

#[derive(Debug)]
struct Tables<T: NodeType> {
    stake_table: Vec<PeerConfig<T>>,
    da_stake_table: Vec<PeerConfig<T>>,
    /// The position of each key in `stake_table`.
    stake_index: HashMap<T::SignatureKey, usize>,
}

/// The position of each key in `table`, the genesis `name`.
///
/// # Errors
///
/// Returns an error if `table` holds a key twice.
fn index_keys<T: NodeType>(
    table: &[PeerConfig<T>],
    name: &str,
) -> anyhow::Result<HashMap<T::SignatureKey, usize>> {
    let mut index = HashMap::with_capacity(table.len());
    for (position, peer) in table.iter().enumerate() {
        let key = peer.stake_table_entry.public_key();
        if index.insert(key.clone(), position).is_some() {
            anyhow::bail!("the genesis {name} holds the key {key} twice");
        }
    }
    Ok(index)
}

/// A protocol for determining membership in and participating in a committee.
///
/// Per-epoch reads go through [`Self::snapshot`], and reads of the views
/// before the first epoch through [`Self::genesis_stake_tables`].
///
/// Each snapshot is a consistent point-in-time view; derived values read from
/// the same snapshot are guaranteed to come from one logical moment.
pub trait Membership<T: NodeType>: Debug + Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// A consistent per-epoch view, returned by [`Self::snapshot`].
    type Snapshot: MembershipSnapshot<T, Error = Self::Error>;

    /// Capture a consistent per-epoch view.
    ///
    /// Returns `None` if no committee is loaded for `epoch`.
    fn snapshot(&self, epoch: EpochNumber) -> Option<Self::Snapshot>;

    /// The stake tables of views before the first epoch.
    fn genesis_stake_tables(&self) -> GenesisStakeTables<T>;

    /// The first epoch, which runs on the genesis stake table.
    fn first_epoch(&self) -> EpochNumber;

    /// Get the highest epoch for which a stake table is currently in memory,
    /// or `None` if no stake tables are loaded. It is where catchup resumes
    /// from, so an implementation that leaves it at `None` makes catchup
    /// rediscover that point one epoch at a time.
    fn highest_known_epoch(&self) -> Option<EpochNumber>;

    /// Gets the validated block header and epoch number of the epoch root
    /// at the given block height.
    fn get_epoch_root(
        &self,
        e: EpochNumber,
        coordinator: &EpochMembershipCoordinator<T>,
    ) -> impl Future<Output = Result<Leaf2<T>, Self::Error>> + Send;

    /// Gets the DRB result for the given epoch.
    fn get_epoch_drb(
        &self,
        e: EpochNumber,
        coordinator: &EpochMembershipCoordinator<T>,
    ) -> impl Future<Output = Result<DrbResult, Self::Error>> + Send;

    /// Handles notifications that a new epoch root has been created.
    fn add_epoch_root(
        &self,
        h: T::BlockHeader,
        coordinator: &EpochMembershipCoordinator<T>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    /// Called to notify the Membership when a new DRB result has been calculated.
    fn add_drb_result(&self, e: EpochNumber, d: DrbResult);

    /// Load the committee for `epoch` from local storage.
    ///
    /// Returns `true` if a snapshot for `epoch` is available afterwards.
    fn load_stake_table(&self, epoch: EpochNumber) -> impl Future<Output = bool> + Send;

    /// Register a DA committee that takes effect starting at `first_epoch`.
    fn add_da_committee(&self, first_epoch: EpochNumber, da_committee: Vec<PeerConfig<T>>);
}
