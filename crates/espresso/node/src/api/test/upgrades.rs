use super::*;

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

    let client: Client<ClientErr, SequencerApiVersion> =
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

/// Run a network on the new protocol from genesis (V0_6) through the
/// LargeBlock (V0_7) upgrade, whose chain config takes effect with the
/// version bump.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_new_protocol_large_block_upgrade() -> anyhow::Result<()> {
    const EPOCH_HEIGHT: u64 = 10;
    const NUM_NODES: usize = 5;
    const UPGRADE_START_PROPOSING_VIEW: u64 = 30;
    const UPGRADE: Upgrade = Upgrade::new(NEW_PROTOCOL_VERSION, LARGE_BLOCK_VERSION);

    let api_port = reserve_tcp_port().expect("No ports free for query service");
    let url: Url = format!("http://localhost:{api_port}").parse().unwrap();

    let test_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .set_upgrades(LARGE_BLOCK_VERSION)
        .await
        .upgrade_proposing_views(UPGRADE_START_PROPOSING_VIEW, 1000)
        .build();

    test_config
        .anvil()
        .expect("TestConfigBuilder starts an anvil")
        .anvil_set_interval_mining(1)
        .await
        .expect("interval mining");

    // The genesis chain config differs from the upgrade's only in
    // `max_block_size`.
    let upgrade_chain_config = test_config
        .get_upgrade_map()
        .chain_config(LARGE_BLOCK_VERSION);
    let genesis_chain_config = ChainConfig {
        max_block_size: ChainConfig::default().max_block_size,
        ..upgrade_chain_config
    };
    assert_ne!(
        genesis_chain_config.max_block_size,
        upgrade_chain_config.max_block_size
    );
    let genesis_state = ValidatedState {
        chain_config: genesis_chain_config.into(),
        ..Default::default()
    };

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
        .persistences(persistence)
        .states(std::array::from_fn(|_| genesis_state.clone()))
        .catchups(std::array::from_fn(|_| {
            StatePeers::<SequencerApiVersion>::from_urls(
                vec![url.clone()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .network_config(test_config)
        .build();

    let network = TestNetwork::new(config, UPGRADE).await;
    let client: Client<ClientErr, StaticVersion<0, 1>> = Client::new(url);
    client.connect(None).await;

    let mut leaves = client
        .socket("availability/stream/leaves/0")
        .subscribe::<LeafQueryData<SeqTypes>>()
        .await
        .unwrap();

    // Activation is at the upgrade proposal's view plus
    // `hotshot_new_protocol::upgrade::FINISH_OFFSET` (20).
    let upgrade_height = timeout(Duration::from_secs(600), async {
        loop {
            let leaf = leaves.next().await.unwrap().unwrap();
            if leaf.header().version() >= LARGE_BLOCK_VERSION {
                assert_eq!(
                    leaf.header().chain_config().commit(),
                    upgrade_chain_config.commit(),
                    "upgraded header must commit to the upgraded chain config"
                );
                break leaf.height();
            }
            assert_eq!(
                leaf.header().chain_config().commit(),
                genesis_chain_config.commit(),
                "pre-upgrade header must commit to the genesis chain config"
            );
            tracing::info!(
                version = %leaf.header().version(),
                height = leaf.header().height(),
                view = ?leaf.leaf().view_number(),
                "waiting for the large-block upgrade"
            );
        }
    })
    .await
    .expect("the network did not upgrade to the large-block version");
    tracing::info!(upgrade_height, "large-block upgrade complete");

    timeout(Duration::from_secs(120), async {
        loop {
            let leaf = leaves.next().await.unwrap().unwrap();
            assert!(leaf.header().version() >= LARGE_BLOCK_VERSION);
            if leaf.height() > upgrade_height + 3 {
                break;
            }
        }
    })
    .await
    .expect("the network stalled after the upgrade");

    for node in &network.peers {
        let state = node
            .decided_state()
            .await
            .expect("node has a decided state");
        assert_eq!(
            state
                .chain_config
                .resolve()
                .expect("decided state resolves its chain config"),
            upgrade_chain_config,
        );
    }

    Ok(())
}

/// Run entirely without the legacy consensus stack: with base version
/// `NEW_PROTOCOL_VERSION` it is torn down at startup, and the explicit
/// mid-run `shut_down_legacy` calls below must be harmless to repeat.
/// The network has to keep deciding across epoch boundaries: DRB
/// computations on the shared membership coordinator must survive the
/// teardown.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_new_protocol_survives_legacy_shutdown() -> anyhow::Result<()> {
    const EPOCH_HEIGHT: u64 = 20;
    const NUM_NODES: usize = 5;
    const SHUTDOWN_HEIGHT: u64 = 10;
    const TARGET_BLOCK_HEIGHT: u64 = 50;

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

    let network = TestNetwork::new(config, NEW_PROTOCOL).await;

    let client: Client<ClientErr, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());
    client.connect(Some(Duration::from_secs(30))).await;

    let mut leaves = client
        .socket("availability/stream/leaves/0")
        .subscribe::<LeafQueryData<SeqTypes>>()
        .await
        .expect("subscribe to leaf stream");

    // Let the new protocol decide a few blocks first.
    let mut height = 0;
    while height < SHUTDOWN_HEIGHT {
        let leaf = leaves
            .next()
            .await
            .expect("leaf stream ended early")
            .expect("leaf stream yielded an error");
        height = leaf.header().height();
    }

    // Tear down the legacy stack on every node.
    network.server.consensus_handle().shut_down_legacy().await;
    for peer in &network.peers {
        peer.consensus_handle().shut_down_legacy().await;
    }

    // The chain must keep growing across the epoch boundaries at 20 and
    // 40 purely on the new protocol.
    while height < TARGET_BLOCK_HEIGHT {
        let leaf = leaves
            .next()
            .await
            .expect("leaf stream ended early")
            .expect("leaf stream yielded an error");
        height = leaf.header().height();
    }

    Ok(())
}
