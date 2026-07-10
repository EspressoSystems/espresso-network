use super::*;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pos_upgrade_view_based() {
    test_upgrade_helper(Upgrade::new(FEE_VERSION, EPOCH_VERSION)).await;
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_epoch_reward_upgrade() {
    // Use fewer nodes: epoch mode from view 0 is resource-heavy on CI with
    // postgres Docker containers, causing view timeouts and consensus stall.
    test_upgrade_helper_with_nodes::<3>(
        Upgrade::new(
            versions::DRB_AND_HEADER_UPGRADE_VERSION,
            versions::EPOCH_REWARD_VERSION,
        ),
        100,
    )
    .await;
}

async fn test_upgrade_helper(upgrade: Upgrade) {
    test_upgrade_helper_with_nodes::<5>(upgrade, 200).await;
}

async fn test_upgrade_helper_with_nodes<const NUM_NODES: usize>(
    upgrade: Upgrade,
    start_proposing_view: u64,
) {
    // wait this number of views beyond the configured first view
    // before asserting anything.
    let wait_extra_views = 10;
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let epoch_start_block = if upgrade.base >= versions::EPOCH_VERSION {
        0
    } else {
        321
    };

    let test_config = TestConfigBuilder::default()
        .epoch_height(200)
        .epoch_start_block(epoch_start_block)
        .set_upgrades(upgrade.target)
        .await
        .upgrade_proposing_views(start_proposing_view, 1000)
        .build();

    let chain_config_genesis = ValidatedState::default().chain_config.resolve().unwrap();
    let chain_config_upgrade = test_config.get_upgrade_map().chain_config(upgrade.target);
    assert_ne!(chain_config_genesis, chain_config_upgrade);
    tracing::debug!(?chain_config_genesis, ?chain_config_upgrade);

    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let mut builder = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(port),
        ))
        .persistences(persistence)
        .catchups(std::array::from_fn(|_| {
            StatePeers::<SequencerApiVersion>::from_urls(
                vec![format!("http://localhost:{port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .network_config(test_config);

    // When the base version already has epochs, the base chain config must
    // include the stake_table_contract
    if upgrade.base >= versions::EPOCH_VERSION {
        let state = ValidatedState {
            chain_config: chain_config_upgrade.into(),
            ..Default::default()
        };
        builder = builder.states(std::array::from_fn(|_| state.clone()));
    }

    let config = builder.build();

    let mut network = TestNetwork::new(config, upgrade).await;
    let _events = network.server.event_stream();

    let target = upgrade.target;

    // First loop to get an `UpgradeProposal`. Note that the
    // actual upgrade will take several to many subsequent views for
    // voting and finally the actual upgrade.
    // Use the raw HotShot event stream for upgrade testing, since
    // UpgradeProposal events are HotShot-specific and not surfaced
    // through the CoordinatorEvent adapter.
    let mut hotshot_events = network
        .server
        .consensus_handle()
        .legacy_consensus()
        .read()
        .await
        .event_stream();
    let upgrade = loop {
        let event = hotshot_events.next().await.unwrap();
        if let EventType::UpgradeProposal { proposal, .. } = event.event {
            tracing::info!(?proposal, "proposal");
            let upgrade = proposal.data.upgrade_proposal;
            let new_version = upgrade.new_version;
            tracing::info!(?new_version, "upgrade proposal new version");
            assert_eq!(new_version, target);
            break upgrade;
        }
    };

    let wanted_view = upgrade.new_version_first_view + wait_extra_views;
    // Loop until we get the `new_version_first_view`, then test the upgrade.
    loop {
        let event = hotshot_events.next().await.unwrap();
        let view_number = event.view_number;

        tracing::debug!(?view_number, ?upgrade.new_version_first_view, "upgrade_new_view");
        if view_number > wanted_view {
            tracing::info!(?view_number, ?upgrade.new_version_first_view, "passed upgrade view");
            let states = join_all(
                network
                    .peers
                    .iter()
                    .map(|peer| async { peer.consensus_handle().decided_state().await.unwrap() }),
            )
            .await;
            let leaves = join_all(
                network
                    .peers
                    .iter()
                    .map(|peer| async { peer.consensus_handle().decided_leaf().await }),
            )
            .await;
            let configs: Vec<ChainConfig> = states
                .iter()
                .map(|state| state.chain_config.resolve().unwrap())
                .collect();

            tracing::info!(?leaves, ?configs, "post upgrade state");
            for config in configs {
                assert_eq!(config, chain_config_upgrade);
            }
            for leaf in leaves {
                assert_eq!(leaf.block_header().version(), target);
            }
            break;
        }
        sleep(Duration::from_millis(200)).await;
    }

    network.server.shut_down().await;
}

/// Run a `TestNetwork` based directly on the new protocol version (V0_6,
/// no upgrade/cutover) and verify it produces blocks from genesis.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_new_protocol_produces_blocks() -> anyhow::Result<()> {
    const EPOCH_HEIGHT: u64 = 100;
    const NUM_NODES: usize = 5;
    const TARGET_BLOCK_HEIGHT: u64 = 100;

    const NEW_PROTOCOL: Upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .build();

    let api_port = reserve_tcp_port().expect("No ports free for query service");

    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(api_port),
        ))
        .network_config(network_config)
        .persistences(persistence)
        .catchups(std::array::from_fn(|_| {
            StatePeers::<SequencerApiVersion>::from_urls(
                vec![format!("http://localhost:{api_port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .pos_hook(
            DelegationConfig::MultipleDelegators,
            StakeTableContractVersion::V3,
            NEW_PROTOCOL,
        )
        .await
        .unwrap()
        .build();

    let _network = TestNetwork::new(config, NEW_PROTOCOL).await;

    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());
    client.connect(Some(Duration::from_secs(30))).await;

    let mut leaves = client
        .socket("availability/stream/leaves/0")
        .subscribe::<LeafQueryData<SeqTypes>>()
        .await
        .expect("subscribe to leaf stream");

    let mut height = 0;
    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.expect("leaf stream yielded an error");
        let header = leaf.header();
        height = header.height();

        if height > 0 {
            assert_eq!(
                header.version(),
                NEW_PROTOCOL_VERSION,
                "block {height} should be produced under the new protocol version",
            );
        }

        if height >= TARGET_BLOCK_HEIGHT {
            break;
        }
    }

    assert!(
        height >= TARGET_BLOCK_HEIGHT,
        "expected at least {TARGET_BLOCK_HEIGHT} blocks, got {height} (leaf stream ended early)",
    );

    Ok(())
}
