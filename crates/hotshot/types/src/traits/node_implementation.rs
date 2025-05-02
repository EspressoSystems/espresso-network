// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Composite trait for node behavior
//!
//! This module defines the [`NodeImplementation`] trait, which is a composite trait used for
//! describing the overall behavior of a node, as a composition of implementations of the node trait.

use std::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::{self, Deref, Sub},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use committable::Committable;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::Url;
use vbs::version::StaticVersionType;

use super::{
    auction_results_provider::AuctionResultsProvider,
    block_contents::{BlockHeader, TestableBlock, Transaction},
    network::{
        AsyncGenerator, ConnectedNetwork, NetworkReliability, TestableNetworkingImplementation,
    },
    signature_key::{BuilderSignatureKey, StateSignatureKey},
    states::TestableState,
    storage::Storage,
    ValidatedState,
};
use crate::{
    constants::DEFAULT_UPGRADE_CONSTANTS,
    data::{Leaf2, TestableLeaf},
    traits::{
        election::Membership, signature_key::SignatureKey, states::InstanceState, BlockPayload,
    },
    upgrade_config::UpgradeConstants,
};

/// This trait guarantees that a particular type has urls that can be extracted from it. This trait
/// essentially ensures that the results returned by the [`AuctionResultsProvider`] trait includes a
/// list of urls for the builders that HotShot must request from.
pub trait HasUrls {
    /// Returns the builder url associated with the datatype
    fn urls(&self) -> Vec<Url>;
}

/// Node implementation aggregate trait
///
/// This trait exists to collect multiple behavior implementations into one type, to allow
/// `HotShot` to avoid annoying numbers of type arguments and type patching.
///
/// It is recommended you implement this trait on a zero sized type, as `HotShot`does not actually
/// store or keep a reference to any value implementing this trait.
pub trait NodeImplementation<TYPES: NodeType>:
    Send + Sync + Clone + Eq + Hash + 'static + Serialize + for<'de> Deserialize<'de>
{
    /// The underlying network type
    type Network: ConnectedNetwork<TYPES::SignatureKey>;

    /// Storage for DA layer interactions
    type Storage: Storage<TYPES>;

    /// The auction results type for Solver interactions
    type AuctionResultsProvider: AuctionResultsProvider<TYPES>;
}

/// extra functions required on a node implementation to be usable by hotshot-testing
#[allow(clippy::type_complexity)]
#[async_trait]
pub trait TestableNodeImplementation<TYPES: NodeType>: NodeImplementation<TYPES> {
    /// Creates random transaction if possible
    /// otherwise panics
    /// `padding` is the bytes of padding to add to the transaction
    fn state_create_random_transaction(
        state: Option<&TYPES::ValidatedState>,
        rng: &mut dyn rand::RngCore,
        padding: u64,
    ) -> <TYPES::BlockPayload as BlockPayload<TYPES>>::Transaction;

    /// Creates random transaction if possible
    /// otherwise panics
    /// `padding` is the bytes of padding to add to the transaction
    fn leaf_create_random_transaction(
        leaf: &Leaf2<TYPES>,
        rng: &mut dyn rand::RngCore,
        padding: u64,
    ) -> <TYPES::BlockPayload as BlockPayload<TYPES>>::Transaction;

    /// generate a genesis block
    fn block_genesis() -> TYPES::BlockPayload;

    /// the number of transactions in a block
    fn txn_count(block: &TYPES::BlockPayload) -> u64;

    /// Generate the communication channels for testing
    fn gen_networks(
        expected_node_count: usize,
        num_bootstrap: usize,
        da_committee_size: usize,
        reliability_config: Option<Box<dyn NetworkReliability>>,
        secondary_network_delay: Duration,
    ) -> AsyncGenerator<Arc<Self::Network>>;
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>> TestableNodeImplementation<TYPES> for I
where
    TYPES::ValidatedState: TestableState<TYPES>,
    TYPES::BlockPayload: TestableBlock<TYPES>,
    I::Network: TestableNetworkingImplementation<TYPES>,
{
    fn state_create_random_transaction(
        state: Option<&TYPES::ValidatedState>,
        rng: &mut dyn rand::RngCore,
        padding: u64,
    ) -> <TYPES::BlockPayload as BlockPayload<TYPES>>::Transaction {
        <TYPES::ValidatedState as TestableState<TYPES>>::create_random_transaction(
            state, rng, padding,
        )
    }

    fn leaf_create_random_transaction(
        leaf: &Leaf2<TYPES>,
        rng: &mut dyn rand::RngCore,
        padding: u64,
    ) -> <TYPES::BlockPayload as BlockPayload<TYPES>>::Transaction {
        Leaf2::create_random_transaction(leaf, rng, padding)
    }

    fn block_genesis() -> TYPES::BlockPayload {
        <TYPES::BlockPayload as TestableBlock<TYPES>>::genesis()
    }

    fn txn_count(block: &TYPES::BlockPayload) -> u64 {
        <TYPES::BlockPayload as TestableBlock<TYPES>>::txn_count(block)
    }

    fn gen_networks(
        expected_node_count: usize,
        num_bootstrap: usize,
        da_committee_size: usize,
        reliability_config: Option<Box<dyn NetworkReliability>>,
        secondary_network_delay: Duration,
    ) -> AsyncGenerator<Arc<Self::Network>> {
        <I::Network as TestableNetworkingImplementation<TYPES>>::generator(
            expected_node_count,
            num_bootstrap,
            0,
            da_committee_size,
            reliability_config.clone(),
            secondary_network_delay,
        )
    }
}

/// Trait for time compatibility needed for reward collection
pub trait ConsensusTime:
    PartialOrd
    + Ord
    + Send
    + Sync
    + Debug
    + Clone
    + Copy
    + Hash
    + Deref<Target = u64>
    + serde::Serialize
    + for<'de> serde::Deserialize<'de>
    + ops::AddAssign<u64>
    + ops::Add<u64, Output = Self>
    + Sub<u64, Output = Self>
    + 'static
    + Committable
{
    /// Create a new instance of this time unit at time number 0
    #[must_use]
    fn genesis() -> Self {
        Self::new(0)
    }

    /// Create a new instance of this time unit
    fn new(val: u64) -> Self;

    /// Get the u64 format of time
    fn u64(&self) -> u64;
}

/// Trait with all the type definitions that are used in the current hotshot setup.
pub trait NodeType:
    Clone
    + Copy
    + Debug
    + Hash
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Default
    + serde::Serialize
    + for<'de> Deserialize<'de>
    + Send
    + Sync
    + 'static
{
    /// Constants used to construct upgrade proposals
    const UPGRADE_CONSTANTS: UpgradeConstants = DEFAULT_UPGRADE_CONSTANTS;
    /// The time type that this hotshot setup is using.
    ///
    /// This should be the same `Time` that `ValidatedState::Time` is using.
    type View: ConsensusTime + Display;
    /// Same as above but for epoch.
    type Epoch: ConsensusTime + Display;
    /// The AuctionSolverResult is a type that holds the data associated with a particular solver
    /// run, for a particular view.
    type AuctionResult: Debug
        + HasUrls
        + DeserializeOwned
        + Default
        + PartialEq
        + Eq
        + Clone
        + Send
        + Sync;
    /// The block header type that this hotshot setup is using.
    type BlockHeader: BlockHeader<Self>;
    /// The block type that this hotshot setup is using.
    ///
    /// This should be the same block that `ValidatedState::BlockPayload` is using.
    type BlockPayload: BlockPayload<
        Self,
        Instance = Self::InstanceState,
        Transaction = Self::Transaction,
        ValidatedState = Self::ValidatedState,
    >;
    /// The signature key that this hotshot setup is using.
    type SignatureKey: SignatureKey;
    /// The transaction type that this hotshot setup is using.
    ///
    /// This should be equal to `BlockPayload::Transaction`
    type Transaction: Transaction;

    /// The instance-level state type that this hotshot setup is using.
    type InstanceState: InstanceState;

    /// The validated state type that this hotshot setup is using.
    type ValidatedState: ValidatedState<Self, Instance = Self::InstanceState, Time = Self::View>;

    /// Membership used for this implementation
    type Membership: Membership<Self>;

    /// The type builder uses to sign its messages
    type BuilderSignatureKey: BuilderSignatureKey;

    /// The type replica uses to sign the light client state
    type StateSignatureKey: StateSignatureKey;
}

/// Version information for HotShot
pub trait Versions: Clone + Copy + Debug + Send + Sync + 'static {
    /// The base version of HotShot this node is instantiated with.
    type Base: StaticVersionType;

    /// The version of HotShot this node may be upgraded to. Set equal to `Base` to disable upgrades.
    type Upgrade: StaticVersionType;

    /// The hash for the upgrade.
    const UPGRADE_HASH: [u8; 32];

    /// The version at which to switch over to marketplace logic
    type Marketplace: StaticVersionType;

    /// The version at which to switch over to epochs logic
    type Epochs: StaticVersionType;
}
