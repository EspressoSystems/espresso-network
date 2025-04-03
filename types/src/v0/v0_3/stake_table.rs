use std::collections::HashMap;

use crate::SeqTypes;
use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
};
use derive_more::derive::{From, Into};
use ethers::core::k256::schnorr;
use hotshot::types::{BLSPrivKey, BLSPubKey, SignatureKey};
use hotshot_contract_adapter::stake_table::NodeInfoJf;
use hotshot_types::{
    data::EpochNumber,
    light_client::{StateKeyPair, StateSignKey, StateVerKey},
    network::PeerConfigKeys,
    PeerConfig,
};
use indexmap::IndexMap;
pub(crate) use jf_signature::bls_over_bn254::KeyPair as BLSKeyPair;
use serde::{Deserialize, Serialize};

use crate::PubKey;

#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub struct PermissionedStakeTableEntry(NodeInfoJf);

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

#[derive(Clone, Debug)]
/// Validator Data useful for testing, holds Secret data need to signing.
pub struct InsecureValidator {
    pub consensus_key_pair: BLSKeyPair,
    pub state_key_pair: StateKeyPair,
    pub validator: Validator<BLSPubKey>,
    pub wallet: EthereumWallet,
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
