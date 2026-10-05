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
    let client: Client<ClientErr, SequencerApiVersion> =
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
    let client: Client<ClientErr, SequencerApiVersion> =
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
    let client: Client<ClientErr, SequencerApiVersion> =
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
    let client: Client<ClientErr, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    // Wait for chain to reach epoch 5
    let height_client: Client<ClientErr, StaticVersion<0, 1>> =
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
    let client: Client<ClientErr, SequencerApiVersion> =
        Client::new(format!("http://localhost:{api_port}").parse().unwrap());

    let height_client: Client<ClientErr, StaticVersion<0, 1>> =
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
    let client: Client<ClientErr, SequencerApiVersion> =
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
    let client: Client<ClientErr, SequencerApiVersion> =
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

    let network = TestNetwork::new(config, upgrade).await;
    let mut events = network.server.event_stream();
    let client: Client<ClientErr, SequencerApiVersion> =
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

    let v2_block_reward: serde_json::Value = client
        .get("v2/node/block-reward")
        .send()
        .await
        .expect("failed to get v2 block reward");
    assert_eq!(
        v2_block_reward,
        serde_json::json!({"amount": block_reward.0.to_string()})
    );

    // An epoch the chain has not reached has no committee and so no reward, which is what
    // makes this the probe that `epoch` reaches the per-epoch lookup at all: dropping the
    // parameter falls back to the fixed reward asserted above, and that is not empty.
    const UNREACHED_EPOCH: u64 = 1_000_000;
    let v1_epoch_reward = client
        .get::<Option<RewardAmount>>(&format!("node/block-reward/epoch/{UNREACHED_EPOCH}"))
        .send()
        .await
        .expect("failed to get v1 block reward for epoch");
    assert!(v1_epoch_reward.is_none(), "{v1_epoch_reward:?}");
    let v2_epoch_reward: serde_json::Value = client
        .get(&format!("v2/node/block-reward?epoch={UNREACHED_EPOCH}"))
        .send()
        .await
        .expect("failed to get v2 block reward for epoch");
    assert_eq!(v2_epoch_reward, serde_json::json!({}));

    // This is the only harness that registers validators, so it is the only place the
    // validator and participation mappings meet real data.
    let (epoch, _) = wait_for_committee(&client, &mut events, epoch_height, 1, 5, |validators| {
        !validators.is_empty()
    })
    .await;
    let v1_validators: serde_json::Value = client
        .get(&format!("node/validators/{epoch}"))
        .send()
        .await
        .expect("failed to get v1 validators");
    let v1_validators = v1_validators.as_object().expect("a map of validators");
    assert!(!v1_validators.is_empty());
    let v2_validators: espresso_api::proto::ValidatorsResponse = client
        .get(&format!("v2/node/validators?epoch={epoch}"))
        .send()
        .await
        .expect("failed to get v2 validators");
    assert_eq!(v2_validators.validators.len(), v1_validators.len());
    for v2 in &v2_validators.validators {
        // v1 keys each validator by the account the entry itself carries.
        let v1 = &v1_validators[&v2.account];
        assert_eq!(v2.stake, v1["stake"].as_str().unwrap());
        assert_eq!(v2.commission, v1["commission"].as_u64().unwrap() as u32);
        assert_eq!(v2.authenticated, v1["authenticated"].as_bool().unwrap());
        assert_eq!(
            v2.stake_table_key.as_ref().map(|key| key.key.as_str()),
            v1["stake_table_key"].as_str()
        );
        assert_eq!(
            v2.state_ver_key.as_ref().map(|key| key.key.as_str()),
            v1["state_ver_key"].as_str()
        );
        let v1_delegators = v1["delegators"].as_object().unwrap();
        assert_eq!(v2.delegators.len(), v1_delegators.len());
        for delegator in &v2.delegators {
            assert_eq!(
                delegator.amount,
                v1_delegators[&delegator.account].as_str().unwrap()
            );
        }
    }

    let v1_page: serde_json::Value = client
        .get(&format!("node/all-validators/{epoch}/0/1000"))
        .send()
        .await
        .expect("failed to get the v1 validator page");
    let v2_page: espresso_api::proto::ValidatorsResponse = client
        .get(&format!(
            "v2/node/all-validators?epoch={epoch}&offset=0&limit=1000"
        ))
        .send()
        .await
        .expect("failed to get the v2 validator page");
    let v1_page = v1_page.as_array().unwrap();
    assert!(!v1_page.is_empty());
    assert_eq!(
        v2_page
            .validators
            .iter()
            .map(|validator| validator.account.as_str())
            .collect::<Vec<_>>(),
        v1_page
            .iter()
            .map(|validator| validator["account"].as_str().unwrap())
            .collect::<Vec<_>>()
    );

    // Walking one row at a time is what pins `offset` against `limit`: transposed, the page
    // never moves. Only reachable with more than one registered validator.
    for (offset, v1_row) in v1_page.iter().enumerate() {
        let v2_row: espresso_api::proto::ValidatorsResponse = client
            .get(&format!(
                "v2/node/all-validators?epoch={epoch}&offset={offset}&limit=1"
            ))
            .send()
            .await
            .expect("failed to get a v2 validator row");
        assert_eq!(
            v2_row.validators.first().map(|v| v.account.as_str()),
            Some(v1_row["account"].as_str().unwrap()),
            "offset {offset}"
        );
    }

    // v1 refuses this as a bad request, so v2 must not report it as an internal error.
    let v1_err = client
        .get::<serde_json::Value>(&format!("node/all-validators/{epoch}/0/1001"))
        .send()
        .await
        .unwrap_err();
    let v2_err = client
        .get::<serde_json::Value>(&format!(
            "v2/node/all-validators?epoch={epoch}&offset=0&limit=1001"
        ))
        .send()
        .await
        .unwrap_err();
    assert_eq!(v1_err.status, StatusCode::BAD_REQUEST, "{v1_err}");
    assert_eq!(v2_err.status, v1_err.status, "{v2_err}");

    // Omitting a required parameter is refused rather than read as epoch or limit zero.
    for route in [
        "v2/node/validators",
        "v2/node/all-validators?epoch=1&offset=0",
        "v2/node/header-window?start_time=0",
    ] {
        let err = client
            .get::<serde_json::Value>(route)
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST, "{route}: {err}");
    }

    // Proposal participation is the arm the shorter test cannot reach, and comparing it here
    // catches a handler that delegates to the vote method instead.
    let v1_proposals: serde_json::Value = client
        .get("node/participation/proposal/current")
        .send()
        .await
        .expect("failed to get v1 proposal participation");
    let v2_proposals: espresso_api::proto::ParticipationResponse = client
        .get("v2/node/participation/proposal")
        .send()
        .await
        .expect("failed to get v2 proposal participation");
    let v1_proposals = v1_proposals.as_object().unwrap();
    assert_eq!(v2_proposals.participation.len(), v1_proposals.len());
    for entry in &v2_proposals.participation {
        let key = &entry.key.as_ref().unwrap().key;
        assert_eq!(entry.participation, v1_proposals[key].as_f64().unwrap());
    }

    Ok(())
}

/// Every reward-state response on v2 must be the conversion of what v1 serves for the same
/// request. `height` must be one the light client contract finalized, since only those carry
/// stored proofs, and `address` an account in the reward tree.
async fn check_reward_state_v2_parity(
    client: &HttpClient,
    height: u64,
    address: alloy::primitives::Address,
) {
    use espresso_api::proto;

    let v1_balance: espresso_types::v0_3::RewardAmount = fetch(
        client,
        &format!("reward-state-v2/reward-balance/{height}/{address}"),
    )
    .await;
    let v2: proto::RewardBalanceResponse = fetch(
        client,
        &format!("v2/merklized-state/reward/balance?address={address}&height={height}"),
    )
    .await;
    assert_eq!(v2.balance, v1_balance.to_string());
    let v1_latest: espresso_types::v0_3::RewardAmount = fetch(
        client,
        &format!("reward-state-v2/reward-balance/latest/{address}"),
    )
    .await;
    let v2: proto::RewardBalanceResponse = fetch(
        client,
        &format!("v2/merklized-state/reward/balance?address={address}"),
    )
    .await;
    assert_eq!(v2.balance, v1_latest.to_string());

    for (v1, v2) in [
        (
            format!("reward-state-v2/proof/{height}/{address}"),
            format!("v2/merklized-state/reward/proof?address={address}&height={height}"),
        ),
        (
            format!("reward-state-v2/proof/latest/{address}"),
            format!("v2/merklized-state/reward/proof?address={address}"),
        ),
    ] {
        let v1_proof: RewardAccountQueryDataV2 = fetch(client, &v1).await;
        assert!(matches!(
            v1_proof.proof.proof,
            RewardMerkleProofV2::Presence(_)
        ));
        let v2_proof: proto::RewardAccountProofResponse = fetch(client, &v2).await;
        assert_eq!(
            v2_proof,
            proto::RewardAccountProofResponse::from(v1_proof),
            "{v2}"
        );
    }

    let v1_claim: RewardClaimInput = fetch(
        client,
        &format!("reward-state-v2/reward-claim-input/{height}/{address}"),
    )
    .await;
    let v2: proto::RewardClaimInputResponse = fetch(
        client,
        &format!("v2/merklized-state/reward/claim-input?address={address}&height={height}"),
    )
    .await;
    assert_eq!(v2.lifetime_rewards, v1_claim.lifetime_rewards.to_string());
    assert_eq!(
        v2.auth_data,
        alloy::primitives::Bytes::from(v1_claim.auth_data).to_string()
    );

    // v1 reverses each page, and v2 serves the tree's own order.
    let v1_amounts: Vec<(
        alloy::primitives::Address,
        espresso_types::v0_3::RewardAmount,
    )> = fetch(
        client,
        &format!("reward-state-v2/reward-amounts/{height}/0/1000"),
    )
    .await;
    let v2: proto::RewardAmountsResponse = fetch(
        client,
        &format!("v2/merklized-state/reward/amounts?height={height}&offset=0&limit=1000"),
    )
    .await;
    assert!(!v1_amounts.is_empty());
    assert_eq!(
        v2.amounts,
        v1_amounts
            .iter()
            .rev()
            .map(|(address, amount)| proto::RewardAmountPair {
                address: address.to_string(),
                amount: amount.to_string(),
            })
            .collect::<Vec<_>>()
    );

    let v1_tree: Vec<u8> = fetch(
        client,
        &format!("reward-state-v2/reward-merkle-tree-v2/{height}"),
    )
    .await;
    let v2: proto::RewardMerkleTreeV2Response = fetch(
        client,
        &format!("v2/merklized-state/reward/tree?height={height}"),
    )
    .await;
    assert_eq!(v2.tree, v1_tree);

    let absent = alloy::primitives::Address::with_last_byte(0xaa);
    let beyond = height + 1_000_000;
    for (v1, v2) in [
        (
            format!("reward-state-v2/reward-balance/{height}/{absent}"),
            format!("v2/merklized-state/reward/balance?address={absent}&height={height}"),
        ),
        (
            format!("reward-state-v2/proof/{height}/{absent}"),
            format!("v2/merklized-state/reward/proof?address={absent}&height={height}"),
        ),
        (
            format!("reward-state-v2/reward-claim-input/{height}/{absent}"),
            format!("v2/merklized-state/reward/claim-input?address={absent}&height={height}"),
        ),
        (
            format!("reward-state-v2/reward-balance/{height}/not-an-address"),
            format!("v2/merklized-state/reward/balance?address=not-an-address&height={height}"),
        ),
        (
            format!("reward-state-v2/reward-balance/{beyond}/{address}"),
            format!("v2/merklized-state/reward/balance?address={address}&height={beyond}"),
        ),
        (
            format!("reward-state-v2/reward-amounts/{height}/0/10001"),
            format!("v2/merklized-state/reward/amounts?height={height}&offset=0&limit=10001"),
        ),
        (
            format!("reward-state-v2/reward-amounts/{height}/1000000/10"),
            format!("v2/merklized-state/reward/amounts?height={height}&offset=1000000&limit=10"),
        ),
    ] {
        assert_eq!(
            error_status(client, &v2).await,
            error_status(client, &v1).await,
            "{v2}"
        );
    }
    for missing in [
        "balance".to_owned(),
        "proof".to_owned(),
        format!("claim-input?address={address}"),
        format!("claim-input?height={height}"),
        format!("amounts?height={height}&offset=0"),
        "tree".to_owned(),
    ] {
        let status = error_status(client, &format!("v2/merklized-state/reward/{missing}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{missing}");
    }
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

/// Assert the endpoint returns a 2xx status and a valid JSON body.
async fn assert_json_endpoint(
    http: &reqwest::Client,
    api_port: u16,
    path: &str,
) -> anyhow::Result<()> {
    let resp = http
        .get(format!("http://localhost:{api_port}/v1/{path}"))
        .send()
        .await?;
    let status = resp.status();
    assert!(
        status.is_success(),
        "v1/{path}: returned {status}, expected 2xx"
    );
    resp.json::<serde_json::Value>().await?;
    Ok(())
}

/// Assert the endpoint returns a well-formed JSON body, without constraining the status.
/// For routes where an error response is the expected outcome but its exact status is not
/// part of the contract.
async fn assert_json_body(http: &reqwest::Client, api_port: u16, path: &str) -> anyhow::Result<()> {
    http.get(format!("http://localhost:{api_port}/v1/{path}"))
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;
    Ok(())
}

/// Assert the endpoint returns the expected HTTP status code.
async fn assert_endpoint_status(
    http: &reqwest::Client,
    api_port: u16,
    path: &str,
    expected_status: u16,
) -> anyhow::Result<()> {
    let status = http
        .get(format!("http://localhost:{api_port}/v1/{path}"))
        .send()
        .await?
        .status()
        .as_u16();
    assert_eq!(
        status, expected_status,
        "v1/{path}: should return {expected_status}, got {status}"
    );
    Ok(())
}

/// Assert the endpoint returns a 2xx status, without requiring a JSON body. Used for
/// endpoints whose content is not JSON or varies between calls (e.g. live metrics).
async fn assert_endpoint_ok(
    http: &reqwest::Client,
    api_port: u16,
    path: &str,
) -> anyhow::Result<()> {
    let status = http
        .get(format!("http://localhost:{api_port}/v1/{path}"))
        .send()
        .await?
        .status();
    assert!(
        status.is_success(),
        "v1/{path}: returned {status}, expected 2xx"
    );
    Ok(())
}

/// Assert an endpoint that fails via `ApiError` returns the expected status and the
/// `{"Custom":{"message","status"}}` error envelope that existing clients parse.
async fn assert_error_body(
    http: &reqwest::Client,
    api_port: u16,
    path: &str,
    expected_status: u16,
) -> anyhow::Result<()> {
    let resp = http
        .get(format!("http://localhost:{api_port}/v1/{path}"))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(status, expected_status, "v1/{path}: status");
    let custom = body
        .get("Custom")
        .unwrap_or_else(|| panic!("v1/{path}: error body missing Custom envelope: {body}"));
    assert_eq!(
        custom.get("status").and_then(|s| s.as_u64()),
        Some(u64::from(expected_status)),
        "v1/{path}: envelope status: {body}"
    );
    assert!(
        custom.get("message").is_some_and(|m| m.is_string()),
        "v1/{path}: envelope message missing: {body}"
    );
    Ok(())
}

/// POST a VBS-binary body and assert the server accepts it.
///
/// VBS (Versioned Binary Serialization) is what production peer-catchup and
/// `submit-transactions` clients use via `http_client::Request::body_binary`. This helper
/// catches regressions where the handler accepts only JSON.
async fn assert_post_binary<B: serde::Serialize>(
    http: &reqwest::Client,
    api_port: u16,
    path: &str,
    body: &B,
) -> anyhow::Result<()> {
    use vbs::{BinarySerializer, Serializer, version::StaticVersion};
    let payload = Serializer::<StaticVersion<0, 1>>::serialize(body)?;
    let resp = http
        .post(format!("http://localhost:{api_port}/v1/{path}"))
        .header("Content-Type", "application/octet-stream")
        .header("Accept", "application/octet-stream")
        .body(payload)
        .send()
        .await?;
    let status = resp.status();
    assert!(
        status.is_success(),
        "v1/{path}: binary POST returned {status}, expected 2xx"
    );
    Ok(())
}

/// Connect to the WebSocket endpoint, collect up to 10 messages, and assert that at least 2
/// of them are valid JSON.
async fn assert_ws_endpoint(api_port: u16, path: &str) -> anyhow::Result<()> {
    use std::time::Duration;

    use futures::StreamExt as _;
    use tokio::time::timeout;
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let url = format!("ws://localhost:{api_port}/v1/{path}");
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

    assert!(
        messages.len() >= 2,
        "v1/{path}: expected >=2 JSON messages from the stream, got {}",
        messages.len(),
    );
    Ok(())
}

/// Same as `assert_ws_endpoint` but exercises the binary (`Accept: application/octet-stream`)
/// path that our clients use by default. Asserts the server sends `Message::Binary` frames
/// carrying VBS-encoded payloads.
async fn assert_ws_endpoint_binary(api_port: u16, path: &str) -> anyhow::Result<()> {
    use std::time::Duration;

    use futures::StreamExt as _;
    use tokio::time::timeout;
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{client::IntoClientRequest, http::HeaderValue, protocol::Message},
    };

    let url = format!("ws://localhost:{api_port}/v1/{path}");
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

    assert!(
        !frames.is_empty(),
        "v1/{path}: no binary frames (Accept: application/octet-stream); handler likely always \
         sends text",
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
        println!("API PORT = {api_port}");

        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let api_opts = Options::with_port(api_port)
            .catchup(Default::default())
            .config(Default::default())
            .explorer(Default::default())
            .light_client(Default::default());

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
        let client: Client<ClientErr, StaticVersion<0, 1>> = Client::new(url);

        let validated_state = network.server.decided_state().await.unwrap();
        let decided_leaf = network.server.decided_leaf().await;
        let height = decided_leaf.height();

        // validate proof returned from the api
        if upgrade.base == EPOCH_VERSION {
            // V1 case: only the legacy v1 reward tree endpoints apply here
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

                // Behavior relied on by scripts/claim-rewards-loop: an account with no
                // rewards yields 404; any other error status makes the claim loop exit and
                // process-compose tear down the whole demo.
                let absent = alloy::primitives::Address::with_last_byte(0xaa);
                assert!(
                    validated_state
                        .reward_merkle_tree_v2
                        .iter()
                        .all(|(addr, _)| addr.0 != absent),
                    "sentinel address unexpectedly present in reward tree"
                );
                let err = client
                    .get::<RewardClaimInput>(&format!(
                        "reward-state-v2/reward-claim-input/{height}/{absent}"
                    ))
                    .send()
                    .await
                    .unwrap_err();
                assert_matches!(err, ClientErr { status, .. } if status == StatusCode::NOT_FOUND);

                // Smoke-check each per-address endpoint under reward-state-v2.
                assert_json_endpoint(
                    &http,
                    api_port,
                    &format!("reward-state-v2/proof/{height}/{address}"),
                )
                .await?;
                assert_json_endpoint(
                    &http,
                    api_port,
                    &format!("reward-state-v2/reward-claim-input/{height}/{address}"),
                )
                .await?;
                assert_json_endpoint(
                    &http,
                    api_port,
                    &format!("reward-state-v2/reward-balance/{height}/{address}"),
                )
                .await?;
                assert_json_endpoint(
                    &http,
                    api_port,
                    &format!("reward-state-v2/proof/latest/{address}"),
                )
                .await?;
                assert_json_endpoint(
                    &http,
                    api_port,
                    &format!("reward-state-v2/reward-balance/latest/{address}"),
                )
                .await?;

                // The reward-state mount shares its handlers with reward-state-v2 for
                // backwards compatibility, so these two routes hit the same v2-tree-backed
                // handlers as the pair above, just under reward-state.
                assert_json_endpoint(
                    &http,
                    api_port,
                    &format!("reward-state/proof/latest/{address}"),
                )
                .await?;
                assert_json_endpoint(
                    &http,
                    api_port,
                    &format!("reward-state/reward-balance/latest/{address}"),
                )
                .await?;
            }

            let (address, _) = validated_state
                .reward_merkle_tree_v2
                .iter()
                .next()
                .expect("a proof-of-stake network has reward accounts");
            check_reward_state_v2_parity(&client, height, address.0).await;

            assert_json_endpoint(
                &http,
                api_port,
                &format!("reward-state-v2/reward-amounts/{height}/0/1000"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("reward-state-v2/reward-merkle-tree-v2/{height}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("reward-state/reward-amounts/{height}/0/1000"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("reward-state/reward-merkle-tree-v2/{height}"),
            )
            .await?;

            // Merklized-state `get_path` routes, inherited by both reward mounts from
            // the legacy `hotshot-query-service` merklized-state base routes (mirrors the block-state /
            // fee-state checks below). Nothing in this codebase populates the generic
            // merklized-state tables for the reward trees today; the reward-state modules
            // persist snapshots via the separate `persist_tree`/`load_tree` bincode-blob
            // mechanism instead, so these routes fail in practice. We only assert that both
            // mounts, in both height and commit form, return well-formed JSON.
            let reward_address = validated_state
                .reward_merkle_tree_v2
                .iter()
                .next()
                .map(|(addr, _)| *addr)
                .expect("reward tree should have at least one account");
            let reward_header: Header = client
                .get(&format!("availability/header/{height}"))
                .send()
                .await
                .unwrap();
            let reward_mt_commit = match reward_header.reward_merkle_tree_root() {
                either::Either::Left(commit) => commit.to_string(),
                either::Either::Right(commit) => commit.to_string(),
            };
            for mount in ["reward-state", "reward-state-v2"] {
                assert_json_body(
                    &http,
                    api_port,
                    &format!("{mount}/{height}/{reward_address}"),
                )
                .await?;
                assert_json_body(
                    &http,
                    api_port,
                    &format!("{mount}/commit/{reward_mt_commit}/{reward_address}"),
                )
                .await?;
            }

            // Availability v1 routes.

            // Namespace proof by height
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/block/{avail_block}/namespace/{avail_ns}"),
            )
            .await?;

            // Namespace proof by block hash and payload hash
            let avail_header: Header = client
                .get(&format!("availability/header/{avail_block}"))
                .send()
                .await
                .unwrap();
            assert_json_endpoint(
                &http,
                api_port,
                &format!(
                    "availability/block/hash/{}/namespace/{avail_ns}",
                    avail_header.commit()
                ),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!(
                    "availability/block/payload-hash/{}/namespace/{avail_ns}",
                    avail_header.payload_commitment()
                ),
            )
            .await?;

            // Namespace proof range
            assert_json_endpoint(
                &http,
                api_port,
                &format!(
                    "availability/block/{avail_block}/{}/namespace/{avail_ns}",
                    avail_block + 1
                ),
            )
            .await?;

            // State certificate endpoints (epoch 1 is complete after 4 epochs)
            assert_json_endpoint(&http, api_port, "availability/state-cert/1").await?;
            assert_json_endpoint(&http, api_port, "availability/state-cert-v2/1").await?;

            // HotShot availability endpoints: leaf, header, block, payload, vid/common, etc.
            let avail_leaf: LeafQueryData<SeqTypes> = client
                .get(&format!("availability/leaf/{avail_block}"))
                .send()
                .await
                .unwrap();
            let leaf_hash = avail_leaf.hash();
            let block_hash = avail_header.commit();
            let payload_hash = avail_header.payload_commitment();

            // Leaf endpoints
            assert_json_endpoint(&http, api_port, &format!("availability/leaf/{avail_block}"))
                .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/leaf/hash/{leaf_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/leaf/{avail_block}/{}", avail_block + 1),
            )
            .await?;

            // Header endpoints
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/header/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/header/hash/{block_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/header/payload-hash/{payload_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/header/{avail_block}/{}", avail_block + 1),
            )
            .await?;

            // Block endpoints
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/block/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/block/hash/{block_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/block/payload-hash/{payload_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/block/{avail_block}/{}", avail_block + 1),
            )
            .await?;

            // Payload endpoints
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/payload/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/payload/hash/{payload_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/payload/block-hash/{block_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/payload/{avail_block}/{}", avail_block + 1),
            )
            .await?;

            // VID common endpoints
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/vid/common/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/vid/common/hash/{block_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/vid/common/payload-hash/{payload_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/vid/common/{avail_block}/{}", avail_block + 1),
            )
            .await?;

            // Transaction endpoints
            let tx_hash = avail_tx.commit();
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/transaction/{avail_block}/0/noproof"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/transaction/hash/{tx_hash}/noproof"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/transaction/{avail_block}/0/proof"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/transaction/hash/{tx_hash}/proof"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/transaction/{avail_block}/0"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/transaction/hash/{tx_hash}"),
            )
            .await?;

            // Block summary endpoints
            assert_json_endpoint(
                &http,
                api_port,
                &format!("availability/block/summary/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!(
                    "availability/block/summaries/{avail_block}/{}",
                    avail_block + 1
                ),
            )
            .await?;

            // Limits endpoint (static response)
            assert_json_endpoint(&http, api_port, "availability/limits").await?;

            // Cert2 endpoint: `avail_block` is a mid-chain block with no cert2, so both APIs
            // return 404. Compare status only, since the two error bodies differ by design.
            assert_endpoint_status(
                &http,
                api_port,
                &format!("availability/cert2/{avail_block}"),
                404,
            )
            .await?;

            // WebSocket streaming endpoints.
            //
            // For unfiltered streams, start 10 blocks before avail_block so there are at
            // least 10 committed blocks ready to stream (consensus has already stopped).
            // For namespace-filtered streams, start at avail_block where the two submitted
            // transactions were included, giving >=2 matching messages.
            let ws_start = avail_block.saturating_sub(10);
            assert_ws_endpoint(api_port, &format!("availability/stream/leaves/{ws_start}")).await?;
            assert_ws_endpoint(api_port, &format!("availability/stream/headers/{ws_start}"))
                .await?;
            assert_ws_endpoint(api_port, &format!("availability/stream/blocks/{ws_start}")).await?;
            assert_ws_endpoint(
                api_port,
                &format!("availability/stream/payloads/{ws_start}"),
            )
            .await?;
            assert_ws_endpoint(
                api_port,
                &format!("availability/stream/vid/common/{ws_start}"),
            )
            .await?;
            assert_ws_endpoint(
                api_port,
                &format!("availability/stream/transactions/{ws_start}"),
            )
            .await?;
            // Namespace-filtered streams: start at avail_block; two transactions were
            // submitted so the stream produces ≥2 messages.
            assert_ws_endpoint(
                api_port,
                &format!("availability/stream/transactions/{avail_block}/namespace/{avail_ns}"),
            )
            .await?;
            assert_ws_endpoint(
                api_port,
                &format!("availability/stream/blocks/{avail_block}/namespace/{avail_ns}"),
            )
            .await?;

            // Our clients default to `Accept: application/octet-stream`, so the server must
            // emit `Message::Binary` (VBS-encoded) frames on that path. Verify it does so
            // on a representative stream.
            assert_ws_endpoint_binary(api_port, &format!("availability/stream/leaves/{ws_start}"))
                .await?;

            // Merklized state endpoints (block-state and fee-state). Wait for
            // the backend to have indexed the snapshot we'll query.
            wait_until_block_height(&client, "block-state/block-height", avail_block).await;
            wait_until_block_height(&client, "fee-state/block-height", avail_block).await;

            // block-state/block-height and fee-state/block-height (latest
            // height for which merklized state is available).
            assert_json_endpoint(&http, api_port, "block-state/block-height").await?;
            assert_json_endpoint(&http, api_port, "fee-state/block-height").await?;

            // block-state path by height: the merkle tree at height H
            // contains the headers of blocks [0, H), so a valid key is H-1.
            assert_json_endpoint(
                &http,
                api_port,
                &format!(
                    "block-state/{avail_block}/{}",
                    avail_block.saturating_sub(1)
                ),
            )
            .await?;

            // block-state path by commit. Use the tree commitment from
            // the header at avail_block.
            let block_mt_commit = avail_header.block_merkle_tree_root().to_string();
            assert_json_endpoint(
                &http,
                api_port,
                &format!(
                    "block-state/commit/{block_mt_commit}/{}",
                    avail_block.saturating_sub(1)
                ),
            )
            .await?;

            // fee-state path by height for a known fee account (sampled above while
            // consensus was running), and fee-balance/latest for the same account.
            assert_json_endpoint(
                &http,
                api_port,
                &format!("fee-state/{avail_block}/{fee_account}"),
            )
            .await?;
            let fee_mt_commit = avail_header.fee_merkle_tree_root().to_string();
            assert_json_endpoint(
                &http,
                api_port,
                &format!("fee-state/commit/{fee_mt_commit}/{fee_account}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("fee-state/fee-balance/latest/{fee_account}"),
            )
            .await?;

            // Status endpoints. Block height and success rate are stable since consensus is
            // stopped; time-since-last-decide and metrics vary by wall-clock so we only
            // check for a 2xx.
            assert_json_endpoint(&http, api_port, "status/block-height").await?;
            assert_json_endpoint(&http, api_port, "status/success-rate").await?;
            assert_endpoint_ok(&http, api_port, "status/time-since-last-decide").await?;
            assert_endpoint_ok(&http, api_port, "status/metrics").await?;

            // Config endpoints. /runtime returns 404 because no PublicNodeConfig was
            // configured for this test.
            assert_json_endpoint(&http, api_port, "config/hotshot").await?;
            assert_json_endpoint(&http, api_port, "config/env").await?;
            assert_endpoint_status(&http, api_port, "config/runtime", 404).await?;

            // Node endpoints.
            assert_json_endpoint(&http, api_port, "node/block-height").await?;
            assert_json_endpoint(&http, api_port, "node/transactions/count").await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/transactions/count/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/transactions/count/0/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/transactions/count/namespace/{avail_ns}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/transactions/count/namespace/{avail_ns}/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/transactions/count/namespace/{avail_ns}/0/{avail_block}"),
            )
            .await?;

            assert_json_endpoint(&http, api_port, "node/payloads/size").await?;
            assert_json_endpoint(&http, api_port, "node/payloads/total-size").await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/payloads/size/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/payloads/size/0/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/payloads/size/namespace/{avail_ns}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/payloads/size/namespace/{avail_ns}/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/payloads/size/namespace/{avail_ns}/0/{avail_block}"),
            )
            .await?;

            assert_json_endpoint(&http, api_port, &format!("node/vid/share/{avail_block}")).await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/vid/share/hash/{block_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/vid/share/payload-hash/{payload_hash}"),
            )
            .await?;

            assert_json_endpoint(&http, api_port, "node/sync-status").await?;
            assert_json_endpoint(&http, api_port, "node/limits").await?;

            // Header window: cover all three start variants (time, height, hash). `end` is
            // an exclusive Unix-second cutoff; using the block's own timestamp + 1 yields
            // a deterministic single-block window.
            let avail_ts = avail_header.timestamp();
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/header/window/{avail_ts}/{}", avail_ts + 1),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/header/window/from/{avail_block}/{}", avail_ts + 1),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("node/header/window/from/hash/{block_hash}/{}", avail_ts + 1),
            )
            .await?;

            assert_json_endpoint(&http, api_port, "node/stake-table/current").await?;
            assert_json_endpoint(&http, api_port, "node/stake-table/1").await?;
            assert_json_endpoint(&http, api_port, "node/da-stake-table/current").await?;
            assert_json_endpoint(&http, api_port, "node/da-stake-table/1").await?;

            assert_json_endpoint(&http, api_port, "node/validators/1").await?;
            assert_json_endpoint(&http, api_port, "node/all-validators/1/0/100").await?;

            assert_json_endpoint(&http, api_port, "node/participation/proposal/current").await?;
            assert_json_endpoint(&http, api_port, "node/participation/proposal/1").await?;
            assert_json_endpoint(&http, api_port, "node/participation/vote/current").await?;
            assert_json_endpoint(&http, api_port, "node/participation/vote/1").await?;

            assert_json_endpoint(&http, api_port, "node/block-reward").await?;
            assert_json_endpoint(&http, api_port, "node/block-reward/epoch/1").await?;

            assert_json_endpoint(&http, api_port, "node/oldest-block").await?;
            assert_json_endpoint(&http, api_port, "node/oldest-leaf").await?;

            // Catchup endpoints. View number and height for in-memory state aren't readily
            // available after stopping consensus, so we check error semantics on
            // intentionally invalid lookups and the deprecated routes.
            let decided_view = decided_leaf.view_number().u64();
            assert_json_endpoint(
                &http,
                api_port,
                &format!("catchup/{height}/{decided_view}/blocks"),
            )
            .await?;
            // chain-config: a malformed TaggedBase64 commitment (bad checksum) parses-fails
            // on the request path and yields 400.
            assert_endpoint_status(
                &http,
                api_port,
                "catchup/chain-config/CHAINCONFIG~AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                400,
            )
            .await?;
            // leafchain: undecided height returns 404 from both.
            assert_endpoint_status(&http, api_port, "catchup/999999/leafchain", 404).await?;
            // cert2: missing cert returns 404.
            assert_endpoint_status(&http, api_port, "catchup/999999/cert2", 404).await?;
            // Deprecated catchup routes still respond 404.
            assert_endpoint_status(&http, api_port, "catchup/1/reward-amounts/100/0", 404).await?;

            // Production peer-catchup posts VBS-binary bodies via http-client.
            // Exercise the bulk-account POST endpoints in that exact wire format so any
            // regression to "JSON-only body" is caught here.
            // Reuse the account sampled above; `validated_state.fee_merkle_tree` was
            // captured before the fee-paying blocks and can be empty.
            assert_post_binary(
                &http,
                api_port,
                &format!("catchup/{height}/{decided_view}/accounts"),
                &vec![fee_account],
            )
            .await?;
            // reward-accounts V1 takes a Vec<RewardAccountV1>. We send empty since the V2
            // tree may not have V1-shaped entries in this test, but the wire format is what
            // we're validating.
            assert_post_binary(
                &http,
                api_port,
                &format!("catchup/{height}/{decided_view}/reward-accounts"),
                &Vec::<espresso_types::v0_3::RewardAccountV1>::new(),
            )
            .await?;

            // State signature: missing heights should 404.
            assert_endpoint_status(&http, api_port, "state-signature/block/999999", 404).await?;

            // Error bodies for endpoints that fail via `ApiError` must keep the
            // `{"Custom":{"message","status"}}` JSON envelope that existing clients parse.
            // Availability endpoints (`availability/leaf/...`, etc.) are excluded because
            // they use per-endpoint error variants like `{"FetchLeaf":{...}}`; their status
            // codes are still checked by `assert_endpoint_status` above.
            assert_error_body(&http, api_port, "catchup/999999/cert2", 404).await?;
            assert_error_body(&http, api_port, "catchup/999999/leafchain", 404).await?;
            assert_error_body(&http, api_port, "state-signature/block/999999", 404).await?;

            // Explorer endpoints.
            assert_json_endpoint(&http, api_port, "explorer/explorer-summary").await?;
            assert_json_endpoint(&http, api_port, &format!("explorer/block/{avail_block}")).await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("explorer/block/hash/{block_hash}"),
            )
            .await?;
            assert_json_endpoint(&http, api_port, "explorer/blocks/latest/10").await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("explorer/blocks/{avail_block}/10"),
            )
            .await?;
            assert_json_endpoint(&http, api_port, "explorer/transactions/latest/10").await?;

            // Light-client endpoints. Use the same block we used for availability tests.
            assert_json_endpoint(&http, api_port, &format!("light-client/leaf/{avail_block}"))
                .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("light-client/leaf/hash/{leaf_hash}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("light-client/payload/{avail_block}"),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!("light-client/payload/{avail_block}/{}", avail_block + 1),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!(
                    "light-client/namespace/{avail_block}/{}",
                    u64::from(avail_ns)
                ),
            )
            .await?;
            assert_json_endpoint(
                &http,
                api_port,
                &format!(
                    "light-client/namespace/{avail_block}/{}/{}",
                    avail_block + 1,
                    u64::from(avail_ns)
                ),
            )
            .await?;

            // Regression: an oversized range on the plural namespaces route must return
            // 400 Bad Request (the status carried by the query-service error), not 500.
            let encoded_ns = tagged_base64::TaggedBase64::new(
                ::light_client::client::NAMESPACES_PARAM_TAG,
                &serde_json::to_vec(&vec![u64::from(avail_ns)])?,
            )?;
            assert_endpoint_status(
                &http,
                api_port,
                &format!(
                    "light-client/namespaces/{avail_block}/{}/{encoded_ns}",
                    avail_block + 200
                ),
                400,
            )
            .await?;

            // Token endpoints.
            assert_json_endpoint(&http, api_port, "token/total-minted-supply").await?;
            assert_json_endpoint(&http, api_port, "token/circulating-supply").await?;
            assert_json_endpoint(&http, api_port, "token/circulating-supply-ethereum").await?;
            assert_json_endpoint(&http, api_port, "token/total-issued-supply").await?;
            assert_json_endpoint(&http, api_port, "token/total-reward-distributed").await?;

            // HTTP status codes for common failure cases that clients depend on.

            // Requesting a leaf far ahead of the chain tip times out and returns
            // 404 Not Found.
            assert_endpoint_status(&http, api_port, "availability/leaf/999999", 404).await?;

            // Requesting a block range that exceeds the per-request limit
            // returns 400 Bad Request.
            assert_endpoint_status(
                &http,
                api_port,
                &format!("availability/block/{avail_block}/{}", avail_block + 200),
                400,
            )
            .await?;

            // Requesting a namespace proof range that exceeds the limit also
            // returns 400 Bad Request.
            assert_endpoint_status(
                &http,
                api_port,
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

    let client: Client<ClientErr, StaticVersion<0, 1>> =
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

    assert_matches!(err, ClientErr { status, .. } if
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
