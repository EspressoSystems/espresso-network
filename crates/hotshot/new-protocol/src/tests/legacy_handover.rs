//! End-to-end handover tests: legacy + new-protocol clusters run
//! concurrently per node. The new-protocol coordinator drives a
//! [`HandoverGate`] on every loop iteration — the same gating call site
//! production uses from [`ConsensusHandle::new_protocol`].

use std::{
    collections::{BTreeMap, BTreeSet},
    net::Ipv4Addr,
    sync::Arc,
    time::{Duration, Instant},
};

use async_lock::RwLock;
use committable::Committable;
use hotshot::{
    HotShotInitializer, SystemContext,
    types::{BLSPubKey, SystemContextHandle},
};
use hotshot_example_types::{
    membership::TestableMembership,
    node_types::{MemoryImpl, TestTypes},
    state_types::TestInstanceState,
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
        leaf_fetcher_network::ConnectedNetworkLeafFetcher, node_implementation::NodeType,
        signature_key::SignatureKey,
    },
    x25519::Keypair,
};
use tokio::{
    sync::mpsc::{self, UnboundedSender},
    task::AbortHandle,
    time::sleep,
};
use url::Url;
use versions::{CLIQUENET_VERSION, Upgrade, version};

use crate::{
    consensus::ConsensusOutput,
    coordinator::{Coordinator, CoordinatorOutput, error::Severity, timer::Timer},
    harvest::{HandoverGate, forward_legacy_timeout_votes},
    helpers::test_upgrade_lock,
    network::cliquenet::Cliquenet,
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
    let pre_cliquenet = version(CLIQUENET_VERSION.major, CLIQUENET_VERSION.minor - 1);
    let mut metadata: TestDescription<TestTypes, MemoryImpl> =
        TestDescription::default_multiple_rounds();
    metadata = metadata.set_num_nodes(num_nodes as u64, num_nodes as u64);
    metadata.upgrade = Upgrade::new(pre_cliquenet, CLIQUENET_VERSION);
    metadata.upgrade_view = Some(upgrade_view);
    metadata.test_config.epoch_height = EPOCH_HEIGHT;
    metadata.test_config.set_view_upgrade(upgrade_view);
    metadata.test_config.upgrade_propose_offset = Some(1);
    metadata.test_config.upgrade_decide_by_offset = Some(10);
    metadata.test_config.upgrade_begin_offset = Some(12);
    metadata.test_config.upgrade_finish_offset = Some(15);

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
        .name("legacy-handover")
        .keypair(parties[i].0.clone().into())
        .bind(parties[i].2.clone())
        .random_connect_delay(false)
        .parties(
            peer_infos
                .iter()
                .map(|(_, info)| (info.x25519_key.into(), info.p2p_addr.clone())),
        )
        .build();
    Cliquenet::create_with_config(parties[i].1, lock.clone(), config, peer_infos.clone())
        .await
        .expect("cliquenet creation should succeed")
}

#[allow(clippy::too_many_arguments)]
async fn build_handover_coordinator(
    node_index: u64,
    network: Cliquenet<TestTypes>,
    membership: EpochMembershipCoordinator<TestTypes>,
    storage: hotshot_example_types::storage_types::TestStorage<TestTypes>,
    client: crate::client::CoordinatorClient<TestTypes>,
    epoch_height: u64,
    view_timeout: Duration,
) -> Coordinator<
    TestTypes,
    Cliquenet<TestTypes>,
    hotshot_example_types::storage_types::TestStorage<TestTypes>,
> {
    use hotshot_example_types::{node_types::TEST_VERSIONS, state_types::TestValidatedState};
    use hotshot_types::{data::Leaf2, light_client::StateKeyPair};

    use crate::{
        block::{BlockBuilder, BlockBuilderConfig},
        consensus::Consensus,
        epoch::EpochManager,
        epoch_root_vote_collector::EpochRootVoteCollector,
        proposal::{ProposalValidator, VidShareValidator},
        state::StateManager,
        tests::common::coordinator_builder::{build_genesis_cert1, build_genesis_proposal},
        vid::{VidDisperser, VidReconstructor},
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
    consensus.seed_genesis(genesis_cert1, genesis_proposal);
    state_manager.seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);

    let block_builder = BlockBuilder::new(
        instance.clone(),
        membership.clone(),
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
        .checkpoint_collector(VoteCollector::new(membership.clone(), upgrade_lock.clone()))
        .epoch_root_collector(EpochRootVoteCollector::new(
            membership.clone(),
            upgrade_lock.clone(),
        ))
        .vid_disperser(VidDisperser::new(membership.clone()))
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

async fn run_handover_node(
    mut coord: Coordinator<
        TestTypes,
        Cliquenet<TestTypes>,
        hotshot_example_types::storage_types::TestStorage<TestTypes>,
    >,
    decision_tx: UnboundedSender<DecisionEvent>,
    external_events_tx: async_broadcast::Sender<Event<TestTypes>>,
    legacy: Arc<RwLock<SystemContextHandle<TestTypes, MemoryImpl>>>,
    handover_gate: HandoverGate,
) {
    // Mirror production (`consensus_handle::run_coordinator`): kick the
    // coordinator before pumping the event loop so it runs from genesis
    // until the cutover seed lands.
    coord.start();
    let client_api = coord.client_api().clone();

    loop {
        // Mirror production (`ConsensusHandle::new_protocol`): poll the
        // handover gate on each iteration so the shared latching path
        // is exercised, not a test-only watcher task.
        if !handover_gate.is_active() {
            let guard = legacy.read().await;
            handover_gate.check(&guard, &client_api).await;
        }

        match coord.next_consensus_input().await {
            Ok(input) => coord.apply_consensus(input),
            Err(err) if err.severity == Severity::Critical => {
                tracing::error!(%err, "handover coord: critical error");
                return;
            },
            Err(err) => tracing::warn!(%err, "handover coord: non-critical error"),
        }

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
                tracing::error!(%err, "handover coord: critical error processing output");
                return;
            }
        }

        while let Some(output) = coord.coordinator_outbox_mut().pop_front() {
            if let CoordinatorOutput::ExternalMessageReceived { sender, data } = output {
                let _ = external_events_tx
                    .broadcast_direct(Event {
                        view_number: coord.current_view(),
                        event: EventType::ExternalMessageReceived { sender, data },
                    })
                    .await;
            }
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
        mock_membership_with_client(num_nodes, EPOCH_HEIGHT, parties[i].1).await;

    let coord = build_handover_coordinator(
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

    let legacy_event_rx = legacy.read().await.event_stream_known_impl().deactivate();
    bg_handles.push(
        tokio::spawn(forward_legacy_timeout_votes(
            legacy_event_rx,
            client_api.clone(),
        ))
        .abort_handle(),
    );

    let (decision_tx, decision_rx) = mpsc::unbounded_channel::<DecisionEvent>();
    let runner_abort = tokio::spawn(run_handover_node(
        coord,
        decision_tx,
        external_events_tx,
        legacy,
        HandoverGate::new(),
    ))
    .abort_handle();

    NodeState {
        decision_rx,
        runner_abort,
    }
}

struct NodeState {
    decision_rx: mpsc::UnboundedReceiver<DecisionEvent>,
    runner_abort: AbortHandle,
}

struct SilentNode {
    idx: usize,
    /// Shut down once any other node's `cur_view` reaches this view.
    at_view: ViewNumber,
}

/// Verify every live node decides `target_decisions` views with no
/// gap outside `expected_failed_views`.
///
/// When `loose` is `false`, the assertion is exact: every view in
/// `expected_failed_views` that falls inside the decided range must
/// actually be a gap. When `loose` is `true`, `expected_failed_views`
/// is interpreted as a permitted-failures superset — gaps must lie
/// inside it, but predicted views that ended up being decided do not
/// trip the assertion. The loose variant is used by the permutation
/// sweep below, where the exact post-cutover failure pattern is hard
/// to predict precisely.
async fn run_handover_test(
    num_nodes: usize,
    target_decisions: usize,
    expected_failed_views: BTreeSet<ViewNumber>,
    deadline: Duration,
    view_timeout: Duration,
    silent_nodes: Vec<SilentNode>,
    loose: bool,
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

    for silent in &silent_nodes {
        bg_handles.push(spawn_silence_at_view(
            &legacy_arcs,
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
            if !loose && in_chain && expected_fail {
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

async fn await_legacy_view(
    watch: &[Arc<RwLock<SystemContextHandle<TestTypes, MemoryImpl>>>],
    target_view: ViewNumber,
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() > deadline {
            panic!("watcher did not observe cur_view >= {target_view} in time");
        }
        for legacy in watch {
            if legacy.read().await.cur_view().await >= target_view {
                return;
            }
        }
        sleep(Duration::from_millis(20)).await;
    }
}

fn spawn_silence_at_view(
    legacy_arcs: &[Arc<RwLock<SystemContextHandle<TestTypes, MemoryImpl>>>],
    silent: &SilentNode,
    runner_abort: AbortHandle,
) -> AbortHandle {
    let watch: Vec<_> = legacy_arcs
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != silent.idx)
        .map(|(_, l)| l.clone())
        .collect();
    let target = silent.idx;
    let target_view = silent.at_view;
    let target_legacy = legacy_arcs[silent.idx].clone();
    tokio::spawn(async move {
        await_legacy_view(&watch, target_view, Duration::from_secs(120)).await;
        runner_abort.abort();
        target_legacy.write().await.shut_down().await;
        tracing::info!(node = target, at_view = *target_view, "took node offline");
    })
    .abort_handle()
}

/// `upgrade_view + upgrade_finish_offset`.
const PREDICTED_CUTOVER_VIEW: u64 = UPGRADE_VIEW + 15;

/// Last legacy view — naturally TC2-skipped at handover. Used as the
/// anchor for the permutation sweep below: each test silences some
/// subset of `{V-2, V-1, V, V+1, V+2}`.
const V: u64 = PREDICTED_CUTOVER_VIEW - 1;

/// Happy path. `cutover_view - 1` reliably has no QC at handover, so
/// the new protocol skips it via TC2.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_runs_upgrade_then_new_protocol_takes_over() {
    run_handover_test(
        4,
        6,
        views([PREDICTED_CUTOVER_VIEW - 1]),
        Duration::from_secs(180),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        Vec::new(),
        false,
    )
    .await;
}

/// Silence the leader of the last legacy view; exercises the
/// `TimeoutVote2` bridge into TC2.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_last_view_times_out_then_new_protocol_takes_over() {
    const NUM_NODES: usize = 4;
    let silent_idx = ((PREDICTED_CUTOVER_VIEW - 1) as usize) % NUM_NODES;
    run_handover_test(
        NUM_NODES,
        6,
        views([PREDICTED_CUTOVER_VIEW - 2, PREDICTED_CUTOVER_VIEW - 1, 23]),
        Duration::from_secs(180),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        vec![SilentNode {
            idx: silent_idx,
            at_view: ViewNumber::new(PREDICTED_CUTOVER_VIEW - 2),
        }],
        false,
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
    run_handover_test(
        NUM_NODES,
        6,
        views([
            PREDICTED_CUTOVER_VIEW - 3,
            PREDICTED_CUTOVER_VIEW - 2,
            PREDICTED_CUTOVER_VIEW - 1,
            25,
            26,
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
        false,
    )
    .await;
}

/// Silence the leader of `cutover_view`; exercises native TC2 formation.
#[tokio::test(flavor = "multi_thread")]
async fn new_protocol_first_leader_offline_then_recovers() {
    const NUM_NODES: usize = 7;
    let silent_idx = (PREDICTED_CUTOVER_VIEW as usize) % NUM_NODES;
    run_handover_test(
        NUM_NODES,
        6,
        views([PREDICTED_CUTOVER_VIEW - 1, PREDICTED_CUTOVER_VIEW]),
        Duration::from_secs(240),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        vec![SilentNode {
            idx: silent_idx,
            at_view: ViewNumber::new(PREDICTED_CUTOVER_VIEW),
        }],
        false,
    )
    .await;
}

/// Non-terminal legacy timeout: silence view 18's leader. View 16 must
/// be decided by the new protocol via the seeded `justify_qc` chain
/// walk — the regression this test guards against.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_view_before_last_times_out_then_new_protocol_takes_over() {
    const NUM_NODES: usize = 4;
    let silent_idx = ((PREDICTED_CUTOVER_VIEW - 2) as usize) % NUM_NODES;
    run_handover_test(
        NUM_NODES,
        6,
        views([17, 18, 19, 22, 26]),
        Duration::from_secs(240),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        vec![SilentNode {
            idx: silent_idx,
            at_view: ViewNumber::new(PREDICTED_CUTOVER_VIEW - 3),
        }],
        false,
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

fn perm_deadline(n_silent: usize) -> Duration {
    match n_silent {
        0..=2 => Duration::from_secs(240),
        3 => Duration::from_secs(300),
        4 => Duration::from_secs(360),
        _ => Duration::from_secs(480),
    }
}

/// Build a `SilentNode` that takes out the leader of `view`. For
/// pre-cutover views we trip the silencer at `view - 1` so the node is
/// gone before its leader slot; post-cutover, legacy doesn't reliably
/// advance far past `PREDICTED_CUTOVER_VIEW`, so use `view` itself
/// (matching `new_protocol_first_leader_offline_then_recovers`).
fn silent_for_view(view: u64, num_nodes: usize) -> SilentNode {
    let silent_idx = view as usize % num_nodes;
    let at_view = if view < PREDICTED_CUTOVER_VIEW {
        view - 1
    } else {
        view
    };
    SilentNode {
        idx: silent_idx,
        at_view: ViewNumber::new(at_view),
    }
}

/// Build a permissive failed-views superset for the loose check.
/// Includes the natural TC2 skip, each silencer's `at_view` (boundary
/// effect when the silent node disconnects mid-view), and every
/// downstream view where the silent node would be leader. `max_view`
/// is a conservative ceiling on the highest view a live node will
/// decide before the loop exits.
fn permitted_failures(
    num_nodes: usize,
    silent_nodes: &[SilentNode],
    max_view: u64,
) -> BTreeSet<ViewNumber> {
    let mut failed = BTreeSet::new();
    failed.insert(ViewNumber::new(PREDICTED_CUTOVER_VIEW - 1));
    for s in silent_nodes {
        let at_view = *s.at_view;
        failed.insert(ViewNumber::new(at_view));
        for v in at_view..=max_view {
            if (v as usize) % num_nodes == s.idx {
                failed.insert(ViewNumber::new(v));
            }
        }
    }
    failed
}

/// Run a single permutation: silence the leader of every view in
/// `views_to_silence`, then assert liveness with a permissive
/// failed-views set (gaps must lie inside the predicted set, but
/// predicted views that actually decided don't trip the assertion).
async fn run_perm_test(views_to_silence: Vec<u64>) {
    let n_silent = views_to_silence.len();
    let num_nodes = perm_num_nodes(n_silent);
    let silent_nodes: Vec<SilentNode> = views_to_silence
        .iter()
        .map(|&v| silent_for_view(v, num_nodes))
        .collect();
    let permitted = permitted_failures(num_nodes, &silent_nodes, 50);
    run_handover_test(
        num_nodes,
        6,
        permitted,
        perm_deadline(n_silent),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        silent_nodes,
        true,
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
