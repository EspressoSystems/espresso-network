use std::{marker::PhantomData, sync::Arc, time::Duration};

use committable::Committable;
use hotshot::types::BLSPubKey;
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
    traits::{signature_key::SignatureKey, storage::Storage as _},
};

use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    client::CoordinatorClient,
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
///
/// If `pre_cutover_seed` is provided, it is applied **synchronously before**
/// `coord.start()` runs. This prevents the startup race where `start()`
/// emits `ViewChanged(1)` and the view-1 leader proposes before an
/// async-dispatched seed can land. With a seed in place, `start()` reads the
/// (now advanced) `current_view` and emits `ViewChanged(max_seeded_view + 1)`
/// instead.
#[allow(clippy::too_many_arguments)]
pub async fn build_test_coordinator<N: Network<TestTypes>>(
    node_index: u64,
    network: N,
    membership: EpochMembershipCoordinator<TestTypes>,
    storage: TestStorage<TestTypes>,
    client: CoordinatorClient<TestTypes>,
    epoch_height: u64,
    view_timeout: Duration,
    pre_cutover_seed: Option<crate::tests::common::runner::PreCutoverSeed>,
) -> Coordinator<TestTypes, N, TestStorage<TestTypes>> {
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    let state_key_pair = StateKeyPair::generate_from_seed_indexed([0u8; 32], node_index);
    let state_private_key = state_key_pair.sign_key_ref().clone();
    let instance = Arc::new(TestInstanceState::default());
    let upgrade_lock = test_upgrade_lock();

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

    // If a pre-cutover seed is provided, seed the StateManager for each
    // pre-cutover leaf. The new protocol's proposal validator pipelines
    // state validation against the parent's stored state — without this,
    // the leader of `max_seeded_view + 1` cannot validate its own proposal
    // (no parent state on file). For tests we use the default
    // `TestValidatedState`; in production the espresso bridge would carry
    // legacy state forward.
    if let Some(seed) = pre_cutover_seed.as_ref() {
        let default_state = Arc::new(TestValidatedState::default());
        state_manager.seed_state(
            seed.decided_anchor.view_number(),
            default_state.clone(),
            seed.decided_anchor.clone(),
        );
        for leaf in &seed.undecided {
            state_manager.seed_state(leaf.view_number(), default_state.clone(), leaf.clone());
        }
    }

    // Build a genesis cert1 and proposal so consensus can self-start.
    let genesis_cert1 = build_genesis_cert1(&genesis_leaf);
    let genesis_proposal = build_genesis_proposal(&genesis_leaf, &genesis_cert1);
    consensus.seed_genesis(genesis_cert1.clone(), genesis_proposal.clone());

    // Apply the legacy → new-protocol seed (if provided) BEFORE we hand
    // consensus to the coordinator builder. After this, `current_view` is
    // advanced past the seeded views and `coord.start()` will emit
    // `ViewChanged(max_seeded_view + 1)` instead of the genesis-default
    // `ViewChanged(1)`.
    if let Some(seed) = pre_cutover_seed {
        consensus.set_pre_cutover_anchor(seed.decided_anchor);
        consensus.seed_pre_cutover_leaves(seed.undecided);
        consensus.register_proposal_justify_qc(&seed.high_qc);
    }

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
        .client(client)
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
