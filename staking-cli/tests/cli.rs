use std::process::{Command, Output};

use alloy::primitives::U256;
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
fn test_cli_created_and_remove_config_file() -> anyhow::Result<()> {
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
    let amount = U256::from(123);
    system.delegate(amount).await?;

    system
        .cmd()
        .arg("undelegate")
        .arg("--validator-address")
        .arg(system.deployer_address.to_string())
        .arg("--amount")
        .arg(amount.to_string())
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
async fn test_cli_stake_for_demo() -> Result<()> {
    let system = TestSystem::deploy().await?;

    system
        .cmd()
        .arg("stake-for-demo")
        .output()?
        .assert_success();
    Ok(())
}
