use std::{marker::PhantomData, sync::Arc, time::Duration};

use async_broadcast::Sender;
use committable::Committable;
use hotshot::{
    traits::NodeImplementation,
    types::{BLSPubKey, Event},
};
use hotshot_example_types::{
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    light_client::StateKeyPair,
    message::Proposal as SignedProposal,
    simple_vote::QuorumData2,
    traits::{signature_key::SignatureKey, storage::Storage as StorageTrait},
};

use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    consensus::Consensus,
    coordinator::{Coordinator, timer::Timer},
    epoch::EpochManager,
    epoch_root_vote_collector::EpochRootVoteCollector,
    helpers::test_upgrade_lock,
    message::{Certificate1, Proposal},
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
    mut membership: EpochMembershipCoordinator<TestTypes>,
    storage: TestStorage<TestTypes>,
    epoch_height: u64,
    view_timeout: Duration,
) -> (
    Coordinator<TestTypes, I::Network, TestStorage<TestTypes>>,
    Sender<Event<TestTypes>>,
) {
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    let state_key_pair = StateKeyPair::generate_from_seed_indexed([0u8; 32], node_index);
    let state_private_key = state_key_pair.sign_key_ref().clone();
    let instance = Arc::new(TestInstanceState::default());
    let upgrade_lock = test_upgrade_lock();

    // Channel used by the Coordinator to forward ExternalMessageReceived
    // events to the Membership's Leaf2Fetcher.  The fetcher drives epoch
    // catchup (leaf request/response over external messages).  Overflow
    // is enabled so slow listeners don't stall the Coordinator.
    let (mut external_events_tx, mut external_events_rx) =
        async_broadcast::broadcast::<hotshot_types::event::Event<TestTypes>>(1024);
    external_events_tx.set_overflow(true);
    external_events_rx.set_overflow(true);
    membership.set_external_channel(external_events_rx).await;

    let epoch_manager = EpochManager::new(epoch_height, membership.clone());

    let vote1_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let vote2_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_one_honest_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let checkpoint_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let epoch_root_collector =
        EpochRootVoteCollector::new(membership.clone(), upgrade_lock.clone());

    let genesis_state = TestValidatedState::default();
    let genesis_leaf =
        Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;

    let mut consensus = Consensus::new(
        membership.clone(),
        public_key,
        private_key.clone(),
        state_private_key,
        10,
        upgrade_lock.clone(),
        genesis_leaf.clone(),
        epoch_height,
    );

    let vid_disperser = VidDisperser::new(membership.clone());
    let vid_reconstructor = VidReconstructor::new();

    let block_builder = BlockBuilder::new(
        instance.clone(),
        membership.clone(),
        BlockBuilderConfig::default(),
        upgrade_lock.clone(),
    );

    let mut state_manager = StateManager::new(instance.clone(), upgrade_lock.clone());
    state_manager.seed_state(
        ViewNumber::genesis(),
        Arc::new(genesis_state),
        genesis_leaf.clone(),
    );

    // Build a genesis cert1 and proposal so consensus can self-start.
    let genesis_cert1 = build_genesis_cert1(&genesis_leaf);
    let genesis_proposal = build_genesis_proposal(&genesis_leaf, &genesis_cert1);
    consensus.seed_genesis(genesis_cert1.clone(), genesis_proposal.clone());

    // Seed the genesis proposal into the backing TestStorage so that
    // peers can serve the genesis block to late-joiners during
    // `EpochMembershipCoordinator::catchup` (epoch 0 root block == 0).
    let genesis_wrapper = QuorumProposalWrapper::<TestTypes> {
        proposal: QuorumProposal2 {
            block_header: genesis_leaf.block_header().clone(),
            view_number: ViewNumber::genesis(),
            epoch: Some(EpochNumber::genesis()),
            justify_qc: genesis_cert1.clone(),
            next_epoch_justify_qc: None,
            upgrade_certificate: None,
            view_change_evidence: None,
            next_drb_result: None,
            state_cert: None,
        },
    };
    let genesis_signed = SignedProposal::<TestTypes, QuorumProposalWrapper<TestTypes>> {
        data: genesis_wrapper,
        signature: BLSPubKey::sign(&private_key, &[]).expect("sign genesis"),
        _pd: PhantomData,
    };
    storage
        .append_proposal_wrapper(&genesis_signed)
        .await
        .expect("seed genesis proposal");

    let proposal_validator =
        ProposalValidator::new(membership.clone(), epoch_height, upgrade_lock.clone());

    let network = Network::new(network, membership.clone(), upgrade_lock);

    let mut coordinator = Coordinator::builder()
        .consensus(consensus)
        .network(network)
        .state_manager(state_manager)
        .vote1_collector(vote1_collector)
        .vote2_collector(vote2_collector)
        .timeout_collector(timeout_collector)
        .timeout_one_honest_collector(timeout_one_honest_collector)
        .checkpoint_collector(checkpoint_collector)
        .epoch_root_collector(epoch_root_collector)
        .vid_disperser(vid_disperser)
        .vid_reconstructor(vid_reconstructor)
        .epoch_manager(epoch_manager)
        .block_builder(block_builder)
        .proposal_validator(proposal_validator)
        .storage(crate::storage::Storage::new(storage, private_key))
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

    (coordinator, external_events_tx)
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
