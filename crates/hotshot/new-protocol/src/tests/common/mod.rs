pub(crate) mod test_utils;

use std::sync::Arc;

use hotshot::types::BLSPubKey;
use hotshot_example_types::{
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_types::{
    data::{Leaf2, ViewNumber},
    traits::signature_key::SignatureKey,
};
use test_utils::mock_membership;
use tokio::task::JoinHandle;

use crate::{
    Outbox,
    consensus::Consensus,
    coordinator::{handle::CoordinatorHandle, mock::testing::MockCoordinator},
    events::{Action, ConsensusInput, ConsensusOutput, Event},
    validated_state::ValidatedStateManager,
};

/// Test harness that spawns consensus + mock coordinator and provides
/// helpers to send events and collect results.
///
/// All inputs are sent directly as `ConsensusInput` to the mock coordinator.
/// When a `ValidatedStateManager` is wired in, its responses are bridged
/// from `ConsensusOutput::Event` to `ConsensusInput` via a bridge task.
pub(crate) struct TestHarness {
    /// Send ConsensusInput to the mock coordinator
    input_tx: tokio::sync::mpsc::Sender<ConsensusInput<TestTypes>>,
    /// Oneshot to signal shutdown
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    /// Join handle for mock coordinator (collects received events)
    mock_join: JoinHandle<Vec<ConsensusOutput<TestTypes>>>,
}

impl TestHarness {
    /// Create a test harness with the given node index (0-9).
    /// State verification is handled inline by the mock coordinator.
    pub async fn new(node_index: u64) -> Self {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let membership = mock_membership().await;
        let (input_tx, input_rx) = tokio::sync::mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let consensus = Consensus::new(membership.clone(), public_key, private_key);

        let mock_coordinator = MockCoordinator {
            consensus,
            input_rx,
            shutdown_rx,
            state_tx: None,
            membership_coordinator: membership,
            outbox: Outbox::new(),
            received_events: Vec::new(),
        };

        let mock_join = tokio::spawn(async move { mock_coordinator.run().await });

        Self {
            input_tx,
            shutdown_tx: Some(shutdown_tx),
            mock_join,
        }
    }

    /// Create a test harness that wires Consensus and ValidatedStateManager
    /// together through the MockCoordinator.
    pub async fn new_with_state_manager(node_index: u64) -> Self {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let membership = mock_membership().await;
        let (input_tx, input_rx) = tokio::sync::mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let (state_tx, state_rx) = tokio::sync::mpsc::channel(100);

        // The ValidatedStateManager needs a CoordinatorHandle to send responses.
        // We bridge its output channel back to ConsensusInput for the mock.
        let (coordinator_tx, mut coordinator_rx) = tokio::sync::mpsc::channel(100);
        let coordinator_handle = CoordinatorHandle::new(coordinator_tx);
        let bridge_input_tx = input_tx.clone();
        tokio::spawn(async move {
            while let Some(output) = coordinator_rx.recv().await {
                if let ConsensusOutput::Event(event) = output {
                    if let Ok(input) = ConsensusInput::try_from(event) {
                        bridge_input_tx.send(input).await.ok();
                    }
                }
            }
        });

        let mut state_manager = ValidatedStateManager::new(
            state_rx,
            Arc::new(TestInstanceState::default()),
            coordinator_handle,
        );

        let genesis_state = TestValidatedState::default();
        let genesis_leaf = Leaf2::<TestTypes>::genesis(
            &genesis_state,
            &TestInstanceState::default(),
            TEST_VERSIONS.test.base,
        )
        .await;
        state_manager.seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);

        tokio::spawn(async move {
            state_manager.run().await;
        });

        let consensus = Consensus::new(membership.clone(), public_key, private_key);

        let mock_coordinator = MockCoordinator {
            consensus,
            input_rx,
            shutdown_rx,
            state_tx: Some(state_tx),
            membership_coordinator: membership,
            outbox: Outbox::new(),
            received_events: Vec::new(),
        };
        let mock_join = tokio::spawn(async move { mock_coordinator.run().await });

        Self {
            input_tx,
            shutdown_tx: Some(shutdown_tx),
            mock_join,
        }
    }

    /// Send a ConsensusInput to the mock coordinator.
    pub async fn send(&self, input: ConsensusInput<TestTypes>) {
        self.input_tx.send(input).await.unwrap();
    }

    /// Shut down and return all events the mock coordinator collected.
    pub async fn shutdown(mut self) -> Vec<ConsensusOutput<TestTypes>> {
        // Small delay to let async processing complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = self.shutdown_tx.take().unwrap().send(());
        self.mock_join.await.unwrap()
    }
}

// ── ConsensusOutput assertion helpers ──

pub(crate) fn has_vote1(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::SendVote1(_))))
}

pub(crate) fn has_vote2(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::SendVote2(_))))
}

pub(crate) fn has_leaf_decided(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Event(Event::LeafDecided(_))))
}

pub(crate) fn has_request_state(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::RequestState(_))))
}

pub(crate) fn has_proposal(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::SendProposal(..))))
}

pub(crate) fn has_request_block_and_header(events: &[ConsensusOutput<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, ConsensusOutput::Action(Action::RequestBlockAndHeader(_))))
}

pub(crate) fn count_vote1(events: &[ConsensusOutput<TestTypes>]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, ConsensusOutput::Action(Action::SendVote1(_))))
        .count()
}

pub(crate) fn count_vote2(events: &[ConsensusOutput<TestTypes>]) -> usize {
    events
        .iter()
        .filter(|e| matches!(e, ConsensusOutput::Action(Action::SendVote2(_))))
        .count()
}

/// Find the node index (0..10) for a given public key.
pub(crate) fn node_index_for_key(key: &BLSPubKey) -> u64 {
    for i in 0..10 {
        let (pk, _) = BLSPubKey::generated_from_seed_indexed([0; 32], i);
        if pk == *key {
            return i;
        }
    }
    panic!("Key not found in test keys (indices 0..10)");
}
