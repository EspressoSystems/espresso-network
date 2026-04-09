use std::{sync::Arc, time::Duration};

use hotshot::{traits::NodeImplementation, types::BLSPubKey};
use hotshot_example_types::{
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::signature_key::SignatureKey,
};

use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    consensus::Consensus,
    coordinator::{Coordinator, timer::Timer},
    epoch::EpochManager,
    helpers::upgrade_lock,
    network::Network,
    outbox::Outbox,
    proposal::ProposalValidator,
    state::StateManager,
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

/// Build a [`Coordinator`] for testing with an externally provided network.
///
/// This is the generic version of the coordinator construction previously
/// hardcoded in the memory-network test.  The caller is responsible for
/// creating the network instance; everything else (keys, consensus, vote
/// collectors, VID, block builder, state manager, epoch manager, proposal
/// validator) is constructed here.
pub async fn build_test_coordinator<I: NodeImplementation<TestTypes>>(
    node_index: u64,
    network: I::Network,
    membership: EpochMembershipCoordinator<TestTypes>,
    epoch_height: u64,
    view_timeout: Duration,
) -> Coordinator<TestTypes, I> {
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    let instance = Arc::new(TestInstanceState::default());

    let epoch_manager = EpochManager::new(epoch_height, membership.clone());

    let vote1_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let vote2_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let timeout_one_honest_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let checkpoint_collector = VoteCollector::new(membership.clone(), upgrade_lock());

    let consensus = Consensus::new(
        membership.clone(),
        public_key,
        private_key.clone(),
        epoch_height,
    );

    let vid_disperser = VidDisperser::new(membership.clone());
    let vid_reconstructor = VidReconstructor::new();

    let block_builder = BlockBuilder::new(
        instance.clone(),
        membership.clone(),
        BlockBuilderConfig::default(),
    );

    let mut state_manager = StateManager::new(instance.clone());
    let genesis_state = TestValidatedState::default();
    let genesis_leaf =
        Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;
    state_manager.seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);

    let proposal_validator = ProposalValidator::new(membership.clone());

    let network = Network::new(network, membership.clone(), upgrade_lock());

    Coordinator::builder()
        .consensus(consensus)
        .network(network)
        .state_manager(state_manager)
        .vote1_collector(vote1_collector)
        .vote2_collector(vote2_collector)
        .timeout_collector(timeout_collector)
        .timeout_one_honest_collector(timeout_one_honest_collector)
        .checkpoint_collector(checkpoint_collector)
        .vid_disperser(vid_disperser)
        .vid_reconstructor(vid_reconstructor)
        .epoch_manager(epoch_manager)
        .block_builder(block_builder)
        .proposal_validator(proposal_validator)
        .membership_coordinator(membership)
        .outbox(Outbox::new())
        .timer(Timer::new(
            view_timeout,
            ViewNumber::genesis(),
            EpochNumber::genesis(),
        ))
        .public_key(public_key)
        .build()
}
