mod validated_state_and_consensus;

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
use tokio::{sync::mpsc::Sender, task::JoinHandle};

use crate::{
    consensus::Consensus,
    coordinator::{handle::CoordinatorHandle, mock::testing::MockCoordinator},
    events::{Action, ConsensusEvent, Event, RequestMessageSender, Update},
    test_utils::mock_membership,
    validated_state::ValidatedStateManager,
};

/// Test harness that spawns consensus + mock coordinator and provides
/// helpers to send events and collect results.
///
/// Can be created with or without a `ValidatedStateManager`:
/// - `new()` — consensus + mock coordinator (inline state handling)
/// - `new_with_state_manager()` — consensus + state manager + mock coordinator
pub(crate) struct TestHarness {
    /// Send ConsensusEvents directly to consensus
    consensus_tx: Sender<ConsensusEvent<TestTypes>>,
    /// Send Events to the mock coordinator (for shutdown etc.)
    coordinator_handle: CoordinatorHandle<TestTypes>,
    /// Join handle for mock coordinator (collects received events)
    mock_join: JoinHandle<Vec<Event<TestTypes>>>,
}

impl TestHarness {
    /// Create a test harness with the given node index (0-9).
    /// State verification is handled inline by the mock coordinator.
    pub async fn new(node_index: u64) -> Self {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let membership = mock_membership().await;
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        let (consensus_tx, consensus_rx) = tokio::sync::mpsc::channel(100);

        let mock_coordinator = MockCoordinator {
            event_rx,
            consensus_tx: consensus_tx.clone(),
            state_tx: None,
            membership_coordinator: membership.clone(),
            received_events: Vec::new(),
        };
        let coordinator_handle = CoordinatorHandle::new(event_tx);
        let mut consensus = Consensus::new(
            consensus_rx,
            coordinator_handle.clone(),
            membership,
            public_key,
            private_key,
        );

        tokio::spawn(async move {
            consensus.run().await;
        });
        let mock_join = tokio::spawn(async move { mock_coordinator.run().await });

        Self {
            consensus_tx,
            coordinator_handle,
            mock_join,
        }
    }

    /// Create a test harness that wires Consensus and ValidatedStateManager
    /// together through the MockCoordinator.
    pub async fn new_with_state_manager(node_index: u64) -> Self {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let membership = mock_membership().await;
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        let (consensus_tx, consensus_rx) = tokio::sync::mpsc::channel(100);
        let (state_tx, state_rx) = tokio::sync::mpsc::channel(100);

        // The ValidatedStateManager shares the same coordinator handle so
        // its responses flow back through the mock coordinator.
        let coordinator_handle = CoordinatorHandle::new(event_tx);
        let mut state_manager = ValidatedStateManager::new(
            state_rx,
            Arc::new(TestInstanceState::default()),
            coordinator_handle.clone(),
        );

        // Seed genesis state so view-1 state requests can find a parent.
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

        let mock_coordinator = MockCoordinator {
            event_rx,
            consensus_tx: consensus_tx.clone(),
            state_tx: Some(state_tx),
            membership_coordinator: membership.clone(),
            received_events: Vec::new(),
        };
        let mut consensus = Consensus::new(
            consensus_rx,
            coordinator_handle.clone(),
            membership,
            public_key,
            private_key,
        );

        tokio::spawn(async move {
            consensus.run().await;
        });
        let mock_join = tokio::spawn(async move { mock_coordinator.run().await });

        Self {
            consensus_tx,
            coordinator_handle,
            mock_join,
        }
    }

    /// Send a ConsensusEvent directly to the consensus state machine.
    pub async fn send(&self, event: ConsensusEvent<TestTypes>) {
        self.consensus_tx.send(event).await.unwrap();
    }

    /// Shut down consensus and the mock coordinator, returning all
    /// events the mock coordinator received from consensus.
    pub async fn shutdown(self) -> Vec<Event<TestTypes>> {
        self.consensus_tx
            .send(ConsensusEvent::Shutdown)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        self.coordinator_handle
            .send_event(Event::Action(Action::Shutdown))
            .await
            .unwrap();
        self.mock_join.await.unwrap()
    }
}

// ── Event assertion helpers ──

pub(crate) fn has_vote1(events: &[Event<TestTypes>]) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            Event::Action(Action::SendMessage(RequestMessageSender::Vote1(_)))
        )
    })
}

pub(crate) fn has_vote2(events: &[Event<TestTypes>]) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            Event::Action(Action::SendMessage(RequestMessageSender::Vote2(_)))
        )
    })
}

pub(crate) fn has_leaf_decided(events: &[Event<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, Event::Update(Update::LeafDecided(_))))
}

pub(crate) fn has_request_state(events: &[Event<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, Event::Action(Action::RequestState(_))))
}

pub(crate) fn has_proposal(events: &[Event<TestTypes>]) -> bool {
    events.iter().any(|e| {
        matches!(
            e,
            Event::Action(Action::SendMessage(RequestMessageSender::Proposal(_, _)))
        )
    })
}

pub(crate) fn has_request_block_and_header(events: &[Event<TestTypes>]) -> bool {
    events
        .iter()
        .any(|e| matches!(e, Event::Action(Action::RequestBlockAndHeader(_))))
}

pub(crate) fn count_vote1(events: &[Event<TestTypes>]) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::Action(Action::SendMessage(RequestMessageSender::Vote1(_)))
            )
        })
        .count()
}

pub(crate) fn count_vote2(events: &[Event<TestTypes>]) -> usize {
    events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::Action(Action::SendMessage(RequestMessageSender::Vote2(_)))
            )
        })
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
