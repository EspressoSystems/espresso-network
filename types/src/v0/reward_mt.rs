use std::borrow::Borrow;

use jf_merkle_tree_compat::{
    internal::{MerkleTreeIntoIter, MerkleTreeIter},
    universal_merkle_tree::UniversalMerkleTree,
    ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme, LookupResult,
    MerkleTreeCommitment, MerkleTreeError, MerkleTreeScheme, PersistentUniversalMerkleTreeScheme,
    UniversalMerkleTreeScheme,
};
use serde::{Deserialize, Serialize};

use crate::v0::{
    sparse_mt::{Keccak256Hasher, KeccakNode},
    v0_3::RewardAmount,
    v0_4::RewardAccountV2,
};

pub const REWARD_MERKLE_TREE_V2_HEIGHT: usize = 160;
pub const REWARD_MERKLE_TREE_V2_ARITY: usize = 2;

type InnerRewardMerkleTreeV2 = UniversalMerkleTree<
    RewardAmount,
    Keccak256Hasher,
    RewardAccountV2,
    REWARD_MERKLE_TREE_V2_ARITY,
    KeccakNode,
>;

/// Reward merkle tree V2 with keccak256 hashing
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RewardMerkleTreeV2 {
    inner: InnerRewardMerkleTreeV2,
}

pub type RewardMerkleCommitmentV2 = MerkleTreeCommitment<KeccakNode>;

impl RewardMerkleTreeV2 {
    pub const ARITY: usize = REWARD_MERKLE_TREE_V2_ARITY;

    pub fn new(height: usize) -> Self {
        Self {
            inner: InnerRewardMerkleTreeV2::new(height),
        }
    }

    pub fn from_commitment(commitment: impl Borrow<RewardMerkleCommitmentV2>) -> Self {
        Self {
            inner: InnerRewardMerkleTreeV2::from_commitment(commitment),
        }
    }

    /// Build a universal merkle tree from a key-value set.
    /// * `height` - height of the merkle tree
    /// * `data` - an iterator of key-value pairs. Could be a hashmap or simply
    ///   an array or a slice of (key, value) pairs
    pub fn from_kv_set<BI, BE>(
        height: usize,
        data: impl IntoIterator<Item = impl Borrow<(BI, BE)>>,
    ) -> Result<Self, MerkleTreeError>
    where
        BI: Borrow<RewardAccountV2>,
        BE: Borrow<RewardAmount>,
    {
        let mut mt = Self::new(height);
        for tuple in data.into_iter() {
            let (key, value) = tuple.borrow();
            UniversalMerkleTreeScheme::update(&mut mt, key.borrow(), value.borrow())?;
        }
        Ok(mt)
    }
}

/// Glorified Boolean type
pub type VerificationResult = Result<(), ()>;

// Core merkle tree trait
impl MerkleTreeScheme for RewardMerkleTreeV2 {
    type Element = RewardAmount;
    type Index = RewardAccountV2;
    type MembershipProof = <InnerRewardMerkleTreeV2 as MerkleTreeScheme>::MembershipProof;
    type Commitment = RewardMerkleCommitmentV2;
    type NodeValue = KeccakNode;
    type BatchMembershipProof = ();

    const ARITY: usize = REWARD_MERKLE_TREE_V2_ARITY;

    fn height(&self) -> usize {
        self.inner.height()
    }

    fn capacity(&self) -> bigdecimal::num_bigint::BigUint {
        self.inner.capacity()
    }

    fn num_leaves(&self) -> u64 {
        self.inner.num_leaves()
    }

    fn commitment(&self) -> Self::Commitment {
        self.inner.commitment()
    }

    fn lookup(
        &self,
        index: impl Borrow<Self::Index>,
    ) -> LookupResult<&Self::Element, Self::MembershipProof, ()> {
        self.inner.lookup(index)
    }

    fn verify(
        root: impl Borrow<Self::Commitment>,
        index: impl Borrow<Self::Index>,
        proof: impl Borrow<Self::MembershipProof>,
    ) -> Result<VerificationResult, MerkleTreeError> {
        InnerRewardMerkleTreeV2::verify(root, index, proof)
    }

    fn iter(&'_ self) -> MerkleTreeIter<'_, Self::Element, Self::Index, Self::NodeValue> {
        self.inner.iter()
    }
}

impl IntoIterator for RewardMerkleTreeV2 {
    type Item = (RewardAccountV2, RewardAmount);

    type IntoIter = MerkleTreeIntoIter<RewardAmount, RewardAccountV2, KeccakNode>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

// Universal merkle tree trait (supports non-membership proofs)
impl UniversalMerkleTreeScheme for RewardMerkleTreeV2 {
    type NonMembershipProof =
        <InnerRewardMerkleTreeV2 as UniversalMerkleTreeScheme>::NonMembershipProof;
    type BatchNonMembershipProof = ();

    fn update(
        &mut self,
        index: impl Borrow<Self::Index>,
        elem: impl Borrow<Self::Element>,
    ) -> Result<LookupResult<Self::Element, (), ()>, MerkleTreeError> {
        self.inner.update(index, elem)
    }

    fn update_with<F>(
        &mut self,
        pos: impl Borrow<Self::Index>,
        f: F,
    ) -> Result<LookupResult<Self::Element, (), ()>, MerkleTreeError>
    where
        F: FnOnce(Option<&Self::Element>) -> Option<Self::Element>,
    {
        self.inner.update_with(pos, f)
    }

    fn universal_lookup(
        &self,
        index: impl Borrow<Self::Index>,
    ) -> LookupResult<&Self::Element, Self::MembershipProof, Self::NonMembershipProof> {
        self.inner.universal_lookup(index)
    }

    fn non_membership_verify(
        root: impl Borrow<Self::Commitment>,
        index: impl Borrow<Self::Index>,
        proof: impl Borrow<Self::NonMembershipProof>,
    ) -> Result<bool, MerkleTreeError> {
        InnerRewardMerkleTreeV2::non_membership_verify(root, index, proof)
    }
}

// Persistent updates (functional style updates)
impl PersistentUniversalMerkleTreeScheme for RewardMerkleTreeV2 {
    fn persistent_update_with<F>(
        &self,
        pos: impl Borrow<Self::Index>,
        f: F,
    ) -> Result<Self, MerkleTreeError>
    where
        F: FnOnce(Option<&Self::Element>) -> Option<Self::Element>,
    {
        Ok(Self {
            inner: self.inner.persistent_update_with(pos, f)?,
        })
    }
}

// Forgetable trait (for pruning)
impl ForgetableMerkleTreeScheme for RewardMerkleTreeV2 {
    fn from_commitment(commitment: impl Borrow<Self::Commitment>) -> Self {
        Self {
            inner: InnerRewardMerkleTreeV2::from_commitment(commitment),
        }
    }

    fn forget(
        &mut self,
        index: impl Borrow<Self::Index>,
    ) -> LookupResult<Self::Element, Self::MembershipProof, ()> {
        self.inner.forget(index)
    }

    fn remember(
        &mut self,
        pos: impl Borrow<Self::Index>,
        element: impl Borrow<Self::Element>,
        proof: impl Borrow<Self::MembershipProof>,
    ) -> Result<(), MerkleTreeError> {
        self.inner.remember(pos, element, proof)
    }
}

// Forgetable universal merkle tree trait
impl ForgetableUniversalMerkleTreeScheme for RewardMerkleTreeV2 {
    fn universal_forget(
        &mut self,
        pos: Self::Index,
    ) -> LookupResult<Self::Element, Self::MembershipProof, Self::NonMembershipProof> {
        self.inner.universal_forget(pos)
    }

    fn non_membership_remember(
        &mut self,
        pos: Self::Index,
        proof: impl Borrow<Self::NonMembershipProof>,
    ) -> Result<(), MerkleTreeError> {
        self.inner.non_membership_remember(pos, proof)
    }
}
