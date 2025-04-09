use alloy::primitives::{Address, U256};
use alloy_compat::ethers_serde;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use derive_more::{Add, Display, From, Into, Mul, Sub};
use jf_merkle_tree::{MerkleTreeScheme, UniversalMerkleTreeScheme};
use serde::{Deserialize, Serialize};

use crate::FeeMerkleTree;

// New Type for `U256` in order to implement `CanonicalSerialize` and
// `CanonicalDeserialize`
#[derive(
    Default,
    Hash,
    Copy,
    Clone,
    Debug,
    Display,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Add,
    Sub,
    Mul,
    From,
    Into,
)]
#[display("{_0}")]
pub struct FeeAmount(pub U256);

// New Type for `Address` in order to implement `CanonicalSerialize` and
// `CanonicalDeserialize`
#[derive(
    Default,
    Hash,
    Copy,
    Clone,
    Debug,
    Display,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    From,
    Into,
)]
#[display("{_0:x}")]
#[serde(transparent)]
pub struct FeeAccount(#[serde(with = "ethers_serde::address")] pub Address);

#[derive(
    Hash,
    Copy,
    Clone,
    Debug,
    Deserialize,
    Serialize,
    PartialEq,
    Eq,
    CanonicalSerialize,
    CanonicalDeserialize,
)]
/// `FeeInfo` holds data related to builder fees.
pub struct FeeInfo {
    pub account: FeeAccount,
    pub amount: FeeAmount,
}

/// A proof of the balance of an account in the fee ledger.
///
/// If the account of interest does not exist in the fee state, this is a Merkle non-membership
/// proof, and the balance is implicitly zero. Otherwise, this is a normal Merkle membership proof.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FeeAccountProof {
    pub account: Address,
    pub proof: FeeMerkleProof,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum FeeMerkleProof {
    Presence(<FeeMerkleTree as MerkleTreeScheme>::MembershipProof),
    Absence(<FeeMerkleTree as UniversalMerkleTreeScheme>::NonMembershipProof),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountQueryData {
    pub balance: U256,
    pub proof: FeeAccountProof,
}
