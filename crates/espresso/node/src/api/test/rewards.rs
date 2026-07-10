use super::*;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_pos_rewards_basic() -> anyhow::Result<()> {
    // Basic PoS rewards test:
    // - Sets up a single validator and a single delegator (the node itself).
    // - Sets the number of blocks in each epoch to 20.
    // - Rewards begin applying from block 41 (i.e., the start of the 3rd epoch).
    // - Since the validator is also the delegator, it receives the full reward.
    // - Verifies that the reward at block height 60 matches the expected amount.
    let epoch_height = 20;

    let network_config = TestConfigBuilder::default()
        .epoch_height(epoch_height)
        .build();

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    const NUM_NODES: usize = 1;
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
        .pos_hook(
            DelegationConfig::VariableAmounts,
            Default::default(),
            POS_V4,
        )
        .await
        .unwrap()
        .build();

    let network = TestNetwork::new(config, POS_V4).await;
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    // first two epochs will be 1 and 2
    // rewards are distributed starting third epoch
    // third epoch starts from block 40 as epoch height is 20
    // wait for atleast 65 blocks
    let _blocks = client
        .socket("availability/stream/blocks/0")
        .subscribe::<BlockQueryData<SeqTypes>>()
        .await
        .unwrap()
        .take(65)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    let staking_priv_keys = network_config.staking_priv_keys();
    let account = staking_priv_keys[0].signer.clone();
    let address = account.address();

    let block_height = 60;

    let node_state = network.server.node_state();
    let membership = node_state.coordinator.membership();
    let expected_amount = U256::from(20)
        * (membership
            .epoch_block_reward(3.into())
            .expect("block reward is not None"))
        .0;

    // get the validator address balance at block height 60
    let amount = client
        .get::<Option<RewardAmount>>(&format!(
            "reward-state/reward-balance/{block_height}/{address}"
        ))
        .send()
        .await
        .unwrap()
        .unwrap();

    tracing::info!("amount={amount:?}");

    assert_eq!(amount.0, expected_amount, "reward amount don't match");

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_cumulative_pos_rewards() -> anyhow::Result<()> {
    // This test registers 5 validators and multiple delegators for each validator.
    // One of the delegators is also a validator.
    // The test verifies that the cumulative reward at each block height equals
    // the total block reward, which is a constant.

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
    let node_state = network.server.node_state();
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    // wait for atleast 75 blocks
    let _blocks = client
        .socket("availability/stream/blocks/0")
        .subscribe::<BlockQueryData<SeqTypes>>()
        .await
        .unwrap()
        .take(75)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    // We are going to check cumulative blocks from block height 40 to 67
    // Basically epoch 3 and epoch 4 as epoch height is 20
    // get all the validators
    let validators = client
        .get::<AuthenticatedValidatorMap>("node/validators/3")
        .send()
        .await
        .expect("failed to get validator");

    // insert all the address in a map
    // We will query the reward-balance at each block height for all the addresses
    // We don't know which validator was the leader because we don't have access to Membership
    let mut addresses = HashSet::new();
    for v in validators.values() {
        addresses.insert(v.account);
        addresses.extend(v.clone().delegators.keys().collect::<Vec<_>>());
    }
    // get all the validators
    let validators = client
        .get::<AuthenticatedValidatorMap>("node/validators/4")
        .send()
        .await
        .expect("failed to get validator");
    for v in validators.values() {
        addresses.insert(v.account);
        addresses.extend(v.clone().delegators.keys().collect::<Vec<_>>());
    }

    let mut prev_cumulative_amount = U256::ZERO;
    // Check Cumulative rewards for epochs 3 (= block height 41 to 59) & 4 (= block height 60 to 67)
    for block in 41..=67 {
        let membership = node_state.coordinator.membership();
        let block_reward = membership
            .epoch_block_reward(epoch_from_block_number(block, epoch_height).into())
            .expect("block reward is not None");

        let mut cumulative_amount = U256::ZERO;
        for address in addresses.clone() {
            let amount = client
                .get::<Option<RewardAmount>>(&format!(
                    "reward-state/reward-balance/{block}/{address}"
                ))
                .send()
                .await
                .ok()
                .flatten();

            if let Some(amount) = amount {
                tracing::info!("address={address}, amount={amount}");
                cumulative_amount += amount.0;
            };
        }

        // assert cumulative reward is equal to block reward
        assert_eq!(cumulative_amount - prev_cumulative_amount, block_reward.0);
        tracing::info!("cumulative_amount is correct for block={block}");
        prev_cumulative_amount = cumulative_amount;
    }

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_rewards_v4() -> anyhow::Result<()> {
    // This test verifies PoS reward distribution logic for multiple delegators per validator.
    //
    //  assertions:
    // - No rewards are distributed during the first 2 epochs.
    // - Rewards begin from epoch 3 onward.
    // - Delegator stake sums match the corresponding validator stake.
    // - Reward values match those returned by the reward state API.
    // - Commission calculations are within a small acceptable rounding tolerance.
    // - Ensure that the `total_reward_distributed` field in the block header matches the total block reward distributed
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

    // Wait for the chain to progress beyond epoch 3 so rewards start being distributed.
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

    // Verify that there are no validators for epoch # 1 and epoch # 2
    {
        client
            .get::<AuthenticatedValidatorMap>("node/validators/1")
            .send()
            .await
            .unwrap()
            .is_empty();

        client
            .get::<AuthenticatedValidatorMap>("node/validators/2")
            .send()
            .await
            .unwrap()
            .is_empty();
    }

    // Get the epoch # 3 validators
    let validators = client
        .get::<AuthenticatedValidatorMap>("node/validators/3")
        .send()
        .await
        .expect("validators");

    assert!(!validators.is_empty());

    // Collect addresses to track rewards for all participants.
    let mut addresses = HashSet::new();
    for v in validators.values() {
        addresses.insert(v.account);
        addresses.extend(v.clone().delegators.keys().collect::<Vec<_>>());
    }

    let mut leaves = client
        .socket("availability/stream/leaves/0")
        .subscribe::<LeafQueryData<SeqTypes>>()
        .await
        .unwrap();

    let node_state = network.server.node_state();
    let coordinator = node_state.coordinator;

    let membership = coordinator.membership();

    // Ensure rewards remain zero up for the first two epochs
    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.unwrap();
        let header = leaf.header();
        assert_eq!(header.total_reward_distributed().unwrap().0, U256::ZERO);

        let epoch_number = EpochNumber::new(epoch_from_block_number(leaf.height(), EPOCH_HEIGHT));

        assert!(membership.epoch_block_reward(epoch_number).is_none());

        let height = header.height();
        for address in addresses.clone() {
            let amount = client
                .get::<Option<RewardAmount>>(&format!(
                    "reward-state-v2/reward-balance/{height}/{address}"
                ))
                .send()
                .await
                .ok()
                .flatten();
            assert!(amount.is_none(), "amount is not none for block {height}")
        }

        if leaf.height() == EPOCH_HEIGHT * 2 {
            break;
        }
    }

    let mut rewards_map = HashMap::new();
    let mut total_distributed = U256::ZERO;
    let mut epoch_rewards = HashMap::<EpochNumber, U256>::new();

    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.unwrap();

        let header = leaf.header();
        let distributed = header
            .total_reward_distributed()
            .expect("rewards distributed is none");

        let block = leaf.height();
        tracing::info!("verify rewards for block={block:?}");
        let membership = coordinator.membership();
        let epoch_number = EpochNumber::new(epoch_from_block_number(leaf.height(), EPOCH_HEIGHT));

        let snapshot = membership.snapshot(epoch_number).expect("snapshot");
        let block_reward = snapshot.epoch_block_reward().unwrap();
        let leader = snapshot.leader(leaf.leaf().view_number()).expect("leader");
        let leader_eth_address = snapshot
            .validator_config(&leader)
            .expect("validator config")
            .account;

        let validators = client
            .get::<AuthenticatedValidatorMap>(&format!("node/validators/{epoch_number}"))
            .send()
            .await
            .expect("validators");

        let leader_validator = validators
            .get(&leader_eth_address)
            .expect("leader not found");

        let distributor =
            RewardDistributor::new(leader_validator.clone(), block_reward, distributed);
        // Verify that the sum of delegator stakes equals the validator's total stake.
        for validator in validators.values() {
            let delegator_stake_sum: U256 = validator.delegators.values().cloned().sum();

            assert_eq!(delegator_stake_sum, validator.stake);
        }

        let computed_rewards = distributor.compute_rewards().expect("reward computation");

        // Validate that the leader's commission is within a 10 wei tolerance of the expected value.
        let total_reward = block_reward.0;
        let leader_commission_basis_points = U256::from(leader_validator.commission);
        let calculated_leader_commission_reward = leader_commission_basis_points
            .checked_mul(total_reward)
            .context("overflow")?
            .checked_div(U256::from(COMMISSION_BASIS_POINTS))
            .context("overflow")?;

        assert!(
            computed_rewards.leader_commission().0 - calculated_leader_commission_reward
                <= U256::from(10_u64)
        );

        // Aggregate rewards by address (both delegator and leader).
        let leader_commission = *computed_rewards.leader_commission();
        for (address, amount) in computed_rewards.delegators().clone() {
            rewards_map
                .entry(address)
                .and_modify(|entry| *entry += amount)
                .or_insert(amount);
        }

        // add leader commission reward
        rewards_map
            .entry(leader_eth_address)
            .and_modify(|entry| *entry += leader_commission)
            .or_insert(leader_commission);

        // assert that the reward matches to what is in the reward merkle tree
        for (address, calculated_amount) in rewards_map.iter() {
            let mut attempt = 0;
            let amount_from_api = loop {
                let result = client
                    .get::<Option<RewardAmount>>(&format!(
                        "reward-state-v2/reward-balance/{block}/{address}"
                    ))
                    .send()
                    .await
                    .ok()
                    .flatten();

                if let Some(amount) = result {
                    break amount;
                }

                attempt += 1;
                if attempt >= 3 {
                    panic!("Failed to fetch reward amount for address {address} after 3 retries");
                }

                sleep(Duration::from_secs(2)).await;
            };

            assert_eq!(amount_from_api, *calculated_amount);
        }

        // Confirm the header's total distributed field matches the cumulative expected amount.
        total_distributed += block_reward.0;
        assert_eq!(
            header.total_reward_distributed().unwrap().0,
            total_distributed
        );

        // Block reward shouldn't change for the same epoch
        epoch_rewards
            .entry(epoch_number)
            .and_modify(|r| assert_eq!(*r, block_reward.0))
            .or_insert(block_reward.0);

        // Stop the test after verifying 5 full epochs.
        if leaf.height() == EPOCH_HEIGHT * 5 {
            break;
        }
    }

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_epoch_reward_distribution_basic() -> anyhow::Result<()> {
    const EPOCH_HEIGHT: u64 = 10;
    const NUM_NODES: usize = 5;

    const V5: Upgrade = Upgrade::trivial(EPOCH_REWARD_VERSION);

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    let api_port = reserve_tcp_port().expect("No ports free for query service");

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
        .pos_hook(DelegationConfig::MultipleDelegators, Default::default(), V5)
        .await
        .unwrap()
        .build();

    let _network = TestNetwork::new(config, V5).await;
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    // Wait for chain to reach epoch 5
    let height_client: Client<ServerError, StaticVersion<0, 1>> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());
    wait_until_block_height(&height_client, "node/block-height", EPOCH_HEIGHT * 5).await;

    let mut leaves = client
        .socket("availability/stream/leaves/0")
        .subscribe::<LeafQueryData<SeqTypes>>()
        .await
        .unwrap();

    // Epochs 1-3: verify no rewards
    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.unwrap();
        let header = leaf.header();
        let height = header.height();

        let total_distributed = header.total_reward_distributed().unwrap();
        assert_eq!(
            total_distributed.0,
            U256::ZERO,
            "epochs 1-3 should have no rewards, height={height}"
        );

        if height == EPOCH_HEIGHT * 3 {
            break;
        }
    }

    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.unwrap();
        let header = leaf.header();
        let height = header.height();

        if height == EPOCH_HEIGHT * 4 {
            let total_distributed = header.total_reward_distributed().unwrap();
            assert!(total_distributed.0 > U256::ZERO,);
            break;
        }
    }

    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.unwrap();
        let header = leaf.header();
        let height = header.height();

        if height == EPOCH_HEIGHT * 5 {
            let total_distributed = header.total_reward_distributed().unwrap();
            assert!(total_distributed.0 > U256::ZERO,);
            break;
        }
    }

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_epoch_reward_total_distributed_rewards() -> anyhow::Result<()> {
    // Epochs 1-3: No rewards distributed (total_reward_distributed = 0)
    // Epoch 4: Rewards only distributed in the LAST block
    // Epoch 5: All blocks before last have same total as epoch 4 last block,
    //          last block has higher total because of new distribution
    const EPOCH_HEIGHT: u64 = 10;
    const NUM_NODES: usize = 5;

    const V5: Upgrade = Upgrade::trivial(EPOCH_REWARD_VERSION);

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    let api_port = reserve_tcp_port().expect("No ports free for query service");

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
        .pos_hook(DelegationConfig::MultipleDelegators, Default::default(), V5)
        .await
        .unwrap()
        .build();

    let _network = TestNetwork::new(config, V5).await;
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    let height_client: Client<ServerError, StaticVersion<0, 1>> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());
    wait_until_block_height(&height_client, "node/block-height", EPOCH_HEIGHT * 5).await;

    let mut leaves = client
        .socket("availability/stream/leaves/0")
        .subscribe::<LeafQueryData<SeqTypes>>()
        .await
        .unwrap();

    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.unwrap();
        let header = leaf.header();
        let height = header.height();

        let total_distributed = header.total_reward_distributed().unwrap();
        assert_eq!(total_distributed.0, U256::ZERO,);

        if height == EPOCH_HEIGHT * 3 {
            break;
        }
    }

    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.unwrap();
        let header = leaf.header();
        let height = header.height();

        let total_distributed = header.total_reward_distributed().unwrap();

        if height < EPOCH_HEIGHT * 4 {
            assert_eq!(total_distributed.0, U256::ZERO,);
        } else {
            assert!(total_distributed.0 > U256::ZERO,);
            break;
        }
    }

    let epoch4_last_reward = {
        let header = client
            .get::<Header>(&format!("availability/header/{}", EPOCH_HEIGHT * 4))
            .send()
            .await
            .unwrap();
        header.total_reward_distributed().unwrap()
    };

    assert!(
        epoch4_last_reward.0 > U256::ZERO,
        "epoch 4 last block should have positive rewards"
    );

    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.unwrap();
        let header = leaf.header();
        let height = header.height();

        let total_distributed = header.total_reward_distributed().unwrap();

        if height < EPOCH_HEIGHT * 5 {
            assert_eq!(total_distributed, epoch4_last_reward,);
        } else {
            assert!(total_distributed.0 > epoch4_last_reward.0,);
            break;
        }
    }

    Ok(())
}

// test actual rewards
// todo: test each account rewards by querying merklized state api
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_reward_state_v2_epoch_distribution() -> anyhow::Result<()> {
    const EPOCH_HEIGHT: u64 = 10;
    const NUM_NODES: usize = 5;
    const NUM_EPOCHS: u64 = 6;
    const V5: Upgrade = Upgrade::trivial(EPOCH_REWARD_VERSION);

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    let api_port = reserve_tcp_port().expect("No ports free for query service");

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
        .pos_hook(DelegationConfig::MultipleDelegators, Default::default(), V5)
        .await
        .unwrap()
        .build();

    let network = TestNetwork::new(config, V5).await;
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    let node_state = network.server.node_state();
    let coordinator = node_state.coordinator;

    let mut expected_total_distributed = U256::ZERO;

    let mut leaves = client
        .socket("availability/stream/leaves/0")
        .subscribe::<LeafQueryData<SeqTypes>>()
        .await
        .unwrap();

    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.unwrap();
        let header = leaf.header();
        let height = header.height();

        let epoch = epoch_from_block_number(height, EPOCH_HEIGHT);

        let is_epoch_last_block = height % EPOCH_HEIGHT == 0;

        if epoch <= 3 {
            continue;
        }

        let header_total_distributed = header
            .total_reward_distributed()
            .expect("total_reward_distributed should exist");

        if is_epoch_last_block {
            let prev_epoch = epoch - 1;
            let prev_epoch_number = EpochNumber::new(prev_epoch);
            let membership = coordinator.membership();
            let prev_block_reward = membership
                .epoch_block_reward(prev_epoch_number)
                .expect("epoch block reward should exist");

            let epoch_total = prev_block_reward.0 * U256::from(EPOCH_HEIGHT);
            expected_total_distributed += epoch_total;
        }

        assert_eq!(
            header_total_distributed.0, expected_total_distributed,
            "total_reward_distributed mismatch at height {height}"
        );

        if height >= NUM_EPOCHS * EPOCH_HEIGHT {
            break;
        }
    }

    Ok(())
}

/// Verifies that the `leader_counts` array in V5+ headers is correct.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_epoch_leader_counts() -> anyhow::Result<()> {
    const EPOCH_HEIGHT: u64 = 10;
    const NUM_NODES: usize = 5;
    const NUM_EPOCHS: u64 = 6;
    const V5: Upgrade = Upgrade::trivial(EPOCH_REWARD_VERSION);

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    let api_port = reserve_tcp_port().expect("No ports free for query service");

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
        .pos_hook(DelegationConfig::MultipleDelegators, Default::default(), V5)
        .await
        .unwrap()
        .build();

    let network = TestNetwork::new(config, V5).await;
    let client: Client<ServerError, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    let node_state = network.server.node_state();
    let coordinator = node_state.coordinator;

    // Track expected leader counts by address
    let mut expected_counts: HashMap<Address, u16> = HashMap::new();

    let mut leaves = client
        .socket("availability/stream/leaves/0")
        .subscribe::<LeafQueryData<SeqTypes>>()
        .await
        .unwrap();

    while let Some(leaf) = leaves.next().await {
        let leaf = leaf.unwrap();
        let header = leaf.header();
        let height = header.height();
        let epoch = epoch_from_block_number(height, EPOCH_HEIGHT);
        let epoch_number = EpochNumber::new(epoch);

        if epoch <= 2 {
            continue;
        }

        let header_leader_counts = header
            .leader_counts()
            .expect("V5+ header must have leader_counts");

        // Reset counts at the start of a new epoch
        let is_epoch_start = (height - 1) % EPOCH_HEIGHT == 0;
        if is_epoch_start {
            expected_counts.clear();
        }

        // Determine the leader for this block and track by address.
        let view_number = leaf.leaf().view_number();
        let snapshot = coordinator
            .membership()
            .snapshot(epoch_number)
            .expect("committee for epoch_number");
        let leader = snapshot.leader(view_number).expect("leader should exist");
        let leader_address = snapshot
            .validator_config(&leader)
            .expect("leader should have an address")
            .account;

        let validator_leader_counts = ValidatorLeaderCounts::new(&snapshot, *header_leader_counts)
            .expect("ValidatorLeaderCounts should build from header leader_counts");

        *expected_counts.entry(leader_address).or_insert(0) += 1;

        let header_counts: HashMap<Address, u16> = validator_leader_counts
            .active_leaders()
            .map(|(v, count)| (v.account, count))
            .collect();

        assert_eq!(
            header_counts, expected_counts,
            "leader_counts mismatch at height {height} (epoch {epoch})"
        );

        if height % EPOCH_HEIGHT == 0 {
            let total: u16 = expected_counts.values().sum();
            assert_eq!(
                total, EPOCH_HEIGHT as u16,
                "total leader_counts at epoch boundary should equal EPOCH_HEIGHT at height \
                 {height}"
            );
        }

        if height >= NUM_EPOCHS * EPOCH_HEIGHT {
            break;
        }
    }

    Ok(())
}

#[rstest]
#[case(POS_V3)]
#[case(POS_V4)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_block_reward_api(#[case] upgrade: Upgrade) -> anyhow::Result<()> {
    let epoch_height = 10;

    let network_config = TestConfigBuilder::default()
        .epoch_height(epoch_height)
        .build();

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    const NUM_NODES: usize = 1;
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

    let block_reward = client
        .get::<Option<RewardAmount>>("node/block-reward")
        .send()
        .await
        .expect("failed to get block reward")
        .expect("block reward is None");
    tracing::info!("block_reward={block_reward:?}");

    assert!(block_reward.0 > U256::ZERO);

    Ok(())
}

#[rstest]
#[case(POS_V3)]
#[case(POS_V4)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_v3_and_v4_reward_tree_updates(#[case] upgrade: Upgrade) -> anyhow::Result<()> {
    // This test checks that the correct merkle tree is updated based on version
    //
    // When the protocol version is v3:
    // - The v3 Merkle tree is updated
    // - The v4 Merkle tree must be empty.
    //
    // When the protocol version is v4:
    // - The v4 Merkle tree is updated
    // - The v3 Merkle tree must be empty.
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
    let mut network = TestNetwork::new(config, upgrade).await;

    let mut events = network.peers[2].event_stream();
    // wait for 4 epochs
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 4).await;

    let validated_state = network.server.decided_state().await.unwrap();
    if upgrade.base == EPOCH_VERSION {
        let v1_tree = &validated_state.reward_merkle_tree_v1;
        assert!(v1_tree.num_leaves() > 0, "v1 reward tree tree is empty");
        let v2_tree = &validated_state.reward_merkle_tree_v2;
        assert!(
            v2_tree.num_leaves() == 0,
            "v2 reward tree tree is not empty"
        );
    } else {
        let v1_tree = &validated_state.reward_merkle_tree_v1;
        assert!(
            v1_tree.num_leaves() == 0,
            "v1 reward tree tree is not empty"
        );
        let v2_tree = &validated_state.reward_merkle_tree_v2;
        assert!(v2_tree.num_leaves() > 0, "v2 reward tree tree is empty");
    }

    network.stop_consensus().await;
    Ok(())
}

async fn compare_endpoints(
    http: &reqwest::Client,
    api_port: u16,
    axum_port: u16,
    path: &str,
) -> anyhow::Result<()> {
    let tide: serde_json::Value = http
        .get(format!("http://localhost:{api_port}/v1/{path}"))
        .send()
        .await?
        .json()
        .await?;
    let axum: serde_json::Value = http
        .get(format!("http://localhost:{axum_port}/v1/{path}"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(tide, axum, "v1/{path}: tide and axum v1 responses differ");
    Ok(())
}

/// Assert both tide-disco and the Axum endpoint return the same HTTP error status code.
async fn compare_error_endpoints(
    http: &reqwest::Client,
    api_port: u16,
    axum_port: u16,
    path: &str,
    expected_status: u16,
) -> anyhow::Result<()> {
    let tide_status = http
        .get(format!("http://localhost:{api_port}/v1/{path}"))
        .send()
        .await?
        .status()
        .as_u16();
    let axum_status = http
        .get(format!("http://localhost:{axum_port}/v1/{path}"))
        .send()
        .await?
        .status()
        .as_u16();
    assert_eq!(
        tide_status, expected_status,
        "v1/{path}: tide should return {expected_status}, got {tide_status}"
    );
    assert_eq!(
        axum_status, expected_status,
        "v1/{path}: axum should return {expected_status}, got {axum_status}"
    );
    Ok(())
}

/// Connect to both tide-disco and axum WebSocket endpoints, collect up to 10 messages each,
/// and assert that at least 2 messages appear in both streams.
async fn compare_ws_endpoints(api_port: u16, axum_port: u16, path: &str) -> anyhow::Result<()> {
    use std::{collections::HashSet, time::Duration};

    use futures::StreamExt as _;
    use tokio::time::timeout;
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    async fn collect_messages(port: u16, path: &str) -> anyhow::Result<Vec<serde_json::Value>> {
        let url = format!("ws://localhost:{port}/v1/{path}");
        let (mut ws, _) = connect_async(&url).await?;
        let mut messages = Vec::new();
        while messages.len() < 10 {
            match timeout(Duration::from_millis(500), ws.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        messages.push(v);
                    }
                },
                _ => break,
            }
        }
        Ok(messages)
    }

    let (tide_msgs, axum_msgs) = tokio::join!(
        collect_messages(api_port, path),
        collect_messages(axum_port, path)
    );
    let tide_msgs = tide_msgs?;
    let axum_msgs = axum_msgs?;

    let tide_set: HashSet<String> = tide_msgs.iter().map(|v| v.to_string()).collect();
    let common = axum_msgs
        .iter()
        .filter(|v| tide_set.contains(&v.to_string()))
        .count();

    assert!(
        common >= 2,
        "v1/{path}: expected ≥2 messages in common between tide ({} msgs) and axum ({} msgs), got \
         {common}",
        tide_msgs.len(),
        axum_msgs.len(),
    );
    Ok(())
}

#[rstest]
#[case(POS_V4)]
#[test_log::test]
fn test_reward_proof_endpoint(#[case] upgrade: Upgrade) {
    let test = async move {
        const EPOCH_HEIGHT: u64 = 10;
        const NUM_NODES: usize = 5;

        let network_config = TestConfigBuilder::default()
            .epoch_height(EPOCH_HEIGHT)
            .build();

        let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");
        let axum_port = reserve_tcp_port().expect("OS should have ephemeral ports available");
        println!("API PORT = {api_port}");
        println!("AXUM PORT = {axum_port}");

        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let mut api_opts = Options::with_port(api_port).catchup(Default::default());
        api_opts.http.axum_port = Some(axum_port);

        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(SqlDataSource::options(&storage[0], api_opts))
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
            .pos_hook(
                DelegationConfig::MultipleDelegators,
                hotshot_contract_adapter::stake_table::StakeTableContractVersion::V3,
                upgrade,
            )
            .await
            .unwrap()
            .build();

        let mut network = TestNetwork::new(config, upgrade).await;

        // wait for 4 epochs
        let mut events = network.server.event_stream();
        wait_for_epochs(&mut events, EPOCH_HEIGHT, 4).await;

        let url = format!("http://localhost:{api_port}").parse().unwrap();
        let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

        let validated_state = network.server.decided_state().await.unwrap();
        let decided_leaf = network.server.decided_leaf().await;
        let height = decided_leaf.height();

        // validate proof returned from the api
        if upgrade.base == EPOCH_VERSION {
            // V1 case — axum only implements the v2 reward tree, so no axum comparison here
            wait_until_block_height(&client, "reward-state/block-height", height).await;

            network.stop_consensus().await;

            for (address, _) in validated_state.reward_merkle_tree_v1.iter() {
                let (_, expected_proof) = validated_state
                    .reward_merkle_tree_v1
                    .lookup(*address)
                    .expect_ok()
                    .unwrap();

                let res = client
                    .get::<RewardAccountQueryDataV1>(&format!(
                        "reward-state/proof/{height}/{address}"
                    ))
                    .send()
                    .await
                    .unwrap();

                match res.proof.proof {
                    RewardMerkleProofV1::Presence(p) => {
                        assert_eq!(
                            p, expected_proof,
                            "Proof mismatch for V1 at {height}, addr={address}"
                        );
                    },
                    other => panic!(
                        "Expected Present proof for V1 at {height}, addr={address}, got {other:?}"
                    ),
                }
            }
        } else {
            // V2 case

            // Submit two transactions to the same namespace in separate blocks
            // so the namespace-filtered WS stream produces ≥2 messages.
            // Submitting both at once risks the builder batching them into a
            // single block; submitting sequentially (wait between) guarantees
            // different blocks so the second wait_for_decide_on_handle doesn't
            // hang looking for an event that was already consumed by the first.
            let avail_ns = NamespaceId::from(42_u32);
            let avail_tx = Transaction::new(avail_ns, vec![1, 2, 3]);
            network
                .server
                .submit_transaction(avail_tx.clone())
                .await
                .unwrap();
            let (avail_block, _) = wait_for_decide_on_handle(&mut events, &avail_tx).await;

            // Submit the second transaction only after the first is decided,
            // ensuring it lands in a strictly later block.
            let avail_tx2 = Transaction::new(avail_ns, vec![4, 5, 6]);
            network
                .server
                .submit_transaction(avail_tx2.clone())
                .await
                .unwrap();
            wait_for_decide_on_handle(&mut events, &avail_tx2).await;

            wait_until_block_height(&client, "reward-state-v2/block-height", height).await;
            // Wait for the availability query service to index avail_block.
            wait_until_block_height(&client, "node/block-height", avail_block).await;

            network.stop_consensus().await;

            let http = reqwest::Client::new();

            for (address, _) in validated_state.reward_merkle_tree_v2.iter() {
                let (_, expected_proof) = validated_state
                    .reward_merkle_tree_v2
                    .lookup(*address)
                    .expect_ok()
                    .unwrap();

                let res = client
                    .get::<RewardAccountQueryDataV2>(&format!(
                        "reward-state-v2/proof/{height}/{address}"
                    ))
                    .send()
                    .await
                    .unwrap();

                match res.proof.proof.clone() {
                    RewardMerkleProofV2::Presence(p) => {
                        assert_eq!(
                            p, expected_proof,
                            "Proof mismatch for V2 at {height}, addr={address}"
                        );
                    },
                    other => panic!(
                        "Expected Present proof for V2 at {height}, addr={address}, got {other:?}"
                    ),
                }

                let reward_claim_input = client
                    .get::<RewardClaimInput>(&format!(
                        "reward-state-v2/reward-claim-input/{height}/{address}"
                    ))
                    .send()
                    .await
                    .unwrap();

                assert_eq!(reward_claim_input, res.to_reward_claim_input()?);

                // Both servers share the same underlying SQL data source; compare responses
                // for each per-address endpoint under reward-state-v2.
                compare_endpoints(
                    &http,
                    api_port,
                    axum_port,
                    &format!("reward-state-v2/proof/{height}/{address}"),
                )
                .await?;
                compare_endpoints(
                    &http,
                    api_port,
                    axum_port,
                    &format!("reward-state-v2/reward-claim-input/{height}/{address}"),
                )
                .await?;
                compare_endpoints(
                    &http,
                    api_port,
                    axum_port,
                    &format!("reward-state-v2/reward-balance/{height}/{address}"),
                )
                .await?;
                compare_endpoints(
                    &http,
                    api_port,
                    axum_port,
                    &format!("reward-state-v2/proof/latest/{address}"),
                )
                .await?;
                compare_endpoints(
                    &http,
                    api_port,
                    axum_port,
                    &format!("reward-state-v2/reward-balance/latest/{address}"),
                )
                .await?;
            }

            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("reward-state-v2/reward-amounts/{height}/0/1000"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("reward-state-v2/reward-merkle-tree-v2/{height}"),
            )
            .await?;

            // Availability v1 parity: verify the axum v1 routes return the same JSON as tide.

            // Namespace proof by height
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/block/{avail_block}/namespace/{avail_ns}"),
            )
            .await?;

            // Namespace proof by block hash and payload hash
            let avail_header: Header = client
                .get(&format!("availability/header/{avail_block}"))
                .send()
                .await
                .unwrap();
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!(
                    "availability/block/hash/{}/namespace/{avail_ns}",
                    avail_header.commit()
                ),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!(
                    "availability/block/payload-hash/{}/namespace/{avail_ns}",
                    avail_header.payload_commitment()
                ),
            )
            .await?;

            // Namespace proof range
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!(
                    "availability/block/{avail_block}/{}/namespace/{avail_ns}",
                    avail_block + 1
                ),
            )
            .await?;

            // State certificate parity (epoch 1 is complete after 4 epochs)
            compare_endpoints(&http, api_port, axum_port, "availability/state-cert/1").await?;
            compare_endpoints(&http, api_port, axum_port, "availability/state-cert-v2/1").await?;

            // HotShot availability parity: leaf, header, block, payload, vid/common, etc.
            let avail_leaf: LeafQueryData<SeqTypes> = client
                .get(&format!("availability/leaf/{avail_block}"))
                .send()
                .await
                .unwrap();
            let leaf_hash = avail_leaf.hash();
            let block_hash = avail_header.commit();
            let payload_hash = avail_header.payload_commitment();

            // Leaf endpoints
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/leaf/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/leaf/hash/{leaf_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/leaf/{avail_block}/{}", avail_block + 1),
            )
            .await?;

            // Header endpoints
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/header/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/header/hash/{block_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/header/payload-hash/{payload_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/header/{avail_block}/{}", avail_block + 1),
            )
            .await?;

            // Block endpoints
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/block/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/block/hash/{block_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/block/payload-hash/{payload_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/block/{avail_block}/{}", avail_block + 1),
            )
            .await?;

            // Payload endpoints
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/payload/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/payload/hash/{payload_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/payload/block-hash/{block_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/payload/{avail_block}/{}", avail_block + 1),
            )
            .await?;

            // VID common endpoints
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/vid/common/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/vid/common/hash/{block_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/vid/common/payload-hash/{payload_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/vid/common/{avail_block}/{}", avail_block + 1),
            )
            .await?;

            // Transaction endpoints
            let tx_hash = avail_tx.commit();
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/transaction/{avail_block}/0/noproof"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/transaction/hash/{tx_hash}/noproof"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/transaction/{avail_block}/0/proof"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/transaction/hash/{tx_hash}/proof"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/transaction/{avail_block}/0"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/transaction/hash/{tx_hash}"),
            )
            .await?;

            // Block summary endpoints
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/block/summary/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!(
                    "availability/block/summaries/{avail_block}/{}",
                    avail_block + 1
                ),
            )
            .await?;

            // Limits endpoint (static response)
            compare_endpoints(&http, api_port, axum_port, "availability/limits").await?;

            // Cert2 endpoint (returns null when no cert is available at this height)
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/cert2/{avail_block}"),
            )
            .await?;

            // WebSocket streaming parity: both servers share the same data source, so their
            // streams must produce the same items. We collect up to 10 messages from each and
            // verify ≥2 appear in both.
            //
            // For unfiltered streams, start 10 blocks before avail_block so there are at
            // least 10 committed blocks ready to stream (consensus has already stopped).
            // For namespace-filtered streams, start at avail_block where the two submitted
            // transactions were included, giving ≥2 matching messages.
            let ws_start = avail_block.saturating_sub(10);
            compare_ws_endpoints(
                api_port,
                axum_port,
                &format!("availability/stream/leaves/{ws_start}"),
            )
            .await?;
            compare_ws_endpoints(
                api_port,
                axum_port,
                &format!("availability/stream/headers/{ws_start}"),
            )
            .await?;
            compare_ws_endpoints(
                api_port,
                axum_port,
                &format!("availability/stream/blocks/{ws_start}"),
            )
            .await?;
            compare_ws_endpoints(
                api_port,
                axum_port,
                &format!("availability/stream/payloads/{ws_start}"),
            )
            .await?;
            compare_ws_endpoints(
                api_port,
                axum_port,
                &format!("availability/stream/vid/common/{ws_start}"),
            )
            .await?;
            compare_ws_endpoints(
                api_port,
                axum_port,
                &format!("availability/stream/transactions/{ws_start}"),
            )
            .await?;
            // Namespace-filtered streams: start at avail_block; two transactions were
            // submitted so the stream produces ≥2 messages.
            compare_ws_endpoints(
                api_port,
                axum_port,
                &format!("availability/stream/transactions/{avail_block}/namespace/{avail_ns}"),
            )
            .await?;
            compare_ws_endpoints(
                api_port,
                axum_port,
                &format!("availability/stream/blocks/{avail_block}/namespace/{avail_ns}"),
            )
            .await?;

            // Merklized state parity (block-state and fee-state). Wait for
            // both backends to have indexed the snapshot we'll query.
            wait_until_block_height(&client, "block-state/block-height", avail_block).await;
            wait_until_block_height(&client, "fee-state/block-height", avail_block).await;

            // block-state/block-height and fee-state/block-height (latest
            // height for which merklized state is available).
            compare_endpoints(&http, api_port, axum_port, "block-state/block-height").await?;
            compare_endpoints(&http, api_port, axum_port, "fee-state/block-height").await?;

            // block-state path by height: the merkle tree at height H
            // contains the headers of blocks [0, H), so a valid key is H-1.
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!(
                    "block-state/{avail_block}/{}",
                    avail_block.saturating_sub(1)
                ),
            )
            .await?;

            // block-state path by commit. Use the tree commitment from
            // the header at avail_block.
            let block_mt_commit = avail_header.block_merkle_tree_root().to_string();
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!(
                    "block-state/commit/{block_mt_commit}/{}",
                    avail_block.saturating_sub(1)
                ),
            )
            .await?;

            // fee-state path by height for a known fee account, and
            // fee-balance/latest for the same account.
            let fee_account = validated_state
                .fee_merkle_tree
                .iter()
                .next()
                .map(|(addr, _)| *addr)
                .expect("fee tree should have at least one account");
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("fee-state/{avail_block}/{fee_account}"),
            )
            .await?;
            let fee_mt_commit = avail_header.fee_merkle_tree_root().to_string();
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("fee-state/commit/{fee_mt_commit}/{fee_account}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("fee-state/fee-balance/latest/{fee_account}"),
            )
            .await?;

            // Error equivalence: both tide-disco and Axum must return the same
            // HTTP status codes for common failure cases that clients encounter.

            // Requesting a leaf far ahead of the chain tip times out and returns
            // 404 Not Found from both servers.
            compare_error_endpoints(&http, api_port, axum_port, "availability/leaf/999999", 404)
                .await?;

            // Requesting a block range that exceeds the per-request limit
            // returns 400 Bad Request from both servers.
            compare_error_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("availability/block/{avail_block}/{}", avail_block + 200),
                400,
            )
            .await?;

            // Requesting a namespace proof range that exceeds the limit also
            // returns 400 Bad Request from both servers.
            compare_error_endpoints(
                &http,
                api_port,
                axum_port,
                &format!(
                    "availability/block/{avail_block}/{}/namespace/{avail_ns}",
                    avail_block + 200
                ),
                400,
            )
            .await?;
        }

        anyhow::Ok(())
    };

    // `block_on` polls the future on the *calling* thread. The default test thread stack is
    // 2 MiB on Linux, which isn't enough for this test's large async state machine. We spawn
    // a fresh thread with 32 MiB and run the tokio runtime there instead.
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(test)
                .unwrap()
        })
        .unwrap()
        .join()
        .unwrap()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_reward_accounts_catchup_endpoint() -> anyhow::Result<()> {
    const EPOCH_HEIGHT: u64 = 10;
    const NUM_NODES: usize = 3;

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .build();

    let api_port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    println!("API PORT = {api_port}");

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
            POS_V4,
        )
        .await
        .unwrap()
        .build();

    let mut network = TestNetwork::new(config, POS_V4).await;

    let client: Client<ServerError, StaticVersion<0, 1>> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    client.connect(None).await;

    let mut events = network.server.event_stream();
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 3).await;

    network.stop_consensus().await;
    let height = network.server.decided_leaf().await.height();
    wait_until_block_height(&client, "reward-state-v2/block-height", height).await;

    let err = client
        .get::<Vec<(RewardAccountV2, RewardAmount)>>(&format!(
            "reward-state-v2/reward-amounts/{height}/0/10001"
        ))
        .send()
        .await
        .unwrap_err();

    assert_matches!(err, ServerError { status, .. } if
        status == StatusCode::BAD_REQUEST

    );

    let mut expected: Vec<_> = network
        .server
        .decided_state()
        .await
        .unwrap()
        .reward_merkle_tree_v2
        .iter()
        .map(|(addr, amt)| (*addr, *amt))
        .collect();
    // Results are sorted by account address descending
    expected.sort_by_key(|(acct, _)| std::cmp::Reverse(*acct));

    tracing::info!("expected accounts = {expected:?}");
    let limit = expected.len().min(10_000) as u64;
    let offset = 0u64;
    let expected: Vec<_> = expected.into_iter().take(limit as usize).collect();

    let res = client
        .get::<Vec<(RewardAccountV2, RewardAmount)>>(&format!(
            "reward-state-v2/reward-amounts/{height}/{offset}/{limit}"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(res, expected);

    Ok(())
}
