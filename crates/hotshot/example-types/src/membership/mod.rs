// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! elections used for consensus

use std::sync::Arc;

use async_broadcast::Receiver;
use hotshot_types::{
    PeerConfig,
    event::Event,
    traits::{
        election::Membership, leaf_fetcher_network::LeafFetcherNetwork,
        node_implementation::NodeType,
    },
};

use crate::storage_types::TestStorage;

/// leader completely randomized every view
pub mod randomized_committee;

/// quorum randomized every view, with configurable overlap
pub mod randomized_committee_members;

/// static (round robin) committee election
pub mod static_committee;

/// static (round robin leader for 2 consecutive views) committee election
pub mod static_committee_leader_two_views;
/// two static (round robin) committees for even and odd epochs
pub mod two_static_committees;

/// general helpers
pub mod helpers;

pub mod fetcher;

pub mod stake_table;

pub mod strict_membership;

/// Test-only extension of [`Membership`] that lets tests install the
/// `Leaf2Fetcher` wiring needed for epoch-root catchup. Production
/// memberships don't need this — only types that route catchup through a
/// test fetcher (e.g. `StrictMembership`) implement it.
pub trait TestableMembership<TYPES: NodeType>: Membership<TYPES> {
    /// Construct a membership for tests.
    fn new(
        quorum_members: Vec<PeerConfig<TYPES>>,
        da_members: Vec<PeerConfig<TYPES>>,
        public_key: TYPES::SignatureKey,
        epoch_height: u64,
    ) -> Self;

    /// Install a fully wired leaf fetcher. Must be called before any code
    /// path that triggers catchup (`get_epoch_root` / `get_epoch_drb`).
    fn set_leaf_fetcher(
        &self,
        network: Arc<dyn LeafFetcherNetwork<TYPES>>,
        storage: TestStorage<TYPES>,
        public_key: TYPES::SignatureKey,
        channel: Receiver<Event<TYPES>>,
    );
}
