use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use hotshot::{
    traits::{BlockPayload, implementations::Cliquenet},
    types::BLSPubKey,
};
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_new_protocol::{
    block::{BlockBuilder, BlockBuilderConfig},
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::{Coordinator, timer::Timer},
    epoch::EpochManager,
    network::Network,
    outbox::Outbox,
    proposal::ProposalValidator,
    state::StateManager,
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};
use hotshot_types::{
    PeerConnectInfo,
    addr::NetAddr,
    data::{EpochNumber, Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    traits::{metrics::NoMetrics, signature_key::SignatureKey},
    x25519::Keypair,
};
use tracing::{error, info, warn};
use versions::{CLIQUENET_VERSION, Upgrade};

use crate::{config::NodeConfig, membership::make_membership, metrics::MetricsCollector};

type BenchCoordinator = Coordinator<TestTypes, Cliquenet<BLSPubKey>, TestStorage<TestTypes>>;

/// Build and run a single benchmark node.
pub async fn run(cfg: NodeConfig) -> Result<()> {
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], cfg.node_id);
    info!(node_id = cfg.node_id, %public_key, "starting node");

    let membership = make_membership(cfg.total_nodes).await;
    let network = create_network(cfg.node_id, &public_key, &private_key, &cfg).await?;

    let coordinator = build_coordinator(public_key, private_key, membership, network, &cfg).await;

    run_instrumented(coordinator, &cfg).await
}

async fn create_network(
    node_id: u64,
    public_key: &BLSPubKey,
    private_key: &<BLSPubKey as SignatureKey>::PrivateKey,
    cfg: &NodeConfig,
) -> Result<Cliquenet<BLSPubKey>> {
    let keypair = Keypair::derive_from::<BLSPubKey>(private_key);
    let bind_addr: NetAddr = cfg
        .bind_addr
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind address '{}': {e}", cfg.bind_addr))?;

    let mut parties = Vec::new();
    for (i, addr_str) in cfg.peers.iter().enumerate() {
        let i = i as u64;
        if i == node_id {
            continue; // skip self
        }
        let (peer_pk, peer_sk) = BLSPubKey::generated_from_seed_indexed([0u8; 32], i);
        let peer_keypair = Keypair::derive_from::<BLSPubKey>(&peer_sk);
        let peer_addr: NetAddr = addr_str
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid peer address '{addr_str}': {e}"))?;
        parties.push((
            peer_pk,
            PeerConnectInfo {
                x25519_key: peer_keypair.public_key(),
                p2p_addr: peer_addr,
            },
        ));
    }

    let net = Cliquenet::create(
        "bench",
        *public_key,
        keypair,
        bind_addr,
        parties,
        Box::new(NoMetrics),
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to create cliquenet: {e}"))?;

    Ok(net)
}

async fn build_coordinator(
    public_key: BLSPubKey,
    private_key: <BLSPubKey as SignatureKey>::PrivateKey,
    membership: EpochMembershipCoordinator<TestTypes>,
    network: Cliquenet<BLSPubKey>,
    cfg: &NodeConfig,
) -> BenchCoordinator {
    let instance = Arc::new(TestInstanceState::default());
    let epoch_height = u64::MAX;

    let genesis_state = TestValidatedState::default();
    let genesis_leaf =
        Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;
    let upgrade_lock = bench_upgrade_lock();

    let mut consensus = Consensus::new(
        membership.clone(),
        public_key,
        private_key.clone(),
        upgrade_lock.clone(),
        genesis_leaf.clone(),
        epoch_height,
    );

    let vote1_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let vote2_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_one_honest_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let checkpoint_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());

    let epoch_manager = EpochManager::new(epoch_height, membership.clone());

    let vid_disperser = VidDisperser::new(membership.clone());
    let vid_reconstructor = VidReconstructor::new();

    let block_config = BlockBuilderConfig::default();
    let block_builder = BlockBuilder::new(
        instance.clone(),
        membership.clone(),
        block_config,
        upgrade_lock.clone(),
    );

    let mut state_manager = StateManager::new(instance.clone(), upgrade_lock.clone());
    state_manager.seed_state(
        ViewNumber::genesis(),
        Arc::new(genesis_state),
        genesis_leaf.clone(),
    );

    // Seed consensus with genesis cert + proposal so the view-1 leader
    // can self-start without external injection from the orchestrator.
    let genesis_cert1 = build_genesis_cert1(&genesis_leaf);
    let genesis_proposal = build_genesis_proposal(&genesis_leaf, &genesis_cert1);
    consensus.seed_genesis(genesis_cert1, genesis_proposal);

    let proposal_validator = ProposalValidator::new(membership.clone(), upgrade_lock.clone());

    let net = Network::new(network, membership.clone(), upgrade_lock);

    let timer = Timer::new(
        cfg.timeout_duration(),
        ViewNumber::genesis(),
        EpochNumber::genesis(),
    );

    let mut coordinator = Coordinator::builder()
        .consensus(consensus)
        .network(net)
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
        .storage(hotshot_new_protocol::storage::Storage::new(
            TestStorage::default(),
            private_key,
        ))
        .membership_coordinator(membership)
        .outbox(Outbox::new())
        .timer(timer)
        .public_key(public_key)
        .build();

    // Emit initial ViewChanged and (for the leader) RequestBlockAndHeader.
    coordinator.start().await;

    // Process initial outputs so the timer resets before the event loop.
    while let Some(output) = coordinator.outbox_mut().pop_front() {
        if let Err(e) = coordinator.process_consensus_output(output).await {
            warn!(%e, "error processing initial output");
        }
    }

    coordinator
}

/// Run coordinator with metrics instrumentation and block injection.
async fn run_instrumented(mut coordinator: BenchCoordinator, cfg: &NodeConfig) -> Result<()> {
    let mut metrics = MetricsCollector::new(cfg.node_id);
    let output_path = PathBuf::from(&cfg.output_file);

    info!(
        node_id = cfg.node_id,
        target_views = cfg.target_views,
        "entering event loop"
    );

    loop {
        match coordinator.next_consensus_input().await {
            Ok(input) => {
                metrics.on_input(&input);
                coordinator.apply_consensus(input).await;
            },
            Err(err)
                if err.severity == hotshot_new_protocol::coordinator::error::Severity::Critical =>
            {
                error!(%err, "critical error in consensus input");
                metrics.write_csv(&output_path)?;
                return Err(anyhow::anyhow!("{err}"));
            },
            Err(err) => {
                warn!(%err, "recoverable error in consensus input");
                continue;
            },
        }

        while let Some(output) = coordinator.outbox_mut().pop_front() {
            metrics.on_output(&output);

            // Intercept block requests and inject test block (bypassing BlockBuilder).
            if let ConsensusOutput::RequestBlockAndHeader(ref req) = output
                && cfg.block_size > 0
            {
                let block = build_test_block(cfg.block_size, cfg.total_nodes);
                let parent_leaf = req.parent_proposal.clone().into();
                let version = bench_upgrade_lock().version_infallible(req.view);
                let header = TestBlockHeader::new::<TestTypes>(
                    &parent_leaf,
                    block.payload_commitment,
                    block.builder_commitment,
                    block.metadata,
                    version,
                );
                let header_input = ConsensusInput::HeaderCreated(req.view, header);
                metrics.on_input(&header_input);
                coordinator.apply_consensus(header_input).await;
                let block_input = ConsensusInput::BlockBuilt {
                    view: req.view,
                    epoch: req.epoch,
                    payload: block.block,
                    metadata: block.metadata,
                };
                metrics.on_input(&block_input);
                coordinator.apply_consensus(block_input).await;
                continue; // skip process_consensus_output for this one
            }

            if let Err(err) = coordinator.process_consensus_output(output).await {
                if err.severity == hotshot_new_protocol::coordinator::error::Severity::Critical {
                    error!(%err, "critical error processing output");
                    metrics.write_csv(&output_path)?;
                    return Err(anyhow::anyhow!("{err}"));
                }
                warn!(%err, "recoverable error processing output");
            }
        }

        // Check after processing all outputs for this round.
        let decided = metrics.max_decided_view();
        if decided >= cfg.target_views {
            info!(
                node_id = cfg.node_id,
                decided_view = decided,
                "target views reached, shutting down"
            );
            metrics.write_csv(&output_path)?;
            return Ok(());
        }
    }
}

/// Build a test block with a single transaction of the given size.
struct TestBlock {
    block: TestBlockPayload,
    metadata: TestMetadata,
    payload_commitment: hotshot_types::data::VidCommitment,
    builder_commitment: hotshot_types::utils::BuilderCommitment,
}

fn build_test_block(size: usize, num_nodes: usize) -> TestBlock {
    use hotshot_types::traits::EncodeBytes;

    let tx = TestTransaction::new(vec![0u8; size]);
    let block = TestBlockPayload {
        transactions: vec![tx],
    };
    let metadata = TestMetadata {
        num_transactions: 1,
    };
    // Use the actual committee size so the commitment matches what
    // VidDisperse::calculate_vid_disperse will produce.
    let payload_commitment = hotshot_types::data::vid_commitment(
        &block.encode(),
        &metadata.encode(),
        num_nodes,
        versions::VID2_UPGRADE_VERSION,
    );
    let builder_commitment =
        <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(&block, &metadata);
    TestBlock {
        block,
        metadata,
        payload_commitment,
        builder_commitment,
    }
}

fn bench_upgrade_lock() -> UpgradeLock<TestTypes> {
    UpgradeLock::new(Upgrade::trivial(CLIQUENET_VERSION))
}

/// Create a genesis `Certificate1` that references the genesis leaf.
fn build_genesis_cert1(
    genesis_leaf: &Leaf2<TestTypes>,
) -> hotshot_new_protocol::message::Certificate1<TestTypes> {
    use committable::Committable;
    use hotshot_types::simple_vote::QuorumData2;

    let data = QuorumData2 {
        leaf_commit: genesis_leaf.commit(),
        epoch: Some(EpochNumber::genesis()),
        block_number: Some(0),
    };
    hotshot_new_protocol::message::Certificate1::new(
        data.clone(),
        data.commit(),
        ViewNumber::genesis(),
        None,
        std::marker::PhantomData,
    )
}

/// Create a genesis `Proposal` from the genesis leaf and cert.
fn build_genesis_proposal(
    genesis_leaf: &Leaf2<TestTypes>,
    genesis_cert1: &hotshot_new_protocol::message::Certificate1<TestTypes>,
) -> hotshot_new_protocol::message::Proposal<TestTypes> {
    hotshot_new_protocol::message::Proposal {
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
