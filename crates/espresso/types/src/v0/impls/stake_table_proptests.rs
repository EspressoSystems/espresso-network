//! Invariants of the stake table state machine and its commitment.
//!
//! These complement the fixture pins in `stake_table_history_tests.rs`: the fixtures catch a
//! change against known histories, these catch a change against histories nobody has recorded
//! yet. All of them are pure -- no RPC, no anvil -- so they run on every PR.

use alloy::primitives::{Address, U256};
use proptest::{collection::vec, prelude::*};
use rand::SeedableRng;

use super::{testing::TestValidator, *};

/// Small pools keep collisions (re-registration, over-undelegation, duplicate keys) frequent
/// enough that the interesting branches of `apply_event` are actually reached.
const VALIDATORS: usize = 4;
const DELEGATORS: usize = 3;

/// A validator pool derived from a seed, so a shrunk failure is reproducible from the seed alone.
fn validator_pool(seed: u64) -> Vec<TestValidator> {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    (0..VALIDATORS)
        .map(|_| TestValidator::random_with(&mut rng))
        .collect()
}

fn delegator_pool() -> Vec<Address> {
    (0..DELEGATORS)
        .map(|i| Address::repeat_byte(0xA0 + i as u8))
        .collect()
}

/// An action, resolved against the pools into a concrete `StakeTableEvent`.
///
/// Deliberately includes actions that will fail: the point is to reach the error paths, and
/// `apply_event`'s contract is that a failure leaves the state untouched.
#[derive(Clone, Debug)]
enum Action {
    Register(usize),
    RegisterV2(usize),
    RegisterV3(usize),
    RotateKeys(usize),
    Delegate(usize, usize, u64),
    Undelegate(usize, usize, u64),
    Exit(usize),
    UpdateCommission(usize, u16),
    UpdateX25519(usize, u8),
    UpdateP2p(usize, u16),
}

fn action_strategy() -> impl Strategy<Value = Action> {
    let v = 0..VALIDATORS;
    let d = 0..DELEGATORS;
    prop_oneof![
        v.clone().prop_map(Action::Register),
        v.clone().prop_map(Action::RegisterV2),
        v.clone().prop_map(Action::RegisterV3),
        v.clone().prop_map(Action::RotateKeys),
        (v.clone(), d.clone(), 0..3u64).prop_map(|(a, b, c)| Action::Delegate(a, b, c)),
        (v.clone(), d, 0..3u64).prop_map(|(a, b, c)| Action::Undelegate(a, b, c)),
        v.clone().prop_map(Action::Exit),
        (v.clone(), 0..=COMMISSION_BASIS_POINTS).prop_map(|(a, b)| Action::UpdateCommission(a, b)),
        (v.clone(), any::<u8>()).prop_map(|(a, b)| Action::UpdateX25519(a, b)),
        (v, any::<u16>()).prop_map(|(a, b)| Action::UpdateP2p(a, b)),
    ]
}

fn to_event(action: &Action, pool: &[TestValidator], delegators: &[Address]) -> StakeTableEvent {
    // Distinct nonzero amounts, so an undelegation can be smaller, equal, or larger than what
    // was delegated.
    let amount = |n: u64| U256::from(n + 1) * U256::from(10u64).pow(U256::from(18));
    match *action {
        Action::Register(i) => StakeTableEvent::Register((&pool[i]).into()),
        Action::RegisterV2(i) => StakeTableEvent::RegisterV2((&pool[i]).into()),
        Action::RegisterV3(i) => StakeTableEvent::RegisterV3((&pool[i]).into()),
        Action::RotateKeys(i) => {
            // Re-signed for the same account, so the rotation authenticates.
            let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(i as u64);
            StakeTableEvent::KeyUpdateV2((&pool[i].randomize_keys_with(&mut rng)).into())
        },
        Action::Delegate(i, j, amt) => StakeTableEvent::Delegate(Delegated {
            delegator: delegators[j],
            validator: pool[i].account,
            amount: amount(amt),
        }),
        Action::Undelegate(i, j, amt) => StakeTableEvent::Undelegate(Undelegated {
            delegator: delegators[j],
            validator: pool[i].account,
            amount: amount(amt),
        }),
        Action::Exit(i) => StakeTableEvent::Deregister(ValidatorExit {
            validator: pool[i].account,
        }),
        Action::UpdateCommission(i, c) => StakeTableEvent::CommissionUpdate(CommissionUpdated {
            validator: pool[i].account,
            timestamp: U256::from(1_700_000_000u64),
            oldCommission: pool[i].commission,
            newCommission: c,
        }),
        Action::UpdateX25519(i, b) => StakeTableEvent::X25519KeyUpdate(X25519KeyUpdated {
            validator: pool[i].account,
            x25519Key: alloy::primitives::FixedBytes([b; 32]),
        }),
        Action::UpdateP2p(i, port) => StakeTableEvent::P2pAddrUpdate(P2pAddrUpdated {
            validator: pool[i].account,
            p2pAddr: format!("127.0.0.1:{port}"),
        }),
    }
}

/// Apply a sequence, stopping at the first fatal error (as `validators_from_l1_events` does).
fn apply_all(events: &[StakeTableEvent]) -> StakeTableState {
    let mut state = StakeTableState::default();
    for event in events {
        if state.apply_event(event.clone()).is_err() {
            break;
        }
    }
    state
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 96, ..ProptestConfig::default() })]

    /// `apply_event` documents that it must not modify the state when it returns an error.
    ///
    /// The hand-written `test_apply_event_does_not_modify_state_on_error` checks four chosen
    /// cases; this checks it for every error any generated sequence can reach. A partial
    /// mutation before a rejected event would silently fork the chain.
    #[test]
    fn apply_event_never_mutates_on_error(
        seed in any::<u64>(),
        actions in vec(action_strategy(), 1..40),
    ) {
        let pool = validator_pool(seed);
        let delegators = delegator_pool();
        let mut state = StakeTableState::default();

        for action in &actions {
            let event = to_event(action, &pool, &delegators);
            let before = state.clone();
            if state.apply_event(event.clone()).is_err() {
                prop_assert_eq!(
                    &state, &before,
                    "state mutated while rejecting {:?}", event
                );
                break;
            }
        }
    }

    /// Replaying the same events yields the same commitment. Guards against a commitment that
    /// depends on allocation addresses, iteration nondeterminism, or time.
    #[test]
    fn replay_is_deterministic(
        seed in any::<u64>(),
        actions in vec(action_strategy(), 1..40),
    ) {
        let pool = validator_pool(seed);
        let delegators = delegator_pool();
        let events: Vec<_> = actions.iter().map(|a| to_event(a, &pool, &delegators)).collect();

        prop_assert_eq!(apply_all(&events).commit(), apply_all(&events).commit());
    }

    /// The commitment is a function of the state, not of the order independent validators
    /// happened to register in.
    ///
    /// `StakeTableState::validators` is an insertion-ordered `IndexMap` and `delegators` is a
    /// `HashMap`, so `commit` has to sort. Dropping either `sorted_by_key` would still pass
    /// every example-based test in this file but fail here -- and would fork the network.
    #[test]
    fn commit_ignores_registration_order(
        seed in any::<u64>(),
        order_keys in vec(any::<u64>(), VALIDATORS),
        stakes in vec(1..5u64, VALIDATORS),
    ) {
        let pool = validator_pool(seed);
        let delegators = delegator_pool();

        // A permutation of 0..VALIDATORS, ordered by proptest-generated keys.
        let mut order: Vec<usize> = (0..VALIDATORS).collect();
        order.sort_by_key(|&i| order_keys[i]);

        // Registering distinct validators and funding each from its own delegator is
        // order-independent: no two of these events interact.
        let build = |sequence: &[usize]| {
            let mut events = vec![];
            for &i in sequence {
                events.push(StakeTableEvent::RegisterV3((&pool[i]).into()));
                events.push(StakeTableEvent::Delegate(Delegated {
                    delegator: delegators[i % DELEGATORS],
                    validator: pool[i].account,
                    amount: U256::from(stakes[i]) * U256::from(10u64).pow(U256::from(18)),
                }));
            }
            apply_all(&events)
        };

        let ascending: Vec<usize> = (0..VALIDATORS).collect();
        let a = build(&ascending);
        let b = build(&order);

        prop_assert_eq!(a.validators().len(), b.validators().len());
        prop_assert_eq!(
            a.commit(), b.commit(),
            "commitment changed under registration order {:?}", order
        );
    }
}

// `StakeTableState`'s RLP codec has a fallible `try_from`, so round-tripping is a real risk.
// The codec is how the stake table crosses the persistence boundary; a state that encodes but
// decodes to something else would produce a different `stake_table_hash` after a restart.
#[cfg(feature = "rlp")]
proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    #[test]
    fn state_rlp_round_trips(
        seed in any::<u64>(),
        actions in vec(action_strategy(), 1..24),
    ) {
        use alloy_rlp::{Decodable, Encodable};

        let pool = validator_pool(seed);
        let delegators = delegator_pool();
        let events: Vec<_> = actions.iter().map(|a| to_event(a, &pool, &delegators)).collect();
        let state = apply_all(&events);

        let mut encoded = vec![];
        state.encode(&mut encoded);
        let decoded = StakeTableState::decode(&mut encoded.as_slice())
            .expect("a state we just encoded must decode");

        prop_assert_eq!(state.commit(), decoded.commit());
        prop_assert_eq!(state, decoded);
    }
}
