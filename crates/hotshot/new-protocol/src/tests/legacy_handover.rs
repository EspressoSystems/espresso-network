//! End-to-end handover test mirroring `SequencerContext::init`: per
//! node, a legacy `SystemContext` (MemoryNetwork) and a new-protocol
//! `Coordinator` (Cliquenet) run concurrently. A per-node watcher polls
//! [`harvest::try_perform_handover`] — the same trigger
//! `ConsensusHandle::new_protocol` uses — so the seed flows through
//! `ClientApi::seed_pre_cutover` and into the coordinator's
//! `SeedPreCutover` handler, the only seeding path production uses.

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
    storage_metrics::StorageMetricsValue,
    traits::{
        election::Membership, leaf_fetcher_network::ConnectedNetworkLeafFetcher,
        node_implementation::NodeType, signature_key::SignatureKey,
    },
    x25519::Keypair,
};
use tokio::{
    sync::mpsc::{self, UnboundedSender},
    task::{AbortHandle, JoinHandle},
    time::sleep,
};
use url::Url;
use versions::{CLIQUENET_VERSION, Upgrade, version};

use crate::{
    client::ClientApi,
    consensus::ConsensusOutput,
    coordinator::{Coordinator, CoordinatorOutput, error::Severity, timer::Timer},
    harvest::try_perform_handover,
    helpers::test_upgrade_lock,
    network::cliquenet::Cliquenet,
    outbox::Outbox,
    tests::common::utils::mock_membership_with_client,
};

const NUM_NODES: usize = 4;
const UPGRADE_VIEW: u64 = 5;
const EPOCH_HEIGHT: u64 = 1000;
/// Headroom so the view-0 timer doesn't fire during the legacy phase.
const NEW_PROTO_VIEW_TIMEOUT: Duration = Duration::from_secs(60);

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
    // Tighten cutover offsets; defaults push it ~20 views out.
    metadata.test_config.upgrade_propose_offset = Some(1);
    metadata.test_config.upgrade_decide_by_offset = Some(10);
    metadata.test_config.upgrade_begin_offset = Some(12);
    metadata.test_config.upgrade_finish_offset = Some(15);

    // SimpleBuilder HTTP server — without it the upgrade task never
    // sees proposals. Leaked so it outlives the test run.
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
        let mut membership = <TestTypes as NodeType>::Membership::new(
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
        let memberships = Arc::new(RwLock::new(membership));
        let coordinator =
            EpochMembershipCoordinator::new(memberships, hotshot_config.epoch_height, &storage);

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

/// Per-node `(x25519 keypair, BLS public key, Cliquenet addr)`,
/// deterministic from the same seed the legacy cluster uses so both
/// stacks share BLS identities (one validator key, two transports).
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

/// Production-shaped `Coordinator` build (mirrors `Coordinator::maker`):
/// no `seed_genesis`, no `coord.start()`, no inline pre-cutover seeding.
/// Boots at view 0 and waits for the seed via `ClientApi`. The pre-built
/// `CoordinatorClient` is shared with the membership's leaf fetcher.
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
        proposal::ProposalValidator,
        state::StateManager,
        vid::{VidDisperser, VidReconstructor},
        vote::VoteCollector,
    };

    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    let state_key_pair = StateKeyPair::generate_from_seed_indexed([0u8; 32], node_index);
    let state_private_key = state_key_pair.sign_key_ref().clone();
    let instance = Arc::new(TestInstanceState::default());
    let upgrade_lock = test_upgrade_lock();

    // Throwaway view-0 anchor; the seed advances past it later.
    let genesis_state = TestValidatedState::default();
    let genesis_leaf =
        Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;

    let consensus = Consensus::new(
        membership.clone(),
        public_key,
        private_key.clone(),
        state_private_key,
        10,
        upgrade_lock.clone(),
        genesis_leaf,
        epoch_height,
    );

    let state_manager = StateManager::new(instance.clone(), upgrade_lock.clone());

    let block_builder = BlockBuilder::new(
        instance.clone(),
        membership.clone(),
        BlockBuilderConfig::default(),
        upgrade_lock.clone(),
    );

    let proposal_validator =
        ProposalValidator::new(membership.clone(), epoch_height, upgrade_lock.clone());

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

/// Mirror of `consensus_handle::run_coordinator`: drive the coordinator
/// loop, forward `ExternalMessageReceived` to the leaf-fetcher channel,
/// and report decided views to `decision_tx`.
async fn run_handover_node(
    mut coord: Coordinator<
        TestTypes,
        Cliquenet<TestTypes>,
        hotshot_example_types::storage_types::TestStorage<TestTypes>,
    >,
    decision_tx: UnboundedSender<DecisionEvent>,
    external_events_tx: async_broadcast::Sender<hotshot_types::event::Event<TestTypes>>,
) {
    use hotshot_types::event::{Event, EventType};

    loop {
        match coord.next_consensus_input().await {
            Ok(input) => coord.apply_consensus(input).await,
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
            if let Err(err) = coord.process_consensus_output(output).await
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

/// Polls [`try_perform_handover`] until the cutover crosses. Production
/// triggers the same call lazily from any `ConsensusHandle` method;
/// the test polls because nothing else exercises the gate.
async fn handover_watcher(
    legacy: Arc<RwLock<SystemContextHandle<TestTypes, MemoryImpl>>>,
    client_api: ClientApi<TestTypes>,
) {
    loop {
        let crossed = {
            let guard = legacy.read().await;
            try_perform_handover(&guard, &client_api).await
        };
        if crossed {
            return;
        }
        sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn legacy_runs_upgrade_then_new_protocol_takes_over() {
    crate::logging::init_test_logging();

    let parties = build_parties(NUM_NODES);
    let new_proto_lock = test_upgrade_lock();

    let legacy_handles = spawn_legacy_cluster(NUM_NODES, UPGRADE_VIEW).await;

    // Both stacks alive concurrently from here — same shape as
    // `SequencerContext::init`.
    let mut node_state: Vec<NodeState> = Vec::with_capacity(NUM_NODES);
    for i in 0..NUM_NODES {
        let network = build_new_protocol_network(i, &parties, &new_proto_lock).await;
        let (membership, storage, client, external_events_tx) =
            mock_membership_with_client(NUM_NODES, EPOCH_HEIGHT, parties[i].1).await;

        let coord = build_handover_coordinator(
            i as u64,
            network,
            membership,
            storage,
            client,
            EPOCH_HEIGHT,
            NEW_PROTO_VIEW_TIMEOUT,
        )
        .await;

        let client_api = coord.client_api().clone();
        let (decision_tx, decision_rx) = mpsc::unbounded_channel::<DecisionEvent>();
        let runner_task = tokio::spawn(run_handover_node(coord, decision_tx, external_events_tx));

        node_state.push(NodeState {
            client_api,
            decision_rx,
            runner_task,
        });
    }

    for h in &legacy_handles {
        h.hotshot.start_consensus().await;
    }

    let legacy_arcs: Vec<Arc<RwLock<SystemContextHandle<TestTypes, MemoryImpl>>>> = legacy_handles
        .into_iter()
        .map(|h| Arc::new(RwLock::new(h)))
        .collect();
    let mut watcher_handles: Vec<AbortHandle> = Vec::with_capacity(NUM_NODES);
    for i in 0..NUM_NODES {
        let legacy = legacy_arcs[i].clone();
        let client_api = node_state[i].client_api.clone();
        watcher_handles.push(tokio::spawn(handover_watcher(legacy, client_api)).abort_handle());
    }

    // Wait until every node has decided this many views via the
    // new-protocol decide-rule (Cert2 path on the seeded chain + new
    // post-cutover decisions).
    let target_post_cutover_decisions: usize = 6;
    let deadline = Instant::now() + Duration::from_secs(180);
    let mut decided_per_node: Vec<BTreeMap<ViewNumber, [u8; 32]>> =
        vec![BTreeMap::new(); NUM_NODES];
    while !decided_per_node
        .iter()
        .all(|m| m.len() >= target_post_cutover_decisions)
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
            panic!("not all nodes reached the post-cutover decision target in time");
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

    // Cross-check commits for every view all nodes decided — catches
    // forks the per-node counter alone misses.
    let common_views: BTreeSet<ViewNumber> = decided_per_node.iter().skip(1).fold(
        decided_per_node[0].keys().copied().collect(),
        |acc: BTreeSet<ViewNumber>, m| {
            acc.intersection(&m.keys().copied().collect())
                .copied()
                .collect()
        },
    );
    assert!(
        common_views.len() >= target_post_cutover_decisions,
        "nodes do not agree on enough decided views: common={} target={}",
        common_views.len(),
        target_post_cutover_decisions
    );
    for view in &common_views {
        let commit = decided_per_node[0][view];
        for (i, m) in decided_per_node.iter().enumerate().skip(1) {
            assert_eq!(
                m[view], commit,
                "node {i} decided a different leaf than node 0 at view {}",
                **view
            );
        }
    }

    for w in watcher_handles {
        w.abort();
    }
    for ns in &node_state {
        ns.runner_task.abort();
    }
    for legacy in legacy_arcs {
        legacy.write().await.shut_down().await;
    }
}

struct NodeState {
    client_api: ClientApi<TestTypes>,
    decision_rx: mpsc::UnboundedReceiver<DecisionEvent>,
    runner_task: JoinHandle<()>,
}
