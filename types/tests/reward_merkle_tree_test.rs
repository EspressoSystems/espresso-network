use alloy::{
    network::EthereumWallet,
    node_bindings::Anvil,
    primitives::{Address, B256, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    sol_types::SolValue as _,
};
use anyhow::Result;
use espresso_contract_deployer::{
    builder::DeployerArgsBuilder, network_config::light_client_genesis_from_stake_table, Contract,
    Contracts,
};
use espresso_types::{
    v0::v0_4::{RewardAccountProofV2, RewardAccountV2, RewardMerkleTreeV2},
    v0_3::RewardAmount,
    v0_4::{RewardAccountQueryDataV2, REWARD_MERKLE_TREE_V2_HEIGHT},
};
use jf_merkle_tree_compat::{MerkleCommitment, MerkleTreeScheme, UniversalMerkleTreeScheme};
use rand::Rng as _;

#[test_log::test(tokio::test)]
async fn test_single_key_tree() -> Result<()> {
    run_multiple_tests(1, 10).await
}

#[test_log::test(tokio::test)]
async fn test_large_tree() -> Result<()> {
    run_multiple_tests(10_000, 10).await
}

async fn run_multiple_tests(num_keys: usize, iterations: usize) -> Result<()> {
    let mut gas_measurements = Vec::new();

    for i in 0..iterations {
        println!(
            "Running iteration {} of {} for {}-key tree",
            i + 1,
            iterations,
            num_keys
        );
        let gas_used = test_tree_helper(num_keys).await?;
        gas_measurements.push(gas_used as f64);
    }

    let mean = gas_measurements.iter().sum::<f64>() / gas_measurements.len() as f64;
    let variance = gas_measurements
        .iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>()
        / gas_measurements.len() as f64;
    let std_dev = variance.sqrt();

    let min_gas = gas_measurements
        .iter()
        .fold(f64::INFINITY, |a, &b| a.min(b));
    let max_gas = gas_measurements
        .iter()
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    println!(
        "\n=== Gas Usage for {}-key tree ({} runs) ===",
        num_keys, iterations
    );
    println!(
        "Gas usage: {:.1} ± {:.2} k",
        mean / 1000.0,
        std_dev / 1000.0
    );
    println!("Range: {:.1}k - {:.1}k", min_gas / 1000.0, max_gas / 1000.0);

    Ok(())
}

/// Tests that we can verify a proof in the solidity verifier
///
/// Show that we maintain overall compatibility with jellyfish with reasonable
/// gas cost as we develop reward claims.
async fn test_tree_helper(num_keys: usize) -> Result<u64> {
    let anvil = Anvil::new().try_spawn()?;
    let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
    let wallet = EthereumWallet::from(signer);
    let deployer_address = wallet.default_signer().address();
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(anvil.endpoint_url());

    let (genesis_state, genesis_stake) =
        light_client_genesis_from_stake_table(&Default::default(), 10).unwrap();

    let mut contracts = Contracts::new();
    let args = DeployerArgsBuilder::default()
        .deployer(provider.clone())
        .mock_light_client(true)
        .genesis_lc_state(genesis_state)
        .genesis_st_state(genesis_stake)
        .blocks_per_epoch(100)
        .epoch_start_block(1)
        .multisig_pauser(deployer_address)
        .exit_escrow_period(U256::from(250))
        .token_name("Espresso".to_string())
        .token_symbol("ESP".to_string())
        .initial_token_supply(U256::from(3590000000u64))
        .ops_timelock_delay(U256::from(0))
        .ops_timelock_admin(deployer_address)
        .ops_timelock_proposers(vec![deployer_address])
        .ops_timelock_executors(vec![deployer_address])
        .safe_exit_timelock_delay(U256::from(10))
        .safe_exit_timelock_admin(deployer_address)
        .safe_exit_timelock_proposers(vec![deployer_address])
        .safe_exit_timelock_executors(vec![deployer_address])
        .use_timelock_owner(false)
        .build()
        .unwrap();

    // Deploy all contracts including RewardClaim
    args.deploy_all(&mut contracts).await?;

    // Upgrade to LightClientV3
    use espresso_contract_deployer::{upgrade_esp_token_v2, upgrade_light_client_v3};
    upgrade_light_client_v3(&provider, &mut contracts, true).await?;

    // Upgrade to EspTokenV2
    upgrade_esp_token_v2(&provider, &mut contracts).await?;

    let light_client_address = contracts
        .address(Contract::LightClientProxy)
        .expect("LightClientProxy deployed");
    let reward_claim_address = contracts
        .address(Contract::RewardClaimProxy)
        .expect("RewardClaimProxy deployed");

    // Create contract instances
    use hotshot_contract_adapter::sol_types::{EspTokenV2, LightClientV3Mock, RewardClaim};
    let light_client = LightClientV3Mock::new(light_client_address, &provider);
    let reward_claim = RewardClaim::new(reward_claim_address, &provider);

    // Grant minter role to RewardClaim contract on EspTokenV2
    let esp_token_address = contracts
        .address(Contract::EspTokenProxy)
        .expect("EspTokenProxy deployed");
    let esp_token = EspTokenV2::new(esp_token_address, &provider);

    // Get the MINTER_ROLE hash (should be keccak256("MINTER_ROLE"))
    let minter_role = esp_token.MINTER_ROLE().call().await?;

    // Check if we have admin privileges first
    let default_admin_role = esp_token.DEFAULT_ADMIN_ROLE().call().await?;
    let has_admin_role = esp_token
        .hasRole(default_admin_role, deployer_address)
        .call()
        .await?;
    println!("Deployer has admin role: {has_admin_role}");

    if !has_admin_role {
        println!(
            "Warning: Deployer doesn't have admin role, trying to grant minter role anyway..."
        );
    }

    // Grant minter role to RewardClaim contract
    let receipt = esp_token
        .grantRole(minter_role, reward_claim_address)
        .send()
        .await?
        .get_receipt()
        .await?;

    if !receipt.status() {
        println!("Failed to grant minter role to RewardClaim");
        return Err(anyhow::anyhow!("Failed to grant minter role"));
    }
    println!("Successfully granted minter role to RewardClaim");

    let mut tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);

    let mut test_data = Vec::new();
    for _i in 0..num_keys {
        let key = RewardAccountV2::from(Address::random());
        // Use u64 values to ensure they fit in Solidity proofs
        let value = RewardAmount::from(rand::thread_rng().gen::<u64>());
        test_data.push((key, value));
    }

    for (account, amount) in &test_data {
        tree.update(*account, *amount).unwrap();
    }

    // Get the tree root
    let commitment = tree.commitment();
    let root = commitment.digest().into();
    println!("Tree root: {root}");

    // Set the authRoot in the light client mock
    // The authRoot should be keccak256 of 8 fields: [merkle_tree_root, 0, 0, 0, 0, 0, 0, 0]
    let auth_root_fields: [B256; 8] = [
        root,               // merkle tree root
        Default::default(), // zero
        Default::default(), // zero
        Default::default(), // zero
        Default::default(), // zero
        Default::default(), // zero
        Default::default(), // zero
        Default::default(), // zero
    ];
    let auth_root_hash = alloy::primitives::keccak256(auth_root_fields.abi_encode());
    let auth_root_u256 = U256::from_be_bytes(auth_root_hash.0);

    let receipt = light_client
        .setAuthRoot(auth_root_u256)
        .send()
        .await?
        .get_receipt()
        .await?;
    assert!(receipt.status());

    let test_account = test_data[0].0;
    let test_amount = test_data[0].1;

    println!("Generating proof for account: {test_account}");

    let query_data: RewardAccountQueryDataV2 = RewardAccountProofV2::prove(&tree, test_account.0)
        .expect("can generate proof")
        .into();
    assert_eq!(query_data.balance, test_amount.0);

    let account_sol = test_account.into();
    let reward_claim_input = query_data.to_reward_claim_input(Default::default())?;

    println!("Attempting to claim rewards for account: {test_account}");
    println!("Amount: {test_amount}");

    // First, let's verify the minter role was granted correctly
    let has_minter_role = esp_token
        .hasRole(minter_role, reward_claim_address)
        .call()
        .await?;
    println!("RewardClaim has minter role: {has_minter_role}");
    assert!(has_minter_role, "RewardClaim should have minter role");

    // Verify the auth root was set correctly
    let current_auth_root = light_client.authRoot().call().await?;
    println!("Current auth root in light client: {current_auth_root:#x}");
    println!("Expected auth root: {auth_root_u256:#x}");
    assert_eq!(current_auth_root, auth_root_u256, "Auth roots should match");

    // Estimate gas for the claim transaction instead of sending it
    println!("Estimating gas for claim transaction...");
    let gas_estimate = reward_claim
        .claimRewards(
            reward_claim_input.lifetime_rewards,
            reward_claim_input.auth_data.into(),
        )
        .from(account_sol)
        .estimate_gas()
        .await?;

    Ok(gas_estimate)
}
