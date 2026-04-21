use std::{marker::PhantomData, sync::Arc, time::Duration};

use committable::Committable;
use hotshot::{traits::NodeImplementation, types::BLSPubKey};
use hotshot_example_types::{
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    simple_vote::QuorumData2,
    traits::signature_key::SignatureKey,
};

use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    consensus::Consensus,
    coordinator::{Coordinator, timer::Timer},
    epoch::EpochManager,
    helpers::upgrade_lock,
    message::{Certificate1, Certificate2, Proposal},
    network::Network,
    outbox::Outbox,
    proposal::ProposalValidator,
    state::StateManager,
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

/// Build a [`Coordinator`] for testing with an externally provided network.
///
/// The coordinator is fully bootstrapped: consensus is seeded with a genesis
/// certificate and proposal so that the view-1 leader can propose without any
/// external injection.  The initial `ViewChanged` and (for the leader)
/// `RequestBlockAndHeader` outputs are already queued in the outbox.
pub async fn build_test_coordinator<I: NodeImplementation<TestTypes>>(
    node_index: u64,
    network: I::Network,
    membership: EpochMembershipCoordinator<TestTypes>,
    epoch_height: u64,
    view_timeout: Duration,
) -> Coordinator<TestTypes, I::Network, TestStorage<TestTypes>> {
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    let instance = Arc::new(TestInstanceState::default());

    let epoch_manager = EpochManager::new(epoch_height, membership.clone());

    let vote1_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let vote2_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let timeout_one_honest_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let checkpoint_collector = VoteCollector::new(membership.clone(), upgrade_lock());

    let genesis_state = TestValidatedState::default();
    let genesis_leaf =
        Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;

    let mut consensus = Consensus::new(
        membership.clone(),
        public_key,
        private_key.clone(),
        genesis_leaf.clone(),
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
    state_manager.seed_state(
        ViewNumber::genesis(),
        Arc::new(genesis_state),
        genesis_leaf.clone(),
    );

    // Build a genesis cert1 and proposal so consensus can self-start.
    let genesis_cert1 = build_genesis_cert1(&genesis_leaf);
    let genesis_cert2 = build_genesis_cert2(&genesis_leaf);
    let genesis_proposal = build_genesis_proposal(&genesis_leaf, &genesis_cert1);
    consensus.seed_genesis(genesis_cert1, genesis_proposal);

    // Store the genesis leaf in the leaf store so that epoch catchup can
    // find it.  The genesis block is the epoch root for the earliest
    // epochs and must be available for peers requesting catchup data.
    epoch_manager
        .leaf_store()
        .insert(genesis_leaf.clone(), genesis_cert2);

    let proposal_validator = ProposalValidator::new(membership.clone());

    let network = Network::new(network, membership.clone(), upgrade_lock());

    let mut coordinator = Coordinator::builder()
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
        .storage(crate::storage::Storage::new(
            TestStorage::default(),
            private_key,
        ))
        .membership_coordinator(membership)
        .outbox(Outbox::new())
        .timer(Timer::new(
            view_timeout,
            ViewNumber::genesis(),
            EpochNumber::genesis(),
        ))
        .public_key(public_key)
        .build();

    // Emit initial ViewChanged + RequestBlockAndHeader (if leader).
    coordinator.start().await;

    // Process the initial outputs so the timer resets and block builder
    // gets notified before the event loop starts.
    while let Some(output) = coordinator.outbox_mut().pop_front() {
        let _ = coordinator.process_consensus_output(output).await;
    }

    coordinator
}

/// Create a genesis `Certificate1` that references the genesis leaf.
///
/// Uses `QuorumCertificate2::new` with `None` signatures, matching the
/// pattern used by `Leaf2::genesis` for its justify_qc.
fn build_genesis_cert1(genesis_leaf: &Leaf2<TestTypes>) -> Certificate1<TestTypes> {
    let data = QuorumData2 {
        leaf_commit: genesis_leaf.commit(),
        epoch: Some(EpochNumber::genesis()),
        block_number: Some(0),
    };
    Certificate1::new(
        data.clone(),
        data.commit(),
        ViewNumber::genesis(),
        None,
        PhantomData,
    )
}

/// Create a genesis `Certificate2` that references the genesis leaf.
fn build_genesis_cert2(genesis_leaf: &Leaf2<TestTypes>) -> Certificate2<TestTypes> {
    use hotshot_types::simple_vote::Vote2Data;

    let data = Vote2Data {
        leaf_commit: genesis_leaf.commit(),
        epoch: EpochNumber::genesis(),
        block_number: 0,
    };
    Certificate2::new(
        data.clone(),
        data.commit(),
        ViewNumber::genesis(),
        None,
        PhantomData,
    )
}

/// Create a genesis `Proposal` from the genesis leaf and cert.
fn build_genesis_proposal(
    genesis_leaf: &Leaf2<TestTypes>,
    genesis_cert1: &Certificate1<TestTypes>,
) -> Proposal<TestTypes> {
    Proposal {
        block_header: genesis_leaf.block_header().clone(),
        view_number: ViewNumber::genesis(),
        epoch: EpochNumber::genesis(),
        justify_qc: genesis_cert1.clone(),
        next_epoch_justify_qc: None,
        upgrade_certificate: None,
        view_change_evidence: None,
        next_drb_result: None,
        state_cert: None,
    }
}
