use std::collections::HashMap;

use crate::SeqTypes;
use alloy::primitives::{Address, U256};
use derive_more::derive::{From, Into};
use hotshot::types::{BLSPubKey, SignatureKey};
use hotshot_types::{
    data::EpochNumber, light_client::StateVerKey, network::PeerConfigKeys,
    traits::node_implementation::NodeType, PeerConfig,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(bound = "TYPES: NodeType")]
pub struct PermissionedStakeTableEntry<TYPES: NodeType>(PeerConfigKeys<TYPES>);

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
