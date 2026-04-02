use std::{sync::Arc, time::Duration};

use hotshot::types::BLSPubKey;
use hotshot_example_types::{
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_types::{
    data::{Leaf2, ViewNumber},
    traits::signature_key::SignatureKey,
};

use super::utils::mock_membership;
use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::timer::Timer,
    epoch::EpochManager,
    helpers::upgrade_lock,
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
        // Default timer is long enough to not fire during normal tests,
        // which complete in ~100-200ms.
        Self::new_with_timer(node_index, Duration::from_secs(2)).await
    }

    pub async fn new_with_timer(node_index: u64, timer_duration: Duration) -> Self {
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
        let instance = Arc::new(TestInstanceState::default());
        let membership = mock_membership().await;

        let epoch_manager = EpochManager::new(10, membership.clone());

        let vote1_task = VoteCollector::new(membership.clone(), upgrade_lock());
        let vote2_task = VoteCollector::new(membership.clone(), upgrade_lock());
        let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock());
        let checkpoint_collector = VoteCollector::new(membership.clone(), upgrade_lock());

        let consensus = Consensus::new(membership.clone(), public_key, private_key.clone(), 10);

        let vid_disperse_task = VidDisperser::new(membership.clone());
        let vid_reconstruction_task = VidReconstructor::new();

        let block_config = BlockBuilderConfig::default();
        let block_builder = BlockBuilder::new(instance.clone(), membership.clone(), block_config);

        let mut state_manager = StateManager::new(instance.clone());
        let genesis_state = TestValidatedState::default();
        let genesis_leaf =
            Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;
        state_manager.seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);

        let proposal_validator = ProposalValidator::new(membership.clone());

        let network = Network::new(MockNetwork::default(), membership.clone(), upgrade_lock());

        let coordinator = MockCoordinator::builder()
            .consensus(consensus)
            .network(network)
            .state_manager(state_manager)
            .vote1_collector(vote1_task)
            .vote2_collector(vote2_task)
            .timeout_collector(timeout_collector)
            .checkpoint_collector(checkpoint_collector)
            .vid_disperser(vid_disperse_task)
            .vid_reconstructor(vid_reconstruction_task)
            .epoch_manager(epoch_manager)
            .block_builder(block_builder)
            .proposal_validator(proposal_validator)
            .membership_coordinator(membership)
            .outbox(Outbox::new())
            .timer(Timer::new(timer_duration, ViewNumber::genesis()))
            .public_key(public_key)
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
    pub async fn process_until<P>(&mut self, pred: P) -> Vec<ConsensusInput<TestTypes>>
    where
        P: Fn(&[ConsensusInput<TestTypes>]) -> bool,
    {
        let mut inputs = Vec::new();
        while !pred(&inputs) {
            match self.coordinator.next_consensus_input().await {
                Ok(input) => {
                    self.apply_and_process(input.clone()).await;
                    inputs.push(input);
                },
                Err(err) => panic!("Unexpected error: {err}"),
            }
        }
        inputs
    }

    pub fn outputs(&self) -> &Outbox<ConsensusOutput<TestTypes>> {
        &self.outputs
    }
}
