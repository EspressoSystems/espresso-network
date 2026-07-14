use super::*;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_duplicate_events_from_contract() -> anyhow::Result<()> {
    // TODO(abdul): This test currently uses TestNetwork only for contract deployment and for L1 block number.
    // Once the stake table deployment logic is refactored and isolated, TestNetwork here will be unnecessary

    let epoch_height = 20;

    let network_config = TestConfigBuilder::default()
        .epoch_height(epoch_height)
        .build();

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    const NUM_NODES: usize = 5;
    // Initialize nodes.
    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let l1_url = network_config.l1_url();
    let config = TestNetworkConfigBuilder::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(api_port),
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
            Default::default(),
            POS_V3,
        )
        .await
        .unwrap()
        .build();

    let network = TestNetwork::new(config, POS_V3).await;

    let mut prev_st = None;
    let state = network.server.decided_state().await.unwrap();
    let chain_config = state.chain_config.resolve().expect("resolve chain config");
    let stake_table = chain_config.stake_table_contract.unwrap();

    let l1_client = L1ClientOptions::default()
        .connect(vec![l1_url])
        .expect("failed to connect to l1");

    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    let mut headers = client
        .socket("availability/stream/headers/0")
        .subscribe::<Header>()
        .await
        .unwrap();

    let mut target_bh = 0;
    while let Some(header) = headers.next().await {
        let header = header.unwrap();
        println!("got header with height {}", header.height());
        if header.height() == 0 {
            continue;
        }
        let l1_block = header.l1_finalized().expect("l1 block not found");

        let sorted_events = Fetcher::fetch_events_from_contract(
            l1_client.clone(),
            stake_table,
            None,
            l1_block.number(),
        )
        .await?;

        let mut sorted_dedup_removed = sorted_events.clone();
        sorted_dedup_removed.dedup();

        assert_eq!(
            sorted_events.len(),
            sorted_dedup_removed.len(),
            "duplicates found"
        );

        // This also checks if there is a duplicate registration
        let stake_table =
            validators_from_l1_events(sorted_events.into_iter().map(|(_, e)| e)).unwrap();
        if let Some(prev_st) = prev_st {
            assert_eq!(stake_table, prev_st);
        }

        prev_st = Some(stake_table);

        if target_bh == 100 {
            break;
        }

        target_bh = header.height();
    }

    Ok(())
}

#[rstest]
#[case(POS_V3)]
#[case(POS_V4)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_node_stake_table_api(#[case] upgrade: Upgrade) {
    let epoch_height = 20;

    let network_config = TestConfigBuilder::default()
        .epoch_height(epoch_height)
        .build();

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    const NUM_NODES: usize = 2;
    // Initialize nodes.
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
            Options::with_port(api_port),
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
            Default::default(),
            upgrade,
        )
        .await
        .unwrap()
        .build();

    let _network = TestNetwork::new(config, upgrade).await;

    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    // wait for atleast 2 epochs
    let _blocks = client
        .socket("availability/stream/blocks/0")
        .subscribe::<BlockQueryData<SeqTypes>>()
        .await
        .unwrap()
        .take(40)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    for i in 1..=3 {
        let _st = client
            .get::<Vec<PeerConfig<SeqTypes>>>(&format!("node/stake-table/{}", i as u64))
            .send()
            .await
            .expect("failed to get stake table");
    }

    let _st = client
        .get::<StakeTableWithEpochNumber<SeqTypes>>("node/stake-table/current")
        .send()
        .await
        .expect("failed to get stake table");
}

#[rstest]
#[case(POS_V3)]
#[case(POS_V4)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_epoch_stake_table_catchup(#[case] upgrade: Upgrade) {
    const EPOCH_HEIGHT: u64 = 10;
    const NUM_NODES: usize = 6;

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    // Initialize storage for each node
    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;

    let persistence_options: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    // setup catchup peers
    let catchup_peers = std::array::from_fn(|_| {
        StatePeers::<StaticVersion<0, 1>>::from_urls(
            vec![format!("http://localhost:{port}").parse().unwrap()],
            Default::default(),
            Duration::from_secs(2),
            &NoMetrics,
        )
    });
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(port),
        ))
        .network_config(network_config)
        .persistences(persistence_options.clone())
        .catchups(catchup_peers)
        .pos_hook(
            DelegationConfig::MultipleDelegators,
            Default::default(),
            upgrade,
        )
        .await
        .unwrap()
        .build();

    let state = config.states()[0].clone();
    let mut network = TestNetwork::new(config, upgrade).await;

    // Wait for the peer 0 (node 1) to advance past three epochs
    let mut events = network.peers[0].event_stream();
    while let Some(event) = events.next().await {
        if let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        {
            let height = leaf_chain[0].leaf.height();
            tracing::info!("Node 0 decided at height: {height}");
            if height > EPOCH_HEIGHT * 3 {
                break;
            }
        }
    }

    // Shutdown and remove node 1 to simulate falling behind
    tracing::info!("Shutting down peer 0");
    network.peers.remove(0);

    // Wait for epochs to progress with node 1 offline
    let mut events = network.server.event_stream();
    while let Some(event) = events.next().await {
        if let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        {
            let height = leaf_chain[0].leaf.height();
            if height > EPOCH_HEIGHT * 7 {
                break;
            }
        }
    }

    // add node 1 to the network with fresh storage
    let storage = SqlDataSource::create_storage().await;
    let options = <SqlDataSource as TestableSequencerDataSource>::persistence_options(&storage);
    tracing::info!("Restarting peer 0");
    let node = network
        .cfg
        .init_node(
            1,
            state,
            options,
            Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![format!("http://localhost:{port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )),
            None,
            &NoMetrics,
            test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
            NullEventConsumer,
            upgrade,
            Default::default(),
        )
        .await;

    let coordinator = node.node_state().coordinator;
    let server_node_state = network.server.node_state();
    let server_coordinator = server_node_state.coordinator;
    // Verify that the restarted node catches up for each epoch
    for epoch_num in 1..=7 {
        let epoch = EpochNumber::new(epoch_num);
        let node_em = match coordinator.membership_for_epoch(Some(epoch)) {
            Ok(em) => em,
            Err(_) => coordinator.wait_for_catchup(epoch).await.unwrap(),
        };
        let server_em = match server_coordinator.membership_for_epoch(Some(epoch)) {
            Ok(em) => em,
            Err(_) => server_coordinator.wait_for_catchup(epoch).await.unwrap(),
        };

        println!("have stake table for epoch = {epoch_num}");

        let node_stake_table = HSStakeTable::from_iter(node_em.stake_table());
        let stake_table = HSStakeTable::from_iter(server_em.stake_table());
        println!("asserting stake table for epoch = {epoch_num}");

        assert_eq!(
            node_stake_table, stake_table,
            "Stake table mismatch for epoch {epoch_num}",
        );
    }
}

#[rstest]
#[case(POS_V3)]
#[case(POS_V4)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_epoch_stake_table_catchup_stress(#[case] upgrade: Upgrade) {
    const EPOCH_HEIGHT: u64 = 10;
    const NUM_NODES: usize = 6;

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    // Initialize storage for each node
    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;

    let persistence_options: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    // setup catchup peers
    let catchup_peers = std::array::from_fn(|_| {
        StatePeers::<StaticVersion<0, 1>>::from_urls(
            vec![format!("http://localhost:{port}").parse().unwrap()],
            Default::default(),
            Duration::from_secs(2),
            &NoMetrics,
        )
    });
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(port),
        ))
        .network_config(network_config)
        .persistences(persistence_options.clone())
        .catchups(catchup_peers)
        .pos_hook(
            DelegationConfig::MultipleDelegators,
            Default::default(),
            upgrade,
        )
        .await
        .unwrap()
        .build();

    let state = config.states()[0].clone();
    let mut network = TestNetwork::new(config, upgrade).await;

    // Wait for the peer 0 (node 1) to advance past three epochs
    let mut events = network.peers[0].event_stream();
    while let Some(event) = events.next().await {
        if let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        {
            let height = leaf_chain[0].leaf.height();
            tracing::info!("Node 0 decided at height: {height}");
            if height > EPOCH_HEIGHT * 3 {
                break;
            }
        }
    }

    // Shutdown and remove node 1 to simulate falling behind
    tracing::info!("Shutting down peer 0");
    network.peers.remove(0);

    // Wait for epochs to progress with node 1 offline
    let mut events = network.server.event_stream();
    while let Some(event) = events.next().await {
        if let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = event
        {
            let height = leaf_chain[0].leaf.height();
            tracing::info!("Server decided at height: {height}");
            //  until 7 epochs
            if height > EPOCH_HEIGHT * 7 {
                break;
            }
        }
    }

    // add node 1 to the network with fresh storage
    let storage = SqlDataSource::create_storage().await;
    let options = <SqlDataSource as TestableSequencerDataSource>::persistence_options(&storage);

    tracing::info!("Restarting peer 0");
    let node = network
        .cfg
        .init_node(
            1,
            state,
            options,
            Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![format!("http://localhost:{port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )),
            None,
            &NoMetrics,
            test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
            NullEventConsumer,
            upgrade,
            Default::default(),
        )
        .await;

    let coordinator = node.node_state().coordinator;

    let server_node_state = network.server.node_state();
    let server_coordinator = server_node_state.coordinator;

    // Trigger catchup for all epochs in quick succession and in random order
    let mut rand_epochs: Vec<_> = (1..=7).collect();
    rand_epochs.shuffle(&mut rand::thread_rng());
    println!("trigger catchup in this order: {rand_epochs:?}");
    for epoch_num in rand_epochs {
        let epoch = EpochNumber::new(epoch_num);
        let _ = coordinator.membership_for_epoch(Some(epoch));
    }

    // Verify that the restarted node catches up for each epoch
    for epoch_num in 1..=7 {
        println!("getting stake table for epoch = {epoch_num}");
        let epoch = EpochNumber::new(epoch_num);
        let node_em = coordinator.wait_for_catchup(epoch).await.unwrap();
        let server_em = match server_coordinator.membership_for_epoch(Some(epoch)) {
            Ok(em) => em,
            Err(_) => server_coordinator.wait_for_catchup(epoch).await.unwrap(),
        };

        println!("have stake table for epoch = {epoch_num}");

        let node_stake_table = HSStakeTable::from_iter(node_em.stake_table());
        let stake_table = HSStakeTable::from_iter(server_em.stake_table());

        println!("asserting stake table for epoch = {epoch_num}");

        assert_eq!(
            node_stake_table, stake_table,
            "Stake table mismatch for epoch {epoch_num}",
        );
    }
}

/// `chain_id`: None = default (35353, non-mainnet), Some(1) = mainnet
#[rstest]
#[case(POS_V4, None)]
#[case(POS_V4, Some(1u64))]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_token_supply_api(
    #[case] upgrade: Upgrade,
    #[case] chain_id: Option<u64>,
) -> anyhow::Result<()> {
    use alloy::primitives::utils::parse_ether;
    use espresso_types::v0_3::ChainConfig;

    let epoch_height = 10;
    let network_config = TestConfigBuilder::default()
        .epoch_height(epoch_height)
        .build();

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    const NUM_NODES: usize = 1;
    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    // Use the real initial supply (3.59B tokens) so the unlock schedule
    // produces realistic locked/unlocked values in the supply calculations.
    let initial_supply_tokens = U256::from(3_590_000_000u64);
    let initial_supply_wei = parse_ether("3590000000").unwrap();

    let mut builder = TestNetworkConfigBuilder::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(api_port),
        ))
        .network_config(network_config.clone())
        .persistences(persistence.clone())
        .catchups(std::array::from_fn(|_| {
            StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![format!("http://localhost:{api_port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .initial_token_supply(initial_supply_tokens);

    // Must set states before pos_hook, which preserves chain_id from state[0].
    if let Some(id) = chain_id {
        let state = ValidatedState {
            chain_config: ChainConfig {
                chain_id: U256::from(id).into(),
                ..Default::default()
            }
            .into(),
            ..Default::default()
        };
        builder = builder.states(std::array::from_fn(|_| state.clone()));
    }

    let config = builder
        .pos_hook(
            DelegationConfig::VariableAmounts,
            Default::default(),
            upgrade,
        )
        .await
        .unwrap()
        .build();

    let _network = TestNetwork::new(config, upgrade).await;
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    let _blocks = client
        .socket("availability/stream/blocks/0")
        .subscribe::<BlockQueryData<SeqTypes>>()
        .await
        .unwrap()
        .take(3)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    let minted: String = client
        .get("token/total-minted-supply")
        .send()
        .await
        .expect("total-minted-supply");
    let circ_eth: String = client
        .get("token/circulating-supply-ethereum")
        .send()
        .await
        .expect("circulating-supply-ethereum");
    let circulating: String = client
        .get("token/circulating-supply")
        .send()
        .await
        .expect("circulating-supply");
    tracing::info!(%minted, %circ_eth, %circulating);

    let minted = parse_ether(&minted)?;
    let circ_eth = parse_ether(&circ_eth)?;
    let circ = parse_ether(&circulating)?;

    assert_eq!(minted, initial_supply_wei);
    assert!(circ_eth <= minted);
    assert!(circ >= circ_eth);
    assert!(circ > U256::ZERO);

    if chain_id == Some(1) {
        // Proves the unlock schedule is hooked up: locked > 0 means
        // the mainnet code path ran. Vesting ends ~2032; delete after.
        assert!(circ_eth < minted);
    }

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_scanning_token_contract_initialized_event() -> anyhow::Result<()> {
    use espresso_types::v0_3::ChainConfig;

    let blocks_per_epoch = 10;

    let network_config = TestConfigBuilder::<1>::default()
        .epoch_height(blocks_per_epoch)
        .build();

    let (genesis_state, genesis_stake) = light_client_genesis_from_stake_table(
        &network_config.hotshot_config().hotshot_stake_table(),
        STAKE_TABLE_CAPACITY_FOR_TEST,
    )
    .unwrap();

    let deployer = ProviderBuilder::new()
        .wallet(EthereumWallet::from(network_config.signer().clone()))
        .connect_http(network_config.l1_url().clone());

    let mut contracts = Contracts::new();
    let args = DeployerArgsBuilder::default()
        .deployer(deployer.clone())
        .rpc_url(network_config.l1_url().clone())
        .mock_light_client(true)
        .genesis_lc_state(genesis_state)
        .genesis_st_state(genesis_stake)
        .blocks_per_epoch(blocks_per_epoch)
        .epoch_start_block(1)
        .multisig_pauser(network_config.signer().address())
        .token_name("Espresso".to_string())
        .token_symbol("ESP".to_string())
        .initial_token_supply(U256::from(3590000000u64))
        .ops_timelock_delay(U256::from(0))
        .ops_timelock_admin(network_config.signer().address())
        .ops_timelock_proposers(vec![network_config.signer().address()])
        .ops_timelock_executors(vec![network_config.signer().address()])
        .safe_exit_timelock_delay(U256::from(0))
        .safe_exit_timelock_admin(network_config.signer().address())
        .safe_exit_timelock_proposers(vec![network_config.signer().address()])
        .safe_exit_timelock_executors(vec![network_config.signer().address()])
        .build()
        .unwrap();

    args.deploy_to_stake_table_v3(&mut contracts).await.unwrap();

    let st_addr = contracts
        .address(Contract::StakeTableProxy)
        .expect("StakeTableProxy deployed");

    let l1_url = network_config.l1_url().clone();

    let storage = SqlDataSource::create_storage().await;
    let mut opt = <SqlDataSource as TestableSequencerDataSource>::persistence_options(&storage);
    let persistence = opt.create().await.unwrap();

    let l1_client = L1ClientOptions {
        stake_table_update_interval: Duration::from_secs(7),
        l1_retry_delay: Duration::from_millis(10),
        l1_events_max_block_range: 10000,
        ..Default::default()
    }
    .connect(vec![l1_url])
    .unwrap();
    l1_client.spawn_tasks().await;

    let fetcher = Fetcher::new(
        Arc::new(NullStateCatchup::default()),
        Arc::new(Mutex::new(persistence.clone())),
        l1_client.clone(),
        ChainConfig {
            stake_table_contract: Some(st_addr),
            base_fee: 0.into(),
            ..Default::default()
        },
    );

    let provider = l1_client.provider;
    let stake_table = StakeTableV3::new(st_addr, provider.clone());

    let stake_table_init_block = stake_table
        .initializedAtBlock()
        .block(BlockId::finalized())
        .call()
        .await?
        .to::<u64>();

    tracing::info!("stake table init block = {stake_table_init_block}");

    let token_address = stake_table
        .token()
        .block(BlockId::finalized())
        .call()
        .await
        .context("Failed to get token address")?;

    let token = EspToken::new(token_address, provider.clone());

    let init_log = fetcher
        .scan_token_contract_initialized_event_log(stake_table_init_block, token.clone())
        .await
        .unwrap();

    let init_block = init_log.block_number.context("missing block number")?;
    let init_tx_hash = init_log
        .transaction_hash
        .context("missing transaction hash")?;

    let transfer_logs = token
        .Transfer_filter()
        .from_block(init_block)
        .to_block(init_block)
        .query()
        .await
        .unwrap();

    let (mint_transfer, _) = transfer_logs
        .iter()
        .find(|(transfer, log)| {
            log.transaction_hash == Some(init_tx_hash) && transfer.from == Address::ZERO
        })
        .context("no mint transfer event in init tx")?;

    assert!(mint_transfer.value > U256::ZERO);

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_integration_commission_updates() -> anyhow::Result<()> {
    const NUM_NODES: usize = 3;
    const EPOCH_HEIGHT: u64 = 10;

    // Use version that supports epochs (V3 or V4)
    let versions = POS_V4;

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    // Initialize storage for nodes
    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    // Configure test network with epochs
    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    // Build test network configuration starting with V1 stake table
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(api_port),
        ))
        .network_config(network_config.clone())
        .persistences(persistence.clone())
        .catchups(std::array::from_fn(|_| {
            StatePeers::<SequencerApiVersion>::from_urls(
                vec![format!("http://localhost:{api_port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .pos_hook(
            // We want no new rewards after setting the commission to zero.
            DelegationConfig::NoSelfDelegation,
            StakeTableContractVersion::V1, // upgraded later
            POS_V4,
        )
        .await
        .unwrap()
        .build();

    let network = TestNetwork::new(config, versions).await;
    let provider = network.cfg.anvil().unwrap();
    let deployer_addr = network.cfg.signer().address();
    let mut contracts = network.contracts.unwrap();
    let st_addr = contracts.address(Contract::StakeTableProxy).unwrap();
    upgrade_stake_table_v2(
        provider,
        L1Client::new(vec![network.cfg.l1_url()])?,
        &mut contracts,
        deployer_addr,
        deployer_addr,
    )
    .await?;

    let mut commissions = vec![];
    for (i, (validator, provider)) in network_config.validator_providers().into_iter().enumerate() {
        let commission = fetch_commission(provider.clone(), st_addr, validator).await?;
        let new_commission = match i {
            0 => 0u16,
            1 => commission.to_evm() + 500u16,
            2 => commission.to_evm() - 100u16,
            _ => unreachable!(),
        }
        .try_into()?;
        commissions.push((validator, commission, new_commission));
        tracing::info!(%validator, %commission, %new_commission, "Update commission");
        update_commission(provider, st_addr, new_commission)
            .await?
            .get_receipt()
            .await?;
    }

    // wait until new stake table takes effect
    let current_epoch = network.peers[0]
        .decided_leaf()
        .await
        .epoch(EPOCH_HEIGHT)
        .unwrap();
    let target_epoch = current_epoch.u64() + 3;
    println!("target epoch for new stake table: {target_epoch}");
    let mut events = network.peers[0].event_stream();
    wait_for_epochs(&mut events, EPOCH_HEIGHT, target_epoch).await;

    // the last epoch with the old commissions
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());
    let validators = client
        .get::<AuthenticatedValidatorMap>(&format!("node/validators/{}", target_epoch - 1))
        .send()
        .await
        .expect("validators");
    assert!(!validators.is_empty());
    for (val, old_comm, _) in commissions.clone() {
        assert_eq!(validators.get(&val).unwrap().commission, old_comm.to_evm());
    }

    // the first epoch with the new commissions
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());
    let validators = client
        .get::<AuthenticatedValidatorMap>(&format!("node/validators/{target_epoch}"))
        .send()
        .await
        .expect("validators");
    assert!(!validators.is_empty());
    for (val, _, new_comm) in commissions.clone() {
        assert_eq!(validators.get(&val).unwrap().commission, new_comm.to_evm());
    }

    let last_block_with_old_commissions = EPOCH_HEIGHT * (target_epoch - 1);
    let block_with_new_commissions = EPOCH_HEIGHT * target_epoch;
    let mut new_amounts = vec![];
    for (val, ..) in commissions {
        let before = client
            .get::<Option<RewardAmount>>(&format!(
                "reward-state-v2/reward-balance/{last_block_with_old_commissions}/{val}"
            ))
            .send()
            .await?
            .unwrap();
        let after = client
            .get::<Option<RewardAmount>>(&format!(
                "reward-state-v2/reward-balance/{block_with_new_commissions}/{val}"
            ))
            .send()
            .await?
            .unwrap();
        new_amounts.push(after - before);
    }

    let tolerance = U256::from(10 * EPOCH_HEIGHT).into();
    // validator zero got new new rewards except remainders
    assert!(new_amounts[0] < tolerance);

    // other validators are still receiving rewards
    assert!(new_amounts[1] + new_amounts[2] > tolerance);

    Ok(())
}

/// Start on StakeTable V2, upgrade to V3, call `updateNetworkConfig` on one
/// validator, and verify the indexer surfaces the new x25519 key and p2p
/// address in the validator map.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_integration_update_fast_finality_network_config() -> anyhow::Result<()> {
    const NUM_NODES: usize = 3;
    const EPOCH_HEIGHT: u64 = 10;

    let versions = POS_V4;
    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(api_port),
        ))
        .network_config(network_config.clone())
        .persistences(persistence.clone())
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
            StakeTableContractVersion::V2,
            POS_V4,
        )
        .await
        .unwrap()
        .build();

    let network = TestNetwork::new(config, versions).await;
    let provider = network.cfg.anvil().unwrap();
    let mut contracts = network.contracts.unwrap();
    let st_addr = contracts.address(Contract::StakeTableProxy).unwrap();

    upgrade_stake_table_v3(provider, &mut contracts).await?;

    let (validator, validator_provider) = network_config
        .validator_providers()
        .into_iter()
        .next()
        .unwrap();
    let x25519_key = x25519::Keypair::generate().unwrap().public_key();
    let p2p_addr: NetAddr = "127.0.0.1:9000".parse().unwrap();
    update_network_config(validator_provider, st_addr, x25519_key, p2p_addr.clone())
        .await?
        .get_receipt()
        .await?;

    let current_epoch = network.peers[0]
        .decided_leaf()
        .await
        .epoch(EPOCH_HEIGHT)
        .unwrap();
    let target_epoch = current_epoch.u64() + 3;
    let mut events = network.peers[0].event_stream();
    wait_for_epochs(&mut events, EPOCH_HEIGHT, target_epoch).await;

    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());
    let validators = client
        .get::<AuthenticatedValidatorMap>(&format!("node/validators/{target_epoch}"))
        .send()
        .await
        .expect("validators");
    let v = validators.get(&validator).expect("validator present");
    assert_eq!(v.x25519_key, Some(x25519_key));
    assert_eq!(v.p2p_addr, Some(p2p_addr));

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_all_validators_endpoint() -> anyhow::Result<()> {
    const EPOCH_HEIGHT: u64 = 20;

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

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
            Options::with_port(api_port),
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
            Default::default(),
            POS_V4,
        )
        .await
        .unwrap()
        .build();

    let network = TestNetwork::new(config, POS_V4).await;
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    let err = client
        .get::<Vec<RegisteredValidator<PubKey>>>("node/all-validators/1/0/1001")
        .header("Accept", "application/json")
        .send()
        .await
        .unwrap_err();

    assert_matches!(err, ServerError { status, message} if
            status == StatusCode::BAD_REQUEST
            && message.contains("Limit cannot be greater than 1000")
    );

    // Wait for the chain to progress beyond epoch 3
    let mut events = network.peers[0].event_stream();
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 3).await;

    // Verify that there are no validators for epoch # 1 and epoch # 2
    {
        client
            .get::<Vec<RegisteredValidator<PubKey>>>("node/all-validators/1/0/100")
            .send()
            .await
            .unwrap()
            .is_empty();

        client
            .get::<Vec<RegisteredValidator<PubKey>>>("node/all-validators/2/0/100")
            .send()
            .await
            .unwrap()
            .is_empty();
    }

    // Get the epoch # 3 validators
    let validators = client
        .get::<Vec<RegisteredValidator<PubKey>>>("node/all-validators/3/0/100")
        .send()
        .await
        .expect("validators");

    assert!(!validators.is_empty());

    Ok(())
}
