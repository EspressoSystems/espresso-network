//! Differential fuzz: the Rust stake table state machine against the real StakeTable contract.
//!
//! Drives random validator lifecycles through a `StakeTableV3` on anvil, reads the logs back
//! through the production fetcher, and checks the derived `RegisteredValidatorMap` against the
//! contract, an oracle a pure-Rust property test cannot have.
//!
//! Scope: one account, acting as its own delegator. Cross-validator interactions need several
//! funded signers and are left to the proptests in `espresso-types`.

use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

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
use rand::SeedableRng as _;
use staking_cli::{
    Commission, NodeSignatures, Transaction, demo::StakingKeySet, deploy::TestSystem,
};

/// Bounds every provider call, so a stuck transaction fails instead of hanging.
const STEP_TIMEOUT: Duration = Duration::from_secs(20);

/// `StakeTable.ValidatorStatus`, whose Rust representation is a plain `u8`.
const VALIDATOR_UNKNOWN: u8 = 0;
const VALIDATOR_EXITED: u8 = 2;

/// Namespaced by `kind`: the contract never frees a key it has seen, so a step reusing one
/// reverts on uniqueness and hides whatever it meant to exercise.
fn keys_for(kind: u64, seed: u8) -> StakingKeySet {
    TestSystem::gen_keys(&mut rand_chacha::ChaCha20Rng::seed_from_u64(
        kind << 8 | u64::from(seed),
    ))
}

const KIND_REGISTER: u64 = 0;
const KIND_ROTATE: u64 = 1;
const KIND_NETWORK: u64 = 2;

/// Most generated sequences contain steps the contract rejects, which is the point: a rejected
/// transaction emits no event, so our state must not move.
#[derive(Clone, Debug)]
enum Step {
    Register(u8),
    Delegate(u8),
    Undelegate(u8),
    UpdateKeys(u8),
    UpdateCommission(u16),
    UpdateNetworkConfig(u8),
    Deregister,
}

fn step_strategy() -> impl Strategy<Value = Step> {
    prop_oneof![
        3 => any::<u8>().prop_map(Step::Register),
        4 => (1..4u8).prop_map(Step::Delegate),
        4 => (1..4u8).prop_map(Step::Undelegate),
        2 => any::<u8>().prop_map(Step::UpdateKeys),
        2 => (0..=10_000u16).prop_map(Step::UpdateCommission),
        2 => (1..4u8).prop_map(Step::UpdateNetworkConfig),
        1 => Just(Step::Deregister),
    ]
}

/// Returns the consensus keys the step installs when mined; `None` if it was rejected or does
/// not touch keys.
async fn submit(
    system: &TestSystem,
    sender: &impl Provider,
    step: &Step,
) -> Result<(bool, Option<NodeSignatures>)> {
    let stake_table = system.stake_table;
    let mut installs_keys = None;
    let tx = match step {
        Step::Register(seed) => {
            let keys = keys_for(KIND_REGISTER, *seed);
            let payload = NodeSignatures::create(system.deployer_address, &keys.bls, &keys.state);
            installs_keys = Some(payload.clone());
            Transaction::RegisterValidator {
                stake_table,
                commission: system.commission,
                metadata_uri: "https://example.com/metadata".parse()?,
                payload,
                version: StakeTableContractVersion::V3,
                x25519_key: Some(keys.x25519.public_key()),
                p2p_addr: Some("127.0.0.1:8080".parse()?),
            }
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
        Step::UpdateKeys(seed) => {
            let keys = keys_for(KIND_ROTATE, *seed);
            let payload = NodeSignatures::create(system.deployer_address, &keys.bls, &keys.state);
            installs_keys = Some(payload.clone());
            Transaction::UpdateConsensusKeys {
                stake_table,
                payload,
                version: StakeTableContractVersion::V3,
            }
        },
        Step::UpdateCommission(bps) => Transaction::UpdateCommission {
            stake_table,
            new_commission: Commission::try_from(*bps)?,
        },
        Step::Deregister => Transaction::DeregisterValidator { stake_table },
        Step::UpdateNetworkConfig(n) => Transaction::UpdateNetworkConfig {
            stake_table,
            x25519_key: keys_for(KIND_NETWORK, *n).x25519.public_key(),
            p2p_addr: format!("127.0.0.1:900{n}").parse::<NetAddr>()?,
        },
    };

    // A revert is an expected outcome here, not a test failure.
    let Ok(pending) = tx.send(sender).await else {
        return Ok((false, None));
    };
    let Ok(receipt) = pending.get_receipt().await else {
        return Ok((false, None));
    };
    let mined = receipt.status();
    Ok((mined, if mined { installs_keys } else { None }))
}

/// Keys live only in the event log, so `expected_keys` (from the last mined registration or
/// rotation) stands in for the contract there.
async fn assert_matches_contract(
    system: &TestSystem,
    l1: &L1Client,
    expected_keys: Option<&NodeSignatures>,
) -> Result<()> {
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

            assert_eq!(
                validator.commission,
                system.fetch_commission().await?.to_evm(),
                "commission disagrees with the contract for {account:#x}"
            );

            if let Some(keys) = expected_keys {
                assert_eq!(
                    validator.stake_table_key,
                    Some(keys.bls_vk),
                    "BLS key does not match the last mined registration or rotation"
                );
                assert_eq!(
                    validator.state_ver_key.as_ref(),
                    Some(&keys.schnorr_vk),
                    "Schnorr key does not match the last mined registration or rotation"
                );
            }
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

    // A broken deploy or dead anvil makes every step revert, which `submit` reports as an
    // ordinary rejection, and the run would pass having tested nothing.
    let mined_steps = AtomicUsize::new(0);

    runner
        .run(&proptest::collection::vec(step_strategy(), 1..8), |steps| {
            rt.block_on(async {
                // A fresh chain per case: rewinding a shared one with `evm_revert` or
                // `load_state` leaves the provider's cached nonce ahead of the chain.
                let system = TestSystem::deploy_version(StakeTableContractVersion::V3)
                    .await
                    .expect("deploy");

                // Alloy's default *cached* nonce filler burns a nonce even when the send fails,
                // so after a revert the next transaction has a nonce gap and anvil queues it.
                let sender = ProviderBuilder::new()
                    .wallet(EthereumWallet::from(system.signer.clone()))
                    .with_simple_nonce_management()
                    .connect_http(system.rpc_url.clone());

                // One per chain: building one per check exhausts the transport, and the default
                // 20-minute retry window turns that into a hang rather than a failure.
                let l1 = L1ClientOptions {
                    l1_events_max_retry_duration: Duration::from_secs(15),
                    l1_retry_delay: Duration::from_millis(200),
                    ..Default::default()
                }
                .connect(vec![system.rpc_url.clone()])
                .expect("l1 client");

                // Cleared on a mined exit: the validator is gone, so there is nothing to check.
                let mut expected_keys: Option<NodeSignatures> = None;

                for step in &steps {
                    // Bounded: a transaction anvil accepts but never mines would stall
                    // `get_receipt` forever and hang CI rather than fail.
                    let (mined, installed) =
                        tokio::time::timeout(STEP_TIMEOUT, submit(&system, &sender, step))
                            .await
                            .unwrap_or_else(|_| panic!("submitting {step:?} timed out"))
                            .expect("submit");
                    if mined {
                        mined_steps.fetch_add(1, Ordering::Relaxed);
                    }
                    if let Some(keys) = installed {
                        expected_keys = Some(keys);
                    }
                    if mined && matches!(step, Step::Deregister) {
                        expected_keys = None;
                    }
                    // Checked after every step, accepted or not: a reverted transaction must
                    // leave our view of the chain exactly as it was.
                    tokio::time::timeout(
                        STEP_TIMEOUT,
                        assert_matches_contract(&system, &l1, expected_keys.as_ref()),
                    )
                    .await
                    .unwrap_or_else(|_| panic!("checking after {step:?} timed out"))
                    .unwrap_or_else(|e| panic!("after {step:?} (mined={mined}): {e}"));
                }
            });
            Ok(())
        })
        .unwrap();

    assert!(
        mined_steps.load(Ordering::Relaxed) > 0,
        "no step mined across {cases} cases; the harness is broken, not the state machine"
    );
}

/// A step the contract always rejects is dead coverage that still looks live, and the fuzz cannot
/// catch it because a rejected step is legitimate there. `UpdateNetworkConfig` was dead this way
/// until it stopped resubmitting the x25519 key registration had claimed.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn every_step_can_mine() {
    let system = TestSystem::deploy_version(StakeTableContractVersion::V3)
        .await
        .expect("deploy");
    let sender = ProviderBuilder::new()
        .wallet(EthereumWallet::from(system.signer.clone()))
        .with_simple_nonce_management()
        .connect_http(system.rpc_url.clone());

    // Ordered so each step is legal when it runs. The commission only decreases, which the
    // contract does not rate-limit.
    let lifecycle = [
        Step::Register(0),
        Step::Delegate(3),
        Step::UpdateCommission(system.commission.to_evm() / 2),
        Step::UpdateKeys(0),
        Step::UpdateNetworkConfig(1),
        Step::Undelegate(1),
        Step::Deregister,
    ];

    for step in &lifecycle {
        let (mined, _) = submit(&system, &sender, step).await.expect("submit");
        assert!(
            mined,
            "{step:?} cannot mine, so the fuzz never exercises it"
        );
    }
}

/// The contract rejects this on status (`ValidatorAlreadyExited`). The fuzz reaches it only when
/// a 4-step prefix lines up, so script it too.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn exited_validator_cannot_rejoin() {
    let system = TestSystem::deploy_version(StakeTableContractVersion::V3)
        .await
        .expect("deploy");
    let sender = ProviderBuilder::new()
        .wallet(EthereumWallet::from(system.signer.clone()))
        .with_simple_nonce_management()
        .connect_http(system.rpc_url.clone());

    for step in [Step::Register(0), Step::Delegate(1), Step::Deregister] {
        let (mined, _) = submit(&system, &sender, &step).await.expect("submit");
        assert!(mined, "{step:?} did not mine");
    }

    // Seed 7 shares no key material with seed 0, so only the status check can reject this.
    let (mined, _) = submit(&system, &sender, &Step::Register(7))
        .await
        .expect("submit");
    assert!(!mined, "an exited validator re-registered with fresh keys");
}
