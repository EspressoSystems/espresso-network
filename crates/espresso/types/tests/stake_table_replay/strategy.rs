//! Generated stake table events, and the pools and history they draw on.

use super::*;

pub(super) const POOL_SIZE: usize = 8;

/// Most signed events one sequence may hold. Each one costs a pairing to check, and the rest of
/// a sequence costs little next to it.
pub(super) const MAX_SIGNED_EVENTS: usize = 6;

pub(super) const ESP: u64 = 1_000_000_000_000_000_000;

/// The state a sequence starts from.
#[derive(Clone, Debug)]
pub(super) enum Start {
    Empty,
    /// The state after the first `n` events of decaf's history.
    Decaf(usize),
}

#[derive(Clone, Debug)]
pub(super) enum Op {
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
pub(super) enum Validator {
    Pool(usize),
    /// A validator of the start state, by index. A sequence that starts empty uses the pool.
    Start(usize),
}

#[derive(Clone, Debug)]
pub(super) enum Delegator {
    Pool(usize),
    /// One of the validator's delegators in the state so far, by index in address order.
    Existing(usize),
}

#[derive(Clone, Debug)]
pub(super) enum BlsKey {
    Pool(usize),
    Invalid,
}

#[derive(Clone, Debug)]
pub(super) enum SchnorrKey {
    Pool(usize),
    Invalid,
}

/// The account each signature is over. Anything other than the registering account is something
/// only a server can send, since the contract checks the BLS signature.
#[derive(Clone, Debug)]
pub(super) enum Signer {
    Account,
    Other(usize),
}

#[derive(Clone, Debug)]
pub(super) struct SignedKeys {
    bls: BlsKey,
    schnorr: SchnorrKey,
    bls_signer: Signer,
    schnorr_signer: Signer,
}

#[derive(Clone, Debug)]
pub(super) enum X25519Key {
    Pool(usize),
    Raw([u8; 32]),
    /// Rejected by the contract, so only a server can send it.
    Zero,
    /// At or above the field modulus, also rejected by the contract.
    Unreduced,
}

#[derive(Clone, Debug)]
pub(super) enum Amount {
    Exact(U256),
    /// An amount an earlier delegation or undelegation in the sequence used.
    Earlier(usize),
    /// What the delegator holds with the validator in the state so far.
    Held,
}

pub(super) struct Pool {
    validators: [Address; POOL_SIZE],
    delegators: [Address; POOL_SIZE],
    bls: [BLSKeyPair; POOL_SIZE],
    schnorr: [StateKeyPair; POOL_SIZE],
}

pub(super) static POOL: LazyLock<Pool> = LazyLock::new(|| Pool {
    validators: std::array::from_fn(|i| Address::repeat_byte(0x10 + i as u8)),
    delegators: std::array::from_fn(|i| Address::repeat_byte(0x80 + i as u8)),
    bls: std::array::from_fn(|i| BLSKeyPair::generate(&mut StdRng::seed_from_u64(i as u64))),
    schnorr: std::array::from_fn(|i| StateKeyPair::generate_from_seed_indexed([i as u8; 32], 0)),
});

/// The state after each prefix of decaf's stake table history, indexed by prefix length.
pub(super) static DECAF_PREFIXES: LazyLock<Vec<StakeTableState>> = LazyLock::new(|| {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../data/v3/decaf_stake_table_events.json"
    ))
    .expect("the decaf history is in the repository");
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

pub(super) fn sequence() -> impl Strategy<Value = (Start, Vec<Op>)> {
    (start(), prop::collection::vec(op(), 0..24))
}

pub(super) fn start() -> impl Strategy<Value = Start> {
    prop_oneof![
        1 => Just(Start::Empty),
        2 => (0..DECAF_PREFIXES.len()).prop_map(Start::Decaf),
    ]
}

pub(super) fn op() -> impl Strategy<Value = Op> {
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
pub(super) fn pool_index() -> impl Strategy<Value = usize> {
    prop_oneof![3 => 0..3usize, 1 => 0..POOL_SIZE]
}

pub(super) fn validator() -> impl Strategy<Value = Validator> {
    prop_oneof![
        3 => pool_index().prop_map(Validator::Pool),
        1 => any::<usize>().prop_map(Validator::Start),
    ]
}

pub(super) fn delegator() -> impl Strategy<Value = Delegator> {
    prop_oneof![
        3 => pool_index().prop_map(Delegator::Pool),
        1 => any::<usize>().prop_map(Delegator::Existing),
    ]
}

/// Mostly a delegator the validator already has, since an undelegation from anyone else fails.
pub(super) fn undelegator() -> impl Strategy<Value = Delegator> {
    prop_oneof![
        3 => any::<usize>().prop_map(Delegator::Existing),
        1 => pool_index().prop_map(Delegator::Pool),
    ]
}

pub(super) fn bls_key() -> impl Strategy<Value = BlsKey> {
    prop_oneof![8 => pool_index().prop_map(BlsKey::Pool), 1 => Just(BlsKey::Invalid)]
}

pub(super) fn schnorr_key() -> impl Strategy<Value = SchnorrKey> {
    prop_oneof![8 => pool_index().prop_map(SchnorrKey::Pool), 1 => Just(SchnorrKey::Invalid)]
}

pub(super) fn signer() -> impl Strategy<Value = Signer> {
    prop_oneof![6 => Just(Signer::Account), 1 => pool_index().prop_map(Signer::Other)]
}

pub(super) fn signed_keys() -> impl Strategy<Value = SignedKeys> {
    (bls_key(), schnorr_key(), signer(), signer()).prop_map(
        |(bls, schnorr, bls_signer, schnorr_signer)| SignedKeys {
            bls,
            schnorr,
            bls_signer,
            schnorr_signer,
        },
    )
}

pub(super) fn commission() -> impl Strategy<Value = u16> {
    prop_oneof![
        6 => 0..=COMMISSION_BASIS_POINTS,
        1 => Just(0),
        1 => Just(COMMISSION_BASIS_POINTS),
        1 => COMMISSION_BASIS_POINTS + 1..=u16::MAX,
    ]
}

pub(super) fn amount() -> impl Strategy<Value = Amount> {
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
pub(super) fn undelegated() -> impl Strategy<Value = Amount> {
    prop_oneof![
        3 => Just(Amount::Held),
        2 => any::<usize>().prop_map(Amount::Earlier),
        2 => amount(),
    ]
}

pub(super) fn x25519_key() -> impl Strategy<Value = X25519Key> {
    prop_oneof![
        6 => pool_index().prop_map(X25519Key::Pool),
        2 => any::<[u8; 32]>().prop_map(X25519Key::Raw),
        1 => Just(X25519Key::Zero),
        1 => Just(X25519Key::Unreduced),
    ]
}

/// Addresses `StakeTableV3.validateP2pAddr` accepts: a nonempty host, a colon, and a port from 1
/// to 65535. Only these reach an honest light client, so the round trip check relies on it.
pub(super) fn p2p_addr() -> impl Strategy<Value = String> {
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

pub(super) struct Replay {
    pub(super) state: StakeTableState,
    /// Validators of the start state, in its order.
    start_validators: Vec<Address>,
    /// Amounts of the delegations and undelegations so far.
    amounts: Vec<U256>,
    signed: usize,
}

impl Replay {
    pub(super) fn new(start: &Start) -> Self {
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
    pub(super) fn events(&mut self, op: &Op) -> Vec<StakeTableEvent> {
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
