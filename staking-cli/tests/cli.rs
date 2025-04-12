use std::process::{Command, Output};

use alloy::primitives::{
    utils::{format_ether, parse_ether},
    Address, U256,
};
use anyhow::Result;
use staking_cli::*;

use crate::deploy::TestSystem;

trait AssertSuccess {
    fn assert_success(&self) -> &Self;
}

impl AssertSuccess for Output {
    fn assert_success(&self) -> &Self {
        if !self.status.success() {
            let stderr = String::from_utf8(self.stderr.clone()).expect("stderr is utf8");
            let stdout = String::from_utf8(self.stdout.clone()).expect("stdout is utf8");
            panic!("Command failed:\nstderr: {}\nstdout: {}", stderr, stdout);
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

fn cmd() -> Command {
    escargot::CargoBuild::new()
        .bin("staking-cli")
        .current_release()
        .current_target()
        .run()
        .unwrap()
        .command()
}

#[test]
fn test_cli_version() -> Result<()> {
    cmd().arg("version").output()?.assert_success();
    Ok(())
}

#[test]
fn test_cli_create_and_remove_config_file() -> anyhow::Result<()> {
    let tmpdir = tempfile::tempdir()?;
    let config_path = tmpdir.path().join("config.toml");

    assert!(!config_path.exists());

    cmd()
        .arg("-c")
        .arg(&config_path)
        .arg("init")
        .output()?
        .assert_success();

    assert!(config_path.exists());

    cmd()
        .arg("-c")
        .arg(&config_path)
        .arg("purge")
        .arg("--force")
        .output()?
        .assert_success();

    assert!(!config_path.exists());

    Ok(())
}

#[tokio::test]
async fn test_cli_register_validator() -> Result<()> {
    let system = TestSystem::deploy().await?;
    system
        .cmd()
        .arg("register-validator")
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
                .schnorr_key_pair
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

#[tokio::test]
async fn test_cli_delegate() -> Result<()> {
    let system = TestSystem::deploy().await?;
    system.register_validator().await?;

    system
        .cmd()
        .arg("delegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg("123")
        .output()?
        .assert_success();
    Ok(())
}

#[tokio::test]
async fn test_cli_deregister_validator() -> Result<()> {
    let system = TestSystem::deploy().await?;
    system.register_validator().await?;

    system
        .cmd()
        .arg("deregister-validator")
        .output()?
        .assert_success();
    Ok(())
}

#[tokio::test]
async fn test_cli_undelegate() -> Result<()> {
    let system = TestSystem::deploy().await?;
    system.register_validator().await?;
    let amount = "123";
    system.delegate(parse_ether(amount)?).await?;

    system
        .cmd()
        .arg("undelegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg(amount)
        .output()?
        .assert_success();
    Ok(())
}

#[tokio::test]
async fn test_cli_claim_withdrawal() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let amount = U256::from(123);
    system.register_validator().await?;
    system.delegate(amount).await?;
    system.undelegate(amount).await?;
    system.warp_to_unlock_time().await?;

    system
        .cmd()
        .arg("claim-withdrawal")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .output()?
        .assert_success();
    Ok(())
}

#[tokio::test]
async fn test_cli_claim_validator_exit() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let amount = U256::from(123);
    system.register_validator().await?;
    system.delegate(amount).await?;
    system.deregister_validator().await?;
    system.warp_to_unlock_time().await?;

    system
        .cmd()
        .arg("claim-validator-exit")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .output()?
        .assert_success();
    Ok(())
}

#[tokio::test]
async fn test_cli_stake_for_demo_default_num_validators() -> Result<()> {
    let system = TestSystem::deploy().await?;

    system
        .cmd()
        .arg("stake-for-demo")
        .output()?
        .assert_success();
    Ok(())
}

#[tokio::test]
async fn test_cli_stake_for_demo_three_validators() -> Result<()> {
    let system = TestSystem::deploy().await?;

    system
        .cmd()
        .arg("stake-for-demo")
        .arg("--num-validators")
        .arg("3")
        .output()?
        .assert_success();
    Ok(())
}

#[tokio::test]
async fn test_cli_approve() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let amount = "123";

    system
        .cmd()
        .arg("approve")
        .arg("--amount")
        .arg(amount)
        .output()?
        .assert_success();

    assert!(system.allowance(system.deployer_address).await? == parse_ether(amount)?);

    Ok(())
}

#[tokio::test]
async fn test_cli_balance() -> Result<()> {
    let system = TestSystem::deploy().await?;

    // Check balance of account owner
    let s = system
        .cmd()
        .arg("token-balance")
        .output()?
        .assert_success()
        .utf8();

    assert!(s.contains(&system.deployer_address.to_string()));
    assert!(s.contains(" 10000000000.0"));

    // Check balance of other address
    let addr = "0x1111111111111111111111111111111111111111";
    let s = system
        .cmd()
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

#[tokio::test]
async fn test_cli_allowance() -> Result<()> {
    let system = TestSystem::deploy().await?;

    // Check allowance of account owner
    let out = system
        .cmd()
        .arg("token-allowance")
        .output()?
        .assert_success()
        .utf8();

    assert!(out.contains(&system.deployer_address.to_string()));
    assert!(out.contains(&format_ether(system.approval_amount)));

    // Check allowance of other address
    let addr = "0x1111111111111111111111111111111111111111".to_string();
    let out = system
        .cmd()
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

#[tokio::test]
async fn test_cli_transfer() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let addr = "0x1111111111111111111111111111111111111111".parse::<Address>()?;
    let amount = parse_ether("0.123")?;
    system
        .cmd()
        .arg("transfer")
        .arg("--to")
        .arg(addr.to_string())
        .arg("--amount")
        .arg(format_ether(amount))
        .output()?
        .assert_success();

    assert_eq!(system.balance(addr).await?, amount);

    Ok(())
}

#[tokio::test]
async fn test_cli_info_full() -> Result<()> {
    let system = TestSystem::deploy().await?;
    system.register_validator().await?;

    let amount = parse_ether("0.123")?;
    system.delegate(amount).await?;

    let out = system.cmd().arg("info").output()?.assert_success().utf8();

    // Print output to fix test more easily.
    println!("{}", out);
    out.contains("Validator 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266: BLS_VER_KEY~ksjrqSN9jEvKOeCNNySv9Gcg7UjZvROpOm99zHov8SgxfzhLyno8IUfE1nxOBhGnajBmeTbchVI94ZUg5VLgAT2DBKXBnIC6bY9y2FBaK1wPpIQVgx99-fAzWqbweMsiXKFYwiT-0yQjJBXkWyhtCuTHT4l3CRok68mkobI09q0c comm=12.34 % stake=0.123000000000000000 ESP");
    out.contains(
        " - Delegator 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266: stake=0.123000000000000000 ESP",
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_info_compact() -> Result<()> {
    let system = TestSystem::deploy().await?;
    system.register_validator().await?;

    let amount = parse_ether("0.123")?;
    system.delegate(amount).await?;

    let out = system
        .cmd()
        .arg("info")
        .arg("--compact")
        .output()?
        .assert_success()
        .utf8();

    // Print output to fix test more easily.
    println!("{}", out);
    out.contains("Validator 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266: BLS_VER_KEY~ksjrqSN9jEvKOeCNNySv9Gcg7UjZ.. comm=12.34 % stake=0.123000000000000000 ESP");
    out.contains(
        " - Delegator 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266: stake=0.123000000000000000 ESP",
    );

    Ok(())
}
