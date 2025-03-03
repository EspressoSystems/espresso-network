// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! The election trait, used to decide which node is the leader and determine if a vote is valid.
use std::{collections::BTreeSet, fmt::Debug, num::NonZeroU64};

use async_trait::async_trait;
use hotshot_utils::anytrace::Result;

use super::node_implementation::NodeType;
use crate::{drb::DrbResult, PeerConfig};

#[async_trait]
/// A protocol for determining membership in and participating in a committee.
pub trait Membership<TYPES: NodeType>: Debug + Send + Sync {
    /// The error type returned by methods like `lookup_leader`.
    type Error: std::fmt::Display;
    /// Create a committee
    fn new(
        // Note: eligible_leaders is currently a hack because the DA leader == the quorum leader
        // but they should not have voting power.
        stake_committee_members: Vec<PeerConfig<TYPES::SignatureKey>>,
        da_committee_members: Vec<PeerConfig<TYPES::SignatureKey>>,
    ) -> Self;

    /// Get all participants in the committee (including their stake) for a specific epoch
    fn stake_table(&self, epoch: Option<TYPES::Epoch>) -> Vec<PeerConfig<TYPES::SignatureKey>>;

    /// Get all participants in the committee (including their stake) for a specific epoch
    fn da_stake_table(&self, epoch: Option<TYPES::Epoch>) -> Vec<PeerConfig<TYPES::SignatureKey>>;

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

    /// Get all leaders in the committee for a specific view for a specific epoch
    fn committee_leaders(
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
    ) -> Option<PeerConfig<TYPES::SignatureKey>>;

    /// Get the DA stake table entry for a public key, returns `None` if the
    /// key is not in the table for a specific epoch
    fn da_stake(
        &self,
        pub_key: &TYPES::SignatureKey,
        epoch: Option<TYPES::Epoch>,
    ) -> Option<PeerConfig<TYPES::SignatureKey>>;

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
            "Failed to get leader for view {view} in epoch {epoch}"
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
    fn success_threshold(&self, epoch: Option<TYPES::Epoch>) -> NonZeroU64;

    /// Returns the DA threshold for a specific `Membership` implementation
    fn da_success_threshold(&self, epoch: Option<TYPES::Epoch>) -> NonZeroU64;

    /// Returns the threshold for a specific `Membership` implementation
    fn failure_threshold(&self, epoch: Option<TYPES::Epoch>) -> NonZeroU64;

    /// Returns the threshold required to upgrade the network protocol
    fn upgrade_threshold(&self, epoch: Option<TYPES::Epoch>) -> NonZeroU64;

    #[allow(clippy::type_complexity)]
    /// Handles notifications that a new epoch root has been created
    /// Is called under a read lock to the Membership. Return a callback
    /// with Some to have that callback invoked under a write lock.
    ///
    /// #3967 REVIEW NOTE: this is only called if epoch is Some. Is there any reason to do otherwise?
    async fn add_epoch_root(
        &self,
        _epoch: TYPES::Epoch,
        _block_header: TYPES::BlockHeader,
    ) -> Option<Box<dyn FnOnce(&mut Self) + Send>> {
        None
    }

    /// Called to notify the Membership when a new DRB result has been calculated.
    /// Observes the same semantics as add_epoch_root
    fn add_drb_result(&mut self, _epoch: TYPES::Epoch, _drb_result: DrbResult);
}
