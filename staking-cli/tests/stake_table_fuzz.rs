//! Differential fuzz: the Rust stake table state machine against the real StakeTable contract.
//!
//! The checked-in history fixtures in `espresso-types` pin the pipeline against event sequences
//! that have already happened. This covers the ones that have not: it drives random validator
//! lifecycles through a real `StakeTableV3` on anvil, reads the emitted logs back through the
//! production fetcher, and checks the resulting `RegisteredValidatorMap` against the contract's
//! own view of itself.
//!
//! The contract is the oracle, which is what makes this stronger than a pure-Rust property test:
//! a Rust-only test can only check the state machine against itself.

use std::time::Duration;

use alloy::{
    network::EthereumWallet,
    primitives::{U256, utils::parse_ether},
    providers::{Provider, ProviderBuilder},
};
use anyhow::Result;
use espresso_types::{L1Client, L1ClientOptions, v0_3::Fetcher};
use hotshot_contract_adapter::{sol_types::StakeTableV3, stake_table::StakeTableContractVersion};
use hotshot_types::addr::NetAddr;
use proptest::{
    prelude::*,
    test_runner::{Config as ProptestConfig, TestRunner},
};
use staking_cli::{Commission, NodeSignatures, Transaction, deploy::TestSystem};

/// Every provider call is bounded by this, so a stuck transaction fails the test instead of
/// hanging it. Generous relative to anvil, which mines instantly.
const STEP_TIMEOUT: Duration = Duration::from_secs(20);

/// `StakeTable.ValidatorStatus`, whose Rust representation is a plain `u8`.
const VALIDATOR_UNKNOWN: u8 = 0;
const VALIDATOR_EXITED: u8 = 2;

/// The registration payload, built the same way `TestSystem::register_validator` builds it.
fn node_signatures(system: &TestSystem) -> NodeSignatures {
    NodeSignatures::create(
        system.deployer_address,
        &system.bls_key_pair.clone(),
        &system.state_key_pair.clone(),
    )
}

/// One step in a validator's lifecycle. Sequences are generated freely, so most contain steps
/// the contract will reject -- which is the point: a rejected transaction emits no event, so our
/// state must not move either.
#[derive(Clone, Debug)]
enum Step {
    Register,
    Delegate(u8),
    Undelegate(u8),
    UpdateKeys,
    UpdateCommission(u16),
    UpdateNetworkConfig(u8),
    Deregister,
}

fn step_strategy() -> impl Strategy<Value = Step> {
    prop_oneof![
        3 => Just(Step::Register),
        4 => (1..4u8).prop_map(Step::Delegate),
        4 => (1..4u8).prop_map(Step::Undelegate),
        2 => Just(Step::UpdateKeys),
        2 => (0..=10_000u16).prop_map(Step::UpdateCommission),
        2 => (1..4u8).prop_map(Step::UpdateNetworkConfig),
        1 => Just(Step::Deregister),
    ]
}

/// Submit one step, tolerating reverts. Returns `true` if it was mined successfully.
async fn submit(system: &TestSystem, sender: &impl Provider, step: &Step) -> Result<bool> {
    let stake_table = system.stake_table;
    let tx = match step {
        Step::Register => Transaction::RegisterValidator {
            stake_table,
            commission: system.commission,
            metadata_uri: "https://example.com/metadata".parse()?,
            payload: node_signatures(system),
            version: StakeTableContractVersion::V3,
            x25519_key: Some(system.x25519_keypair.public_key()),
            p2p_addr: Some("127.0.0.1:8080".parse()?),
        },
        Step::Delegate(n) => Transaction::Delegate {
            stake_table,
            validator: system.deployer_address,
            amount: parse_ether(&format!("{n}00"))?,
        },
        Step::Undelegate(n) => Transaction::Undelegate {
            stake_table,
            validator: system.deployer_address,
            amount: parse_ether(&format!("{n}00"))?,
        },
        Step::UpdateKeys => Transaction::UpdateConsensusKeys {
            stake_table,
            payload: node_signatures(system),
            version: StakeTableContractVersion::V3,
        },
        Step::UpdateCommission(bps) => Transaction::UpdateCommission {
            stake_table,
            new_commission: Commission::try_from(*bps)?,
        },
        Step::Deregister => Transaction::DeregisterValidator { stake_table },
        Step::UpdateNetworkConfig(n) => Transaction::UpdateNetworkConfig {
            stake_table,
            x25519_key: system.x25519_keypair.public_key(),
            p2p_addr: format!("127.0.0.1:900{n}").parse::<NetAddr>()?,
        },
    };

    // A revert is an expected outcome here, not a test failure.
    let Ok(pending) = tx.send(sender).await else {
        return Ok(false);
    };
    let Ok(receipt) = pending.get_receipt().await else {
        return Ok(false);
    };
    Ok(receipt.status())
}

/// Compare our derived stake table against the contract's own storage.
async fn assert_matches_contract(system: &TestSystem, l1: &L1Client) -> Result<()> {
    let head = system.provider.get_block_number().await?;
    let (validators, _hash) =
        Fetcher::fetch_all_validators_from_contract(l1.clone(), system.stake_table, head).await?;

    let contract = StakeTableV3::new(system.stake_table, &system.provider);
    let account = system.deployer_address;
    let on_chain = contract.validators(account).call().await?;
    let exited = on_chain.status == VALIDATOR_EXITED;

    match validators.get(&account) {
        Some(validator) => {
            assert!(
                !exited,
                "we hold a validator the contract reports as exited: {account:#x}"
            );
            assert_eq!(
                validator.stake, on_chain.delegatedAmount,
                "stake disagrees with the contract for {account:#x}"
            );

            let delegated = contract.delegations(account, account).call().await?;
            let ours = validator
                .delegators
                .get(&account)
                .copied()
                .unwrap_or(U256::ZERO);
            assert_eq!(
                ours, delegated,
                "delegation disagrees with the contract for {account:#x}"
            );

            // `stake` is defined as the sum of the delegations we track.
            let summed: U256 = validator.delegators.values().copied().sum();
            assert_eq!(
                summed, validator.stake,
                "stake is not the sum of delegators"
            );
        },
        None => assert!(
            exited || on_chain.status == VALIDATOR_UNKNOWN,
            "we dropped a validator the contract still reports as active: {account:#x}"
        ),
    }
    Ok(())
}

/// Random lifecycles, checked against the contract after every step.
///
/// A fresh chain per case: ~12 cases in a couple of seconds, and `PROPTEST_CASES` raises it
/// (150 cases run in about 90s locally).
#[test_log::test]
fn stake_table_matches_contract_under_random_lifecycles() {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(12);
    let mut runner = TestRunner::new(ProptestConfig {
        cases,
        // Shrinking replays whole lifecycles against a live chain; keep it bounded.
        max_shrink_iters: 24,
        ..ProptestConfig::default()
    });

    runner
        .run(&proptest::collection::vec(step_strategy(), 1..8), |steps| {
            rt.block_on(async {
                // A fresh chain per case: rewinding a shared one with `evm_revert` or
                // `load_state` leaves the provider's cached nonce ahead of the chain.
                let system = TestSystem::deploy_version(StakeTableContractVersion::V3)
                    .await
                    .expect("deploy");

                // `ProviderBuilder::new()` installs alloy's *cached* nonce filler, which consumes
                // a nonce even when the send then fails. After a reverted step the cache runs
                // ahead of the chain, so the next transaction carries a nonce gap, anvil queues
                // it instead of mining it, and `get_receipt` never returns. Reading the nonce
                // from the chain each time keeps one revert from poisoning every later step.
                let sender = ProviderBuilder::new()
                    .wallet(EthereumWallet::from(system.signer.clone()))
                    .with_simple_nonce_management()
                    .connect_http(system.rpc_url.clone());

                // One client per chain: building one per check exhausts the transport, and
                // the default 20-minute retry window turns that into a silent hang rather
                // than a failure. The short window makes a real fetch failure surface fast.
                let l1 = L1ClientOptions {
                    l1_events_max_retry_duration: Duration::from_secs(15),
                    l1_retry_delay: Duration::from_millis(200),
                    ..Default::default()
                }
                .connect(vec![system.rpc_url.clone()])
                .expect("l1 client");

                for step in &steps {
                    // Bounded: a transaction anvil accepts but never mines would otherwise
                    // stall `get_receipt` forever and hang CI rather than fail.
                    let mined = tokio::time::timeout(STEP_TIMEOUT, submit(&system, &sender, step))
                        .await
                        .unwrap_or_else(|_| panic!("submitting {step:?} timed out"))
                        .expect("submit");
                    // Checked after every step, accepted or not: a reverted transaction must
                    // leave our view of the chain exactly as it was.
                    tokio::time::timeout(STEP_TIMEOUT, assert_matches_contract(&system, &l1))
                        .await
                        .unwrap_or_else(|_| panic!("checking after {step:?} timed out"))
                        .unwrap_or_else(|e| panic!("after {step:?} (mined={mined}): {e}"));
                }
            });
            Ok(())
        })
        .unwrap();
}
