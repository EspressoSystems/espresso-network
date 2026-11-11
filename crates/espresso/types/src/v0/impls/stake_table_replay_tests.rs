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
    stake_table::{StateSignatureSol, sign_address_bls, sign_address_schnorr},
};
use hotshot_types::{light_client::StateKeyPair, signature_key::BLSKeyPair};
use proptest::{option, prelude::*, strategy::ValueTree, test_runner::TestRunner};
use rand::{SeedableRng, rngs::StdRng};
use vbs::version::Version;
use versions::{EPOCH_VERSION, NEW_PROTOCOL_VERSION};

use crate::{
    PubKey, RegisteredValidatorMap, StakeTableState, ValidatorSet,
    v0_3::{
        COMMISSION_BASIS_POINTS, EventKey, ExpectedStakeTableError, RegisteredValidator,
        StakeTableError, StakeTableEvent,
    },
};

const POOL_SIZE: usize = 8;

/// Most signed events one sequence may hold. Each one costs a pairing to check, and the rest of
/// a sequence costs little next to it.
const MAX_SIGNED_EVENTS: usize = 6;

const ESP: u64 = 1_000_000_000_000_000_000;

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
    #![proptest_config(ProptestConfig::with_cases(64))]

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

/// The state a sequence starts from.
#[derive(Clone, Debug)]
enum Start {
    Empty,
    /// The state after the first `n` events of decaf's history.
    Decaf(usize),
}

#[derive(Clone, Debug)]
enum Op {
    Register {
        account: Validator,
        bls: BlsKey,
        schnorr: SchnorrKey,
        commission: u16,
        /// Delegated right after registering, so an active validator takes one op. A server
        /// can send the two events together, and the L1 order allows it.
        self_stake: Option<Amount>,
    },
    RegisterV2 {
        account: Validator,
        keys: SignedKeys,
        commission: u16,
    },
    RegisterV3 {
        account: Validator,
        keys: SignedKeys,
        commission: u16,
        x25519_key: X25519Key,
        p2p_addr: String,
    },
    Deregister {
        validator: Validator,
    },
    DeregisterV2 {
        validator: Validator,
    },
    Delegate {
        delegator: Delegator,
        validator: Validator,
        amount: Amount,
    },
    Undelegate {
        delegator: Delegator,
        validator: Validator,
        amount: Amount,
    },
    UndelegateV2 {
        delegator: Delegator,
        validator: Validator,
        amount: Amount,
    },
    KeyUpdate {
        validator: Validator,
        bls: BlsKey,
        schnorr: SchnorrKey,
    },
    KeyUpdateV2 {
        validator: Validator,
        keys: SignedKeys,
    },
    CommissionUpdate {
        validator: Validator,
        commission: u16,
    },
    X25519KeyUpdate {
        validator: Validator,
        key: X25519Key,
    },
    P2pAddrUpdate {
        validator: Validator,
        addr: String,
    },
}

#[derive(Clone, Debug)]
enum Validator {
    Pool(usize),
    /// A validator of the start state, by index. A sequence that starts empty uses the pool.
    Start(usize),
}

#[derive(Clone, Debug)]
enum Delegator {
    Pool(usize),
    /// One of the validator's delegators in the state so far, by index in address order.
    Existing(usize),
}

#[derive(Clone, Debug)]
enum BlsKey {
    Pool(usize),
    Invalid,
}

#[derive(Clone, Debug)]
enum SchnorrKey {
    Pool(usize),
    Invalid,
}

/// The account each signature is over. Anything other than the registering account is something
/// only a server can send, since the contract checks the BLS signature.
#[derive(Clone, Debug)]
enum Signer {
    Account,
    Other(usize),
}

#[derive(Clone, Debug)]
struct SignedKeys {
    bls: BlsKey,
    schnorr: SchnorrKey,
    bls_signer: Signer,
    schnorr_signer: Signer,
}

#[derive(Clone, Debug)]
enum X25519Key {
    Pool(usize),
    Raw([u8; 32]),
    /// Rejected by the contract, so only a server can send it.
    Zero,
    /// At or above the field modulus, also rejected by the contract.
    Unreduced,
}

#[derive(Clone, Debug)]
enum Amount {
    Exact(U256),
    /// An amount an earlier delegation or undelegation in the sequence used.
    Earlier(usize),
    /// What the delegator holds with the validator in the state so far.
    Held,
}

struct Pool {
    validators: [Address; POOL_SIZE],
    delegators: [Address; POOL_SIZE],
    bls: [BLSKeyPair; POOL_SIZE],
    schnorr: [StateKeyPair; POOL_SIZE],
}

static POOL: LazyLock<Pool> = LazyLock::new(|| Pool {
    validators: std::array::from_fn(|i| Address::repeat_byte(0x10 + i as u8)),
    delegators: std::array::from_fn(|i| Address::repeat_byte(0x80 + i as u8)),
    bls: std::array::from_fn(|i| BLSKeyPair::generate(&mut StdRng::seed_from_u64(i as u64))),
    schnorr: std::array::from_fn(|i| StateKeyPair::generate_from_seed_indexed([i as u8; 32], 0)),
});

/// The state after each prefix of decaf's stake table history, indexed by prefix length.
static DECAF_PREFIXES: LazyLock<Vec<StakeTableState>> = LazyLock::new(|| {
    let json = std::fs::read_to_string("../../../data/v3/decaf_stake_table_events.json").unwrap();
    let events: Vec<(EventKey, StakeTableEvent)> = serde_json::from_str(&json).unwrap();
    let mut state = StakeTableState::default();
    let mut prefixes = vec![state.clone()];
    for (_, event) in events {
        // Decaf's history holds rejected but expected events, such as a reused Schnorr key.
        let _ = state
            .apply_event(event)
            .expect("decaf history applies without a fatal error");
        prefixes.push(state.clone());
    }
    prefixes
});

fn sequence() -> impl Strategy<Value = (Start, Vec<Op>)> {
    (start(), prop::collection::vec(op(), 0..24))
}

fn start() -> impl Strategy<Value = Start> {
    prop_oneof![
        1 => Just(Start::Empty),
        2 => (0..DECAF_PREFIXES.len()).prop_map(Start::Decaf),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => (validator(), bls_key(), schnorr_key(), commission(), option::of(amount())).prop_map(
            |(account, bls, schnorr, commission, self_stake)| Op::Register {
                account,
                bls,
                schnorr,
                commission,
                self_stake,
            }
        ),
        2 => (validator(), signed_keys(), commission()).prop_map(|(account, keys, commission)| {
            Op::RegisterV2 {
                account,
                keys,
                commission,
            }
        }),
        2 => (validator(), signed_keys(), commission(), x25519_key(), p2p_addr()).prop_map(
            |(account, keys, commission, x25519_key, p2p_addr)| Op::RegisterV3 {
                account,
                keys,
                commission,
                x25519_key,
                p2p_addr,
            }
        ),
        1 => validator().prop_map(|validator| Op::Deregister { validator }),
        1 => validator().prop_map(|validator| Op::DeregisterV2 { validator }),
        4 => (delegator(), validator(), amount()).prop_map(|(delegator, validator, amount)| {
            Op::Delegate {
                delegator,
                validator,
                amount,
            }
        }),
        2 => (undelegator(), validator(), undelegated()).prop_map(
            |(delegator, validator, amount)| Op::Undelegate {
                delegator,
                validator,
                amount,
            }
        ),
        1 => (undelegator(), validator(), undelegated()).prop_map(
            |(delegator, validator, amount)| Op::UndelegateV2 {
                delegator,
                validator,
                amount,
            }
        ),
        1 => (validator(), bls_key(), schnorr_key()).prop_map(|(validator, bls, schnorr)| {
            Op::KeyUpdate {
                validator,
                bls,
                schnorr,
            }
        }),
        1 => (validator(), signed_keys())
            .prop_map(|(validator, keys)| Op::KeyUpdateV2 { validator, keys }),
        1 => (validator(), commission()).prop_map(|(validator, commission)| {
            Op::CommissionUpdate {
                validator,
                commission,
            }
        }),
        1 => (validator(), x25519_key())
            .prop_map(|(validator, key)| Op::X25519KeyUpdate { validator, key }),
        1 => (validator(), p2p_addr())
            .prop_map(|(validator, addr)| Op::P2pAddrUpdate { validator, addr }),
    ]
}

/// Mostly the first few pool entries, so that ops in one sequence often meet the same account or
/// key.
fn pool_index() -> impl Strategy<Value = usize> {
    prop_oneof![3 => 0..3usize, 1 => 0..POOL_SIZE]
}

fn validator() -> impl Strategy<Value = Validator> {
    prop_oneof![
        3 => pool_index().prop_map(Validator::Pool),
        1 => any::<usize>().prop_map(Validator::Start),
    ]
}

fn delegator() -> impl Strategy<Value = Delegator> {
    prop_oneof![
        3 => pool_index().prop_map(Delegator::Pool),
        1 => any::<usize>().prop_map(Delegator::Existing),
    ]
}

/// Mostly a delegator the validator already has, since an undelegation from anyone else fails.
fn undelegator() -> impl Strategy<Value = Delegator> {
    prop_oneof![
        3 => any::<usize>().prop_map(Delegator::Existing),
        1 => pool_index().prop_map(Delegator::Pool),
    ]
}

fn bls_key() -> impl Strategy<Value = BlsKey> {
    prop_oneof![8 => pool_index().prop_map(BlsKey::Pool), 1 => Just(BlsKey::Invalid)]
}

fn schnorr_key() -> impl Strategy<Value = SchnorrKey> {
    prop_oneof![8 => pool_index().prop_map(SchnorrKey::Pool), 1 => Just(SchnorrKey::Invalid)]
}

fn signer() -> impl Strategy<Value = Signer> {
    prop_oneof![6 => Just(Signer::Account), 1 => pool_index().prop_map(Signer::Other)]
}

fn signed_keys() -> impl Strategy<Value = SignedKeys> {
    (bls_key(), schnorr_key(), signer(), signer()).prop_map(
        |(bls, schnorr, bls_signer, schnorr_signer)| SignedKeys {
            bls,
            schnorr,
            bls_signer,
            schnorr_signer,
        },
    )
}

fn commission() -> impl Strategy<Value = u16> {
    prop_oneof![
        6 => 0..=COMMISSION_BASIS_POINTS,
        1 => Just(0),
        1 => Just(COMMISSION_BASIS_POINTS),
        1 => COMMISSION_BASIS_POINTS + 1..=u16::MAX,
    ]
}

fn amount() -> impl Strategy<Value = Amount> {
    let exact = |value: U256| Amount::Exact(value);
    prop_oneof![
        4 => (1..1_000u64).prop_map(move |n| exact(U256::from(n))),
        3 => (1..100_000u64).prop_map(move |n| exact(U256::from(n) * U256::from(ESP))),
        1 => any::<u64>().prop_map(move |n| exact(U256::from(n))),
        1 => any::<[u64; 4]>().prop_map(move |limbs| exact(U256::from_limbs(limbs))),
        1 => (0..1_000u64).prop_map(move |n| exact(U256::MAX - U256::from(n))),
        1 => Just(exact(U256::ZERO)),
        2 => any::<usize>().prop_map(Amount::Earlier),
    ]
}

/// Mostly what the delegator holds or an amount used before, so an undelegation can empty a
/// delegation or leave part of it.
fn undelegated() -> impl Strategy<Value = Amount> {
    prop_oneof![
        3 => Just(Amount::Held),
        2 => any::<usize>().prop_map(Amount::Earlier),
        2 => amount(),
    ]
}

fn x25519_key() -> impl Strategy<Value = X25519Key> {
    prop_oneof![
        6 => pool_index().prop_map(X25519Key::Pool),
        2 => any::<[u8; 32]>().prop_map(X25519Key::Raw),
        1 => Just(X25519Key::Zero),
        1 => Just(X25519Key::Unreduced),
    ]
}

/// Addresses `StakeTableV3.validateP2pAddr` accepts: a nonempty host, a colon, and a port from 1
/// to 65535. Only these reach an honest light client, so the round trip check relies on it.
fn p2p_addr() -> impl Strategy<Value = String> {
    let host = prop_oneof![
        3 => any::<Ipv4Addr>().prop_map(|ip| ip.to_string()),
        2 => any::<Ipv6Addr>().prop_map(|ip| format!("[{ip}]")),
        1 => any::<Ipv6Addr>().prop_map(|ip| ip.to_string()),
        3 => "[a-z0-9]([a-z0-9-]{0,8}[a-z0-9])?(\\.[a-z0-9]([a-z0-9-]{0,8}[a-z0-9])?){0,3}",
        2 => "[\\[\\]:._0-9a-zA-Z-]{1,16}",
    ];
    let port = prop_oneof![8 => 1..=u16::MAX, 1 => Just(1u16), 1 => Just(u16::MAX)];
    (host, port).prop_map(|(host, port)| format!("{host}:{port}"))
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

struct Replay {
    state: StakeTableState,
    /// Validators of the start state, in its order.
    start_validators: Vec<Address>,
    /// Amounts of the delegations and undelegations so far.
    amounts: Vec<U256>,
    signed: usize,
}

impl Replay {
    fn new(start: &Start) -> Self {
        let state = match *start {
            Start::Empty => StakeTableState::default(),
            Start::Decaf(n) => DECAF_PREFIXES[n].clone(),
        };
        Self {
            start_validators: state.validators().keys().copied().collect(),
            state,
            amounts: Vec::new(),
            signed: 0,
        }
    }

    /// The events `op` stands for in the state so far. Empty once the sequence has used up its
    /// signed events, or when an amount refers to an earlier one that does not exist.
    fn events(&mut self, op: &Op) -> Vec<StakeTableEvent> {
        match op {
            Op::Register {
                account,
                bls,
                schnorr,
                commission,
                self_stake,
            } => {
                let account = self.validator(account);
                let register = StakeTableEvent::Register(ValidatorRegistered {
                    account,
                    blsVk: bls.vk(),
                    schnorrVk: schnorr.vk(),
                    commission: *commission,
                });
                let delegate = self_stake.as_ref().and_then(|amount| {
                    let delegator = POOL.delegators[0];
                    let amount = self.amount(amount, delegator, account)?;
                    Some(StakeTableEvent::Delegate(Delegated {
                        delegator,
                        validator: account,
                        amount,
                    }))
                });
                std::iter::once(register).chain(delegate).collect()
            },
            Op::RegisterV2 {
                account,
                keys,
                commission,
            } => {
                let account = self.validator(account);
                let Some((bls_sig, schnorr_sig)) = self.sign(keys, account) else {
                    return Vec::new();
                };
                vec![StakeTableEvent::RegisterV2(ValidatorRegisteredV2 {
                    account,
                    blsVK: keys.bls.vk(),
                    schnorrVK: keys.schnorr.vk(),
                    commission: *commission,
                    blsSig: bls_sig,
                    schnorrSig: schnorr_sig,
                    metadataUri: String::new(),
                })]
            },
            Op::RegisterV3 {
                account,
                keys,
                commission,
                x25519_key,
                p2p_addr,
            } => {
                let account = self.validator(account);
                let Some((bls_sig, schnorr_sig)) = self.sign(keys, account) else {
                    return Vec::new();
                };
                vec![StakeTableEvent::RegisterV3(ValidatorRegisteredV3 {
                    account,
                    blsVK: keys.bls.vk(),
                    schnorrVK: keys.schnorr.vk(),
                    commission: *commission,
                    blsSig: bls_sig,
                    schnorrSig: schnorr_sig,
                    metadataUri: String::new(),
                    x25519Key: x25519_key.bytes(),
                    p2pAddr: p2p_addr.clone(),
                })]
            },
            Op::Deregister { validator } => vec![StakeTableEvent::Deregister(ValidatorExit {
                validator: self.validator(validator),
            })],
            Op::DeregisterV2 { validator } => {
                vec![StakeTableEvent::DeregisterV2(ValidatorExitV2 {
                    validator: self.validator(validator),
                    unlocksAt: U256::ZERO,
                })]
            },
            Op::Delegate {
                delegator,
                validator,
                amount,
            } => {
                let validator = self.validator(validator);
                let delegator = self.delegator(delegator, validator);
                let Some(amount) = self.amount(amount, delegator, validator) else {
                    return Vec::new();
                };
                vec![StakeTableEvent::Delegate(Delegated {
                    delegator,
                    validator,
                    amount,
                })]
            },
            Op::Undelegate {
                delegator,
                validator,
                amount,
            } => {
                let validator = self.validator(validator);
                let delegator = self.delegator(delegator, validator);
                let Some(amount) = self.amount(amount, delegator, validator) else {
                    return Vec::new();
                };
                vec![StakeTableEvent::Undelegate(Undelegated {
                    delegator,
                    validator,
                    amount,
                })]
            },
            Op::UndelegateV2 {
                delegator,
                validator,
                amount,
            } => {
                let validator = self.validator(validator);
                let delegator = self.delegator(delegator, validator);
                let Some(amount) = self.amount(amount, delegator, validator) else {
                    return Vec::new();
                };
                vec![StakeTableEvent::UndelegateV2(UndelegatedV2 {
                    delegator,
                    validator,
                    undelegationId: 0,
                    amount,
                    unlocksAt: U256::ZERO,
                })]
            },
            Op::KeyUpdate {
                validator,
                bls,
                schnorr,
            } => vec![StakeTableEvent::KeyUpdate(ConsensusKeysUpdated {
                account: self.validator(validator),
                blsVK: bls.vk(),
                schnorrVK: schnorr.vk(),
            })],
            Op::KeyUpdateV2 { validator, keys } => {
                let account = self.validator(validator);
                let Some((bls_sig, schnorr_sig)) = self.sign(keys, account) else {
                    return Vec::new();
                };
                vec![StakeTableEvent::KeyUpdateV2(ConsensusKeysUpdatedV2 {
                    account,
                    blsVK: keys.bls.vk(),
                    schnorrVK: keys.schnorr.vk(),
                    blsSig: bls_sig,
                    schnorrSig: schnorr_sig,
                })]
            },
            Op::CommissionUpdate {
                validator,
                commission,
            } => vec![StakeTableEvent::CommissionUpdate(CommissionUpdated {
                validator: self.validator(validator),
                timestamp: U256::ZERO,
                oldCommission: 0,
                newCommission: *commission,
            })],
            Op::X25519KeyUpdate { validator, key } => {
                vec![StakeTableEvent::X25519KeyUpdate(X25519KeyUpdated {
                    validator: self.validator(validator),
                    x25519Key: key.bytes(),
                })]
            },
            Op::P2pAddrUpdate { validator, addr } => {
                vec![StakeTableEvent::P2pAddrUpdate(P2pAddrUpdated {
                    validator: self.validator(validator),
                    p2pAddr: addr.clone(),
                })]
            },
        }
    }

    fn validator(&self, validator: &Validator) -> Address {
        match *validator {
            Validator::Start(i) if !self.start_validators.is_empty() => {
                self.start_validators[i % self.start_validators.len()]
            },
            Validator::Start(i) | Validator::Pool(i) => POOL.validators[i % POOL_SIZE],
        }
    }

    fn delegator(&self, delegator: &Delegator, validator: Address) -> Address {
        match *delegator {
            Delegator::Existing(i) => {
                let mut delegators: Vec<_> = self
                    .state
                    .validators()
                    .get(&validator)
                    .map(|v| v.delegators.keys().copied().collect())
                    .unwrap_or_default();
                delegators.sort();
                if delegators.is_empty() {
                    POOL.delegators[i % POOL_SIZE]
                } else {
                    delegators[i % delegators.len()]
                }
            },
            Delegator::Pool(i) => POOL.delegators[i % POOL_SIZE],
        }
    }

    fn amount(&mut self, amount: &Amount, delegator: Address, validator: Address) -> Option<U256> {
        let value = match *amount {
            Amount::Exact(value) => value,
            Amount::Earlier(i) => *self.amounts.get(i % self.amounts.len().max(1))?,
            Amount::Held => self
                .state
                .validators()
                .get(&validator)
                .and_then(|v| v.delegators.get(&delegator))
                .copied()
                .unwrap_or_default(),
        };
        self.amounts.push(value);
        Some(value)
    }

    /// Signatures over the signers' accounts, or `None` past [`MAX_SIGNED_EVENTS`].
    fn sign(&mut self, keys: &SignedKeys, account: Address) -> Option<(StakeTableG1Point, Bytes)> {
        self.signed += 1;
        if self.signed > MAX_SIGNED_EVENTS {
            return None;
        }
        let over = |signer: &Signer| match *signer {
            Signer::Account => account,
            Signer::Other(i) => POOL.validators[i % POOL_SIZE],
        };
        let bls_key = match keys.bls {
            BlsKey::Pool(i) => &POOL.bls[i % POOL_SIZE],
            BlsKey::Invalid => &POOL.bls[0],
        };
        let schnorr_key = match keys.schnorr {
            SchnorrKey::Pool(i) => &POOL.schnorr[i % POOL_SIZE],
            SchnorrKey::Invalid => &POOL.schnorr[0],
        };
        let bls_sig: G1PointSol = sign_address_bls(bls_key, over(&keys.bls_signer)).into();
        let schnorr_sig = StateSignatureSol::from(sign_address_schnorr(
            schnorr_key,
            over(&keys.schnorr_signer),
        ))
        .into();
        Some((bls_sig.into(), schnorr_sig))
    }
}

impl BlsKey {
    fn vk(&self) -> G2PointSol {
        match *self {
            BlsKey::Pool(i) => POOL.bls[i % POOL_SIZE].ver_key().to_affine().into(),
            BlsKey::Invalid => G2PointSol::default(),
        }
    }
}

impl SchnorrKey {
    fn vk(&self) -> EdOnBN254PointSol {
        match *self {
            SchnorrKey::Pool(i) => POOL.schnorr[i % POOL_SIZE].ver_key().to_affine().into(),
            SchnorrKey::Invalid => EdOnBN254PointSol::default(),
        }
    }
}

impl X25519Key {
    fn bytes(&self) -> FixedBytes<32> {
        FixedBytes(match *self {
            X25519Key::Pool(i) => [(i % POOL_SIZE) as u8 + 1; 32],
            // Clearing the top two bits keeps the little endian value below the modulus.
            X25519Key::Raw(mut bytes) => {
                bytes[31] &= 0x3f;
                bytes[0] |= 1;
                bytes
            },
            X25519Key::Zero => [0; 32],
            X25519Key::Unreduced => [0xff; 32],
        })
    }
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
