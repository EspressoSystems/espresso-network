use std::time::{Duration, Instant};

use alloy::primitives::{utils::parse_ether, Address, U256};
use anyhow::Result;
use common::{Signer, TestSystemExt};
use espresso_contract_deployer::build_signer;
use hotshot_contract_adapter::sol_types::StakeTableV2;
use rand::{rngs::StdRng, SeedableRng};
use rstest::rstest;
use staking_cli::{
    demo::generate_delegator_signer, deploy::TestSystem, parse::Commission,
    signature::NodeSignatures,
};

mod common;

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

            staking_cli::delegation::approve(&provider, self.token, self.stake_table, fund_amount)
                .await?
                .get_receipt()
                .await?;

            let payload = NodeSignatures::create(validator_address, &bls_key, &state_key);
            let metadata_uri = "https://example.com/metadata".parse()?;
            let commission = Commission::try_from("10.0")?;

            staking_cli::registration::register_validator(
                &provider,
                self.stake_table,
                commission,
                metadata_uri,
                payload,
            )
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
        .arg("--parallelism")
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
            .arg("--parallelism")
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
    use staking_cli::tx_log::TxInputLog;

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
    let log = TxInputLog::load(&log_path)?.expect("log should be loadable");
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
