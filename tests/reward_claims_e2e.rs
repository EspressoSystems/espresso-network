use std::{collections::HashMap, time::Duration};

use alloy::{
    network::EthereumWallet,
    node_bindings::Anvil,
    primitives::U256,
    providers::{Provider, ProviderBuilder, WalletProvider},
    rpc::client::RpcClient,
};
use espresso_contract_deployer::{build_signer, Contract};
use espresso_types::{DrbAndHeaderUpgradeVersion, L1ClientOptions, SeqTypes, SequencerVersions};
use hotshot_contract_adapter::{
    reward::RewardClaimInput,
    sol_types::{EspTokenV2, LightClientV3, RewardClaim},
    stake_table::StakeTableContractVersion,
};
use hotshot_query_service::data_source::SqlDataSource;
use hotshot_state_prover::{v3::service::run_prover_once, StateProverConfig};
use hotshot_types::{
    stake_table::{one_honest_threshold, HSStakeTable},
    utils::epoch_from_block_number,
};
use portpicker::pick_unused_port;
use sequencer::{
    api::{
        data_source::testing::TestableSequencerDataSource,
        options,
        test_helpers::{TestNetwork, TestNetworkConfigBuilder, STAKE_TABLE_CAPACITY_FOR_TEST},
    },
    state_signature::relay_server::{run_relay_server_with_state, StateRelayServerState},
    testing::{wait_for_epochs, TestConfigBuilder},
    SequencerApiVersion,
};
use staking_cli::demo::DelegationConfig;
use tokio::spawn;
use url::Url;
use vbs::version::StaticVersionType;

type ConsensusVersion = SequencerVersions<DrbAndHeaderUpgradeVersion, DrbAndHeaderUpgradeVersion>;

const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";
const BLOCKS_PER_EPOCH: u64 = 7;
const PROVER_UPDATE_INTERVAL: Duration = Duration::from_secs(60);
const RETRY_INTERVAL: Duration = Duration::from_secs(2);

#[test_log::test(tokio::test)]
async fn test_reward_claims_e2e() -> anyhow::Result<()> {
    // Finalize blocks immediately to ensure we have a finalized stake table on L1 for consensus.
    let anvil = Anvil::new().args(["--slots-in-an-epoch", "0"]).spawn();
    let l1_url = anvil.endpoint_url();

    let relay_server_port = pick_unused_port().unwrap();
    let relay_server_url: Url = format!("http://localhost:{relay_server_port}")
        .parse()
        .unwrap();
    let sequencer_api_port = pick_unused_port().unwrap();

    let network_config = TestConfigBuilder::default()
        .epoch_height(BLOCKS_PER_EPOCH)
        .stake_table_capacity(STAKE_TABLE_CAPACITY_FOR_TEST)
        .state_relay_url(relay_server_url.clone())
        .l1_url(l1_url.clone())
        .l1_opt(L1ClientOptions::default())
        .build();
    let epoch_start_block = network_config.hotshot_config().epoch_start_block;

    let initial_stake_table: HSStakeTable<SeqTypes> = network_config
        .hotshot_config()
        .known_nodes_with_stake
        .clone()
        .into();
    let initial_total_stakes = initial_stake_table.total_stakes();

    let api_options = options::Options::with_port(sequencer_api_port)
        .config(Default::default())
        .catchup(Default::default());

    const NUM_NODES: usize = 2;
    let storage =
        futures::future::join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource<SeqTypes, _> as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(&storage[0], api_options))
        .network_config(network_config)
        .persistences(persistence.clone())
        .pos_hook::<ConsensusVersion>(DelegationConfig::default(), StakeTableContractVersion::V2)
        .await?
        .build();

    println!("Starting Espresso TestNetwork with {} nodes...", NUM_NODES);
    let network = TestNetwork::new(config, ConsensusVersion::new()).await;
    println!("TestNetwork started successfully");

    let contracts = network.contracts.unwrap();

    let lc_addr = contracts.address(Contract::LightClientProxy).unwrap();
    let token_addr = contracts.address(Contract::EspTokenProxy).unwrap();
    let reward_claim_addr = contracts.address(Contract::RewardClaimProxy).unwrap();

    // Start the relay server
    let first_epoch = epoch_from_block_number(epoch_start_block, BLOCKS_PER_EPOCH);
    let mut thresholds = HashMap::new();
    thresholds.insert(first_epoch, one_honest_threshold(initial_total_stakes));

    let genesis_known_nodes: HashMap<_, _> = initial_stake_table
        .iter()
        .map(|config| {
            (
                config.state_ver_key.clone(),
                config.stake_table_entry.stake_amount,
            )
        })
        .collect();
    let mut known_nodes = HashMap::new();
    known_nodes.insert(first_epoch, genesis_known_nodes);

    let relay_state = StateRelayServerState::new(
        Url::parse(&format!("http://localhost:{sequencer_api_port}")).unwrap(),
    );

    println!("Starting relay server on port {}...", relay_server_port);
    let _relay_server_handle = spawn(run_relay_server_with_state(
        format!("http://localhost:{relay_server_port}")
            .parse()
            .unwrap(),
        SequencerApiVersion::instance(),
        relay_state,
    ));

    let l1_rpc_client = RpcClient::new_http(l1_url.clone());
    let prover_config = StateProverConfig {
        relay_server: relay_server_url,
        update_interval: PROVER_UPDATE_INTERVAL,
        retry_interval: RETRY_INTERVAL,
        sequencer_url: Url::parse(&format!("http://localhost:{sequencer_api_port}/")).unwrap(),
        port: None, // No HTTP server needed for run_prover_once
        stake_table_capacity: STAKE_TABLE_CAPACITY_FOR_TEST,
        l1_rpc_client,
        light_client_address: lc_addr,
        signer: build_signer(TEST_MNEMONIC, 0),
        blocks_per_epoch: BLOCKS_PER_EPOCH,
        epoch_start_block,
        max_retries: 5,
        max_gas_price: None,
    };

    println!("Waiting for 3 epochs to elapse to ensure rewards accrue...");
    let mut events = network.peers[0].event_stream().await;
    wait_for_epochs(&mut events, BLOCKS_PER_EPOCH, 3).await;

    // Now run the prover once to generate and submit a proof for the current state
    println!("Running prover once to generate proof...");
    run_prover_once(prover_config, SequencerApiVersion::instance()).await?;
    println!("Prover completed successfully");

    // Create claimer wallet and provider for claiming rewards
    let claimer_provider = ProviderBuilder::new()
        .wallet(EthereumWallet::from(
            network.cfg.staking_priv_keys()[0].0.clone(),
        ))
        .connect_http(l1_url.clone());
    let claimer_address = claimer_provider.default_signer_address();

    let light_client_contract = LightClientV3::new(lc_addr, &claimer_provider);
    let finalized_state = light_client_contract.finalizedState().call().await?;

    // Use the block height from the light client contract for the reward proof
    let lc_block_height = finalized_state.blockHeight;
    let lc_view_number = finalized_state.viewNum;

    let sequencer_url: Url = format!("http://localhost:{sequencer_api_port}").parse()?;
    println!(
        "Fetching reward proof for LC block height: {}, LC view number: {}",
        lc_block_height, lc_view_number
    );

    let reward_claim_url = format!(
        "{}reward-state-v2/reward-claim-input/{}/{}",
        sequencer_url, lc_block_height, claimer_address
    );
    println!("Fetching reward claim input from: {}", reward_claim_url);

    // Retry fetching the reward claim input up to 5 times with 2 second delays
    let http_client = reqwest::Client::new();
    let mut attempt = 0;
    let claim_input = loop {
        attempt += 1;
        match http_client
            .get(&reward_claim_url)
            .header("Accept", "application/json")
            .send()
            .await
            .and_then(|r| r.error_for_status())
        {
            Ok(response) => match response.json::<RewardClaimInput>().await {
                Ok(data) => {
                    break data;
                },
                Err(e) if attempt == 5 => {
                    panic!("Failed to parse reward claim input: {}", e);
                },
                Err(_) => {},
            },
            Err(e) if attempt == 5 => {
                panic!("Request for reward claim input failed: {}", e);
            },
            Err(_) => {},
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    };

    println!(
        "Reward claim input received: lifetime_rewards={}",
        claim_input.lifetime_rewards
    );

    if claim_input.lifetime_rewards == U256::ZERO {
        panic!("Reward balance is zero for delegator {claimer_address}");
    }
    println!("Validator reward balance: {}", claim_input.lifetime_rewards);

    // check Eth balance of claimer
    let eth_balance = claimer_provider.get_balance(claimer_address).await?;
    println!("Eth balance of claimer {claimer_address}: {eth_balance} wei");

    let reward_claim_contract = RewardClaim::new(reward_claim_addr, &claimer_provider);
    let esp_token_contract = EspTokenV2::new(token_addr, &claimer_provider);

    let balance_before = esp_token_contract.balanceOf(claimer_address).call().await?;
    println!("ESP token balance before claim: {balance_before}");

    println!("Attempting to claim with invalid proof");
    let auth_data_bytes: alloy::primitives::Bytes = claim_input.auth_data.clone().into();
    let mut invalid_auth_data_bytes = auth_data_bytes.to_vec();
    // Corrupt the auth data by changing a byte
    invalid_auth_data_bytes[0] = invalid_auth_data_bytes[0].wrapping_add(1);

    let invalid_proof_result = reward_claim_contract
        .claimRewards(claim_input.lifetime_rewards, invalid_auth_data_bytes.into())
        .call()
        .await;
    assert!(invalid_proof_result.is_err());

    println!("Attempting to claim with invalid balance");
    let invalid_balance = claim_input.lifetime_rewards + U256::from(1);

    let invalid_balance_result = reward_claim_contract
        .claimRewards(invalid_balance, claim_input.auth_data.clone().into())
        .call()
        .await;
    assert!(invalid_balance_result.is_err());

    // check that we can claim
    reward_claim_contract
        .claimRewards(
            claim_input.lifetime_rewards,
            claim_input.auth_data.clone().into(),
        )
        .call()
        .await?;

    println!("Attempting to claim rewards with valid proof");
    let pending = reward_claim_contract
        .claimRewards(
            claim_input.lifetime_rewards,
            claim_input.auth_data.clone().into(),
        )
        .send()
        .await?;
    println!("pending tx: {:?}", pending);
    let claim_receipt = pending.get_receipt().await?;
    assert!(claim_receipt.status(), "Valid claim should succeed");
    println!("Successful claim - Gas used: {}", claim_receipt.gas_used);

    // Check we got a reward claim event
    let log = claim_receipt
        .decoded_log::<RewardClaim::RewardsClaimed>()
        .unwrap();
    println!("Emitted event: {:?}", log);

    let balance_after = esp_token_contract.balanceOf(claimer_address).call().await?;
    println!("ESP token balance after claim: {balance_after}");
    assert_eq!(
        balance_after,
        balance_before + claim_input.lifetime_rewards,
        "ESP token balance did not increase correctly"
    );

    println!("Attempting to double-claim rewards");
    let double_claim_result = reward_claim_contract
        .claimRewards(claim_input.lifetime_rewards, claim_input.auth_data.into())
        .send()
        .await;
    assert!(double_claim_result.is_err());

    println!("All reward claim tests passed successfully!");
    Ok(())
}
