// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Implementations for examples and tests only
use std::fmt::Debug;

use async_trait::async_trait;
use committable::{Commitment, Committable};
use hotshot_types::{
    data::{fake_commitment, BlockError, Leaf2, ViewNumber},
    traits::{
        block_contents::BlockHeader,
        node_implementation::NodeType,
        states::{InstanceState, StateDelta, TestableState, ValidatedState},
        BlockPayload,
    },
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use vbs::version::Version;

pub use crate::node_types::{TestTwoStakeTablesTypes, TestTypes};
use crate::{
    block_types::{TestBlockPayload, TestTransaction},
    testable_delay::{DelayConfig, SupportedTraitTypesForAsyncDelay, TestableDelay},
};

/// Instance-level state implementation for testing purposes.
#[derive(Clone, Debug, Default)]
pub struct TestInstanceState {
    pub delay_config: DelayConfig,
}

impl InstanceState for TestInstanceState {}

impl TestInstanceState {
    pub fn new(delay_config: DelayConfig) -> Self {
        TestInstanceState { delay_config }
    }
}

/// Application-specific state delta implementation for testing purposes.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TestStateDelta {}

impl StateDelta for TestStateDelta {}

/// Validated state implementation for testing purposes.
#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Debug)]
pub struct TestValidatedState {
    /// the block height
    block_height: u64,
    /// the previous state commitment
    prev_state_commitment: Commitment<Self>,
}

impl Committable for TestValidatedState {
    fn commit(&self) -> Commitment<Self> {
        committable::RawCommitmentBuilder::new("Test State Commit")
            .u64_field("block_height", self.block_height)
            .field("prev_state_commitment", self.prev_state_commitment)
            .finalize()
    }

    fn tag() -> String {
        "TEST_STATE".to_string()
    }
}

impl Default for TestValidatedState {
    fn default() -> Self {
        Self {
            block_height: 0,
            prev_state_commitment: fake_commitment(),
        }
    }
}

#[async_trait]
impl TestableDelay for TestValidatedState {
    async fn run_delay_settings_from_config(delay_config: &DelayConfig) {
        if let Some(settings) =
            delay_config.get_setting(&SupportedTraitTypesForAsyncDelay::ValidatedState)
        {
            Self::handle_async_delay(settings).await;
        }
    }
}

impl<TYPES: NodeType> ValidatedState<TYPES> for TestValidatedState {
    type Error = BlockError;

    type Instance = TestInstanceState;

    type Delta = TestStateDelta;

    type Time = ViewNumber;

    async fn validate_and_apply_header(
        &self,
        instance: &Self::Instance,
        _parent_leaf: &Leaf2<TYPES>,
        _proposed_header: &TYPES::BlockHeader,
        _payload_byte_len: u32,
        _version: Version,
        _view_number: u64,
    ) -> Result<(Self, Self::Delta), Self::Error> {
        Self::run_delay_settings_from_config(&instance.delay_config).await;
        Ok((
            TestValidatedState {
                block_height: self.block_height + 1,
                prev_state_commitment: self.commit(),
            },
            TestStateDelta {},
        ))
    }

    fn from_header(block_header: &TYPES::BlockHeader) -> Self {
        Self {
            block_height: block_header.block_number(),
            ..Default::default()
        }
    }

    fn on_commit(&self) {}

    fn genesis(_instance: &Self::Instance) -> (Self, Self::Delta) {
        (Self::default(), TestStateDelta {})
    }
}

impl<TYPES: NodeType<BlockPayload = TestBlockPayload>> TestableState<TYPES> for TestValidatedState {
    fn create_random_transaction(
        _state: Option<&Self>,
        rng: &mut dyn rand::RngCore,
        padding: u64,
    ) -> <TYPES::BlockPayload as BlockPayload<TYPES>>::Transaction {
        /// clippy appeasement for `RANDOM_TX_BASE_SIZE`
        const RANDOM_TX_BASE_SIZE: usize = 8;

        // Generate a random transaction
        let mut tx = rng.gen::<[u8; RANDOM_TX_BASE_SIZE]>().to_vec();

        // Create and add padding to the transaction
        let padding = vec![0; padding.try_into().expect("transaction padding too large")];
        tx.extend(padding);

        // Return the transaction
        TestTransaction::new(tx)
    }
}
