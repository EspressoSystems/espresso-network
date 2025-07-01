use std::{
    path::PathBuf,
    process::{Command, Output, Stdio},
};

use alloy::primitives::{
    utils::{format_ether, parse_ether},
    Address, U256,
};
use anyhow::Result;
use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
use rand::{rngs::StdRng, SeedableRng as _};
use sequencer_utils::test_utils::setup_test;
use staking_cli::{demo::DelegationConfig, deploy, deploy::Signer, Config};

use crate::deploy::TestSystem;

#[rstest_reuse::template]
#[rstest::rstest]
#[case::v1(StakeTableContractVersion::V1)]
#[case::v2(StakeTableContractVersion::V2)]
#[tokio::test]
async fn stake_table_versions(#[case] _version: StakeTableContractVersion) {}

const TEST_MNEMONIC: &str = "wool upset allow cheap purity craft hat cute below useful reject door";

trait AssertSuccess {
    fn assert_success(&self) -> &Self;
}

impl AssertSuccess for Output {
    fn assert_success(&self) -> &Self {
        if !self.status.success() {
            let stderr = String::from_utf8(self.stderr.clone()).expect("stderr is utf8");
            let stdout = String::from_utf8(self.stdout.clone()).expect("stdout is utf8");
            panic!("Command failed:\nstderr: {stderr}\nstdout: {stdout}");
        }
        self
    }
}

trait AssertFailure {
    fn assert_failure(&self) -> &Self;
}

impl AssertFailure for Output {
    fn assert_failure(&self) -> &Self {
        if self.status.success() {
            let stderr = String::from_utf8(self.stderr.clone()).expect("stderr is utf8");
            let stdout = String::from_utf8(self.stdout.clone()).expect("stdout is utf8");
            panic!("Command succeeded but should have failed:\nstderr: {stderr}\nstdout: {stdout}");
        }
        self
    }
}

trait Utf8 {
    fn utf8(&self) -> String;
}

impl Utf8 for Output {
    fn utf8(&self) -> String {
        String::from_utf8(self.stdout.clone()).expect("stdout is utf8")
    }
}

trait Utf8Err {
    fn utf8_err(&self) -> String;
}

impl Utf8Err for Output {
    fn utf8_err(&self) -> String {
        String::from_utf8(self.stderr.clone()).expect("stderr is utf8")
    }
}

/// Creates a new command to run the staking-cli binary.
///
/// Will use `NEXTEST_BIN_EXE_staking-cli` if available, otherwise falls back to
/// `CARGO_BIN_EXE_staking-cli` which is set by cargo at compile time for integration tests.
fn base_cmd() -> Command {
    // From nextest docs:
    //
    // To obtain the path to a crate's executables, Cargo provides the [CARGO_BIN_EXE_<name>]
    // option to integration tests at build time. To handle target directory remapping, use the
    // value of NEXTEST_BIN_EXE_<name> at runtime. To retain compatibility with cargo test, you
    // can fall back to the value of CARGO_BIN_EXE_<name> at build time.
    let path: PathBuf = std::env::var("NEXTEST_BIN_EXE_staking-cli")
        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_staking-cli").to_string())
        .into();
    tracing::debug!("staking-cli path: {}", path.display());
    if !path.exists() {
        panic!("staking-cli binary not found at {}", path.display());
    };
    Command::new(path)
}

#[test]
fn test_cli_version() -> Result<()> {
    setup_test();
    base_cmd().arg("version").output()?.assert_success();
    Ok(())
}

#[test]
fn test_cli_create_and_remove_config_file_mnemonic() -> anyhow::Result<()> {
    setup_test();
    let tmpdir = tempfile::tempdir()?;
    let config_path = tmpdir.path().join("config.toml");

    assert!(!config_path.exists());

    base_cmd()
        .arg("-c")
        .arg(&config_path)
        .arg("init")
        .args(["--mnemonic", TEST_MNEMONIC])
        .args(["--account-index", "123"])
        .output()?
        .assert_success();

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
        .output()?
        .assert_success();

    assert!(!config_path.exists());

    Ok(())
}

#[test]
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
        .output()?
        .assert_success();

    assert!(config_path.exists());

    let config: Config = toml::de::from_str(&std::fs::read_to_string(&config_path)?)?;
    assert!(config.signer.ledger);
    assert_eq!(config.signer.account_index, Some(42));

    Ok(())
}

// TODO: ideally we would test that the decoding works for all the commands
#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_contract_revert(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);

    let output = cmd
        .arg("transfer")
        .arg("--to")
        .arg("0x1111111111111111111111111111111111111111")
        .arg("--amount")
        .arg(U256::MAX.to_string())
        .output()?
        .assert_failure()
        .utf8_err();
    assert!(output.contains("ERC20InsufficientBalance"));
    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_register_validator(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("register-validator")
        .arg("--consensus-private-key")
        .arg(
            system
                .bls_key_pair
                .sign_key_ref()
                .to_tagged_base64()?
                .to_string(),
        )
        .arg("--state-private-key")
        .arg(
            system
                .state_key_pair
                .sign_key()
                .to_tagged_base64()?
                .to_string(),
        )
        .arg("--commission")
        .arg("12.34")
        .output()?
        .assert_success();
    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_update_consensus_keys(#[case] version: StakeTableContractVersion) -> Result<()> {
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;

    let mut rng = StdRng::from_seed([43u8; 32]);
    let (_, new_bls, new_state) = TestSystem::gen_keys(&mut rng);

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("update-consensus-keys")
        .arg("--consensus-private-key")
        .arg(new_bls.sign_key_ref().to_tagged_base64()?.to_string())
        .arg("--state-private-key")
        .arg(new_state.sign_key().to_tagged_base64()?.to_string())
        .output()?
        .assert_success();
    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_delegate(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("delegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg("123")
        .output()?
        .assert_success();
    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_deregister_validator(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("deregister-validator").output()?.assert_success();
    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_undelegate(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;
    let amount = "123";
    system.delegate(parse_ether(amount)?).await?;

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("undelegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg(amount)
        .output()?
        .assert_success();
    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_claim_withdrawal(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    let amount = U256::from(123);
    system.register_validator().await?;
    system.delegate(amount).await?;
    system.undelegate(amount).await?;
    system.warp_to_unlock_time().await?;

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("claim-withdrawal")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .output()?
        .assert_success();
    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_claim_validator_exit(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    let amount = U256::from(123);
    system.register_validator().await?;
    system.delegate(amount).await?;
    system.deregister_validator().await?;
    system.warp_to_unlock_time().await?;

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("claim-validator-exit")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .output()?
        .assert_success();
    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_stake_for_demo_default_num_validators(
    #[case] version: StakeTableContractVersion,
) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("stake-for-demo").output()?.assert_success();
    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_stake_for_demo_three_validators(
    #[case] version: StakeTableContractVersion,
) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("stake-for-demo")
        .arg("--num-validators")
        .arg("3")
        .output()?
        .assert_success();
    Ok(())
}

#[rstest::rstest]
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
    setup_test();
    let system = TestSystem::deploy_version(version).await?;

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("stake-for-demo")
        .arg("--delegation-config")
        .arg(config.to_string())
        .output()?
        .assert_success();
    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_approve(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    let amount = "123";

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("approve")
        .arg("--amount")
        .arg(amount)
        .output()?
        .assert_success();

    assert!(system.allowance(system.deployer_address).await? == parse_ether(amount)?);

    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_balance(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;

    // Check balance of account owner
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    let s = cmd.arg("token-balance").output()?.assert_success().utf8();
    let parts: Vec<&str> = s.split_whitespace().collect();
    let balance = parts[8].split(".").next().unwrap().parse::<U256>()?;

    assert!(s.contains(&system.deployer_address.to_string()));
    assert_eq!(balance, U256::from(3590000000u64));

    // Check balance of other address
    let addr = "0x1111111111111111111111111111111111111111";
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    let s = cmd
        .arg("token-balance")
        .arg("--address")
        .arg(addr)
        .output()?
        .assert_success()
        .utf8();

    assert!(s.contains(addr));
    assert!(s.contains(" 0.0"));

    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_allowance(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;

    // Check allowance of account owner
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    let out = cmd.arg("token-allowance").output()?.assert_success().utf8();

    assert!(out.contains(&system.deployer_address.to_string()));
    assert!(out.contains(&format_ether(system.approval_amount)));

    // Check allowance of other address
    let addr = "0x1111111111111111111111111111111111111111".to_string();
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    let out = cmd
        .arg("token-allowance")
        .arg("--owner")
        .arg(&addr)
        .output()?
        .assert_success()
        .utf8();

    assert!(out.contains(&addr));
    assert!(out.contains(" 0.0"));

    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_transfer(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    let addr = "0x1111111111111111111111111111111111111111".parse::<Address>()?;
    let amount = parse_ether("0.123")?;
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("transfer")
        .arg("--to")
        .arg(addr.to_string())
        .arg("--amount")
        .arg(format_ether(amount))
        .output()?
        .assert_success();

    assert_eq!(system.balance(addr).await?, amount);

    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_stake_table_full(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;

    let amount = parse_ether("0.123")?;
    system.delegate(amount).await?;

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    let out = cmd.arg("stake-table").output()?.assert_success().utf8();

    // Print output to fix test more easily.
    println!("{out}");
    out.contains("Validator 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266: BLS_VER_KEY~ksjrqSN9jEvKOeCNNySv9Gcg7UjZvROpOm99zHov8SgxfzhLyno8IUfE1nxOBhGnajBmeTbchVI94ZUg5VLgAT2DBKXBnIC6bY9y2FBaK1wPpIQVgx99-fAzWqbweMsiXKFYwiT-0yQjJBXkWyhtCuTHT4l3CRok68mkobI09q0c comm=12.34 % stake=0.123000000000000000 ESP");
    out.contains(
        " - Delegator 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266: stake=0.123000000000000000 ESP",
    );

    Ok(())
}

#[rstest_reuse::apply(stake_table_versions)]
async fn test_cli_stake_table_compact(#[case] version: StakeTableContractVersion) -> Result<()> {
    setup_test();
    let system = TestSystem::deploy_version(version).await?;
    system.register_validator().await?;

    let amount = parse_ether("0.123")?;
    system.delegate(amount).await?;

    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    let out = cmd
        .arg("stake-table")
        .arg("--compact")
        .output()?
        .assert_success()
        .utf8();

    // Print output to fix test more easily.
    println!("{out}");
    out.contains("Validator 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266: BLS_VER_KEY~ksjrqSN9jEvKOeCNNySv9Gcg7UjZ.. comm=12.34 % stake=0.123000000000000000 ESP");
    out.contains(
        " - Delegator 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266: stake=0.123000000000000000 ESP",
    );

    Ok(())
}

async fn address_from_cli(system: &TestSystem) -> Result<Address> {
    println!("Unlock the ledger");
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Ledger);
    // spawn the command first to show stderr output with errors/instructions
    let child = cmd.arg("account").stdout(Stdio::piped()).spawn()?;

    // wait for command to exit
    let output = child.wait_with_output()?.assert_success().utf8();

    // dump output for debugging purposes
    println!("staking-cli account output: {output}");

    Ok(output
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap()
        .parse()?)
}

/// This test requires a ledger device to be connected and unlocked.
/// cargo test -p staking-cli -- --ignored --nocapture transfer_ledger
#[ignore]
#[tokio::test]
async fn test_cli_transfer_ledger() -> Result<()> {
    setup_test();
    let system = TestSystem::deploy().await?;
    let address = address_from_cli(&system).await?;

    let amount = parse_ether("0.123")?;
    system.transfer_eth(address, amount).await?;
    system.transfer(address, amount).await?;

    // Assume the ledger is unlocked and the Ethereum app remains open
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Mnemonic);
    cmd.arg("transfer")
        .arg("--to")
        .arg(address.to_string())
        .arg("--amount")
        .arg(format_ether(amount))
        .output()?
        .assert_success();

    // Make a token transfer with the ledger
    println!("Sign the transaction in the ledger");
    let addr = "0x1111111111111111111111111111111111111111".parse::<Address>()?;
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Ledger);
    cmd.arg("transfer")
        .arg("--to")
        .arg(addr.to_string())
        .arg("--amount")
        .arg(format_ether(amount))
        .output()?
        .assert_success();

    assert_eq!(system.balance(addr).await?, amount);

    Ok(())
}

/// This test requires a ledger device to be connected and unlocked.
/// cargo test -p staking-cli -- --ignored --nocapture delegate_ledger
#[ignore]
#[tokio::test]
async fn test_cli_delegate_ledger() -> Result<()> {
    setup_test();
    let system = TestSystem::deploy().await?;
    system.register_validator().await?;
    let address = address_from_cli(&system).await?;

    let amount = parse_ether("0.123")?;
    system.transfer_eth(address, amount).await?;
    system.transfer(address, amount).await?;

    // Assume the ledger is unlocked and the Ethereum app remains open
    println!("Sign the transaction in the ledger");
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Ledger);
    cmd.arg("approve")
        .arg("--amount")
        .arg(format_ether(amount))
        .output()?
        .assert_success();

    println!("Sign the transaction in the ledger (again)");
    let mut cmd = base_cmd();
    system.args(&mut cmd, Signer::Ledger);
    cmd.arg("delegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg(format_ether(amount))
        .output()?
        .assert_success();

    Ok(())
}
