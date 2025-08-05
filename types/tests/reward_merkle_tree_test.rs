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
use hotshot_contract_adapter::{sol_types::RewardClaimPrototypeMock, ToSol, TryToSol};
use jf_merkle_tree::{MerkleCommitment, MerkleTreeScheme, UniversalMerkleTreeScheme};
use rand::Rng as _;

#[test_log::test(tokio::test)]
async fn test_single_key_tree() -> Result<()> {
    test_tree_helper(1).await
}

#[test_log::test(tokio::test)]
async fn test_large_tree() -> Result<()> {
    test_tree_helper(1000).await
}

/// Tests that we can verify a proof in the solidity verifier
///
/// Show that we maintain overall compatibility with jellyfish with reasonable
/// gas cost as we develop reward claims.
async fn test_tree_helper(num_keys: usize) -> Result<()> {
    // Start Anvil
    let anvil = Anvil::new().try_spawn()?;

    // Create wallet and provider
    let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
    let wallet = EthereumWallet::from(signer);
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(anvil.endpoint_url());

    // Deploy contract
    let contract = RewardClaimPrototypeMock::deploy(&provider).await?;

    println!(
        "Testing {num_keys}-key RewardMerkleTreeV2 at: {}",
        contract.address()
    );

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
    // TODO: ToSol impl for the root
    let root = FixedBytes::from(root_bytes);
    println!("Tree root: {root}");

    let test_account = &test_data[0].0;
    let test_amount = &test_data[0].1;

    println!("Generating proof for account: {test_account}");

    let (proof, amount) =
        RewardAccountProofV2::prove(&tree, test_account.0).expect("can generate proof");
    assert_eq!(amount, test_amount.0);

    // Convert proof to Solidity format
    let proof_sol = proof.try_to_sol()?;
    // Convert account and amount to Solidity format using ToSol trait
    let account_sol = test_account.to_sol();

    // Verify membership using Solidity contract
    let is_valid = contract
        .verifyRewardClaim(root, account_sol, amount, proof_sol.clone())
        .call()
        .await?;

    assert!(is_valid._0, "Membership proof invalid");

    let is_valid = contract
        .verifyRewardClaim(root, account_sol, amount + U256::from(1), proof_sol.clone())
        .call()
        .await?;

    assert!(!is_valid._0, "Membership proof should be invalid");

    // Check gas usage
    let gas_used = contract
        .verifyRewardClaim(root, account_sol, amount, proof_sol)
        .estimate_gas()
        .await?;
    println!("Gas used for membership verification: {gas_used}");

    Ok(())
}
