use std::{collections::HashMap, sync::Arc};

use alloy::primitives::{Address, U256};
use async_lock::Mutex;
use derive_more::derive::{From, Into};
use hotshot::types::{BLSPubKey, SchnorrPubKey, SignatureKey};
use hotshot_contract_adapter::{
    sol_types::StakeTableV2::{
        ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated, Undelegated, ValidatorExit,
        ValidatorRegistered, ValidatorRegisteredV2,
    },
    stake_table::StakeTableSolError,
};
use hotshot_types::{
    data::EpochNumber, light_client::StateVerKey, network::PeerConfigKeys, PeerConfig,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
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

/// Type for holding result sets matching epochs to stake tables.
pub type IndexedStake = (
    EpochNumber,
    IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>,
);

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

type ValidatorMap = IndexMap<Address, Validator<BLSPubKey>>;

enum StakeTableEventHandlerError {
    FailedToAuthenticate(StakeTableSolError),
}

// TODO move to impl folder
// impl StakeTableEvent {
//     pub fn handle(&self) -> Result<ValidatorMap, StakeTableEventHandlerError> {
//         let mut validators = IndexMap::new();
//         match self {
//             Self::RegisterV2(event) => {
//                 event
//                     .authenticate()
//                     .map_err(StakeTableEventHandlerError::FailedToAuthenticate)?;
//                 self.register(validators);
//             },
//             _ => todo!(),
//         }
//     }
//
//     fn register(&self) -> Result<ValidatorMap, StakeTableEventHandlerError> {
//         let ValidatorRegisteredV2 {
//             account,
//             blsVK,
//             schnorrVK,
//             commission,
//             ..
//         } = self;
//
//         let stake_table_key: BLSPubKey = blsVK.into();
//         let state_ver_key: SchnorrPubKey = schnorrVK.into();
//         // TODO uncomment
//         // The stake table contract enforces that each bls key is only used once.
//         // if bls_keys.contains(&stake_table_key) {
//         //     bail!("bls key already used: {}", stake_table_key.to_string());
//         // };
//
//         // // The contract does *not* enforce that each schnorr key is only used once.
//         // if schnorr_keys.contains(&state_ver_key) {
//         //     tracing::warn!("schnorr key already used: {}", state_ver_key.to_string());
//         // };
//
//         bls_keys.insert(stake_table_key);
//         schnorr_keys.insert(state_ver_key.clone());
//     }
// }
