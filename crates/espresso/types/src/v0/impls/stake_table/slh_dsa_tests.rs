//! SLH-DSA keys in the stake table: the state rules, and the fetcher against StakeTableV4.
//!
//! Nodes on older releases do not read SLH-DSA key events. So the events must not change what
//! those nodes compute: the validators and the stake table hash.

use alloy::{
    node_bindings::{Anvil, AnvilInstance},
    primitives::{Address, Bytes, FixedBytes, U256},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    sol_types::SolCall,
};
use espresso_contract_deployer::{
    Contract, Contracts, DEFAULT_EXIT_ESCROW_PERIOD_SECONDS, builder::DeployerArgsBuilder,
    network_config::light_client_genesis_from_stake_table,
};
use hotshot_contract_adapter::{
    sol_types::{
        StakeTableV4::{self, SlhDsaKeyUpdated},
        stake_table_v4,
    },
    stake_table::sign_address_slh_dsa,
};
use hotshot_types::slh_dsa::{self, Keypair, PUBLIC_KEY_LEN, SIGNATURE_LEN};
use pretty_assertions::assert_matches;

use super::{testing::TestValidator, *};
use crate::L1ClientOptions;

fn key_bytes(fill: u8) -> [u8; PUBLIC_KEY_LEN] {
    [fill; PUBLIC_KEY_LEN]
}

fn key(fill: u8) -> slh_dsa::PublicKey {
    slh_dsa::PublicKey::try_from(key_bytes(fill).as_slice()).unwrap()
}

/// A key event with any signature bytes: `apply_event` does not check the proof of possession.
fn key_update(validator: Address, key: &[u8]) -> StakeTableEvent {
    SlhDsaKeyUpdated {
        validator,
        slhDsaVK: Bytes::copy_from_slice(key),
        slhDsaSig: Bytes::from(vec![0u8; SIGNATURE_LEN]),
    }
    .into()
}

fn state_with(validators: &[&TestValidator]) -> StakeTableState {
    let mut state = StakeTableState::default();
    for val in validators {
        state
            .apply_event(StakeTableEvent::RegisterV3((*val).into()))
            .unwrap()
            .unwrap();
    }
    state
}

#[test]
fn slh_dsa_key_update_sets_key() {
    let val = TestValidator::random();
    let mut state = state_with(&[&val]);
    assert_eq!(state.slh_dsa_key(&val.account), None);

    let result = state.apply_event(key_update(val.account, &key_bytes(1)));

    assert_matches!(result, Ok(Ok(())));
    assert_eq!(state.slh_dsa_key(&val.account), Some(&key(1)));
}

#[test]
fn slh_dsa_key_is_none_without_key_event() {
    let val_v2 = TestValidator::random();
    let val_v3 = TestValidator::random();
    let mut state = state_with(&[&val_v3]);
    state
        .apply_event(StakeTableEvent::RegisterV2((&val_v2).into()))
        .unwrap()
        .unwrap();

    assert_eq!(state.slh_dsa_key(&val_v2.account), None);
    assert_eq!(state.slh_dsa_key(&val_v3.account), None);
    assert_eq!(state.slh_dsa_key(&Address::random()), None);
}

/// The core property: old nodes ignore key events, so the events must change neither the stake
/// table hash nor the validators.
#[test]
fn slh_dsa_key_update_keeps_commit_and_validators() {
    let val_1 = TestValidator::random();
    let val_2 = TestValidator::random();
    let mut state = state_with(&[&val_1]);
    let events: [StakeTableEvent; 4] = [
        StakeTableEvent::RegisterV2((&val_2).into()),
        Delegated {
            delegator: Address::random(),
            validator: val_1.account,
            amount: U256::from(10),
        }
        .into(),
        Delegated {
            delegator: Address::random(),
            validator: val_2.account,
            amount: U256::from(5),
        }
        .into(),
        val_2.x25519_update([9u8; 32]),
    ];
    for event in events {
        state.apply_event(event).unwrap().unwrap();
    }
    let commit = state.commit();
    let validators = state.validators().clone();

    for (validator, fill) in [(val_1.account, 1), (val_2.account, 2), (val_1.account, 3)] {
        let result = state.apply_event(key_update(validator, &key_bytes(fill)));
        assert_matches!(result, Ok(Ok(())));
        assert_eq!(state.commit(), commit);
        assert_eq!(state.validators(), &validators);
    }

    assert_eq!(state.slh_dsa_key(&val_1.account), Some(&key(3)));
    assert_eq!(state.slh_dsa_key(&val_2.account), Some(&key(2)));
}

#[test]
fn slh_dsa_key_update_unknown_validator_is_fatal() {
    let val = TestValidator::random();
    let mut state = state_with(&[&val]);
    let unknown = Address::random();
    let before = state.clone();

    let result = state.apply_event(key_update(unknown, &key_bytes(1)));

    assert_matches!(
        result,
        Err(StakeTableError::ValidatorNotFound(addr)) if addr == unknown
    );
    assert_eq!(state, before);
    assert_eq!(state.slh_dsa_key(&unknown), None);
    // The failed event did not use up the key.
    assert_matches!(
        state.apply_event(key_update(val.account, &key_bytes(1))),
        Ok(Ok(()))
    );
}

#[test]
fn slh_dsa_key_of_other_validator_is_fatal() {
    let val_1 = TestValidator::random();
    let val_2 = TestValidator::random();
    let mut state = state_with(&[&val_1, &val_2]);
    state
        .apply_event(key_update(val_1.account, &key_bytes(1)))
        .unwrap()
        .unwrap();
    let before = state.clone();

    let result = state.apply_event(key_update(val_2.account, &key_bytes(1)));

    assert_matches!(result, Err(StakeTableError::SlhDsaKeyAlreadyUsed(_)));
    assert_eq!(state, before);
    assert_eq!(state.slh_dsa_key(&val_1.account), Some(&key(1)));
    assert_eq!(state.slh_dsa_key(&val_2.account), None);
}

/// Keys are never freed: after a rotation, nobody can set the old key again, also not the
/// validator that used it.
#[test]
fn slh_dsa_key_used_before_is_fatal() {
    let val_1 = TestValidator::random();
    let val_2 = TestValidator::random();
    let mut state = state_with(&[&val_1, &val_2]);
    state
        .apply_event(key_update(val_1.account, &key_bytes(1)))
        .unwrap()
        .unwrap();

    let before = state.clone();
    let result = state.apply_event(key_update(val_1.account, &key_bytes(1)));
    assert_matches!(result, Err(StakeTableError::SlhDsaKeyAlreadyUsed(_)));
    assert_eq!(state, before);

    state
        .apply_event(key_update(val_1.account, &key_bytes(2)))
        .unwrap()
        .unwrap();
    let before = state.clone();
    for validator in [val_1.account, val_2.account] {
        let result = state.apply_event(key_update(validator, &key_bytes(1)));
        assert_matches!(result, Err(StakeTableError::SlhDsaKeyAlreadyUsed(_)));
        assert_eq!(state, before);
    }
    assert_eq!(state.slh_dsa_key(&val_1.account), Some(&key(2)));
    assert_eq!(state.slh_dsa_key(&val_2.account), None);
}

#[test]
fn slh_dsa_key_rotation_replaces_key() {
    let val = TestValidator::random();
    let mut state = state_with(&[&val]);

    for fill in [1, 2] {
        let result = state.apply_event(key_update(val.account, &key_bytes(fill)));
        assert_matches!(result, Ok(Ok(())));
    }

    assert_eq!(state.slh_dsa_key(&val.account), Some(&key(2)));
}

/// The fetcher drops events with a key of the wrong length. If one reaches the state anyway, it
/// is ignored, like an invalid x25519 key: no error and no change.
#[test]
fn slh_dsa_key_wrong_length_is_ignored() {
    let without_key = TestValidator::random();
    let with_key = TestValidator::random();
    let mut state = state_with(&[&without_key, &with_key]);
    state
        .apply_event(key_update(with_key.account, &key_bytes(1)))
        .unwrap()
        .unwrap();
    let before = state.clone();

    for len in [
        0,
        PUBLIC_KEY_LEN - 1,
        PUBLIC_KEY_LEN + 1,
        2 * PUBLIC_KEY_LEN,
    ] {
        for val in [&without_key, &with_key] {
            let result = state.apply_event(key_update(val.account, &vec![5u8; len]));
            assert_matches!(result, Ok(Ok(())), "key length {len}");
            assert_eq!(state, before, "key length {len}");
        }
    }
    assert_eq!(state.slh_dsa_key(&without_key.account), None);
    assert_eq!(state.slh_dsa_key(&with_key.account), Some(&key(1)));
}

/// "Old node vs new node": the same L1 history with and without key events gives the same
/// validators and the same stake table hash.
#[test]
fn validators_from_l1_events_ignores_slh_dsa_key_events() {
    let val_v1 = TestValidator::random();
    let val_v2 = TestValidator::random();
    let val_v3 = TestValidator::random();
    let val_v4 = TestValidator::random();
    let delegator = Address::random();
    let delegate = |validator: Address, amount: u64| -> StakeTableEvent {
        Delegated {
            delegator,
            validator,
            amount: U256::from(amount),
        }
        .into()
    };

    // (event, whether only new nodes see it)
    let history: Vec<(StakeTableEvent, bool)> = vec![
        (ValidatorRegistered::from(&val_v1).into(), false),
        (ValidatorRegisteredV2::from(&val_v2).into(), false),
        (delegate(val_v1.account, 10), false),
        (delegate(val_v2.account, 20), false),
        (StakeTableEvent::RegisterV3((&val_v3).into()), false),
        (key_update(val_v1.account, &key_bytes(1)), true),
        (key_update(val_v2.account, &key_bytes(2)), true),
        (delegate(val_v3.account, 30), false),
        (val_v2.x25519_update([9u8; 32]), false),
        (
            Undelegated {
                delegator,
                validator: val_v2.account,
                amount: U256::from(5),
            }
            .into(),
            false,
        ),
        // A registerValidatorV4 call: the registration, then the key event.
        (StakeTableEvent::RegisterV3((&val_v4).into()), false),
        (key_update(val_v4.account, &key_bytes(4)), true),
        (delegate(val_v4.account, 40), false),
        // Rotation.
        (key_update(val_v1.account, &key_bytes(5)), true),
        (ValidatorExit::from(&val_v1).into(), false),
    ];
    let new_node_events = history.iter().map(|(e, _)| e.clone());
    let old_node_events = history
        .iter()
        .filter(|(_, new_only)| !new_only)
        .map(|(e, _)| e.clone());

    let (old_validators, old_hash) = validators_from_l1_events(old_node_events).unwrap();
    let (new_validators, new_hash) = validators_from_l1_events(new_node_events).unwrap();

    assert_eq!(new_validators, old_validators);
    assert_eq!(new_hash, old_hash);
    for val in [&val_v2, &val_v3, &val_v4] {
        assert!(new_validators.contains_key(&val.account));
    }

    // A new node applies every event and keeps the keys.
    let mut state = StakeTableState::default();
    for (event, _) in history {
        let result = state.apply_event(event.clone());
        assert_matches!(result, Ok(Ok(())), "{event:?}");
    }
    assert_eq!(state.slh_dsa_key(&val_v2.account), Some(&key(2)));
    assert_eq!(state.slh_dsa_key(&val_v3.account), None);
    assert_eq!(state.slh_dsa_key(&val_v4.account), Some(&key(4)));
}

/// The fetcher reads key events from a real StakeTableV4 contract, drops the ones with a bad
/// proof of possession, and old and new nodes still compute the same stake table.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn fetcher_reads_slh_dsa_key_events_from_contract() -> anyhow::Result<()> {
    let anvil = Anvil::new().args(["--slots-in-an-epoch", "1"]).spawn();
    let (deployer, proxy) = deploy_stake_table_v3(&anvil).await?;

    // Anvil account 0 deploys; validator `i` uses account `i`, since the BLS and Schnorr proofs
    // sign the sender address.
    let [val_1, val_2, val_3] = [1, 2, 3].map(|i| anvil_validator(&anvil, i));
    register_v3(&anvil, 1, proxy, &val_1).await?;
    register_v3(&anvil, 2, proxy, &val_2).await?;

    upgrade_to_v4(&deployer, proxy).await?;

    let keypair_1 = Keypair::generate();
    let keypair_2 = Keypair::generate();
    let keypair_3 = Keypair::generate();
    let pop_1 = sign_address_slh_dsa(&keypair_1, val_1.account);
    let pop_3 = sign_address_slh_dsa(&keypair_3, val_3.account);
    // Right length, but it signs another address. The contract does not check it.
    let bad_pop_2 = sign_address_slh_dsa(&keypair_2, val_1.account);

    let st_1 = StakeTableV4::new(proxy, validator_provider(&anvil, 1));
    let receipt = st_1
        .updateSlhDsaKey(
            Bytes::copy_from_slice(keypair_1.public_key().as_bytes()),
            Bytes::copy_from_slice(pop_1.as_bytes()),
        )
        .send()
        .await?
        .get_receipt()
        .await?;
    assert!(receipt.status());

    register_v4(&anvil, 3, proxy, &val_3, &keypair_3, &pop_3).await?;

    let st_2 = StakeTableV4::new(proxy, validator_provider(&anvil, 2));
    let receipt = st_2
        .updateSlhDsaKey(
            Bytes::copy_from_slice(keypair_2.public_key().as_bytes()),
            Bytes::copy_from_slice(bad_pop_2.as_bytes()),
        )
        .send()
        .await?
        .get_receipt()
        .await?;
    assert!(receipt.status());

    let l1_client = L1ClientOptions::default().connect(vec![anvil.endpoint_url()])?;
    let to_block = deployer.get_block_number().await?;
    let events =
        Fetcher::fetch_events_from_contract(l1_client.clone(), proxy, None, to_block).await?;
    let single_request =
        Fetcher::try_fetch_events_from_contract(l1_client, proxy, 0, to_block).await?;
    assert_eq!(single_request, events);

    // Validator 2's event with the bad proof is dropped.
    let key_events: Vec<&SlhDsaKeyUpdated> = events
        .iter()
        .filter_map(|(_, e)| match e {
            StakeTableEvent::SlhDsaKeyUpdate(update) => Some(update),
            _ => None,
        })
        .collect();
    assert_eq!(
        key_events.iter().map(|e| e.validator).collect::<Vec<_>>(),
        vec![val_1.account, val_3.account]
    );
    assert_eq!(
        key_events[0].slhDsaVK.as_ref(),
        keypair_1.public_key().as_bytes()
    );
    assert_eq!(
        key_events[1].slhDsaVK.as_ref(),
        keypair_3.public_key().as_bytes()
    );

    // registerValidatorV4 shows up as a V3 registration, followed by the key event.
    let registration_3 = events
        .iter()
        .position(
            |(_, e)| matches!(e, StakeTableEvent::RegisterV3(r) if r.account == val_3.account),
        )
        .expect("validator 3 registration is fetched");
    let key_3 = events
        .iter()
        .position(
            |(_, e)| matches!(e, StakeTableEvent::SlhDsaKeyUpdate(u) if u.validator == val_3.account),
        )
        .expect("validator 3 key event is fetched");
    assert!(registration_3 < key_3);
    assert_eq!(events[registration_3].0.0, events[key_3].0.0);

    // An old node does not fetch key events.
    let old_node_events = events
        .iter()
        .map(|(_, e)| e.clone())
        .filter(|e| !matches!(e, StakeTableEvent::SlhDsaKeyUpdate(_)));
    let (old_validators, old_hash) = validators_from_l1_events(old_node_events)?;
    let (new_validators, new_hash) =
        validators_from_l1_events(events.iter().map(|(_, e)| e.clone()))?;
    assert_eq!(new_validators, old_validators);
    assert_eq!(new_hash, old_hash);
    for val in [&val_1, &val_2, &val_3] {
        assert!(new_validators.contains_key(&val.account));
    }

    let mut state = StakeTableState::default();
    for (_, event) in events {
        state.apply_event(event)??;
    }
    assert_eq!(
        state.slh_dsa_key(&val_1.account),
        Some(&keypair_1.public_key())
    );
    assert_eq!(state.slh_dsa_key(&val_2.account), None);
    assert_eq!(
        state.slh_dsa_key(&val_3.account),
        Some(&keypair_3.public_key())
    );
    Ok(())
}

/// Deploy the contracts up to StakeTableV3. Returns the deployer's provider and the stake table
/// proxy address.
async fn deploy_stake_table_v3(anvil: &AnvilInstance) -> anyhow::Result<(impl Provider, Address)> {
    let wallet = anvil.wallet().unwrap();
    let deployer_address = wallet.default_signer().address();
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_http(anvil.endpoint_url());
    let (genesis_state, genesis_stake) =
        light_client_genesis_from_stake_table(&Default::default(), 10)?;
    let mut contracts = Contracts::new();
    let args = DeployerArgsBuilder::default()
        .deployer(provider.clone())
        .rpc_url(anvil.endpoint_url())
        .mock_light_client(true)
        .genesis_lc_state(genesis_state)
        .genesis_st_state(genesis_stake)
        .blocks_per_epoch(100)
        .epoch_start_block(1)
        .multisig_pauser(deployer_address)
        .exit_escrow_period(U256::from(DEFAULT_EXIT_ESCROW_PERIOD_SECONDS))
        .token_name("Espresso".to_string())
        .token_symbol("ESP".to_string())
        .initial_token_supply(U256::from(100000u64))
        .ops_timelock_delay(U256::from(0))
        .ops_timelock_admin(deployer_address)
        .ops_timelock_proposers(vec![deployer_address])
        .ops_timelock_executors(vec![deployer_address])
        .safe_exit_timelock_delay(U256::from(10))
        .safe_exit_timelock_admin(deployer_address)
        .safe_exit_timelock_proposers(vec![deployer_address])
        .safe_exit_timelock_executors(vec![deployer_address])
        .build()?;
    args.deploy_to_stake_table_v3(&mut contracts).await?;
    let proxy = contracts
        .address(Contract::StakeTableProxy)
        .expect("stake table proxy is deployed");
    Ok((provider, proxy))
}

async fn upgrade_to_v4(deployer: &impl Provider, proxy: Address) -> anyhow::Result<()> {
    let v4 = StakeTableV4::deploy(deployer).await?;
    let init = StakeTableV4::initializeV4Call {}.abi_encode();
    let receipt = StakeTableV4::new(proxy, deployer)
        .upgradeToAndCall(*v4.address(), init.into())
        .send()
        .await?
        .get_receipt()
        .await?;
    assert!(receipt.status());
    Ok(())
}

fn validator_provider(anvil: &AnvilInstance, index: usize) -> impl Provider {
    let signer: PrivateKeySigner = anvil.keys()[index].clone().into();
    ProviderBuilder::new()
        .wallet(signer)
        .connect_http(anvil.endpoint_url())
}

/// A validator for anvil account `index`, with x25519 key and p2p address the contract accepts.
fn anvil_validator(anvil: &AnvilInstance, index: usize) -> TestValidator {
    TestValidator::random_update_keys(anvil.addresses()[index], 100)
        .with_x25519_key([index as u8; 32])
        .with_p2p_addr(format!("127.0.0.1:{}", 9000 + index))
}

async fn register_v3(
    anvil: &AnvilInstance,
    index: usize,
    proxy: Address,
    val: &TestValidator,
) -> anyhow::Result<()> {
    let receipt = StakeTableV3::new(proxy, validator_provider(anvil, index))
        .registerValidatorV3(
            val.bls_vk,
            val.schnorr_vk,
            val.bls_sig.into(),
            val.schnorr_sig.clone(),
            val.commission,
            String::new(),
            FixedBytes(val.x25519_key),
            val.p2p_addr.clone(),
        )
        .send()
        .await?
        .get_receipt()
        .await?;
    assert!(receipt.status());
    Ok(())
}

async fn register_v4(
    anvil: &AnvilInstance,
    index: usize,
    proxy: Address,
    val: &TestValidator,
    keypair: &Keypair,
    pop: &slh_dsa::Signature,
) -> anyhow::Result<()> {
    let receipt = StakeTableV4::new(proxy, validator_provider(anvil, index))
        .registerValidatorV4(
            stake_table_v4::BN254::G2Point {
                x0: val.bls_vk.x0,
                x1: val.bls_vk.x1,
                y0: val.bls_vk.y0,
                y1: val.bls_vk.y1,
            },
            stake_table_v4::EdOnBN254::EdOnBN254Point {
                x: val.schnorr_vk.x,
                y: val.schnorr_vk.y,
            },
            stake_table_v4::BN254::G1Point {
                x: val.bls_sig.x,
                y: val.bls_sig.y,
            },
            val.schnorr_sig.clone(),
            val.commission,
            String::new(),
            FixedBytes(val.x25519_key),
            val.p2p_addr.clone(),
            Bytes::copy_from_slice(keypair.public_key().as_bytes()),
            Bytes::copy_from_slice(pop.as_bytes()),
        )
        .send()
        .await?
        .get_receipt()
        .await?;
    assert!(receipt.status());
    Ok(())
}
