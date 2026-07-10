use super::*;

#[rstest]
#[case(POS_V3)]
#[case(POS_V4)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub(crate) async fn test_state_cert_query(#[case] upgrade: Upgrade) {
    const TEST_EPOCH_HEIGHT: u64 = 10;
    const TEST_EPOCHS: u64 = 5;

    let network_config = TestConfigBuilder::default()
        .epoch_height(TEST_EPOCH_HEIGHT)
        .build();

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    tracing::info!("API PORT = {api_port}");
    const NUM_NODES: usize = 2;

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
            Options::with_port(api_port).catchup(Default::default()),
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

    let network = TestNetwork::new(config, upgrade).await;
    let mut events = network.server.event_stream();

    // Wait until 5 epochs have passed.
    loop {
        let event = events.next().await.unwrap();
        tracing::info!("Received event from handle: {event:?}");

        if let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        {
            println!(
                "Decide event received: {:?}",
                leaf_chain.first().unwrap().leaf.height()
            );
            if let Some(first_leaf) = leaf_chain.first() {
                let height = first_leaf.leaf.height();
                tracing::info!("Decide event received at height: {height}");

                if height >= TEST_EPOCHS * TEST_EPOCH_HEIGHT {
                    break;
                }
            }
        }
    }

    // Connect client.
    let client: Client<ServerError, StaticVersion<0, 1>> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());
    client.connect(Some(Duration::from_secs(10))).await;

    // Get the state cert for the epoch 3 to 5
    for i in 3..=TEST_EPOCHS {
        // v2

        let state_query_data_v2 = client
            .get::<StateCertQueryDataV2<SeqTypes>>(&format!("availability/state-cert-v2/{i}"))
            .send()
            .await
            .unwrap();
        let state_cert_v2 = state_query_data_v2.0.clone();
        tracing::info!("state_cert_v2: {state_cert_v2:?}");
        assert_eq!(state_cert_v2.epoch.u64(), i);
        assert_eq!(
            state_cert_v2.light_client_state.block_height,
            i * TEST_EPOCH_HEIGHT - 5
        );
        let block_height = state_cert_v2.light_client_state.block_height;

        let header: Header = client
            .get(&format!("availability/header/{block_height}"))
            .send()
            .await
            .unwrap();

        // verify auth root if the consensus version is v4
        if header.version() == DRB_AND_HEADER_UPGRADE_VERSION {
            let auth_root = state_cert_v2.auth_root;
            let header_auth_root = header.auth_root().unwrap();
            if auth_root.is_zero() || header_auth_root.is_zero() {
                panic!("auth root shouldn't be zero");
            }

            assert_eq!(auth_root, header_auth_root, "auth root mismatch");
        }

        // v1
        let state_query_data_v1 = client
            .get::<StateCertQueryDataV1<SeqTypes>>(&format!("availability/state-cert/{i}"))
            .send()
            .await
            .unwrap();

        let state_cert_v1 = state_query_data_v1.0.clone();
        tracing::info!("state_cert_v1: {state_cert_v1:?}");
        assert_eq!(state_query_data_v1, state_query_data_v2.into());
    }
}

/// Test state certificate catchup functionality by simulating a node that falls behind and needs
/// to catch up. This test starts a 5-node network with epoch height 10, waits for 3 epochs to
/// pass, then removes and restarts node 0 with a fresh storage. The
/// restarted node catches up for the missing state certificates.
#[rstest]
#[case(POS_V3)]
#[case(POS_V4)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
pub(crate) async fn test_state_cert_catchup(#[case] upgrade: Upgrade) {
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

    let mut events = network.peers[2].event_stream();
    // Wait until at least 5 epochs have passed
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 3).await;

    // Remove peer 0 and restart it with the query module enabled.
    // Adding an additional node to the test network is not straight forward,
    // as the keys have already been initialized in the config above.
    // So, we remove this node and re-add it using the same index.
    network.peers.remove(0);

    let new_storage: hotshot_query_service::data_source::sql::testing::TmpDb =
        SqlDataSource::create_storage().await;
    let new_persistence: persistence::sql::Options =
        <SqlDataSource as TestableSequencerDataSource>::persistence_options(&new_storage);

    let node_0_port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    tracing::info!("node_0_port {node_0_port}");
    let opt = Options::with_port(node_0_port).query_sql(
        Query {
            peers: vec![format!("http://localhost:{api_port}").parse().unwrap()],
            ..Query::test()
        },
        tmp_options(&new_storage),
    );
    let node_0 = opt
        .clone()
        .serve(|metrics, consumer, storage| {
            let cfg = network.cfg.clone();
            let new_persistence = new_persistence.clone();
            let state = state.clone();
            async move {
                Ok(cfg
                    .init_node(
                        1,
                        state,
                        new_persistence.clone(),
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

    let mut events = node_0.event_stream();
    // Wait until at least 5 epochs have passed
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 5).await;

    let client: Client<ServerError, StaticVersion<0, 1>> =
        Client::new(format!("http://localhost:{node_0_port}").parse().unwrap());
    client.connect(Some(Duration::from_secs(60))).await;

    for epoch in 3..=5 {
        let state_cert = client
            .get::<StateCertQueryDataV2<SeqTypes>>(&format!("availability/state-cert-v2/{epoch}"))
            .send()
            .await
            .unwrap();
        assert_eq!(state_cert.0.epoch.u64(), epoch);
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_light_client_completeness() {
    // Run the through a protocol upgrade and epoch change, then check that we are able to get a
    // correct light client proof for every finalized leaf.

    const NUM_NODES: usize = 1;
    const EPOCH_HEIGHT: u64 = 200;

    let upgrade = Upgrade::new(LEGACY_VERSION, EPOCH_VERSION);
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let url: Url = format!("http://localhost:{port}").parse().unwrap();

    let test_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(321)
        .set_upgrades(upgrade.target)
        .await
        .build();

    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(
            SqlDataSource::options(&storage[0], Options::with_port(port))
                .light_client(Default::default()),
        )
        .persistences(persistence.clone())
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

    let mut network = TestNetwork::new(config, upgrade).await;
    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);
    client.connect(None).await;

    // Get a leaf stream so that we can wait for various events. Also keep track of each leaf
    // yielded, which we can use as ground truth later in the test.
    let mut actual_leaves = vec![];
    let mut actual_blocks = vec![];
    let mut leaves = client
        .socket("availability/stream/leaves/0")
        .subscribe::<LeafQueryData<SeqTypes>>()
        .await
        .unwrap()
        .zip(
            client
                .socket("availability/stream/blocks/0")
                .subscribe::<BlockQueryData<SeqTypes>>()
                .await
                .unwrap(),
        )
        .map(|(leaf, block)| {
            let leaf = leaf.unwrap();
            let block = block.unwrap();
            actual_leaves.push(leaf.clone());
            actual_blocks.push(block);
            leaf
        });

    // Wait for the upgrade to take effect.
    let (upgrade_height, first_epoch) = loop {
        let leaf: LeafQueryData<SeqTypes> = leaves.next().await.unwrap();
        if leaf.header().version() < EPOCH_VERSION {
            tracing::info!(version = %leaf.header().version(), height = leaf.header().height(), view = ?leaf.leaf().view_number(), "waiting for epoch upgrade");
            continue;
        }
        break (leaf.height(), leaf.leaf().epoch(EPOCH_HEIGHT).unwrap());
    };
    tracing::info!(upgrade_height, ?first_epoch, "epochs enabled");

    // Wait for two epoch changes (so we get to the first epoch that actually uses the stake
    // table).
    let mut epoch_heights = [0; 2];
    for (i, epoch_height) in epoch_heights.iter_mut().enumerate() {
        let desired_epoch = first_epoch + (i as u64) + 1;
        *epoch_height = loop {
            let leaf = leaves.next().await.unwrap();
            let epoch = leaf.leaf().epoch(EPOCH_HEIGHT).unwrap();
            if epoch > desired_epoch {
                tracing::info!(
                    height = leaf.height(),
                    ?desired_epoch,
                    ?epoch,
                    "changed epoch"
                );
                break leaf.height();
            }
            tracing::info!(
                ?desired_epoch,
                height = leaf.header().height(),
                view = ?leaf.leaf().view_number(),
                "waiting for epoch change"
            );
        };
    }

    // Wait a few more blocks.
    let max_block = epoch_heights[1] + 1;
    loop {
        let leaf = leaves.next().await.unwrap();
        if leaf.height() > max_block {
            break;
        }
        tracing::info!(max_block, height = leaf.height(), "waiting for block");
    }

    // Stop consensus. All the blocks we are going to query have already been produced.
    // Continuing to run consensus would just waste resources while we check stuff.
    network.stop_consensus().await;

    // Check light client. Querying every single block is too slow, so we'll check a few blocks
    // around various critical points:
    let heights =
    // * The first few blocks, including genesis
        (0..=1)
    // * A few blocks just before and after the upgrade
        .chain(upgrade_height-1..=upgrade_height+1)
    // * A few blocks just before and after the first epoch change
        .chain(epoch_heights[0]-1..=epoch_heights[0] + 1)
    // * A few blocks just before and after the stake table comes into effect
        .chain(epoch_heights[1]-1..=max_block);

    let quorum = EpochChangeQuorum::new(EPOCH_HEIGHT);
    for i in heights {
        let leaf = &actual_leaves[i as usize];
        let block = &actual_blocks[i as usize];
        tracing::info!(i, ?leaf, ?block, "check leaf");

        // Get the same leaf proof by various IDs.
        let client = &client;
        let proofs = try_join_all(
            [
                format!("light-client/leaf/{i}"),
                format!("light-client/leaf/hash/{}", leaf.hash()),
                format!("light-client/leaf/block-hash/{}", leaf.block_hash()),
            ]
            .into_iter()
            .map(|path| async move {
                tracing::info!(i, path, "fetch leaf proof");
                let proof = client.get::<LeafProof>(&path).send().await?;
                Ok::<_, anyhow::Error>((path, proof))
            }),
        )
        .await
        .unwrap();

        // Check proofs against expected leaf.
        for (path, proof) in proofs {
            tracing::info!(i, path, ?proof, "check leaf proof");
            assert_eq!(
                proof.verify(LeafProofHint::Quorum(&quorum)).await.unwrap(),
                *leaf
            );
        }

        // Get the corresponding header.
        let root_height = i + 1;
        let root = actual_leaves[root_height as usize].header();
        let proofs = try_join_all(
            [
                format!("light-client/header/{root_height}/{i}"),
                format!(
                    "light-client/header/{root_height}/hash/{}",
                    leaf.block_hash()
                ),
            ]
            .into_iter()
            .map(|path| async move {
                tracing::info!(i, path, "get header proof");
                let proof = client.get::<HeaderProof>(&path).send().await?;
                Ok::<_, anyhow::Error>((path, proof))
            }),
        )
        .await
        .unwrap();
        for (path, proof) in proofs {
            tracing::info!(i, path, ?proof, "check header proof");
            assert_eq!(
                proof.verify_ref(root.block_merkle_tree_root()).unwrap(),
                leaf.header()
            );
        }

        // Get the corresponding payload.
        let proof = client
            .get::<PayloadProof>(&format!("light-client/payload/{i}"))
            .send()
            .await
            .unwrap();
        assert_eq!(proof.verify(leaf.header()).unwrap(), *block.payload());
    }

    // Check light client stake table.
    let events: Vec<StakeTableEvent> = client
        .get(&format!("light-client/stake-table/{}", first_epoch + 2))
        .send()
        .await
        .unwrap();
    let mut state_from_events = StakeTableState::default();
    for event in events {
        state_from_events.apply_event(event).unwrap().unwrap();
    }

    assert_eq!(
        state_from_events.into_validators(),
        network
            .server
            .consensus_handle()
            .storage()
            .await
            .load_all_validators(first_epoch + 2, 0, 1_000_000)
            .await
            .unwrap()
            .into_iter()
            .map(|v| (v.account, v))
            .collect::<RegisteredValidatorMap>()
    );

    // Querying for a stake table before the first real epoch is an error.
    let err = client
        .get::<Vec<StakeTableEvent>>(&format!("light-client/stake-table/{}", first_epoch + 1))
        .send()
        .await
        .unwrap_err();
    assert_eq!(err.status(), StatusCode::BAD_REQUEST);
}
