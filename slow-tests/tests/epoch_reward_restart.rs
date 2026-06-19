use std::time::Duration;

use alloy::primitives::U256;
use espresso_node::{
    SequencerApiVersion,
    api::{
        Options,
        data_source::testing::TestableSequencerDataSource,
        sql::DataSource as SqlDataSource,
        test_helpers::{TestNetwork, TestNetworkConfigBuilder},
    },
    catchup::StatePeers,
    testing::TestConfigBuilder,
};
use espresso_types::ValidatedState;
use futures::{StreamExt, future::join_all};
use hotshot_types::{event::EventType, traits::metrics::NoMetrics, utils::epoch_from_block_number};
use jf_merkle_tree_compat::MerkleTreeScheme;
use test_utils::reserve_tcp_port;
use tokio::time::sleep;
use versions::{DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_REWARD_VERSION, Upgrade};

/// Regression guard: restarting ALL nodes after the reward tree is non-empty causes
/// ValidatedState::from_header to build a sparse tree. At the next epoch boundary the full
/// tree is needed to compute the previous epoch's rewards; no peer can serve it, and the chain
/// stalls.
///
/// The V4 base already distributes rewards every block, so the reward tree is non-empty before
/// the upgrade. The V5 upgrade is configured to take effect in epoch 4 (epoch boundaries at
/// 200, 400, 600, 800 with EPOCH_HEIGHT = 200), and the restart happens a few blocks later,
/// mid-epoch-4, before the epoch 4 -> 5 boundary at block 800. Crossing that boundary requires
/// computing the pre-V5 previous epoch's rewards from the full tree.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn slow_test_epoch_reward_restart() {
    const NUM_NODES: usize = 3;
    const EPOCH_HEIGHT: u64 = 200;
    // Propose the upgrade mid-epoch-4 so it takes effect before the epoch 4 -> 5 boundary.
    const UPGRADE_START_VIEW: u64 = 650;
    const UPGRADE_STOP_VIEW: u64 = 1400;

    let upgrade = Upgrade::new(DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_REWARD_VERSION);

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    // V4 base already has epochs, so epoch_start_block = 0.
    let test_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .set_upgrades(upgrade.target)
        .await
        .upgrade_proposing_views(UPGRADE_START_VIEW, UPGRADE_STOP_VIEW)
        .build();

    let chain_config_upgrade = test_config.get_upgrade_map().chain_config(upgrade.target);

    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    // V4 base requires stake_table_contract in chain config from genesis.
    let genesis_state = ValidatedState {
        chain_config: chain_config_upgrade.into(),
        ..Default::default()
    };
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(port),
        ))
        .persistences(persistence.clone())
        .catchups(std::array::from_fn(|_| {
            StatePeers::<SequencerApiVersion>::from_urls(
                vec![format!("http://localhost:{port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .states(std::array::from_fn(|_| genesis_state.clone()))
        .network_config(test_config)
        .build();

    let mut network = TestNetwork::new(config, upgrade).await;

    // Wait for the upgrade proposal and then for the upgrade to take effect.
    let mut hotshot_events = network
        .server
        .consensus_handle()
        .legacy_consensus()
        .read()
        .await
        .event_stream();
    let upgrade_cert = loop {
        let event = hotshot_events.next().await.unwrap();
        if let EventType::UpgradeProposal { proposal, .. } = event.event {
            let cert = proposal.data.upgrade_proposal;
            assert_eq!(cert.new_version, EPOCH_REWARD_VERSION);
            break cert;
        }
    };
    let wanted_view = upgrade_cert.new_version_first_view;
    loop {
        let event = hotshot_events.next().await.unwrap();
        if event.view_number > wanted_view {
            break;
        }
    }
    drop(hotshot_events);

    // Wait for the upgrade to appear on the decided chain, then advance a few more blocks so
    // the restart happens mid-epoch, before the next epoch boundary.
    let upgrade_height = loop {
        sleep(Duration::from_secs(1)).await;
        let leaf = network.server.decided_leaf().await;
        if leaf.block_header().version() >= EPOCH_REWARD_VERSION {
            break leaf.height();
        }
    };
    loop {
        sleep(Duration::from_secs(1)).await;
        if network.server.decided_leaf().await.height() >= upgrade_height + 5 {
            break;
        }
    }

    let decided_state = network
        .server
        .decided_state()
        .await
        .expect("decided state must be available");
    let decided_leaf = network.server.decided_leaf().await;
    let num_leaves_before_restart = decided_state.reward_merkle_tree_v2.num_leaves();
    let rewards_before_restart = decided_leaf
        .block_header()
        .total_reward_distributed()
        .expect("V5+ header must carry total_reward_distributed");
    let restart_height = decided_leaf.height();
    let restart_epoch = epoch_from_block_number(restart_height, EPOCH_HEIGHT);
    let upgrade_epoch = epoch_from_block_number(upgrade_height, EPOCH_HEIGHT);
    tracing::info!(
        upgrade_height,
        upgrade_epoch,
        restart_height,
        restart_epoch,
        num_leaves_before_restart,
        total_reward_distributed = %rewards_before_restart.0,
        "restarting all nodes with non-empty reward tree"
    );
    // Preconditions for the test to exercise the bug:
    // - The reward tree must be non-empty; otherwise from_header builds a fresh empty tree
    //   rather than a sparse one and the bug cannot trigger.
    // - The upgrade must take effect after the first epoch and the restart must stay in the
    //   same epoch as the upgrade, so the upcoming boundary computes the pre-V5 previous
    //   epoch's rewards from the recovered tree.
    assert!(
        num_leaves_before_restart > 0,
        "num_leaves={num_leaves_before_restart}: reward tree must be non-empty at restart"
    );
    assert!(
        rewards_before_restart.0 > U256::ZERO,
        "no rewards distributed before restart (total_reward_distributed=0)"
    );
    assert!(
        upgrade_epoch > 1,
        "upgrade must not take effect in the first epoch (upgrade_epoch={upgrade_epoch})"
    );
    assert_eq!(
        restart_epoch, upgrade_epoch,
        "restart must stay in the upgrade epoch, before the next epoch boundary"
    );

    // Clone the TestConfig before dropping the network so the anvil/L1/contracts stay alive.
    let saved_cfg = network.cfg.clone();

    network.stop_consensus().await;
    drop(network);

    // Rebuild reusing the same persistence so nodes resume from stored state.
    let port2 = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let config2 = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(port2),
        ))
        .persistences(persistence)
        .catchups(std::array::from_fn(|_| {
            StatePeers::<SequencerApiVersion>::from_urls(
                vec![format!("http://localhost:{port2}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .states(std::array::from_fn(|_| genesis_state.clone()))
        .network_config(saved_cfg)
        .build();
    let network2 = TestNetwork::new(config2, upgrade).await;

    // The chain must keep advancing across two epoch boundaries whose reward trees are computed
    // entirely after the restart. If the sparse tree cannot be recovered, consensus wedges and
    // the decided height freezes. Use a lack-of-progress watchdog (not a fixed deadline) so a
    // healthy-but-slow chain still passes while a genuine stall fails.
    let target_height = (restart_epoch + 2) * EPOCH_HEIGHT;
    let stall_limit = 30; // 30 polls * 2s = 60s without a new decide => stalled
    let mut last_height = network2.server.decided_leaf().await.height();
    let mut stalled_polls = 0;
    while last_height < target_height {
        sleep(Duration::from_secs(2)).await;
        let height = network2.server.decided_leaf().await.height();
        if height > last_height {
            last_height = height;
            stalled_polls = 0;
        } else {
            stalled_polls += 1;
            if stalled_polls >= stall_limit {
                let stalled_epoch = epoch_from_block_number(last_height, EPOCH_HEIGHT);
                let next_boundary = (stalled_epoch + 1) * EPOCH_HEIGHT;
                panic!(
                    "chain stalled after restart: no new decide for 60s at height {last_height} \
                     (epoch {stalled_epoch}), unable to cross next epoch boundary at height \
                     {next_boundary}. The sparse reward tree could not be recovered (target \
                     height was {target_height})."
                );
            }
        }
    }

    // Rewards must keep being distributed across the post-restart boundaries, confirming the
    // reward path is still exercised (not merely that empty blocks advanced).
    let rewards_after_restart = network2
        .server
        .decided_leaf()
        .await
        .block_header()
        .total_reward_distributed()
        .expect("V5+ header must carry total_reward_distributed");
    assert!(
        rewards_after_restart.0 > rewards_before_restart.0,
        "no rewards distributed after restart: before={}, after={}",
        rewards_before_restart.0,
        rewards_after_restart.0
    );

    // Consistency check: all peers agree on the reward_merkle_tree_root at the decided leaf.
    let roots: Vec<_> = join_all(network2.peers.iter().map(|peer| async {
        peer.consensus_handle()
            .decided_state()
            .await
            .unwrap()
            .reward_merkle_tree_v2
            .commitment()
    }))
    .await;
    let server_root = network2
        .server
        .decided_state()
        .await
        .unwrap()
        .reward_merkle_tree_v2
        .commitment();
    for root in &roots {
        assert_eq!(
            root, &server_root,
            "reward_merkle_tree_root mismatch across peers"
        );
    }
}
