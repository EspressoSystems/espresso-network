use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::Result;
use hotshot::{traits::BlockPayload, types::BLSPubKey};
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_new_protocol::{
    block::{BlockBuilder, BlockBuilderConfig},
    client::CoordinatorClient,
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::{Coordinator, timer::Timer},
    epoch::EpochManager,
    epoch_root_vote_collector::EpochRootVoteCollector,
    helpers::proposal_commitment,
    leader_trace::LeaderTracerHandle,
    network::cliquenet::Cliquenet,
    outbox::Outbox,
    proposal::{ProposalValidator, VidShareValidator},
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
    traits::{metrics::NoMetrics, node_implementation::NodeType, signature_key::SignatureKey},
    x25519::Keypair,
};
use tracing::{error, info, warn};
use versions::{NEW_PROTOCOL_VERSION, Upgrade};

use crate::{
    config::NodeConfig, cpu_sampler::CpuSampler, leader_trace::CsvLeaderTracer,
    membership::make_membership, metrics::MetricsCollector,
};

type BenchCoordinator = Coordinator<TestTypes, Cliquenet<TestTypes>, TestStorage<TestTypes>>;

/// Build and run a single benchmark node.
pub async fn run(cfg: NodeConfig) -> Result<()> {
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], cfg.node_id);
    info!(node_id = cfg.node_id, %public_key, "starting node");

    let (membership, client) = make_membership(cfg.total_nodes, public_key).await;
    let network = create_network(cfg.node_id, &public_key, &private_key, &cfg).await?;

    // Per-node leader-event tracer. Production binaries leave this `None`; the
    // bench wires it through `Consensus::set_tracer` so every leader-duty call
    // site emits a wall-clock-ns stamp to disk for offline reconstruction.
    let trace_path = leader_trace_path(&cfg);
    let tracer = Arc::new(CsvLeaderTracer::new(cfg.node_id, trace_path));

    // Start the CPU sampler (no-op on non-Linux). Outputs land in the same
    // directory as the leader-trace CSV so analysis scripts can pick them up.
    let cpu_out_dir = PathBuf::from(&cfg.output_file)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_default();
    let cpu_sampler = CpuSampler::start(
        cfg.node_id,
        cpu_out_dir,
        Duration::from_millis(cfg.sampler_tick_ms),
    );

    let coordinator = build_coordinator(
        public_key,
        private_key,
        membership,
        network,
        client,
        &cfg,
        tracer.clone() as LeaderTracerHandle,
    )
    .await;

    let result = run_instrumented(coordinator, &cfg).await;
    if let Err(err) = tracer.flush() {
        warn!(%err, "failed to flush leader trace");
    }
    cpu_sampler.stop().await;
    result
}

fn leader_trace_path(cfg: &NodeConfig) -> PathBuf {
    let out = PathBuf::from(&cfg.output_file);
    let dir = out.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    dir.join(format!("leader_trace_node{}.csv", cfg.node_id))
}

async fn create_network(
    node_id: u64,
    public_key: &BLSPubKey,
    private_key: &<BLSPubKey as SignatureKey>::PrivateKey,
    cfg: &NodeConfig,
) -> Result<Cliquenet<TestTypes>> {
    let keypair = Keypair::derive_from::<BLSPubKey>(private_key)?;
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
        let peer_keypair = Keypair::derive_from::<BLSPubKey>(&peer_sk)?;
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
        upgrade_lock(),
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
    network: Cliquenet<TestTypes>,
    client: CoordinatorClient<TestTypes>,
    cfg: &NodeConfig,
    tracer: LeaderTracerHandle,
) -> BenchCoordinator {
    let instance = Arc::new(TestInstanceState::default());
    let epoch_height = u64::MAX;

    let genesis_state = TestValidatedState::default();
    let genesis_leaf =
        Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;
    let upgrade_lock = bench_upgrade_lock();

    let state_key_pair = hotshot_types::light_client::StateKeyPair::generate_from_seed_indexed(
        [0u8; 32],
        cfg.node_id,
    );
    let state_private_key = state_key_pair.sign_key_ref().clone();

    let mut consensus = Consensus::new(
        membership.clone(),
        public_key,
        private_key.clone(),
        state_private_key,
        cfg.total_nodes,
        upgrade_lock.clone(),
        genesis_leaf.clone(),
        epoch_height,
        100,
    );
    consensus.set_tracer(Some(tracer.clone()));

    let vote1_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let vote2_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_one_honest_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let checkpoint_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let epoch_root_collector =
        EpochRootVoteCollector::new(membership.clone(), upgrade_lock.clone());

    let epoch_manager = EpochManager::new(epoch_height, membership.clone());

    let vid_disperser = VidDisperser::new(membership.clone());
    let mut vid_reconstructor = VidReconstructor::new();
    vid_reconstructor.set_tracer(Some(tracer.clone()));

    let block_config = BlockBuilderConfig::default();
    let block_builder = BlockBuilder::new(
        instance.clone(),
        membership.clone(),
        block_config,
        upgrade_lock.clone(),
    );

    let mut state_manager = StateManager::new(instance.clone(), upgrade_lock.clone());
    let genesis_state = Arc::new(genesis_state);
    state_manager.seed_state(
        ViewNumber::genesis(),
        genesis_state.clone(),
        genesis_leaf.clone(),
    );

    // Seed consensus with genesis cert + proposal so the view-1 leader
    // can self-start without external injection from the orchestrator.
    let genesis_cert1 = build_genesis_cert1(&genesis_leaf);
    let genesis_proposal = build_genesis_proposal(&genesis_leaf, &genesis_cert1);
    // The synthetic genesis proposal has a non-null justify_qc so the leaf
    // derived from it has a different commitment than `genesis_leaf`.
    // `request_header` for view 1 looks up the parent state by the
    // proposal's leaf commitment, so seed the same state under that
    // commitment too (mirrors coordinator builder behavior).
    state_manager.seed_state(
        ViewNumber::genesis(),
        genesis_state,
        Leaf2::from(genesis_proposal.clone()),
    );
    consensus.seed_genesis(genesis_cert1, genesis_proposal);

    let proposal_validator =
        ProposalValidator::new(membership.clone(), epoch_height, upgrade_lock.clone());
    let share_validator =
        VidShareValidator::new(membership.clone(), epoch_height, upgrade_lock.clone());

    let timer = Timer::new(
        cfg.timeout_duration(),
        ViewNumber::genesis(),
        EpochNumber::genesis(),
    );

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
        .share_validator(share_validator)
        .storage(hotshot_new_protocol::storage::Storage::new(
            TestStorage::default(),
            private_key,
        ))
        .client(client)
        .membership_coordinator(membership)
        .outbox(Outbox::new())
        .timer(timer)
        .public_key(public_key)
        .build();

    // Emit initial ViewChanged and (for the leader) RequestBlockAndHeader.
    coordinator.start();

    // Process initial outputs so the timer resets before the event loop.
    while let Some(output) = coordinator.outbox_mut().pop_front() {
        if let Err(e) = coordinator.process_consensus_output(output) {
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
                coordinator.apply_consensus(input);
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
                let block = build_test_block(cfg.block_size, cfg.total_nodes, cfg.namespaces);
                let parent_leaf = req.parent_proposal.clone().into();
                let version = bench_upgrade_lock().version_infallible(req.view);
                let header = TestBlockHeader::new::<TestTypes>(
                    &parent_leaf,
                    block.payload_commitment,
                    block.builder_commitment,
                    block.metadata,
                    version,
                );
                let header_input = ConsensusInput::HeaderCreated(
                    req.view,
                    proposal_commitment(&req.parent_proposal),
                    header,
                );
                metrics.on_input(&header_input);
                coordinator.apply_consensus(header_input);
                let block_input = ConsensusInput::BlockBuilt {
                    view: req.view,
                    epoch: req.epoch,
                    payload: block.block,
                    metadata: block.metadata,
                };
                metrics.on_input(&block_input);
                coordinator.apply_consensus(block_input);
                continue; // skip process_consensus_output for this one
            }

            if let Err(err) = coordinator.process_consensus_output(output) {
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

/// Per-transaction byte size when splitting the configured `--block-size` into
/// many small transactions.  1 KiB matches realistic rollup-style traffic and
/// — critically — turns `transaction_commitments` into a long Vec of small
/// Keccak256 calls that `TestBlockPayload::transaction_commitments` parallelizes
/// over the rayon pool, instead of one giant single-threaded Keccak.
const BENCH_TX_BYTES: usize = 1024;

fn build_test_block(size: usize, num_nodes: usize, n_namespaces: u32) -> TestBlock {
    use hotshot_types::traits::EncodeBytes;

    // Split the configured payload into many BENCH_TX_BYTES-byte transactions.
    // At least one tx so an empty `--block-size=0` config still produces a
    // valid (small) payload.
    let num_txs = (size / BENCH_TX_BYTES).max(1);
    let transactions: Vec<TestTransaction> = (0..num_txs)
        .map(|_| TestTransaction::new(vec![0u8; BENCH_TX_BYTES]))
        .collect();
    let block = TestBlockPayload { transactions };
    let encoded = block.encode();

    // `TestMetadata` itself emits the namespace table when `num_transactions
    // > 1` AND `payload_byte_len > 0`. The bench sets both so AvidM dispersal
    // splits the payload into N evenly-sized namespaces and parallelizes
    // per-namespace via rayon (same wire format as production `NsTable`).
    //
    // NOTE: `metadata.num_transactions` here is being repurposed as the
    // namespace count for the wiring trick; it is independent of the actual
    // `block.transactions.len()` (which is now ≈ size / 1 KiB).
    let n = n_namespaces.max(1);
    let metadata = TestMetadata {
        num_transactions: n as u64,
        payload_byte_len: if n > 1 { encoded.len() as u64 } else { 0 },
    };
    let payload_commitment = hotshot_types::data::vid_commitment(
        &encoded,
        &metadata.encode(),
        num_nodes,
        versions::NEW_PROTOCOL_VERSION,
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
    UpgradeLock::new(Upgrade::trivial(NEW_PROTOCOL_VERSION))
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
        data,
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

pub fn upgrade_lock<T: NodeType>() -> UpgradeLock<T> {
    UpgradeLock::new(NEW_PROTOCOL_VERSION.into())
}
