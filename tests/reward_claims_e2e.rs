use std::{collections::HashMap, time::Duration};

use alloy::{
    network::EthereumWallet,
    node_bindings::Anvil,
    primitives::{FixedBytes, U256},
    providers::{ProviderBuilder, WalletProvider},
    rpc::client::RpcClient,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use client::SequencerClient;
use espresso_contract_deployer::{
    network_config::light_client_genesis_from_stake_table, Contracts,
};
use espresso_types::{
    v0_4::{ChainConfig, RewardAccountQueryDataV2},
    DrbAndHeaderUpgradeVersion, L1ClientOptions, SeqTypes, SequencerVersions, ValidatedState,
};
use hotshot_contract_adapter::sol_types::{
    AccruedRewardsProofSol, EspTokenV2, LightClientV3, RewardClaim,
};
use hotshot_query_service::data_source::SqlDataSource;
use hotshot_state_prover::{v3::service::run_prover_service, StateProverConfig};
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
    testing::TestConfigBuilder,
    SequencerApiVersion,
};
use staking_cli::demo::{setup_stake_table_contract_for_test, DelegationConfig};
use tokio::spawn;
use url::Url;
use vbs::version::StaticVersionType;

const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";
const EPOCH_HEIGHT: u64 = 10;
const MAX_BLOCK_SIZE: u64 = 1000000;
const UPDATE_INTERVAL: Duration = Duration::from_secs(10);
const RETRY_INTERVAL: Duration = Duration::from_secs(2);
const INITIAL_TOKEN_SUPPLY: U256 = U256::from_limbs([3590000000u64, 0, 0, 0]);
const TOKEN_NAME: &str = "Espresso";
const TOKEN_SYMBOL: &str = "ESP";

#[test_log::test(tokio::test)]
async fn test_reward_claims_e2e() -> anyhow::Result<()> {
    let anvil = Anvil::new()
        .args([
            "--slots-in-an-epoch",
            "0",
            "--mnemonic",
            TEST_MNEMONIC,
            "--accounts",
            "10",
            "--balance",
            "1000000",
        ])
        .spawn();
    let l1_url = anvil.endpoint_url();
    // TODO: remove, use below for external anvil, to see console.log statements
    // let l1_url = "http://localhost:8545".parse::<Url>()?;
    println!("L1 URL: {}", l1_url);

    let signer = MnemonicBuilder::<English>::default()
        .phrase(TEST_MNEMONIC)
        .index(0)
        .expect("error building wallet")
        .build()
        .expect("error opening wallet");
    let wallet = EthereumWallet::from(signer.clone());
    let provider = ProviderBuilder::new()
        .wallet(wallet.clone())
        .on_http(l1_url.clone());
    let admin = provider.default_signer_address();
    println!("Admin address: {}", admin);

    let relay_server_port = pick_unused_port().unwrap();
    let relay_server_url: Url = format!("http://localhost:{relay_server_port}")
        .parse()
        .unwrap();
    let sequencer_api_port = pick_unused_port().unwrap();

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .stake_table_capacity(STAKE_TABLE_CAPACITY_FOR_TEST)
        .state_relay_url(relay_server_url.clone())
        .l1_url(l1_url.clone())
        .l1_opt(L1ClientOptions::default())
        .build();
    let blocks_per_epoch = network_config.hotshot_config().epoch_height;
    let epoch_start_block = network_config.hotshot_config().epoch_start_block;

    let initial_stake_table: HSStakeTable<SeqTypes> = network_config
        .hotshot_config()
        .known_nodes_with_stake
        .clone()
        .into();
    let initial_total_stakes = initial_stake_table.total_stakes();
    let (genesis_state, genesis_stake) =
        light_client_genesis_from_stake_table(&initial_stake_table, STAKE_TABLE_CAPACITY_FOR_TEST)?;

    let mut l1_contracts = Contracts::new();

    println!("Deploying L1 contracts...");

    // Deploy Light Client proxy
    let lc_proxy_addr = espresso_contract_deployer::deploy_light_client_proxy(
        &provider,
        &mut l1_contracts,
        true, // use mock
        genesis_state.clone(),
        genesis_stake.clone(),
        admin,
        None, // no permissioned prover
    )
    .await?;
    println!("Light Client proxy deployed at: {}", lc_proxy_addr);

    // Upgrade to LightClientV2
    espresso_contract_deployer::upgrade_light_client_v2(
        &provider,
        &mut l1_contracts,
        true, // use mock
        blocks_per_epoch,
        epoch_start_block,
    )
    .await?;
    println!("Light Client upgraded to V2");

    // Upgrade to LightClientV3
    espresso_contract_deployer::upgrade_light_client_v3(
        &provider,
        &mut l1_contracts,
        true, // use mock
    )
    .await?;
    println!("Light Client upgraded to V3");

    // Deploy Fee Contract proxy
    let fee_proxy_addr =
        espresso_contract_deployer::deploy_fee_contract_proxy(&provider, &mut l1_contracts, admin)
            .await?;
    println!("Fee Contract proxy deployed at: {}", fee_proxy_addr);

    // Deploy ESP Token proxy
    let token_proxy_addr = espresso_contract_deployer::deploy_token_proxy(
        &provider,
        &mut l1_contracts,
        admin,
        admin,
        INITIAL_TOKEN_SUPPLY,
        TOKEN_NAME,
        TOKEN_SYMBOL,
    )
    .await?;
    println!("ESP Token proxy deployed at: {}", token_proxy_addr);

    // Deploy Stake Table proxy
    let exit_escrow_period = U256::from(300); // 300 seconds
    let stake_table_proxy_addr = espresso_contract_deployer::deploy_stake_table_proxy(
        &provider,
        &mut l1_contracts,
        token_proxy_addr,
        lc_proxy_addr,
        exit_escrow_period,
        admin,
    )
    .await?;
    println!("Stake Table proxy deployed at: {}", stake_table_proxy_addr);

    // Set up stake table with validators
    let staking_priv_keys = network_config.staking_priv_keys();
    let first_validator_address = staking_priv_keys[0].0.address();
    println!("First validator address: {}", first_validator_address);

    setup_stake_table_contract_for_test(
        l1_url.clone(),
        &provider,
        stake_table_proxy_addr,
        staking_priv_keys,
        DelegationConfig::default(),
    )
    .await?;
    println!("Stake table populated with validators");

    // Deploy RewardClaim contract
    let reward_claim_proxy_addr = espresso_contract_deployer::deploy_reward_claim_proxy(
        &provider,
        &mut l1_contracts,
        token_proxy_addr,
        lc_proxy_addr,
        admin,
    )
    .await?;
    println!("RewardClaim proxy deployed at: {}", reward_claim_proxy_addr);

    println!("All L1 contracts deployed successfully!");

    // Set up chain config for TestNetwork
    let chain_config = ChainConfig {
        max_block_size: MAX_BLOCK_SIZE.into(),
        base_fee: 0.into(),
        stake_table_contract: Some(stake_table_proxy_addr),
        ..Default::default()
    };
    println!("Chain config: {chain_config:?}");

    let state = ValidatedState {
        chain_config: chain_config.into(),
        ..Default::default()
    };
    let states = std::array::from_fn(|_| state.clone());

    let api_options = options::Options::from(options::Http {
        port: sequencer_api_port,
        max_connections: None,
    })
    .submit(Default::default())
    .config(Default::default())
    .catchup(Default::default())
    .explorer(Default::default());

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
        .api_config(api_options)
        .network_config(network_config)
        .states(states)
        .persistences(persistence.clone())
        .build();

    // Start the TestNetwork
    println!("Starting Espresso TestNetwork with {} nodes...", NUM_NODES);
    let network = TestNetwork::new(
        config,
        SequencerVersions::<DrbAndHeaderUpgradeVersion, DrbAndHeaderUpgradeVersion>::new(),
    )
    .await;
    println!("TestNetwork started successfully");

    // Start the relay server
    let first_epoch = epoch_from_block_number(epoch_start_block, blocks_per_epoch);
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
    let relay_server_handle = spawn(run_relay_server_with_state(
        format!("http://localhost:{relay_server_port}")
            .parse()
            .unwrap(),
        SequencerApiVersion::instance(),
        relay_state,
    ));

    // Start the prover service
    let prover_port = pick_unused_port().unwrap();
    let l1_rpc_client = RpcClient::new_http(l1_url.clone());
    let prover_config = StateProverConfig {
        relay_server: relay_server_url,
        update_interval: UPDATE_INTERVAL,
        retry_interval: RETRY_INTERVAL,
        sequencer_url: Url::parse(&format!("http://localhost:{sequencer_api_port}/")).unwrap(),
        port: Some(prover_port),
        stake_table_capacity: STAKE_TABLE_CAPACITY_FOR_TEST,
        l1_rpc_client,
        light_client_address: lc_proxy_addr,
        signer,
        blocks_per_epoch,
        epoch_start_block,
        max_retries: 0,
        max_gas_price: None,
    };
    println!("Prover service configuration: {:?}", prover_config);

    println!("Starting prover service on port {}...", prover_port);
    let prover_handle = spawn(run_prover_service(
        prover_config,
        SequencerApiVersion::instance(),
    ));

    // Wait for blocks to be produced and 3 epochs to elapse to ensure rewards accrue
    println!(
        "Waiting for 3 epochs to elapse (blocks_per_epoch = {})...",
        blocks_per_epoch
    );
    let target_block = epoch_start_block + (3 * blocks_per_epoch);
    println!("Target block for 3 epochs: {}", target_block);

    // Wait and monitor progress
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let sequencer_url_str = format!("http://localhost:{sequencer_api_port}");
        let sequencer_url: Url = sequencer_url_str.parse().unwrap();
        let sequencer_client = SequencerClient::new(sequencer_url);

        match sequencer_client.get_height().await {
            Ok(current_height) => {
                println!(
                    "Current block height: {}, target: {}",
                    current_height, target_block
                );
                if current_height >= target_block {
                    println!("Reached target block height, 3 epochs have elapsed");
                    break;
                }
            },
            Err(e) => {
                println!("Failed to get block height: {}", e);
            },
        }
    }

    // Verify that the prover has submitted proofs to the Light Client
    println!("Verifying Light Client has received state updates from prover...");
    let light_client_contract = LightClientV3::new(lc_proxy_addr, &provider);

    // Wait for prover to submit at least one state update
    let mut attempts = 0;
    const MAX_ATTEMPTS: u32 = 10;

    loop {
        let finalized_state = light_client_contract.finalizedState().call().await?;
        let auth_root = light_client_contract.authRoot().call().await?;

        println!(
            "Light Client state - Block height: {}, View: {}, Auth root: {:#x}",
            finalized_state.blockHeight, finalized_state.viewNum, auth_root._0
        );

        // Check if we have received state updates (block height > genesis)
        if finalized_state.blockHeight > genesis_state.blockHeight && auth_root._0 != U256::ZERO {
            println!("Light Client has received state updates from prover!");
            break;
        }

        attempts += 1;
        if attempts >= MAX_ATTEMPTS {
            panic!("Timed out waiting for prover to submit state update");
        }

        println!(
            "Waiting for prover to submit state updates... (attempt {}/{})",
            attempts, MAX_ATTEMPTS
        );
        tokio::time::sleep(Duration::from_secs(3)).await;
    }

    println!("Reward claims E2E test setup completed successfully!");
    println!("Contract addresses:");
    println!("  Light Client proxy: {}", lc_proxy_addr);
    println!("  Fee Contract proxy: {}", fee_proxy_addr);
    println!("  ESP Token proxy: {}", token_proxy_addr);
    println!("  Stake Table proxy: {}", stake_table_proxy_addr);
    println!("  RewardClaim proxy: {}", reward_claim_proxy_addr);
    println!("Network info:");
    println!("  L1 URL: {}", l1_url);
    println!("  Sequencer API port: {}", sequencer_api_port);
    println!("  Relay server port: {}", relay_server_port);
    println!("  Prover port: {}", prover_port);

    let sequencer_url: Url = format!("http://localhost:{sequencer_api_port}")
        .parse()
        .unwrap();
    let sequencer_client = SequencerClient::new(sequencer_url.clone());
    let current_height = sequencer_client.get_height().await?;
    println!("Current block height: {}", current_height);

    // TODO: get the actual view number
    let view_number = current_height;

    let reward_account = format!("0x{:x}", first_validator_address);
    println!(
        "Testing reward claim for validator account: {}",
        reward_account
    );

    let reward_proof_url = format!(
        "{}catchup/{}/{}/reward-account-v2/{}",
        sequencer_url, current_height, view_number, reward_account
    );
    println!("Fetching reward proof from: {}", reward_proof_url);

    // sleep
    tokio::time::sleep(Duration::from_secs(300)).await;

    // XXX: this fails, we probably need to
    let http_client = reqwest::Client::new();
    let reward_data: RewardAccountQueryDataV2 = http_client
        .get(&reward_proof_url)
        .header("Accept", "application/json")
        .send()
        .await?
        .json()
        .await?;

    println!(
        "Reward data received: balance={}, proof account={}",
        reward_data.balance, reward_data.proof.account
    );

    if reward_data.balance == U256::ZERO {
        panic!(
            "Reward balance is zero for validator account {}. Expected non-zero rewards after 3 \
             epochs.",
            reward_account
        );
    }
    println!(
        "Validator has non-zero reward balance: {}",
        reward_data.balance
    );

    let proof_sol: AccruedRewardsProofSol = reward_data.proof.try_into().unwrap();
    let reward_claim_contract = RewardClaim::new(reward_claim_proxy_addr, &provider);
    let esp_token_contract = EspTokenV2::new(token_proxy_addr, &provider);

    let balance_before = esp_token_contract.balanceOf(admin).call().await?._0;
    println!(
        "ESP token balance before claim: {} for account: {}",
        balance_before, admin
    );

    let auth_root_inputs = [FixedBytes::default(); 7];

    println!("Attempting to claim rewards...");
    reward_claim_contract
        .claimRewards(
            reward_data.balance.into(),
            proof_sol.into(),
            auth_root_inputs,
        )
        .send()
        .await?
        .get_receipt()
        .await?;

    let balance_after = esp_token_contract.balanceOf(admin).call().await?._0;
    println!(
        "ESP token balance after claim: {} for account: {}",
        balance_after, admin
    );

    let expected_balance = balance_before + reward_data.balance;
    if balance_after != expected_balance {
        panic!(
            "ESP token balance did not increase correctly. Expected: {}, Actual: {}, Reward \
             amount: {}",
            expected_balance, balance_after, reward_data.balance
        );
    }

    relay_server_handle.abort();
    prover_handle.abort();
    drop(network);
    drop(anvil);

    Ok(())
}
