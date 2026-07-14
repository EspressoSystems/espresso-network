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

/// Assert both tide-disco and axum return a 2xx for the path. Used for endpoints whose
/// content varies between calls (e.g. wall-clock-dependent fields, live metrics).
async fn compare_endpoints_ok(
    http: &reqwest::Client,
    api_port: u16,
    axum_port: u16,
    path: &str,
) -> anyhow::Result<()> {
    let tide = http
        .get(format!("http://localhost:{api_port}/v1/{path}"))
        .send()
        .await?
        .status();
    let axum = http
        .get(format!("http://localhost:{axum_port}/v1/{path}"))
        .send()
        .await?
        .status();
    assert!(
        tide.is_success(),
        "v1/{path}: tide returned {tide}, expected 2xx"
    );
    assert!(
        axum.is_success(),
        "v1/{path}: axum returned {axum}, expected 2xx"
    );
    Ok(())
}

/// Byte-compare error response bodies between tide and axum for an endpoint that uses
/// `Error::catch_all` (which emits `{"Custom":{"message","status"}}` on tide). Axum's
/// `ErrorResponse` is shaped to match that envelope, so the JSON bytes are equal modulo
/// minor whitespace differences — comparing parsed JSON values neutralizes those.
async fn compare_error_body(
    http: &reqwest::Client,
    api_port: u16,
    axum_port: u16,
    path: &str,
    expected_status: u16,
) -> anyhow::Result<()> {
    let fetch = |port: u16| async move {
        let resp = http
            .get(format!("http://localhost:{port}/v1/{path}"))
            .send()
            .await?;
        let status = resp.status().as_u16();
        let body: serde_json::Value = resp.json().await?;
        anyhow::Ok((status, body))
    };
    let (tide_status, tide_body) = fetch(api_port).await?;
    let (axum_status, axum_body) = fetch(axum_port).await?;
    assert_eq!(tide_status, expected_status, "v1/{path}: tide status");
    assert_eq!(axum_status, expected_status, "v1/{path}: axum status");
    assert_eq!(
        tide_body, axum_body,
        "v1/{path}: tide and axum error bodies differ\n  tide: {tide_body}\n  axum: {axum_body}"
    );
    Ok(())
}

/// POST a VBS-binary body to both servers and assert their responses are byte-equal.
///
/// VBS (Versioned Binary Serialization) is what production peer-catchup and
/// `submit-transactions` clients use via `surf-disco::Request::body_binary`. This helper
/// catches regressions where the axum handler accepts only JSON.
async fn compare_post_binary<B: serde::Serialize>(
    http: &reqwest::Client,
    api_port: u16,
    axum_port: u16,
    path: &str,
    body: &B,
) -> anyhow::Result<()> {
    use vbs::{BinarySerializer, Serializer, version::StaticVersion};
    let payload = Serializer::<StaticVersion<0, 1>>::serialize(body)?;
    let send = |port: u16| {
        let payload = payload.clone();
        http.post(format!("http://localhost:{port}/v1/{path}"))
            .header("Content-Type", "application/octet-stream")
            .header("Accept", "application/octet-stream")
            .body(payload)
            .send()
    };
    let (tide_resp, axum_resp) = tokio::join!(send(api_port), send(axum_port));
    let tide_resp = tide_resp?;
    let axum_resp = axum_resp?;
    assert_eq!(
        tide_resp.status(),
        axum_resp.status(),
        "v1/{path}: tide status {} != axum status {}",
        tide_resp.status(),
        axum_resp.status(),
    );
    // Compare raw bytes — VBS responses aren't JSON.
    let tide_body = tide_resp.bytes().await?;
    let axum_body = axum_resp.bytes().await?;
    assert_eq!(
        tide_body, axum_body,
        "v1/{path}: tide and axum binary POST responses differ"
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

/// Same as `compare_ws_endpoints` but exercises the binary (`Accept: application/octet-stream`)
/// path that surf-disco clients use by default. Asserts both servers send `Message::Binary`
/// frames carrying VBS-encoded payloads.
async fn compare_ws_endpoints_binary(
    api_port: u16,
    axum_port: u16,
    path: &str,
) -> anyhow::Result<()> {
    use std::time::Duration;

    use futures::StreamExt as _;
    use tokio::time::timeout;
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{client::IntoClientRequest, http::HeaderValue, protocol::Message},
    };

    async fn collect_binary(port: u16, path: &str) -> anyhow::Result<Vec<Vec<u8>>> {
        let url = format!("ws://localhost:{port}/v1/{path}");
        let mut req = url.as_str().into_client_request()?;
        req.headers_mut().insert(
            "Accept",
            HeaderValue::from_static("application/octet-stream"),
        );
        let (mut ws, _) = connect_async(req).await?;
        let mut frames = Vec::new();
        while frames.len() < 3 {
            match timeout(Duration::from_millis(500), ws.next()).await {
                Ok(Some(Ok(Message::Binary(bytes)))) => frames.push(bytes.to_vec()),
                _ => break,
            }
        }
        Ok(frames)
    }

    let (tide_frames, axum_frames) = tokio::join!(
        collect_binary(api_port, path),
        collect_binary(axum_port, path)
    );
    let tide_frames = tide_frames?;
    let axum_frames = axum_frames?;

    assert!(
        !tide_frames.is_empty(),
        "v1/{path}: tide sent no binary frames (Accept: application/octet-stream)"
    );
    assert!(
        !axum_frames.is_empty(),
        "v1/{path}: axum sent no binary frames (Accept: application/octet-stream); handler likely \
         always sends text",
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

        let mut api_opts = Options::with_port(api_port)
            .catchup(Default::default())
            .config(Default::default())
            .submit(Default::default())
            .explorer(Default::default())
            .light_client(Default::default())
            .hotshot_events(HotshotEvents);
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

            // Sample a fee account for the fee-state comparisons below.
            // `validated_state` was captured before the fee-paying blocks above were
            // decided, so its fee tree can still be empty; poll the decided state while
            // consensus is still running (it is frozen after stop_consensus). The
            // decided state can also contain accounts added after `avail_block`, so
            // only accept an account provable at the `avail_block` snapshot queried
            // in the comparisons.
            let sample_start = Instant::now();
            let fee_account = 'fee_account: loop {
                let state = network.server.decided_state().await.unwrap();
                for (addr, _) in state.fee_merkle_tree.iter() {
                    if client
                        .get::<MerkleProof<FeeAmount, FeeAccount, Sha3Node, 256>>(&format!(
                            "fee-state/{avail_block}/{addr}"
                        ))
                        .send()
                        .await
                        .is_ok()
                    {
                        break 'fee_account *addr;
                    }
                }
                assert!(
                    sample_start.elapsed() < Duration::from_secs(30),
                    "no fee account provable at avail_block {avail_block} after 30s"
                );
                sleep(Duration::from_millis(500)).await;
            };

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

            // surf-disco clients default to `Accept: application/octet-stream`, so the
            // server must emit `Message::Binary` (VBS-encoded) frames on that path.
            // Verify both servers do so on a representative stream.
            compare_ws_endpoints_binary(
                api_port,
                axum_port,
                &format!("availability/stream/leaves/{ws_start}"),
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

            // fee-state path by height for a known fee account (sampled above while
            // consensus was running), and fee-balance/latest for the same account.
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

            // Status parity. Block height and success rate are stable since consensus is
            // stopped; time-since-last-decide and metrics vary by wall-clock so we only
            // check that both servers return 2xx.
            compare_endpoints(&http, api_port, axum_port, "status/block-height").await?;
            compare_endpoints(&http, api_port, axum_port, "status/success-rate").await?;
            compare_endpoints_ok(&http, api_port, axum_port, "status/time-since-last-decide")
                .await?;
            compare_endpoints_ok(&http, api_port, axum_port, "status/metrics").await?;

            // Config parity. /hotshot and /env are derived from process-level state shared
            // by both servers; /runtime returns 404 in both because no PublicNodeConfig was
            // configured for this test.
            compare_endpoints(&http, api_port, axum_port, "config/hotshot").await?;
            compare_endpoints(&http, api_port, axum_port, "config/env").await?;
            compare_error_endpoints(&http, api_port, axum_port, "config/runtime", 404).await?;

            // Node parity. All endpoints share the same data source so byte-equal responses
            // are expected once consensus is stopped.
            compare_endpoints(&http, api_port, axum_port, "node/block-height").await?;
            compare_endpoints(&http, api_port, axum_port, "node/transactions/count").await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/transactions/count/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/transactions/count/0/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/transactions/count/namespace/{avail_ns}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/transactions/count/namespace/{avail_ns}/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/transactions/count/namespace/{avail_ns}/0/{avail_block}"),
            )
            .await?;

            compare_endpoints(&http, api_port, axum_port, "node/payloads/size").await?;
            compare_endpoints(&http, api_port, axum_port, "node/payloads/total-size").await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/payloads/size/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/payloads/size/0/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/payloads/size/namespace/{avail_ns}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/payloads/size/namespace/{avail_ns}/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/payloads/size/namespace/{avail_ns}/0/{avail_block}"),
            )
            .await?;

            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/vid/share/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/vid/share/hash/{block_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/vid/share/payload-hash/{payload_hash}"),
            )
            .await?;

            compare_endpoints(&http, api_port, axum_port, "node/sync-status").await?;
            compare_endpoints(&http, api_port, axum_port, "node/limits").await?;

            // Header window: cover all three start variants (time, height, hash). `end` is
            // an exclusive Unix-second cutoff; using the block's own timestamp + 1 yields
            // a deterministic single-block window on both servers.
            let avail_ts = avail_header.timestamp();
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/header/window/{avail_ts}/{}", avail_ts + 1),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/header/window/from/{avail_block}/{}", avail_ts + 1),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("node/header/window/from/hash/{block_hash}/{}", avail_ts + 1),
            )
            .await?;

            compare_endpoints(&http, api_port, axum_port, "node/stake-table/current").await?;
            compare_endpoints(&http, api_port, axum_port, "node/stake-table/1").await?;
            compare_endpoints(&http, api_port, axum_port, "node/da-stake-table/current").await?;
            compare_endpoints(&http, api_port, axum_port, "node/da-stake-table/1").await?;

            compare_endpoints(&http, api_port, axum_port, "node/validators/1").await?;
            compare_endpoints(&http, api_port, axum_port, "node/all-validators/1/0/100").await?;

            compare_endpoints(
                &http,
                api_port,
                axum_port,
                "node/participation/proposal/current",
            )
            .await?;
            compare_endpoints(&http, api_port, axum_port, "node/participation/proposal/1").await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                "node/participation/vote/current",
            )
            .await?;
            compare_endpoints(&http, api_port, axum_port, "node/participation/vote/1").await?;

            compare_endpoints(&http, api_port, axum_port, "node/block-reward").await?;
            compare_endpoints(&http, api_port, axum_port, "node/block-reward/epoch/1").await?;

            compare_endpoints(&http, api_port, axum_port, "node/oldest-block").await?;
            compare_endpoints(&http, api_port, axum_port, "node/oldest-leaf").await?;

            // Catchup parity. View number and height for in-memory state aren't readily
            // available after stopping consensus, so we compare error semantics on
            // intentionally invalid lookups and the deprecated routes.
            let decided_view = decided_leaf.view_number().u64();
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("catchup/{height}/{decided_view}/blocks"),
            )
            .await?;
            // chain-config: a malformed TaggedBase64 commitment (bad checksum) parses-fails
            // on the request path and yields 400 from both servers.
            compare_error_endpoints(
                &http,
                api_port,
                axum_port,
                "catchup/chain-config/CHAINCONFIG~AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                400,
            )
            .await?;
            // leafchain: undecided height returns 404 from both.
            compare_error_endpoints(&http, api_port, axum_port, "catchup/999999/leafchain", 404)
                .await?;
            // cert2: missing cert returns 404.
            compare_error_endpoints(&http, api_port, axum_port, "catchup/999999/cert2", 404)
                .await?;
            // Deprecated catchup routes still respond 404.
            compare_error_endpoints(
                &http,
                api_port,
                axum_port,
                "catchup/1/reward-amounts/100/0",
                404,
            )
            .await?;

            // Production peer-catchup posts VBS-binary bodies via surf-disco.
            // Exercise the bulk-account POST endpoints in that exact wire format so any
            // regression to "JSON-only body" is caught here.
            // Reuse the account sampled above; `validated_state.fee_merkle_tree` was
            // captured before the fee-paying blocks and can be empty.
            compare_post_binary(
                &http,
                api_port,
                axum_port,
                &format!("catchup/{height}/{decided_view}/accounts"),
                &vec![fee_account],
            )
            .await?;
            // reward-accounts V1 takes a Vec<RewardAccountV1>. We send empty since the V2
            // tree may not have V1-shaped entries in this test, but the wire format is what
            // we're validating.
            compare_post_binary(
                &http,
                api_port,
                axum_port,
                &format!("catchup/{height}/{decided_view}/reward-accounts"),
                &Vec::<espresso_types::v0_3::RewardAccountV1>::new(),
            )
            .await?;

            // State signature parity. Heights that have a signature should return matching
            // JSON; missing heights should 404 from both servers.
            compare_error_endpoints(
                &http,
                api_port,
                axum_port,
                "state-signature/block/999999",
                404,
            )
            .await?;

            // Error body parity for endpoints that use `Error::catch_all` — both servers
            // must emit byte-identical `{"Custom":{"message","status"}}` JSON. Availability
            // endpoints (`availability/leaf/...`, etc.) are excluded because tide-disco
            // returns specific variants like `{"FetchLeaf":{...}}` there; their status-code
            // parity is still enforced by `compare_error_endpoints` above.
            compare_error_body(&http, api_port, axum_port, "catchup/999999/cert2", 404).await?;
            compare_error_body(&http, api_port, axum_port, "catchup/999999/leafchain", 404).await?;
            compare_error_body(
                &http,
                api_port,
                axum_port,
                "state-signature/block/999999",
                404,
            )
            .await?;

            // Explorer parity.
            compare_endpoints(&http, api_port, axum_port, "explorer/explorer-summary").await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("explorer/block/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("explorer/block/hash/{block_hash}"),
            )
            .await?;
            compare_endpoints(&http, api_port, axum_port, "explorer/blocks/latest/10").await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("explorer/blocks/{avail_block}/10"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                "explorer/transactions/latest/10",
            )
            .await?;

            // Light-client parity. Use the same block we used for availability tests.
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("light-client/leaf/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("light-client/leaf/hash/{leaf_hash}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("light-client/payload/{avail_block}"),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!("light-client/payload/{avail_block}/{}", avail_block + 1),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!(
                    "light-client/namespace/{avail_block}/{}",
                    u64::from(avail_ns)
                ),
            )
            .await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                &format!(
                    "light-client/namespace/{avail_block}/{}/{}",
                    avail_block + 1,
                    u64::from(avail_ns)
                ),
            )
            .await?;

            // hotshot-events startup info: both must return matching JSON.
            compare_endpoints(&http, api_port, axum_port, "hotshot-events/startup_info").await?;

            // Token parity. Both servers share the same data source, so the cached
            // L1 supply values must match across calls.
            compare_endpoints(&http, api_port, axum_port, "token/total-minted-supply").await?;
            compare_endpoints(&http, api_port, axum_port, "token/circulating-supply").await?;
            compare_endpoints(
                &http,
                api_port,
                axum_port,
                "token/circulating-supply-ethereum",
            )
            .await?;
            compare_endpoints(&http, api_port, axum_port, "token/total-issued-supply").await?;
            compare_endpoints(&http, api_port, axum_port, "token/total-reward-distributed").await?;

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
