//! Catchup requests, and the values a peer sends back once they are verified against the request.
//!
//! The leaf, fee account, reward account, reward tree and chain config methods of
//! [`StateCatchup`](super::traits::StateCatchup) return [`Verified`] values. A provider gets one
//! only from the `verify` function of the request it was asked to answer, or from
//! [`Verified::new_unchecked`] for a trusted source. The types do not stop a provider from
//! verifying against a request it built itself, so providers must use the one they were given.
//!
//! Not covered yet: the blocks frontier is checked by `remember` inside each provider, and state
//! certificates are checked only by their consumer.

use std::{fmt, sync::Arc};

use anyhow::{Context, anyhow, bail, ensure};
use committable::{Commitment, Committable};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
};
#[cfg(feature = "node")]
use hotshot_types::{
    message::UpgradeLock,
    utils::{StakeTableUnavailable, verify_leaf_chain},
};
use jf_merkle_tree_compat::MerkleTreeScheme;
use versions::NEW_PROTOCOL_VERSION;
#[cfg(feature = "node")]
use versions::{EPOCH_VERSION, Upgrade};

#[cfg(feature = "node")]
use crate::Certificate2;
use crate::{
    FeeAccount, FeeAccountProof, FeeMerkleCommitment, FeeMerkleTree, Leaf2, SeqTypes,
    v0_3::{
        ChainConfig, RewardAccountProofV1, RewardAccountV1, RewardAmount, RewardMerkleCommitmentV1,
        RewardMerkleTreeV1,
    },
    v0_4::{
        PermittedRewardMerkleTreeV2, RewardAccountV2, RewardMerkleCommitmentV2,
        forgotten_accounts_include,
    },
};

/// Why a response failed verification.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    /// The response is wrong. The responder is at fault, and another one may answer correctly.
    #[error(transparent)]
    Invalid(anyhow::Error),
    /// This node does not hold the stake table the response must be checked against yet. Looking
    /// it up started its catchup, so the same response may verify later.
    #[error("no stake table available for epoch {0} to verify the response against")]
    Unavailable(EpochNumber),
}

/// A value checked against the request it answers.
#[derive(Clone, Debug)]
pub struct Verified<T>(T);

impl<T> Verified<T> {
    fn new(value: T) -> Self {
        Self(value)
    }

    /// Marks `value` as verified without checking it.
    ///
    /// Only for values from a source that holds nothing unverified: local storage written only
    /// with verified values, or data this node produced itself.
    pub fn new_unchecked(value: T) -> Self {
        Self(value)
    }

    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

/// Fee accounts at a block, and the fee state root they must prove against.
#[derive(Clone, Debug)]
pub struct FeeAccountsAt {
    pub height: u64,
    pub view: ViewNumber,
    pub root: FeeMerkleCommitment,
    pub accounts: Vec<FeeAccount>,
}

impl FeeAccountsAt {
    /// One proof per requested account, each checked against [`Self::root`].
    pub fn verify(
        &self,
        tree: &FeeMerkleTree,
    ) -> Result<Verified<Vec<FeeAccountProof>>, VerifyError> {
        self.accounts
            .iter()
            .map(|account| {
                let (proof, _) = FeeAccountProof::prove(tree, (*account).into())
                    .with_context(|| format!("response missing fee account {account}"))?;
                proof.verify(&self.root).with_context(|| {
                    format!(
                        "invalid proof for fee account {account}, root: {}",
                        self.root
                    )
                })?;
                Ok(proof)
            })
            .collect::<anyhow::Result<_>>()
            .map(Verified::new)
            .map_err(VerifyError::Invalid)
    }
}

/// V1 reward accounts at a block, and the v1 reward root they must prove against.
#[derive(Clone, Debug)]
pub struct RewardAccountsV1At {
    pub height: u64,
    pub view: ViewNumber,
    pub root: RewardMerkleCommitmentV1,
    pub accounts: Vec<RewardAccountV1>,
}

impl RewardAccountsV1At {
    /// One proof per requested account, each checked against [`Self::root`].
    pub fn verify(
        &self,
        tree: &RewardMerkleTreeV1,
    ) -> Result<Verified<Vec<RewardAccountProofV1>>, VerifyError> {
        self.accounts
            .iter()
            .map(|account| {
                let (proof, _) = RewardAccountProofV1::prove(tree, (*account).into())
                    .with_context(|| format!("response missing v1 reward account {account}"))?;
                proof.verify(&self.root).with_context(|| {
                    format!(
                        "invalid proof for v1 reward account {account}, root: {}",
                        self.root
                    )
                })?;
                Ok(proof)
            })
            .collect::<anyhow::Result<_>>()
            .map(Verified::new)
            .map_err(VerifyError::Invalid)
    }
}

/// The whole v2 reward tree at a block, which must have the given root and must not have
/// forgotten any of the given accounts.
#[derive(Clone, Debug)]
pub struct RewardTreeV2At {
    pub height: u64,
    pub view: ViewNumber,
    pub root: RewardMerkleCommitmentV2,
    pub accounts: Arc<Vec<RewardAccountV2>>,
}

impl RewardTreeV2At {
    /// Rebuilds the tree from its balances, then verifies it as [`Self::verify_tree`] does.
    ///
    /// Building the tree waits for the process-wide reward tree permit.
    pub async fn verify_balances(
        &self,
        balances: Vec<(RewardAccountV2, RewardAmount)>,
    ) -> Result<Verified<PermittedRewardMerkleTreeV2>, VerifyError> {
        let tree = PermittedRewardMerkleTreeV2::try_from_kv_set(balances)
            .await
            .map_err(VerifyError::Invalid)?;
        self.verify_tree(tree)
    }

    /// Accepts `tree` if its root is [`Self::root`] and none of [`Self::accounts`] is forgotten
    /// in it.
    pub fn verify_tree(
        &self,
        tree: PermittedRewardMerkleTreeV2,
    ) -> Result<Verified<PermittedRewardMerkleTreeV2>, VerifyError> {
        self.check_tree(&tree).map_err(VerifyError::Invalid)?;
        Ok(Verified::new(tree))
    }

    fn check_tree(&self, tree: &PermittedRewardMerkleTreeV2) -> anyhow::Result<()> {
        ensure!(
            tree.tree.commitment() == self.root,
            "reward merkle tree v2 does not match root {}",
            self.root
        );
        ensure!(
            !forgotten_accounts_include(&tree.tree, &self.accounts),
            "reward merkle tree v2 is missing requested accounts"
        );
        Ok(())
    }
}

/// The leaf at a height, verified through the stake tables `coordinator` resolves.
#[derive(Clone)]
pub struct LeafAt {
    pub height: u64,
    pub coordinator: EpochMembershipCoordinator<SeqTypes>,
}

impl fmt::Debug for LeafAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LeafAt")
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl LeafAt {
    /// The height of the cert2 that finalizes the chain, taken from its last leaf, if the first
    /// leaf is new-protocol. `None` means a legacy chain, which needs no cert2. Errors on an
    /// empty chain.
    pub fn cert2_height(&self, leaves: &[Leaf2]) -> anyhow::Result<Option<u64>> {
        let (Some(first), Some(last)) = (leaves.first(), leaves.last()) else {
            bail!("empty leaf chain for height {}", self.height);
        };
        Ok((first.block_header().version() >= NEW_PROTOCOL_VERSION).then(|| last.height()))
    }

    /// Verifies `leaves` and returns the leaf at [`Self::height`].
    ///
    /// A new-protocol chain needs the cert2 at [`Self::cert2_height`]. A legacy chain ignores
    /// `cert2`.
    #[cfg(feature = "node")]
    pub async fn verify(
        &self,
        leaves: Vec<Leaf2>,
        cert2: Option<Certificate2<SeqTypes>>,
    ) -> Result<Verified<Leaf2>, VerifyError> {
        match self.verify_chain(leaves, cert2).await {
            Ok(leaf) => Ok(Verified::new(leaf)),
            Err(err) => match err.downcast_ref::<StakeTableUnavailable>() {
                Some(StakeTableUnavailable(epoch)) => Err(VerifyError::Unavailable(*epoch)),
                None => Err(VerifyError::Invalid(err)),
            },
        }
    }

    #[cfg(feature = "node")]
    async fn verify_chain(
        &self,
        leaves: Vec<Leaf2>,
        cert2: Option<Certificate2<SeqTypes>>,
    ) -> anyhow::Result<Leaf2> {
        let leaf = match self.cert2_height(&leaves)? {
            Some(_) => {
                let cert2 = cert2.context("new-protocol leaf chain needs its cert2")?;
                let upgrade_lock =
                    UpgradeLock::<SeqTypes>::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));
                hotshot_new_protocol::utils::verify_new_protocol_leaf_chain(
                    leaves,
                    &self.coordinator,
                    self.height,
                    &upgrade_lock,
                    cert2,
                )
                .await
                .with_context(|| {
                    format!(
                        "failed to verify leaf chain with cert2 at height {}",
                        self.height
                    )
                })?
            },
            None => {
                let upgrade_lock = UpgradeLock::<SeqTypes>::new(Upgrade::trivial(EPOCH_VERSION));
                verify_leaf_chain(leaves, &self.coordinator, self.height, &upgrade_lock)
                    .await
                    .with_context(|| {
                        format!("failed to verify leaf chain at height {}", self.height)
                    })?
            },
        };
        Ok(leaf)
    }
}

/// The chain config, if it has the requested commitment.
pub fn verify_chain_config(
    commitment: Commitment<ChainConfig>,
    chain_config: ChainConfig,
) -> Result<Verified<ChainConfig>, VerifyError> {
    if chain_config.commit() != commitment {
        return Err(VerifyError::Invalid(anyhow!(
            "received chain config with mismatched commitment: expected {commitment}, got {}",
            chain_config.commit()
        )));
    }
    Ok(Verified::new(chain_config))
}

#[cfg(test)]
mod tests {
    use alloy::primitives::{Address, U256};
    use jf_merkle_tree_compat::UniversalMerkleTreeScheme;

    use super::*;
    use crate::{BlockSize, FEE_MERKLE_TREE_HEIGHT, FeeAmount, retain_accounts};

    fn account(byte: u8) -> FeeAccount {
        FeeAccount::from(Address::repeat_byte(byte))
    }

    fn fee_tree(accounts: &[FeeAccount]) -> FeeMerkleTree {
        let mut tree = FeeMerkleTree::new(FEE_MERKLE_TREE_HEIGHT);
        for account in accounts {
            tree.update(*account, FeeAmount::from(100)).unwrap();
        }
        tree
    }

    fn fee_accounts_at(root: FeeMerkleCommitment, accounts: Vec<FeeAccount>) -> FeeAccountsAt {
        FeeAccountsAt {
            height: 1,
            view: ViewNumber::new(1),
            root,
            accounts,
        }
    }

    #[test]
    fn fee_accounts_prove_presence_and_absence_against_the_requested_root() {
        let tree = fee_tree(&[account(1)]);
        let req = fee_accounts_at(tree.commitment(), vec![account(1), account(2)]);

        let proofs = req.verify(&tree).unwrap();

        let balances = proofs
            .inner()
            .iter()
            .map(|proof| proof.verify(&req.root).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(balances, [U256::from(100), U256::ZERO]);
    }

    #[test]
    fn fee_accounts_reject_a_tree_with_another_root() {
        let tree = fee_tree(&[account(1)]);
        let req = fee_accounts_at(fee_tree(&[]).commitment(), vec![account(1)]);

        req.verify(&tree).unwrap_err();
    }

    #[test]
    fn fee_accounts_reject_a_response_missing_a_requested_account() {
        let tree = fee_tree(&[account(1), account(2)]);
        let response = retain_accounts(&tree, [account(2)]).unwrap();
        let req = fee_accounts_at(tree.commitment(), vec![account(1)]);

        let err = req.verify(&response).unwrap_err();

        assert!(
            format!("{err:#}").contains("response missing fee account"),
            "{err:#}"
        );
    }

    #[test]
    fn chain_config_must_have_the_requested_commitment() {
        let requested = ChainConfig::default();
        let other = ChainConfig {
            max_block_size: BlockSize(1),
            ..requested
        };

        verify_chain_config(requested.commit(), other).unwrap_err();
        assert_eq!(
            verify_chain_config(requested.commit(), requested)
                .unwrap()
                .into_inner(),
            requested
        );
    }
}
