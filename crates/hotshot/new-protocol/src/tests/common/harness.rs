use std::sync::Arc;

use hotshot::types::BLSPubKey;
use hotshot_example_types::{
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_types::{
    data::{Leaf2, ViewNumber},
    traits::{
        signature_key::SignatureKey,
        storage::{null_load_drb_progress_fn, null_store_drb_progress_fn},
    },
};
use tokio::task::JoinHandle;

use super::utils::mock_membership;
use crate::{
    Outbox,
    consensus::Consensus,
    drb::DrbRequester,
    events::ConsensusOutput,
    helpers::upgrade_lock,
    state::StateManager,
    tests::common::mock::testing::MockCoordinator,
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

/// Test harness that spawns consensus + mock coordinator and provides
/// helpers to send events and collect results.
///
/// All inputs are sent directly as `ConsensusInput` to the mock coordinator.
/// When a `StateManager` is wired in, the mock coordinator owns it
/// directly and polls `next()` to feed completions back as `ConsensusInput`.
pub(crate) struct TestHarness {
    /// Send ConsensusInput to the mock coordinator
    input_tx: tokio::sync::mpsc::Sender<ConsensusOutput<TestTypes>>,
    /// Oneshot to signal shutdown
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    /// Join handle for mock coordinator (collects received events)
    mock_join: JoinHandle<Vec<ConsensusOutput<TestTypes>>>,
}

impl TestHarness {
    pub async fn new_with_cpu_tasks(node_index: u64) -> Self {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let membership = mock_membership().await;
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let (coordinator_tx, coordinator_rx) = tokio::sync::mpsc::channel(100);

        let store_drb_progress = null_store_drb_progress_fn();
        let load_drb_progress = null_load_drb_progress_fn();
        let drb_request_task = DrbRequester::new(store_drb_progress, load_drb_progress);

        let vote1_task = VoteCollector::new(membership.clone(), upgrade_lock());
        let vote2_task = VoteCollector::new(membership.clone(), upgrade_lock());

        let consensus = Consensus::new(membership.clone(), public_key, private_key);

        let vid_disperse_task = VidDisperser::new(membership.clone());
        let vid_reconstruction_task = VidReconstructor::new();
        let mock_coordinator = MockCoordinator {
            consensus,
            input_rx: coordinator_rx,
            shutdown_rx,
            state_manager: None,
            vote1_task: Some(vote1_task),
            vote2_task: Some(vote2_task),
            vid_disperse_task: Some(vid_disperse_task),
            vid_reconstruction_task: Some(vid_reconstruction_task),
            drb_request_task: Some(drb_request_task),
            membership_coordinator: membership,
            outbox: Outbox::new(),
            received_events: Vec::new(),
        };
        let mock_join = tokio::spawn(async move { mock_coordinator.run().await });

        Self {
            input_tx: coordinator_tx,
            shutdown_tx: Some(shutdown_tx),
            mock_join,
        }
    }

    /// Create a test harness that wires Consensus and StateManager
    /// together through the MockCoordinator.
    pub async fn new_with_state_manager(node_index: u64) -> Self {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let membership = mock_membership().await;
        let (input_tx, input_rx) = tokio::sync::mpsc::channel(100);
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let genesis_state = TestValidatedState::default();
        let genesis_leaf = Leaf2::<TestTypes>::genesis(
            &genesis_state,
            &TestInstanceState::default(),
            TEST_VERSIONS.test.base,
        )
        .await;

        let mut state_manager = StateManager::new(Arc::new(TestInstanceState::default()));
        state_manager.seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);

        let consensus = Consensus::new(membership.clone(), public_key, private_key);

        let mock_coordinator = MockCoordinator {
            consensus,
            input_rx,
            shutdown_rx,
            vote1_task: None,
            vote2_task: None,
            vid_disperse_task: None,
            vid_reconstruction_task: None,
            drb_request_task: None,
            state_manager: Some(state_manager),
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

    /// Send an event to the mock coordinator.
    pub async fn send(&self, input: impl Into<ConsensusOutput<TestTypes>>) {
        self.input_tx.send(input.into()).await.unwrap();
    }

    /// Shut down and return all events the mock coordinator collected.
    pub async fn shutdown(mut self) -> Vec<ConsensusOutput<TestTypes>> {
        // Small delay to let async processing complete
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let _ = self.shutdown_tx.take().unwrap().send(());
        self.mock_join.await.unwrap()
    }
}
