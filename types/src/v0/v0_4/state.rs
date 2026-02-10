use std::{
    collections::HashSet,
    sync::{Arc, OnceLock},
};

use alloy::primitives::{Address, U256};
use anyhow::Context;
use derive_more::{Display, From, Into};
use hotshot_contract_adapter::reward::{RewardAuthData, RewardClaimInput};
use jf_merkle_tree_compat::{
    universal_merkle_tree::UniversalMerkleTree, MerkleTreeScheme, UniversalMerkleTreeScheme,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::FeeAccount;
use crate::{
    v0::sparse_mt::{Keccak256Hasher, KeccakNode},
    v0_3::{RewardAccountV1, RewardAmount},
};

static REWARD_MERKLE_TREE_V2_MEMORY_LOCK: OnceLock<Arc<Semaphore>> = OnceLock::new();

pub const REWARD_MERKLE_TREE_V2_HEIGHT: usize = 160;
pub const REWARD_MERKLE_TREE_V2_ARITY: usize = 2;

pub type RewardMerkleCommitmentV2 = <RewardMerkleTreeV2 as MerkleTreeScheme>::Commitment;

pub type RewardMerkleTreeV2 = UniversalMerkleTree<
    RewardAmount,
    Keccak256Hasher,
    RewardAccountV2,
    REWARD_MERKLE_TREE_V2_ARITY,
    KeccakNode,
>;

#[derive(Clone)]
pub struct PermittedRewardMerkleTreeV2 {
    pub tree: RewardMerkleTreeV2,
    pub permit: Arc<OwnedSemaphorePermit>,
}

impl std::ops::Deref for PermittedRewardMerkleTreeV2 {
    type Target = RewardMerkleTreeV2;

    fn deref(&self) -> &Self::Target {
        &self.tree
    }
}

impl PermittedRewardMerkleTreeV2 {
    pub async fn try_from_kv_set(
        balances: Vec<(RewardAccountV2, RewardAmount)>,
    ) -> anyhow::Result<Self> {
        let permit = REWARD_MERKLE_TREE_V2_MEMORY_LOCK
            .get_or_init(|| Arc::new(Semaphore::new(1)))
            .clone()
            .acquire_owned()
            .await
            .context("Failed to acquire permit for RewardMerkleTreeV2")?;

        let tree = RewardMerkleTreeV2::from_kv_set(REWARD_MERKLE_TREE_V2_HEIGHT, balances)
            .context("Failed to rebuild reward merkle tree from balances")?;

        Ok(PermittedRewardMerkleTreeV2 {
            tree,
            permit: Arc::new(permit),
        })
    }
}

/// Return `true` if any of the given accounts have been forgotten in the `ValidatedState` reward_merkle_tree_v2
pub fn forgotten_accounts_include(tree: &RewardMerkleTreeV2, accounts: &[RewardAccountV2]) -> bool {
    for account in accounts {
        if tree.lookup(*account).expect_not_in_memory().is_ok() {
            return true;
        }
    }

    false
}

// New Type for `Address` in order to implement `CanonicalSerialize` and
// `CanonicalDeserialize`
// This is the same as `RewardAccountV1` but the `ToTraversal` trait implementation
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
#[display("{_0}")]
pub struct RewardAccountV2(pub Address);

impl From<RewardAccountV2> for RewardAccountV1 {
    fn from(account: RewardAccountV2) -> Self {
        RewardAccountV1(account.0)
    }
}

impl From<RewardAccountV1> for RewardAccountV2 {
    fn from(account: RewardAccountV1) -> Self {
        RewardAccountV2(account.0)
    }
}

/// A proof of the balance of an account in the fee ledger.
///
/// If the account of interest does not exist in the fee state, this is a Merkle non-membership
/// proof, and the balance is implicitly zero. Otherwise, this is a normal Merkle membership proof.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RewardAccountProofV2 {
    pub account: Address,
    pub proof: RewardMerkleProofV2,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum RewardMerkleProofV2 {
    Presence(<RewardMerkleTreeV2 as MerkleTreeScheme>::MembershipProof),
    Absence(<RewardMerkleTreeV2 as UniversalMerkleTreeScheme>::NonMembershipProof),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RewardAccountQueryDataV2 {
    pub balance: U256,
    pub proof: RewardAccountProofV2,
}

#[derive(Debug, thiserror::Error)]
pub enum RewardClaimError {
    #[error("Zero reward balance")]
    ZeroRewardError,
    #[error("Failed to convert proof: {0}")]
    ProofConversionError(#[from] anyhow::Error),
}

impl RewardAccountQueryDataV2 {
    /// Convert query data to reward claim input for contract submission.
    ///
    /// Auth root inputs (other than the reward merkle tree root) are currently
    /// all zero placeholder values.
    pub fn to_reward_claim_input(self) -> Result<RewardClaimInput, RewardClaimError> {
        if self.balance == U256::ZERO {
            return Err(RewardClaimError::ZeroRewardError);
        }

        let account_proof = match self.proof.proof {
            RewardMerkleProofV2::Presence(_) => self.proof,
            RewardMerkleProofV2::Absence(_) => {
                return Err(RewardClaimError::ZeroRewardError);
            },
        };

        Ok(RewardClaimInput {
            lifetime_rewards: self.balance,
            auth_data: RewardAuthData::new(account_proof.try_into()?).into(),
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Delta {
    pub fees_delta: HashSet<FeeAccount>,
    pub rewards_delta: HashSet<RewardAccountV2>,
}
