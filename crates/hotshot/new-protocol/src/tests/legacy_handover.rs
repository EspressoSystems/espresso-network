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
    client::ClientApi,
    consensus::ConsensusOutput,
    coordinator::{Coordinator, CoordinatorOutput, error::Severity, timer::Timer},
    harvest::{forward_legacy_timeout_votes, try_perform_handover},
    helpers::test_upgrade_lock,
    network::cliquenet::Cliquenet,
    outbox::Outbox,
    tests::common::{utils::mock_membership_with_client, views},
};

const UPGRADE_VIEW: u64 = 5;
const EPOCH_HEIGHT: u64 = 1000;
/// Default new-protocol view timeout. Matches the legacy
/// `next_view_timeout` (6s, from `default_multiple_rounds`) so silent-leader
/// post-cutover views advance promptly. The view-0 timer firing during the
/// legacy phase is harmless: the resulting TC2 advances `current_view` but
/// `handle_timeout_certificate` aborts when `locked_cert` is unset, so no
/// proposal/vote work happens until the seed lands.
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
        proposal::{ProposalValidator, VidShareValidator},
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

/// Build a new-protocol coordinator + spawn its runner + watcher +
/// timeout-vote forwarder for one node — exactly the bundle
/// `ConsensusHandle::new` spawns in production.
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

    bg_handles.push(tokio::spawn(handover_watcher(legacy, client_api.clone())).abort_handle());

    let (decision_tx, decision_rx) = mpsc::unbounded_channel::<DecisionEvent>();
    let runner_abort =
        tokio::spawn(run_handover_node(coord, decision_tx, external_events_tx)).abort_handle();

    NodeState {
        decision_rx,
        runner_abort,
    }
}

struct NodeState {
    decision_rx: mpsc::UnboundedReceiver<DecisionEvent>,
    runner_abort: AbortHandle,
}

/// A node held silent for the test, with its shutdown timed by view.
struct SilentNode {
    /// Index of the legacy node to shut down.
    idx: usize,
    /// Wait until any non-silent legacy node's `cur_view` reaches this
    /// view, then shut down the silent node. Setting `idx=3` and
    /// `at_view=18` (with num_nodes=4, cutover=20) makes view 19
    /// (leader=node 3) time out cluster-wide.
    at_view: ViewNumber,
}

/// Run a handover scenario: spin up legacy + new-protocol clusters
/// per node, optionally silence nodes at either layer on per-view
/// triggers, then verify every non-silent node decides at least
/// `target_decisions` views and that no view fails unless it is in
/// `expected_failed_views`.
///
/// `silent_nodes[i]` takes node `silent_nodes[i].idx` fully offline —
/// shutting down its legacy `SystemContext` AND aborting its
/// new-protocol `Coordinator` runner — once any other node's legacy
/// `cur_view` reaches `silent_nodes[i].at_view`. Models a node that
/// has either crashed or been disconnected at the trigger view, just
/// like in production: a node is either online or offline.
///
/// `num_nodes` must satisfy supermajority thresholds with the silent
/// nodes excluded — i.e.
/// `num_nodes - silent_nodes.len() >= (2*num_nodes/3) + 1`.
///
/// `expected_failed_views` is the set of views that are *allowed* to be
/// missing from each alive node's decided chain (silent-leader views
/// that legitimately time out, or legacy views the new protocol skipped
/// via forwarded TC2s). The verifier walks each alive node's decided
/// chain from `min(decided)` to `max(decided)` and asserts that every
/// view in that range is either decided or in `expected_failed_views`.
/// An unexpected gap means the new protocol skipped or timed out a view
/// we predicted would commit — a real consensus deviation worth
/// surfacing. An expected-failed view that *does* show up in the chain
/// means we mispredicted the failure and is also surfaced.
async fn run_handover_test(
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

    // Both stacks alive concurrently from here — same shape as
    // `SequencerContext::init`.
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

    // Walk every alive node's decided chain from `min` to `max` and assert
    // that every intermediate view is either committed or explicitly
    // listed in `expected_failed_views`. An unlisted gap means a view we
    // predicted would commit was actually skipped (e.g. timed out from
    // a race) — a consensus deviation worth surfacing. A view in
    // `expected_failed_views` that *does* commit means we mispredicted
    // the failure and is also surfaced.
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

    // Cross-check commits for every shared decided view among live nodes
    // — catches forks the per-node walk alone misses.
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

/// Poll non-silent legacy `cur_view`s until one reaches `target_view`.
/// Returns when crossed; panics if `timeout` elapses first.
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

/// Watcher that takes `silent.idx` fully offline — shuts down its
/// legacy `SystemContext` AND aborts its new-protocol coordinator —
/// once any non-silent node's legacy `cur_view` reaches `silent.at_view`.
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

/// Predicted cutover view = `upgrade_view + upgrade_finish_offset`. Used
/// by timeout scenarios that need to know which node leads which view
/// before the cluster actually decides the upgrade cert.
const PREDICTED_CUTOVER_VIEW: u64 = UPGRADE_VIEW + 15;

/// End-to-end happy-path handover: legacy + new-protocol clusters run
/// concurrently, the upgrade cert decides naturally, and the new
/// protocol takes over via the seed-bootstrap path.
///
/// No silent nodes. The new protocol starts proposing at `cutover_view`
/// and never proposes or decides anything below it; pre-cutover views
/// belong to legacy. Every post-cutover view should commit.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_runs_upgrade_then_new_protocol_takes_over() {
    run_handover_test(
        4,
        6,
        BTreeSet::new(),
        Duration::from_secs(180),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        Vec::new(),
    )
    .await;
}

/// Timeout-bridge handover: the leader of the last legacy view
/// (`cutover_view - 1`) is shut down one view *before* it would propose,
/// so that view times out cluster-wide. Active legacy nodes emit
/// `TimeoutVote2`s; the bridge forwards them; the new-protocol
/// coordinator rebroadcasts on cliquenet; `TimeoutCertificate2` forms;
/// the first 0.8 leader uses the TC as view-change evidence; and the
/// network decides past the cutover with one validator down.
///
/// Silent leader 3 leads new-proto view 23 (rotates back every 4
/// views), so view 23 times out. Pre-cutover views (≤19) are owned by
/// legacy and never appear in the new protocol's decided chain.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_last_view_times_out_then_new_protocol_takes_over() {
    const NUM_NODES: usize = 4;
    let silent_idx = ((PREDICTED_CUTOVER_VIEW - 1) as usize) % NUM_NODES;
    run_handover_test(
        NUM_NODES,
        6,
        views([23]),
        Duration::from_secs(180),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        vec![SilentNode {
            idx: silent_idx,
            at_view: ViewNumber::new(PREDICTED_CUTOVER_VIEW - 2),
        }],
    )
    .await;
}

/// View-sync handover: the leaders of the **two** views right before
/// the cutover (`cutover_view - 2` and `cutover_view - 1`) are both
/// silent, so two consecutive legacy views time out at the boundary.
/// Bumped to 7 nodes — the BFT supermajority threshold for n=7 is 5,
/// so silencing 2 leaves exactly quorum on the live nodes. The bridge
/// forwards two batches of `TimeoutVote2`s; the new-protocol forms TC2s
/// for both views; `handle_timeout_certificate` advances through the
/// sequence; and the first 0.8 leader proposes against `locked_cert`
/// and the latest TC.
///
/// Pre-cutover views are owned by legacy; the new protocol starts at
/// `cutover_view` (= 20). Silent leaders 4 and 5 lead post-cutover
/// views 25 and 26 (their first rotation after the cutover) — both
/// time out, and the test reaches its 6-decision target on view 27.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_two_views_view_sync_then_new_protocol_takes_over() {
    const NUM_NODES: usize = 7;
    let silent_n_minus_2 = ((PREDICTED_CUTOVER_VIEW - 2) as usize) % NUM_NODES;
    let silent_n_minus_1 = ((PREDICTED_CUTOVER_VIEW - 1) as usize) % NUM_NODES;
    let trigger = ViewNumber::new(PREDICTED_CUTOVER_VIEW - 3);
    run_handover_test(
        NUM_NODES,
        6,
        views([25, 26]),
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

/// New-protocol first-leader timeout: the leader of `cutover_view` (=
/// the first post-cutover view) goes offline right at `cutover_view`.
/// Trigger is set to `cutover_view` so that view (cutover_view - 1)
/// has already QC'd in the legacy (its votes go to leader-of-cutover_view
/// = the silent node, who must be alive long enough to aggregate them).
/// After silence: legacy view `cutover_view` times out (TC routed to
/// leader of cutover_view+1, alive); legacy advances; alive watchers
/// seed the new-protocol cluster; new-protocol view `cutover_view`
/// also times out (silent leader); new-proto-native TC2 forms on
/// cliquenet (no bridge involved); leader of `cutover_view + 1`
/// proposes; the network decides.
///
/// Bumped to 7 nodes so the silent leader (= leader of view 20) only
/// rotates back every 7 views, keeping the number of new-protocol
/// timeouts bounded within the deadline.
///
/// Silent leader 6 leads view 20 only within the test horizon (next
/// rotation is view 27, past the 6-decision exit), so view 20 is the
/// only expected gap.
#[tokio::test(flavor = "multi_thread")]
async fn new_protocol_first_leader_offline_then_recovers() {
    const NUM_NODES: usize = 7;
    let silent_idx = (PREDICTED_CUTOVER_VIEW as usize) % NUM_NODES;
    run_handover_test(
        NUM_NODES,
        6,
        views([20]),
        Duration::from_secs(240),
        DEFAULT_NEW_PROTO_VIEW_TIMEOUT,
        vec![SilentNode {
            idx: silent_idx,
            at_view: ViewNumber::new(PREDICTED_CUTOVER_VIEW),
        }],
    )
    .await;
}
