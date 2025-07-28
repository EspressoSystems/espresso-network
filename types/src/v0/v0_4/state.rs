use std::collections::HashSet;

use alloy::primitives::{Address, U256}; 
use derive_more::{ Display, From, Into, };
use jf_merkle_tree::{
    universal_merkle_tree::UniversalMerkleTree,
    MerkleTreeScheme, UniversalMerkleTreeScheme,
};
use serde::{Deserialize, Serialize};

use super::{FeeAccount};
use crate::{v0::sparse_mt::{Keccak256Hasher, KeccakNode}, v0_3::{RewardAccountLegacy, RewardAmount}};

pub const REWARD_MERKLE_TREE_HEIGHT: usize = 160; 
const REWARD_MERKLE_TREE_ARITY: usize = 2; 

pub type RewardMerkleCommitment = <RewardMerkleTree as MerkleTreeScheme>::Commitment;

pub type RewardMerkleTree = UniversalMerkleTree<
    RewardAmount,
    Keccak256Hasher,
    RewardAccount,
    REWARD_MERKLE_TREE_ARITY,
    KeccakNode,
>;
// New Type for `Address` in order to implement `CanonicalSerialize` and
// `CanonicalDeserialize`
// This is the same as `RewardAccountLegacy` but the `ToTraversal` trait implementation 
// for this type is different
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
pub struct RewardAccount(pub Address);

impl From<RewardAccount> for RewardAccountLegacy {
    fn from(account: RewardAccount) -> Self {
        RewardAccountLegacy(account.0)
    }
}

impl From<RewardAccountLegacy> for RewardAccount {
    fn from(account: RewardAccountLegacy) -> Self {
        RewardAccount(account.0)
    }
}

/// A proof of the balance of an account in the fee ledger.
///
/// If the account of interest does not exist in the fee state, this is a Merkle non-membership
/// proof, and the balance is implicitly zero. Otherwise, this is a normal Merkle membership proof.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RewardAccountProof {
    pub account: Address,
    pub proof: RewardMerkleProof,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum RewardMerkleProof {
    Presence(<RewardMerkleTree as MerkleTreeScheme>::MembershipProof),
    Absence(<RewardMerkleTree as UniversalMerkleTreeScheme>::NonMembershipProof),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardAccountQueryData {
    pub balance: U256,
    pub proof: RewardAccountProof,
}

 #[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Delta {
    pub fees_delta: HashSet<FeeAccount>,
    pub rewards_delta: HashSet<RewardAccount>,
}