//! End-to-end cutover tests: legacy + new-protocol clusters run
//! concurrently per node. Each runner keeps its coordinator parked until
//! legacy crosses the upgrade boundary, then extracts the seed and starts
//! the coordinator — the same activation flow production drives from
//! `ConsensusHandle::activate`.

use std::{
    collections::{BTreeMap, BTreeSet},
    net::Ipv4Addr,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use async_lock::RwLock;
use cliquenet::noise::Protocol;
use committable::Committable;
use hotshot::{
    HotShotInitializer, SystemContext,
    types::{BLSPubKey, SystemContextHandle},
};
use hotshot_example_types::{
    membership::TestableMembership,
    node_types::{MemoryImpl, TestTypes},
    state_types::TestInstanceState,
    storage_types::TestStorage,
};
use hotshot_testing::{
    block_builder::{SimpleBuilderImplementation, TestBuilderImplementation},
    test_builder::TestDescription,
};
use hotshot_types::{
    PeerConnectInfo, ValidatorConfig,
    addr::NetAddr,
    consensus::ConsensusMetricsValue,
    data::ViewNumber,
    epoch_membership::EpochMembershipCoordinator,
    event::{Event, EventType},
    storage_metrics::StorageMetricsValue,
    traits::{
        leaf_fetcher_network::ConnectedNetworkLeafFetcher, metrics::NoMetrics,
        node_implementation::NodeType, signature_key::SignatureKey,
    },
    x25519::Keypair,
};
use tokio::{
    sync::mpsc::{self, UnboundedSender},
    task::AbortHandle,
    time::sleep,
};
use url::Url;
use versions::{NEW_PROTOCOL_VERSION, Upgrade, version};

use crate::{
    consensus::ConsensusOutput,
    coordinator::{Coordinator, error::Severity, timer::Timer},
    cutover::{extract_pre_cutover_seed, forward_legacy_high_qc, forward_legacy_timeout_votes},
    helpers::test_upgrade_lock,
    network::Cliquenet,
    outbox::Outbox,
    tests::common::{utils::mock_membership_with_client, views},
};

const UPGRADE_VIEW: u64 = 5;
const EPOCH_HEIGHT: u64 = 1000;
const DEFAULT_NEW_PROTO_VIEW_TIMEOUT: Duration = Duration::from_secs(6);

async fn spawn_legacy_cluster(
    num_nodes: usize,
    upgrade_view: u64,
) -> Vec<SystemContextHandle<TestTypes, MemoryImpl>> {
    let pre_cliquenet = version(NEW_PROTOCOL_VERSION.major, NEW_PROTOCOL_VERSION.minor - 1);
    let mut metadata: TestDescription<TestTypes, MemoryImpl> =
        TestDescription::default_multiple_rounds();
    metadata = metadata.set_num_nodes(num_nodes as u64, num_nodes as u64);
    metadata.upgrade = Upgrade::new(pre_cliquenet, NEW_PROTOCOL_VERSION);
    metadata.upgrade_view = Some(upgrade_view);
    metadata.test_config.epoch_height = EPOCH_HEIGHT;
    metadata.test_config.set_view_upgrade(upgrade_view);

    let port = test_utils::reserve_tcp_port().expect("port");
    let builder_url = Url::parse(&format!("http://localhost:{port}")).expect("url");
    let builder_task =
        <SimpleBuilderImplementation as TestBuilderImplementation<TestTypes>>::start(
            num_nodes,
            builder_url.clone(),
            (),
            Default::default(),
        )
        .await;
    Box::leak(Box::new(builder_task));

    let launcher = metadata.gen_launcher();
    let url_for_config = builder_url;
    let launcher = launcher.map_hotshot_config(move |config| {
        config.builder_urls = vec1::vec1![url_for_config.clone()];
    });

    let mut handles = Vec::with_capacity(num_nodes);
    for node_id in 0..num_nodes as u64 {
        let network = (launcher.resource_generators.channel_generator)(node_id).await;
        let storage = (launcher.resource_generators.storage)(node_id);
        let hotshot_config = (launcher.resource_generators.hotshot_config)(node_id);

        let is_da = node_id < hotshot_config.da_staked_committee_size as u64;
        let validator_config: ValidatorConfig<TestTypes> =
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                node_id,
                launcher.metadata.node_stakes.get(node_id),
                is_da,
            );
        let public_key = validator_config.public_key;
        let membership = <TestTypes as NodeType>::Membership::new(
            hotshot_config.known_nodes_with_stake.clone(),
            hotshot_config.known_da_nodes.clone(),
            public_key,
            launcher.metadata.test_config.epoch_height,
        );
        let external_chan = async_broadcast::broadcast(64);
        membership.set_leaf_fetcher(
            Arc::new(ConnectedNetworkLeafFetcher::<TestTypes, _>::new(
                Arc::clone(&network),
            )),
            storage.clone(),
            public_key,
            external_chan.1.new_receiver(),
        );
        let coordinator =
            EpochMembershipCoordinator::new(membership, hotshot_config.epoch_height, &storage);

        let initializer = HotShotInitializer::<TestTypes>::from_genesis(
            TestInstanceState::default(),
            launcher.metadata.test_config.epoch_height,
            launcher.metadata.test_config.epoch_start_block,
            vec![],
            launcher.metadata.upgrade,
        )
        .await
        .expect("initializer");

        let hotshot = SystemContext::<TestTypes, MemoryImpl>::new(
            public_key,
            validator_config.private_key.clone(),
            validator_config.state_private_key.clone(),
            node_id,
            hotshot_config,
            launcher.metadata.upgrade,
            coordinator,
            network,
            initializer,
            ConsensusMetricsValue::default(),
            storage,
            StorageMetricsValue::default(),
        )
        .await;

        let handle = hotshot.run_tasks().await;
        handles.push(handle);
    }
    handles
}

fn build_parties(num_nodes: usize) -> Vec<(Keypair, BLSPubKey, NetAddr)> {
    (0..num_nodes)
        .map(|i| {
            let (pk, sk) = BLSPubKey::generated_from_seed_indexed([0u8; 32], i as u64);
            let kp = Keypair::derive_from::<BLSPubKey>(&sk).unwrap();
            let port = test_utils::reserve_tcp_port().expect("port");
            let addr = NetAddr::Inet(Ipv4Addr::LOCALHOST.into(), port);
            (kp, pk, addr)
        })
        .collect()
}

async fn build_new_protocol_network(
    i: usize,
    parties: &[(Keypair, BLSPubKey, NetAddr)],
    lock: &hotshot_types::message::UpgradeLock<TestTypes>,
) -> Cliquenet<TestTypes> {
    let peer_infos: Vec<(BLSPubKey, PeerConnectInfo)> = parties
        .iter()
        .map(|(kp, pk, addr)| {
            (
                *pk,
                PeerConnectInfo {
                    x25519_key: kp.public_key(),
                    p2p_addr: addr.clone(),
                },
            )
        })
        .collect();
    let config = cliquenet::Config::builder()
        .name("legacy-cutover")
        .keypair(parties[i].0.clone().into())
        .bind(parties[i].2.clone())
        .random_connect_delay(false)
        .parties(
            peer_infos
                .iter()
                .map(|(_, info)| (info.x25519_key.into(), info.p2p_addr.clone())),
        )
        .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
        .build();
    let met = Box::new(NoMetrics);
    Cliquenet::create_with_config(parties[i].1, lock.clone(), config, peer_infos.clone(), met)
        .await
        .expect("cliquenet creation should succeed")
}

#[allow(clippy::too_many_arguments)]
async fn build_cutover_coordinator(
    node_index: u64,
    network: Cliquenet<TestTypes>,
    membership: EpochMembershipCoordinator<TestTypes>,
    storage: hotshot_example_types::storage_types::TestStorage<TestTypes>,
    client: crate::client::CoordinatorClient<TestTypes>,
    epoch_height: u64,
    view_timeout: Duration,
) -> Coordinator<TestTypes, TestStorage<TestTypes>> {
    use hotshot_example_types::{node_types::TEST_VERSIONS, state_types::TestValidatedState};
    use hotshot_types::{data::Leaf2, light_client::StateKeyPair};

    use crate::{
        block::{BlockBuilder, BlockBuilderConfig},
        consensus::Consensus,
        epoch::EpochManager,
        proposal::{ProposalValidator, VidShareValidator},
        state::StateManager,
        tests::common::coordinator_builder::{build_genesis_cert1, build_genesis_proposal},
        vid::VidReconstructor,
        vote::VoteCollector,
    };

    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    let state_key_pair = StateKeyPair::generate_from_seed_indexed([0u8; 32], node_index);
    let state_private_key = state_key_pair.sign_key_ref().clone();
    let instance = Arc::new(TestInstanceState::default());
    let upgrade_lock = test_upgrade_lock();

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

    let mut state_manager = StateManager::new(instance.clone(), upgrade_lock.clone());

    // Mirror production (`Coordinator::maker`): seed genesis cert/proposal
    // and genesis state so the coordinator can run consensus from view 1
    // alongside legacy until the cutover seed lands.
    let genesis_cert1 = build_genesis_cert1(&genesis_leaf);
    let genesis_proposal = build_genesis_proposal(&genesis_leaf, &genesis_cert1);
    let genesis_state = Arc::new(genesis_state);
    state_manager.seed_state(ViewNumber::genesis(), genesis_state.clone(), genesis_leaf);
    // The synthetic genesis proposal carries the genesis cert1 as its
    // justify_qc, so the leaf derived from it has a different commitment than
    // the natural `Leaf2::genesis`. `request_header` for view 1 looks up the
    // parent state by the proposal's leaf commitment, so seed under that
    // commitment too — otherwise the view-1 leader's header request never
    // completes and the new protocol cannot make progress before cutover.
    state_manager.seed_state(
        ViewNumber::genesis(),
        genesis_state,
        Leaf2::from(genesis_proposal.clone()),
    );
    consensus.seed_parent(genesis_cert1, genesis_proposal, std::iter::empty());

    let block_builder = BlockBuilder::new(
        instance.clone(),
        membership.clone(),
        network.sender().clone(),
        public_key,
        private_key.clone(),
        BlockBuilderConfig::default(),
        upgrade_lock.clone(),
    );

    let proposal_validator =
        ProposalValidator::new(membership.clone(), epoch_height, upgrade_lock.clone());
    let share_validator =
        VidShareValidator::new(membership.clone(), epoch_height, upgrade_lock.clone());

    Coordinator::builder()
        .consensus(consensus)
        .network(network)
        .state_manager(state_manager)
        .vote1_collector(VoteCollector::new(membership.clone(), upgrade_lock.clone()))
        .vote2_collector(VoteCollector::new(membership.clone(), upgrade_lock.clone()))
        .timeout_collector(VoteCollector::new(membership.clone(), upgrade_lock.clone()))
        .timeout_one_honest_collector(VoteCollector::new(membership.clone(), upgrade_lock.clone()))
        .epoch_root_collector(VoteCollector::new(membership.clone(), upgrade_lock.clone()))
        .vid_reconstructor(VidReconstructor::new())
        .epoch_manager(EpochManager::new(epoch_height, membership.clone()))
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
            hotshot_types::data::EpochNumber::genesis(),
        ))
        .public_key(public_key)
        .build()
}

#[derive(Clone, Debug)]
struct DecisionEvent {
    view: ViewNumber,
    commit: [u8; 32],
}

async fn run_cutover_node(
    mut coord: Coordinator<TestTypes, TestStorage<TestTypes>>,
    decision_tx: UnboundedSender<DecisionEvent>,
    external_events_tx: async_broadcast::Sender<Event<TestTypes>>,
    legacy: Arc<RwLock<SystemContextHandle<TestTypes, MemoryImpl>>>,
    new_proto_view: Arc<AtomicU64>,
) {
    // Mirror production (`ConsensusHandle::activate`): the coordinator
    // stays parked until legacy crosses the upgrade boundary; only then
    // is the seed extracted and the coordinator started.
    let seed = loop {
        let guard = legacy.read().await;
        let view = guard.cur_view().await;
        if guard.hotshot.upgrade_lock.new_protocol_active(view) {
            break extract_pre_cutover_seed(&guard).await;
        }
        drop(guard);
        sleep(Duration::from_millis(100)).await;
    };

    if seed.is_none() {
        tracing::warn!("seed extraction returned None; coordinator will not be seeded");
    }
    coord.start(seed);
    new_proto_view.store(*coord.current_view(), Ordering::Relaxed);

    loop {
        match coord.next_consensus_input().await {
            Ok(input) => coord.apply_consensus(input),
            Err(err) if err.severity == Severity::Critical => {
                tracing::error!(%err, "cutover coord: critical error");
                return;
            },
            Err(err) => tracing::warn!(%err, "cutover coord: non-critical error"),
        }
        // Publish progress for the silencers, which key node shutdowns off
        // the fastest live node's view (legacy or new-protocol).
        new_proto_view.store(*coord.current_view(), Ordering::Relaxed);

        while let Some(output) = coord.outbox_mut().pop_front() {
            if let ConsensusOutput::LeafDecided { leaves, .. } = &output {
                for leaf in leaves {
                    let _ = decision_tx.send(DecisionEvent {
                        view: leaf.view_number(),
                        commit: leaf.commit().into(),
                    });
                }
            }
            if let Err(err) = coord.process_consensus_output(output)
                && err.severity == Severity::Critical
            {
                tracing::error!(%err, "cutover coord: critical error processing output");
                return;
            }
        }

        while let Some(output) = coord.coordinator_outbox_mut().pop_front() {
            let _ = external_events_tx
                .broadcast_direct(Event {
                    view_number: coord.current_view(),
                    event: EventType::ExternalMessageReceived {
                        sender: output.sender,
                        data: output.data,
                    },
                })
                .await;
        }
    }
}

async fn spawn_node(
    i: usize,
    num_nodes: usize,
    view_timeout: Duration,
    parties: &[(Keypair, BLSPubKey, NetAddr)],
    new_proto_lock: &hotshot_types::message::UpgradeLock<TestTypes>,
    legacy: Arc<RwLock<SystemContextHandle<TestTypes, MemoryImpl>>>,
    bg_handles: &mut Vec<AbortHandle>,
) -> NodeState {
    let network = build_new_protocol_network(i, parties, new_proto_lock).await;
    let (membership, storage, client, external_events_tx) =
        mock_membership_with_client(num_nodes, EPOCH_HEIGHT, parties[i].1, Default::default());

    let coord = build_cutover_coordinator(
        i as u64,
        network,
        membership,
        storage,
        client,
        EPOCH_HEIGHT,
        view_timeout,
    )
    .await;

    let client_api = coord.client_api().clone();

    // Same lock production wires in: the legacy handle's, which receives the
    // decided upgrade certificate and opens the forwarders' cutover gate.
    let legacy_upgrade_lock = legacy.read().await.hotshot.upgrade_lock.clone();
    let legacy_event_rx = legacy.read().await.event_stream_known_impl().deactivate();
    bg_handles.push(
        tokio::spawn(forward_legacy_timeout_votes(
            legacy_event_rx.clone(),
            client_api.clone(),
            legacy_upgrade_lock.clone(),
            None,
        ))
        .abort_handle(),
    );
    bg_handles.push(
        tokio::spawn(forward_legacy_high_qc(
            legacy_event_rx,
            client_api.clone(),
            legacy_upgrade_lock,
        ))
        .abort_handle(),
    );

    let (decision_tx, decision_rx) = mpsc::unbounded_channel::<DecisionEvent>();
    let new_proto_view = Arc::new(AtomicU64::new(0));
    let runner_abort = tokio::spawn(run_cutover_node(
        coord,
        decision_tx,
        external_events_tx,
        legacy,
        new_proto_view.clone(),
    ))
    .abort_handle();

    NodeState {
        decision_rx,
        runner_abort,
        new_proto_view,
    }
}

struct NodeState {
    decision_rx: mpsc::UnboundedReceiver<DecisionEvent>,
    runner_abort: AbortHandle,
    /// Latest view this node's new-protocol coordinator has entered
    /// (0 while parked pre-cutover).
    new_proto_view: Arc<AtomicU64>,
}

struct SilentNode {
    idx: usize,
    /// Shut down once any other node's legacy or new-protocol view
    /// reaches this view.
    at_view: ViewNumber,
}

/// Verify every live node decides `target_decisions` views and that the
/// gaps inside each node's decided range are exactly
/// `expected_failed_views`: every gap must be listed, and every listed
/// view that falls inside the range must actually be a gap.
async fn run_cutover_test(
    num_nodes: usize,
    target_decisions: usize,
    expected_failed_views: BTreeSet<ViewNumber>,
    deadline: Duration,
    view_timeout: Duration,
    silent_nodes: Vec<SilentNode>,
) {
    crate::logging::init_test_logging();

    let parties = build_parties(num_nodes);
    let new_proto_lock = test_upgrade_lock();

    let legacy_handles = spawn_legacy_cluster(num_nodes, UPGRADE_VIEW).await;
    let legacy_arcs: Vec<Arc<RwLock<SystemContextHandle<TestTypes, MemoryImpl>>>> = legacy_handles
        .into_iter()
        .map(|h| Arc::new(RwLock::new(h)))
        .collect();

    let mut bg_handles: Vec<AbortHandle> = Vec::new();
    let mut node_state: Vec<NodeState> = Vec::with_capacity(num_nodes);
    for (i, legacy_arc) in legacy_arcs.iter().enumerate() {
        node_state.push(
            spawn_node(
                i,
                num_nodes,
                view_timeout,
                &parties,
                &new_proto_lock,
                legacy_arc.clone(),
                &mut bg_handles,
            )
            .await,
        );
    }

    let new_proto_views: Vec<Arc<AtomicU64>> = node_state
        .iter()
        .map(|ns| ns.new_proto_view.clone())
        .collect();
    for silent in &silent_nodes {
        bg_handles.push(spawn_silence_at_view(
            &legacy_arcs,
            &new_proto_views,
            silent,
            node_state[silent.idx].runner_abort.clone(),
        ));
    }

    for legacy in &legacy_arcs {
        legacy.read().await.hotshot.start_consensus().await;
    }

    let silent_idxs: BTreeSet<usize> = silent_nodes.iter().map(|s| s.idx).collect();
    let live_indices: Vec<usize> = (0..num_nodes)
        .filter(|i| !silent_idxs.contains(i))
        .collect();
    let mut decided_per_node: Vec<BTreeMap<ViewNumber, [u8; 32]>> =
        vec![BTreeMap::new(); num_nodes];
    let deadline = Instant::now() + deadline;
    while !live_indices
        .iter()
        .all(|i| decided_per_node[*i].len() >= target_decisions)
    {
        if Instant::now() > deadline {
            for (i, m) in decided_per_node.iter().enumerate() {
                tracing::error!(
                    node = i,
                    decided = m.len(),
                    views = ?m.keys().map(|v| **v).collect::<Vec<_>>(),
                    "node decisions at deadline",
                );
            }
            panic!("live nodes did not reach the post-cutover decision target in time");
        }
        for (i, ns) in node_state.iter_mut().enumerate() {
            while let Ok(ev) = ns.decision_rx.try_recv() {
                if decided_per_node[i].insert(ev.view, ev.commit).is_none() {
                    tracing::info!(node = i, view = *ev.view, "new-protocol decided leaf");
                }
            }
        }
        sleep(Duration::from_millis(50)).await;
    }

    // Each live node's `min..=max` decided range must match `expected_failed_views`.
    for &i in &live_indices {
        let decided: BTreeSet<ViewNumber> = decided_per_node[i].keys().copied().collect();
        let (&min_v, &max_v) = match (decided.iter().next(), decided.iter().last()) {
            (Some(min), Some(max)) => (min, max),
            _ => continue,
        };
        for v in *min_v..=*max_v {
            let view = ViewNumber::new(v);
            let in_chain = decided.contains(&view);
            let expected_fail = expected_failed_views.contains(&view);
            if !in_chain && !expected_fail {
                panic!(
                    "live node {i} skipped view {v} (between {} and {}) without it being in \
                     expected_failed_views={:?}",
                    *min_v,
                    *max_v,
                    expected_failed_views
                        .iter()
                        .map(|v| **v)
                        .collect::<Vec<_>>(),
                );
            }
            if in_chain && expected_fail {
                panic!(
                    "live node {i} committed view {v} but it was listed in \
                     expected_failed_views={:?}",
                    expected_failed_views
                        .iter()
                        .map(|v| **v)
                        .collect::<Vec<_>>(),
                );
            }
        }
    }

    let live_decided: Vec<&BTreeMap<ViewNumber, [u8; 32]>> =
        live_indices.iter().map(|i| &decided_per_node[*i]).collect();
    let common_views: BTreeSet<ViewNumber> =
        live_decided
            .iter()
            .skip(1)
            .fold(live_decided[0].keys().copied().collect(), |acc, m| {
                acc.intersection(&m.keys().copied().collect())
                    .copied()
                    .collect()
            });
    assert!(
        common_views.len() >= target_decisions,
        "live nodes do not agree on enough decided views: common={} target={target_decisions}",
        common_views.len(),
    );
    for view in &common_views {
        let commit = live_decided[0][view];
        for (k, m) in live_decided.iter().enumerate().skip(1) {
            assert_eq!(
                m[view], commit,
                "live node {} decided a different leaf than live node 0 at view {}",
                live_indices[k], **view
            );
        }
    }

    for w in bg_handles {
        w.abort();
    }
    for ns in &node_state {
        ns.runner_abort.abort();
    }
    for legacy in legacy_arcs {
        legacy.write().await.shut_down().await;
    }
}

/// Wait until any watched node's view — legacy `cur_view` or new-protocol
/// coordinator view — reaches `target_view`. Watching both sides is what
/// makes post-cutover silencing punctual: legacy parks at the cutover view
/// and only creeps past it via timeouts, long after the new protocol has
/// raced ahead.
async fn await_node_at_view(
    legacy: &[Arc<RwLock<SystemContextHandle<TestTypes, MemoryImpl>>>],
    new_proto: &[Arc<AtomicU64>],
    target_view: ViewNumber,
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() > deadline {
            panic!("watcher did not observe any view >= {target_view} in time");
        }
        if new_proto
            .iter()
            .any(|v| v.load(Ordering::Relaxed) >= *target_view)
        {
            return;
        }
        for legacy in legacy {
            if legacy.read().await.cur_view().await >= target_view {
                return;
            }
        }
        sleep(Duration::from_millis(20)).await;
    }
}

fn spawn_silence_at_view(
    legacy_arcs: &[Arc<RwLock<SystemContextHandle<TestTypes, MemoryImpl>>>],
    new_proto_views: &[Arc<AtomicU64>],
    silent: &SilentNode,
    runner_abort: AbortHandle,
) -> AbortHandle {
    let watch: Vec<_> = legacy_arcs
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != silent.idx)
        .map(|(_, l)| l.clone())
        .collect();
    let np_watch: Vec<_> = new_proto_views
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != silent.idx)
        .map(|(_, v)| v.clone())
        .collect();
    let target = silent.idx;
    let target_view = silent.at_view;
    let target_legacy = legacy_arcs[silent.idx].clone();
    tokio::spawn(async move {
        await_node_at_view(&watch, &np_watch, target_view, Duration::from_secs(120)).await;
        runner_abort.abort();
        target_legacy.write().await.shut_down().await;
        tracing::info!(node = target, at_view = *target_view, "took node offline");
    })
    .abort_handle()
}

/// `upgrade_view + TEST_UPGRADE_CONSTANTS.finish_offset`.
const PREDICTED_CUTOVER_VIEW: u64 = UPGRADE_VIEW + 20;

/// Last legacy view. Anchor for the permutation sweep below: each test
/// silences some subset of `{V-2, V-1, V, V+1, V+2}`.
const V: u64 = PREDICTED_CUTOVER_VIEW - 1;

/// Happy path. The QC for `cutover_view - 1` reaches the new protocol via
/// the cutover seed's `high_qc` or the `LegacyHighQcFormed` bridge, and the
/// first leader proposes on it. No view may fail: every leader is online.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_runs_upgrade_then_new_protocol_takes_over() {
    run_cutover_test(
        4,
        6,
        BTreeSet::new(),
        Duration::from_secs(180),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        Vec::new(),
    )
    .await;
}

/// Silence the leader of the last legacy view; exercises the
/// `TimeoutVote2` bridge into TC2.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_last_view_times_out_then_new_protocol_takes_over() {
    const NUM_NODES: usize = 4;
    let silent_idx = ((PREDICTED_CUTOVER_VIEW - 1) as usize) % NUM_NODES;
    run_cutover_test(
        NUM_NODES,
        6,
        views([PREDICTED_CUTOVER_VIEW - 2, PREDICTED_CUTOVER_VIEW - 1, 28]),
        Duration::from_secs(180),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        vec![SilentNode {
            idx: silent_idx,
            at_view: ViewNumber::new(PREDICTED_CUTOVER_VIEW - 2),
        }],
    )
    .await;
}

/// Silence two consecutive pre-cutover leaders; exercises TC2 chaining.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_two_views_view_sync_then_new_protocol_takes_over() {
    const NUM_NODES: usize = 7;
    let silent_n_minus_2 = ((PREDICTED_CUTOVER_VIEW - 2) as usize) % NUM_NODES;
    let silent_n_minus_1 = ((PREDICTED_CUTOVER_VIEW - 1) as usize) % NUM_NODES;
    let trigger = ViewNumber::new(PREDICTED_CUTOVER_VIEW - 3);
    run_cutover_test(
        NUM_NODES,
        6,
        views([
            PREDICTED_CUTOVER_VIEW - 3,
            PREDICTED_CUTOVER_VIEW - 2,
            PREDICTED_CUTOVER_VIEW - 1,
            30,
            31,
        ]),
        Duration::from_secs(240),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        vec![
            SilentNode {
                idx: silent_n_minus_2,
                at_view: trigger,
            },
            SilentNode {
                idx: silent_n_minus_1,
                at_view: trigger,
            },
        ],
    )
    .await;
}

/// Silence the leader of `cutover_view`; exercises native TC2 formation.
#[tokio::test(flavor = "multi_thread")]
async fn new_protocol_first_leader_offline_then_recovers() {
    const NUM_NODES: usize = 7;
    let silent_idx = (PREDICTED_CUTOVER_VIEW as usize) % NUM_NODES;
    run_cutover_test(
        NUM_NODES,
        6,
        views([PREDICTED_CUTOVER_VIEW - 1, PREDICTED_CUTOVER_VIEW]),
        Duration::from_secs(240),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        vec![SilentNode {
            idx: silent_idx,
            at_view: ViewNumber::new(PREDICTED_CUTOVER_VIEW),
        }],
    )
    .await;
}

/// Non-terminal legacy timeout: silence a pre-cutover leader several
/// views before cutover. Views 22 and 23 can never commit (votes for 22
/// die with the silenced leader of 23), and the dead node's post-cutover
/// slots (27, 31) time out. Every other view, including the boundary,
/// must be decided.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_view_before_last_times_out_then_new_protocol_takes_over() {
    const NUM_NODES: usize = 4;
    let silent_idx = ((PREDICTED_CUTOVER_VIEW - 2) as usize) % NUM_NODES;
    run_cutover_test(
        NUM_NODES,
        6,
        views([22, 23, 27, 31]),
        Duration::from_secs(240),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        vec![SilentNode {
            idx: silent_idx,
            at_view: ViewNumber::new(PREDICTED_CUTOVER_VIEW - 3),
        }],
    )
    .await;
}

// ============================================================
// Permutation sweep: timeouts in {V-2, V-1, V, V+1, V+2}.
//
// Five candidate views (`V-2..=V+2`) × {silenced, not} = 32 subsets.
// Existing tests above cover five of them (∅, {V-1}, {V}, {V+1},
// {V-1, V}); the 27 tests below cover the rest.

/// Pick a cluster size so that:
/// - all five candidate views have distinct leaders (≥5 nodes), and
/// - the cluster tolerates `n_silent` faults (n ≥ 3f+1).
fn perm_num_nodes(n_silent: usize) -> usize {
    match n_silent {
        0..=2 => 7,
        3 => 10,
        4 => 13,
        _ => 16,
    }
}

// Deadlines must stay under nextest's terminate-after ceiling (3 x 2m, .config/nextest.toml)
// so the deadline panic with per-node diagnostics fires before nextest kills the test.
fn perm_deadline(n_silent: usize) -> Duration {
    match n_silent {
        0..=2 => Duration::from_secs(240),
        3 => Duration::from_secs(300),
        _ => Duration::from_secs(350),
    }
}

/// Build a `SilentNode` that takes out the leader of `view`: the silencer
/// trips at `view - 1`, so the ~50ms kill lands well before the leader's
/// proposal for `view` (which needs at least a full round of cert
/// aggregation plus a block build). The silencer watches legacy and
/// new-protocol progress, so this holds on both sides of the cutover.
fn silent_for_view(view: u64, num_nodes: usize) -> SilentNode {
    SilentNode {
        idx: view as usize % num_nodes,
        at_view: ViewNumber::new(view - 1),
    }
}

/// The exact set of views that fail when the leaders of `views_to_silence`
/// are taken down. Every failure is attributable to a silenced leader:
/// - the silenced view itself (its leader never proposes),
/// - for silenced views at or before the cutover, the preceding view too
///   (legacy vote1s are unicast to the next leader, so they die with it;
///   post-cutover, vote1s are aggregated by the same view's leader, so
///   there is no such spillover), and
/// - every later leader slot of the silent node (dead from `at_view` on).
///
/// `max_view` bounds the slot enumeration; it only needs to exceed the
/// highest view a live node decides before the collection loop exits.
fn expected_failures(
    num_nodes: usize,
    views_to_silence: &[u64],
    max_view: u64,
) -> BTreeSet<ViewNumber> {
    let mut failed = BTreeSet::new();
    for &view in views_to_silence {
        let s = silent_for_view(view, num_nodes);
        failed.insert(ViewNumber::new(view));
        if view <= PREDICTED_CUTOVER_VIEW {
            failed.insert(ViewNumber::new(view - 1));
        }
        for v in *s.at_view..=max_view {
            if (v as usize) % num_nodes == s.idx {
                failed.insert(ViewNumber::new(v));
            }
        }
    }
    failed
}

/// Run a single permutation: silence the leader of every view in
/// `views_to_silence` and assert that exactly the attributable views
/// fail.
async fn run_perm_test(views_to_silence: Vec<u64>) {
    let n_silent = views_to_silence.len();
    let num_nodes = perm_num_nodes(n_silent);
    let expected = expected_failures(num_nodes, &views_to_silence, 50);
    let silent_nodes: Vec<SilentNode> = views_to_silence
        .iter()
        .map(|&v| silent_for_view(v, num_nodes))
        .collect();
    run_cutover_test(
        num_nodes,
        6,
        expected,
        perm_deadline(n_silent),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        silent_nodes,
    )
    .await;
}

// --- Singletons (2 of 5 not already covered) -----------------

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2() {
    run_perm_test(vec![V - 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_p2() {
    run_perm_test(vec![V + 2]).await;
}

// --- Pairs (9 of 10 not already covered) ---------------------

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_m1() {
    run_perm_test(vec![V - 2, V - 1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_v() {
    run_perm_test(vec![V - 2, V]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_p1() {
    run_perm_test(vec![V - 2, V + 1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_p2() {
    run_perm_test(vec![V - 2, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m1_p1() {
    run_perm_test(vec![V - 1, V + 1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m1_p2() {
    run_perm_test(vec![V - 1, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_v_p1() {
    run_perm_test(vec![V, V + 1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_v_p2() {
    run_perm_test(vec![V, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_p1_p2() {
    run_perm_test(vec![V + 1, V + 2]).await;
}

// --- Triples (10) -------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_m1_v() {
    run_perm_test(vec![V - 2, V - 1, V]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_m1_p1() {
    run_perm_test(vec![V - 2, V - 1, V + 1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_m1_p2() {
    run_perm_test(vec![V - 2, V - 1, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_v_p1() {
    run_perm_test(vec![V - 2, V, V + 1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_v_p2() {
    run_perm_test(vec![V - 2, V, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_p1_p2() {
    run_perm_test(vec![V - 2, V + 1, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m1_v_p1() {
    run_perm_test(vec![V - 1, V, V + 1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m1_v_p2() {
    run_perm_test(vec![V - 1, V, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m1_p1_p2() {
    run_perm_test(vec![V - 1, V + 1, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_v_p1_p2() {
    run_perm_test(vec![V, V + 1, V + 2]).await;
}

// --- Quads (5) ----------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_m1_v_p1() {
    run_perm_test(vec![V - 2, V - 1, V, V + 1]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_m1_v_p2() {
    run_perm_test(vec![V - 2, V - 1, V, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_m1_p1_p2() {
    run_perm_test(vec![V - 2, V - 1, V + 1, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m2_v_p1_p2() {
    run_perm_test(vec![V - 2, V, V + 1, V + 2]).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_m1_v_p1_p2() {
    run_perm_test(vec![V - 1, V, V + 1, V + 2]).await;
}

// --- Quint (1) ----------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn perm_silence_all() {
    run_perm_test(vec![V - 2, V - 1, V, V + 1, V + 2]).await;
}
