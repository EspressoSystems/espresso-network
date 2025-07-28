
use alloy::primitives::{Address, U256};
use derive_more::{derive::AddAssign, Add, Display, From, Into, Mul, Sub};
use jf_merkle_tree::{
    prelude::{ Sha3Digest, Sha3Node},
    universal_merkle_tree::UniversalMerkleTree,
    MerkleTreeScheme, UniversalMerkleTreeScheme,
};
use serde::{Deserialize, Serialize};

pub const LEGACY_REWARD_MERKLE_TREE_HEIGHT: usize = 20; 
const REWARD_MERKLE_TREE_ARITY_LEGACY: usize = 256;

pub type RewardMerkleTreeLegacy = UniversalMerkleTree<
    RewardAmount,
    Sha3Digest,
    RewardAccountLegacy,
    REWARD_MERKLE_TREE_ARITY_LEGACY,
    Sha3Node,
>;
 
pub type RewardMerkleCommitmentLegacy = <RewardMerkleTreeLegacy as MerkleTreeScheme>::Commitment;

// New Type for `Address` in order to implement `CanonicalSerialize` and
// `CanonicalDeserialize`
// This is the same as `RewardAccount`` but the `ToTraversal` trait implementation 
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
pub struct RewardAccountLegacy(pub Address);


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
    AddAssign,
)]
#[display("{_0}")]
pub struct RewardAmount(pub U256);
 
pub(crate) const INFLATION_RATE: u128 = 300; // 3% in basis points
pub(crate) const ASSUMED_BLOCK_TIME_SECONDS: u128 = 2;
pub(crate) const SECONDS_PER_YEAR: u128 = 60 * 60 * 24 * 365;
pub(crate) const MILLISECONDS_PER_YEAR: u128 = 86_400_000 * 365;
pub(crate) const BLOCKS_PER_YEAR: u128 = SECONDS_PER_YEAR / ASSUMED_BLOCK_TIME_SECONDS;
pub const COMMISSION_BASIS_POINTS: u16 = 10_000;
  
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardAccountQueryDataLegacy {
    pub balance: U256,
    pub proof: RewardAccountProofLegacy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RewardAccountProofLegacy {
    pub account: Address,
    pub proof: RewardMerkleProofLegacy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum RewardMerkleProofLegacy {
    Presence(<RewardMerkleTreeLegacy as MerkleTreeScheme>::MembershipProof),
    Absence(<RewardMerkleTreeLegacy as UniversalMerkleTreeScheme>::NonMembershipProof),
}



