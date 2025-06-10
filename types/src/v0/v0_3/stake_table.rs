use std::{collections::HashMap, sync::Arc};

use alloy::{
    primitives::{Address, U256},
    rpc::types::Log,
    sol_types::{Error as ABIError, SolEventInterface},
};
use async_lock::Mutex;
use derive_more::derive::{From, Into};
use hotshot::types::{BLSPubKey, SchnorrPubKey, SignatureKey};
use hotshot_contract_adapter::{
    sol_types::StakeTableV2::{
        ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated, StakeTableV2Events, Undelegated,
        ValidatorExit, ValidatorRegistered, ValidatorRegisteredV2,
    },
    stake_table::StakeTableSolError,
};
use hotshot_types::{
    data::EpochNumber, light_client::StateVerKey, network::PeerConfigKeys, PeerConfig,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::JoinHandle;

use super::L1Client;
use crate::{
    traits::{MembershipPersistence, StateCatchup},
    v0::ChainConfig,
    SeqTypes,
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

/// Validators mapped to `Address`s
pub type ValidatorMap = IndexMap<Address, Validator<BLSPubKey>>;

/// Type for holding result sets matching epochs to stake tables.
pub type IndexedStake = (EpochNumber, ValidatorMap);

#[derive(Debug, PartialEq, Eq, Error)]
/// Possible errors from fetching stake table from contract.
pub enum StakeTableFetchError {
    #[error("Failed to fetch stake table events.")]
    FetchError,
    #[error("No stake table contract address found in Chain config.")]
    ContractAddressNotFound,
    #[error("The epoch root for epoch {0} is missing the L1 finalized block info. This is a fatal error. Consensus is blocked and will not recover.")]
    MissingL1BlockInfo(EpochNumber),
    #[error("Failed to construct stake table")]
    StakeTableConstructionError,
    #[error("Failed to load from persistence")]
    PersistenceLoadError(String),
    #[error("Failed to events")]
    PersistenceStoreError,
    #[error("Evnt Handling Error: {0}")]
    StakeTableEventHandleError(String),
    #[error("To block greater than from_block")]
    ToBlockTooTall,
    #[error("Failed to fetch ChainConfig")]
    FailedToFetchChainConfig,
}

#[derive(Clone, derive_more::derive::Debug)]
pub struct StakeTableFetcher {
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

/// Type to represent event including metadata used for persistence and sorting.
#[derive(Debug, Clone, PartialEq)]
pub struct StakeTableEventType {
    /// Data represented as an enum variant.
    pub data: StakeTableEvent,
    /// Block number used for sorting and required by persistence.
    pub block_number: u64,
    /// Log index required by persistence.
    pub log_index: u64,
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

#[derive(thiserror::Error, Debug)]
pub enum StakeTableEventHandlerError {
    #[error("Authentication Error: {0}.")]
    FailedToAuthenticate(#[from] StakeTableSolError),
    #[error("ABI Error: {0}.")]
    ABIError(#[from] ABIError),
}

#[derive(thiserror::Error, Debug, derive_more::From)]
pub enum StakeTableStateInsertError {
    #[error("`insert` called and `Validator` already present in validator state")]
    UpdateOnInsertValidator,
    #[error("`insert` called and `Validator` already present in address mapping")]
    UpdateOnInsertAddressMapping,
    #[error("`insert` called and `Peer` already present in stake table")]
    UpdateOnInsertStakeTable,
    #[error("`insert` called and `EpochCommittee` already present in for epoch")]
    UpdateOnInsertEpochCommittee,
}

#[derive(thiserror::Error, Debug)]
pub enum StakeTableApplyEventError {
    #[error("BLS key already used: {0}")]
    DuplicateBlsKey(BLSPubKey),
    #[error("Authentication Error: {0}.")]
    FailedToAuthenticate(#[from] StakeTableSolError),
    #[error("Registration Error: {0}.")]
    RegistrationError(#[from] StakeTableStateInsertError),
}

impl TryFrom<&Log> for StakeTableEventType {
    type Error = StakeTableEventHandlerError;

    fn try_from(log: &Log) -> Result<Self, Self::Error> {
        // TODO map `None` to error type.
        let block_number = log.block_number.expect("block number");
        // TODO map `None` to error type.
        let log_index = log.log_index.expect("log index");
        let event_variant = StakeTableEvent::try_from(log)?;
        let event_type = StakeTableEventType {
            data: event_variant,
            block_number,
            log_index,
        };
        Ok(event_type)
    }
}

impl TryFrom<&Log> for StakeTableEvent {
    type Error = StakeTableEventHandlerError;

    fn try_from(log: &Log) -> Result<Self, Self::Error> {
        let event = StakeTableV2Events::decode_log(log.as_ref(), true)?;
        let event = match event.data {
            StakeTableV2Events::Delegated(event) => Self::Delegate(event),
            StakeTableV2Events::ValidatorRegisteredV2(event) => {
                event.authenticate()?;
                Self::RegisterV2(event)
            },
            _ => todo!(),
        };
        Ok(event)
    }
}

impl From<(EventKey, StakeTableEvent)> for StakeTableEventType {
    fn from(((block_number, log_index), data): (EventKey, StakeTableEvent)) -> Self {
        Self {
            block_number,
            log_index,
            data,
        }
    }
}

// TODO move to impl folder
impl StakeTableEvent {
    pub fn handle(&self) -> Result<(), StakeTableEventHandlerError> {
        // let mut validators = IndexMap::new();
        match self {
            Self::RegisterV2(event) => {
                event
                    .authenticate()
                    .map_err(StakeTableEventHandlerError::FailedToAuthenticate)?;
                // let validator = Validator::from_event(event);
                // self.register(validators);
            },
            _ => todo!(),
        }
        Ok(())
    }

    // fn register(&self) -> Result<ValidatorMap, StakeTableEventHandlerError> {
    //     let ValidatorRegisteredV2 {
    //         account,
    //         blsVK,
    //         schnorrVK,
    //         commission,
    //         ..
    //     } = self;

    //     let stake_table_key: BLSPubKey = blsVK.into();
    //     let state_ver_key: SchnorrPubKey = schnorrVK.into();
    //     // TODO uncomment
    //     // The stake table contract enforces that each bls key is only used once.
    //     // if bls_keys.contains(&stake_table_key) {
    //     //     bail!("bls key already used: {}", stake_table_key.to_string());
    //     // };

    //     // // The contract does *not* enforce that each schnorr key is only used once.
    //     // if schnorr_keys.contains(&state_ver_key) {
    //     //     tracing::warn!("schnorr key already used: {}", state_ver_key.to_string());
    //     // };

    //     bls_keys.insert(stake_table_key);
    //     schnorr_keys.insert(state_ver_key.clone());
    // }
}
