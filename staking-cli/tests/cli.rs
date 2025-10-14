use alloy::primitives::{
    utils::{format_ether, parse_ether},
    Address, U256,
};
use anyhow::Result;
use common::{base_cmd, Signer, TestSystemExt};
use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
use predicates::str;
use rand::{rngs::StdRng, SeedableRng as _};
use staking_cli::{
    demo::DelegationConfig,
    deploy::{self},
    Config,
};

use crate::deploy::TestSystem;

mod common;

#[rstest_reuse::template]
#[rstest::rstest]
#[case::v1(StakeTableContractVersion::V1)]
#[case::v2(StakeTableContractVersion::V2)]
#[tokio::test]
async fn stake_table_versions(#[case] _version: StakeTableContractVersion) {}

const TEST_MNEMONIC: &str = "wool upset allow cheap purity craft hat cute below useful reject door";

#[test_log::test]
fn test_cli_version() -> Result<()> {
    base_cmd().arg("version").assert().success();
    Ok(())
}

#[test_log::test]
fn test_cli_create_and_remove_config_file_mnemonic() -> anyhow::Result<()> {
    let tmpdir = tempfile::tempdir()?;
    let config_path = tmpdir.path().join("config.toml");

    assert!(!config_path.exists());

    base_cmd()
        .arg("-c")
        .arg(&config_path)
        .arg("init")
        .args(["--mnemonic", TEST_MNEMONIC])
        .args(["--account-index", "123"])
        .assert()
        .success();

    assert!(config_path.exists());

    let config: Config = toml::de::from_str(&std::fs::read_to_string(&config_path)?)?;
    assert_eq!(config.signer.mnemonic, Some(TEST_MNEMONIC.to_string()));
    assert_eq!(config.signer.account_index, Some(123));
    assert!(!config.signer.ledger);

    base_cmd()
        .arg("-c")
        .arg(&config_path)
        .arg("purge")
        .arg("--force")
        .assert()
        .success();

    assert!(!config_path.exists());

    Ok(())
}

#[test_log::test]
fn test_cli_create_file_ledger() -> anyhow::Result<()> {
    let tmpdir = tempfile::tempdir()?;
    let config_path = tmpdir.path().join("config.toml");

    assert!(!config_path.exists());

    base_cmd()
        .arg("-c")
        .arg(&config_path)
        .arg("init")
        .arg("--ledger")
        .args(["--account-index", "42"])
        .assert()
        .success();

    assert!(config_path.exists());

    let config: Config = toml::de::from_str(&std::fs::read_to_string(&config_path)?)?;
    assert!(config.signer.ledger);
    assert_eq!(config.signer.account_index, Some(42));

    Ok(())
}

// TODO: ideally we would test that the decoding works for all the commands
#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_contract_revert(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    let mut cmd = system.cmd(Signer::Mnemonic);

    cmd.arg("transfer")
        .arg("--to")
        .arg("0x1111111111111111111111111111111111111111")
        .arg("--amount")
        .arg(U256::MAX.to_string())
        .assert()
        .failure()
        .stderr(str::contains("ERC20InsufficientBalance"));
    Ok(())
}

#[test_log::test(rstest::rstest)]
#[tokio::test]
async fn test_cli_register_validator(
    #[values(StakeTableContractVersion::V1, StakeTableContractVersion::V2)]
    version: StakeTableContractVersion,
    #[values(Signer::Mnemonic, Signer::BrokeMnemonic)] signer: Signer,
) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    let mut cmd = system.cmd(signer);
    match signer {
        Signer::Mnemonic => {
            cmd.arg("register-validator")
                .arg("--consensus-private-key")
                .arg(system.bls_private_key_str()?)
                .arg("--state-private-key")
                .arg(system.state_private_key_str()?)
                .arg("--commission")
                .arg("12.34")
                .assert()
                .success()
                .stdout(str::contains("ValidatorRegistered"));
        },
        Signer::BrokeMnemonic => {
            cmd.arg("register-validator")
                .arg("--consensus-private-key")
                .arg(system.bls_private_key_str()?)
                .arg("--state-private-key")
                .arg(system.state_private_key_str()?)
                .arg("--commission")
                .arg("12.34")
                .assert()
                .failure()
                .stderr(str::contains("zero Ethereum balance"));
        },
        Signer::Ledger => unreachable!(),
    };

    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_update_consensus_keys(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;

    let mut rng = StdRng::from_seed([43u8; 32]);
    let (_, new_bls, new_state) = TestSystem::gen_keys(&mut rng);

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("update-consensus-keys")
        .arg("--consensus-private-key")
        .arg(new_bls.sign_key_ref().to_tagged_base64()?.to_string())
        .arg("--state-private-key")
        .arg(new_state.sign_key().to_tagged_base64()?.to_string())
        .assert()
        .success()
        .stdout(str::contains("ConsensusKeysUpdated"));
    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_cli_update_commission() -> Result<()> {
    // Only test on V2 since V1 doesn't support commission updates
    let system = TestSystem::deploy_version(StakeTableContractVersion::V2).await?;
    system.register_validator().await?;

    let new_commission = "8.5".try_into()?;
    assert_ne!(system.fetch_commission().await?, new_commission);

    system
        .cmd(Signer::Mnemonic)
        .arg("update-commission")
        .arg("--new-commission")
        .arg("8.5")
        .assert()
        .success()
        .stdout(str::contains("CommissionUpdated"));
    assert_eq!(system.fetch_commission().await?, new_commission);

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_cli_increase_commission_too_soon() -> Result<()> {
    // Only test on V2 since V1 doesn't support commission updates
    let system = TestSystem::deploy_version(StakeTableContractVersion::V2).await?;
    system.register_validator().await?;

    let old_commission = system.fetch_commission().await?;
    let basis_point = 1u64.try_into()?;
    let first_update = old_commission + basis_point;
    system
        .cmd(Signer::Mnemonic)
        .arg("update-commission")
        .arg("--new-commission")
        .arg(first_update.to_string())
        .assert()
        .success()
        .stdout(str::contains("CommissionUpdated"));
    assert_eq!(system.fetch_commission().await?, first_update);

    let interval = system.get_min_commission_increase_interval().await?;
    // Warp to just before the end of the delay
    system.anvil_increase_time(interval - U256::from(5)).await?;

    let second_update = first_update + basis_point;
    system
        .cmd(Signer::Mnemonic)
        .arg("update-commission")
        .arg("--new-commission")
        .arg(second_update.to_string())
        .assert()
        .failure()
        .stderr(str::contains("TooSoon"));
    assert_eq!(system.fetch_commission().await?, first_update);

    system.anvil_increase_time(U256::from(5)).await?;
    system
        .cmd(Signer::Mnemonic)
        .arg("update-commission")
        .arg("--new-commission")
        .arg(second_update.to_string())
        .assert()
        .success()
        .stdout(str::contains("CommissionUpdated"));
    assert_eq!(system.fetch_commission().await?, second_update);

    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_delegate(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("delegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg("123")
        .assert()
        .success()
        .stdout(str::contains("Delegated"));
    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_deregister_validator(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("deregister-validator")
        .assert()
        .success()
        .stdout(str::contains("ValidatorExit"));
    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_undelegate(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;
    let amount = "123";
    system.delegate(parse_ether(amount)?).await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("undelegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg(amount)
        .assert()
        .success()
        .stdout(str::contains("Undelegated"));
    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_claim_withdrawal(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    let amount = U256::from(123);
    system.register_validator().await?;
    system.delegate(amount).await?;
    system.undelegate(amount).await?;
    system.warp_to_unlock_time().await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("claim-withdrawal")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .assert()
        .success()
        .stdout(str::contains("Withdrawal"));
    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_claim_validator_exit(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    let amount = U256::from(123);
    system.register_validator().await?;
    system.delegate(amount).await?;
    system.deregister_validator().await?;
    system.warp_to_unlock_time().await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("claim-validator-exit")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .assert()
        .success()
        .stdout(str::contains("Withdrawal"));
    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_stake_for_demo_default_num_validators(
    #[case] version: StakeTableContractVersion,
) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("stake-for-demo").assert().success();
    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_stake_for_demo_three_validators(
    #[case] version: StakeTableContractVersion,
) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("stake-for-demo")
        .arg("--num-validators")
        .arg("3")
        .assert()
        .success();
    Ok(())
}

#[test_log::test(rstest::rstest)]
#[tokio::test]
async fn stake_for_demo_delegation_config_helper(
    #[values(StakeTableContractVersion::V1, StakeTableContractVersion::V2)]
    version: StakeTableContractVersion,
    #[values(
        DelegationConfig::EqualAmounts,
        DelegationConfig::VariableAmounts,
        DelegationConfig::MultipleDelegators
    )]
    config: DelegationConfig,
) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("stake-for-demo")
        .arg("--delegation-config")
        .arg(config.to_string())
        .assert()
        .success();
    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_approve(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    let amount = "123";

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("approve")
        .arg("--amount")
        .arg(amount)
        .assert()
        .success()
        .stdout(str::contains("Approval"));

    assert!(system.allowance(system.deployer_address).await? == parse_ether(amount)?);

    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_balance(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;

    // Check balance of account owner
    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("token-balance")
        .assert()
        .success()
        .stdout(str::contains(system.deployer_address.to_string()))
        .stdout(str::contains("3590000000.0"));

    // Check balance of other address
    let addr = "0x1111111111111111111111111111111111111111";
    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("token-balance")
        .arg("--address")
        .arg(addr)
        .assert()
        .success()
        .stdout(str::contains(addr))
        .stdout(str::contains(" 0.0"));

    Ok(())
}

// This test can be remove when the deprecated argument is removed
#[test_log::test(tokio::test)]
async fn test_deprecated_token_address_cli_arg() -> Result<()> {
    let system = TestSystem::deploy().await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    // Add the deprecated --token_address argument
    cmd.arg("--token-address").arg(system.token.to_string());
    cmd.arg("token-balance").assert().success();
    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_allowance(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;

    // Check allowance of account owner
    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("token-allowance")
        .assert()
        .success()
        .stdout(str::contains(system.deployer_address.to_string()))
        .stdout(str::contains(format_ether(system.approval_amount)));

    // Check allowance of other address
    let addr = "0x1111111111111111111111111111111111111111".to_string();
    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("token-allowance")
        .arg("--owner")
        .arg(&addr)
        .assert()
        .success()
        .stdout(str::contains(&addr))
        .stdout(str::contains(" 0.0"));

    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_transfer(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    let addr = "0x1111111111111111111111111111111111111111".parse::<Address>()?;
    let amount = parse_ether("0.123")?;
    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("transfer")
        .arg("--to")
        .arg(addr.to_string())
        .arg("--amount")
        .arg(format_ether(amount))
        .assert()
        .success()
        .stdout(str::contains("Transfer"));

    assert_eq!(system.balance(addr).await?, amount);

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_cli_claim_reward() -> Result<()> {
    let system = TestSystem::deploy_version(StakeTableContractVersion::V2).await?;
    let reward_balance = U256::from(1000000);

    let balance_before = system.balance(system.deployer_address).await?;

    let espresso_url = system.setup_reward_claim_mock(reward_balance).await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("--espresso-url")
        .arg(espresso_url.to_string())
        .arg("claim-reward")
        .assert()
        .success()
        .stdout(str::contains("RewardsClaimed"));

    let balance_after = system.balance(system.deployer_address).await?;

    assert_eq!(balance_after, balance_before + reward_balance,);

    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_stake_table_full(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;

    let amount = parse_ether("0.123")?;
    system.delegate(amount).await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("stake-table")
        .assert()
        .success()
        .stdout(str::contains("BLS_VER_KEY~ksjrqSN9jEvKOeCNNySv9Gcg7UjZvROpOm99zHov8SgxfzhLyno8IUfE1nxOBhGnajBmeTbchVI94ZUg5VLgAT2DBKXBnIC6bY9y2FBaK1wPpIQVgx99-fAzWqbweMsiXKFYwiT-0yQjJBXkWyhtCuTHT4l3CRok68mkobI09q0c comm=12.34 % stake=0.123000000000000000 ESP"))
        .stdout(str::contains(" - Delegator 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266: stake=0.123000000000000000 ESP"));

    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_stake_table_compact(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;

    let amount = parse_ether("0.123")?;
    system.delegate(amount).await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("stake-table")
        .arg("--compact")
        .assert()
        .success()
        .stdout(str::contains(
            "BLS_VER_KEY~ksjrqSN9jEvKOeCNNySv9Gcg7UjZ.. comm=12.34 % stake=0.123000000000000000 \
             ESP",
        ))
        .stdout(str::contains(
            " - Delegator 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266: stake=0.123000000000000000 \
             ESP",
        ));

    Ok(())
}

async fn address_from_cli(system: &TestSystem) -> Result<Address> {
    println!("Unlock the ledger");
    let stdout = system
        .cmd(Signer::Ledger)
        .arg("account")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    Ok(String::from_utf8(stdout)?
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .expect("non-empty line")
        .parse()?)
}

/// This test requires a ledger device to be connected and unlocked.
/// cargo test -p staking-cli -- --ignored --nocapture transfer_ledger
#[ignore]
#[test_log::test(tokio::test)]
async fn test_cli_transfer_ledger() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let address = address_from_cli(&system).await?;

    let amount = parse_ether("0.123")?;
    system.transfer_eth(address, amount).await?;
    system.transfer(address, amount).await?;

    // Assume the ledger is unlocked and the Ethereum app remains open
    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("transfer")
        .arg("--to")
        .arg(address.to_string())
        .arg("--amount")
        .arg(format_ether(amount))
        .assert()
        .success()
        .stdout(str::contains("Transfer"));

    // Make a token transfer with the ledger
    println!("Sign the transaction in the ledger");
    let addr = "0x1111111111111111111111111111111111111111".parse::<Address>()?;
    let mut cmd = system.cmd(Signer::Ledger);
    cmd.arg("transfer")
        .arg("--to")
        .arg(addr.to_string())
        .arg("--amount")
        .arg(format_ether(amount))
        .assert()
        .success()
        .stdout(str::contains("Transfer"));

    assert_eq!(system.balance(addr).await?, amount);

    Ok(())
}

/// This test requires a ledger device to be connected and unlocked.
/// cargo test -p staking-cli -- --ignored --nocapture delegate_ledger
#[ignore]
#[test_log::test(tokio::test)]
async fn test_cli_delegate_ledger() -> Result<()> {
    let system = TestSystem::deploy().await?;
    system.register_validator().await?;
    let address = address_from_cli(&system).await?;

    let amount = parse_ether("0.123")?;
    system.transfer_eth(address, amount).await?;
    system.transfer(address, amount).await?;

    // Assume the ledger is unlocked and the Ethereum app remains open
    println!("Sign the transaction in the ledger");
    let mut cmd = system.cmd(Signer::Ledger);
    cmd.arg("approve")
        .arg("--amount")
        .arg(format_ether(amount))
        .assert()
        .success()
        .stdout(str::contains("Approval"));

    println!("Sign the transaction in the ledger (again)");
    let mut cmd = system.cmd(Signer::Ledger);
    cmd.arg("delegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg(format_ether(amount))
        .assert()
        .success()
        .stdout(str::contains("Delegated"));

    Ok(())
}

#[test_log::test(rstest_reuse::apply(stake_table_versions))]
async fn test_cli_all_operations_manual_inspect(
    #[case] version: StakeTableContractVersion,
) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;

    let output = system
        .cmd(Signer::Mnemonic)
        .arg("register-validator")
        .arg("--consensus-private-key")
        .arg(system.bls_private_key_str()?)
        .arg("--state-private-key")
        .arg(system.state_private_key_str()?)
        .arg("--commission")
        .arg("12.34")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("{}", String::from_utf8_lossy(&output));

    let addr = "0x1111111111111111111111111111111111111111";
    let output = system
        .cmd(Signer::Mnemonic)
        .arg("transfer")
        .arg("--to")
        .arg(addr)
        .arg("--amount")
        .arg("100")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("{}", String::from_utf8_lossy(&output));

    let output = system
        .cmd(Signer::Mnemonic)
        .arg("approve")
        .arg("--amount")
        .arg("1000")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("{}", String::from_utf8_lossy(&output));

    let output = system
        .cmd(Signer::Mnemonic)
        .arg("delegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg("500")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("{}", String::from_utf8_lossy(&output));

    if matches!(version, StakeTableContractVersion::V2) {
        let output = system
            .cmd(Signer::Mnemonic)
            .arg("update-commission")
            .arg("--new-commission")
            .arg("15.5")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        println!("{}", String::from_utf8_lossy(&output));
    }

    let mut rng = StdRng::from_seed([99u8; 32]);
    let (_, new_bls, new_state) = TestSystem::gen_keys(&mut rng);
    let output = system
        .cmd(Signer::Mnemonic)
        .arg("update-consensus-keys")
        .arg("--consensus-private-key")
        .arg(new_bls.sign_key_ref().to_tagged_base64()?.to_string())
        .arg("--state-private-key")
        .arg(new_state.sign_key().to_tagged_base64()?.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("{}", String::from_utf8_lossy(&output));

    let output = system
        .cmd(Signer::Mnemonic)
        .arg("undelegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg("200")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("{}", String::from_utf8_lossy(&output));

    system.warp_to_unlock_time().await?;

    let output = system
        .cmd(Signer::Mnemonic)
        .arg("claim-withdrawal")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("{}", String::from_utf8_lossy(&output));

    let output = system
        .cmd(Signer::Mnemonic)
        .arg("deregister-validator")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("{}", String::from_utf8_lossy(&output));

    system.warp_to_unlock_time().await?;

    let output = system
        .cmd(Signer::Mnemonic)
        .arg("claim-validator-exit")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("{}", String::from_utf8_lossy(&output));

    Ok(())
}
