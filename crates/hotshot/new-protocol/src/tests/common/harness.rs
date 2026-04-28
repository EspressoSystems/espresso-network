use std::{sync::Arc, time::Duration};

use hotshot::types::BLSPubKey;
use hotshot_example_types::{
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    traits::signature_key::SignatureKey,
};

use super::utils::mock_membership;
use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::{error::Severity, timer::Timer},
    epoch::EpochManager,
    helpers::test_upgrade_lock,
    logging::KeyPrefix,
    message::Message,
    network::Network,
    outbox::Outbox,
    proposal::ProposalValidator,
    state::StateManager,
    tests::common::mock::testing::{MockCoordinator, MockNetwork},
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

/// Test harness that spawns consensus + mock coordinator and provides
/// helpers to send events and collect results.
pub(crate) struct TestHarness {
    coordinator: MockCoordinator,
    outputs: Outbox<ConsensusOutput<TestTypes>>,
}

impl TestHarness {
    pub async fn new(node_index: u64) -> Self {
        // Default timer must be long enough to not fire during tests even
        // under heavy CPU load (e.g. full test suite running in parallel).
        Self::new_with_timer(node_index, Duration::from_secs(2)).await
    }

    pub async fn new_with_timer(node_index: u64, timer_duration: Duration) -> Self {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let instance = Arc::new(TestInstanceState::default());
        let membership = mock_membership().await;
        let upgrade_lock = test_upgrade_lock();

        let epoch_manager = EpochManager::new(10, membership.clone());

        let vote1_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
        let vote2_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
        let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
        let timeout_one_honest_collector =
            VoteCollector::new(membership.clone(), upgrade_lock.clone());
        let checkpoint_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());

        let genesis_state = TestValidatedState::default();
        let genesis_leaf =
            Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;

        let consensus = Consensus::new(
            membership.clone(),
            public_key,
            private_key.clone(),
            upgrade_lock.clone(),
            genesis_leaf.clone(),
            10,
        );

        let vid_disperse_task = VidDisperser::new(membership.clone());
        let vid_reconstruction_task = VidReconstructor::new();

        let block_config = BlockBuilderConfig::default();
        let block_builder = BlockBuilder::new(
            instance.clone(),
            membership.clone(),
            block_config,
            upgrade_lock.clone(),
        );

        let mut state_manager = StateManager::new(instance.clone(), upgrade_lock.clone());
        state_manager.seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);

        let proposal_validator = ProposalValidator::new(membership.clone(), upgrade_lock.clone());

        let network = Network::new(MockNetwork::default(), membership.clone(), upgrade_lock);

        let coordinator = MockCoordinator::builder()
            .consensus(consensus)
            .network(network)
            .state_manager(state_manager)
            .vote1_collector(vote1_collector)
            .vote2_collector(vote2_collector)
            .timeout_collector(timeout_collector)
            .timeout_one_honest_collector(timeout_one_honest_collector)
            .checkpoint_collector(checkpoint_collector)
            .vid_disperser(vid_disperse_task)
            .vid_reconstructor(vid_reconstruction_task)
            .epoch_manager(epoch_manager)
            .block_builder(block_builder)
            .proposal_validator(proposal_validator)
            .membership_coordinator(membership)
            .outbox(Outbox::new())
            .timer(Timer::new(
                timer_duration,
                ViewNumber::genesis(),
                EpochNumber::genesis(),
            ))
            .public_key(public_key)
            .node_id(KeyPrefix::from(&public_key))
            .build();
        Self {
            coordinator,
            outputs: Outbox::new(),
        }
    }

    pub async fn message<S>(&mut self, m: Message<TestTypes, S>) {
        if let Some(input) = self
            .coordinator
            .on_network_message(m.into_unchecked())
            .await
        {
            self.apply_and_process(input).await;
        }
    }

    pub async fn apply_and_process(&mut self, input: ConsensusInput<TestTypes>) {
        self.coordinator.apply_consensus(input).await;
        self.outputs
            .extend(self.coordinator.outbox().iter().cloned());
        for out in self.coordinator.outbox_mut().take() {
            if let Err(err) = self.coordinator.process_consensus_output(out).await {
                panic!("unexpected error: {err}")
            }
        }
    }

    /// Process events from the coordinator until `predicate` is satisfied.
    ///
    /// Each event is immediately applied and appended to the collected list.
    /// The predicate is checked after every event; once it returns `true`
    /// the collected inputs are returned.
    ///
    /// This avoids any assumption about the order or number of events
    /// produced by asynchronous coordinator subsystems (proposal validator,
    /// VID reconstructor, vote collectors, state manager, timer).
    pub async fn process_until<P, F>(
        &mut self,
        pred: P,
        fail_pred: F,
    ) -> Vec<ConsensusInput<TestTypes>>
    where
        P: Fn(&[ConsensusInput<TestTypes>]) -> bool,
        F: Fn(&[ConsensusInput<TestTypes>]) -> bool,
    {
        let mut inputs = Vec::new();
        while !pred(&inputs) {
            match self.coordinator.next_consensus_input().await {
                Ok(input) => {
                    self.apply_and_process(input.clone()).await;
                    inputs.push(input);
                },
                Err(err) if err.severity == Severity::Critical => {
                    panic!("Critical coordinator error: {err}")
                },
                Err(_err) => {
                    // Non-critical errors (e.g., epoch root computation failures
                    // in the test environment) are expected and skipped.
                },
            }
            if fail_pred(&inputs) {
                panic!("Received Failure inputs: {inputs:?}");
            }
        }
        inputs
    }

    pub fn outputs(&self) -> &Outbox<ConsensusOutput<TestTypes>> {
        &self.outputs
    }
}
