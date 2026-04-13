use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use hotshot::{traits::implementations::Cliquenet, types::BLSPubKey};
use hotshot_example_types::{
    node_types::{CliquenetImpl, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_new_protocol::{
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
use hotshot_types::{
    PeerConnectInfo,
    addr::NetAddr,
    data::{EpochNumber, Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{metrics::NoMetrics, signature_key::SignatureKey},
    x25519::Keypair,
};
use tracing::{error, info, warn};

use crate::{config::NodeConfig, membership::make_membership, metrics::MetricsCollector};

type BenchCoordinator = Coordinator<TestTypes, CliquenetImpl>;

/// Build and run a single benchmark node.
pub async fn run(cfg: NodeConfig) -> Result<()> {
    let (public_key, private_key) =
        BLSPubKey::generated_from_seed_indexed([cfg.seed; 32], cfg.node_id);
    info!(node_id = cfg.node_id, %public_key, "starting node");

    let membership = make_membership(cfg.total_nodes, cfg.seed).await;
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

    // Build peer list: for each peer address, derive the corresponding public key
    // and x25519 key from the deterministic seed.
    let mut parties = Vec::new();
    for (i, addr_str) in cfg.peers.iter().enumerate() {
        let i = i as u64;
        if i == node_id {
            continue; // skip self
        }
        let (_peer_pk, peer_sk) = BLSPubKey::generated_from_seed_indexed([cfg.seed; 32], i);
        let peer_pk = BLSPubKey::from_private(&peer_sk);
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

    let mut consensus = Consensus::new(membership.clone(), public_key, private_key, epoch_height);

    let vote1_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let vote2_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let timeout_one_honest_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let checkpoint_collector = VoteCollector::new(membership.clone(), upgrade_lock());

    let epoch_manager = EpochManager::new(epoch_height, membership.clone());

    let vid_disperser = VidDisperser::new(membership.clone());
    let vid_reconstructor = VidReconstructor::new();

    let block_config = BlockBuilderConfig::default();
    let block_builder = BlockBuilder::new(instance.clone(), membership.clone(), block_config);

    let mut state_manager = StateManager::new(instance.clone());
    let genesis_state = TestValidatedState::default();
    let genesis_leaf = Leaf2::<TestTypes>::genesis(
        &genesis_state,
        &instance,
        hotshot_example_types::node_types::TEST_VERSIONS.test.base,
    )
    .await;
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

    let proposal_validator = ProposalValidator::new(membership.clone());

    let net = Network::new(network, membership.clone(), upgrade_lock());

    let timer = Timer::new(cfg.timeout_duration(), ViewNumber::genesis(), EpochNumber::genesis());

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
        .membership_coordinator(membership)
        .outbox(Outbox::new())
        .timer(timer)
        .public_key(public_key)
        .build();

    // Emit initial ViewChanged and (for the leader) RequestBlockAndHeader.
    coordinator.start().await;

    // Process initial outputs so the timer resets before the event loop.
    while let Some(output) = coordinator.outbox_mut().pop_front() {
        let _ = coordinator.process_consensus_output(output).await;
    }

    coordinator
}

/// Run the coordinator with metrics instrumentation.
async fn run_instrumented(mut coordinator: BenchCoordinator, cfg: &NodeConfig) -> Result<()> {
    let mut metrics = MetricsCollector::new(cfg.node_id);

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
                break;
            },
            Err(err) => {
                warn!(%err, "recoverable error in consensus input");
                continue;
            },
        }

        while let Some(output) = coordinator.outbox_mut().pop_front() {
            metrics.on_output(&output);

            if let Err(err) = coordinator.process_consensus_output(output).await {
                if err.severity == hotshot_new_protocol::coordinator::error::Severity::Critical {
                    error!(%err, "critical error processing output");
                    let path = PathBuf::from(&cfg.output_file);
                    metrics.write_csv(&path)?;
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
            let path = PathBuf::from(&cfg.output_file);
            metrics.write_csv(&path)?;
            return Ok(());
        }
    }

    let path = PathBuf::from(&cfg.output_file);
    metrics.write_csv(&path)?;
    Ok(())
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
