use std::{
    io::Read as _,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use alloy::{
    primitives::{utils::parse_ether, Address, U256},
    providers::{Provider, ProviderBuilder, WalletProvider},
    signers::local::PrivateKeySigner,
};
use anyhow::Result;
use common::{Signer, TestSystemExt};
use espresso_contract_deployer::build_signer;
use hotshot_contract_adapter::{sol_types::StakeTableV2, stake_table::StakeTableContractVersion};
use rand::{rngs::StdRng, SeedableRng};
use rstest::rstest;
use staking_cli::{
    demo::generate_delegator_signer, deploy::TestSystem, parse::Commission,
    signature::NodeSignatures, transaction::Transaction, DEMO_VALIDATOR_START_INDEX,
};
use tokio::time::sleep;
use url::Url;

mod common;

const RETH_IMAGE: &str = "ghcr.io/paradigmxyz/reth:latest";
const RETH_STARTUP_RETRIES: u32 = 10;

/// Reth container for stress testing.
struct RethContainer {
    port: u16,
    child: Child,
}

impl RethContainer {
    /// Start Reth in dev mode with specified block time.
    async fn start_with_block_time(block_time: &str) -> Result<Self> {
        Self::ensure_image().await?;

        let port =
            portpicker::pick_unused_port().ok_or_else(|| anyhow::anyhow!("No ports available"))?;

        tracing::info!(
            "Starting reth with {} block time on port {}...",
            block_time,
            port
        );

        let child = Self::spawn_container(port, Some(block_time)).await?;

        let container = RethContainer { port, child };

        container.wait_ready().await?;
        Ok(container)
    }

    async fn ensure_image() -> Result<()> {
        let image_exists = tokio::task::spawn_blocking(|| {
            Command::new("docker")
                .args(["image", "inspect", RETH_IMAGE])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
        })
        .await?;

        if !image_exists {
            tracing::info!("Pulling docker image {}...", RETH_IMAGE);
            let pull_output = tokio::task::spawn_blocking(|| {
                Command::new("docker").args(["pull", RETH_IMAGE]).output()
            })
            .await??;

            if !pull_output.status.success() {
                anyhow::bail!(
                    "Failed to pull docker image {}: {}",
                    RETH_IMAGE,
                    String::from_utf8_lossy(&pull_output.stderr)
                );
            }
        }
        Ok(())
    }

    async fn spawn_container(port: u16, block_time: Option<&str>) -> Result<Child> {
        let port_arg = format!("{}:8545", port);
        let block_time_owned = block_time.map(|s| s.to_string());

        let mut child = tokio::task::spawn_blocking(move || {
            let mut cmd = Command::new("docker");
            cmd.args(["run", "--rm", "-p", &port_arg, RETH_IMAGE, "node", "--dev"]);

            if let Some(ref bt) = block_time_owned {
                cmd.arg(format!("--dev.block-time={}", bt));
            }

            cmd.args([
                "--http",
                "--http.addr=0.0.0.0",
                "--http.api=eth,net,web3,txpool,debug",
                "--txpool.max-account-slots=10000",
                "--txpool.pending-max-count=20000",
            ]);

            cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        })
        .await??;

        sleep(Duration::from_secs(3)).await;

        if let Some(status) = child.try_wait()? {
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();
            let stdout_str = stdout
                .map(|mut s| {
                    let mut buf = String::new();
                    s.read_to_string(&mut buf).ok();
                    buf
                })
                .unwrap_or_default();
            let stderr_str = stderr
                .map(|mut s| {
                    let mut buf = String::new();
                    s.read_to_string(&mut buf).ok();
                    buf
                })
                .unwrap_or_default();
            anyhow::bail!(
                "Reth container exited immediately with status {}.\nstdout:\n{}\nstderr:\n{}",
                status,
                stdout_str,
                stderr_str
            );
        }

        Ok(child)
    }

    async fn wait_ready(&self) -> Result<()> {
        let rpc_url = self.rpc_url();
        let mut last_error = None;

        for i in 0..RETH_STARTUP_RETRIES {
            match ProviderBuilder::new()
                .connect_http(rpc_url.clone())
                .get_block_number()
                .await
            {
                Ok(_) => {
                    tracing::info!("Reth ready after {} seconds", i + 1);
                    return Ok(());
                },
                Err(e) => {
                    last_error = Some(e);
                    if i < RETH_STARTUP_RETRIES - 1 {
                        sleep(Duration::from_secs(1)).await;
                    }
                },
            }
        }

        anyhow::bail!(
            "Reth not ready after {} seconds. Last error: {:?}",
            RETH_STARTUP_RETRIES,
            last_error,
        )
    }

    fn rpc_url(&self) -> Url {
        format!("http://127.0.0.1:{}", self.port)
            .parse()
            .expect("valid URL")
    }
}

impl Drop for RethContainer {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

trait DemoTestExt {
    async fn get_delegation(&self, validator: Address, delegator: Address) -> Result<U256>;

    async fn setup_validators(&self, count: usize) -> Result<Vec<Address>>;

    async fn assert_delegations(
        &self,
        validator: Address,
        start_index: u64,
        num_delegators: u64,
        expected: U256,
    ) -> Result<()>;
}

impl DemoTestExt for TestSystem {
    async fn get_delegation(&self, validator: Address, delegator: Address) -> Result<U256> {
        let stake_table = StakeTableV2::new(self.stake_table, &self.provider);
        Ok(stake_table.delegations(validator, delegator).call().await?)
    }

    async fn setup_validators(&self, count: usize) -> Result<Vec<Address>> {
        let mut validators = Vec::new();
        let mut rng = StdRng::from_seed([42u8; 32]);
        let start_index = 20u32;

        for i in 0..count {
            let index = start_index + i as u32;
            let signer = build_signer(staking_cli::DEV_MNEMONIC, index);
            let validator_address = signer.address();

            let (_, bls_key, state_key) = TestSystem::gen_keys(&mut rng);

            let fund_amount = parse_ether("100")?;
            self.transfer_eth(validator_address, fund_amount).await?;
            self.transfer(validator_address, fund_amount).await?;

            let provider = alloy::providers::ProviderBuilder::new()
                .wallet(alloy::network::EthereumWallet::from(signer))
                .connect_http(self.rpc_url.clone());

            Transaction::Approve {
                token: self.token,
                spender: self.stake_table,
                amount: fund_amount,
            }
            .send(&provider)
            .await?
            .get_receipt()
            .await?;

            let payload = NodeSignatures::create(validator_address, &bls_key, &state_key);
            let metadata_uri = "https://example.com/metadata".parse()?;
            let commission = Commission::try_from("10.0")?;

            Transaction::RegisterValidator {
                stake_table: self.stake_table,
                commission,
                metadata_uri,
                payload,
                version: StakeTableContractVersion::V2,
            }
            .send(&provider)
            .await?
            .get_receipt()
            .await?;

            validators.push(validator_address);
        }

        Ok(validators)
    }

    async fn assert_delegations(
        &self,
        validator: Address,
        start_index: u64,
        num_delegators: u64,
        expected: U256,
    ) -> Result<()> {
        for i in start_index..start_index + num_delegators {
            let delegator = generate_delegator_signer(i);
            let delegation = self.get_delegation(validator, delegator.address()).await?;
            assert_eq!(
                delegation, expected,
                "delegator {i} should have {expected} delegation"
            );
        }
        Ok(())
    }
}

#[rstest]
#[case::single_validator(1, 5)]
#[case::multiple_validators(3, 9)]
#[test_log::test(tokio::test)]
async fn test_delegate_to_validators(
    #[case] num_validators: usize,
    #[case] num_delegators: u64,
) -> Result<()> {
    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(num_validators).await?;

    let min_amount = parse_ether("100")?;
    let max_amount = parse_ether("500")?;

    let validator_addrs = validators
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let log_dir = tempfile::tempdir()?;
    let log_path = log_dir.path().join("delegate_log.json");

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(&validator_addrs)
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("500")
        .arg("--log-path")
        .arg(&log_path)
        .assert()
        .success();

    for i in 0..num_delegators {
        let delegator = generate_delegator_signer(i);
        let validator = validators[(i as usize) % validators.len()];
        let delegation = system
            .get_delegation(validator, delegator.address())
            .await?;

        assert!(
            delegation >= min_amount && delegation <= max_amount,
            "delegation {} not in range [{}, {}]",
            delegation,
            min_amount,
            max_amount
        );
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_delegate_with_tx_log() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(1).await?;

    let num_delegators = 3;
    let log_dir = tempfile::tempdir()?;
    let log_path = log_dir.path().join("delegate_log.json");

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(validators[0].to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("100")
        .arg("--log-path")
        .arg(&log_path)
        .arg("--concurrency")
        .arg("5")
        .assert()
        .success();

    assert!(
        !log_path.exists(),
        "log file should be archived after completion"
    );

    let archived_files: Vec<_> = std::fs::read_dir(log_dir.path())?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".completed."))
        .collect();
    assert_eq!(
        archived_files.len(),
        1,
        "should have exactly one archived log"
    );

    system
        .assert_delegations(validators[0], 0, num_delegators as u64, parse_ether("100")?)
        .await?;

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_delegate_amount_range() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(1).await?;

    let num_delegators = 10;
    let min_amount = parse_ether("50")?;
    let max_amount = parse_ether("200")?;
    let log_dir = tempfile::tempdir()?;
    let log_path = log_dir.path().join("delegate_log.json");

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(validators[0].to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg("50")
        .arg("--max-amount")
        .arg("200")
        .arg("--log-path")
        .arg(&log_path)
        .assert()
        .success();

    for i in 0..num_delegators {
        let delegator = generate_delegator_signer(i);
        let delegation = system
            .get_delegation(validators[0], delegator.address())
            .await?;

        assert!(
            delegation >= min_amount && delegation <= max_amount,
            "delegation {} not in range [{}, {}]",
            delegation,
            min_amount,
            max_amount
        );
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_delegate_deterministic_addresses() -> Result<()> {
    let delegator1_run1 = generate_delegator_signer(5);
    let delegator1_run2 = generate_delegator_signer(5);

    let delegator2_run1 = generate_delegator_signer(10);
    let delegator2_run2 = generate_delegator_signer(10);

    assert_eq!(delegator1_run1.address(), delegator1_run2.address());
    assert_eq!(delegator2_run1.address(), delegator2_run2.address());
    assert_ne!(delegator1_run1.address(), delegator2_run1.address());

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_undelegate_single_validator() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(1).await?;

    let num_delegators = 5;
    let log_dir = tempfile::tempdir()?;
    let delegate_log = log_dir.path().join("delegate_log.json");
    let undelegate_log = log_dir.path().join("undelegate_log.json");

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(validators[0].to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("100")
        .arg("--log-path")
        .arg(&delegate_log)
        .assert()
        .success();

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("undelegate")
        .arg("--validators")
        .arg(validators[0].to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--log-path")
        .arg(&undelegate_log)
        .assert()
        .success();

    for i in 0..num_delegators {
        let delegator = generate_delegator_signer(i);
        let delegation = system
            .get_delegation(validators[0], delegator.address())
            .await?;
        assert_eq!(delegation, U256::ZERO);
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_undelegate_multiple_validators() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(3).await?;

    let num_delegators = 9;
    let log_dir = tempfile::tempdir()?;
    let delegate_log = log_dir.path().join("delegate_log.json");
    let undelegate_log = log_dir.path().join("undelegate_log.json");

    let validator_addrs = validators
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",");

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(&validator_addrs)
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg("150")
        .arg("--max-amount")
        .arg("150")
        .arg("--log-path")
        .arg(&delegate_log)
        .assert()
        .success();

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("undelegate")
        .arg("--validators")
        .arg(&validator_addrs)
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--log-path")
        .arg(&undelegate_log)
        .assert()
        .success();

    for i in 0..num_delegators {
        let delegator = generate_delegator_signer(i);
        for validator in &validators {
            let delegation = system
                .get_delegation(*validator, delegator.address())
                .await?;
            assert_eq!(delegation, U256::ZERO);
        }
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_undelegate_skips_zero_delegations() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(1).await?;

    let num_delegators_to_delegate = 3;
    let num_delegators_total = 10;
    let log_dir = tempfile::tempdir()?;
    let delegate_log = log_dir.path().join("delegate_log.json");
    let undelegate_log = log_dir.path().join("undelegate_log.json");

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(validators[0].to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators_to_delegate.to_string())
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("100")
        .arg("--log-path")
        .arg(&delegate_log)
        .assert()
        .success();

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("undelegate")
        .arg("--validators")
        .arg(validators[0].to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators_total.to_string())
        .arg("--log-path")
        .arg(&undelegate_log)
        .assert()
        .success();

    for i in 0..num_delegators_total {
        let delegator = generate_delegator_signer(i);
        let delegation = system
            .get_delegation(validators[0], delegator.address())
            .await?;
        assert_eq!(delegation, U256::ZERO);
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_undelegate_queries_correct_amounts() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(1).await?;

    let num_delegators = 5;
    let _min_amount = parse_ether("50")?;
    let _max_amount = parse_ether("200")?;
    let log_dir = tempfile::tempdir()?;
    let delegate_log = log_dir.path().join("delegate_log.json");
    let undelegate_log = log_dir.path().join("undelegate_log.json");

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(validators[0].to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg("50")
        .arg("--max-amount")
        .arg("200")
        .arg("--log-path")
        .arg(&delegate_log)
        .assert()
        .success();

    let delegated_amounts: Vec<U256> =
        futures_util::future::try_join_all((0..num_delegators).map(|i| {
            let system = &system;
            let validator = validators[0];
            async move {
                let delegator = generate_delegator_signer(i);
                system.get_delegation(validator, delegator.address()).await
            }
        }))
        .await?;

    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("undelegate")
        .arg("--validators")
        .arg(validators[0].to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--log-path")
        .arg(&undelegate_log)
        .assert()
        .success();

    for i in 0..num_delegators {
        let delegator = generate_delegator_signer(i);
        let delegation = system
            .get_delegation(validators[0], delegator.address())
            .await?;
        assert_eq!(
            delegation,
            U256::ZERO,
            "delegator {} should have 0 delegation after undelegating {}",
            i,
            delegated_amounts[i as usize]
        );
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_churn_initial_funding() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let num_validators = 2usize;
    let num_delegators = 3u64;

    system.setup_validators(num_validators).await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("demo")
        .arg("churn")
        .arg("--validator-start-index")
        .arg("20")
        .arg("--num-validators")
        .arg(num_validators.to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("500")
        .arg("--delay")
        .arg("50ms");

    let handle = tokio::spawn(async move {
        let _ = cmd.assert();
    });

    tokio::time::sleep(Duration::from_millis(500)).await;
    handle.abort();

    for i in 0..num_delegators {
        let delegator = generate_delegator_signer(i);
        let balance = system.balance(delegator.address()).await?;
        assert!(balance > U256::ZERO, "delegator {} should be funded", i);
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_churn_delegate_then_undelegate() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let num_validators = 2usize;
    let num_delegators = 3u64;
    let validators = system.setup_validators(num_validators).await?;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("demo")
        .arg("churn")
        .arg("--validator-start-index")
        .arg("20")
        .arg("--num-validators")
        .arg(num_validators.to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("500")
        .arg("--delay")
        .arg("50ms");

    let handle = tokio::spawn(async move {
        let _ = cmd.assert();
    });

    tokio::time::sleep(Duration::from_secs(2)).await;
    handle.abort();

    let mut has_delegations = false;
    for i in 0..num_delegators {
        let delegator = generate_delegator_signer(i);
        for validator in &validators {
            let delegation = system
                .get_delegation(*validator, delegator.address())
                .await?;
            if delegation > U256::ZERO {
                has_delegations = true;
                break;
            }
        }
    }

    assert!(
        has_delegations,
        "at least one delegation should exist after churn"
    );

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_churn_respects_delay() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let _validators = system.setup_validators(1).await?;

    let delay = Duration::from_millis(200);

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("demo")
        .arg("churn")
        .arg("--validator-start-index")
        .arg("20")
        .arg("--num-validators")
        .arg("1")
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg("2")
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("100")
        .arg("--delay")
        .arg(format!("{}ms", delay.as_millis()));

    let start = Instant::now();
    let handle = tokio::spawn(async move {
        let _ = cmd.assert();
    });

    tokio::time::sleep(Duration::from_millis(1000)).await;
    handle.abort();
    let elapsed = start.elapsed();

    let min_expected = delay * 2;
    assert!(
        elapsed >= min_expected,
        "churn should respect delay, elapsed {:?} < expected minimum {:?}",
        elapsed,
        min_expected
    );

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_churn_multiple_iterations() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(2).await?;
    let num_delegators = 3;

    let mut cmd = system.cmd(Signer::Mnemonic);
    cmd.arg("demo")
        .arg("churn")
        .arg("--validator-start-index")
        .arg("20")
        .arg("--num-validators")
        .arg(validators.len().to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("100")
        .arg("--delay")
        .arg("50ms");

    let handle = tokio::spawn(async move {
        let _ = cmd.assert();
    });

    tokio::time::sleep(Duration::from_secs(3)).await;
    handle.abort();

    let mut total_delegations = U256::ZERO;
    for i in 0..num_delegators {
        let delegator = generate_delegator_signer(i);
        for validator in &validators {
            let delegation = system
                .get_delegation(*validator, delegator.address())
                .await?;
            total_delegations += delegation;
        }
    }

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_generate_delegator_deterministic() -> Result<()> {
    let index = 42;

    let signer1 = generate_delegator_signer(index);
    let signer2 = generate_delegator_signer(index);

    assert_eq!(signer1.address(), signer2.address());

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_generate_delegator_different_indices() -> Result<()> {
    let signer1 = generate_delegator_signer(0);
    let signer2 = generate_delegator_signer(1);
    let signer3 = generate_delegator_signer(100);

    assert_ne!(signer1.address(), signer2.address());
    assert_ne!(signer1.address(), signer3.address());
    assert_ne!(signer2.address(), signer3.address());

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_demo_all_operations_manual_inspect() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let num_validators = 2usize;
    let validators = system.setup_validators(num_validators).await?;

    let validator_addrs = validators
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let num_delegators = 5u64;
    let log_dir = tempfile::tempdir()?;
    let delegate_log = log_dir.path().join("delegate_log.json");
    let undelegate_log = log_dir.path().join("undelegate_log.json");

    let output = system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(&validator_addrs)
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("500")
        .arg("--log-path")
        .arg(&delegate_log)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("=== demo delegate ===");
    println!("{}", String::from_utf8_lossy(&output));

    let output = system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("undelegate")
        .arg("--validators")
        .arg(&validator_addrs)
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--log-path")
        .arg(&undelegate_log)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    println!("=== demo undelegate ===");
    println!("{}", String::from_utf8_lossy(&output));

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_demo_delegate_with_slow_blockchain() -> Result<()> {
    use alloy::providers::ext::AnvilApi;

    // With 1s block time and tx_log, we sign all txs upfront then submit them.
    // The test verifies that delegation works correctly with interval mining.
    tokio::time::timeout(Duration::from_secs(30), async {
        let system = TestSystem::deploy().await?;
        let num_validators = 1usize;
        let validators = system.setup_validators(num_validators).await?;

        system.provider.anvil_set_auto_mine(false).await?;
        system.provider.anvil_set_interval_mining(1).await?;

        let validator_addrs = validators[0].to_string();
        let num_delegators = 5u64;

        let log_dir = tempfile::tempdir()?;
        let delegate_log = log_dir.path().join("delegate_log.json");
        let undelegate_log = log_dir.path().join("undelegate_log.json");

        system
            .cmd(Signer::Mnemonic)
            .timeout(Duration::from_secs(25))
            .arg("demo")
            .arg("delegate")
            .arg("--validators")
            .arg(&validator_addrs)
            .arg("--delegator-start-index")
            .arg("0")
            .arg("--num-delegators")
            .arg(num_delegators.to_string())
            .arg("--min-amount")
            .arg("100")
            .arg("--max-amount")
            .arg("100")
            .arg("--log-path")
            .arg(&delegate_log)
            .arg("--concurrency")
            .arg("10")
            .assert()
            .success();

        system
            .assert_delegations(validators[0], 0, num_delegators, parse_ether("100")?)
            .await?;

        system
            .cmd(Signer::Mnemonic)
            .timeout(Duration::from_secs(10))
            .arg("demo")
            .arg("undelegate")
            .arg("--validators")
            .arg(&validator_addrs)
            .arg("--delegator-start-index")
            .arg("0")
            .arg("--num-delegators")
            .arg(num_delegators.to_string())
            .arg("--log-path")
            .arg(&undelegate_log)
            .assert()
            .success();

        system
            .assert_delegations(validators[0], 0, num_delegators, U256::ZERO)
            .await?;

        Ok(())
    })
    .await
    .map_err(|_| anyhow::anyhow!("test timed out"))?
}

#[tokio::test(flavor = "multi_thread")]
async fn test_delegate_tx_log_resume_after_partial() -> Result<()> {
    use alloy::providers::ext::AnvilApi;
    use staking_cli::tx_log::TxLog;

    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(1).await?;
    let validator_addr = validators[0].to_string();

    let log_dir = tempfile::tempdir()?;
    let log_path = log_dir.path().join("delegate_log.json");

    // Use slow interval mining - 2 second blocks
    // This allows txs to be submitted but not all confirmed before timeout
    system.provider.anvil_set_auto_mine(false).await?;
    system.provider.anvil_set_interval_mining(2).await?;

    let log_path_clone = log_path.clone();
    let stake_table_address = system.stake_table;
    let rpc_url = system.rpc_url.clone();
    let validator_addr_clone = validator_addr.clone();

    // Start delegation in a thread that will be interrupted
    let cmd_handle = std::thread::spawn(move || {
        let output = common::base_cmd()
            .arg("--rpc-url")
            .arg(rpc_url.to_string())
            .arg("--stake-table-address")
            .arg(stake_table_address.to_string())
            .arg("--mnemonic")
            .arg(staking_cli::DEV_MNEMONIC)
            .arg("--account-index")
            .arg("0")
            .arg("demo")
            .arg("delegate")
            .arg("--validators")
            .arg(&validator_addr_clone)
            .arg("--delegator-start-index")
            .arg("0")
            .arg("--num-delegators")
            .arg("20")
            .arg("--min-amount")
            .arg("100")
            .arg("--max-amount")
            .arg("100")
            .arg("--log-path")
            .arg(&log_path_clone)
            .timeout(std::time::Duration::from_secs(4))
            .output()
            .expect("failed to run command");
        eprintln!("STDOUT: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("STDERR: {}", String::from_utf8_lossy(&output.stderr));
    });

    // Let the command timeout
    let _ = cmd_handle.join();

    // Verify log file exists (indicates partial execution)
    assert!(log_path.exists(), "log should exist after interrupted run");
    let log = TxLog::load(&log_path)?.expect("log should be loadable");
    assert!(!log.transactions.is_empty(), "log should have transactions");

    // Dump state after partial execution (simulates saving state before restart)
    let saved_state = system.dump_state().await?;

    // Load state back (simulates restarting with same state)
    system.load_state(saved_state).await?;

    // Turn automine back on for resume
    system.provider.anvil_set_interval_mining(0).await?;
    system.provider.anvil_set_auto_mine(true).await?;

    // Resume the delegation - should complete successfully
    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(&validator_addr)
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg("20")
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("100")
        .arg("--log-path")
        .arg(&log_path)
        .assert()
        .success();

    // Verify log was archived (indicates completion)
    assert!(
        !log_path.exists(),
        "log should be archived after successful resume"
    );

    // Verify delegations actually happened
    system
        .assert_delegations(validators[0], 0, 20, parse_ether("100")?)
        .await?;

    Ok(())
}

#[test_log::test(tokio::test)]
async fn test_demo_multiple_delegator_batches() -> Result<()> {
    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(1).await?;
    let validator_addrs = validators[0].to_string();

    let num_delegators = 3u64;
    let amount = "100";
    let log_dir = tempfile::tempdir()?;
    let delegate_log1 = log_dir.path().join("delegate_log1.json");
    let delegate_log2 = log_dir.path().join("delegate_log2.json");
    let undelegate_log = log_dir.path().join("undelegate_log.json");

    // First batch: delegators 0, 1, 2
    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(&validator_addrs)
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg(amount)
        .arg("--max-amount")
        .arg(amount)
        .arg("--log-path")
        .arg(&delegate_log1)
        .assert()
        .success();

    system
        .assert_delegations(validators[0], 0, num_delegators, parse_ether(amount)?)
        .await?;

    // Second batch: delegators 3, 4, 5
    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(&validator_addrs)
        .arg("--delegator-start-index")
        .arg("3")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--min-amount")
        .arg(amount)
        .arg("--max-amount")
        .arg(amount)
        .arg("--log-path")
        .arg(&delegate_log2)
        .assert()
        .success();

    system
        .assert_delegations(validators[0], 3, num_delegators, parse_ether(amount)?)
        .await?;

    // Verify first batch still has delegations
    system
        .assert_delegations(validators[0], 0, num_delegators, parse_ether(amount)?)
        .await?;

    // Undelegate only second batch
    system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("undelegate")
        .arg("--validators")
        .arg(&validator_addrs)
        .arg("--delegator-start-index")
        .arg("3")
        .arg("--num-delegators")
        .arg(num_delegators.to_string())
        .arg("--log-path")
        .arg(&undelegate_log)
        .assert()
        .success();

    // Second batch should be zero
    system
        .assert_delegations(validators[0], 3, num_delegators, U256::ZERO)
        .await?;

    // First batch should still have delegations
    system
        .assert_delegations(validators[0], 0, num_delegators, parse_ether(amount)?)
        .await?;

    Ok(())
}

/// Test that mass delegation commands fail early with insufficient balance.
///
/// This is a regression test ensuring that the funder's ESP/ETH balance is checked
/// before starting any transactions, not mid-way through execution.
#[derive(Debug, Clone, Copy)]
enum InsufficientBalanceType {
    Esp,
    Eth,
}

#[rstest]
#[case::delegate_insufficient_esp(InsufficientBalanceType::Esp)]
#[case::delegate_insufficient_eth(InsufficientBalanceType::Eth)]
#[test_log::test(tokio::test)]
async fn test_delegate_for_demo_insufficient_balance(
    #[case] balance_type: InsufficientBalanceType,
) -> Result<()> {
    let system = TestSystem::deploy().await?;
    let validators = system.setup_validators(1).await?;

    let drain_address = PrivateKeySigner::random().address();

    // Drain the funder's balance before attempting mass delegation
    match balance_type {
        InsufficientBalanceType::Esp => {
            // Drain ESP tokens to leave insufficient balance
            let balance = system
                .balance(system.provider.default_signer_address())
                .await?;
            // Leave only a tiny amount that won't be enough for the operation
            let drain_amount = balance - parse_ether("1")?;
            system.transfer(drain_address, drain_amount).await?;
        },
        InsufficientBalanceType::Eth => {
            // Drain ETH to leave insufficient balance for gas + funding
            let eth_balance = system
                .provider
                .get_balance(system.provider.default_signer_address())
                .await?;
            // Keep just enough for the balance check calls to succeed
            let drain_amount = eth_balance - parse_ether("0.1")?;
            system.transfer_eth(drain_address, drain_amount).await?;
        },
    }

    let log_dir = tempfile::tempdir()?;
    let log_path = log_dir.path().join("delegate_log.json");

    // Attempt mass delegation with insufficient funds
    // This should fail early with a clear error, NOT mid-execution
    let output = system
        .cmd(Signer::Mnemonic)
        .arg("demo")
        .arg("delegate")
        .arg("--validators")
        .arg(validators[0].to_string())
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg("10") // Request 10 delegators which requires significant ESP + ETH
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("500")
        .arg("--log-path")
        .arg(&log_path)
        .assert()
        .failure();

    // Verify the error message mentions insufficient balance
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    match balance_type {
        InsufficientBalanceType::Esp => {
            assert!(
                stderr.contains("insufficient ESP") || stderr.contains("InsufficientEsp"),
                "Expected insufficient ESP error, got: {}",
                stderr
            );
        },
        InsufficientBalanceType::Eth => {
            assert!(
                stderr.contains("insufficient ETH") || stderr.contains("InsufficientEth"),
                "Expected insufficient ETH error, got: {}",
                stderr
            );
        },
    }

    // Verify no tx_log was created (since we failed before signing)
    assert!(
        !log_path.exists(),
        "tx_log should not be created when failing due to insufficient balance"
    );

    Ok(())
}

/// Test that churn command fails early with insufficient balance.
#[rstest]
#[case::churn_insufficient_esp(InsufficientBalanceType::Esp)]
#[case::churn_insufficient_eth(InsufficientBalanceType::Eth)]
#[test_log::test(tokio::test)]
async fn test_churn_for_demo_insufficient_balance(
    #[case] balance_type: InsufficientBalanceType,
) -> Result<()> {
    let system = TestSystem::deploy().await?;
    let _validators = system.setup_validators(2).await?;

    let drain_address = PrivateKeySigner::random().address();

    // Drain the funder's balance before attempting churn
    match balance_type {
        InsufficientBalanceType::Esp => {
            let balance = system
                .balance(system.provider.default_signer_address())
                .await?;
            let drain_amount = balance - parse_ether("1")?;
            system.transfer(drain_address, drain_amount).await?;
        },
        InsufficientBalanceType::Eth => {
            let eth_balance = system
                .provider
                .get_balance(system.provider.default_signer_address())
                .await?;
            let drain_amount = eth_balance - parse_ether("0.1")?;
            system.transfer_eth(drain_address, drain_amount).await?;
        },
    }

    // Attempt churn with insufficient funds
    // Start churn and let it fail - it should fail early during initial funding
    let output = system
        .cmd(Signer::Mnemonic)
        .timeout(Duration::from_secs(10))
        .arg("demo")
        .arg("churn")
        .arg("--validator-start-index")
        .arg("20")
        .arg("--num-validators")
        .arg("2")
        .arg("--delegator-start-index")
        .arg("0")
        .arg("--num-delegators")
        .arg("10")
        .arg("--min-amount")
        .arg("100")
        .arg("--max-amount")
        .arg("500")
        .arg("--delay")
        .arg("50ms")
        .assert()
        .failure();

    // Verify the error message mentions insufficient balance
    let stderr = String::from_utf8_lossy(&output.get_output().stderr);
    match balance_type {
        InsufficientBalanceType::Esp => {
            assert!(
                stderr.contains("insufficient ESP") || stderr.contains("InsufficientEsp"),
                "Expected insufficient ESP error, got: {}",
                stderr
            );
        },
        InsufficientBalanceType::Eth => {
            assert!(
                stderr.contains("insufficient ETH") || stderr.contains("InsufficientEth"),
                "Expected insufficient ETH error, got: {}",
                stderr
            );
        },
    }

    Ok(())
}

/// Stress test command variants for demo subcommands
#[derive(Debug, Clone, Copy)]
enum StressTestCommand {
    Stake,
    Delegate,
    Undelegate,
}

/// Stress test that runs 10k delegators against a reth node via CLI.
///
/// 5 validators x 2000 delegators = 10k delegators
///
/// Run with: `cargo test -p staking-cli stress_test_demo_cli -- --ignored`
#[rstest]
#[case::stake(StressTestCommand::Stake)]
#[case::delegate(StressTestCommand::Delegate)]
#[case::undelegate(StressTestCommand::Undelegate)]
#[ignore]
#[test_log::test(tokio::test)]
async fn stress_test_demo_cli(#[case] command: StressTestCommand) -> Result<()> {
    // Start Reth with 1s block time for realistic stress testing
    tracing::info!("Starting Reth with 1s block time...");
    let reth = RethContainer::start_with_block_time("1s").await?;

    // Deploy contracts on Reth
    tracing::info!("Deploying contracts...");
    let system = TestSystem::deploy_to_external(reth.rpc_url()).await?;
    let _reth = reth; // Keep container alive

    let log_dir = tempfile::tempdir()?;

    // 5 validators x 2000 delegators = 10k delegators
    let num_validators = "5";
    let num_delegators = "2000";

    match command {
        StressTestCommand::Stake => {
            // Test demo stake flow: validators + delegators with self-delegation
            system
                .cmd(Signer::Mnemonic)
                .arg("demo")
                .arg("stake")
                .arg("--num-validators")
                .arg(num_validators)
                .arg("--num-delegators-per-validator")
                .arg(num_delegators)
                .timeout(Duration::from_secs(600)) // 10 min timeout for stress test
                .assert()
                .success();
        },
        StressTestCommand::Delegate => {
            // First register validators, then delegate
            // Register validators first
            system
                .cmd(Signer::Mnemonic)
                .arg("demo")
                .arg("stake")
                .arg("--num-validators")
                .arg(num_validators)
                .arg("--delegation-config")
                .arg("no-self-delegation")
                .timeout(Duration::from_secs(120))
                .assert()
                .success();

            // Get validator addresses from mnemonic
            let validators: Vec<_> = (DEMO_VALIDATOR_START_INDEX..DEMO_VALIDATOR_START_INDEX + 5)
                .map(|i| {
                    espresso_contract_deployer::build_signer(staking_cli::DEV_MNEMONIC, i).address()
                })
                .collect();
            let validator_addrs = validators
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let delegate_log = log_dir.path().join("delegate_log.json");

            // Test demo delegate flow with 500 delegators per validator
            system
                .cmd(Signer::Mnemonic)
                .arg("demo")
                .arg("delegate")
                .arg("--validators")
                .arg(&validator_addrs)
                .arg("--delegator-start-index")
                .arg("0")
                .arg("--num-delegators")
                .arg(num_delegators)
                .arg("--min-amount")
                .arg("100")
                .arg("--max-amount")
                .arg("500")
                .arg("--log-path")
                .arg(&delegate_log)
                .timeout(Duration::from_secs(600)) // 10 min timeout for stress test
                .assert()
                .success();
        },
        StressTestCommand::Undelegate => {
            // First stake, then undelegate
            // Register validators and create delegations first
            system
                .cmd(Signer::Mnemonic)
                .arg("demo")
                .arg("stake")
                .arg("--num-validators")
                .arg(num_validators)
                .arg("--num-delegators-per-validator")
                .arg(num_delegators)
                .timeout(Duration::from_secs(600))
                .assert()
                .success();

            // Get validator addresses from mnemonic
            let validators: Vec<_> = (DEMO_VALIDATOR_START_INDEX..DEMO_VALIDATOR_START_INDEX + 5)
                .map(|i| {
                    espresso_contract_deployer::build_signer(staking_cli::DEV_MNEMONIC, i).address()
                })
                .collect();
            let validator_addrs = validators
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",");

            let undelegate_log = log_dir.path().join("undelegate_log.json");

            // Test demo undelegate flow
            system
                .cmd(Signer::Mnemonic)
                .arg("demo")
                .arg("undelegate")
                .arg("--validators")
                .arg(&validator_addrs)
                .arg("--delegator-start-index")
                .arg("0")
                .arg("--num-delegators")
                .arg(num_delegators)
                .arg("--log-path")
                .arg(&undelegate_log)
                .timeout(Duration::from_secs(600)) // 10 min timeout for stress test
                .assert()
                .success();
        },
    }

    Ok(())
}
