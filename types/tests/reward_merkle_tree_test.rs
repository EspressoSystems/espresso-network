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
use hotshot_contract_adapter::sol_types::{LightClientV3Mock, RewardClaim};
use jf_merkle_tree_compat::{MerkleCommitment, MerkleTreeScheme, UniversalMerkleTreeScheme};
use rand::Rng as _;

#[test_log::test(tokio::test)]
async fn test_single_key_tree() -> Result<()> {
    run_multiple_tests(1, 1).await
}

#[test_log::test(tokio::test)]
async fn test_large_tree() -> Result<()> {
    run_multiple_tests(10_000, 1).await
}

#[ignore]
#[test_log::test(tokio::test)]
async fn test_large_tree_gas_estimate() -> Result<()> {
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
        .rpc_url(anvil.endpoint_url())
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

    args.deploy_all(&mut contracts).await?;

    let light_client_address = contracts
        .address(Contract::LightClientProxy)
        .expect("LightClientProxy deployed");
    let reward_claim_address = contracts
        .address(Contract::RewardClaimProxy)
        .expect("RewardClaimProxy deployed");

    let light_client = LightClientV3Mock::new(light_client_address, &provider);
    let reward_claim = RewardClaim::new(reward_claim_address, &provider);

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

    let commitment = tree.commitment();
    let root = commitment.digest().into();

    let auth_root_fields: [B256; 8] = [
        root,
        B256::ZERO,
        B256::ZERO,
        B256::ZERO,
        B256::ZERO,
        B256::ZERO,
        B256::ZERO,
        B256::ZERO,
    ];
    let auth_root = alloy::primitives::keccak256(auth_root_fields.abi_encode());
    let auth_root_u256 = auth_root.into();

    let receipt = light_client
        .setAuthRoot(auth_root_u256)
        .send()
        .await?
        .get_receipt()
        .await?;
    assert!(receipt.status());
    assert_eq!(light_client.authRoot().call().await?, auth_root_u256);

    let (test_account, test_amount) = test_data[0];

    let query_data: RewardAccountQueryDataV2 = RewardAccountProofV2::prove(&tree, test_account.0)
        .expect("can generate proof")
        .into();
    assert_eq!(query_data.balance, test_amount.0);

    let account_sol = test_account.into();
    let reward_claim_input = query_data.to_reward_claim_input()?;

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
