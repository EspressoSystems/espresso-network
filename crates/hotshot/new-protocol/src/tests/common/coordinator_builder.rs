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
    traits::{
        ValidatedState, block_contents::BlockHeader, election::Membership,
        signature_key::SignatureKey, storage::Storage as _,
    },
    utils::{is_epoch_root, is_transition_block},
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

/// Chain state for a restarted node, tracked by the test runner from decide
/// events. Plays the role of production persistence: `leaf`/`cert1` stand in
/// for `HotShotInitializer::{anchor_leaf, high_qc}` and `epoch_roots` for
/// the persisted epoch info (`load_start_epoch_info`) — without them, DRBs
/// for epochs whose roots were decided before the restart are unrecoverable.
#[derive(Clone)]
pub struct RestartAnchor {
    pub leaf: Leaf2<TestTypes>,
    pub cert1: Certificate1<TestTypes>,
    /// Decided epoch-root and epoch-transition leaves, oldest first.
    pub epoch_leaves: Vec<Leaf2<TestTypes>>,
}

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
    restart_anchor: Option<RestartAnchor>,
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

    let last_decided_leaf = restart_anchor
        .as_ref()
        .map_or_else(|| genesis_leaf.clone(), |anchor| anchor.leaf.clone());
    let mut consensus = Consensus::new(
        membership.clone(),
        public_key,
        private_key.clone(),
        state_private_key,
        10,
        upgrade_lock.clone(),
        last_decided_leaf,
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

    if let Some(seed) = pre_cutover_seed {
        consensus.apply_pre_cutover_seed(seed);
    }

    // Seed the network's last decided leaf as the consensus anchor,
    // mirroring how `Coordinator::maker` seeds `initializer.anchor_leaf` /
    // `initializer.high_qc` from production persistence. The test runner
    // tracks the anchor from `LeafDecided` events instead.
    if let Some(RestartAnchor {
        leaf: anchor_leaf,
        cert1: anchor_cert,
        epoch_leaves,
    }) = restart_anchor
    {
        // Replay decided epoch-root and epoch-transition leaves into the
        // fresh membership, mirroring `EpochManager::handle_leaf_decided`,
        // so the stake tables and DRBs for epochs established before the
        // restart are available again (production loads these via
        // `load_start_epoch_info`).
        for leaf in &epoch_leaves {
            let block_number = BlockHeader::<TestTypes>::block_number(leaf.block_header());
            let epoch = leaf.epoch(epoch_height).expect("epoch leaf has an epoch");
            if is_epoch_root(block_number, epoch_height) {
                membership
                    .membership()
                    .add_epoch_root(leaf.block_header().clone())
                    .await
                    .expect("seed epoch root");
            }
            if is_transition_block(block_number, epoch_height)
                && let Some(drb) = leaf.next_drb_result
            {
                membership.supply_drb(epoch + 1, drb);
            }
        }
        let anchor_view = anchor_leaf.view_number();
        let anchor_epoch = anchor_leaf
            .epoch(epoch_height)
            .unwrap_or(EpochNumber::genesis());
        let anchor_proposal = Proposal {
            block_header: anchor_leaf.block_header().clone(),
            view_number: anchor_view,
            epoch: anchor_epoch,
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
        state_manager.seed_state(
            anchor_view,
            Arc::new(
                <TestValidatedState as ValidatedState<TestTypes>>::from_header(
                    anchor_leaf.block_header(),
                ),
            ),
            anchor_leaf.clone(),
        );
        // If the anchor is an epoch-root block, extending it requires the
        // matching state certificate (epoch-root atomicity invariant); the
        // node's storage persisted it when the root was decided.
        if let Some(state_cert) = storage.state_cert_cloned().await {
            consensus.seed_state_cert(state_cert);
        }
        let reconstructed =
            match BlockHeader::<TestTypes>::payload_commitment(anchor_leaf.block_header()) {
                VidCommitment::V2(commitment) => Some((anchor_view, commitment)),
                _ => None,
            };
        consensus.seed_parent(anchor_cert, anchor_proposal, reconstructed);
        consensus.set_view(anchor_view, anchor_epoch);
    }

    // Resume from the persisted restart view, mirroring production where
    // `Coordinator::maker` seeds the guard from the `HotShotInitializer`.
    // A fresh storage holds genesis views, making this a no-op.
    consensus.seed_restart_guard(
        storage.restart_view().await,
        storage.last_actioned_view().await,
    );

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
