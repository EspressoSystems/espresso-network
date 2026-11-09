//! Stake table events as a light client replays them from its server.
//!
//! The light client rebuilds each epoch's stake table by applying the events
//! its server returns, and only then compares the hash of the result with the
//! one in the epoch root header. Applying events must never panic, a rejected
//! event must leave the state untouched, and two event sequences that reach the
//! same hash must give the same quorum, since the quorum is all the light
//! client checks certificates against.
//!
//! Registrations are signed by a fixed pool of keys over a fixed pool of
//! accounts, and p2p addresses and x25519 keys pass the contract's own checks,
//! so the fuzzer spends its time on event orderings rather than on signatures.

#![no_main]

use std::sync::LazyLock;

use alloy::primitives::{Address, Bytes, FixedBytes, U256};
use arbitrary::Arbitrary;
use committable::Committable;
use espresso_types::{
    PubKey, StakeTableState, ValidatorSet,
    v0_3::{RegisteredValidator, StakeTableEvent},
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

/// Most signed events one input may hold. Authenticating each one costs a
/// pairing, which dominates the time of a run.
const MAX_SIGNED_EVENTS: usize = 6;

/// `MAX_P2P_ADDR_LENGTH` in `StakeTableV3.sol`.
const MAX_P2P_ADDR_LENGTH: usize = 512;

const P2P_ADDR_ALPHABET: &[u8] = b"[]:.0123456789abcdefx";

#[derive(Arbitrary, Debug)]
struct Input {
    ops: Vec<Op>,
    /// Swaps applied in order to the events for the second replay.
    swaps: Vec<(u8, u8)>,
}

/// Account, key and delegator numbers index fixed pools and wrap around them.
/// `signer` is the account a registration's signatures were made over, so a
/// signer other than `account` registers an unauthenticated validator.
#[derive(Arbitrary, Debug)]
enum Op {
    Register {
        account: u8,
        keys: Keys,
        commission: u16,
    },
    RegisterV2 {
        account: u8,
        keys: u8,
        signer: u8,
        commission: u16,
    },
    RegisterV3 {
        account: u8,
        keys: u8,
        signer: u8,
        commission: u16,
        x25519_key: X25519Key,
        p2p_addr: Vec<u8>,
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
        keys: u8,
        signer: u8,
    },
    CommissionUpdate {
        validator: u8,
        commission: u16,
    },
    X25519KeyUpdate {
        validator: u8,
        key: X25519Key,
    },
    P2pAddrUpdate {
        validator: u8,
        addr: Vec<u8>,
    },
}

#[derive(Arbitrary, Debug)]
enum Keys {
    Pool(u8),
    Zero,
}

#[derive(Arbitrary, Debug)]
enum X25519Key {
    Pool(u8),
    Raw([u8; 32]),
}

#[derive(Arbitrary, Debug)]
enum Amount {
    Small(u64),
    Limbs([u64; 4]),
    Max,
}

struct Pool {
    validators: [Address; POOL_SIZE],
    delegators: [Address; POOL_SIZE],
    keys: Vec<SignedKeys>,
}

struct SignedKeys {
    bls_vk: G2PointSol,
    schnorr_vk: EdOnBN254PointSol,
    /// Signatures over each pool validator account, by index.
    bls_sigs: Vec<G1PointSol>,
    schnorr_sigs: Vec<Bytes>,
}

static POOL: LazyLock<Pool> = LazyLock::new(Pool::new);

fuzz_target!(init: { LazyLock::force(&POOL); }, |input: Input| {
    let events = POOL.events(&input.ops);
    let Some(state) = replay(&events) else {
        return;
    };
    assert_validators_round_trip(&state);

    let mut swapped = events;
    let len = swapped.len();
    if len > 0 {
        for &(a, b) in &input.swaps {
            swapped.swap(usize::from(a) % len, usize::from(b) % len);
        }
    }
    let Some(swapped) = replay(&swapped) else {
        return;
    };
    if swapped.commit() == state.commit() {
        for version in [EPOCH_VERSION, NEW_PROTOCOL_VERSION] {
            assert_eq!(
                quorum(&state, version),
                quorum(&swapped, version),
                "stake tables with the same hash give different quorums at {version}"
            );
        }
    }
});

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
    for version in [EPOCH_VERSION, NEW_PROTOCOL_VERSION] {
        assert_active_keys_unique(&state, version);
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

fn assert_active_keys_unique(state: &StakeTableState, version: Version) {
    let Ok(set) = ValidatorSet::from_state(state, version) else {
        return;
    };
    let active = set.active_validators().values();
    let bls_keys: Vec<_> = active.clone().map(|v| *v.stake_table_key()).collect();
    let schnorr_keys: Vec<_> = active.map(|v| v.state_ver_key().clone()).collect();
    for (i, key) in bls_keys.iter().enumerate() {
        assert!(
            !bls_keys[..i].contains(key),
            "two active validators share a BLS key at {version}"
        );
    }
    for (i, key) in schnorr_keys.iter().enumerate() {
        assert!(
            !schnorr_keys[..i].contains(key),
            "two active validators share a Schnorr key at {version}"
        );
    }
}

/// The light client stores validators as JSON and reloads them, and nodes send
/// them as bincode. Either way the validator must keep its commitment.
fn assert_validators_round_trip(state: &StakeTableState) {
    for validator in state.validators().values() {
        // A host starting with `[` does not parse back from its unbracketed
        // form. Tracked separately, so skip it here.
        if validator
            .p2p_addr
            .as_ref()
            .is_some_and(|addr| addr.unbracketed_string().contains('['))
        {
            continue;
        }
        let json: RegisteredValidator<PubKey> =
            serde_json::from_value(serde_json::to_value(validator).unwrap())
                .expect("a validator decodes from its JSON");
        assert_eq!(
            json.commit(),
            validator.commit(),
            "JSON changes {validator:?}"
        );
        let binary: RegisteredValidator<PubKey> =
            bincode::deserialize(&bincode::serialize(validator).unwrap())
                .expect("a validator decodes from its bincode");
        assert_eq!(
            binary.commit(),
            validator.commit(),
            "bincode changes {validator:?}"
        );
    }
}

/// The quorum the light client verifies certificates against, in order.
fn quorum(state: &StakeTableState, version: Version) -> Option<Vec<(PubKey, U256)>> {
    let set = ValidatorSet::from_state(state, version).ok()?;
    Some(
        set.active_validators()
            .values()
            .map(|v| (*v.stake_table_key(), v.stake))
            .collect(),
    )
}

impl Pool {
    fn new() -> Self {
        let validators = std::array::from_fn(|i| Address::repeat_byte(0x10 + i as u8));
        let delegators = std::array::from_fn(|i| Address::repeat_byte(0x80 + i as u8));
        let keys = (0..POOL_SIZE as u64)
            .map(|seed| SignedKeys::new(seed, &validators))
            .collect();
        Self {
            validators,
            delegators,
            keys,
        }
    }

    /// The events `ops` describe, leaving out the ones the contract would
    /// revert on and signed ones past [`MAX_SIGNED_EVENTS`].
    fn events(&self, ops: &[Op]) -> Vec<StakeTableEvent> {
        let mut signed = 0;
        ops.iter()
            .filter_map(|op| {
                let event = self.event(op)?;
                if matches!(
                    event,
                    StakeTableEvent::RegisterV2(_)
                        | StakeTableEvent::RegisterV3(_)
                        | StakeTableEvent::KeyUpdateV2(_)
                ) {
                    signed += 1;
                    if signed > MAX_SIGNED_EVENTS {
                        return None;
                    }
                }
                Some(event)
            })
            .collect()
    }

    fn event(&self, op: &Op) -> Option<StakeTableEvent> {
        let event = match *op {
            Op::Register {
                account,
                ref keys,
                commission,
            } => {
                let (bls_vk, schnorr_vk) = self.unsigned_keys(keys);
                StakeTableEvent::Register(ValidatorRegistered {
                    account: self.validator(account),
                    blsVk: bls_vk,
                    schnorrVk: schnorr_vk,
                    commission,
                })
            },
            Op::RegisterV2 {
                account,
                keys,
                signer,
                commission,
            } => {
                let keys = self.signed_keys(keys);
                StakeTableEvent::RegisterV2(ValidatorRegisteredV2 {
                    account: self.validator(account),
                    blsVK: keys.bls_vk,
                    schnorrVK: keys.schnorr_vk,
                    commission,
                    blsSig: keys.bls_sig(signer).into(),
                    schnorrSig: keys.schnorr_sig(signer),
                    metadataUri: String::new(),
                })
            },
            Op::RegisterV3 {
                account,
                keys,
                signer,
                commission,
                ref x25519_key,
                ref p2p_addr,
            } => {
                let keys = self.signed_keys(keys);
                StakeTableEvent::RegisterV3(ValidatorRegisteredV3 {
                    account: self.validator(account),
                    blsVK: keys.bls_vk,
                    schnorrVK: keys.schnorr_vk,
                    commission,
                    blsSig: keys.bls_sig(signer).into(),
                    schnorrSig: keys.schnorr_sig(signer),
                    metadataUri: String::new(),
                    x25519Key: x25519_key.accepted()?,
                    p2pAddr: accepted_p2p_addr(p2p_addr)?,
                })
            },
            Op::Deregister { validator } => StakeTableEvent::Deregister(ValidatorExit {
                validator: self.validator(validator),
            }),
            Op::Delegate {
                delegator,
                validator,
                ref amount,
            } => StakeTableEvent::Delegate(Delegated {
                delegator: self.delegator(delegator),
                validator: self.validator(validator),
                amount: amount.value(),
            }),
            Op::Undelegate {
                delegator,
                validator,
                ref amount,
            } => StakeTableEvent::Undelegate(Undelegated {
                delegator: self.delegator(delegator),
                validator: self.validator(validator),
                amount: amount.value(),
            }),
            Op::KeyUpdate {
                validator,
                ref keys,
            } => {
                let (bls_vk, schnorr_vk) = self.unsigned_keys(keys);
                StakeTableEvent::KeyUpdate(ConsensusKeysUpdated {
                    account: self.validator(validator),
                    blsVK: bls_vk,
                    schnorrVK: schnorr_vk,
                })
            },
            Op::KeyUpdateV2 {
                validator,
                keys,
                signer,
            } => {
                let keys = self.signed_keys(keys);
                StakeTableEvent::KeyUpdateV2(ConsensusKeysUpdatedV2 {
                    account: self.validator(validator),
                    blsVK: keys.bls_vk,
                    schnorrVK: keys.schnorr_vk,
                    blsSig: keys.bls_sig(signer).into(),
                    schnorrSig: keys.schnorr_sig(signer),
                })
            },
            Op::CommissionUpdate {
                validator,
                commission,
            } => StakeTableEvent::CommissionUpdate(CommissionUpdated {
                validator: self.validator(validator),
                timestamp: U256::ZERO,
                oldCommission: 0,
                newCommission: commission,
            }),
            Op::X25519KeyUpdate { validator, ref key } => {
                StakeTableEvent::X25519KeyUpdate(X25519KeyUpdated {
                    validator: self.validator(validator),
                    x25519Key: key.accepted()?,
                })
            },
            Op::P2pAddrUpdate {
                validator,
                ref addr,
            } => StakeTableEvent::P2pAddrUpdate(P2pAddrUpdated {
                validator: self.validator(validator),
                p2pAddr: accepted_p2p_addr(addr)?,
            }),
        };
        Some(event)
    }

    fn validator(&self, i: u8) -> Address {
        self.validators[usize::from(i) % POOL_SIZE]
    }

    fn delegator(&self, i: u8) -> Address {
        self.delegators[usize::from(i) % POOL_SIZE]
    }

    fn signed_keys(&self, i: u8) -> &SignedKeys {
        &self.keys[usize::from(i) % POOL_SIZE]
    }

    fn unsigned_keys(&self, keys: &Keys) -> (G2PointSol, EdOnBN254PointSol) {
        match keys {
            Keys::Pool(i) => {
                let keys = self.signed_keys(*i);
                (keys.bls_vk, keys.schnorr_vk)
            },
            Keys::Zero => (G2PointSol::default(), EdOnBN254PointSol::default()),
        }
    }
}

impl SignedKeys {
    fn new(seed: u64, accounts: &[Address]) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let bls = BLSKeyPair::generate(&mut rng);
        let schnorr = StateKeyPair::generate_from_seed_indexed([seed as u8; 32], 0);
        Self {
            bls_vk: bls.ver_key().to_affine().into(),
            schnorr_vk: schnorr.ver_key().to_affine().into(),
            bls_sigs: accounts
                .iter()
                .map(|account| sign_address_bls(&bls, *account).into())
                .collect(),
            schnorr_sigs: accounts
                .iter()
                .map(|account| {
                    StateSignatureSol::from(sign_address_schnorr(&schnorr, *account)).into()
                })
                .collect(),
        }
    }

    fn bls_sig(&self, signer: u8) -> G1PointSol {
        self.bls_sigs[usize::from(signer) % self.bls_sigs.len()]
    }

    fn schnorr_sig(&self, signer: u8) -> Bytes {
        self.schnorr_sigs[usize::from(signer) % self.schnorr_sigs.len()].clone()
    }
}

impl X25519Key {
    /// The key, if `StakeTableV3.validateX25519Key` accepts it.
    fn accepted(&self) -> Option<FixedBytes<32>> {
        let bytes = match *self {
            X25519Key::Pool(i) => [i % POOL_SIZE as u8 + 1; 32],
            X25519Key::Raw(bytes) => bytes,
        };
        // A nonzero little endian value below 2^255 - 19.
        let value = U256::from_le_bytes(bytes);
        let p = (U256::from(1) << 255) - U256::from(19);
        (!value.is_zero() && value < p).then_some(FixedBytes(bytes))
    }
}

impl Amount {
    fn value(&self) -> U256 {
        match *self {
            Amount::Small(amount) => U256::from(amount),
            Amount::Limbs(limbs) => U256::from_limbs(limbs),
            Amount::Max => U256::MAX,
        }
    }
}

/// `bytes` mapped onto an address alphabet, if `StakeTableV3.validateP2pAddr`
/// accepts the result.
fn accepted_p2p_addr(bytes: &[u8]) -> Option<String> {
    let addr: String = bytes
        .iter()
        .map(|b| char::from(P2P_ADDR_ALPHABET[usize::from(*b) % P2P_ADDR_ALPHABET.len()]))
        .collect();
    if addr.is_empty() || addr.len() > MAX_P2P_ADDR_LENGTH {
        return None;
    }
    let (host, port) = addr.rsplit_once(':')?;
    let digits = port.bytes().all(|b| b.is_ascii_digit());
    let leading_zero = port.len() > 1 && port.starts_with('0');
    let in_range = port.parse::<u32>().is_ok_and(|p| (1..=65535).contains(&p));
    (!host.is_empty() && (1..=5).contains(&port.len()) && digits && !leading_zero && in_range)
        .then_some(addr)
}
