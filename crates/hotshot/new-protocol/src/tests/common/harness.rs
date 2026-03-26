use std::{sync::Arc, time::Duration};

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

use super::utils::mock_membership;
use crate::{
    Outbox,
    consensus::Consensus,
    coordinator::Timer,
    drb::DrbRequester,
    events::{ConsensusInput, ConsensusOutput},
    helpers::upgrade_lock,
    message::Message,
    network::Network,
    state::StateManager,
    tests::common::mock::testing::{MockCoordinator, MockNetwork},
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
    coordinator: MockCoordinator,
    outputs: Vec<ConsensusOutput<TestTypes>>,
}

impl TestHarness {
    pub async fn new(node_index: u64) -> Self {
        // Default timer is long enough to not fire during normal tests,
        // which complete in ~100-200ms.
        Self::new_with_timer(node_index, Duration::from_secs(2)).await
    }

    pub async fn new_with_timer(node_index: u64, timer_duration: Duration) -> Self {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let membership = mock_membership().await;

        let store_drb_progress = null_store_drb_progress_fn();
        let load_drb_progress = null_load_drb_progress_fn();
        let drb_request_task = DrbRequester::new(store_drb_progress, load_drb_progress);

        let vote1_task = VoteCollector::new(membership.clone(), upgrade_lock());
        let vote2_task = VoteCollector::new(membership.clone(), upgrade_lock());
        let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock());
        let checkpoint_collector = VoteCollector::new(membership.clone(), upgrade_lock());

        let consensus = Consensus::new(membership.clone(), public_key, private_key.clone());

        let vid_disperse_task = VidDisperser::new(membership.clone());
        let vid_reconstruction_task = VidReconstructor::new();

        let mut state_manager = StateManager::new(Arc::new(TestInstanceState::default()));
        let genesis_state = TestValidatedState::default();
        let genesis_leaf = Leaf2::<TestTypes>::genesis(
            &genesis_state,
            &TestInstanceState::default(),
            TEST_VERSIONS.test.base,
        )
        .await;
        state_manager.seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);

        let network = Network::new(MockNetwork::default(), membership.clone(), upgrade_lock());

        let (tx, _rx) = async_broadcast::broadcast(1);

        let coordinator = MockCoordinator::builder()
            .external_tx(tx)
            .consensus(consensus)
            .network(network)
            .state_manager(state_manager)
            .vote1_collector(vote1_task)
            .vote2_collector(vote2_task)
            .timeout_collector(timeout_collector)
            .checkpoint_collector(checkpoint_collector)
            .vid_disperser(vid_disperse_task)
            .vid_reconstructor(vid_reconstruction_task)
            .drb_requester(drb_request_task)
            .membership_coordinator(membership)
            .outbox(Outbox::new())
            .timer(Timer::new(timer_duration))
            .public_key(public_key)
            .build();
        Self {
            coordinator,
            outputs: Vec::new(),
        }
    }

    pub async fn message(&mut self, message: Message<TestTypes>) {
        if let Some(input) = self.coordinator.on_message(message).await {
            self.send_input(input).await;
        }
    }

    pub async fn send_input(&mut self, input: ConsensusInput<TestTypes>) {
        self.coordinator
            .consensus
            .apply(input, &mut self.coordinator.outbox)
            .await;
        for output in self.coordinator.outbox().iter() {
            self.outputs.push(output.clone());
        }
        self.coordinator.evaluate_outputs().await;
    }

    pub async fn next_inputs(&mut self, num_inputs: usize) -> Vec<ConsensusInput<TestTypes>> {
        let mut inputs = Vec::new();
        for _ in 0..num_inputs {
            if let Some(input) = self.coordinator.next_input().await {
                if matches!(input, ConsensusInput::Timeout(_)) {
                    panic!("Expected a non-timeout input, got timeout");
                }
                inputs.push(input);
            }
        }
        for input in inputs.clone() {
            self.send_input(input).await;
        }
        inputs
    }

    pub async fn next_timeout(&mut self) -> Option<ConsensusInput<TestTypes>> {
        let next = self.coordinator.next_input().await;
        if let Some(input) = next
            && matches!(input, ConsensusInput::Timeout(_))
        {
            return Some(input);
        }

        None
    }
    pub fn outputs(&self) -> &Vec<ConsensusOutput<TestTypes>> {
        &self.outputs
    }
}
