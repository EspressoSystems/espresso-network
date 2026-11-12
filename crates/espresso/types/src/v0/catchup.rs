//! Catchup requests, and the values a peer sends back once they are verified against the request.
//!
//! Every fetch method of [`StateCatchup`](super::traits::StateCatchup) takes a [`CatchupRequest`]
//! and returns a [`Verified`] value. A provider gets one only from [`CatchupRequest::verify`] on
//! the request it was asked to answer, or from [`Verified::new_unchecked`] for a trusted source.
//! The types do not stop a provider from verifying against a request it built itself, so
//! providers must use the one they were given.

use std::{fmt, future::Future, sync::Arc};

use anyhow::{Context, anyhow, ensure};
use committable::{Commitment, Committable};
#[cfg(feature = "node")]
use hotshot_types::utils::{StakeTableUnavailable, verify_leaf_chain};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_certificate::LightClientStateUpdateCertificateV2,
    stake_table::HSStakeTable,
};
use jf_merkle_tree_compat::{ForgetableMerkleTreeScheme, MerkleTreeScheme};
#[cfg(feature = "node")]
use versions::{EPOCH_VERSION, NEW_PROTOCOL_VERSION, Upgrade};

use crate::{
    BlockMerkleTree, FEE_MERKLE_TREE_HEIGHT, FeeAccount, FeeAccountProof, FeeMerkleCommitment,
    FeeMerkleTree, SeqTypes,
    v0_3::{
        ChainConfig, REWARD_MERKLE_TREE_V1_HEIGHT, RewardAccountProofV1, RewardAccountV1,
        RewardMerkleCommitmentV1, RewardMerkleTreeV1,
    },
    v0_4::{
        PermittedRewardMerkleTreeV2, RewardAccountV2, RewardMerkleCommitmentV2,
        RewardMerkleTreeV2Data, forgotten_accounts_include,
    },
    validate_state_cert,
};
#[cfg(feature = "node")]
use crate::{Certificate2, Leaf2};

/// A request for data from outside this node, carrying what the answer must match.
pub trait CatchupRequest: Send + Sync {
    /// What a responder sends back, as decoded from the wire.
    type Response: Send;
    /// What the caller receives once the response is checked.
    type Output: Send;

    /// Checks `response` against this request.
    fn verify(
        &self,
        response: Self::Response,
    ) -> impl Future<Output = Result<Verified<Self::Output>, VerifyError>> + Send;
}

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

impl CatchupRequest for FeeAccountsAt {
    type Response = FeeMerkleTree;
    type Output = Vec<FeeAccountProof>;

    /// One proof per requested account, each checked against [`Self::root`].
    async fn verify(
        &self,
        tree: FeeMerkleTree,
    ) -> Result<Verified<Vec<FeeAccountProof>>, VerifyError> {
        // `prove` sizes its lookup by the tree's own height, which comes from the peer.
        if tree.height() != FEE_MERKLE_TREE_HEIGHT {
            return Err(VerifyError::Invalid(anyhow!(
                "fee merkle tree has height {}, expected {FEE_MERKLE_TREE_HEIGHT}",
                tree.height()
            )));
        }
        self.accounts
            .iter()
            .map(|account| {
                let (proof, _) = FeeAccountProof::prove(&tree, (*account).into())
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

impl CatchupRequest for RewardAccountsV1At {
    type Response = RewardMerkleTreeV1;
    type Output = Vec<RewardAccountProofV1>;

    /// One proof per requested account, each checked against [`Self::root`].
    async fn verify(
        &self,
        tree: RewardMerkleTreeV1,
    ) -> Result<Verified<Vec<RewardAccountProofV1>>, VerifyError> {
        // `prove` sizes its lookup by the tree's own height, which comes from the peer.
        if tree.height() != REWARD_MERKLE_TREE_V1_HEIGHT {
            return Err(VerifyError::Invalid(anyhow!(
                "v1 reward merkle tree has height {}, expected {REWARD_MERKLE_TREE_V1_HEIGHT}",
                tree.height()
            )));
        }
        self.accounts
            .iter()
            .map(|account| {
                let (proof, _) = RewardAccountProofV1::prove(&tree, (*account).into())
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

/// The frontier of the block tree at a block: the proof of its last leaf.
#[derive(Clone, Debug)]
pub struct FrontierAt {
    pub height: u64,
    pub view: ViewNumber,
    /// The tree the frontier must prove against, which holds at least its root.
    pub tree: BlockMerkleTree,
}

impl FrontierAt {
    fn remember(
        &self,
        frontier: &<BlockMerkleTree as MerkleTreeScheme>::MembershipProof,
    ) -> anyhow::Result<BlockMerkleTree> {
        let last = self
            .tree
            .num_leaves()
            .checked_sub(1)
            .context("an empty block tree has no frontier")?;
        let elem = frontier
            .elem()
            .context("frontier is missing its leaf element")?;
        let mut tree = self.tree.clone();
        tree.remember(last, *elem, frontier)
            .context("frontier does not prove against the block tree root")?;
        Ok(tree)
    }
}

impl CatchupRequest for FrontierAt {
    type Response = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;
    type Output = BlockMerkleTree;

    /// A copy of [`Self::tree`] that remembers the frontier.
    async fn verify(
        &self,
        frontier: Self::Response,
    ) -> Result<Verified<BlockMerkleTree>, VerifyError> {
        self.remember(&frontier)
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
    /// Accepts `tree` if its root is [`Self::root`] and none of [`Self::accounts`] is forgotten
    /// in it. For a tree that is already built, such as one loaded from local storage.
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

impl CatchupRequest for RewardTreeV2At {
    /// A bincode-encoded [`RewardMerkleTreeV2Data`].
    type Response = Vec<u8>;
    type Output = PermittedRewardMerkleTreeV2;

    /// Rebuilds the tree from its balances, then verifies it as [`Self::verify_tree`] does.
    ///
    /// Building the tree waits for the process-wide reward tree permit.
    async fn verify(
        &self,
        bytes: Vec<u8>,
    ) -> Result<Verified<PermittedRewardMerkleTreeV2>, VerifyError> {
        let data = bincode::deserialize::<RewardMerkleTreeV2Data>(&bytes)
            .context("decoding reward merkle tree v2")
            .map_err(VerifyError::Invalid)?;
        let tree = PermittedRewardMerkleTreeV2::try_from_kv_set(data.balances)
            .await
            .map_err(VerifyError::Invalid)?;
        self.verify_tree(tree)
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

/// A responder's answer to a [`LeafAt`] request.
#[cfg(feature = "node")]
#[derive(Clone, Debug)]
pub struct LeafResponse {
    /// A legacy 3-chain that decides the requested leaf, or a new-protocol leaf range from the
    /// requested leaf up to the one its cert2 finalizes.
    pub leaves: Vec<Leaf2>,
    /// The cert2 at [`LeafAt::cert2_height`], which only a new-protocol chain has.
    pub cert2: Option<Certificate2<SeqTypes>>,
}

#[cfg(feature = "node")]
impl LeafAt {
    /// The height of the cert2 that finalizes `leaves`, or `None` for a legacy chain, which
    /// decides the requested leaf on its own.
    ///
    /// The protocol comes from the version of the leaf at [`Self::height`], which its commitment
    /// covers, never from the order or the other leaves of the response.
    pub fn cert2_height(&self, leaves: &[Leaf2]) -> Result<Option<u64>, VerifyError> {
        let mut requested = leaves
            .iter()
            .filter(|leaf| leaf.height() == self.height)
            .peekable();
        if requested.peek().is_none() {
            return Err(VerifyError::Invalid(anyhow!(
                "leaf chain has no leaf at the requested height {}",
                self.height
            )));
        }
        if !requested.any(|leaf| leaf.block_header().version() >= NEW_PROTOCOL_VERSION) {
            return Ok(None);
        }
        Ok(leaves.iter().map(Leaf2::height).max())
    }

    async fn verify_chain(&self, response: LeafResponse) -> anyhow::Result<Leaf2> {
        let Some(cert2_height) = self.cert2_height(&response.leaves)? else {
            let upgrade_lock = UpgradeLock::<SeqTypes>::new(Upgrade::trivial(EPOCH_VERSION));
            return verify_leaf_chain(
                response.leaves,
                &self.coordinator,
                self.height,
                &upgrade_lock,
            )
            .await;
        };
        let cert2 = response
            .cert2
            .with_context(|| format!("leaf chain is missing its cert2 at height {cert2_height}"))?;
        let upgrade_lock = UpgradeLock::<SeqTypes>::new(Upgrade::trivial(NEW_PROTOCOL_VERSION));
        hotshot_new_protocol::utils::verify_new_protocol_leaf_chain(
            response.leaves,
            &self.coordinator,
            self.height,
            &upgrade_lock,
            cert2,
        )
        .await
    }
}

#[cfg(feature = "node")]
impl CatchupRequest for LeafAt {
    type Response = LeafResponse;
    type Output = Leaf2;

    /// The leaf at [`Self::height`], finalized by the chain in `response`.
    async fn verify(&self, response: LeafResponse) -> Result<Verified<Leaf2>, VerifyError> {
        let err = match self.verify_chain(response).await {
            Ok(leaf) => return Ok(Verified::new(leaf)),
            Err(err) => err.context(format!(
                "failed to verify leaf chain at height {}",
                self.height
            )),
        };
        match err.downcast_ref::<StakeTableUnavailable>() {
            Some(StakeTableUnavailable(epoch)) => Err(VerifyError::Unavailable(*epoch)),
            None => Err(VerifyError::Invalid(err)),
        }
    }
}

/// The state certificate of an epoch, checked against that epoch's stake table.
#[derive(Clone, Debug)]
pub struct StateCertFor {
    pub epoch: EpochNumber,
    pub stake_table: HSStakeTable<SeqTypes>,
    pub epoch_height: u64,
    pub upgrade_lock: UpgradeLock<SeqTypes>,
}

impl CatchupRequest for StateCertFor {
    type Response = LightClientStateUpdateCertificateV2<SeqTypes>;
    type Output = LightClientStateUpdateCertificateV2<SeqTypes>;

    /// Accepts `cert` if it certifies the root block of [`Self::epoch`] with enough of
    /// [`Self::stake_table`], as [`validate_state_cert`] checks.
    async fn verify(
        &self,
        cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> Result<Verified<LightClientStateUpdateCertificateV2<SeqTypes>>, VerifyError> {
        validate_state_cert(
            &cert,
            &self.stake_table,
            self.epoch,
            self.epoch_height,
            &self.upgrade_lock,
        )
        .with_context(|| format!("state certificate for epoch {}", self.epoch))
        .map_err(VerifyError::Invalid)?;
        Ok(Verified::new(cert))
    }
}

/// A commitment requests the chain config it commits to.
impl CatchupRequest for Commitment<ChainConfig> {
    type Response = ChainConfig;
    type Output = ChainConfig;

    async fn verify(
        &self,
        chain_config: ChainConfig,
    ) -> Result<Verified<ChainConfig>, VerifyError> {
        if chain_config.commit() != *self {
            return Err(VerifyError::Invalid(anyhow!(
                "received chain config with mismatched commitment: expected {self}, got {}",
                chain_config.commit()
            )));
        }
        Ok(Verified::new(chain_config))
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::{Address, U256};
    use committable::RawCommitmentBuilder;
    use jf_merkle_tree_compat::{AppendableMerkleTreeScheme, UniversalMerkleTreeScheme};

    use super::*;
    use crate::{
        BLOCK_MERKLE_TREE_HEIGHT, BlockSize, FEE_MERKLE_TREE_HEIGHT, FeeAmount, Header,
        retain_accounts, v0_3::RewardAmount,
    };

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

    fn block_tree(tag: &str, blocks: u64) -> BlockMerkleTree {
        let mut tree = BlockMerkleTree::new(BLOCK_MERKLE_TREE_HEIGHT);
        for block in 0..blocks {
            tree.push(
                RawCommitmentBuilder::<Header>::new(tag)
                    .u64(block)
                    .finalize(),
            )
            .unwrap();
        }
        tree
    }

    fn frontier_at(tree: &BlockMerkleTree) -> FrontierAt {
        FrontierAt {
            height: tree.num_leaves(),
            view: ViewNumber::new(1),
            tree: BlockMerkleTree::from_commitment(tree.commitment()),
        }
    }

    fn reward_balances() -> Vec<(RewardAccountV2, RewardAmount)> {
        vec![(
            RewardAccountV2(Address::repeat_byte(1)),
            RewardAmount(U256::from(100)),
        )]
    }

    async fn reward_tree_at(balances: Vec<(RewardAccountV2, RewardAmount)>) -> RewardTreeV2At {
        let root = PermittedRewardMerkleTreeV2::try_from_kv_set(balances)
            .await
            .unwrap()
            .tree
            .commitment();
        RewardTreeV2At {
            height: 1,
            view: ViewNumber::new(1),
            root,
            accounts: Default::default(),
        }
    }

    #[tokio::test]
    async fn fee_accounts_prove_presence_and_absence_against_the_requested_root() {
        let tree = fee_tree(&[account(1)]);
        let req = fee_accounts_at(tree.commitment(), vec![account(1), account(2)]);

        let proofs = req.verify(tree).await.unwrap();

        let balances = proofs
            .inner()
            .iter()
            .map(|proof| proof.verify(&req.root).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(balances, [U256::from(100), U256::ZERO]);
    }

    #[tokio::test]
    async fn fee_accounts_reject_a_tree_with_another_root() {
        let tree = fee_tree(&[account(1)]);
        let req = fee_accounts_at(fee_tree(&[]).commitment(), vec![account(1)]);

        req.verify(tree).await.unwrap_err();
    }

    #[tokio::test]
    async fn fee_accounts_reject_a_response_missing_a_requested_account() {
        let tree = fee_tree(&[account(1), account(2)]);
        let response = retain_accounts(&tree, [account(2)]).unwrap();
        let req = fee_accounts_at(tree.commitment(), vec![account(1)]);

        let err = req.verify(response).await.unwrap_err();

        assert!(
            format!("{err:#}").contains("response missing fee account"),
            "{err:#}"
        );
    }

    #[tokio::test]
    async fn fee_accounts_reject_a_tree_of_another_height() {
        let tree = FeeMerkleTree::new(FEE_MERKLE_TREE_HEIGHT + 1);
        let req = fee_accounts_at(tree.commitment(), vec![account(1)]);

        let err = req.verify(tree).await.unwrap_err();

        assert!(format!("{err:#}").contains("has height"), "{err:#}");
    }

    #[tokio::test]
    async fn v1_reward_accounts_reject_a_tree_of_another_height() {
        let tree = RewardMerkleTreeV1::new(REWARD_MERKLE_TREE_V1_HEIGHT + 1);
        let req = RewardAccountsV1At {
            height: 1,
            view: ViewNumber::new(1),
            root: tree.commitment(),
            accounts: vec![RewardAccountV1(Address::repeat_byte(1))],
        };

        let err = req.verify(tree).await.unwrap_err();

        assert!(format!("{err:#}").contains("has height"), "{err:#}");
    }

    #[tokio::test]
    async fn frontier_is_remembered_in_a_tree_that_holds_only_the_root() {
        let tree = block_tree("chain", 3);
        let (_, frontier) = tree.lookup(2).expect_ok().unwrap();

        let remembered = frontier_at(&tree).verify(frontier).await.unwrap();

        assert!(remembered.inner().lookup(2).expect_ok().is_ok());
        assert_eq!(remembered.inner().commitment(), tree.commitment());
    }

    #[tokio::test]
    async fn frontier_of_another_chain_is_rejected() {
        let tree = block_tree("chain", 3);
        let (_, frontier) = block_tree("fork", 3).lookup(2).expect_ok().unwrap();

        let err = frontier_at(&tree).verify(frontier).await.unwrap_err();

        assert!(matches!(err, VerifyError::Invalid(_)), "{err:#}");
    }

    #[tokio::test]
    async fn reward_tree_v2_is_rebuilt_from_its_wire_encoding() {
        let balances = reward_balances();
        let req = reward_tree_at(balances.clone()).await;
        let bytes = bincode::serialize(&RewardMerkleTreeV2Data { balances }).unwrap();

        let tree = req.verify(bytes).await.unwrap();

        assert_eq!(tree.inner().tree.commitment(), req.root);
    }

    #[tokio::test]
    async fn reward_tree_v2_with_other_balances_is_rejected() {
        let req = reward_tree_at(reward_balances()).await;
        let bytes = bincode::serialize(&RewardMerkleTreeV2Data { balances: vec![] }).unwrap();

        let err = req.verify(bytes).await.err().unwrap();

        assert!(
            format!("{err:#}").contains("does not match root"),
            "{err:#}"
        );
    }

    #[tokio::test]
    async fn reward_tree_v2_that_does_not_decode_is_rejected() {
        let req = reward_tree_at(reward_balances()).await;

        let err = req.verify(vec![0xff; 3]).await.err().unwrap();

        assert!(matches!(err, VerifyError::Invalid(_)), "{err:#}");
    }

    #[tokio::test]
    async fn chain_config_must_have_the_requested_commitment() {
        let requested = ChainConfig::default();
        let other = ChainConfig {
            max_block_size: BlockSize(1),
            ..requested
        };

        requested.commit().verify(other).await.unwrap_err();
        assert_eq!(
            requested
                .commit()
                .verify(requested)
                .await
                .unwrap()
                .into_inner(),
            requested
        );
    }
}
