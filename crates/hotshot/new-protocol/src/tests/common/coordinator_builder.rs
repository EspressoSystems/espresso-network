use std::{marker::PhantomData, sync::Arc, time::Duration};

use committable::Committable;
use hotshot::types::BLSPubKey;
use hotshot_example_types::{
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper, VidCommitment,
        ViewChangeEvidence2, ViewNumber,
    },
    epoch_membership::EpochMembershipCoordinator,
    light_client::StateKeyPair,
    message::Proposal as SignedProposal,
    simple_vote::QuorumData2,
    traits::{block_contents::BlockHeader, signature_key::SignatureKey, storage::Storage as _},
};

use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    client::CoordinatorClient,
    consensus::{Consensus, PreCutoverSeed},
    coordinator::{Coordinator, timer::Timer},
    epoch::EpochManager,
    epoch_root_vote_collector::EpochRootVoteCollector,
    helpers::test_upgrade_lock,
    message::{Certificate1, Proposal},
    network::Network,
    outbox::Outbox,
    proposal::{ProposalValidator, VidShareValidator},
    state::StateManager,
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

#[allow(clippy::too_many_arguments)]
pub async fn build_test_coordinator<N: Network<TestTypes>>(
    node_index: u64,
    network: N,
    membership: EpochMembershipCoordinator<TestTypes>,
    storage: TestStorage<TestTypes>,
    client: CoordinatorClient<TestTypes>,
    epoch_height: u64,
    view_timeout: Duration,
    pre_cutover_seed: Option<PreCutoverSeed<TestTypes>>,
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
    let epoch_root_collector =
        EpochRootVoteCollector::new(membership.clone(), upgrade_lock.clone());

    let genesis_state = TestValidatedState::default();
    let genesis_leaf =
        Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;

    // A node restarting with persistent storage resumes from its persisted
    // decided anchor (mirroring production's `Coordinator::maker`, which
    // gets the anchor from the `HotShotInitializer`); otherwise it starts
    // from genesis.
    let restart_anchor = storage.anchor_leaf().await;
    let initial_leaf = restart_anchor
        .as_ref()
        .map(|(leaf, _)| leaf.clone())
        .unwrap_or_else(|| genesis_leaf.clone());

    let mut consensus = Consensus::new(
        membership.clone(),
        public_key,
        private_key.clone(),
        state_private_key,
        10,
        upgrade_lock.clone(),
        initial_leaf,
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
    let genesis_state = Arc::new(genesis_state);
    state_manager.seed_state(
        ViewNumber::genesis(),
        genesis_state.clone(),
        genesis_leaf.clone(),
    );

    if let Some(seed) = pre_cutover_seed.as_ref() {
        let anchor_view = seed.decided_anchor.view_number();
        if let Some(state) = seed.validated_states.get(&anchor_view).cloned() {
            state_manager.seed_state(anchor_view, state, seed.decided_anchor.clone());
        }
        for leaf in &seed.undecided {
            let view = leaf.view_number();
            if let Some(state) = seed.validated_states.get(&view).cloned() {
                state_manager.seed_state(view, state, leaf.clone());
            }
        }
    }

    // Build a genesis cert1 and proposal so consensus can self-start.
    let genesis_cert1 = build_genesis_cert1(&genesis_leaf);
    let genesis_proposal = build_genesis_proposal(&genesis_leaf, &genesis_cert1);

    let anchor_view = if let Some((anchor_leaf, anchor_cert)) = restart_anchor {
        // Seed the persisted anchor as the parent, exactly as production's
        // `Coordinator::maker` does from the initializer: rebuild the parent
        // proposal from the anchor leaf, seed a sparse state for it, and
        // treat the anchor plus persisted proposals as already-reconstructed
        // blocks so the first leader after restart is not stalled by the
        // `parent_block_reconstructed` check.
        let anchor_view = anchor_leaf.view_number();
        let anchor_proposal = Proposal {
            block_header: anchor_leaf.block_header().clone(),
            view_number: anchor_view,
            epoch: anchor_leaf
                .epoch(epoch_height)
                .unwrap_or(EpochNumber::genesis()),
            justify_qc: anchor_leaf.justify_qc(),
            next_epoch_justify_qc: None,
            upgrade_certificate: anchor_leaf.upgrade_certificate(),
            view_change_evidence: anchor_leaf
                .view_change_evidence
                .clone()
                .and_then(|e| match e {
                    ViewChangeEvidence2::Timeout(tc) => Some(tc),
                    ViewChangeEvidence2::ViewSync(_) => None,
                }),
            next_drb_result: anchor_leaf.next_drb_result,
            state_cert: None,
        };
        let anchor_state = <TestValidatedState as hotshot_types::traits::states::ValidatedState<
            TestTypes,
        >>::from_header(anchor_leaf.block_header());
        state_manager.seed_state(anchor_view, Arc::new(anchor_state), anchor_leaf.clone());
        let reconstructed: Vec<_> =
            std::iter::once((anchor_view, anchor_leaf.block_header().clone()))
                .chain(
                    storage
                        .proposals_cloned()
                        .await
                        .into_iter()
                        .map(|(view, p)| (view, p.data.block_header().clone())),
                )
                .filter_map(|(view, header)| {
                    match BlockHeader::<TestTypes>::payload_commitment(&header) {
                        VidCommitment::V2(commitment) => Some((view, commitment)),
                        _ => None,
                    }
                })
                .collect();
        consensus.seed_parent(anchor_cert, anchor_proposal, reconstructed);
        anchor_view
    } else {
        // The synthetic genesis proposal carries the genesis cert1 as its
        // justify_qc, so the leaf derived from it has a different commitment than
        // `genesis_leaf` (which has a null justify_qc). `request_header` for view 1
        // looks up the parent state by the proposal's leaf commitment, so seed the
        // genesis state under that commitment too.
        state_manager.seed_state(
            ViewNumber::genesis(),
            genesis_state,
            Leaf2::from(genesis_proposal.clone()),
        );
        consensus.seed_parent(
            genesis_cert1.clone(),
            genesis_proposal.clone(),
            std::iter::empty(),
        );
        ViewNumber::genesis()
    };

    if let Some(seed) = pre_cutover_seed {
        consensus.apply_pre_cutover_seed(seed);
    }

    // Restarted nodes must not act again in views they acted in before.
    let restart_view = storage.restart_view().await;
    let last_actioned_view = storage.last_actioned_view().await;
    consensus.resume_from_restart(anchor_view, restart_view, last_actioned_view);

    // A leader proposing on an epoch-root parent QC right after restart
    // needs the persisted light-client state cert (as in production, where
    // it arrives via `HotShotInitializer::state_cert`).
    if let Some(state_cert) = storage.state_cert_cloned().await {
        consensus.seed_state_cert(state_cert);
    }

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
    let share_validator =
        VidShareValidator::new(membership.clone(), epoch_height, upgrade_lock.clone());

    let mut coordinator = Coordinator::builder()
        .consensus(consensus)
        .network(network)
        .state_manager(state_manager)
        .vote1_collector(vote1_collector)
        .vote2_collector(vote2_collector)
        .timeout_collector(timeout_collector)
        .timeout_one_honest_collector(timeout_one_honest_collector)
        .epoch_root_collector(epoch_root_collector)
        .vid_disperser(vid_disperser)
        .vid_reconstructor(vid_reconstructor)
        .epoch_manager(epoch_manager)
        .block_builder(block_builder)
        .proposal_validator(proposal_validator)
        .share_validator(share_validator)
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
    coordinator.start();

    // Process the initial outputs so the timer resets and block builder
    // gets notified before the event loop starts.
    while let Some(output) = coordinator.outbox_mut().pop_front() {
        let _ = coordinator.process_consensus_output(output);
    }

    coordinator
}

/// Create a genesis `Certificate1` that references the genesis leaf.
///
/// Uses `QuorumCertificate2::new` with `None` signatures, matching the
/// pattern used by `Leaf2::genesis` for its justify_qc.
pub(crate) fn build_genesis_cert1(genesis_leaf: &Leaf2<TestTypes>) -> Certificate1<TestTypes> {
    let data = QuorumData2 {
        leaf_commit: genesis_leaf.commit(),
        epoch: Some(EpochNumber::genesis()),
        block_number: Some(0),
    };
    Certificate1::new(
        data,
        data.commit(),
        ViewNumber::genesis(),
        None,
        PhantomData,
    )
}

/// Create a genesis `Proposal` from the genesis leaf and cert.
pub(crate) fn build_genesis_proposal(
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
