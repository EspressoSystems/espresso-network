//! Invariants of the stake table state machine and its commitment.
//!
//! The fixture pins catch changes against known histories; these catch changes against histories
//! nobody has recorded yet.

use alloy::primitives::{Address, U256};
use hotshot_contract_adapter::sol_types::{EdOnBN254PointSol, G2PointSol};
use proptest::{collection::vec, prelude::*};
use rand::SeedableRng;

use super::{testing::TestValidator, *};

/// Small pools keep collisions frequent: re-registration, over-undelegation, duplicate keys.
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

/// Deliberately includes actions that will fail: `apply_event` must leave the state untouched.
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
    /// Registers `.0` with `.1`'s Schnorr key. Tolerated: the V1 contract did not enforce
    /// Schnorr uniqueness.
    RegisterDuplicateSchnorr(usize, usize),
    /// Rotates `.0`'s keys to `.1`'s Schnorr key. Tolerated, same reason.
    RotateToDuplicateSchnorr(usize, usize),
    /// Rotates `.0` to an unparsable BLS key. Tolerated: the event is skipped.
    RotateToInvalidBls(usize),
    /// Rotates `.0` to an unparsable Schnorr key. Tolerated: the event is skipped.
    RotateToInvalidSchnorr(usize),
    /// Registers `.0` with `.1`'s BLS key. Fatal: the contract enforces BLS uniqueness, so on
    /// replay this means a corrupt log.
    RegisterDuplicateBls(usize, usize),
    /// Rotates `.0`'s keys to `.1`'s BLS key. Fatal, same reason.
    RotateToDuplicateBls(usize, usize),
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
        (v.clone(), any::<u16>()).prop_map(|(a, b)| Action::UpdateP2p(a, b)),
        // Without these the generated sequences never reach `Ok(Err(..))`.
        (v.clone(), v.clone()).prop_map(|(a, b)| Action::RegisterDuplicateSchnorr(a, b)),
        (v.clone(), v.clone()).prop_map(|(a, b)| Action::RotateToDuplicateSchnorr(a, b)),
        v.clone().prop_map(Action::RotateToInvalidBls),
        v.clone().prop_map(Action::RotateToInvalidSchnorr),
        (v.clone(), v.clone()).prop_map(|(a, b)| Action::RegisterDuplicateBls(a, b)),
        (v.clone(), v).prop_map(|(a, b)| Action::RotateToDuplicateBls(a, b)),
    ]
}

/// A BLS point that is not on the curve, so `BLSPubKey::try_from` rejects it.
fn unparsable_bls() -> G2PointSol {
    G2PointSol {
        x0: U256::ZERO,
        x1: U256::ZERO,
        y0: U256::ZERO,
        y1: U256::ZERO,
    }
}

/// A Schnorr point that is not on the curve, so `SchnorrPubKey::try_from` rejects it.
fn unparsable_schnorr() -> EdOnBN254PointSol {
    EdOnBN254PointSol {
        x: U256::ZERO,
        y: U256::ZERO,
    }
}

fn to_event(action: &Action, pool: &[TestValidator], delegators: &[Address]) -> StakeTableEvent {
    // Distinct nonzero, so an undelegation can be under, equal to, or over what was staked.
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
        Action::RegisterDuplicateSchnorr(i, j) => {
            let mut event = ValidatorRegistered::from(&pool[i]);
            event.schnorrVk = pool[j].schnorr_vk;
            StakeTableEvent::Register(event)
        },
        Action::RotateToDuplicateSchnorr(i, j) => {
            let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64((i * VALIDATORS + j) as u64);
            let rotated = pool[i].randomize_keys_with(&mut rng);
            let mut event = ConsensusKeysUpdated::from(&rotated);
            event.schnorrVK = pool[j].schnorr_vk;
            StakeTableEvent::KeyUpdate(event)
        },
        Action::RotateToInvalidBls(i) => {
            let mut event = ConsensusKeysUpdated::from(&pool[i]);
            event.blsVK = unparsable_bls();
            StakeTableEvent::KeyUpdate(event)
        },
        Action::RotateToInvalidSchnorr(i) => {
            let mut event = ConsensusKeysUpdated::from(&pool[i]);
            event.schnorrVK = unparsable_schnorr();
            StakeTableEvent::KeyUpdate(event)
        },
        Action::RegisterDuplicateBls(i, j) => {
            let mut event = ValidatorRegistered::from(&pool[i]);
            event.blsVk = pool[j].bls_vk;
            StakeTableEvent::Register(event)
        },
        Action::RotateToDuplicateBls(i, j) => {
            let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64((i * VALIDATORS + j) as u64);
            let rotated = pool[i].randomize_keys_with(&mut rng);
            let mut event = ConsensusKeysUpdated::from(&rotated);
            event.blsVK = pool[j].bls_vk;
            StakeTableEvent::KeyUpdate(event)
        },
    }
}

/// None of these events interact, so the resulting commitment must not depend on `order`.
fn register_and_fund(
    pool: &[TestValidator],
    delegators: &[Address],
    order: &[usize],
    stakes: &[u64],
) -> StakeTableState {
    let mut events = vec![];
    for &i in order {
        events.push(StakeTableEvent::RegisterV3((&pool[i]).into()));
        events.push(StakeTableEvent::Delegate(Delegated {
            delegator: delegators[i % DELEGATORS],
            validator: pool[i].account,
            amount: U256::from(stakes[i]) * U256::from(10u64).pow(U256::from(18)),
        }));
    }
    apply_all(&events)
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

    /// A partial mutation before a rejected event silently forks the chain.
    /// `test_apply_event_does_not_modify_state_on_error` covers four chosen cases; this covers
    /// every error a generated sequence can reach.
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
            match state.apply_event(event.clone()) {
                Ok(Ok(())) => {},
                // A tolerated error still rejects the event, so the state must be untouched.
                Ok(Err(err)) => prop_assert_eq!(
                    &state, &before,
                    "state mutated while tolerating {:?} on {:?}", err, event
                ),
                Err(err) => {
                    prop_assert_eq!(
                        &state, &before,
                        "state mutated while rejecting {:?} on {:?}", err, event
                    );
                    break;
                },
            }
        }
    }

    /// Guards against a commitment that depends on allocation addresses, iteration order, or
    /// time.
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

    /// `validators` is an insertion-ordered `IndexMap` and `delegators` a `HashMap`, so `commit`
    /// sorts both. Dropping either sort forks the network.
    #[test]
    fn commit_ignores_registration_order(
        seed in any::<u64>(),
        order_keys in vec(any::<u64>(), VALIDATORS),
        stakes in vec(1..5u64, VALIDATORS),
    ) {
        let pool = validator_pool(seed);
        let delegators = delegator_pool();

        let mut order: Vec<usize> = (0..VALIDATORS).collect();
        order.sort_by_key(|&i| order_keys[i]);

        let ascending: Vec<usize> = (0..VALIDATORS).collect();
        let a = register_and_fund(&pool, &delegators, &ascending, &stakes);
        let b = register_and_fund(&pool, &delegators, &order, &stakes);

        prop_assert_eq!(a.validators().len(), b.validators().len());
        prop_assert_eq!(
            a.commit(), b.commit(),
            "commitment changed under registration order {:?}", order
        );
    }
}

/// The permutation `commit_ignores_registration_order` shrinks to when `commit` stops sorting
/// `validators`, spelled out so the case survives without a stored proptest seed.
#[test]
fn commit_ignores_swapped_last_two_registrations() {
    let pool = validator_pool(0);
    let delegators = delegator_pool();
    let stakes = [1u64; VALIDATORS];

    let ascending = register_and_fund(&pool, &delegators, &[0, 1, 2, 3], &stakes);
    let swapped = register_and_fund(&pool, &delegators, &[0, 1, 3, 2], &stakes);

    assert_eq!(ascending.validators().len(), VALIDATORS);
    assert_eq!(
        ascending.commit(),
        swapped.commit(),
        "commitment depends on the order validators registered in"
    );
}

// The RLP codec crosses the persistence boundary and its `try_from` is fallible: a state that
// decodes to something else changes `stake_table_hash` after a restart.
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
