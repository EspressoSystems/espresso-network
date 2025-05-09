// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! The election trait, used to decide which node is the leader and determine if a vote is valid.
use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

use alloy::primitives::U256;
use async_lock::RwLock;
use hotshot_utils::anytrace::Result;

use super::node_implementation::NodeType;
use crate::{
    data::Leaf2, drb::DrbResult, stake_table::HSStakeTable,
    traits::signature_key::StakeTableEntryType, PeerConfig,
};

/// A protocol for determining membership in and participating in a committee.
pub trait Membership<TYPES: NodeType>: Debug + Send + Sync {
    /// The error type returned by methods like `lookup_leader`.
    type Error: std::fmt::Display;
    /// Create a committee
    fn new(
        // Note: eligible_leaders is currently a hack because the DA leader == the quorum leader
        // but they should not have voting power.
        stake_committee_members: Vec<PeerConfig<TYPES>>,
        da_committee_members: Vec<PeerConfig<TYPES>>,
    ) -> Self;

    fn total_stake(&self, epoch: Option<TYPES::Epoch>) -> U256 {
        self.stake_table(epoch)
            .iter()
            .fold(U256::ZERO, |acc, entry| {
                acc + entry.stake_table_entry.stake()
            })
    }

    fn total_da_stake(&self, epoch: Option<TYPES::Epoch>) -> U256 {
        self.da_stake_table(epoch)
            .iter()
            .fold(U256::ZERO, |acc, entry| {
                acc + entry.stake_table_entry.stake()
            })
    }

    /// Get all participants in the committee (including their stake) for a specific epoch
    fn stake_table(&self, epoch: Option<TYPES::Epoch>) -> HSStakeTable<TYPES>;

    /// Get all participants in the committee (including their stake) for a specific epoch
    fn da_stake_table(&self, epoch: Option<TYPES::Epoch>) -> HSStakeTable<TYPES>;

    /// Get all participants in the committee for a specific view for a specific epoch
    fn committee_members(
        &self,
        view_number: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> BTreeSet<TYPES::SignatureKey>;

    /// Get all participants in the committee for a specific view for a specific epoch
    fn da_committee_members(
        &self,
        view_number: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> BTreeSet<TYPES::SignatureKey>;

    /// Get the stake table entry for a public key, returns `None` if the
    /// key is not in the table for a specific epoch
    fn stake(
        &self,
        pub_key: &TYPES::SignatureKey,
        epoch: Option<TYPES::Epoch>,
    ) -> Option<PeerConfig<TYPES>>;

    /// Get the DA stake table entry for a public key, returns `None` if the
    /// key is not in the table for a specific epoch
    fn da_stake(
        &self,
        pub_key: &TYPES::SignatureKey,
        epoch: Option<TYPES::Epoch>,
    ) -> Option<PeerConfig<TYPES>>;

    /// See if a node has stake in the committee in a specific epoch
    fn has_stake(&self, pub_key: &TYPES::SignatureKey, epoch: Option<TYPES::Epoch>) -> bool;

    /// See if a node has stake in the committee in a specific epoch
    fn has_da_stake(&self, pub_key: &TYPES::SignatureKey, epoch: Option<TYPES::Epoch>) -> bool;

    /// The leader of the committee for view `view_number` in `epoch`.
    ///
    /// Note: this function uses a HotShot-internal error type.
    /// You should implement `lookup_leader`, rather than implementing this function directly.
    ///
    /// # Errors
    /// Returns an error if the leader cannot be calculated.
    fn leader(
        &self,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> Result<TYPES::SignatureKey> {
        use hotshot_utils::anytrace::*;

        self.lookup_leader(view, epoch).wrap().context(info!(
            "Failed to get leader for view {view} in epoch {epoch:?}"
        ))
    }

    /// The leader of the committee for view `view_number` in `epoch`.
    ///
    /// Note: There is no such thing as a DA leader, so any consumer
    /// requiring a leader should call this.
    ///
    /// # Errors
    /// Returns an error if the leader cannot be calculated
    fn lookup_leader(
        &self,
        view: TYPES::View,
        epoch: Option<TYPES::Epoch>,
    ) -> std::result::Result<TYPES::SignatureKey, Self::Error>;

    /// Returns the number of total nodes in the committee in an epoch `epoch`
    fn total_nodes(&self, epoch: Option<TYPES::Epoch>) -> usize;

    /// Returns the number of total DA nodes in the committee in an epoch `epoch`
    fn da_total_nodes(&self, epoch: Option<TYPES::Epoch>) -> usize;

    /// Returns the threshold for a specific `Membership` implementation
    fn success_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256;

    /// Returns the DA threshold for a specific `Membership` implementation
    fn da_success_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256;

    /// Returns the threshold for a specific `Membership` implementation
    fn failure_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256;

    /// Returns the threshold required to upgrade the network protocol
    fn upgrade_threshold(&self, epoch: Option<TYPES::Epoch>) -> U256;

    /// Returns if the stake table is available for the given epoch
    fn has_stake_table(&self, epoch: TYPES::Epoch) -> bool;

    /// Returns if the randomized stake table is available for the given epoch
    fn has_randomized_stake_table(&self, epoch: TYPES::Epoch) -> bool;

    /// Gets the validated block header and epoch number of the epoch root
    /// at the given block height
    fn get_epoch_root(
        _membership: Arc<RwLock<Self>>,
        _block_height: u64,
        _epoch: TYPES::Epoch,
    ) -> impl std::future::Future<Output = anyhow::Result<Leaf2<TYPES>>> + Send {
        async move { anyhow::bail!("Not implemented") }
    }

    /// Gets the DRB result for the given epoch
    fn get_epoch_drb(
        _membership: Arc<RwLock<Self>>,
        _block_height: u64,
        _epoch: TYPES::Epoch,
    ) -> impl std::future::Future<Output = anyhow::Result<DrbResult>> + Send {
        async move { anyhow::bail!("Not implemented") }
    }

    #[allow(clippy::type_complexity)]
    /// Handles notifications that a new epoch root has been created
    /// Is called under a read lock to the Membership. Return a callback
    /// with Some to have that callback invoked under a write lock.
    ///
    /// #3967 REVIEW NOTE: this is only called if epoch is Some. Is there any reason to do otherwise?
    fn add_epoch_root(
        &self,
        _epoch: TYPES::Epoch,
        _block_header: TYPES::BlockHeader,
    ) -> impl std::future::Future<Output = Option<Box<dyn FnOnce(&mut Self) + Send>>> + Send {
        async { None }
    }

    /// Called to notify the Membership when a new DRB result has been calculated.
    /// Observes the same semantics as add_epoch_root
    fn add_drb_result(&mut self, _epoch: TYPES::Epoch, _drb_result: DrbResult);

    /// Called to notify the Membership that Epochs are enabled.
    /// Implementations should copy the pre-epoch stake table into epoch and epoch+1
    /// when this is called. The value of initial_drb_result should be used for DRB
    /// calculations for epochs (epoch+1) and earlier.
    fn set_first_epoch(&mut self, _epoch: TYPES::Epoch, _initial_drb_result: DrbResult);
}
