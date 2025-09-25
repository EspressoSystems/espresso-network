use alloy::{
    network::EthereumWallet,
    node_bindings::Anvil,
    primitives::{Address, FixedBytes, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use anyhow::Result;
use espresso_types::{
    v0::v0_4::{RewardAccountProofV2, RewardAccountV2, RewardMerkleTreeV2},
    v0_3::RewardAmount,
    v0_4::REWARD_MERKLE_TREE_V2_HEIGHT,
};
use hotshot_contract_adapter::sol_types::{LifetimeRewardsProofSol, RewardClaimPrototypeMock};
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
        println!("Running iteration {} of {} for {}-key tree", i + 1, iterations, num_keys);
        let gas_used = test_tree_helper(num_keys).await?;
        gas_measurements.push(gas_used as f64);
    }

    let mean = gas_measurements.iter().sum::<f64>() / gas_measurements.len() as f64;
    let variance = gas_measurements.iter()
        .map(|x| (x - mean).powi(2))
        .sum::<f64>() / gas_measurements.len() as f64;
    let std_dev = variance.sqrt();

    let min_gas = gas_measurements.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_gas = gas_measurements.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    println!("\n=== Gas Usage for {}-key tree ({} runs) ===", num_keys, iterations);
    println!("Gas usage: {:.1} ± {:.2} k", mean / 1000.0, std_dev / 1000.0);
    println!("Range: {:.1}k - {:.1}k", min_gas / 1000.0, max_gas / 1000.0);

    Ok(())
}

/// Tests that we can verify a proof in the solidity verifier
///
/// Show that we maintain overall compatibility with jellyfish with reasonable
/// gas cost as we develop reward claims.
async fn test_tree_helper(num_keys: usize) -> Result<u64> {
    // Start Anvil
    let anvil = Anvil::new().try_spawn()?;

    // Create wallet and provider
    let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(anvil.endpoint_url());

    // Deploy contract
    let contract = RewardClaimPrototypeMock::deploy(&provider).await?;

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
    let root_bytes: [u8; 32] = commitment.digest().as_ref().try_into().unwrap();
    // TODO: a saner way to convert commitments to FixedBytes
    let root = FixedBytes::from(root_bytes);
    println!("Tree root: {root}");

    let test_account = test_data[0].0;
    let test_amount = test_data[0].1;

    println!("Generating proof for account: {test_account}");

    let (proof, amount) =
        RewardAccountProofV2::prove(&tree, test_account.0).expect("can generate proof");
    assert_eq!(amount, test_amount.0);

    let proof_sol: LifetimeRewardsProofSol = proof.try_into()?;
    let account_sol = test_account.into();

    // Verify membership using Solidity contract
    let is_valid = contract
        .verifyAuthRootCommitment(root, account_sol, amount, proof_sol.siblings)
        .call()
        .await?;

    assert!(is_valid, "Membership proof invalid");

    let is_valid = contract
        .verifyAuthRootCommitment(
            root,
            account_sol,
            amount + U256::from(1),
            proof_sol.siblings,
        )
        .call()
        .await?;

    assert!(!is_valid, "Membership proof should be invalid");

    let gas_used = contract
        .verifyAuthRootCommitment(root, account_sol, amount, proof_sol.siblings)
        .estimate_gas()
        .await?;

    Ok(gas_used)
}
