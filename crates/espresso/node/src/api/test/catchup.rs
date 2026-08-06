use super::*;

async fn run_catchup_test(url_suffix: &str) {
    // Start a sequencer network, using the query service for catchup.
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    const NUM_NODES: usize = 5;

    let url: url::Url = format!("http://localhost:{port}{url_suffix}")
        .parse()
        .unwrap();

    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(Options::with_port(port))
        .network_config(TestConfigBuilder::default().build())
        .catchups(std::array::from_fn(|_| {
            StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![url.clone()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .build();
    let mut network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;

    // Wait for replica 0 to reach a (non-genesis) decide, before disconnecting it.
    let mut events = network.peers[0].event_stream();
    loop {
        let event = events.next().await.unwrap();
        let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        else {
            continue;
        };
        if leaf_chain[0].leaf.height() > 0 {
            break;
        }
    }

    // Shut down and restart replica 0. We don't just stop consensus and restart it; we fully
    // drop the node and recreate it so it loses all of its temporary state and starts off from
    // genesis. It should be able to catch up by listening to proposals and then rebuild its
    // state from its peers.
    tracing::info!("shutting down node");
    network.peers.remove(0);

    // Wait for a few blocks to pass while the node is down, so it falls behind.
    network
        .server
        .event_stream()
        .filter(|event| {
            future::ready(matches!(
                event,
                CoordinatorEvent::LegacyEvent(Event {
                    event: EventType::Decide { .. },
                    ..
                })
            ))
        })
        .take(3)
        .collect::<Vec<_>>()
        .await;

    tracing::info!("restarting node");
    let node = network
        .cfg
        .init_node(
            1,
            ValidatedState::default(),
            no_storage::Options,
            Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![url],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )),
            None,
            &NoMetrics,
            test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
            NullEventConsumer,
            MOCK_SEQUENCER_VERSIONS,
            Default::default(),
        )
        .await;
    let mut events = node.event_stream();

    // Wait for a (non-genesis) block proposed by each node, to prove that the lagging node has
    // caught up and all nodes are in sync.
    let mut proposers = [false; NUM_NODES];
    loop {
        let event = events.next().await.unwrap();
        let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        else {
            continue;
        };
        for LeafInfo { leaf, .. } in leaf_chain.iter().rev() {
            let height = leaf.height();
            let leaf_builder = (leaf.view_number().u64() as usize) % NUM_NODES;
            if height == 0 {
                continue;
            }

            tracing::info!(
                "waiting for blocks from {proposers:?}, block {height} is from {leaf_builder}",
            );
            proposers[leaf_builder] = true;
        }

        if proposers.iter().all(|has_proposed| *has_proposed) {
            break;
        }
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_catchup() {
    run_catchup_test("").await;
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_catchup_v0() {
    run_catchup_test("/v0").await;
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_catchup_v1() {
    run_catchup_test("/v1").await;
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_catchup_no_state_peers() {
    // Start a sequencer network, using the query service for catchup.
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    const NUM_NODES: usize = 5;
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(Options::with_port(port))
        .network_config(TestConfigBuilder::default().build())
        .build();
    let mut network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;

    // Wait for replica 0 to reach a (non-genesis) decide, before disconnecting it.
    let mut events = network.peers[0].event_stream();
    loop {
        let event = events.next().await.unwrap();
        let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        else {
            continue;
        };
        if leaf_chain[0].leaf.height() > 0 {
            break;
        }
    }

    // Shut down and restart replica 0. We don't just stop consensus and restart it; we fully
    // drop the node and recreate it so it loses all of its temporary state and starts off from
    // genesis. It should be able to catch up by listening to proposals and then rebuild its
    // state from its peers.
    tracing::info!("shutting down node");
    network.peers.remove(0);

    // Wait for a few blocks to pass while the node is down, so it falls behind.
    network
        .server
        .event_stream()
        .filter(|event| {
            future::ready(matches!(
                event,
                CoordinatorEvent::LegacyEvent(Event {
                    event: EventType::Decide { .. },
                    ..
                })
            ))
        })
        .take(3)
        .collect::<Vec<_>>()
        .await;

    tracing::info!("restarting node");
    let node = network
        .cfg
        .init_node(
            1,
            ValidatedState::default(),
            no_storage::Options,
            None::<NullStateCatchup>,
            None,
            &NoMetrics,
            test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
            NullEventConsumer,
            MOCK_SEQUENCER_VERSIONS,
            Default::default(),
        )
        .await;
    let mut events = node.event_stream();

    // Wait for a (non-genesis) block proposed by each node, to prove that the lagging node has
    // caught up and all nodes are in sync.
    let mut proposers = [false; NUM_NODES];
    loop {
        let event = events.next().await.unwrap();
        let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        else {
            continue;
        };
        for LeafInfo { leaf, .. } in leaf_chain.iter().rev() {
            let height = leaf.height();
            let leaf_builder = (leaf.view_number().u64() as usize) % NUM_NODES;
            if height == 0 {
                continue;
            }

            tracing::info!(
                "waiting for blocks from {proposers:?}, block {height} is from {leaf_builder}",
            );
            proposers[leaf_builder] = true;
        }

        if proposers.iter().all(|has_proposed| *has_proposed) {
            break;
        }
    }
}

#[ignore]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_catchup_epochs_no_state_peers() {
    // Start a sequencer network, using the query service for catchup.
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    const EPOCH_HEIGHT: u64 = 5;
    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();
    const NUM_NODES: usize = 5;
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(Options::with_port(port))
        .network_config(network_config)
        .build();
    let mut network = TestNetwork::new(config, Upgrade::trivial(EPOCH_VERSION)).await;

    // Wait for replica 0 to decide in the third epoch.
    let mut events = network.peers[0].event_stream();
    loop {
        let event = events.next().await.unwrap();
        let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        else {
            continue;
        };
        tracing::error!("got decide height {}", leaf_chain[0].leaf.height());

        if leaf_chain[0].leaf.height() > EPOCH_HEIGHT * 3 {
            tracing::error!("decided past one epoch");
            break;
        }
    }

    // Shut down and restart replica 0. We don't just stop consensus and restart it; we fully
    // drop the node and recreate it so it loses all of its temporary state and starts off from
    // genesis. It should be able to catch up by listening to proposals and then rebuild its
    // state from its peers.
    tracing::info!("shutting down node");
    network.peers.remove(0);

    // Wait for a few blocks to pass while the node is down, so it falls behind.
    network
        .server
        .event_stream()
        .filter(|event| {
            future::ready(matches!(
                event,
                CoordinatorEvent::LegacyEvent(Event {
                    event: EventType::Decide { .. },
                    ..
                })
            ))
        })
        .take(3)
        .collect::<Vec<_>>()
        .await;

    tracing::error!("restarting node");
    let node = network
        .cfg
        .init_node(
            1,
            ValidatedState::default(),
            no_storage::Options,
            None::<NullStateCatchup>,
            None,
            &NoMetrics,
            test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
            NullEventConsumer,
            MOCK_SEQUENCER_VERSIONS,
            Default::default(),
        )
        .await;
    let mut events = node.event_stream();

    // Wait for a (non-genesis) block proposed by each node, to prove that the lagging node has
    // caught up and all nodes are in sync.
    let mut proposers = [false; NUM_NODES];
    loop {
        let event = events.next().await.unwrap();
        let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        else {
            continue;
        };
        for LeafInfo { leaf, .. } in leaf_chain.iter().rev() {
            let height = leaf.height();
            let leaf_builder = (leaf.view_number().u64() as usize) % NUM_NODES;
            if height == 0 {
                continue;
            }

            tracing::info!(
                "waiting for blocks from {proposers:?}, block {height} is from {leaf_builder}",
            );
            proposers[leaf_builder] = true;
        }

        if proposers.iter().all(|has_proposed| *has_proposed) {
            break;
        }
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_chain_config_from_instance() {
    // This test uses a ValidatedState which only has the default chain config commitment.
    // The NodeState has the full chain config.
    // Both chain config commitments will match, so the ValidatedState should have the
    // full chain config after a non-genesis block is decided.

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let chain_config: ChainConfig = ChainConfig::default();

    let state = ValidatedState {
        chain_config: chain_config.commit().into(),
        ..Default::default()
    };

    let states = std::array::from_fn(|_| state.clone());

    let config = TestNetworkConfigBuilder::default()
        .api_config(Options::with_port(port))
        .states(states)
        .catchups(std::array::from_fn(|_| {
            StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![format!("http://localhost:{port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .network_config(TestConfigBuilder::default().build())
        .build();

    let mut network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;

    // Wait for few blocks to be decided.
    network
        .server
        .event_stream()
        .filter(|event| {
            future::ready(matches!(
                event,
                CoordinatorEvent::LegacyEvent(Event {
                    event: EventType::Decide { .. },
                    ..
                })
            ))
        })
        .take(3)
        .collect::<Vec<_>>()
        .await;

    for peer in &network.peers {
        let state = peer.consensus_handle().decided_state().await.unwrap();

        assert_eq!(state.chain_config.resolve().unwrap(), chain_config)
    }

    network.server.shut_down().await;
    drop(network);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_chain_config_catchup() {
    // This test uses a ValidatedState with a non-default chain config
    // so it will be different from the NodeState chain config used by the TestNetwork.
    // However, for this test to work, at least one node should have a full chain config
    // to allow other nodes to catch up.

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let cf = ChainConfig {
        max_block_size: 300.into(),
        base_fee: 1.into(),
        ..Default::default()
    };

    // State1 contains only the chain config commitment
    let state1 = ValidatedState {
        chain_config: cf.commit().into(),
        ..Default::default()
    };

    //state 2 contains the full chain config
    let state2 = ValidatedState {
        chain_config: cf.into(),
        ..Default::default()
    };

    let mut states = std::array::from_fn(|_| state1.clone());
    // only one node has the full chain config
    // all the other nodes should do a catchup to get the full chain config from peer 0
    states[0] = state2;

    const NUM_NODES: usize = 5;
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(Options::from(options::Http {
            port,
            max_connections: None,
            tonic_port: None,
        }))
        .states(states)
        .catchups(std::array::from_fn(|_| {
            StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![format!("http://localhost:{port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .network_config(TestConfigBuilder::default().build())
        .build();

    let mut network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;

    // Wait for a few blocks to be decided.
    network
        .server
        .event_stream()
        .filter(|event| {
            future::ready(matches!(
                event,
                CoordinatorEvent::LegacyEvent(Event {
                    event: EventType::Decide { .. },
                    ..
                })
            ))
        })
        .take(3)
        .collect::<Vec<_>>()
        .await;

    for peer in &network.peers {
        let state = peer.consensus_handle().decided_state().await.unwrap();

        assert_eq!(state.chain_config.resolve().unwrap(), cf)
    }

    network.server.shut_down().await;
    drop(network);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub(crate) async fn test_restart() {
    const NUM_NODES: usize = 5;
    // Initialize nodes.
    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let config = TestNetworkConfigBuilder::default()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(port),
        ))
        .persistences(persistence.clone())
        .network_config(TestConfigBuilder::default().build())
        .build();
    let mut network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;

    // Connect client.
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{port}").parse().unwrap());
    client.connect(None).await;
    tracing::info!(port, "server running");

    // Wait until some blocks have been decided.
    client
        .socket("availability/stream/blocks/0")
        .subscribe::<BlockQueryData<SeqTypes>>()
        .await
        .unwrap()
        .take(3)
        .collect::<Vec<_>>()
        .await;

    // Shut down the consensus nodes.
    tracing::info!("shutting down nodes");
    network.stop_consensus().await;

    // Get the block height we reached.
    let height = client
        .get::<usize>("status/block-height")
        .send()
        .await
        .unwrap();
    tracing::info!("decided {height} blocks before shutting down");

    // Get the decided chain, so we can check consistency after the restart.
    let chain: Vec<LeafQueryData<SeqTypes>> = client
        .socket("availability/stream/leaves/0")
        .subscribe()
        .await
        .unwrap()
        .take(height)
        .try_collect()
        .await
        .unwrap();
    let decided_view = chain.last().unwrap().leaf().view_number();

    // Get the most recent state, for catchup.

    let state = network.server.decided_state().await.unwrap();
    tracing::info!(?decided_view, ?state, "consensus state");

    // Fully shut down the API servers.
    drop(network);

    // Start up again, resuming from the last decided leaf.
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let config = TestNetworkConfigBuilder::default()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(port),
        ))
        .persistences(persistence)
        .catchups(std::array::from_fn(|_| {
            // Catchup using node 0 as a peer. Node 0 was running the archival state service
            // before the restart, so it should be able to resume without catching up by loading
            // state from storage.
            StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![format!("http://localhost:{port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .network_config(TestConfigBuilder::default().build())
        .build();
    let _network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    let client: Client<ServerError, StaticVersion<0, 1>> =
        Client::new(format!("http://localhost:{port}").parse().unwrap());
    client.connect(None).await;
    tracing::info!(port, "server running");

    // Make sure we can decide new blocks after the restart.
    tracing::info!("waiting for decide, height {height}");
    let new_leaf: LeafQueryData<SeqTypes> = client
        .socket(&format!("availability/stream/leaves/{height}"))
        .subscribe()
        .await
        .unwrap()
        .next()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(new_leaf.height(), height as u64);
    assert_eq!(
        new_leaf.leaf().parent_commitment(),
        chain[height - 1].hash()
    );

    // Ensure the new chain is consistent with the old chain.
    let new_chain: Vec<LeafQueryData<SeqTypes>> = client
        .socket("availability/stream/leaves/0")
        .subscribe()
        .await
        .unwrap()
        .take(height)
        .try_collect()
        .await
        .unwrap();
    assert_eq!(chain, new_chain);
}

/// A query node whose database is wiped has to rebuild itself from its peer
/// and rejoin consensus.
///
/// Six nodes run the new protocol from genesis. Nodes 0 and 1 both serve the
/// query API and are peered at each other so either can backfill from the
/// other; nodes 2 through 5 are plain validators. Once the network decides
/// past epoch seven, node 1 is taken down and started again on an empty
/// database: no consensus state, no archive, no merklized state. To recover
/// it has to bootstrap its stake-table window, resync consensus, backfill
/// the chain it lost from node 0 (including the new protocol's cert2
/// finality certificates), and resume proposing.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_new_protocol_query_node_restart_with_fresh_storage() -> anyhow::Result<()> {
    const NUM_NODES: usize = 6;
    const EPOCH_HEIGHT: u64 = 10;
    const EPOCHS_BEFORE_RESTART: u64 = 7;
    const NEW_PROTOCOL: Upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);
    /// Bound on each stage of the restarted node's recovery.
    const RECOVERY_TIMEOUT: Duration = Duration::from_secs(240);

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .build();

    let api_port = reserve_tcp_port().expect("No ports free for query service");
    let query_port = reserve_tcp_port().expect("No ports free for query service");
    let api_url: Url = format!("http://localhost:{api_port}").parse()?;
    let query_url: Url = format!("http://localhost:{query_port}").parse()?;

    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    // Both query nodes need the catchup and light client modules: state
    // catchup is served over the former, and the query service validates
    // leaves fetched from a peer with light client proofs served over the
    // latter. Every node can catch up from either query node.
    let query_urls = vec![api_url.clone(), query_url.clone()];
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(
            Options::with_port(api_port)
                .catchup(Default::default())
                .light_client(Default::default())
                .query_sql(
                    Query {
                        peers: vec![query_url.clone()],
                        ..Default::default()
                    },
                    tmp_options(&storage[0]),
                ),
        )
        .network_config(network_config)
        .persistences(persistence.clone())
        .catchups(std::array::from_fn(|_| {
            StatePeers::<SequencerApiVersion>::from_urls(
                query_urls.clone(),
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
        .await?
        .build();

    let genesis_state = config.states()[0].clone();
    let mut network = TestNetwork::new(config, NEW_PROTOCOL).await;

    // `TestNetwork` only gives node 0 an API, so node 1 has to be served
    // separately to become the second query node. Its keys are already in the
    // config, so it rejoins under the same index.
    network.peers[0].shut_down().await;
    network.peers.remove(0);

    let cfg = network.cfg.clone();
    let peer_url = api_url.clone();
    let start_query_node = move |db: persistence::sql::Options| {
        let cfg = cfg.clone();
        let genesis_state = genesis_state.clone();
        let peer_url = peer_url.clone();
        async move {
            let opt = Options::with_port(query_port)
                .catchup(Default::default())
                .light_client(Default::default())
                .query_sql(
                    Query {
                        peers: vec![peer_url.clone()],
                        ..Default::default()
                    },
                    db.clone(),
                );
            let ctx = opt
                .serve(move |metrics, consumer, storage| {
                    async move {
                        Ok(cfg
                            .init_node(
                                1,
                                genesis_state,
                                db,
                                Some(StatePeers::<SequencerApiVersion>::from_urls(
                                    vec![peer_url],
                                    Default::default(),
                                    Duration::from_secs(2),
                                    &NoMetrics,
                                )),
                                storage,
                                &*metrics,
                                test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                                consumer,
                                NEW_PROTOCOL,
                                Default::default(),
                            )
                            .await)
                    }
                    .boxed()
                })
                .await
                .expect("second query node should start");
            ctx.start_consensus().await;
            ctx
        }
    };

    let mut query_node = start_query_node(tmp_options(&storage[1])).await;

    let api_client: Client<ServerError, SequencerApiVersion> = Client::new(api_url);
    let query_client: Client<ServerError, SequencerApiVersion> = Client::new(query_url);
    assert!(
        api_client.connect(Some(Duration::from_secs(60))).await,
        "node 0 query API did not come up"
    );
    assert!(
        query_client.connect(Some(Duration::from_secs(60))).await,
        "node 1 query API did not come up"
    );

    // Run the network for seven epochs, watching a validator that stays up
    // for the whole test.
    let mut events = network.peers[0].event_stream();
    wait_for_epochs(&mut events, EPOCH_HEIGHT, EPOCHS_BEFORE_RESTART).await;

    // The node about to be wiped must be healthy first, otherwise the test
    // would prove nothing about recovering from a wipe.
    let height_before_restart = query_node.decided_leaf().await.height();
    timeout(
        RECOVERY_TIMEOUT,
        wait_until_block_height(
            &query_client,
            "block-state/block-height",
            height_before_restart,
        ),
    )
    .await
    .context("second query node was not caught up before the restart")?;

    // Pick a height the new protocol finalized with a cert2 to check the
    // restarted node backfills those too. A cert2 is only stored at the
    // height it finalizes, so scan back from the tip for one.
    let mut finalized_height = None;
    for height in (1..height_before_restart).rev() {
        if api_client
            .get::<espresso_types::Certificate2<SeqTypes>>(&format!("availability/cert2/{height}"))
            .send()
            .await
            .is_ok()
        {
            finalized_height = Some(height);
            break;
        }
    }
    let finalized_height = finalized_height.context("no cert2 stored on a new protocol chain")?;

    tracing::info!(
        height_before_restart,
        finalized_height,
        "restarting the second query node with fresh storage"
    );
    query_node.shut_down().await;
    // The node has to come back on the same ports: node 0 dials the query
    // API at the peer URL baked in at network construction, and cliquenet
    // peers dial the address registered in the stake table. `shut_down`
    // aborts the server tasks without waiting for them to finish, so poll
    // until the old listeners are actually gone before rebinding.
    let cliquenet_port = network.cfg.known_nodes_with_stake()[1]
        .connect_info
        .as_ref()
        .expect("node 1 registered cliquenet connect info")
        .p2p_addr
        .port();
    timeout(RECOVERY_TIMEOUT, async {
        for port in [query_port, cliquenet_port] {
            while std::net::TcpListener::bind(("127.0.0.1", port)).is_err() {
                sleep(Duration::from_millis(100)).await;
            }
        }
    })
    .await
    .context("shut-down node did not release its ports")?;

    let fresh_storage = SqlDataSource::create_storage().await;
    let query_node = start_query_node(tmp_options(&fresh_storage)).await;
    assert!(
        query_client.connect(Some(Duration::from_secs(60))).await,
        "restarted query API did not come up"
    );

    // First it has to get back to the height it lost, in both the archive and
    // the merklized state derived from it.
    timeout(
        RECOVERY_TIMEOUT,
        wait_until_block_height(&query_client, "status/block-height", height_before_restart),
    )
    .await
    .context("restarted node did not rebuild its archive")?;
    timeout(
        RECOVERY_TIMEOUT,
        wait_until_block_height(
            &query_client,
            "block-state/block-height",
            height_before_restart,
        ),
    )
    .await
    .context("restarted node did not rebuild its merklized state")?;

    // Reaching the tip only proves it is following consensus again, so stream
    // the whole range it lost from both nodes: the restarted node's copy can
    // only have come from node 0, and it has to be the chain the network
    // actually decided. Streaming also forces the restarted node to backfill
    // every leaf in the range, since the stream endpoint fetches on demand.
    let wiped_range = (height_before_restart - 1) as usize;
    let stream_leaves = |client: Client<ServerError, SequencerApiVersion>, who: &'static str| async move {
        let leaves: Vec<LeafQueryData<SeqTypes>> = client
            .socket("availability/stream/leaves/1")
            .subscribe()
            .await
            .with_context(|| format!("subscribing to {who}'s leaf stream"))?
            .take(wiped_range)
            .try_collect()
            .await
            .with_context(|| format!("{who}'s leaf stream errored"))?;
        anyhow::Ok(leaves)
    };
    // Node 0's stream returns immediately, so the shared timeout is in
    // practice a bound on the restarted node's backfill.
    let (ours, theirs) = timeout(
        RECOVERY_TIMEOUT,
        future::try_join(
            stream_leaves(query_client.clone(), "the restarted node"),
            stream_leaves(api_client.clone(), "node 0"),
        ),
    )
    .await
    .context("streaming the wiped range stalled")??;
    assert_eq!(
        ours.len(),
        wiped_range,
        "restarted node's leaf stream ended early"
    );
    assert_eq!(
        theirs.len(),
        wiped_range,
        "node 0's leaf stream ended early"
    );
    for (ours, theirs) in ours.iter().zip(&theirs) {
        assert_eq!(
            ours.hash(),
            theirs.hash(),
            "restarted node's leaf at height {} diverges from node 0",
            ours.height(),
        );
        assert_eq!(
            ours.header().version(),
            NEW_PROTOCOL_VERSION,
            "block {} should have been produced under the new protocol",
            ours.height(),
        );
    }

    let cert2 = timeout(RECOVERY_TIMEOUT, async {
        loop {
            match query_client
                .get::<espresso_types::Certificate2<SeqTypes>>(&format!(
                    "availability/cert2/{finalized_height}"
                ))
                .send()
                .await
            {
                Ok(cert2) => break cert2,
                Err(err) => {
                    tracing::info!(finalized_height, %err, "cert2 not backfilled yet")
                },
            }
            sleep(Duration::from_secs(2)).await;
        }
    })
    .await
    .context("restarted node did not backfill the cert2")?;
    assert_eq!(cert2.data.block_number, finalized_height);

    // Tracking decides is not enough to call the node a participant: wait for
    // a block it proposed itself to be decided after the restart.
    let node_1_key = network.cfg.known_nodes_with_stake()[1]
        .stake_table_entry
        .stake_key;
    let coordinator = query_node.node_state().coordinator;
    let mut events = query_node.event_stream();
    let proposed = timeout(RECOVERY_TIMEOUT, async {
        while let Some(event) = events.next().await {
            let leaf_infos: &[LeafInfo<SeqTypes>] = match &event {
                CoordinatorEvent::LegacyEvent(Event {
                    event: EventType::Decide { leaf_chain, .. },
                    ..
                }) => leaf_chain,
                CoordinatorEvent::NewDecide { leaf_infos, .. } => leaf_infos,
                _ => continue,
            };
            for LeafInfo { leaf, .. } in leaf_infos {
                if leaf.height() <= height_before_restart {
                    continue;
                }
                let membership = match coordinator.membership_for_epoch(leaf.epoch(EPOCH_HEIGHT)) {
                    Ok(membership) => membership,
                    Err(err) => {
                        tracing::warn!(
                            height = leaf.height(),
                            %err,
                            "no membership for epoch",
                        );
                        continue;
                    },
                };
                match membership.leader(leaf.view_number()) {
                    Ok(leader) if leader == node_1_key => return Some(leaf.height()),
                    Ok(_) => {},
                    Err(err) => {
                        tracing::warn!(view = ?leaf.view_number(), %err, "leader unresolved");
                    },
                }
            }
        }
        None
    })
    .await
    .context("restarted node's event stream stalled")?;
    let proposed = proposed.context("restarted node's event stream ended")?;
    tracing::info!(proposed, "restarted node proposed a decided block");

    Ok(())
}

#[rstest]
#[case(POS_V3)]
#[case(POS_V4)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_merklized_state_catchup_on_restart(#[case] upgrade: Upgrade) -> anyhow::Result<()> {
    // This test verifies that a query node can catch up on
    // merklized state after being offline for multiple epochs.
    //
    // Steps:
    // 1. Start a test network with 5 sequencer nodes.
    // 2. Start a separate node with the query module enabled, connected to the network.
    //    - This node stores merklized state
    // 3. Shut down the query node after 1 epoch.
    // 4. Allow the network to progress 3 more epochs (query node remains offline).
    // 5. Restart the query node.
    //    - The node is expected to reconstruct or catch up on its own
    use espresso_types::{DECAF_CHAIN_ID, v0_3::ChainConfig};

    const EPOCH_HEIGHT: u64 = 10;

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    tracing::info!("API PORT = {api_port}");
    const NUM_NODES: usize = 5;

    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    // The light client skips epoch-root stake-table-hash verification for pre-DRB headers only
    // on the Decaf chain id, so use it to keep the V3->V4 catchup path covered. Must be set
    // before `pos_hook`, which preserves the chain id from `state[0]`.
    let decaf_state = ValidatedState {
        chain_config: ChainConfig {
            chain_id: DECAF_CHAIN_ID,
            ..Default::default()
        }
        .into(),
        ..Default::default()
    };

    let config = TestNetworkConfigBuilder::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(api_port)
                .catchup(Default::default())
                .light_client(Default::default()),
        ))
        .network_config(network_config)
        .persistences(persistence.clone())
        .states(std::array::from_fn(|_| decaf_state.clone()))
        .catchups(std::array::from_fn(|_| {
            StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![format!("http://localhost:{api_port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .pos_hook(
            DelegationConfig::MultipleDelegators,
            hotshot_contract_adapter::stake_table::StakeTableContractVersion::V3,
            upgrade,
        )
        .await
        .unwrap()
        .build();
    let state = config.states()[0].clone();
    let mut network = TestNetwork::new(config, upgrade).await;

    // Remove peer 0 and restart it with the query module enabled.
    // Adding an additional node to the test network is not straight forward,
    // as the keys have already been initialized in the config above.
    // So, we remove this node and re-add it using the same index.
    network.peers[0].shut_down().await;
    network.peers.remove(0);
    let node_0_storage = &storage[1];
    let node_0_persistence = persistence[1].clone();
    let node_0_port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    tracing::info!("node_0_port {node_0_port}");
    // enable query module with api peers
    let opt = Options::with_port(node_0_port).query_sql(
        Query {
            peers: vec![format!("http://localhost:{api_port}").parse().unwrap()],
            ..Default::default()
        },
        tmp_options(node_0_storage),
    );

    // start the query node so that it builds the merklized state
    let node_0 = opt
        .clone()
        .serve(|metrics, consumer, storage| {
            let cfg = network.cfg.clone();
            let node_0_persistence = node_0_persistence.clone();
            let state = state.clone();
            async move {
                Ok(cfg
                    .init_node(
                        1,
                        state,
                        node_0_persistence.clone(),
                        Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                            vec![format!("http://localhost:{api_port}").parse().unwrap()],
                            Default::default(),
                            Duration::from_secs(2),
                            &NoMetrics,
                        )),
                        storage,
                        &*metrics,
                        test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                        consumer,
                        upgrade,
                        Default::default(),
                    )
                    .await)
            }
            .boxed()
        })
        .await
        .unwrap();

    let mut events = network.peers[2].event_stream();
    // wait for 1 epoch
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 1).await;

    // shutdown the node for 3 epochs
    drop(node_0);

    // wait for 4 epochs
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 4).await;

    // start the node again.
    tracing::info!("restarting node");
    let node_0 = opt
        .serve(|metrics, consumer, storage| {
            let cfg = network.cfg.clone();
            async move {
                Ok(cfg
                    .init_node(
                        1,
                        state,
                        node_0_persistence,
                        Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                            vec![format!("http://localhost:{api_port}").parse().unwrap()],
                            Default::default(),
                            Duration::from_secs(2),
                            &NoMetrics,
                        )),
                        storage,
                        &*metrics,
                        test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                        consumer,
                        upgrade,
                        Default::default(),
                    )
                    .await)
            }
            .boxed()
        })
        .await
        .unwrap();

    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{node_0_port}").parse().unwrap());
    client.connect(None).await;

    wait_for_epochs(&mut events, EPOCH_HEIGHT, 6).await;

    let epoch_7_block = EPOCH_HEIGHT * 6 + 1;

    // check that the node's state has reward accounts
    let mut retries = 0;
    loop {
        sleep(Duration::from_secs(1)).await;
        let state = node_0.decided_state().await.unwrap();

        let leaves = if upgrade.base == EPOCH_VERSION {
            // Use legacy tree for V3
            state.reward_merkle_tree_v1.num_leaves()
        } else {
            // Use new tree for V4 and above
            state.reward_merkle_tree_v2.num_leaves()
        };

        if leaves > 0 {
            tracing::info!("Node's state has reward accounts");
            break;
        }

        retries += 1;
        if retries > 120 {
            panic!("max retries reached. failed to catchup reward state");
        }
    }

    retries = 0;
    // check that the node has stored atleast 6 epochs merklized state in persistence
    loop {
        sleep(Duration::from_secs(3)).await;

        let bh = client
            .get::<u64>("block-state/block-height")
            .send()
            .await
            .expect("block height not found");

        tracing::info!("block state: block height={bh}");
        if bh > epoch_7_block {
            break;
        }

        retries += 1;
        if retries > 30 {
            panic!(
                "max retries reached. block state block height is less than epoch 7 start block"
            );
        }
    }

    // shutdown consensus to freeze the state
    node_0.shutdown_consensus().await;
    let decided_leaf = node_0.decided_leaf().await;
    let state = node_0.decided_state().await.unwrap();
    tracing::info!(
        height = decided_leaf.height(),
        ?decided_leaf,
        ?state,
        "final state"
    );

    let height = decided_leaf.height();
    let num_leaves = state.block_merkle_tree.num_leaves();
    tracing::info!(height, num_leaves, "checking block merkle tree state");
    state
        .block_merkle_tree
        .lookup(height - 1)
        .expect_ok()
        .unwrap_or_else(|err| {
            panic!(
                "block state not found ({err:#}):\n{:#?}",
                state.block_merkle_tree
            )
        });

    Ok(())
}

#[rstest]
#[case(POS_V4)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_state_reconstruction(#[case] upgrade: Upgrade) -> anyhow::Result<()> {
    // This test verifies that a query node can successfully reconstruct its state
    // after being shut down from the database
    //
    // Steps:
    // 1. Start a test network with 5 nodes.
    // 2. Add a query node connected to the network.
    // 3. Let the network run until 3 epochs have passed.
    // 4. Shut down the query node.
    // 5. Attempt to reconstruct its state from storage using:
    //    - No fee/reward accounts
    //    - Only fee accounts
    //    - Only reward accounts
    //    - Both fee and reward accounts
    // 6. Assert that the reconstructed state is correct in all scenarios.

    const EPOCH_HEIGHT: u64 = 10;

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    tracing::info!("API PORT = {api_port}");
    const NUM_NODES: usize = 5;

    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let config = TestNetworkConfigBuilder::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(api_port).light_client(Default::default()),
        ))
        .network_config(network_config)
        .persistences(persistence.clone())
        .catchups(std::array::from_fn(|_| {
            StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![format!("http://localhost:{api_port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .pos_hook(
            DelegationConfig::MultipleDelegators,
            hotshot_contract_adapter::stake_table::StakeTableContractVersion::V3,
            upgrade,
        )
        .await
        .unwrap()
        .build();
    let state = config.states()[0].clone();
    let mut network = TestNetwork::new(config, upgrade).await;
    // Remove peer 0 and restart it with the query module enabled.
    // Adding an additional node to the test network is not straight forward,
    // as the keys have already been initialized in the config above.
    // So, we remove this node and re-add it using the same index.
    // Await the shutdown so the node's cliquenet listener releases its
    // port before the replacement node binds the same address.
    network.peers[0].shut_down().await;
    network.peers.remove(0);

    let node_0_storage = &storage[1];
    let node_0_persistence = persistence[1].clone();
    let node_0_port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    tracing::info!("node_0_port {node_0_port}");
    let opt = Options::with_port(node_0_port).query_sql(
        Query {
            peers: vec![format!("http://localhost:{api_port}").parse().unwrap()],
            ..Query::test()
        },
        tmp_options(node_0_storage),
    );
    let node_0 = opt
        .clone()
        .serve(|metrics, consumer, storage| {
            let cfg = network.cfg.clone();
            let node_0_persistence = node_0_persistence.clone();
            let state = state.clone();
            async move {
                Ok(cfg
                    .init_node(
                        1,
                        state,
                        node_0_persistence.clone(),
                        Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                            vec![format!("http://localhost:{api_port}").parse().unwrap()],
                            Default::default(),
                            Duration::from_secs(2),
                            &NoMetrics,
                        )),
                        storage,
                        &*metrics,
                        test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                        consumer,
                        upgrade,
                        Default::default(),
                    )
                    .await)
            }
            .boxed()
        })
        .await
        .unwrap();

    let mut events = network.peers[2].event_stream();
    // Wait until at least 3 epochs have passed
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 3).await;

    tracing::warn!("shutting down node 0");

    node_0.shutdown_consensus().await;

    let instance = node_0.node_state();
    let state = node_0.decided_state().await.unwrap();
    let fee_accounts = state
        .fee_merkle_tree
        .clone()
        .into_iter()
        .map(|(acct, _)| acct)
        .collect::<Vec<_>>();
    let reward_accounts = match upgrade.base {
        EPOCH_VERSION => state
            .reward_merkle_tree_v1
            .clone()
            .into_iter()
            .map(|(acct, _)| RewardAccountV2::from(acct))
            .collect::<Vec<_>>(),
        DRB_AND_HEADER_UPGRADE_VERSION => state
            .reward_merkle_tree_v2
            .clone()
            .into_iter()
            .map(|(acct, _)| acct)
            .collect::<Vec<_>>(),
        _ => panic!("invalid version"),
    };

    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{node_0_port}").parse().unwrap());
    client.connect(Some(Duration::from_secs(10))).await;

    // wait 3s to be sure that all the
    // transactions have been committed
    sleep(Duration::from_secs(3)).await;

    tracing::info!("getting node block height");
    let node_block_height = client
        .get::<u64>("node/block-height")
        .send()
        .await
        .context("getting Espresso block height")
        .unwrap();

    tracing::info!("node block height={node_block_height}");

    let leaf_query_data = client
        .get::<LeafQueryData<SeqTypes>>(&format!("availability/leaf/{}", node_block_height - 1))
        .send()
        .await
        .context("error getting leaf")
        .unwrap();

    tracing::info!("leaf={leaf_query_data:?}");
    let leaf = leaf_query_data.leaf();
    let to_view = leaf.view_number() + 1;

    let ds = SqlStorage::connect(
        Config::try_from(&node_0_persistence).unwrap(),
        StorageConnectionType::Sequencer,
    )
    .await
    .unwrap();
    let mut tx = ds.read().await?;

    let (state, leaf) = reconstruct_state(
        &instance,
        &ds,
        &mut tx,
        node_block_height - 1,
        to_view,
        &[],
        &[],
    )
    .await
    .unwrap();
    assert_eq!(leaf.view_number(), to_view);
    assert!(
        state
            .block_merkle_tree
            .lookup(node_block_height - 1)
            .expect_ok()
            .is_ok(),
        "inconsistent block merkle tree"
    );

    // Reconstruct fee state
    let (state, leaf) = reconstruct_state(
        &instance,
        &ds,
        &mut tx,
        node_block_height - 1,
        to_view,
        &fee_accounts,
        &[],
    )
    .await
    .unwrap();

    assert_eq!(leaf.view_number(), to_view);
    assert!(
        state
            .block_merkle_tree
            .lookup(node_block_height - 1)
            .expect_ok()
            .is_ok(),
        "inconsistent block merkle tree"
    );

    for account in &fee_accounts {
        state.fee_merkle_tree.lookup(account).expect_ok().unwrap();
    }

    // Reconstruct reward state

    let (state, leaf) = reconstruct_state(
        &instance,
        &ds,
        &mut tx,
        node_block_height - 1,
        to_view,
        &[],
        &reward_accounts,
    )
    .await
    .unwrap();

    match upgrade.base {
        EPOCH_VERSION => {
            for account in reward_accounts.clone() {
                state
                    .reward_merkle_tree_v1
                    .lookup(RewardAccountV1::from(account))
                    .expect_ok()
                    .unwrap();
            }
        },
        DRB_AND_HEADER_UPGRADE_VERSION => {
            for account in &reward_accounts {
                state
                    .reward_merkle_tree_v2
                    .lookup(account)
                    .expect_ok()
                    .unwrap();
            }
        },
        _ => panic!("invalid version"),
    };

    assert_eq!(leaf.view_number(), to_view);
    assert!(
        state
            .block_merkle_tree
            .lookup(node_block_height - 1)
            .expect_ok()
            .is_ok(),
        "inconsistent block merkle tree"
    );
    // Reconstruct reward and fee state

    let (state, leaf) = reconstruct_state(
        &instance,
        &ds,
        &mut tx,
        node_block_height - 1,
        to_view,
        &fee_accounts,
        &reward_accounts,
    )
    .await
    .unwrap();

    assert!(
        state
            .block_merkle_tree
            .lookup(node_block_height - 1)
            .expect_ok()
            .is_ok(),
        "inconsistent block merkle tree"
    );
    assert_eq!(leaf.view_number(), to_view);

    match upgrade.base {
        EPOCH_VERSION => {
            for account in reward_accounts.clone() {
                state
                    .reward_merkle_tree_v1
                    .lookup(RewardAccountV1::from(account))
                    .expect_ok()
                    .unwrap();
            }
        },
        DRB_AND_HEADER_UPGRADE_VERSION => {
            for account in &reward_accounts {
                state
                    .reward_merkle_tree_v2
                    .lookup(account)
                    .expect_ok()
                    .unwrap();
            }
        },
        _ => panic!("invalid version"),
    };

    for account in &fee_accounts {
        state.fee_merkle_tree.lookup(account).expect_ok().unwrap();
    }

    Ok(())
}

/// Test that `fetch_leaf` returns a leaf with exactly the requested block height.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_fetch_leaf_returns_exact_height() -> anyhow::Result<()> {
    const EPOCH_HEIGHT: u64 = 10;
    const NUM_NODES: usize = 5;
    const TARGET_HEIGHT: u64 = EPOCH_HEIGHT * 3 + 2;

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    let port = reserve_tcp_port().expect("No ports free for query service");

    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let catchup_peers = std::array::from_fn(|_| {
        StatePeers::<StaticVersion<0, 1>>::from_urls(
            vec![format!("http://localhost:{port}").parse().unwrap()],
            Default::default(),
            Duration::from_secs(2),
            &NoMetrics,
        )
    });

    let config = TestNetworkConfigBuilder::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(port),
        ))
        .network_config(network_config)
        .persistences(persistence)
        .catchups(catchup_peers)
        .pos_hook(
            DelegationConfig::MultipleDelegators,
            Default::default(),
            POS_V4,
        )
        .await?
        .build();

    let network = TestNetwork::new(config, POS_V4).await;

    // Wait for chain to advance past our target height
    let height_client: Client<ServerError, StaticVersion<0, 1>> =
        Client::new(format!("http://localhost:{port}").parse().unwrap());
    wait_until_block_height(&height_client, "node/block-height", TARGET_HEIGHT + 5).await;

    let coordinator = network.server.node_state().coordinator;

    // Use StatePeers to fetch the leaf at the exact target height
    let catchup = StatePeers::<StaticVersion<0, 1>>::from_urls(
        vec![format!("http://localhost:{port}").parse().unwrap()],
        Default::default(),
        Duration::from_secs(5),
        &NoMetrics,
    );

    let leaf = catchup.fetch_leaf(coordinator, TARGET_HEIGHT).await?;

    assert_eq!(
        leaf.height(),
        TARGET_HEIGHT,
        "fetch_leaf must return the leaf at exactly the requested height"
    );

    Ok(())
}

/// Start a network at V5 from genesis, restart ALL nodes just before an epoch transition block
/// (`boundary - 3`), and confirm the chain keeps producing blocks afterward.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_v5_restart_before_epoch_boundary() {
    const NUM_NODES: usize = 3;
    const EPOCH_HEIGHT: u64 = 10;
    // Rewards start in epoch 4. Restart 3 blocks before the boundary that closes epoch 4, so
    // the restarted network produces the boundary block (`4 * EPOCH_HEIGHT`) itself.
    const RESTART_EPOCH: u64 = 4;
    const RESTART_BOUNDARY: u64 = RESTART_EPOCH * EPOCH_HEIGHT;
    const RESTART_HEIGHT: u64 = RESTART_BOUNDARY - 3;
    // Blocks to produce after the restart before declaring success.
    const BLOCKS_AFTER_RESTART: u64 = 5;

    const V5: Upgrade = Upgrade::trivial(EPOCH_REWARD_VERSION);

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    // Slow empty-block production so we comfortably stop at the exact target height. On an idle
    // chain the empty-block time is ~`builder_timeout`; raise `next_view_timeout` above it so
    // the slow (but healthy) views aren't treated as failures.
    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .builder_timeout(Duration::from_secs(3))
        .next_view_timeout(Duration::from_secs(10))
        .build();

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
        .network_config(network_config)
        .pos_hook(DelegationConfig::MultipleDelegators, Default::default(), V5)
        .await
        .unwrap()
        .build();

    let mut network = TestNetwork::new(config, V5).await;

    // Watch the decide stream and stop as soon as `boundary - 3` is decided. Consuming the
    // stream (rather than polling `decided_leaf`) makes this independent of block timing: we
    // see every decided leaf and stop the network the moment the target height appears, so we
    // can never race past it regardless of how fast blocks are produced.
    {
        let mut events = network.server.event_stream();
        'wait: loop {
            let event = events
                .next()
                .await
                .expect("event stream ended unexpectedly");
            let CoordinatorEvent::LegacyEvent(Event {
                event: EventType::Decide { leaf_chain, .. },
                ..
            }) = event
            else {
                continue;
            };
            // `leaf_chain` is newest-first; once any decided leaf has reached the target
            // height, the chain is at or past it.
            for LeafInfo { leaf, .. } in leaf_chain.iter() {
                if leaf.block_header().height() >= RESTART_HEIGHT {
                    break 'wait;
                }
            }
        }
    }

    let restart_height = network.server.decided_leaf().await.height();
    tracing::info!(
        restart_height,
        restart_epoch = RESTART_EPOCH,
        restart_boundary = RESTART_BOUNDARY,
        "restarting all nodes 3 blocks before an epoch boundary"
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
        .network_config(saved_cfg)
        .build();
    let network2 = TestNetwork::new(config2, V5).await;

    // The restarted network must keep advancing, including across the next epoch boundary.
    // Require BLOCKS_AFTER_RESTART new decides, using a lack-of-progress watchdog so a
    // healthy-but-slow chain still passes.
    let target_height = restart_height + BLOCKS_AFTER_RESTART;
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
                panic!(
                    "chain stalled after restart 3 blocks before boundary {RESTART_BOUNDARY}: no \
                     new decide for 60s at height {last_height}, unable to produce/cross the \
                     epoch boundary (target height was {target_height})."
                );
            }
        }
    }
}
