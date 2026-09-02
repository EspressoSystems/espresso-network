//! Invariants of the stake table state machine and its commitment.
//!
//! The fixture pins catch changes against known histories; these catch changes against histories
//! nobody has recorded yet. "Yet" is the constraint: a case starts from decaf's or mainnet's
//! current state and extends it with events the V3 contract can still emit, so a counterexample
//! is a sequence that could happen next on a live chain rather than one that never can.
//!
//! Variants `REACHABILITY` classifies as Historical or Dead are therefore absent from the
//! generator. Nothing is lost by that: the fixtures hold every Historical log that will ever
//! exist and the pin replays them, the states those logs produce are in the seed, and Dead
//! processing cannot fork anything. `every_generated_event_is_live` keeps it that way.

use std::{
    collections::{BTreeMap, HashSet},
    sync::OnceLock,
};

use alloy::primitives::{Address, FixedBytes, U256};
use proptest::{collection::vec, prelude::*, strategy::ValueTree, test_runner::TestRunner};
use rand::SeedableRng;
use versions::NEW_PROTOCOL_VERSION;

use super::{testing::*, *};

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

/// Which live history a case starts from.
///
/// Both, rather than one: decaf is the only corpus carrying validators registered through the V1
/// contract, mainnet the larger V3 set.
#[derive(Clone, Copy, Debug)]
enum Seed {
    Decaf,
    Mainnet,
}

impl Seed {
    fn corpus(self) -> &'static str {
        match self {
            Seed::Decaf => "decaf",
            Seed::Mainnet => "mainnet",
        }
    }
}

fn seed_strategy() -> impl Strategy<Value = Seed> {
    prop_oneof![Just(Seed::Decaf), Just(Seed::Mainnet)]
}

/// A live corpus replayed into a state, with its validator addresses as fuzz targets.
///
/// Replayed once per test binary and cloned per case: 796 and 3463 events are cheap once and
/// ruinous ~200 times.
fn seeded(seed: Seed) -> &'static (StakeTableState, Vec<Address>) {
    static DECAF: OnceLock<(StakeTableState, Vec<Address>)> = OnceLock::new();
    static MAINNET: OnceLock<(StakeTableState, Vec<Address>)> = OnceLock::new();

    let cell = match seed {
        Seed::Decaf => &DECAF,
        Seed::Mainnet => &MAINNET,
    };
    cell.get_or_init(|| {
        let logs = super::history_tests::load_logs(seed.corpus());
        let events = events_from_logs(logs).expect("fixture logs decode, validate, and sort");
        let mut state = StakeTableState::default();
        for (_, event) in events {
            state
                .apply_event(event)
                .expect("fixture history applies without a fatal error")
                .ok();
        }
        let addresses = state.validators().keys().copied().collect();
        (state, addresses)
    })
}

fn seed_state(seed: Seed) -> StakeTableState {
    seeded(seed).0.clone()
}

fn seed_validators(seed: Seed) -> &'static [Address] {
    &seeded(seed).1
}

/// Whose validator an action acts on.
///
/// `Existing` is an index into [`seed_validators`], so a case can delegate to, rotate, or exit a
/// validator a live chain actually registered. `Fresh` is the synthetic pool, so registration of
/// a genuinely new account is still exercised.
#[derive(Clone, Copy, Debug)]
enum Target {
    Existing(usize),
    Fresh(usize),
}

/// The p2p address forms the generator draws from.
///
/// `NetAddr::from_str` rejects only an empty string and an unparsable port; everything else
/// falls through to `Name(host, 0)`. So "unparsable" means a bad port, not a bad host, and the
/// interesting axis is which *shape* survives to `unbracketed_string`, the projection
/// `RegisteredValidator::commit` actually hashes.
const P2P_ADDRS: [&str; 12] = [
    "8.8.8.8:9000",               // globally routable v4
    "127.0.0.1:9000",             // loopback
    "10.0.0.1:9000",              // RFC 1918
    "169.254.0.1:9000",           // link-local
    "255.255.255.255:9000",       // broadcast
    "[2001:db8::1]:9000",         // bracketed v6
    "2001:db8::1",                // bare v6, no port
    "[::1]:9000",                 // v6 loopback
    "validator.example.com:9000", // hostname
    "localhost:9000",             // the one host name treated as non-global
    "example.com",                // hostname, no port -> port 0
    "host:notaport",              // the only genuinely unparsable form here
];

impl Target {
    /// A distinct rotation seed per target. A constant here makes every rotation mint the same
    /// key pair, so the first succeeds and every later one dies on `BlsKeyAlreadyUsed`.
    fn key_seed(self) -> u64 {
        match self {
            Target::Existing(i) => i as u64,
            Target::Fresh(i) => 1_000_000 + i as u64,
        }
    }
}

fn target_strategy() -> impl Strategy<Value = Target> {
    prop_oneof![
        any::<u16>().prop_map(|i| Target::Existing(i as usize)),
        (0..VALIDATORS).prop_map(Target::Fresh),
    ]
}

/// The validator an action acts on, with keys we can sign for.
///
/// For an `Existing` target this mints *fresh* keys for a real address rather than recovering the
/// validator's own. That authenticates because `authenticate_bls_sig` and
/// `authenticate_schnorr_sig` are proofs of possession of the **new** keys over the account
/// address (`contracts/rust/adapter/src/stake_table.rs:150`); nothing in the replay path involves
/// the key being replaced.
fn validator_for(target: Target, pool: &[TestValidator], seed: Seed) -> TestValidator {
    match target {
        Target::Fresh(i) => pool[i % pool.len()].clone(),
        Target::Existing(i) => {
            let addresses = seed_validators(seed);
            let account = addresses[i % addresses.len()];
            let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(i as u64);
            TestValidator::random_update_keys_with(&mut rng, account, 500)
        },
    }
}

/// Deliberately includes actions that will fail: `apply_event` must leave the state untouched.
///
/// Every variant is [`Reachability::Live`], enforced by `every_action_is_live`. Historical and
/// Dead variants are left out on purpose: their processing is frozen exactly by
/// `stake_table_history_pin`, or free to change, so fuzzing them only produces false flags.
#[derive(Clone, Debug)]
enum Action {
    Register(Target),
    RotateKeys(Target),
    Delegate(Target, usize, u64),
    Undelegate(Target, usize, u64),
    Exit(Target),
    UpdateCommission(Target, u16),
    UpdateX25519(Target, u8),
    UpdateP2p(Target, usize),
    /// Registers with an unparsable BLS key. Tolerated: the validator is marked unauthenticated.
    RegisterInvalidBls(Target),
    /// Rotates to an unparsable BLS key. Tolerated: the event is skipped.
    RotateToInvalidBls(Target),
    /// Rotates to an unparsable Schnorr key. Tolerated, same reason.
    RotateToInvalidSchnorr(Target),
    /// Zero x25519 key. Tolerated: degrades to `None`, dropping the validator at 0.6.
    UpdateX25519Zero(Target),
    /// Non-canonical x25519 key. Tolerated, same reason.
    UpdateX25519Noncanonical(Target),
    /// Unparsable p2p address. Tolerated, same reason.
    UpdateP2pUnparsable(Target),
    /// Registers `.0` with `.1`'s x25519 key. Tolerated: the key is dropped.
    RegisterDuplicateX25519(Target, Target),
    /// Registers `.0` with `.1`'s BLS key. Fatal: the contract enforces BLS uniqueness, so on
    /// replay this means a corrupt log. Kept despite being unreachable on V3, because the arm
    /// guards replay against a bad log source rather than against a live chain.
    RegisterDuplicateBls(Target, Target),
    /// Rotates `.0`'s keys to `.1`'s BLS key. Fatal, same reason.
    RotateToDuplicateBls(Target, Target),
}

fn action_strategy() -> impl Strategy<Value = Action> {
    let t = target_strategy;
    let d = 0..DELEGATORS;
    prop_oneof![
        t().prop_map(Action::Register),
        t().prop_map(Action::RotateKeys),
        (t(), d.clone(), 0..3u64).prop_map(|(a, b, c)| Action::Delegate(a, b, c)),
        (t(), d, 0..3u64).prop_map(|(a, b, c)| Action::Undelegate(a, b, c)),
        t().prop_map(Action::Exit),
        (t(), 0..=COMMISSION_BASIS_POINTS).prop_map(|(a, b)| Action::UpdateCommission(a, b)),
        (t(), any::<u8>()).prop_map(|(a, b)| Action::UpdateX25519(a, b)),
        (t(), 0..P2P_ADDRS.len()).prop_map(|(a, b)| Action::UpdateP2p(a, b)),
        // Without these the generated sequences never reach `Ok(Err(..))` or `Err(..)`.
        t().prop_map(Action::RegisterInvalidBls),
        t().prop_map(Action::RotateToInvalidBls),
        t().prop_map(Action::RotateToInvalidSchnorr),
        t().prop_map(Action::UpdateX25519Zero),
        t().prop_map(Action::UpdateX25519Noncanonical),
        t().prop_map(Action::UpdateP2pUnparsable),
        (t(), t()).prop_map(|(a, b)| Action::RegisterDuplicateX25519(a, b)),
        (t(), t()).prop_map(|(a, b)| Action::RegisterDuplicateBls(a, b)),
        (t(), t()).prop_map(|(a, b)| Action::RotateToDuplicateBls(a, b)),
    ]
}

fn to_event(
    action: &Action,
    pool: &[TestValidator],
    delegators: &[Address],
    seed: Seed,
) -> StakeTableEvent {
    // Distinct nonzero, so an undelegation can be under, equal to, or over what was staked.
    let amount = |n: u64| U256::from(n + 1) * U256::from(10u64).pow(U256::from(18));
    let val = |t: Target| validator_for(t, pool, seed);

    match *action {
        Action::Register(t) => StakeTableEvent::RegisterV3((&val(t)).into()),
        Action::RotateKeys(t) => {
            let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(t.key_seed());
            let rotated = val(t).randomize_keys_with(&mut rng);
            StakeTableEvent::KeyUpdateV2((&rotated).into())
        },
        Action::Delegate(t, j, amt) => StakeTableEvent::Delegate(Delegated {
            delegator: delegators[j % delegators.len()],
            validator: val(t).account,
            amount: amount(amt),
        }),
        Action::Undelegate(t, j, amt) => StakeTableEvent::UndelegateV2(UndelegatedV2 {
            delegator: delegators[j % delegators.len()],
            validator: val(t).account,
            undelegationId: amt,
            amount: amount(amt),
            unlocksAt: U256::from(1_700_086_400u64),
        }),
        Action::Exit(t) => StakeTableEvent::DeregisterV2(ValidatorExitV2 {
            validator: val(t).account,
            unlocksAt: U256::from(1_700_172_800u64),
        }),
        Action::UpdateCommission(t, c) => StakeTableEvent::CommissionUpdate(CommissionUpdated {
            validator: val(t).account,
            timestamp: U256::from(1_700_000_000u64),
            oldCommission: 0,
            newCommission: c,
        }),
        Action::UpdateX25519(t, b) => StakeTableEvent::X25519KeyUpdate(X25519KeyUpdated {
            validator: val(t).account,
            x25519Key: FixedBytes([b | 1; 32]),
        }),
        Action::UpdateP2p(t, i) => StakeTableEvent::P2pAddrUpdate(P2pAddrUpdated {
            validator: val(t).account,
            p2pAddr: P2P_ADDRS[i % P2P_ADDRS.len()].to_string(),
        }),
        Action::RegisterInvalidBls(t) => {
            let mut event = ValidatorRegisteredV3::from(&val(t));
            event.blsVK = zero_g2();
            StakeTableEvent::RegisterV3(event)
        },
        Action::RotateToInvalidBls(t) => {
            let mut event = ConsensusKeysUpdatedV2::from(&val(t));
            event.blsVK = zero_g2();
            StakeTableEvent::KeyUpdateV2(event)
        },
        Action::RotateToInvalidSchnorr(t) => {
            let mut event = ConsensusKeysUpdatedV2::from(&val(t));
            event.schnorrVK = zero_ed_on_bn254();
            StakeTableEvent::KeyUpdateV2(event)
        },
        Action::UpdateX25519Zero(t) => StakeTableEvent::X25519KeyUpdate(X25519KeyUpdated {
            validator: val(t).account,
            x25519Key: FixedBytes([0u8; 32]),
        }),
        Action::UpdateX25519Noncanonical(t) => StakeTableEvent::X25519KeyUpdate(X25519KeyUpdated {
            validator: val(t).account,
            x25519Key: FixedBytes(noncanonical_x25519_key()),
        }),
        Action::UpdateP2pUnparsable(t) => StakeTableEvent::P2pAddrUpdate(P2pAddrUpdated {
            validator: val(t).account,
            p2pAddr: "host:notaport".to_string(),
        }),
        Action::RegisterDuplicateX25519(t, u) => {
            let mut event = ValidatorRegisteredV3::from(&val(t));
            event.x25519Key = FixedBytes(val(u).x25519_key);
            StakeTableEvent::RegisterV3(event)
        },
        Action::RegisterDuplicateBls(t, u) => {
            let mut event = ValidatorRegisteredV3::from(&val(t));
            event.blsVK = val(u).bls_vk;
            StakeTableEvent::RegisterV3(event)
        },
        Action::RotateToDuplicateBls(t, u) => {
            let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(t.key_seed() ^ 0x5eed);
            let rotated = val(t).randomize_keys_with(&mut rng);
            let mut event = ConsensusKeysUpdatedV2::from(&rotated);
            event.blsVK = val(u).bls_vk;
            StakeTableEvent::KeyUpdateV2(event)
        },
    }
}

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
    apply_all(StakeTableState::default(), &events)
}

/// Apply a sequence, stopping at the first fatal error (as `validators_from_l1_events` does).
fn apply_all(mut state: StakeTableState, events: &[StakeTableEvent]) -> StakeTableState {
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
        corpus in seed_strategy(),
        actions in vec(action_strategy(), 1..40),
    ) {
        let pool = validator_pool(seed);
        let delegators = delegator_pool();
        let mut state = seed_state(corpus);

        for action in &actions {
            let event = to_event(action, &pool, &delegators, corpus);
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
        corpus in seed_strategy(),
        actions in vec(action_strategy(), 1..40),
    ) {
        let pool = validator_pool(seed);
        let delegators = delegator_pool();
        let events: Vec<_> = actions
            .iter()
            .map(|a| to_event(a, &pool, &delegators, corpus))
            .collect();

        prop_assert_eq!(
            apply_all(seed_state(corpus), &events).commit(),
            apply_all(seed_state(corpus), &events).commit()
        );
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
        corpus in seed_strategy(),
        actions in vec(action_strategy(), 1..24),
    ) {
        use alloy_rlp::{Decodable, Encodable};

        let pool = validator_pool(seed);
        let delegators = delegator_pool();
        let events: Vec<_> = actions
            .iter()
            .map(|a| to_event(a, &pool, &delegators, corpus))
            .collect();
        let state = apply_all(seed_state(corpus), &events);

        let mut encoded = vec![];
        state.encode(&mut encoded);
        let decoded = StakeTableState::decode(&mut encoded.as_slice())
            .expect("a state we just encoded must decode");

        prop_assert_eq!(state.commit(), decoded.commit());
        prop_assert_eq!(state, decoded);
    }
}

/// The pruning that made this generator V3-only has to stay done.
///
/// Reintroducing a `Historical` or `Dead` variant would put the fuzz back to constraining code
/// that cannot fork a live chain, which is the failure this file was rewritten to remove.
#[test]
fn every_generated_event_is_live() {
    use super::history_tests::{REACHABILITY, Reachability, variant_name};

    let live: HashSet<&str> = REACHABILITY
        .iter()
        .filter(|(_, r)| matches!(r, Reachability::Live))
        .map(|(name, _)| *name)
        .collect();

    let pool = validator_pool(0);
    let delegators = delegator_pool();
    let mut runner = TestRunner::deterministic();

    for _ in 0..500 {
        let action = action_strategy().new_tree(&mut runner).unwrap().current();
        let name = variant_name(&to_event(&action, &pool, &delegators, Seed::Decaf));
        assert!(
            live.contains(name),
            "{action:?} generates {name}, which REACHABILITY does not classify as Live. Only \
             variants the V3 contract can still emit belong in this generator."
        );
    }
}

/// Without this, pruning the V1-based error actions could leave
/// `apply_event_never_mutates_on_error` never reaching an error at all, and it would still pass.
#[test]
fn error_arms_are_reachable() {
    let pool = validator_pool(0);
    let delegators = delegator_pool();
    let t = Target::Fresh(0);
    let u = Target::Fresh(1);

    let actions = [
        Action::Register(t),
        Action::Delegate(t, 0, 1),
        Action::RotateToInvalidBls(t),
        Action::RotateToInvalidSchnorr(t),
        Action::UpdateX25519Zero(t),
        Action::UpdateX25519Noncanonical(t),
        Action::UpdateP2pUnparsable(t),
        Action::Register(u),
        Action::RegisterDuplicateBls(u, t),
    ];

    let (mut tolerated, mut fatal) = (0, 0);
    let mut state = StakeTableState::default();
    for action in &actions {
        match state.apply_event(to_event(action, &pool, &delegators, Seed::Decaf)) {
            Ok(Ok(())) => {},
            Ok(Err(_)) => tolerated += 1,
            Err(_) => fatal += 1,
        }
    }

    assert!(tolerated > 0, "no Ok(Err(..)) arm reached");
    assert!(fatal > 0, "no Err(..) arm reached");
}

/// The seed is the point: fuzzing an empty state explores one no chain has been in since its
/// first registration.
#[test]
fn seeds_are_real_chain_state() {
    for corpus in [Seed::Decaf, Seed::Mainnet] {
        let (state, addresses) = seeded(corpus);
        assert!(
            !addresses.is_empty(),
            "{} seeded to an empty validator set",
            corpus.corpus()
        );
        assert_eq!(
            addresses.len(),
            state.validators().len(),
            "{} targets and state disagree",
            corpus.corpus()
        );
        assert!(
            addresses.iter().all(|a| state.validators().contains_key(a)),
            "{} offers a target it did not register",
            corpus.corpus()
        );
    }
}

/// The claim the seeding rests on: a key rotation for a validator whose key we do not hold still
/// authenticates, because the signature proves possession of the *new* keys over the account
/// address and never involves the key being replaced.
///
/// If this breaks, `Target::Existing` silently degrades to generating events that are dropped in
/// validation, and the fuzz stops touching real validators without failing.
#[test]
fn existing_validator_can_be_rekeyed() {
    let corpus = Seed::Decaf;
    let mut state = seed_state(corpus);
    let account = *seed_validators(corpus)
        .iter()
        .find(|a| state.validators()[*a].stake_table_key.is_some())
        .expect("decaf registered an authenticated validator");
    let before = state.validators()[&account].stake_table_key;

    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(0);
    let rotated = TestValidator::random_update_keys_with(&mut rng, account, 500);
    let event = StakeTableEvent::KeyUpdateV2((&rotated).into());

    state
        .apply_event(event)
        .expect("rotation is not fatal")
        .expect("minted keys authenticate for an address we do not hold the key for");

    let after = state.validators()[&account].stake_table_key;
    assert_ne!(before, after, "rotation left the key unchanged");
}

/// Names every [`ExpectedStakeTableError`], the errors `apply_event` tolerates by rejecting the
/// event and continuing. Adding one fails to compile here until it is named, so the pin below
/// cannot silently stop reporting a degradation path.
fn tolerated_variant_name(err: &ExpectedStakeTableError) -> &'static str {
    match err {
        ExpectedStakeTableError::SchnorrKeyAlreadyUsed(_) => "SchnorrKeyAlreadyUsed",
        ExpectedStakeTableError::InvalidBlsKey => "InvalidBlsKey",
        ExpectedStakeTableError::InvalidSchnorrKey => "InvalidSchnorrKey",
    }
}

/// Names every [`StakeTableError`]. These halt replay rather than degrading silently, so one
/// reaching a live chain is a stuck node rather than a fork.
fn fatal_variant_name(err: &StakeTableError) -> &'static str {
    match err {
        StakeTableError::AlreadyRegistered(_) => "AlreadyRegistered",
        StakeTableError::ValidatorNotFound(_) => "ValidatorNotFound",
        StakeTableError::DelegatorNotFound(_) => "DelegatorNotFound",
        StakeTableError::BlsKeyAlreadyUsed(_) => "BlsKeyAlreadyUsed",
        StakeTableError::InsufficientStake => "InsufficientStake",
        StakeTableError::AuthenticationFailed(_) => "AuthenticationFailed",
        StakeTableError::NoValidValidators => "NoValidValidators",
        StakeTableError::MissingMaximumStake => "MissingMaximumStake",
        StakeTableError::MinimumStakeOverflow => "MinimumStakeOverflow",
        StakeTableError::ZeroDelegatorStake(_) => "ZeroDelegatorStake",
        StakeTableError::HashError(_) => "HashError",
        StakeTableError::ValidatorAlreadyExited(_) => "ValidatorAlreadyExited",
        StakeTableError::InvalidCommission(..) => "InvalidCommission",
        StakeTableError::SchnorrKeyAlreadyUsed(_) => "SchnorrKeyAlreadyUsed",
        StakeTableError::X25519KeyAlreadyUsed(_) => "X25519KeyAlreadyUsed",
        StakeTableError::InvalidX25519Key(_) => "InvalidX25519Key",
        StakeTableError::StakeTableEventDecodeError(_) => "StakeTableEventDecodeError",
        StakeTableError::EventSortingError(_) => "EventSortingError",
    }
}

/// How many actions the pinned run generates. Large enough to reach the error arms repeatedly,
/// small enough that the run stays under a second.
const PINNED_RUN_ACTIONS: usize = 400;

#[derive(serde::Serialize)]
struct ArmSummary {
    seed: String,
    actions: usize,
    applied: usize,
    tolerated: BTreeMap<&'static str, usize>,
    fatal: BTreeMap<&'static str, usize>,
    terminal_commit: String,
    /// Selection is gated by `is_eligible`, which reads x25519 and p2p. A rule added there rather
    /// than at the decoder forks the epoch committee while moving no other line here, because
    /// every real and every generated validator is currently eligible.
    active_commit: Option<String>,
    active_validators: Option<usize>,
}

/// Pins what the fixtures structurally cannot reach.
///
/// decaf and mainnet apply all 4259 of their events cleanly, so `stake_table_history_pin` pins
/// zero error handling. A change to a tolerated branch therefore keeps both it and the invariant
/// properties green, and forks the chain the day such an event lands. This replays a fixed
/// generated run on top of each seed state and pins which arms it reaches and where it ends up.
///
/// The run is data, not a fixture: it regenerates from `TestRunner::deterministic()` rather than
/// being checked in. Editing [`action_strategy`] moves these numbers, which is expected and shows
/// up in the same diff; moving them *without* touching the generator means processing changed.
///
/// A fatal error is recorded and its event skipped rather than halting, so one corrupt-log case
/// cannot truncate the run. `apply_event_never_mutates_on_error` is what guarantees skipping is
/// well defined.
#[test]
fn reachable_error_arms_are_pinned() {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path(super::history_tests::data_dir().join("insta_snapshots"));
    settings.set_prepend_module_to_snapshot(false);

    let summaries: Vec<ArmSummary> = [Seed::Decaf, Seed::Mainnet]
        .into_iter()
        .map(|corpus| {
            let pool = validator_pool(0);
            let delegators = delegator_pool();
            let mut runner = TestRunner::deterministic();
            let mut state = seed_state(corpus);
            let (mut applied, mut tolerated, mut fatal) = (0, BTreeMap::new(), BTreeMap::new());

            for _ in 0..PINNED_RUN_ACTIONS {
                let action = action_strategy().new_tree(&mut runner).unwrap().current();
                let event = to_event(&action, &pool, &delegators, corpus);
                match state.apply_event(event) {
                    Ok(Ok(())) => applied += 1,
                    Ok(Err(err)) => {
                        *tolerated.entry(tolerated_variant_name(&err)).or_insert(0) += 1
                    },
                    Err(err) => *fatal.entry(fatal_variant_name(&err)).or_insert(0) += 1,
                }
            }

            let selected = select_active_validator_set(state.validators(), NEW_PROTOCOL_VERSION);

            ArmSummary {
                seed: corpus.corpus().into(),
                actions: PINNED_RUN_ACTIONS,
                applied,
                tolerated,
                fatal,
                terminal_commit: state.commit().to_string(),
                active_commit: selected.as_ref().ok().map(|a| {
                    StakeTableState::new(
                        to_registered_validator_map(a),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                        Default::default(),
                    )
                    .commit()
                    .to_string()
                }),
                active_validators: selected.as_ref().ok().map(|a| a.len()),
            }
        })
        .collect();

    settings.bind(|| insta::assert_yaml_snapshot!("stake_table_reachable_arms", summaries));
}

/// A set-valued field of `StakeTableState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Slot {
    ValidatorExits,
    UsedBlsKeys,
    UsedSchnorrKeys,
    UsedX25519Keys,
}

/// What `apply_event` is allowed to touch for one event.
///
/// Most handlers are near-local: they read and write the entry for their own account and at most
/// two of the key sets. Declaring that makes the blast radius of a change visible without reading
/// the handler, and `apply_event_stays_within_footprint` turns the declaration into an enforced
/// invariant rather than a comment.
///
/// The payoff is that per-handler testing becomes sound. Once a handler provably cannot touch
/// anything outside its footprint, exercising it against a handful of relevant states says
/// something about every state, so coverage does not need long generated sequences.
struct Footprint {
    /// The only validator entry that may be added, removed, or modified.
    account: Address,
    /// The only set-valued fields that may be modified.
    sets: &'static [Slot],
}

/// Exhaustive, so a new `StakeTableEvent` variant must declare its footprint to compile.
fn footprint(event: &StakeTableEvent) -> Footprint {
    use Slot::*;
    let keys: &'static [Slot] = &[UsedBlsKeys, UsedSchnorrKeys];
    match event {
        StakeTableEvent::Register(e) => Footprint {
            account: e.account,
            sets: keys,
        },
        StakeTableEvent::RegisterV2(e) => Footprint {
            account: e.account,
            sets: keys,
        },
        StakeTableEvent::RegisterV3(e) => Footprint {
            account: e.account,
            sets: &[UsedBlsKeys, UsedSchnorrKeys, UsedX25519Keys],
        },
        StakeTableEvent::Deregister(e) => Footprint {
            account: e.validator,
            sets: &[ValidatorExits],
        },
        StakeTableEvent::DeregisterV2(e) => Footprint {
            account: e.validator,
            sets: &[ValidatorExits],
        },
        StakeTableEvent::Delegate(e) => Footprint {
            account: e.validator,
            sets: &[],
        },
        StakeTableEvent::Undelegate(e) => Footprint {
            account: e.validator,
            sets: &[],
        },
        StakeTableEvent::UndelegateV2(e) => Footprint {
            account: e.validator,
            sets: &[],
        },
        StakeTableEvent::KeyUpdate(e) => Footprint {
            account: e.account,
            sets: keys,
        },
        StakeTableEvent::KeyUpdateV2(e) => Footprint {
            account: e.account,
            sets: keys,
        },
        StakeTableEvent::CommissionUpdate(e) => Footprint {
            account: e.validator,
            sets: &[],
        },
        StakeTableEvent::X25519KeyUpdate(e) => Footprint {
            account: e.validator,
            sets: &[UsedX25519Keys],
        },
        StakeTableEvent::P2pAddrUpdate(e) => Footprint {
            account: e.validator,
            sets: &[],
        },
    }
}

/// Names what actually moved outside `fp`, or `None` when the change stayed inside it.
fn escaped_footprint(
    before: &StakeTableState,
    after: &StakeTableState,
    fp: &Footprint,
) -> Option<String> {
    let others = |s: &StakeTableState| {
        s.validators
            .iter()
            .filter(|(a, _)| **a != fp.account)
            .map(|(a, v)| (*a, v.clone()))
            .collect::<Vec<_>>()
    };
    if others(before) != others(after) {
        return Some(format!(
            "a validator entry other than {:?} changed",
            fp.account
        ));
    }

    for (slot, changed) in [
        (
            Slot::ValidatorExits,
            before.validator_exits != after.validator_exits,
        ),
        (
            Slot::UsedBlsKeys,
            before.used_bls_keys != after.used_bls_keys,
        ),
        (
            Slot::UsedSchnorrKeys,
            before.used_schnorr_keys != after.used_schnorr_keys,
        ),
        (
            Slot::UsedX25519Keys,
            before.used_x25519_keys != after.used_x25519_keys,
        ),
    ] {
        if changed && !fp.sets.contains(&slot) {
            return Some(format!("{slot:?} changed but is not in the footprint"));
        }
    }
    None
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 96, ..ProptestConfig::default() })]

    /// Every handler stays inside its declared [`Footprint`].
    ///
    /// A widened footprint is a widened blast radius: it means a change to one event's handling
    /// can now move state another handler owns, which is where a subtle fork hides. Failing here
    /// forces the author to either narrow the change or update the declaration, and updating the
    /// declaration is visible in review.
    #[test]
    fn apply_event_stays_within_footprint(
        seed in any::<u64>(),
        corpus in seed_strategy(),
        actions in vec(action_strategy(), 1..40),
    ) {
        let pool = validator_pool(seed);
        let delegators = delegator_pool();
        let mut state = seed_state(corpus);

        for action in &actions {
            let event = to_event(action, &pool, &delegators, corpus);
            let fp = footprint(&event);
            let before = state.clone();
            // Skip rather than stop: a fatal error leaves the state untouched, which
            // `apply_event_never_mutates_on_error` proves, and `ValidatorNotFound` alone is a
            // third of generated actions, so stopping would end most cases after a few events.
            let _ = state.apply_event(event.clone());
            if let Some(escape) = escaped_footprint(&before, &state, &fp) {
                prop_assert!(false, "{:?} escaped its footprint: {}", event, escape);
            }
        }
    }
}
