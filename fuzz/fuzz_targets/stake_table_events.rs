//! Stake table events as a light client replays them from its server.
//!
//! The light client rebuilds each epoch's stake table by applying the events
//! its server returns, and only then compares the hash of the result with the
//! one in the epoch root header. Applying events must never panic, a rejected
//! event must leave the state untouched, and the hash must fix the quorum,
//! since the quorum is all the light client checks certificates against.
//!
//! Registrations use keys from a fixed pool, signed in advance over a fixed
//! pool of accounts, and p2p addresses and x25519 keys pass the contract's own
//! checks, so the fuzzer spends its time on event sequences rather than on
//! signatures. With that few accounts, the cutoff at `MAX_VALIDATORS` is out of
//! reach.

#![no_main]

use std::{collections::HashSet, sync::LazyLock};

use alloy::primitives::{Address, Bytes, FixedBytes, U256};
use arbitrary::Arbitrary;
use committable::Committable;
use espresso_types::{
    PubKey, RegisteredValidatorMap, StakeTableState, ValidatorSet,
    v0_3::{COMMISSION_BASIS_POINTS, RegisteredValidator, StakeTableEvent},
};
use hotshot_contract_adapter::{
    sol_types::{
        EdOnBN254PointSol, G1PointSol, G2PointSol,
        StakeTableV3::{
            CommissionUpdated, ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated,
            P2pAddrUpdated, Undelegated, ValidatorExit, ValidatorRegistered, ValidatorRegisteredV2,
            ValidatorRegisteredV3, X25519KeyUpdated,
        },
    },
    stake_table::{StateSignatureSol, sign_address_bls, sign_address_schnorr},
};
use hotshot_types::{light_client::StateKeyPair, signature_key::BLSKeyPair};
use libfuzzer_sys::fuzz_target;
use rand::{SeedableRng, rngs::StdRng};
use vbs::version::Version;
use versions::{EPOCH_VERSION, NEW_PROTOCOL_VERSION};

const POOL_SIZE: usize = 8;

/// Most signed events one input may hold. Each one is checked with a BLS
/// pairing and a Schnorr verification, which dominate the time of a run.
const MAX_SIGNED_EVENTS: usize = 6;

/// `MAX_P2P_ADDR_LENGTH` in `StakeTableV3.sol`.
const MAX_P2P_ADDR_LENGTH: usize = 512;

/// `CURVE25519_P` in `StakeTableV3.sol`, 2^255 - 19.
const CURVE25519_P: U256 = U256::from_limbs([
    0xffff_ffff_ffff_ffed,
    0xffff_ffff_ffff_ffff,
    0xffff_ffff_ffff_ffff,
    0x7fff_ffff_ffff_ffff,
]);

/// Characters that decide how a host parses: IPv6 brackets and separators, the
/// characters of IP literals, and the ones `NetAddr::validate` treats specially.
const P2P_HOST_ALPHABET: &[u8] = b"[]:.-_0123456789abcdef";

/// Account, key and delegator numbers index fixed pools and wrap around them.
#[derive(Arbitrary, Debug)]
enum Op {
    Register {
        account: u8,
        keys: Keys,
        commission: Commission,
        /// Delegated to the validator right after it registers, so two
        /// active validators, the smallest state the order check needs, take
        /// two ops instead of four.
        self_stake: Option<Amount>,
    },
    RegisterV2 {
        account: u8,
        keys: SignedKeys,
        commission: Commission,
    },
    RegisterV3 {
        account: u8,
        keys: SignedKeys,
        commission: Commission,
        x25519_key: X25519Key,
        p2p_addr: P2pAddr,
    },
    Deregister {
        validator: u8,
    },
    Delegate {
        delegator: u8,
        validator: u8,
        amount: Amount,
    },
    Undelegate {
        delegator: u8,
        validator: u8,
        amount: Amount,
    },
    KeyUpdate {
        validator: u8,
        keys: Keys,
    },
    KeyUpdateV2 {
        validator: u8,
        keys: SignedKeys,
    },
    CommissionUpdate {
        validator: u8,
        commission: Commission,
    },
    X25519KeyUpdate {
        validator: u8,
        key: X25519Key,
    },
    P2pAddrUpdate {
        validator: u8,
        addr: P2pAddr,
    },
}

/// A BLS and a Schnorr key picked independently, so a fresh key of one kind
/// can come with a used key of the other.
#[derive(Arbitrary, Debug)]
enum Keys {
    Pool { bls: u8, schnorr: u8 },
    Zero,
}

/// Each signature is over the account of its signer. A signer other than the
/// registering account is something only a server can send, since the contract
/// checks the BLS signature, and registers an unauthenticated validator.
#[derive(Arbitrary, Debug)]
struct SignedKeys {
    bls: u8,
    schnorr: u8,
    bls_signer: u8,
    schnorr_signer: u8,
}

#[derive(Arbitrary, Debug)]
enum Commission {
    Valid(u16),
    Invalid(u16),
}

#[derive(Arbitrary, Debug)]
enum X25519Key {
    Pool(u8),
    Raw([u8; 32]),
}

#[derive(Arbitrary, Debug)]
struct P2pAddr {
    host: Vec<u8>,
    port: u16,
}

#[derive(Arbitrary, Debug)]
enum Amount {
    Small(u64),
    Limbs([u64; 4]),
    Max,
    /// An amount an earlier delegation or undelegation in the same input used,
    /// so undelegations can match what was delegated.
    Earlier(u8),
}

struct Pool {
    validators: [Address; POOL_SIZE],
    delegators: [Address; POOL_SIZE],
    bls: [BlsKey; POOL_SIZE],
    schnorr: [SchnorrKey; POOL_SIZE],
}

struct BlsKey {
    vk: G2PointSol,
    /// Signatures over each pool validator account, by index.
    sigs: [G1PointSol; POOL_SIZE],
}

struct SchnorrKey {
    vk: EdOnBN254PointSol,
    /// Signatures over each pool validator account, by index.
    sigs: [Bytes; POOL_SIZE],
}

static POOL: LazyLock<Pool> = LazyLock::new(Pool::new);

fuzz_target!(init: { LazyLock::force(&POOL); }, |ops: Vec<Op>| {
    let Some(state) = replay(&POOL.events(&ops)) else {
        return;
    };
    assert_validators_round_trip(&state);

    for version in [EPOCH_VERSION, NEW_PROTOCOL_VERSION] {
        if let Ok(set) = ValidatorSet::from_state(&state, version) {
            assert_active_keys_unique(&set, version);
        }
    }
    // A state already in account order is the one the check below would build.
    // Testing for it first also gives the fuzzer a branch to steer toward.
    if !state.validators().keys().is_sorted() {
        assert_quorum_fixed_by_hash(&state);
    }
});

/// The hash covers validators in account order, so the light client cannot
/// tell `state` from the same state in that order. Both must give one quorum.
fn assert_quorum_fixed_by_hash(state: &StakeTableState) {
    let sorted = sorted_by_account(state);
    assert!(
        sorted.commit() == state.commit(),
        "the stake table hash depends on validator order"
    );
    for version in [EPOCH_VERSION, NEW_PROTOCOL_VERSION] {
        let Ok(set) = ValidatorSet::from_state(state, version) else {
            continue;
        };
        let sorted_set = ValidatorSet::from_state(&sorted, version)
            .expect("the same validators in another order give a validator set");
        assert_eq!(
            quorum(&set),
            quorum(&sorted_set),
            "stake tables with the same hash give different quorums at {version}"
        );
    }
}

/// Apply `events` as the light client does, giving up at the first fatal
/// error.
fn replay(events: &[StakeTableEvent]) -> Option<StakeTableState> {
    let mut state = StakeTableState::default();
    for event in events {
        let before = state.clone();
        match state.apply_event(event.clone()) {
            Ok(Ok(())) => assert_stake_matches_delegators(&state),
            Ok(Err(_)) => assert_unchanged(&before, &state),
            Err(_) => {
                assert_unchanged(&before, &state);
                return None;
            },
        }
    }
    Some(state)
}

fn assert_unchanged(before: &StakeTableState, after: &StakeTableState) {
    assert!(before == after, "a rejected event changed the stake table");
    assert!(
        before.validators().keys().eq(after.validators().keys()),
        "a rejected event reordered the validators"
    );
}

fn assert_stake_matches_delegators(state: &StakeTableState) {
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

/// The light client stores validators as JSON and reloads them, and nodes send
/// them as bincode. Either way the validator must keep its commitment.
fn assert_validators_round_trip(state: &StakeTableState) {
    for validator in state.validators().values() {
        // A host starting with `[` does not parse back from its unbracketed
        // form, a known NetAddr issue, so skip it here.
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

/// The quorum the light client verifies certificates against, in order.
fn quorum(set: &ValidatorSet) -> Vec<(PubKey, U256)> {
    set.active_validators()
        .values()
        .map(|v| (*v.stake_table_key(), v.stake))
        .collect()
}

fn pick(i: u8) -> usize {
    usize::from(i) % POOL_SIZE
}

impl Pool {
    fn new() -> Self {
        let validators = std::array::from_fn(|i| Address::repeat_byte(0x10 + i as u8));
        let delegators = std::array::from_fn(|i| Address::repeat_byte(0x80 + i as u8));
        Self {
            bls: std::array::from_fn(|seed| BlsKey::new(seed as u64, &validators)),
            schnorr: std::array::from_fn(|seed| SchnorrKey::new(seed as u8, &validators)),
            validators,
            delegators,
        }
    }

    /// The events `ops` describe, leaving out p2p addresses and x25519 keys the
    /// contract rejects, and signed events past [`MAX_SIGNED_EVENTS`].
    fn events(&self, ops: &[Op]) -> Vec<StakeTableEvent> {
        let mut signed = 0;
        let mut amounts = Vec::new();
        let mut events = Vec::new();
        for op in ops {
            let Some(event) = self.event(op, &mut amounts) else {
                continue;
            };
            if matches!(
                event,
                StakeTableEvent::RegisterV2(_)
                    | StakeTableEvent::RegisterV3(_)
                    | StakeTableEvent::KeyUpdateV2(_)
            ) {
                signed += 1;
                if signed > MAX_SIGNED_EVENTS {
                    continue;
                }
            }
            events.push(event);
            if let Op::Register {
                account,
                self_stake: Some(amount),
                ..
            } = op
                && let Some(amount) = amount.value(&mut amounts)
            {
                events.push(StakeTableEvent::Delegate(Delegated {
                    delegator: self.delegators[pick(*account)],
                    validator: self.validators[pick(*account)],
                    amount,
                }));
            }
        }
        events
    }

    /// The event `op` describes. `amounts` holds the amounts of the earlier
    /// delegations and undelegations, and gains this one's.
    fn event(&self, op: &Op, amounts: &mut Vec<U256>) -> Option<StakeTableEvent> {
        let event = match *op {
            Op::Register {
                account,
                ref keys,
                ref commission,
                ..
            } => {
                let (bls_vk, schnorr_vk) = self.unsigned_keys(keys);
                StakeTableEvent::Register(ValidatorRegistered {
                    account: self.validators[pick(account)],
                    blsVk: bls_vk,
                    schnorrVk: schnorr_vk,
                    commission: commission.value(),
                })
            },
            Op::RegisterV2 {
                account,
                ref keys,
                ref commission,
            } => StakeTableEvent::RegisterV2(ValidatorRegisteredV2 {
                account: self.validators[pick(account)],
                blsVK: self.bls[pick(keys.bls)].vk,
                schnorrVK: self.schnorr[pick(keys.schnorr)].vk,
                commission: commission.value(),
                blsSig: self.bls[pick(keys.bls)].sigs[pick(keys.bls_signer)].into(),
                schnorrSig: self.schnorr_sig(keys),
                metadataUri: String::new(),
            }),
            Op::RegisterV3 {
                account,
                ref keys,
                ref commission,
                ref x25519_key,
                ref p2p_addr,
            } => StakeTableEvent::RegisterV3(ValidatorRegisteredV3 {
                account: self.validators[pick(account)],
                blsVK: self.bls[pick(keys.bls)].vk,
                schnorrVK: self.schnorr[pick(keys.schnorr)].vk,
                commission: commission.value(),
                blsSig: self.bls[pick(keys.bls)].sigs[pick(keys.bls_signer)].into(),
                schnorrSig: self.schnorr_sig(keys),
                metadataUri: String::new(),
                x25519Key: x25519_key.accepted()?,
                p2pAddr: p2p_addr.accepted()?,
            }),
            Op::Deregister { validator } => StakeTableEvent::Deregister(ValidatorExit {
                validator: self.validators[pick(validator)],
            }),
            Op::Delegate {
                delegator,
                validator,
                ref amount,
            } => StakeTableEvent::Delegate(Delegated {
                delegator: self.delegators[pick(delegator)],
                validator: self.validators[pick(validator)],
                amount: amount.value(amounts)?,
            }),
            Op::Undelegate {
                delegator,
                validator,
                ref amount,
            } => StakeTableEvent::Undelegate(Undelegated {
                delegator: self.delegators[pick(delegator)],
                validator: self.validators[pick(validator)],
                amount: amount.value(amounts)?,
            }),
            Op::KeyUpdate {
                validator,
                ref keys,
            } => {
                let (bls_vk, schnorr_vk) = self.unsigned_keys(keys);
                StakeTableEvent::KeyUpdate(ConsensusKeysUpdated {
                    account: self.validators[pick(validator)],
                    blsVK: bls_vk,
                    schnorrVK: schnorr_vk,
                })
            },
            Op::KeyUpdateV2 {
                validator,
                ref keys,
            } => StakeTableEvent::KeyUpdateV2(ConsensusKeysUpdatedV2 {
                account: self.validators[pick(validator)],
                blsVK: self.bls[pick(keys.bls)].vk,
                schnorrVK: self.schnorr[pick(keys.schnorr)].vk,
                blsSig: self.bls[pick(keys.bls)].sigs[pick(keys.bls_signer)].into(),
                schnorrSig: self.schnorr_sig(keys),
            }),
            Op::CommissionUpdate {
                validator,
                ref commission,
            } => StakeTableEvent::CommissionUpdate(CommissionUpdated {
                validator: self.validators[pick(validator)],
                timestamp: U256::ZERO,
                oldCommission: 0,
                newCommission: commission.value(),
            }),
            Op::X25519KeyUpdate { validator, ref key } => {
                StakeTableEvent::X25519KeyUpdate(X25519KeyUpdated {
                    validator: self.validators[pick(validator)],
                    x25519Key: key.accepted()?,
                })
            },
            Op::P2pAddrUpdate {
                validator,
                ref addr,
            } => StakeTableEvent::P2pAddrUpdate(P2pAddrUpdated {
                validator: self.validators[pick(validator)],
                p2pAddr: addr.accepted()?,
            }),
        };
        Some(event)
    }

    fn unsigned_keys(&self, keys: &Keys) -> (G2PointSol, EdOnBN254PointSol) {
        match *keys {
            Keys::Pool { bls, schnorr } => (self.bls[pick(bls)].vk, self.schnorr[pick(schnorr)].vk),
            Keys::Zero => (G2PointSol::default(), EdOnBN254PointSol::default()),
        }
    }

    fn schnorr_sig(&self, keys: &SignedKeys) -> Bytes {
        self.schnorr[pick(keys.schnorr)].sigs[pick(keys.schnorr_signer)].clone()
    }
}

impl BlsKey {
    fn new(seed: u64, accounts: &[Address; POOL_SIZE]) -> Self {
        let key = BLSKeyPair::generate(&mut StdRng::seed_from_u64(seed));
        Self {
            vk: key.ver_key().to_affine().into(),
            sigs: accounts.map(|account| sign_address_bls(&key, account).into()),
        }
    }
}

impl SchnorrKey {
    fn new(seed: u8, accounts: &[Address; POOL_SIZE]) -> Self {
        let key = StateKeyPair::generate_from_seed_indexed([seed; 32], 0);
        Self {
            vk: key.ver_key().to_affine().into(),
            sigs: accounts
                .map(|account| StateSignatureSol::from(sign_address_schnorr(&key, account)).into()),
        }
    }
}

impl Commission {
    fn value(&self) -> u16 {
        match *self {
            Commission::Valid(commission) => commission % (COMMISSION_BASIS_POINTS + 1),
            Commission::Invalid(commission) => commission,
        }
    }
}

impl X25519Key {
    /// The key, if `StakeTableV3.validateX25519Key` accepts it.
    fn accepted(&self) -> Option<FixedBytes<32>> {
        let bytes = match *self {
            X25519Key::Pool(i) => [pick(i) as u8 + 1; 32],
            X25519Key::Raw(bytes) => bytes,
        };
        let value = U256::from_le_bytes(bytes);
        (!value.is_zero() && value < CURVE25519_P).then_some(FixedBytes(bytes))
    }
}

impl P2pAddr {
    /// The address, if `StakeTableV3.validateP2pAddr` accepts it.
    fn accepted(&self) -> Option<String> {
        let host: String = self
            .host
            .iter()
            .map(|b| char::from(P2P_HOST_ALPHABET[usize::from(*b) % P2P_HOST_ALPHABET.len()]))
            .collect();
        let addr = format!("{host}:{}", self.port);
        // The contract splits at the last colon, which is the one written here,
        // so its port checks come down to a port that is not zero.
        (!host.is_empty() && self.port != 0 && addr.len() <= MAX_P2P_ADDR_LENGTH).then_some(addr)
    }
}

impl Amount {
    /// The amount, recorded in `earlier` for later ops. `None` for an
    /// [`Amount::Earlier`] with no earlier amount to reuse.
    fn value(&self, earlier: &mut Vec<U256>) -> Option<U256> {
        let amount = match *self {
            Amount::Small(amount) => U256::from(amount),
            Amount::Limbs(limbs) => U256::from_limbs(limbs),
            Amount::Max => U256::MAX,
            Amount::Earlier(i) => *earlier.get(usize::from(i) % earlier.len().max(1))?,
        };
        earlier.push(amount);
        Some(amount)
    }
}
