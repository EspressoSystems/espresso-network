//! Production-flow integration test: spin up a real legacy
//! `SystemContext` cluster on a shared `MemoryNetwork`, let the upgrade
//! task organically form an `UpgradeCertificate`, harvest the legacy
//! state at the cutover boundary, and verify the new protocol takes
//! over and continues to decide.
//!
//! This is the same flow `crates/espresso/node/src/consensus_handle.rs`
//! drives in production: the legacy task forms the cert, the cert
//! decides, and on the first activation the harness reads
//! `decided_leaf` / `high_qc` / `validated_state_map` out of the legacy
//! `SystemContextHandle` and seeds the new-protocol `Coordinator`.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use async_lock::RwLock;
use committable::Committable;
use hotshot::{HotShotInitializer, SystemContext, types::SystemContextHandle};
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
    ValidatorConfig,
    consensus::ConsensusMetricsValue,
    data::{Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    simple_certificate::QuorumCertificate2,
    storage_metrics::StorageMetricsValue,
    traits::{
        election::Membership, leaf_fetcher_network::ConnectedNetworkLeafFetcher,
        node_implementation::NodeType,
    },
    vote::HasViewNumber,
};
use tokio::time::sleep;
use url::Url;
use versions::{CLIQUENET_VERSION, Upgrade, version};

use crate::tests::common::runner::{PreCutoverSeed, TestRunner};

/// Spin up `num_nodes` legacy `SystemContext` nodes on a shared
/// `MemoryNetwork` with `Upgrade::new(0.7, 0.8)` configured to fire at
/// `upgrade_view`. Returns the live handles so the caller can drive
/// consensus and read state.
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
    // Use a large `epoch_height` so the anchor view (~upgrade_view +
    // finish_offset = 20) is well inside epoch 1 — keeps the new
    // protocol away from an epoch transition zone where it'd need
    // additional plumbing (state cert, DRB) that the legacy chain
    // doesn't carry.
    metadata.test_config.epoch_height = 1000;
    metadata.test_config.set_view_upgrade(upgrade_view);
    // Tighten the upgrade offsets so the cutover boundary is close to
    // `upgrade_view` and the legacy cluster reaches `cur_view >=
    // cutover_view` quickly. Default test constants use
    // `finish_offset = 20`, which means the new version doesn't take
    // effect until 20 views past `upgrade_view` — slow for a test.
    //
    // Constraints:
    //   1. propose < decide_by < begin < finish.
    //   2. The quorum-proposal task only attaches the cert if
    //      `decide_by >= latest_proposed_view + 3` at the moment the
    //      cert lands. With small `decide_by_offset` the cert can
    //      arrive too late and never make it onto a leaf, so we leave
    //      it generous (10) while still beating the default of 105.
    metadata.test_config.upgrade_propose_offset = Some(1);
    metadata.test_config.upgrade_decide_by_offset = Some(10);
    metadata.test_config.upgrade_begin_offset = Some(12);
    metadata.test_config.upgrade_finish_offset = Some(15);

    // Stand up a real `SimpleBuilder` HTTP server so the leaders can
    // pull blocks from it. Without this the upgrade task never sees
    // proposals and the cert never forms.
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
    // Leak the builder task so its HTTP server stays alive for the
    // duration of the test (otherwise dropping it would tear down the
    // server and the leaders would 502).
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

        // Build the memberships handle and install the leaf-fetcher
        // wiring BEFORE `SystemContext::new` boots tasks. Without this
        // the first epoch-related call panics with `get_epoch_root
        // called before set_leaf_fetcher_network`.
        let is_da = node_id < hotshot_config.da_staked_committee_size as u64;
        let validator_config: ValidatorConfig<TestTypes> =
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                node_id,
                launcher.metadata.node_stakes.get(node_id),
                is_da,
            );
        let public_key = validator_config.public_key.clone();
        let mut membership = <TestTypes as NodeType>::Membership::new(
            hotshot_config.known_nodes_with_stake.clone(),
            hotshot_config.known_da_nodes.clone(),
            public_key.clone(),
            launcher.metadata.test_config.epoch_height,
        );
        // External-events channel feeds the membership's leaf fetcher
        // with `ExternalMessageReceived` events from the network task.
        let external_chan = async_broadcast::broadcast(64);
        membership.set_leaf_fetcher(
            Arc::new(ConnectedNetworkLeafFetcher::<TestTypes, _>::new(
                Arc::clone(&network),
            )),
            storage.clone(),
            public_key.clone(),
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

/// Walk the legacy `Consensus` state and produce the same seed
/// `consensus_handle::harvest_legacy_pre_cutover_seed` produces in
/// production: decided anchor, undecided chain, high QC, and the
/// validated state for every seeded view (anchor + undecided).
async fn harvest_seed(
    handle: &SystemContextHandle<TestTypes, MemoryImpl>,
) -> Option<(
    Leaf2<TestTypes>,
    Vec<Leaf2<TestTypes>>,
    QuorumCertificate2<TestTypes>,
    BTreeMap<ViewNumber, Arc<<TestTypes as NodeType>::ValidatedState>>,
)> {
    let consensus_arc = handle.hotshot.consensus();
    let consensus = consensus_arc.read().await;
    let decided_anchor = consensus.decided_leaf();
    let decided_view = decided_anchor.view_number();
    let decided_commit = decided_anchor.commit();

    let high_qc = consensus.high_qc().clone();
    let saved = consensus.saved_leaves();

    let mut chain: Vec<Leaf2<TestTypes>> = Vec::new();
    let mut next_commit = high_qc.data.leaf_commit;
    loop {
        if next_commit == decided_commit {
            break;
        }
        let leaf = saved.get(&next_commit)?;
        if leaf.view_number() <= decided_view {
            return None;
        }
        chain.push(leaf.clone());
        next_commit = leaf.justify_qc().data.leaf_commit;
    }
    chain.reverse();

    let mut validated_states = BTreeMap::new();
    if let Some(state) = consensus.state(decided_view) {
        validated_states.insert(decided_view, state.clone());
    }
    for leaf in &chain {
        if let Some(state) = consensus.state(leaf.view_number()) {
            validated_states.insert(leaf.view_number(), state.clone());
        }
    }

    Some((decided_anchor, chain, high_qc, validated_states))
}

/// End-to-end production-flow test: a real legacy hotshot cluster
/// forms an `UpgradeCertificate` via its upgrade task, then a real
/// new-protocol cluster (on Cliquenet) takes over from the harvested
/// legacy state and continues to decide.
#[tokio::test(flavor = "multi_thread")]
async fn legacy_runs_upgrade_then_new_protocol_takes_over() {
    crate::logging::init_test_logging();

    let num_nodes = 4;
    let upgrade_view = 5u64;

    // 1. Build the legacy cluster.
    let handles = spawn_legacy_cluster(num_nodes, upgrade_view).await;

    // 2. Start consensus on every node — same call `TestRunner.run_test`
    //    issues after `add_nodes`.
    for h in &handles {
        h.hotshot.start_consensus().await;
    }

    // 3. Wait for an upgrade certificate to be decided. The legacy
    //    upgrade task proposes at view `upgrade_view`, votes form a
    //    `UpgradeCertificate`, the cert is attached to a subsequent
    //    `QuorumProposal`, and decides when that leaf decides.
    let timeout_at = std::time::Instant::now() + Duration::from_secs(120);
    let decided_cert = loop {
        if std::time::Instant::now() > timeout_at {
            panic!("legacy did not decide an upgrade certificate within timeout");
        }
        if let Some(cert) = handles[0].hotshot.upgrade_lock.decided_upgrade_cert() {
            break cert;
        }
        sleep(Duration::from_millis(200)).await;
    };
    let cutover_view = decided_cert.data.new_version_first_view;
    tracing::info!(%cutover_view, "legacy decided upgrade certificate");

    // 4. Wait for every live node to advance past the cutover view —
    //    the production handover gate fires only once `cur_view >=
    //    cutover_view` AND the upgrade certificate has been decided
    //    (see `consensus_handle::CutoverStatus::Active`).
    loop {
        if std::time::Instant::now() > timeout_at {
            panic!("not all nodes advanced past cutover_view within timeout");
        }
        let mut all_advanced = true;
        for h in &handles {
            if h.cur_view().await < cutover_view {
                all_advanced = false;
                break;
            }
        }
        if all_advanced {
            break;
        }
        sleep(Duration::from_millis(200)).await;
    }
    tracing::info!(%cutover_view, "all legacy nodes advanced past cutover");

    // 5. Harvest the seed from every node — this is the *exact* logic
    //    `harvest_legacy_pre_cutover_seed` runs in production.
    //    Every node must produce the same anchor + undecided chain
    //    (same legacy chain) for the new protocol to decide on a
    //    consistent prefix.
    let mut node0_seed = None;
    for h in &handles {
        let seed = harvest_seed(h)
            .await
            .expect("legacy state must be harvestable at the cutover");
        if node0_seed.is_none() {
            node0_seed = Some(seed);
        }
    }
    let (decided_anchor, undecided, high_qc, _validated_states) = node0_seed.unwrap();
    tracing::info!(
        anchor_view = *decided_anchor.view_number(),
        undecided_len = undecided.len(),
        high_qc_view = *high_qc.view_number(),
        "harvested seed",
    );

    // 6. Tear down the legacy cluster — production stops scheduling
    //    legacy work once `new_protocol_active` flips.
    let mut handles = handles;
    for h in &mut handles {
        h.shut_down().await;
    }

    // 7. Spin up the new-protocol cluster on a real Cliquenet network
    //    seeded with the harvested legacy state. The new-protocol
    //    coordinator's `start()` reads the seeded `current_view` and
    //    begins proposing at `cutover_view`.
    let seed = PreCutoverSeed {
        decided_anchor,
        undecided,
        high_qc,
    };

    // `target_decisions` must cover the seeded views that the runner
    // pre-populates (every view in `1..=anchor_view` plus each undecided
    // leaf) plus enough post-cutover views to actually exercise the new
    // protocol after handover.
    let post_cutover_decisions = 6u64;
    let undecided_count = seed.undecided.len() as u64;
    let target_decisions =
        (*seed.decided_anchor.view_number() + undecided_count + post_cutover_decisions) as usize;

    TestRunner::builder()
        .num_nodes(num_nodes)
        .pre_cutover_seed(seed)
        .target_decisions(target_decisions)
        .max_runtime(Duration::from_secs(60))
        .build()
        .run()
        .await
        .expect("new protocol should decide past the cutover boundary");
}
