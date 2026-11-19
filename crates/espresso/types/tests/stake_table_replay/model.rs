//! A reference model of the stake table for `apply_event` to be compared against.

use super::*;

/// The decisions `apply_event` makes, written out on their own so that a change to one of them
/// shows up as a disagreement. It shares only the key, signature and address parsers with
/// `apply_event`, and follows its order of checks, so it tracks `apply_event` rather than the
/// contract.
#[derive(Clone, Debug)]
pub(super) struct Model {
    validators: RegisteredValidatorMap,
    exits: HashSet<Address>,
    bls_keys: HashSet<PubKey>,
    schnorr_keys: HashSet<SchnorrPubKey>,
    x25519_keys: HashSet<x25519::PublicKey>,
}

/// Keys a registration carries, once its signatures have been checked or not.
struct Registration {
    account: Address,
    bls: Option<PubKey>,
    schnorr: Option<SchnorrPubKey>,
    authenticated: bool,
    commission: u16,
    x25519: Option<x25519::PublicKey>,
    p2p_addr: Option<NetAddr>,
}

impl Model {
    pub(super) fn from_state(state: &StakeTableState) -> Self {
        Self {
            validators: state.validators().clone(),
            exits: state.validator_exits().clone(),
            bls_keys: state.used_bls_keys().clone(),
            schnorr_keys: state.used_schnorr_keys().clone(),
            x25519_keys: state.used_x25519_keys().clone(),
        }
    }

    pub(super) fn matches(&self, state: &StakeTableState) -> bool {
        self.validators.iter().eq(state.validators().iter())
            && self.exits == *state.validator_exits()
            && self.bls_keys == *state.used_bls_keys()
            && self.schnorr_keys == *state.used_schnorr_keys()
            && self.x25519_keys == *state.used_x25519_keys()
    }

    /// Apply `event` if it is accepted, and return what `apply_event` should return for it.
    pub(super) fn apply(&mut self, event: &StakeTableEvent) -> ApplyResult {
        match event {
            StakeTableEvent::Register(e) => {
                let bls = PubKey::try_from(e.blsVk).ok();
                let schnorr = SchnorrPubKey::try_from(e.schnorrVk).ok();
                self.check_new(e.account, bls.as_ref(), None)?;
                // Unlike later versions, a reused Schnorr key rejects the registration.
                if let Some(k) = schnorr.as_ref().filter(|k| self.schnorr_keys.contains(*k)) {
                    return Ok(Err(ExpectedStakeTableError::SchnorrKeyAlreadyUsed(
                        k.to_string(),
                    )));
                }
                self.register(Registration {
                    account: e.account,
                    authenticated: bls.is_some() && schnorr.is_some(),
                    bls,
                    schnorr,
                    commission: e.commission,
                    x25519: None,
                    p2p_addr: None,
                });
            },
            StakeTableEvent::RegisterV2(e) => {
                let (bls, schnorr, authenticated) = match e.authenticate() {
                    Ok((bls, schnorr)) => (Some(bls), Some(schnorr), true),
                    Err(_) => (
                        PubKey::try_from(e.blsVK).ok(),
                        SchnorrPubKey::try_from(e.schnorrVK).ok(),
                        false,
                    ),
                };
                self.check_new(e.account, bls.as_ref(), None)?;
                self.register_dropping_reused_schnorr(Registration {
                    account: e.account,
                    bls,
                    schnorr,
                    authenticated,
                    commission: e.commission,
                    x25519: None,
                    p2p_addr: None,
                });
            },
            StakeTableEvent::RegisterV3(e) => {
                let (bls, schnorr, authenticated) = match e.authenticate() {
                    Ok((bls, schnorr)) => (Some(bls), Some(schnorr), true),
                    Err(_) => (
                        PubKey::try_from(e.blsVK).ok(),
                        SchnorrPubKey::try_from(e.schnorrVK).ok(),
                        false,
                    ),
                };
                let x25519 = parse_x25519_key(e.x25519Key.0);
                self.check_new(e.account, bls.as_ref(), x25519.as_ref())?;
                self.register_dropping_reused_schnorr(Registration {
                    account: e.account,
                    bls,
                    schnorr,
                    authenticated,
                    commission: e.commission,
                    x25519,
                    p2p_addr: e.p2pAddr.parse().ok(),
                });
            },
            StakeTableEvent::Deregister(ValidatorExit { validator })
            | StakeTableEvent::DeregisterV2(ValidatorExitV2 { validator, .. }) => {
                self.registered(*validator)?;
                self.validators.shift_remove(validator);
                self.exits.insert(*validator);
            },
            StakeTableEvent::Delegate(e) => {
                if e.amount.is_zero() {
                    return Err(StakeTableError::ZeroDelegatorStake(e.delegator));
                }
                let v = self.registered(e.validator)?;
                let held = v.delegators.get(&e.delegator).copied().unwrap_or_default();
                let (Some(stake), Some(held)) =
                    (v.stake.checked_add(e.amount), held.checked_add(e.amount))
                else {
                    return Err(StakeTableError::StakeOverflow(e.validator));
                };
                v.stake = stake;
                v.delegators.insert(e.delegator, held);
            },
            StakeTableEvent::Undelegate(Undelegated {
                delegator,
                validator,
                amount,
            })
            | StakeTableEvent::UndelegateV2(UndelegatedV2 {
                delegator,
                validator,
                amount,
                ..
            }) => {
                let v = self.registered(*validator)?;
                if v.stake < *amount {
                    return Err(StakeTableError::InsufficientStake);
                }
                let held = *v
                    .delegators
                    .get(delegator)
                    .ok_or(StakeTableError::DelegatorNotFound(*delegator))?;
                if held < *amount {
                    return Err(StakeTableError::InsufficientStake);
                }
                v.stake -= *amount;
                if held == *amount {
                    v.delegators.remove(delegator);
                } else {
                    v.delegators.insert(*delegator, held - *amount);
                }
            },
            StakeTableEvent::KeyUpdate(e) => {
                let Ok(bls) = PubKey::try_from(e.blsVK) else {
                    return Ok(Err(ExpectedStakeTableError::InvalidBlsKey));
                };
                let Ok(schnorr) = SchnorrPubKey::try_from(e.schnorrVK) else {
                    return Ok(Err(ExpectedStakeTableError::InvalidSchnorrKey));
                };
                return self.update_keys(e.account, bls, schnorr);
            },
            StakeTableEvent::KeyUpdateV2(e) => {
                let (bls, schnorr) = match e.authenticate() {
                    Ok(keys) => keys,
                    Err(StakeTableSolError::InvalidBlsKey) => {
                        return Ok(Err(ExpectedStakeTableError::InvalidBlsKey));
                    },
                    Err(StakeTableSolError::InvalidSchnorrKey) => {
                        return Ok(Err(ExpectedStakeTableError::InvalidSchnorrKey));
                    },
                    Err(err) => return Err(StakeTableError::AuthenticationFailed(err.to_string())),
                };
                return self.update_keys(e.account, bls, schnorr);
            },
            StakeTableEvent::CommissionUpdate(e) => {
                if e.newCommission > COMMISSION_BASIS_POINTS {
                    return Err(StakeTableError::InvalidCommission(
                        e.validator,
                        e.newCommission,
                    ));
                }
                self.registered(e.validator)?.commission = e.newCommission;
            },
            StakeTableEvent::X25519KeyUpdate(e) => {
                self.registered(e.validator)?;
                let key = parse_x25519_key(e.x25519Key.0);
                if let Some(key) = key {
                    if self.x25519_keys.contains(&key) {
                        return Err(StakeTableError::X25519KeyAlreadyUsed(key.to_string()));
                    }
                    self.x25519_keys.insert(key);
                }
                // An unparsable key clears the validator's key but stays out of the used set.
                self.registered(e.validator)?.x25519_key = key;
            },
            StakeTableEvent::P2pAddrUpdate(e) => {
                self.registered(e.validator)?.p2p_addr = e.p2pAddr.parse().ok();
            },
        }
        Ok(Ok(()))
    }

    fn registered(
        &mut self,
        account: Address,
    ) -> Result<&mut RegisteredValidator<PubKey>, StakeTableError> {
        self.validators
            .get_mut(&account)
            .ok_or(StakeTableError::ValidatorNotFound(account))
    }

    /// The checks every registration makes, in the order they are made.
    fn check_new(
        &self,
        account: Address,
        bls: Option<&PubKey>,
        x25519: Option<&x25519::PublicKey>,
    ) -> Result<(), StakeTableError> {
        if self.exits.contains(&account) {
            return Err(StakeTableError::ValidatorAlreadyExited(account));
        }
        if self.validators.contains_key(&account) {
            return Err(StakeTableError::AlreadyRegistered(account));
        }
        if let Some(k) = bls.filter(|k| self.bls_keys.contains(*k)) {
            return Err(StakeTableError::BlsKeyAlreadyUsed(k.to_string()));
        }
        if let Some(k) = x25519.filter(|k| self.x25519_keys.contains(*k)) {
            return Err(StakeTableError::X25519KeyAlreadyUsed(k.to_string()));
        }
        Ok(())
    }

    /// Since the v2 contract, a reused Schnorr key registers the validator without one instead
    /// of rejecting it, because a v1 registration could have used it first.
    fn register_dropping_reused_schnorr(&mut self, mut registration: Registration) {
        if registration
            .schnorr
            .as_ref()
            .is_some_and(|k| self.schnorr_keys.contains(k))
        {
            registration.schnorr = None;
            registration.authenticated = false;
        }
        self.register(registration);
    }

    fn register(&mut self, r: Registration) {
        self.bls_keys.extend(r.bls);
        self.schnorr_keys.extend(r.schnorr.clone());
        self.x25519_keys.extend(r.x25519);
        self.validators.insert(
            r.account,
            RegisteredValidator {
                account: r.account,
                stake_table_key: r.bls,
                state_ver_key: r.schnorr,
                stake: U256::ZERO,
                commission: r.commission,
                delegators: Default::default(),
                authenticated: r.authenticated,
                x25519_key: r.x25519,
                p2p_addr: r.p2p_addr,
            },
        );
    }

    fn update_keys(
        &mut self,
        account: Address,
        bls: PubKey,
        schnorr: SchnorrPubKey,
    ) -> ApplyResult {
        self.registered(account)?;
        if self.bls_keys.contains(&bls) {
            return Err(StakeTableError::BlsKeyAlreadyUsed(bls.to_string()));
        }
        if self.schnorr_keys.contains(&schnorr) {
            return Ok(Err(ExpectedStakeTableError::SchnorrKeyAlreadyUsed(
                schnorr.to_string(),
            )));
        }
        self.bls_keys.insert(bls);
        self.schnorr_keys.insert(schnorr.clone());
        let v = self.registered(account)?;
        v.stake_table_key = Some(bls);
        v.state_ver_key = Some(schnorr);
        v.authenticated = true;
        Ok(Ok(()))
    }
}

/// The x25519 key in `bytes`, if it is one. The all zero encoding parses but is not a key.
fn parse_x25519_key(bytes: [u8; 32]) -> Option<x25519::PublicKey> {
    (bytes != [0; 32])
        .then(|| x25519::PublicKey::try_from(bytes.as_slice()).ok())
        .flatten()
}
