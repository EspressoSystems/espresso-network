

use committable::Commitment;
use jf_merkle_tree::{
    prelude::{LightWeightSHA3MerkleTree, Sha3Digest, Sha3Node},
    universal_merkle_tree::UniversalMerkleTree,
    MerkleTreeScheme, 
};

use super::{FeeAccount, FeeAmount};
use crate::{ Header};

 

pub const BLOCK_MERKLE_TREE_HEIGHT: usize = 32;
pub const FEE_MERKLE_TREE_HEIGHT: usize = 20;
const FEE_MERKLE_TREE_ARITY: usize = 256;

// The block merkle tree accumulates header commitments. However, since the underlying
// representation of the commitment type remains the same even while the header itself changes,
// using the underlying type `[u8; 32]` allows us to use the same state type across minor versions.
pub type BlockMerkleTree = LightWeightSHA3MerkleTree<Commitment<Header>>;
pub type BlockMerkleCommitment = <BlockMerkleTree as MerkleTreeScheme>::Commitment;

pub type FeeMerkleTree =
    UniversalMerkleTree<FeeAmount, Sha3Digest, FeeAccount, FEE_MERKLE_TREE_ARITY, Sha3Node>;
pub type FeeMerkleCommitment = <FeeMerkleTree as MerkleTreeScheme>::Commitment;
