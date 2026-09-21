use std::{path::PathBuf, sync::Arc};

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
    cert_verifier::CertVerifiers,
    client::CoordinatorClient,
    consensus::{Consensus, ConsensusInput, ConsensusOutput},
    coordinator::{Coordinator, timer::Timer},
    epoch::EpochManager,
    helpers::proposal_commitment,
    network::Cliquenet,
    outbox::Outbox,
    proposal::{ProposalValidator, VidShareValidator},
    state::StateManager,
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};
use hotshot_types::{
    PeerConnectInfo,
    addr::NetAddr,
    data::{EpochNumber, Leaf2, VidCommitment, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    traits::{metrics::NoMetrics, node_implementation::NodeType, signature_key::SignatureKey},
    utils::BuilderCommitment,
    vid::avidm_gf2::AvidmGf2Scheme,
    x25519::Keypair,
};
use tokio::select;
use tracing::{error, info, warn};
use vbs::version::Version;
use versions::{NEW_PROTOCOL_VERSION, Upgrade};

use crate::{config::NodeConfig, membership::make_membership, metrics::MetricsCollector};

type BenchCoordinator = Coordinator<TestTypes, TestStorage<TestTypes>>;

/// Build and run a single benchmark node.
pub async fn run(cfg: NodeConfig) -> Result<()> {
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0u8; 32], cfg.node_id);
    info!(node_id = cfg.node_id, %public_key, "starting node");

    let (membership, client) = make_membership(cfg.total_nodes, public_key).await;
    let network = create_network(cfg.node_id, &public_key, &private_key, &cfg).await?;

    let disperser = BenchDisperser {
        network: network.sender().clone(),
        membership: membership.clone(),
        public_key,
        private_key: private_key.clone(),
    };

    let coordinator =
        build_coordinator(public_key, private_key, membership, network, client, &cfg).await;

    run_instrumented(coordinator, &cfg, disperser).await
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
    );

    let vote1_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let vote2_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_one_honest_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout3_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let timeout_one_honest3_collector =
        VoteCollector::new(membership.clone(), upgrade_lock.clone());
    let epoch_root_collector = VoteCollector::new(membership.clone(), upgrade_lock.clone());

    let epoch_manager = EpochManager::new(epoch_height, membership.clone());

    let vid_disperser = VidDisperser::new(
        membership.clone(),
        network.sender().clone(),
        public_key,
        private_key.clone(),
    );

    let vid_reconstructor = VidReconstructor::new();

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
    consensus.seed_parent(genesis_cert1, genesis_proposal, std::iter::empty());

    let proposal_validator =
        ProposalValidator::new(membership.clone(), epoch_height, upgrade_lock.clone());
    let share_validator =
        VidShareValidator::new(membership.clone(), epoch_height, upgrade_lock.clone());

    let timer = Timer::new(cfg.timeout_duration(), ViewNumber::genesis());

    let mut coordinator = Coordinator::builder()
        .consensus(consensus)
        .network(network)
        .state_manager(state_manager)
        .vote1_collector(vote1_collector)
        .vote2_collector(vote2_collector)
        .timeout_collector(timeout_collector)
        .timeout_one_honest_collector(timeout_one_honest_collector)
        .timeout3_collector(timeout3_collector)
        .timeout_one_honest3_collector(timeout_one_honest3_collector)
        .epoch_root_collector(epoch_root_collector)
        .cert_verifiers(CertVerifiers::new(membership.clone(), upgrade_lock.clone()))
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
    coordinator.start(None);

    // Process initial outputs so the timer resets before the event loop.
    while let Some(output) = coordinator.outbox_mut().pop_front() {
        if let Err(e) = coordinator.process_consensus_output(output) {
            warn!(%e, "error processing initial output");
        }
    }

    coordinator
}

/// Run coordinator with metrics instrumentation and block injection.
async fn run_instrumented(
    mut coordinator: BenchCoordinator,
    cfg: &NodeConfig,
    disperser: BenchDisperser,
) -> Result<()> {
    let mut metrics = MetricsCollector::new(cfg.node_id);
    let output_path = PathBuf::from(&cfg.output_file);
    let mut builds: tokio::task::JoinSet<Result<(PendingBuild, TestBlock, Dispersal)>> =
        tokio::task::JoinSet::new();

    info!(
        node_id = cfg.node_id,
        target_views = cfg.target_views,
        "entering event loop"
    );

    loop {
        // A finished build must be able to wake this loop on its own: while it
        // runs, every other node is waiting on this node's proposal, so no
        // consensus input arrives to drive a poll. The coordinator's own select
        // does the same for the production builder (`block_builder.next()`).
        // A finished build must be able to wake this loop on its own: while it
        // runs, every other node is waiting on this node's proposal, so no
        // consensus input arrives to drive a poll. The coordinator's own select
        // does the same for the production builder (`block_builder.next()`).
        select! {
            // Biased: a finished build gates the proposal, so collect it before
            // servicing more input. Without this the arms are chosen at random,
            // and under heavy input traffic a completed block sits uncollected —
            // which showed up as a non-monotonic build window on the fleet.
            biased;

            Some(done) = builds.join_next(), if !builds.is_empty() => {
                let (pending, block, dispersal) = done??;

                // Fan the shares out on a background task: the proposal is gated
                // on the header, not on the network sends.
                let d = disperser.clone();
                let (view, epoch) = (pending.view, pending.epoch);
                tokio::task::spawn_blocking(move || {
                    if let Err(err) = hotshot_new_protocol::vid::fan_out::<TestTypes>(
                        dispersal.shares,
                        dispersal.common,
                        dispersal.commitment,
                        dispersal.recipients,
                        view,
                        epoch,
                        d.network,
                        d.public_key,
                        d.private_key,
                    ) {
                        error!(%view, %err, "bench vid fanout failed");
                    }
                });

                let header = TestBlockHeader::new::<TestTypes>(
                    &pending.parent_leaf,
                    block.payload_commitment,
                    // `TestBlockPayload::builder_commitment` is a serial SHA-256
                    // over the whole payload, and nothing on this path reads it
                    // back: no validator recomputes or compares it. The
                    // placeholder matches `utils.rs`.
                    BuilderCommitment::from_bytes([]),
                    block.metadata,
                    pending.version,
                );
                let header_input =
                    ConsensusInput::HeaderCreated(pending.view, pending.parent_commitment, header);
                metrics.on_input(&header_input);
                coordinator.apply_consensus(header_input);
                let block_input = ConsensusInput::BlockBuilt {
                    view: pending.view,
                    epoch: pending.epoch,
                    payload: block.block,
                    metadata: block.metadata,
                    payload_commitment: block.payload_commitment,
                };
                metrics.on_input(&block_input);
                coordinator.apply_consensus(block_input);
            },
            input = coordinator.next_consensus_input() => match input {
                Ok(input) => {
                    metrics.on_input(&input);
                    coordinator.apply_consensus(input);
                },
                Err(err)
                    if err.severity
                        == hotshot_new_protocol::coordinator::error::Severity::Critical =>
                {
                    error!(%err, "critical error in consensus input");
                    metrics.write_csv(&output_path)?;
                    return Err(anyhow::anyhow!("{err}"));
                },
                Err(err) => {
                    warn!(%err, "recoverable error in consensus input");
                    continue;
                },
            },
        }

        while let Some(output) = coordinator.outbox_mut().pop_front() {
            metrics.on_output(&output);

            // Intercept block requests and inject a test block, bypassing
            // `BlockBuilder`. Production builds on a spawned task
            // (`BlockBuilder::request_block`) and collects the result later, so
            // this does the same: building inline would stall the coordinator for
            // the whole build, starving the share and vote traffic this node is
            // handling as a replica of the previous view.
            if let ConsensusOutput::RequestBlockAndHeader(ref req) = output
                && cfg.block_size > 0
            {
                let pending = PendingBuild {
                    view: req.view,
                    epoch: req.epoch,
                    parent_leaf: req.parent_proposal.clone().into(),
                    parent_commitment: proposal_commitment(&req.parent_proposal),
                    version: bench_upgrade_lock().version_infallible(req.view),
                };
                let (size, namespaces) = (cfg.block_size, cfg.namespaces);
                let d = disperser.clone();
                let (view, epoch) = (req.view, req.epoch);
                builds.spawn_blocking(move || {
                    let (block, dispersal) = build_test_block(size, namespaces, &d, view, epoch)?;
                    Ok((pending, block, dispersal))
                });
                continue; // skip process_consensus_output for this one
            }

            // The bench already erasure-coded and fanned this block out as part
            // of building it, so the coordinator's disperse request would be a
            // second pass over the same payload. Drop it.
            if matches!(output, ConsensusOutput::RequestVidDisperse { .. }) && cfg.block_size > 0 {
                continue;
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

/// Everything the leader needs to erasure-code a block and fan its shares out.
#[derive(Clone)]
struct BenchDisperser {
    network: hotshot_new_protocol::network::Sender<TestTypes>,
    membership: EpochMembershipCoordinator<TestTypes>,
    public_key: BLSPubKey,
    private_key: <BLSPubKey as SignatureKey>::PrivateKey,
}

/// The encode's output, handed to the fan-out after the header is formed so the
/// proposal is not gated on the network sends.
struct Dispersal {
    shares: Vec<hotshot_types::vid::avidm_gf2::AvidmGf2Share>,
    common: hotshot_types::vid::avidm_gf2::AvidmGf2Common,
    commitment: hotshot_types::vid::avidm_gf2::AvidmGf2Commitment,
    recipients: Vec<BLSPubKey>,
}

/// What a spawned block build needs to hand back so the header can be formed
/// once it lands.
struct PendingBuild {
    view: ViewNumber,
    epoch: EpochNumber,
    parent_leaf: Leaf2<TestTypes>,
    parent_commitment: committable::Commitment<Leaf2<TestTypes>>,
    version: Version,
}

/// Size of each synthetic transaction in a bench block.
///
/// Recovery's `transaction_commitments` is a Keccak256 per transaction, so many
/// small transactions spread that cost over the rayon pool instead of running
/// one giant single-threaded hash over the whole payload.
const BENCH_TX_BYTES: usize = 1024;

/// A freshly built test block. Deliberately not cached across views: reusing one
/// block would hand every view an already-warm payload and hide the assembly
/// cost the benchmark is there to measure.
struct TestBlock {
    block: TestBlockPayload,
    metadata: TestMetadata,
    payload_commitment: hotshot_types::data::VidCommitment,
}

fn build_test_block(
    size: usize,
    n_namespaces: u32,
    disperser: &BenchDisperser,
    view: ViewNumber,
    epoch: EpochNumber,
) -> Result<(TestBlock, Dispersal)> {
    use hotshot_types::traits::EncodeBytes;

    // Split the configured payload into BENCH_TX_BYTES-byte transactions, with at
    // least one so `--block-size 0` still yields a valid (tiny) payload.
    let num_txs = size.div_ceil(BENCH_TX_BYTES).max(1);
    let mut transactions = Vec::with_capacity(num_txs);
    transactions.resize_with(num_txs, || TestTransaction::new(vec![0u8; BENCH_TX_BYTES]));
    let block = TestBlockPayload { transactions };
    let encoded = block.encode();

    // `TestMetadata` emits a namespace table when `num_transactions > 1` and
    // `payload_byte_len > 0`. Both are set here so AvidM splits the payload into
    // `n_namespaces` namespaces and parallelizes across them.
    //
    // NOTE: `num_transactions` is repurposed as the namespace count for that
    // wiring; it is independent of `block.transactions.len()`.
    let n = n_namespaces.max(1);
    let metadata = TestMetadata {
        num_transactions: n as u64,
        payload_byte_len: if n > 1 { encoded.len() as u64 } else { 0 },
    };
    // One erasure-code pass yields both the payload commitment and the shares.
    // Deriving the commitment with `vid_commitment` instead would encode the
    // whole payload a second time, and the disperser would then encode it again
    // — the leader paying for three passes over a multi-MB block.
    let params = hotshot_types::data::VidDisperse2::<TestTypes>::disperse_params(
        &block,
        &disperser.membership,
        Some(epoch),
        &metadata,
    )?;
    let (commitment, common, shares) = AvidmGf2Scheme::ns_disperse(
        &params.param,
        &params.weights,
        &params.payload,
        params.ns_table.iter().cloned(),
    )
    .map_err(|err| anyhow::anyhow!("ns_disperse: {err}"))?;

    Ok((
        TestBlock {
            block,
            metadata,
            payload_commitment: VidCommitment::V2(commitment),
        },
        Dispersal {
            shares,
            common,
            commitment,
            recipients: params.recipients,
        },
    ))
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
