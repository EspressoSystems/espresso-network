use std::{collections::HashMap, sync::Arc};

use alloy::{
    primitives::{Address, Log, U256},
    transports::{RpcError, TransportErrorKind},
};
use async_lock::{Mutex, RwLock};
use committable::{Commitment, Committable, RawCommitmentBuilder};
use derive_more::derive::{From, Into};
use hotshot::types::SignatureKey;
use hotshot_contract_adapter::sol_types::StakeTableV3::{
    CommissionUpdated, ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated, P2pAddrUpdated,
    Undelegated, UndelegatedV2, ValidatorExit, ValidatorExitV2, ValidatorRegistered,
    ValidatorRegisteredV2, ValidatorRegisteredV3, X25519KeyUpdated,
};
use hotshot_types::{
    PeerConfig, addr::NetAddr, data::EpochNumber, light_client::StateVerKey,
    network::PeerConfigKeys, x25519,
};
use itertools::Itertools;
use jf_utils::to_bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::JoinHandle;
use vbs::version::Version;
use versions::CLIQUENET_VERSION;

use super::L1Client;
use crate::{
    AuthenticatedValidatorMap, SeqTypes, traits::{MembershipPersistence, StateCatchup},
    v0::{ChainConfig, impls::StakeTableHash},
    v0_3::RewardAmount,
};
/// Stake table holding all staking information (DA and non-DA stakers)
#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub struct CombinedStakeTable(Vec<PeerConfigKeys<SeqTypes>>);

#[derive(Clone, Debug, From, Into, Serialize, Deserialize, PartialEq, Eq)]
/// NewType to disambiguate DA Membership
pub struct DAMembers(pub Vec<PeerConfig<SeqTypes>>);

#[derive(Clone, Debug, From, Into, Serialize, Deserialize, PartialEq, Eq)]
/// NewType to disambiguate StakeTable
pub struct StakeTable(pub Vec<PeerConfig<SeqTypes>>);

pub(crate) fn to_fixed_bytes(value: U256) -> [u8; std::mem::size_of::<U256>()] {
    let bytes: [u8; std::mem::size_of::<U256>()] = value.to_le_bytes();
    bytes
}

/// Validator as registered in the stake table contract.
/// May or may not have valid signatures (contract can't fully verify Schnorr).
/// Used for state tracking. To participate in consensus, must be authenticated
/// and converted to `AuthenticatedValidator` via `TryFrom`.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(bound(deserialize = ""))]
pub struct RegisteredValidator<KEY: SignatureKey> {
    pub account: Address,
    /// The peer's public key
    pub stake_table_key: KEY,
    /// the peer's state public key
    pub state_ver_key: StateVerKey,
    /// the peer's stake
    pub stake: U256,
    // commission
    // TODO: MA commission is only valid from 0 to 10_000. Add newtype to enforce this.
    pub commission: u16,
    pub delegators: HashMap<Address, U256>,
    /// Whether the validator's registration signature has been verified.
    /// Contract can verify BLS but only length-check Schnorr.
    pub authenticated: bool,
    /// Public X25519 key for network communication.
    pub x25519_key: Option<x25519::PublicKey>,
    /// Network address.
    pub p2p_addr: Option<NetAddr>,
}

/// Validator eligible for consensus participation.
/// Guaranteed to have valid BLS and Schnorr signatures.
/// This is a newtype wrapper around RegisteredValidator that guarantees authenticated=true.
#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedValidator<KEY: SignatureKey>(RegisteredValidator<KEY>);

impl<'de, KEY: SignatureKey> Deserialize<'de> for AuthenticatedValidator<KEY> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = RegisteredValidator::deserialize(deserializer)?;
        if !inner.authenticated {
            return Err(serde::de::Error::custom(
                "cannot deserialize unauthenticated validator as AuthenticatedValidator",
            ));
        }
        Ok(AuthenticatedValidator(inner))
    }
}

impl<KEY: SignatureKey> AuthenticatedValidator<KEY> {
    pub fn into_inner(self) -> RegisteredValidator<KEY> {
        self.0
    }

    /// Whether this validator can participate in consensus at `protocol_version`.
    ///
    /// Encodes only protocol-version-gated requirements; stake and delegation checks
    /// live in `select_active_validator_set`.
    pub fn is_eligible(&self, protocol_version: Version) -> bool {
        if protocol_version >= CLIQUENET_VERSION
            && (self.x25519_key.is_none() || self.p2p_addr.is_none())
        {
            return false;
        }
        true
    }
}

impl<KEY: SignatureKey> std::ops::Deref for AuthenticatedValidator<KEY> {
    type Target = RegisteredValidator<KEY>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Error)]
#[error("Validator {0:#x} not authenticated (invalid registration signature)")]
pub struct UnauthenticatedValidatorError(pub Address);

impl<KEY: SignatureKey + Clone> TryFrom<&RegisteredValidator<KEY>> for AuthenticatedValidator<KEY> {
    type Error = UnauthenticatedValidatorError;

    fn try_from(v: &RegisteredValidator<KEY>) -> Result<Self, Self::Error> {
        if !v.authenticated {
            return Err(UnauthenticatedValidatorError(v.account));
        }
        Ok(AuthenticatedValidator(v.clone()))
    }
}

impl<KEY: SignatureKey> TryFrom<RegisteredValidator<KEY>> for AuthenticatedValidator<KEY> {
    type Error = UnauthenticatedValidatorError;

    fn try_from(v: RegisteredValidator<KEY>) -> Result<Self, Self::Error> {
        if !v.authenticated {
            return Err(UnauthenticatedValidatorError(v.account));
        }
        Ok(AuthenticatedValidator(v))
    }
}

impl<KEY: SignatureKey> From<AuthenticatedValidator<KEY>> for RegisteredValidator<KEY> {
    fn from(v: AuthenticatedValidator<KEY>) -> Self {
        v.into_inner()
    }
}

impl<KEY: SignatureKey> Committable for RegisteredValidator<KEY> {
    fn commit(&self) -> Commitment<Self> {
        let mut builder = RawCommitmentBuilder::new(&Self::tag())
            .fixed_size_field("account", &self.account)
            .var_size_field(
                "stake_table_key",
                self.stake_table_key.to_bytes().as_slice(),
            )
            .var_size_field("state_ver_key", &to_bytes!(&self.state_ver_key).unwrap())
            .fixed_size_field("stake", &to_fixed_bytes(self.stake))
            .constant_str("commission")
            .u16(self.commission);

        // x25519_key and p2p_addr are included in the commitment only when set.
        // They are None until StakeTableV3 is deployed and the validator sets them.
        // This maintains backwards compatibility with pre-V3 commitments.
        if let Some(key) = &self.x25519_key {
            builder = builder.var_size_field("x25519_key", key.as_slice());
        }
        if let Some(addr) = &self.p2p_addr {
            builder = builder.var_size_field("p2p_addr", addr.to_string().as_bytes());
        }

        builder = builder.constant_str("delegators");
        for (address, stake) in self.delegators.iter().sorted() {
            builder = builder
                .fixed_size_bytes(address)
                .fixed_size_bytes(&to_fixed_bytes(*stake));
        }

        // Backwards compatibility: don't change the commitment of *authenticated* validators
        if !self.authenticated {
            builder = builder.constant_str("unauthenticated");
        }

        builder.finalize()
    }

    fn tag() -> String {
        "VALIDATOR".to_string()
    }
}

#[derive(serde::Serialize, serde::Deserialize, std::hash::Hash, Clone, Debug, PartialEq, Eq)]
#[serde(bound(deserialize = ""))]
pub struct Delegator {
    pub address: Address,
    pub validator: Address,
    pub stake: U256,
}

/// Type for holding result sets matching epochs to stake tables.
pub type IndexedStake = (
    EpochNumber,
    (AuthenticatedValidatorMap, Option<RewardAmount>),
    Option<StakeTableHash>,
);

#[derive(Clone, derive_more::derive::Debug)]
pub struct Fetcher {
    /// Peers for catching up the stake table
    #[debug(skip)]
    pub(crate) peers: Arc<dyn StateCatchup>,
    /// Methods for stake table persistence.
    #[debug(skip)]
    pub(crate) persistence: Arc<Mutex<dyn MembershipPersistence>>,
    /// L1 provider
    pub(crate) l1_client: L1Client,
    /// Verifiable `ChainConfig` holding contract address
    pub(crate) chain_config: Arc<Mutex<ChainConfig>>,
    pub(crate) update_task: Arc<StakeTableUpdateTask>,
    pub initial_supply: Arc<RwLock<Option<U256>>>,
}

#[derive(Debug, Default)]
pub(crate) struct StakeTableUpdateTask(pub(crate) Mutex<Option<JoinHandle<()>>>);

impl Drop for StakeTableUpdateTask {
    fn drop(&mut self) {
        if let Some(task) = self.0.get_mut().take() {
            task.abort();
        }
    }
}

// (log block number, log index)
pub type EventKey = (u64, u64);

#[derive(Clone, derive_more::From, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StakeTableEvent {
    Register(ValidatorRegistered),
    RegisterV2(ValidatorRegisteredV2),
    Deregister(ValidatorExit),
    DeregisterV2(ValidatorExitV2),
    Delegate(Delegated),
    Undelegate(Undelegated),
    UndelegateV2(UndelegatedV2),
    KeyUpdate(ConsensusKeysUpdated),
    KeyUpdateV2(ConsensusKeysUpdatedV2),
    CommissionUpdate(CommissionUpdated),
    RegisterV3(ValidatorRegisteredV3),
    X25519KeyUpdate(X25519KeyUpdated),
    P2pAddrUpdate(P2pAddrUpdated),
}

#[derive(Debug, Error)]
pub enum StakeTableError {
    #[error("Validator {0:#x} already registered")]
    AlreadyRegistered(Address),
    #[error("Validator {0:#x} not found")]
    ValidatorNotFound(Address),
    #[error("Delegator {0:#x} not found")]
    DelegatorNotFound(Address),
    #[error("BLS key already used: {0}")]
    BlsKeyAlreadyUsed(String),
    #[error("Insufficient stake to undelegate")]
    InsufficientStake,
    #[error("Event authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("No validators met the minimum criteria (non-zero stake and at least one delegator)")]
    NoValidValidators,
    #[error("Could not compute maximum stake from filtered validators")]
    MissingMaximumStake,
    #[error("Overflow when calculating minimum stake threshold")]
    MinimumStakeOverflow,
    #[error("Delegator {0:#x} has 0 stake")]
    ZeroDelegatorStake(Address),
    #[error("Failed to hash stake table: {0}")]
    HashError(#[from] bincode::Error),
    #[error("Validator {0:#x} already exited and cannot be re-registered")]
    ValidatorAlreadyExited(Address),
    #[error("Validator {0:#x} has invalid commission {1}")]
    InvalidCommission(Address, u16),
    #[error("Schnorr key already used: {0}")]
    SchnorrKeyAlreadyUsed(String),
    #[error("x25519 key already used: {0}")]
    X25519KeyAlreadyUsed(String),
    #[error("Invalid x25519 key: {0}")]
    InvalidX25519Key(String),
    #[error("Stake table event decode error {0}")]
    StakeTableEventDecodeError(#[from] alloy::sol_types::Error),
    #[error("Stake table events sorting error: {0}")]
    EventSortingError(#[from] EventSortingError),
}

#[derive(Debug, Error)]
pub enum ExpectedStakeTableError {
    #[error("Schnorr key already used: {0}")]
    SchnorrKeyAlreadyUsed(String),
}

#[derive(Debug, Error)]
pub enum FetchRewardError {
    #[error("No stake table contract address found in chain config")]
    MissingStakeTableContract,

    #[error("Token address fetch failed: {0}")]
    TokenAddressFetch(#[source] alloy::contract::Error),

    #[error("Token Initialized event logs are empty")]
    MissingInitializedEvent,

    #[error("Transaction hash not found in Initialized event log: {init_log:?}")]
    MissingTransactionHash { init_log: Log },

    #[error("Block number not found in Initialized event log")]
    MissingBlockNumber,

    #[error("Transfer event query failed: {0}")]
    TransferEventQuery(#[source] alloy::contract::Error),

    #[error("No Transfer event found in the Initialized event block")]
    MissingTransferEvent,

    #[error("Division by zero {0}")]
    DivisionByZero(&'static str),

    #[error("Overflow {0}")]
    Overflow(&'static str),

    #[error("Contract call failed: {0}")]
    ContractCall(#[source] alloy::contract::Error),

    #[error("Rpc call failed: {0}")]
    Rpc(#[source] RpcError<TransportErrorKind>),

    #[error("Exceeded max block range scan ({0} blocks) while searching for Initialized event")]
    ExceededMaxScanRange(u64),

    #[error("Scanning for Initialized event failed: {0}")]
    ScanQueryFailed(#[source] alloy::contract::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum EventSortingError {
    #[error("Missing block number in log")]
    MissingBlockNumber,

    #[error("Missing log index in log")]
    MissingLogIndex,

    #[error("Invalid stake table event")]
    InvalidStakeTableEvent,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use alloy::primitives::{Address, U256};
    use committable::Committable;
    use hotshot::types::{BLSPubKey, SignatureKey};
    use hotshot_types::{addr::NetAddr, light_client::StateVerKey, x25519};

    use super::RegisteredValidator;

    /// Both x25519_key and p2p_addr must independently affect the commitment.
    #[test]
    fn test_commitment_changes_with_x25519_and_p2p_fields() {
        let base = RegisteredValidator::<BLSPubKey>::mock();
        assert!(base.x25519_key.is_none());
        assert!(base.p2p_addr.is_none());
        let commit_base = base.commit();

        let mut with_x25519 = base.clone();
        with_x25519.x25519_key =
            Some(x25519::PublicKey::try_from([42u8; 32].as_slice()).unwrap());
        let commit_x25519 = with_x25519.commit();

        let mut with_p2p = base.clone();
        with_p2p.p2p_addr = Some("127.0.0.1:8080".parse::<NetAddr>().unwrap());
        let commit_p2p = with_p2p.commit();

        assert_ne!(commit_base, commit_x25519);
        assert_ne!(commit_base, commit_p2p);
        assert_ne!(commit_x25519, commit_p2p);
    }

    /// Unauthenticated validators must produce a different commitment than authenticated ones.
    /// This ensures validators with invalid signatures are distinguishable in the commitment tree.
    #[test]
    fn test_unauthenticated_validator_commitment_differs() {
        let account = Address::random();
        let stake_table_key = BLSPubKey::generated_from_seed_indexed([1u8; 32], 0).0;
        let state_ver_key = StateVerKey::default();
        let stake = U256::from(1000);
        let commission = 500u16;
        let delegators = HashMap::new();

        let authenticated = RegisteredValidator {
            account,
            stake_table_key,
            state_ver_key: state_ver_key.clone(),
            stake,
            commission,
            delegators: delegators.clone(),
            authenticated: true,
            x25519_key: None,
            p2p_addr: None,
        };

        let unauthenticated = RegisteredValidator {
            account,
            stake_table_key,
            state_ver_key,
            stake,
            commission,
            delegators,
            authenticated: false,
            x25519_key: None,
            p2p_addr: None,
        };

        let auth_commitment = authenticated.commit();
        let unauth_commitment = unauthenticated.commit();
        assert_ne!(
            auth_commitment.as_ref() as &[u8],
            unauth_commitment.as_ref() as &[u8]
        );
    }
}
