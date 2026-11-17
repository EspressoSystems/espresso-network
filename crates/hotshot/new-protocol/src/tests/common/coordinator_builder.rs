use std::{marker::PhantomData, sync::Arc, time::Duration};

use committable::Committable;
use consensus_store::mem::MemStore;
use hotshot_example_types::{
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_types::{
    data::{EpochNumber, Leaf2, QuorumProposalWrapper, ViewChangeEvidence2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    light_client::StateKeyPair,
    message::Proposal as SignedProposal,
    signature_key::BLSPubKey,
    simple_certificate::UpgradeCertificate2,
    simple_vote::QuorumData2,
    traits::signature_key::SignatureKey,
    upgrade_config::UpgradeConfig,
};
use tokio::sync::mpsc;

use super::utils::reconstructed_blocks;
use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    cert_verifier::CertVerifiers,
    client::CoordinatorClient,
    consensus::Consensus,
    coordinator::{Coordinator, timer::Timer},
    discovery::{ServiceBook, ServiceRecord},
    epoch::EpochManager,
    helpers::test_upgrade_lock,
    message::{Certificate1, OpaqueMessage, Proposal},
    network::Cliquenet,
    outbox::Outbox,
    proposal::{ProposalValidator, VidShareValidator},
    state::StateManager,
    store::ConsensusStore,
    upgrade::UpgradeProtocol,
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

/// A node's upgrade lock and window configuration. Every node needs its own
/// lock, shared between its network (wire versioning) and its coordinator.
pub struct UpgradeSetup {
    pub lock: hotshot_types::message::UpgradeLock<TestTypes>,
    pub config: UpgradeConfig,
}

impl Default for UpgradeSetup {
    fn default() -> Self {
        Self {
            lock: test_upgrade_lock(),
            config: UpgradeConfig::default(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn build_test_coordinator(
    node_index: u64,
    network: Cliquenet<TestTypes>,
    membership: EpochMembershipCoordinator<TestTypes>,
    storage: TestStorage<TestTypes>,
    store: ConsensusStore<TestTypes, MemStore>,
    client: CoordinatorClient<TestTypes>,
    epoch_height: u64,
    view_timeout: Duration,
    upgrade: UpgradeSetup,
    external_tx: mpsc::UnboundedSender<OpaqueMessage<BLSPubKey>>,
) -> Coordinator<TestTypes, MemStore> {
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    let state_key_pair = StateKeyPair::generate_from_seed_indexed([0u8; 32], node_index);
    let state_private_key = state_key_pair.sign_key_ref().clone();
    let instance = Arc::new(TestInstanceState::default());
    let UpgradeSetup {
        lock: upgrade_lock,
        config: upgrade_config,
    } = upgrade;

    let epoch_manager = EpochManager::new(epoch_height, membership.clone());

    let vote1_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let vote2_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_one_honest_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let epoch_root_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());

    let genesis_state = TestValidatedState::default();
    let genesis_leaf =
        Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;

    // A node restarting with persistent storage resumes from the anchor in
    // its consensus store, as in production; otherwise it starts from genesis.
    let restored = store.load().expect("load consensus store");
    let restart_anchor = restored
        .anchor
        .clone()
        .map(|anchor| (anchor.leaf, anchor.cert1));
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

    let vid_disperser = VidDisperser::new(
        membership.clone(),
        network.sender().clone(),
        public_key,
        private_key.clone(),
    );
    let vid_reconstructor = VidReconstructor::new();

    let block_builder = BlockBuilder::new(
        instance.clone(),
        membership.clone(),
        BlockBuilderConfig {
            empty_block_delay: Duration::from_secs(1),
            ..BlockBuilderConfig::default()
        },
        upgrade_lock.clone(),
    );

    let mut state_manager = StateManager::new(instance.clone(), upgrade_lock.clone());
    let genesis_state = Arc::new(genesis_state);
    state_manager.seed_state(
        ViewNumber::genesis(),
        genesis_state.clone(),
        genesis_leaf.clone(),
    );

    // Build a genesis cert1 and proposal so consensus can self-start.
    let genesis_cert1 = build_genesis_cert1(&genesis_leaf);
    let genesis_proposal = build_genesis_proposal(&genesis_leaf);

    let anchor_view = if let Some((anchor_leaf, anchor_cert)) = restart_anchor {
        // Seed the persisted anchor as the parent, exactly as production's
        // `Coordinator::maker` does from the initializer: rebuild the parent
        // proposal from the anchor leaf, seed a sparse state for it, and
        // treat the anchor plus persisted proposals as already-reconstructed
        // blocks so the first leader after restart is not stalled by the
        // `parent_block_reconstructed` check.
        let anchor_view = anchor_leaf.view_number();
        let anchor_epoch = anchor_leaf
            .epoch(epoch_height)
            .unwrap_or(EpochNumber::genesis());
        let anchor_proposal = Proposal {
            block_header: anchor_leaf.block_header().clone(),
            view_number: anchor_view,
            epoch: anchor_epoch,
            justify_qc: anchor_leaf.justify_qc().clone(),
            next_epoch_justify_qc: None,
            upgrade_certificate: anchor_leaf
                .upgrade_certificate()
                .cloned()
                .map(|cert| UpgradeCertificate2::restore_epoch(cert, anchor_epoch)),
            view_change_evidence: anchor_leaf
                .view_change_evidence
                .clone()
                .and_then(ViewChangeEvidence2::timeout_evidence),
            next_drb_result: anchor_leaf.next_drb_result,
            state_cert: None,
        };
        let anchor_state = <TestValidatedState as hotshot_types::traits::states::ValidatedState<
            TestTypes,
        >>::from_header(anchor_leaf.block_header());
        state_manager.seed_state(anchor_view, Arc::new(anchor_state), anchor_leaf.clone());
        let reconstructed = reconstructed_blocks(
            std::iter::once((anchor_view, anchor_leaf.block_header().clone())).chain(
                restored
                    .proposals
                    .iter()
                    .map(|(view, p)| (*view, p.data.block_header.clone())),
            ),
        );
        // Seed persisted proposals before `seed_parent` so its authoritative
        // anchor wins (mirrors `Coordinator::maker`).
        consensus.seed_signed_proposals(restored.proposals.values().cloned());
        consensus.seed_parent(anchor_cert, anchor_proposal, reconstructed);
        if let Some(cert2) = restored.anchor.as_ref().and_then(|a| a.cert2.clone()) {
            consensus.seed_anchor_cert2(cert2);
        }
        anchor_view
    } else {
        // This genesis leaf is from before epochs, so the leaf rebuilt from the
        // genesis proposal commits differently. `request_header` for view 1
        // looks up the parent state by the proposal's leaf commitment, so seed
        // the genesis state under that commitment too.
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

    // As in `Coordinator::maker`: restore the lock, the boundary certificate
    // and the stored payloads, and never act again in a view acted in before.
    if let Some(locked_qc) = restored.locked_qc {
        consensus.seed_locked_cert(locked_qc);
    }
    if let Some(boundary_qc) = restored.boundary_qc {
        consensus.seed_boundary_cert(boundary_qc);
    }
    consensus.seed_stored_payloads(restored.payloads);
    let restart_view = restored
        .restart_view
        .map_or(anchor_view, |view| view.max(anchor_view));
    consensus.resume_from_restart(
        anchor_view,
        restart_view,
        restored.acted_view.unwrap_or_else(ViewNumber::genesis),
    );

    // A leader proposing on an epoch-root parent QC right after restart
    // needs the persisted light-client state cert.
    if let Some(state_cert) = restored.state_cert {
        consensus.seed_state_cert(state_cert);
    }

    let genesis_signed = SignedProposal::<TestTypes, Proposal<TestTypes>> {
        data: genesis_proposal.clone(),
        signature: BLSPubKey::sign(&private_key, &[]).expect("sign genesis"),
        _pd: PhantomData,
    };
    let mut batch = consensus_store::Batch::new();
    crate::store::put_proposal(&mut batch, &genesis_signed).expect("encode genesis proposal");
    consensus_store::Store::apply(store.store().as_ref(), &batch).expect("seed genesis proposal");
    // The application storage too: its leaf fetcher serves the genesis leaf
    // to peers catching up on the first epochs' stake tables.
    let genesis_wrapper = SignedProposal {
        data: QuorumProposalWrapper::from(genesis_proposal.clone()),
        signature: genesis_signed.signature.clone(),
        _pd: PhantomData,
    };
    storage
        .append_proposal_wrapper(&genesis_wrapper)
        .await
        .expect("seed genesis proposal");

    let proposal_validator = ProposalValidator::new(
        membership.clone(),
        epoch_height,
        upgrade_lock.clone(),
        consensus.cert1_at(ViewNumber::genesis()),
    );
    let share_validator =
        VidShareValidator::new(membership.clone(), epoch_height, upgrade_lock.clone());

    let service_record = ServiceRecord {
        epoch: EpochNumber::genesis(),
        api_url: format!("http://node{node_index}.test:24000"),
        api_key: None,
        sequence: 1,
    };
    let service_book = ServiceBook::new(store.load().expect("load").service_records);

    let mut coordinator = Coordinator::builder()
        .consensus(consensus)
        .network(network)
        .state_manager(state_manager)
        .vote1_collector(vote1_collector)
        .vote2_collector(vote2_collector)
        .timeout_collector(timeout_collector)
        .timeout_one_honest_collector(timeout_one_honest_collector)
        .timeout3_collector(VoteCollector::new(membership.clone(), upgrade_lock.clone()))
        .timeout_one_honest3_collector(VoteCollector::new(membership.clone(), upgrade_lock.clone()))
        .epoch_root_collector(epoch_root_collector)
        .upgrade_vote_collector(VoteCollector::new(membership.clone(), upgrade_lock.clone()))
        .upgrade_protocol(UpgradeProtocol::new(
            upgrade_config,
            upgrade_lock.clone(),
            public_key,
            private_key.clone(),
        ))
        .cert_verifiers(CertVerifiers::new(membership.clone(), upgrade_lock.clone()))
        .vid_disperser(vid_disperser)
        .vid_reconstructor(vid_reconstructor)
        .epoch_manager(epoch_manager)
        .block_builder(block_builder)
        .proposal_validator(proposal_validator)
        .share_validator(share_validator)
        .storage(crate::storage::Storage::new(store, &private_key))
        .client(client)
        .membership_coordinator(membership)
        .outbox(Outbox::new())
        .external_tx(external_tx)
        .timer(Timer::new(view_timeout, ViewNumber::genesis()))
        .public_key(public_key)
        .service_book(service_book)
        .service_record((service_record, private_key.clone()))
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

/// Create a genesis `Proposal` from the genesis leaf, as `Coordinator::maker`
/// builds the anchor proposal: with the leaf's own `justify_qc`. For a leaf of
/// an epoch version, as a new-protocol anchor is, the proposal then commits as
/// the leaf does.
pub(crate) fn build_genesis_proposal(genesis_leaf: &Leaf2<TestTypes>) -> Proposal<TestTypes> {
    Proposal {
        block_header: genesis_leaf.block_header().clone(),
        view_number: ViewNumber::genesis(),
        epoch: EpochNumber::genesis(),
        justify_qc: genesis_leaf.justify_qc().clone(),
        next_epoch_justify_qc: None,
        upgrade_certificate: None,
        view_change_evidence: None,
        next_drb_result: None,
        state_cert: None,
    }
}
