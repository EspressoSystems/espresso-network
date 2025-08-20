use std::{collections::HashMap, time::Duration};

use alloy::{
    network::EthereumWallet,
    node_bindings::Anvil,
    primitives::{FixedBytes, U256},
    providers::{Provider, ProviderBuilder, WalletProvider},
    rpc::client::RpcClient,
    signers::local::{coins_bip39::English, MnemonicBuilder},
};
use espresso_contract_deployer::{
    builder::DeployerArgsBuilder, network_config::light_client_genesis_from_stake_table, Contract,
    Contracts,
};
use espresso_types::{
    v0_4::{ChainConfig, RewardAccountQueryDataV2},
    DrbAndHeaderUpgradeVersion, L1ClientOptions, SeqTypes, SequencerVersions, ValidatedState,
};
use futures::StreamExt;
use hotshot_contract_adapter::sol_types::{
    AccruedRewardsProofSol, EspTokenV2, LightClientV3, RewardClaim,
};
use hotshot_query_service::data_source::SqlDataSource;
use hotshot_state_prover::{v3::service::run_prover_once, StateProverConfig};
use hotshot_types::{
    event::EventType,
    stake_table::{one_honest_threshold, HSStakeTable},
    traits::node_implementation::ConsensusTime,
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
const EPOCH_HEIGHT: u64 = 7;
const MAX_BLOCK_SIZE: u64 = 1000000;
const PROVER_UPDATE_INTERVAL: Duration = Duration::from_secs(60);
const RETRY_INTERVAL: Duration = Duration::from_secs(2);
const INITIAL_TOKEN_SUPPLY: u64 = 3_590_000_000u64;
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

    println!("Deploying L1 contracts using deploy builder...");

    // Use DeployerArgsBuilder to deploy all contracts at once
    let exit_escrow_period = U256::from(300); // 300 seconds
    let args = DeployerArgsBuilder::default()
        .deployer(provider.clone())
        .mock_light_client(true)
        .genesis_lc_state(genesis_state.clone())
        .genesis_st_state(genesis_stake.clone())
        .blocks_per_epoch(blocks_per_epoch)
        .epoch_start_block(epoch_start_block)
        .exit_escrow_period(exit_escrow_period)
        .initial_token_supply(U256::from(INITIAL_TOKEN_SUPPLY))
        .token_name(TOKEN_NAME.to_string())
        .token_symbol(TOKEN_SYMBOL.to_string())
        .multisig_pauser(admin)
        .use_timelock_owner(false)
        .build()
        .unwrap();

    args.deploy(&mut l1_contracts, Contract::FeeContractProxy)
        .await?;
    args.deploy(&mut l1_contracts, Contract::EspTokenProxy)
        .await?;
    args.deploy(&mut l1_contracts, Contract::EspTokenV2).await?;
    args.deploy(&mut l1_contracts, Contract::LightClientProxy)
        .await?;
    args.deploy(&mut l1_contracts, Contract::LightClientV2)
        .await?;
    args.deploy(&mut l1_contracts, Contract::LightClientV3)
        .await?;
    args.deploy(&mut l1_contracts, Contract::StakeTableProxy)
        .await?;
    args.deploy(&mut l1_contracts, Contract::RewardClaimProxy)
        .await?;
    args.deploy(&mut l1_contracts, Contract::StakeTableV2)
        .await?;

    // Get contract addresses
    let lc_proxy_addr = l1_contracts
        .address(Contract::LightClientProxy)
        .expect("LightClientProxy address");
    let fee_proxy_addr = l1_contracts
        .address(Contract::FeeContractProxy)
        .expect("FeeContractProxy address");
    let token_proxy_addr = l1_contracts
        .address(Contract::EspTokenProxy)
        .expect("EspTokenProxy address");
    let stake_table_proxy_addr = l1_contracts
        .address(Contract::StakeTableProxy)
        .expect("StakeTableProxy address");
    let reward_claim_proxy_addr = l1_contracts
        .address(Contract::RewardClaimProxy)
        .expect("RewardClaimProxy address");

    println!("Light Client proxy deployed at: {}", lc_proxy_addr);
    println!("Fee Contract proxy deployed at: {}", fee_proxy_addr);
    println!("ESP Token proxy deployed at: {}", token_proxy_addr);
    println!("Stake Table proxy deployed at: {}", stake_table_proxy_addr);
    println!("RewardClaim proxy deployed at: {}", reward_claim_proxy_addr);

    // Set up stake table with validators
    let staking_priv_keys = network_config.staking_priv_keys();
    let claimer_address = staking_priv_keys[0].0.address();
    println!("First validator address: {}", claimer_address);

    setup_stake_table_contract_for_test(
        l1_url.clone(),
        &provider,
        stake_table_proxy_addr,
        staking_priv_keys.clone(),
        DelegationConfig::default(),
    )
    .await?;
    println!("Stake table populated with validators");

    // Grant minter role to reward claim contract
    println!("Granting MINTER_ROLE to RewardClaim contract...");
    let esp_token_v2 = EspTokenV2::new(token_proxy_addr, &provider);

    let minter_role = esp_token_v2.MINTER_ROLE().call().await?._0;
    let receipt = esp_token_v2
        .grantRole(minter_role, reward_claim_proxy_addr)
        .send()
        .await?
        .get_receipt()
        .await?;
    assert!(receipt.status());

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

    // Prepare prover configuration (will run once after waiting for epochs)
    let l1_rpc_client = RpcClient::new_http(l1_url.clone());
    let prover_config = StateProverConfig {
        relay_server: relay_server_url,
        update_interval: PROVER_UPDATE_INTERVAL,
        retry_interval: RETRY_INTERVAL,
        sequencer_url: Url::parse(&format!("http://localhost:{sequencer_api_port}/")).unwrap(),
        port: None, // No HTTP server needed for run_prover_once
        stake_table_capacity: STAKE_TABLE_CAPACITY_FOR_TEST,
        l1_rpc_client,
        light_client_address: lc_proxy_addr,
        signer,
        blocks_per_epoch,
        epoch_start_block,
        max_retries: 5,
        max_gas_price: None,
    };
    println!("Prover configuration: {:?}", prover_config);

    // Wait for blocks to be produced and 3 epochs to elapse to ensure rewards accrue
    println!(
        "Waiting for 3 epochs to elapse (blocks_per_epoch = {})...",
        blocks_per_epoch
    );
    let target_block = epoch_start_block + (3 * blocks_per_epoch);
    println!("Target block for 3 epochs: {}", target_block);

    // Listen to consensus events to get the actual view number
    let mut events = network.peers[0].event_stream().await;
    let mut latest_height = 0;

    while let Some(event) = events.next().await {
        if let EventType::Decide { leaf_chain, .. } = event.event {
            for leaf_info in leaf_chain.iter() {
                let height = leaf_info.leaf.height();
                let view = leaf_info.leaf.view_number();

                if height > latest_height {
                    latest_height = height;
                    println!(
                        "Block decided - Height: {}, View: {}, Target: {}",
                        height,
                        view.u64(),
                        target_block
                    );
                }

                if height >= target_block {
                    println!(
                        "Reached target block height {}, view number: {}, 3 epochs have elapsed",
                        height,
                        view.u64()
                    );
                    break;
                }
            }

            if latest_height >= target_block {
                break;
            }
        }
    }

    // Now run the prover once to generate and submit a proof for the current state
    println!("Running prover once to generate proof...");
    run_prover_once(prover_config, SequencerApiVersion::instance()).await?;
    println!("Prover completed successfully");

    // Get the finalized state from the Light Client contract
    println!("Getting finalized state from Light Client contract...");
    let light_client_contract = LightClientV3::new(lc_proxy_addr, &provider);
    let finalized_state = light_client_contract.finalizedState().call().await?;
    let auth_root = light_client_contract.authRoot().call().await?;

    println!(
        "Light Client finalized state - Block height: {}, View: {}, Auth root: {:#x}",
        finalized_state.blockHeight, finalized_state.viewNum, auth_root._0
    );

    // Use the block height from the light client contract for the reward proof
    let lc_block_height = finalized_state.blockHeight;
    let lc_view_number = finalized_state.viewNum;

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

    let sequencer_url: Url = format!("http://localhost:{sequencer_api_port}")
        .parse()
        .unwrap();
    println!(
        "Fetching reward proof for LC block height: {}, LC view number: {}",
        lc_block_height, lc_view_number
    );

    let reward_account = format!("0x{:x}", claimer_address);
    println!(
        "Testing reward claim for validator account: {}",
        reward_account
    );

    let reward_proof_url = format!(
        "{}catchup/{}/{}/reward-account-v2/{}",
        sequencer_url, lc_block_height, lc_view_number, reward_account
    );
    println!("Fetching reward proof from: {}", reward_proof_url);

    // Retry fetching the reward proof up to 5 times with 2 second delays
    let http_client = reqwest::Client::new();
    let mut attempt = 0;
    let reward_data = loop {
        attempt += 1;
        match http_client
            .get(&reward_proof_url)
            .header("Accept", "application/json")
            .send()
            .await
            .and_then(|r| r.error_for_status())
        {
            Ok(response) => match response.json::<RewardAccountQueryDataV2>().await {
                Ok(data) => {
                    break data;
                },
                Err(e) if attempt == 5 => {
                    panic!("Failed to parse reward proof data: {}", e);
                },
                Err(_) => {},
            },
            Err(e) if attempt == 5 => {
                panic!("Request for reward proof failed: {}", e);
            },
            Err(_) => {},
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    };

    println!(
        "Reward data received: balance={}, proof account={}",
        reward_data.balance, reward_data.proof.account
    );

    if reward_data.balance == U256::ZERO {
        panic!("Reward balance is zero for validator account {reward_account}");
    }
    println!("Validator reward balance: {}", reward_data.balance);

    let proof_sol: AccruedRewardsProofSol = reward_data.proof.try_into().unwrap();

    // Create claimer wallet and provider for claiming rewards
    let claimer_signer = staking_priv_keys[0].0.clone();
    let claimer_wallet = EthereumWallet::from(claimer_signer.clone());
    let claimer_provider = ProviderBuilder::new()
        .wallet(claimer_wallet.clone())
        .on_http(l1_url.clone());

    // check Eth balance of claimer
    let eth_balance = claimer_provider.get_balance(claimer_address).await?;
    println!("Eth balance of claimer {claimer_address}: {eth_balance} wei");

    let reward_claim_contract = RewardClaim::new(reward_claim_proxy_addr, &claimer_provider);
    let esp_token_contract = EspTokenV2::new(token_proxy_addr, &claimer_provider);

    let balance_before = esp_token_contract
        .balanceOf(claimer_address)
        .call()
        .await?
        ._0;
    println!(
        "ESP token balance before claim: {} for account: {}",
        balance_before, claimer_address
    );

    let auth_root_inputs = [FixedBytes::default(); 7];

    println!("Attempting to claim rewards...");
    reward_claim_contract
        .claimRewards(reward_data.balance, proof_sol.into(), auth_root_inputs)
        .send()
        .await?
        .get_receipt()
        .await?;

    let balance_after = esp_token_contract
        .balanceOf(claimer_address)
        .call()
        .await?
        ._0;
    println!(
        "ESP token balance after claim: {} for account: {}",
        balance_after, claimer_address
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
    drop(network);
    drop(anvil);

    Ok(())
}
