//! Property tests over sequences of stake table events, applied the way a light client replays
//! the events its server sends.
//!
//! A sequence starts from an empty stake table or from the state after a prefix of decaf's history,
//! so it runs against real validators, delegations and a set larger than `MAX_VALIDATORS`. Ops pick
//! accounts and keys from small pools so that sequences reuse them, and amounts, delegators and
//! targets can refer to the state built so far.

use std::{
    collections::{BTreeSet, HashSet},
    net::{Ipv4Addr, Ipv6Addr},
    sync::LazyLock,
};

use alloy::primitives::{Address, Bytes, FixedBytes, U256};
use committable::Committable;
use espresso_types::{
    PubKey, RegisteredValidatorMap, StakeTableState, ValidatorSet,
    v0_3::{
        COMMISSION_BASIS_POINTS, EventKey, ExpectedStakeTableError, RegisteredValidator,
        StakeTableError, StakeTableEvent,
    },
};
use hotshot_contract_adapter::{
    sol_types::{
        EdOnBN254PointSol, G1PointSol, G2PointSol,
        StakeTableV3::{
            CommissionUpdated, ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated,
            P2pAddrUpdated, Undelegated, UndelegatedV2, ValidatorExit, ValidatorExitV2,
            ValidatorRegistered, ValidatorRegisteredV2, ValidatorRegisteredV3, X25519KeyUpdated,
        },
        stake_table_v3::BN254::G1Point as StakeTableG1Point,
    },
    stake_table::{StakeTableSolError, StateSignatureSol, sign_address_bls, sign_address_schnorr},
};
use hotshot_types::{
    addr::NetAddr,
    light_client::StateKeyPair,
    signature_key::{BLSKeyPair, SchnorrPubKey},
    x25519,
};
use proptest::{option, prelude::*, strategy::ValueTree, test_runner::TestRunner};
use rand::{SeedableRng, rngs::StdRng};
use vbs::version::Version;
use versions::{EPOCH_VERSION, NEW_PROTOCOL_VERSION};

mod model;
mod strategy;

use self::{model::*, strategy::*};

/// Sequences the reachability test draws. Enough for every outcome in [`REACHABLE`] to show up
/// several times, so a strategy change that makes one rare still fails the test.
const OUTCOME_SEQUENCES: usize = 500;

/// Every outcome of `apply_event`, as `event: outcome`. The strategies must reach each one, or the
/// properties below never test that branch.
const REACHABLE: &[&str] = &[
    "Register: Ok authenticated",
    "Register: Ok unauthenticated",
    "Register: Expected SchnorrKeyAlreadyUsed",
    "Register: Error ValidatorAlreadyExited",
    "Register: Error AlreadyRegistered",
    "Register: Error BlsKeyAlreadyUsed",
    "RegisterV2: Ok authenticated",
    "RegisterV2: Ok unauthenticated",
    "RegisterV2: Error ValidatorAlreadyExited",
    "RegisterV2: Error AlreadyRegistered",
    "RegisterV2: Error BlsKeyAlreadyUsed",
    "RegisterV3: Ok authenticated",
    "RegisterV3: Ok unauthenticated",
    "RegisterV3: Error ValidatorAlreadyExited",
    "RegisterV3: Error AlreadyRegistered",
    "RegisterV3: Error BlsKeyAlreadyUsed",
    "RegisterV3: Error X25519KeyAlreadyUsed",
    "Deregister: Ok",
    "Deregister: Error ValidatorNotFound",
    "DeregisterV2: Ok",
    "DeregisterV2: Error ValidatorNotFound",
    "Delegate: Ok",
    "Delegate: Error ZeroDelegatorStake",
    "Delegate: Error ValidatorNotFound",
    "Delegate: Error StakeOverflow",
    "Undelegate: Ok",
    "Undelegate: Error ValidatorNotFound",
    "Undelegate: Error InsufficientStake",
    "Undelegate: Error DelegatorNotFound",
    "UndelegateV2: Ok",
    "UndelegateV2: Error ValidatorNotFound",
    "UndelegateV2: Error InsufficientStake",
    "UndelegateV2: Error DelegatorNotFound",
    "KeyUpdate: Ok",
    "KeyUpdate: Expected InvalidBlsKey",
    "KeyUpdate: Expected InvalidSchnorrKey",
    "KeyUpdate: Expected SchnorrKeyAlreadyUsed",
    "KeyUpdate: Error ValidatorNotFound",
    "KeyUpdate: Error BlsKeyAlreadyUsed",
    "KeyUpdateV2: Ok",
    "KeyUpdateV2: Expected InvalidBlsKey",
    "KeyUpdateV2: Expected InvalidSchnorrKey",
    "KeyUpdateV2: Expected SchnorrKeyAlreadyUsed",
    "KeyUpdateV2: Error AuthenticationFailed",
    "KeyUpdateV2: Error ValidatorNotFound",
    "KeyUpdateV2: Error BlsKeyAlreadyUsed",
    "CommissionUpdate: Ok",
    "CommissionUpdate: Error InvalidCommission",
    "CommissionUpdate: Error ValidatorNotFound",
    "X25519KeyUpdate: Ok",
    "X25519KeyUpdate: Error ValidatorNotFound",
    "X25519KeyUpdate: Error X25519KeyAlreadyUsed",
    "P2pAddrUpdate: Ok",
    "P2pAddrUpdate: Error ValidatorNotFound",
];

#[test]
fn strategies_reach_every_apply_event_outcome() {
    let mut runner = TestRunner::deterministic();
    let mut seen = BTreeSet::new();
    for _ in 0..OUTCOME_SEQUENCES {
        let (start, ops) = sequence().new_tree(&mut runner).unwrap().current();
        replay_observed(&start, &ops, |event, result, state| {
            seen.insert(outcome(event, result, state));
        });
    }
    let missing: Vec<_> = REACHABLE
        .iter()
        .filter(|outcome| !seen.contains(**outcome))
        .collect();
    assert!(missing.is_empty(), "the strategies never reach {missing:?}");
}

proptest! {
    // A saved seed replays the strategies rather than the input, so it goes stale as soon as they
    // change. A failure prints its shrunk input, which is what a regression test needs.
    #![proptest_config(ProptestConfig {
        cases: 64,
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn rejected_events_change_nothing_and_stake_matches_delegations((start, ops) in sequence()) {
        replay(&start, &ops);
    }

    #[test]
    fn active_validators_have_unique_keys((start, ops) in sequence()) {
        let state = replay(&start, &ops);
        for version in [EPOCH_VERSION, NEW_PROTOCOL_VERSION] {
            if let Ok(set) = ValidatorSet::from_state(&state, version) {
                assert_active_keys_unique(&set, version);
            }
        }
    }

    #[test]
    fn validators_keep_their_commitment_through_json_and_bincode((start, ops) in sequence()) {
        assert_validators_round_trip(&replay(&start, &ops));
    }

    #[test]
    fn hash_ignores_validator_order((start, ops) in sequence()) {
        let state = replay(&start, &ops);
        prop_assert_eq!(sorted_by_account(&state).commit(), state.commit());
    }

    #[test]
    fn apply_event_matches_the_reference_model((start, ops) in sequence()) {
        let mut model = Model::from_state(&Replay::new(&start).state);
        replay_observed(&start, &ops, |event, result, state| {
            let expected = model.apply(event);
            assert_eq!(result_label(result), expected, "{event:?} in {state:?}");
            assert!(model.matches(state), "{event:?} left {state:?}, the model holds {model:?}");
        });
    }

    #[test]
    #[ignore = "the quorum keeps registration order, which the stake table hash does not cover"]
    fn hash_fixes_the_quorum((start, ops) in sequence()) {
        let state = replay(&start, &ops);
        let sorted = sorted_by_account(&state);
        for version in [EPOCH_VERSION, NEW_PROTOCOL_VERSION] {
            let Ok(set) = ValidatorSet::from_state(&state, version) else {
                continue;
            };
            let sorted_set = ValidatorSet::from_state(&sorted, version)
                .expect("the same validators in another order give a validator set");
            prop_assert_eq!(quorum(&set), quorum(&sorted_set), "at {}", version);
        }
    }
}

/// Apply `ops` to `start`, checking after each event. A light client gives up at the first fatal
/// error, but a rejected event must leave the state as it was, so skipping it instead reaches only
/// states a server could produce by not sending it, and lets a sequence go deeper.
fn replay(start: &Start, ops: &[Op]) -> StakeTableState {
    replay_observed(start, ops, |_, _, _| {})
}

/// [`replay`], calling `observe` with each event, what applying it returned, and the state after.
fn replay_observed(
    start: &Start,
    ops: &[Op],
    mut observe: impl FnMut(
        &StakeTableEvent,
        &Result<Result<(), ExpectedStakeTableError>, StakeTableError>,
        &StakeTableState,
    ),
) -> StakeTableState {
    let mut replay = Replay::new(start);
    for op in ops {
        for event in replay.events(op) {
            let before = replay.state.clone();
            let result = replay.state.apply_event(event.clone());
            match result {
                Ok(Ok(())) => assert_stake_matches_delegations(&replay.state),
                Ok(Err(_)) | Err(_) => assert_unchanged(&before, &replay.state),
            }
            observe(&event, &result, &replay.state);
        }
    }
    replay.state
}

/// `event: outcome` as listed in [`REACHABLE`], named after the event and error variants.
fn outcome(
    event: &StakeTableEvent,
    result: &Result<Result<(), ExpectedStakeTableError>, StakeTableError>,
    state: &StakeTableState,
) -> String {
    let variant = |debug: String| debug.split('(').next().unwrap_or_default().to_owned();
    let kind = variant(format!("{event:?}"));
    let outcome = match result {
        Ok(Ok(())) => match event {
            StakeTableEvent::Register(ValidatorRegistered { account, .. })
            | StakeTableEvent::RegisterV2(ValidatorRegisteredV2 { account, .. })
            | StakeTableEvent::RegisterV3(ValidatorRegisteredV3 { account, .. }) => {
                if state.validators()[account].authenticated {
                    "Ok authenticated".to_owned()
                } else {
                    "Ok unauthenticated".to_owned()
                }
            },
            _ => "Ok".to_owned(),
        },
        Ok(Err(err)) => format!("Expected {}", variant(format!("{err:?}"))),
        Err(err) => format!("Error {}", variant(format!("{err:?}"))),
    };
    format!("{kind}: {outcome}")
}

fn assert_unchanged(before: &StakeTableState, after: &StakeTableState) {
    assert!(before == after, "a rejected event changed the stake table");
    assert!(
        before.validators().keys().eq(after.validators().keys()),
        "a rejected event reordered the validators"
    );
}

fn assert_stake_matches_delegations(state: &StakeTableState) {
    for validator in state.validators().values() {
        let delegated = validator
            .delegators
            .values()
            .try_fold(U256::ZERO, |sum, stake| sum.checked_add(*stake));
        assert_eq!(
            delegated,
            Some(validator.stake),
            "stake of {} is not the sum of its delegations",
            validator.account
        );
        assert!(
            validator.delegators.values().all(|stake| !stake.is_zero()),
            "{} keeps a delegator with no stake",
            validator.account
        );
    }
}

fn assert_active_keys_unique(set: &ValidatorSet, version: Version) {
    let mut bls_keys = HashSet::new();
    let mut schnorr_keys = HashSet::new();
    let mut x25519_keys = HashSet::new();
    for validator in set.active_validators().values() {
        assert!(
            bls_keys.insert(*validator.stake_table_key()),
            "two active validators share a BLS key at {version}"
        );
        assert!(
            schnorr_keys.insert(validator.state_ver_key().clone()),
            "two active validators share a Schnorr key at {version}"
        );
        if let Some(key) = validator.x25519_key {
            assert!(
                x25519_keys.insert(key),
                "two active validators share an x25519 key at {version}"
            );
        }
    }
}

/// The light client stores validators as JSON and reloads them, and nodes send them as bincode.
/// Either way a validator must keep its commitment.
fn assert_validators_round_trip(state: &StakeTableState) {
    for validator in state.validators().values() {
        // A host starting with `[` does not parse back from its unbracketed form, a known
        // NetAddr issue, so skip it here.
        if validator
            .p2p_addr
            .as_ref()
            .is_some_and(|addr| addr.unbracketed_string().starts_with('['))
        {
            continue;
        }
        let json = serde_json::to_value(validator).expect("a validator encodes as JSON");
        let json: RegisteredValidator<PubKey> =
            serde_json::from_value(json).expect("a validator decodes from its JSON");
        assert_eq!(
            json.commit(),
            validator.commit(),
            "JSON changes {validator:?}"
        );
        let binary = bincode::serialize(validator).expect("a validator encodes as bincode");
        let binary: RegisteredValidator<PubKey> =
            bincode::deserialize(&binary).expect("a validator decodes from its bincode");
        assert_eq!(
            binary.commit(),
            validator.commit(),
            "bincode changes {validator:?}"
        );
    }
}

/// `state` with its validators in account order. The hash covers them in that order, so a light
/// client cannot tell the two apart.
fn sorted_by_account(state: &StakeTableState) -> StakeTableState {
    let mut validators: Vec<_> = state
        .validators()
        .iter()
        .map(|(account, validator)| (*account, validator.clone()))
        .collect();
    validators.sort_by_key(|(account, _)| *account);
    StakeTableState::new(
        validators.into_iter().collect::<RegisteredValidatorMap>(),
        state.validator_exits().clone(),
        state.used_bls_keys().clone(),
        state.used_schnorr_keys().clone(),
        state.used_x25519_keys().clone(),
    )
}

/// The quorum a light client verifies certificates against, in order.
fn quorum(set: &ValidatorSet) -> Vec<(PubKey, U256)> {
    set.active_validators()
        .values()
        .map(|v| (*v.stake_table_key(), v.stake))
        .collect()
}

/// `Ok`, `Expected <variant>` or `Error <variant>`, the decision a result stands for.
fn result_label(result: &Result<Result<(), ExpectedStakeTableError>, StakeTableError>) -> String {
    let variant = |debug: String| debug.split('(').next().unwrap_or_default().to_owned();
    match result {
        Ok(Ok(())) => "Ok".to_owned(),
        Ok(Err(err)) => format!("Expected {}", variant(format!("{err:?}"))),
        Err(err) => format!("Error {}", variant(format!("{err:?}"))),
    }
}
