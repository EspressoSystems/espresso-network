use std::{collections::HashMap, sync::Arc};

use alloy::{primitives::{Address, Log, U256}, transports::{RpcError, TransportErrorKind}};
use async_lock::{Mutex, RwLock};
use derive_more::derive::{From, Into};
use hotshot::types::{SignatureKey};
use hotshot_contract_adapter::sol_types::StakeTableV2::{
    ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated, Undelegated, ValidatorExit,
    ValidatorRegistered, ValidatorRegisteredV2,
};
use hotshot_types::{
    data::EpochNumber, light_client::StateVerKey, network::PeerConfigKeys, PeerConfig,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::JoinHandle;

use super::L1Client;
use crate::{
    traits::{MembershipPersistence, StateCatchup}, v0::ChainConfig, v0_3::RewardAmount, SeqTypes, ValidatorMap
};
use crate::StakeTableStateHash;
/// Stake table holding all staking information (DA and non-DA stakers)
#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub struct CombinedStakeTable(Vec<PeerConfigKeys<SeqTypes>>);

#[derive(Clone, Debug, From, Into, Serialize, Deserialize, PartialEq, Eq)]
/// NewType to disambiguate DA Membership
pub struct DAMembers(pub Vec<PeerConfig<SeqTypes>>);

#[derive(Clone, Debug, From, Into, Serialize, Deserialize, PartialEq, Eq)]
/// NewType to disambiguate StakeTable
pub struct StakeTable(pub Vec<PeerConfig<SeqTypes>>);

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(bound(deserialize = ""))]
pub struct Validator<KEY: SignatureKey> {
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
    (ValidatorMap, Option<RewardAmount>),
    StakeTableStateHash,
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
    Delegate(Delegated),
    Undelegate(Undelegated),
    KeyUpdate(ConsensusKeysUpdated),
    KeyUpdateV2(ConsensusKeysUpdatedV2),
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

    #[error("Missing transaction receipt for Initialized event. tx_hash={tx_hash}")]
    MissingTransactionReceipt { tx_hash: String },

    #[error("Failed to get transaction for Initialized event: {0}")]
    MissingTransaction(#[source] alloy::contract::Error),

    #[error("Failed to decode Transfer log. tx_hash={tx_hash}")]
    DecodeTransferLog { tx_hash: String},

    #[error("First transfer should be a mint from the zero address")]
    InvalidMintFromAddress,

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
}