use std::ops::Add;

use alloy::primitives::{Address, U256};
use anyhow::{bail, Context};
use committable::{Commitment, Committable};
use hotshot::types::BLSPubKey;
use hotshot_query_service::merklized_state::MerklizedState;
use hotshot_types::{
    data::{BlockError, ViewNumber},
    traits::{
        block_contents::BlockHeader, node_implementation::ConsensusTime,
        signature_key::BuilderSignatureKey, states::StateDelta, ValidatedState as HotShotState,
    },
};
use itertools::Itertools;
use jf_merkle_tree::{
    prelude::{MerkleProof, Sha3Digest, Sha3Node},
    AppendableMerkleTreeScheme, ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme,
    LookupResult, MerkleCommitment, MerkleTreeError, MerkleTreeScheme,
    PersistentUniversalMerkleTreeScheme, UniversalMerkleTreeScheme,
};
use num_traits::CheckedSub;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use vbs::version::{StaticVersionType, Version};

use super::{
    auction::ExecutionError,
    fee_info::FeeError,
    instance_state::NodeState,
    reward::{apply_rewards, find_validator_info, first_two_epochs},
    v0_1::{
        RewardAccount, RewardAmount, RewardMerkleCommitment, RewardMerkleTree,
        REWARD_MERKLE_TREE_HEIGHT,
    },
    v0_3::Validator,
    BlockMerkleCommitment, BlockSize, EpochVersion, FeeMerkleCommitment, L1Client,
    MarketplaceVersion,
};
use crate::{
    traits::StateCatchup,
    v0_99::{ChainConfig, FullNetworkTx, IterableFeeInfo, ResolvableChainConfig},
    BlockMerkleTree, Delta, FeeAccount, FeeAmount, FeeInfo, FeeMerkleTree, Header, Leaf2,
    NsTableValidationError, PayloadByteLen, SeqTypes, UpgradeType, BLOCK_MERKLE_TREE_HEIGHT,
    FEE_MERKLE_TREE_HEIGHT,
};

/// This enum is not used in code but functions as an index of
/// possible validation errors.
#[allow(dead_code)]
pub enum StateValidationError {
    ProposalValidation(ProposalValidationError),
    BuilderValidation(BuilderValidationError),
    Fee(FeeError),
}

/// Possible builder validation failures
#[derive(Error, Debug, Eq, PartialEq)]
pub enum BuilderValidationError {
    #[error("Builder signature not found")]
    SignatureNotFound,
    #[error("Fee amount out of range: {0}")]
    FeeAmountOutOfRange(FeeAmount),
    #[error("Invalid Builder Signature")]
    InvalidBuilderSignature,
}

/// Possible proposal validation failures
#[derive(Error, Debug, Eq, PartialEq)]
pub enum ProposalValidationError {
    #[error("Invalid ChainConfig: expected={expected:?}, proposal={proposal:?}")]
    InvalidChainConfig {
        expected: Box<ChainConfig>,
        proposal: Box<ResolvableChainConfig>,
    },
    #[error(
        "Invalid Payload Size: (max_block_size={max_block_size}, proposed_block_size={block_size})"
    )]
    MaxBlockSizeExceeded {
        max_block_size: BlockSize,
        block_size: BlockSize,
    },
    #[error("Insufficient Fee: block_size={max_block_size}, base_fee={base_fee}, proposed_fee={proposed_fee}")]
    InsufficientFee {
        max_block_size: BlockSize,
        base_fee: FeeAmount,
        proposed_fee: FeeAmount,
    },
    #[error("Invalid Height: parent_height={parent_height}, proposal_height={proposal_height}")]
    InvalidHeight {
        parent_height: u64,
        proposal_height: u64,
    },
    #[error("Invalid Block Root Error: expected={expected_root:?}, proposal={proposal_root:?}")]
    InvalidBlockRoot {
        expected_root: BlockMerkleCommitment,
        proposal_root: BlockMerkleCommitment,
    },
    #[error("Invalid Fee Root Error: expected={expected_root:?}, proposal={proposal_root:?}")]
    InvalidFeeRoot {
        expected_root: FeeMerkleCommitment,
        proposal_root: FeeMerkleCommitment,
    },
    #[error("Invalid Reward Root Error: expected={expected_root:?}, proposal={proposal_root:?}")]
    InvalidRewardRoot {
        expected_root: RewardMerkleCommitment,
        proposal_root: RewardMerkleCommitment,
    },
    #[error("Invalid namespace table: {0}")]
    InvalidNsTable(NsTableValidationError),
    #[error("Some fee amount or their sum total out of range")]
    SomeFeeAmountOutOfRange,
    #[error("Invalid timestamp: proposal={proposal_timestamp}, parent={parent_timestamp}")]
    DecrementingTimestamp {
        proposal_timestamp: u64,
        parent_timestamp: u64,
    },
    #[error("Timestamp drift too high: proposed:={proposal}, system={system}, diff={diff}")]
    InvalidTimestampDrift {
        proposal: u64,
        system: u64,
        diff: u64,
    },
    #[error("l1_finalized has `None` value")]
    L1FinalizedNotFound,
    #[error("l1_finalized height is decreasing: parent={parent:?} proposed={proposed:?}")]
    L1FinalizedDecrementing {
        parent: Option<(u64, u64)>,
        proposed: Option<(u64, u64)>,
    },
    #[error("Invalid proposal: l1_head height is decreasing")]
    DecrementingL1Head,
    #[error("Builder Validation Error: {0}")]
    BuilderValidationError(BuilderValidationError),
    #[error("Invalid proposal: l1 finalized does not match the proposal")]
    InvalidL1Finalized,
    #[error("reward root not found")]
    RewardRootNotFound {},
}

impl StateDelta for Delta {}

#[derive(Hash, Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
/// State to be validated by replicas.
pub struct ValidatedState {
    /// Frontier of [`BlockMerkleTree`]
    pub block_merkle_tree: BlockMerkleTree,
    /// Frontier of [`FeeMerkleTree`]
    pub fee_merkle_tree: FeeMerkleTree,
    pub reward_merkle_tree: RewardMerkleTree,
    /// Configuration [`Header`] proposals will be validated against.
    pub chain_config: ResolvableChainConfig,
}

impl Default for ValidatedState {
    fn default() -> Self {
        let block_merkle_tree = BlockMerkleTree::from_elems(
            Some(BLOCK_MERKLE_TREE_HEIGHT),
            Vec::<Commitment<Header>>::new(),
        )
        .unwrap();

        // Words of wisdom from @mrain: "capacity = arity^height"
        // "For index space 2^160, arity 256 (2^8),
        // you should set the height as 160/8=20"
        let fee_merkle_tree = FeeMerkleTree::from_kv_set(
            FEE_MERKLE_TREE_HEIGHT,
            Vec::<(FeeAccount, FeeAmount)>::new(),
        )
        .unwrap();

        let reward_merkle_tree = RewardMerkleTree::from_kv_set(
            REWARD_MERKLE_TREE_HEIGHT,
            Vec::<(RewardAccount, RewardAmount)>::new(),
        )
        .unwrap();

        let chain_config = ResolvableChainConfig::from(ChainConfig::default());

        Self {
            block_merkle_tree,
            fee_merkle_tree,
            reward_merkle_tree,
            chain_config,
        }
    }
}

impl ValidatedState {
    /// Prefund an account with a given amount. Only for demo purposes.
    pub fn prefund_account(&mut self, account: FeeAccount, amount: FeeAmount) {
        self.fee_merkle_tree.update(account, amount).unwrap();
    }

    pub fn balance(&mut self, account: FeeAccount) -> Option<FeeAmount> {
        match self.fee_merkle_tree.lookup(account) {
            LookupResult::Ok(balance, _) => Some(*balance),
            LookupResult::NotFound(_) => Some(0.into()),
            LookupResult::NotInMemory => None,
        }
    }

    /// Find accounts that are not in memory.
    ///
    /// As an optimization we could try to apply updates and return the
    /// forgotten accounts to be fetched from peers and update them later.
    pub fn forgotten_accounts(
        &self,
        accounts: impl IntoIterator<Item = FeeAccount>,
    ) -> Vec<FeeAccount> {
        accounts
            .into_iter()
            .unique()
            .filter(|account| {
                self.fee_merkle_tree
                    .lookup(*account)
                    .expect_not_in_memory()
                    .is_ok()
            })
            .collect()
    }

    pub fn forgotten_reward_accounts(
        &self,
        accounts: impl IntoIterator<Item = RewardAccount>,
    ) -> Vec<RewardAccount> {
        accounts
            .into_iter()
            .unique()
            .filter(|account| {
                self.reward_merkle_tree
                    .lookup(*account)
                    .expect_not_in_memory()
                    .is_ok()
            })
            .collect()
    }

    /// Check if the merkle tree is available
    pub fn need_to_fetch_blocks_mt_frontier(&self) -> bool {
        let num_leaves = self.block_merkle_tree.num_leaves();
        if num_leaves == 0 {
            false
        } else {
            self.block_merkle_tree
                .lookup(num_leaves - 1)
                .expect_ok()
                .is_err()
        }
    }

    /// Insert a fee deposit receipt
    pub fn insert_fee_deposit(
        &mut self,
        fee_info: FeeInfo,
    ) -> anyhow::Result<LookupResult<FeeAmount, (), ()>> {
        Ok(self
            .fee_merkle_tree
            .update_with(fee_info.account, |balance| {
                Some(balance.cloned().unwrap_or_default().add(fee_info.amount))
            })?)
    }

    pub fn apply_proposal(
        &mut self,
        delta: &mut Delta,
        parent_leaf: &Leaf2,
        l1_deposits: Vec<FeeInfo>,
    ) {
        // pushing a block into merkle tree shouldn't fail
        self.block_merkle_tree
            .push(parent_leaf.block_header().commit())
            .unwrap();

        for FeeInfo { account, amount } in l1_deposits.iter() {
            self.fee_merkle_tree
                .update_with(account, |balance| {
                    Some(balance.cloned().unwrap_or_default().add(*amount))
                })
                .expect("update_with succeeds");
            delta.fees_delta.insert(*account);
        }
    }

    pub fn charge_fees(
        &mut self,
        delta: &mut Delta,
        fee_info: Vec<FeeInfo>,
        recipient: FeeAccount,
    ) -> Result<(), FeeError> {
        for fee_info in fee_info {
            self.charge_fee(fee_info, recipient)?;
            delta.fees_delta.extend([fee_info.account, recipient]);
        }
        Ok(())
    }

    pub fn distribute_rewards(
        &mut self,
        delta: &mut Delta,
        validator: Validator<BLSPubKey>,
    ) -> anyhow::Result<()> {
        let reward_state = apply_rewards(self.reward_merkle_tree.clone(), validator.clone())?;
        self.reward_merkle_tree = reward_state;

        // Update delta rewards
        delta.rewards_delta.insert(RewardAccount(validator.account));
        delta
            .rewards_delta
            .extend(validator.delegators.keys().map(|d| RewardAccount(*d)));

        Ok(())
    }

    /// Charge a fee to an account, transferring the funds to the fee recipient account.
    pub fn charge_fee(&mut self, fee_info: FeeInfo, recipient: FeeAccount) -> Result<(), FeeError> {
        if fee_info.amount == 0.into() {
            return Ok(());
        }

        let fee_state = self.fee_merkle_tree.clone();

        // Deduct the fee from the paying account.
        let FeeInfo { account, amount } = fee_info;
        let mut err = None;
        let fee_state = fee_state.persistent_update_with(account, |balance| {
            let balance = balance.copied();
            let Some(updated) = balance.unwrap_or_default().checked_sub(&amount) else {
                // Return an error without updating the account.
                err = Some(FeeError::InsufficientFunds { balance, amount });
                return balance;
            };
            if updated == FeeAmount::default() {
                // Delete the account from the tree if its balance ended up at 0; this saves some
                // space since the account is no longer carrying any information.
                None
            } else {
                // Otherwise store the updated balance.
                Some(updated)
            }
        })?;

        // Fail if there was an error during `persistent_update_with` (e.g. insufficient balance).
        if let Some(err) = err {
            return Err(err);
        }

        // If we successfully deducted the fee from the source account, increment the balance of the
        // recipient account.
        let fee_state = fee_state.persistent_update_with(recipient, |balance| {
            Some(balance.copied().unwrap_or_default() + amount)
        })?;

        // If the whole update was successful, update the original state.
        self.fee_merkle_tree = fee_state;
        Ok(())
    }
}
/// Block Proposal to be verified and applied.
#[derive(Debug)]
pub(crate) struct Proposal<'a> {
    header: &'a Header,
    block_size: u32,
}

impl<'a> Proposal<'a> {
    pub(crate) fn new(header: &'a Header, block_size: u32) -> Self {
        Self { header, block_size }
    }
    /// The L1 head block number in the proposal must be non-decreasing relative
    /// to the parent.
    fn validate_l1_head(&self, parent_l1_head: u64) -> Result<(), ProposalValidationError> {
        if self.header.l1_head() < parent_l1_head {
            return Err(ProposalValidationError::DecrementingL1Head);
        }
        Ok(())
    }
    /// The [`ChainConfig`] of proposal must be equal to the one stored in state.
    ///
    /// Equality is checked by comparing commitments.
    fn validate_chain_config(
        &self,
        expected_chain_config: &ChainConfig,
    ) -> Result<(), ProposalValidationError> {
        let proposed_chain_config = self.header.chain_config();
        if proposed_chain_config.commit() != expected_chain_config.commit() {
            return Err(ProposalValidationError::InvalidChainConfig {
                expected: Box::new(*expected_chain_config),
                proposal: Box::new(proposed_chain_config),
            });
        }
        Ok(())
    }

    /// The timestamp must be non-decreasing relative to parent.
    fn validate_timestamp_non_dec(
        &self,
        parent_timestamp: u64,
    ) -> Result<(), ProposalValidationError> {
        if self.header.timestamp() < parent_timestamp {
            return Err(ProposalValidationError::DecrementingTimestamp {
                proposal_timestamp: self.header.timestamp(),
                parent_timestamp,
            });
        }

        Ok(())
    }

    /// The timestamp must not drift too much from local system time.
    ///
    /// The tolerance is currently `12` seconds. This value may be moved to
    /// configuration in the future.
    fn validate_timestamp_drift(&self, system_time: u64) -> Result<(), ProposalValidationError> {
        // TODO 12 seconds of tolerance should be enough for reasonably
        // configured nodes, but we should make this configurable.
        let diff = self.header.timestamp().abs_diff(system_time);
        if diff > 12 {
            return Err(ProposalValidationError::InvalidTimestampDrift {
                proposal: self.header.timestamp(),
                system: system_time,
                diff,
            });
        }

        Ok(())
    }

    /// The proposed ['BlockMerkleTree'] must match the one in ['ValidatedState'].
    fn validate_block_merkle_tree(
        &self,
        block_merkle_tree_root: BlockMerkleCommitment,
    ) -> Result<(), ProposalValidationError> {
        if self.header.block_merkle_tree_root() != block_merkle_tree_root {
            return Err(ProposalValidationError::InvalidBlockRoot {
                expected_root: block_merkle_tree_root,
                proposal_root: self.header.block_merkle_tree_root(),
            });
        }

        Ok(())
    }
}
/// Type to hold cloned validated state and provide validation methods.
///
/// The [Self::validate] method must be called to validate the proposal.
#[derive(Debug)]
pub(crate) struct ValidatedTransition<'a> {
    state: ValidatedState,
    expected_chain_config: ChainConfig,
    parent: &'a Header,
    proposal: Proposal<'a>,
    view_number: u64,
}

impl<'a> ValidatedTransition<'a> {
    pub(crate) fn new(
        state: ValidatedState,
        parent: &'a Header,
        proposal: Proposal<'a>,
        view_number: u64,
    ) -> Self {
        let expected_chain_config = state
            .chain_config
            .resolve()
            .expect("Chain Config not found in validated state");
        Self {
            state,
            expected_chain_config,
            parent,
            proposal,
            view_number,
        }
    }

    /// Top level validation routine. Performs all validation units in
    /// the given order.
    /// ```ignore
    /// self.validate_timestamp()?;
    /// self.validate_builder_fee()?;
    /// self.validate_height()?;
    /// self.validate_chain_config()?;
    /// self.validate_block_size()?;
    /// self.validate_fee()?;
    /// self.validate_fee_merkle_tree()?;
    /// self.validate_block_merkle_tree()?;
    /// self.validate_l1_finalized()?;
    /// self.validate_l1_head()?;
    /// self.validate_namespace_table()?;
    /// ```
    pub(crate) fn validate(self) -> Result<Self, ProposalValidationError> {
        self.validate_timestamp()?;
        self.validate_builder_fee()?;
        self.validate_height()?;
        self.validate_chain_config()?;
        self.validate_block_size()?;
        self.validate_fee()?;
        self.validate_fee_merkle_tree()?;
        self.validate_block_merkle_tree()?;
        self.validate_reward_merkle_tree()?;
        self.validate_l1_finalized()?;
        self.validate_l1_head()?;
        self.validate_namespace_table()?;

        Ok(self)
    }

    /// The proposal [Header::l1_finalized] must be `Some` and non-decreasing relative to parent.
    fn validate_l1_finalized(&self) -> Result<(), ProposalValidationError> {
        let proposed_finalized = self.proposal.header.l1_finalized();
        let parent_finalized = self.parent.l1_finalized();

        if proposed_finalized < parent_finalized {
            // We are keeping the `Option` in the error b/c its the
            // cleanest way to represent all the different error
            // cases. The hash seems less useful and explodes the size
            // of the error, so we strip it out.
            return Err(ProposalValidationError::L1FinalizedDecrementing {
                parent: parent_finalized.map(|block| (block.number, block.timestamp.to::<u64>())),
                proposed: proposed_finalized
                    .map(|block| (block.number, block.timestamp.to::<u64>())),
            });
        }
        Ok(())
    }
    /// Wait for our view of the L1 chain to catch up to the proposal.
    ///
    /// The finalized [L1BlockInfo](super::L1BlockInfo) in the proposal must match the one fetched
    /// from L1.
    async fn wait_for_l1(self, l1_client: &L1Client) -> Result<Self, ProposalValidationError> {
        self.wait_for_l1_head(l1_client).await;
        self.wait_for_finalized_block(l1_client).await?;
        Ok(self)
    }

    /// Wait for our view of the latest L1 block number to catch up to the
    /// proposal.
    async fn wait_for_l1_head(&self, l1_client: &L1Client) {
        let _ = l1_client
            .wait_for_block(self.proposal.header.l1_head())
            .await;
    }
    /// Wait for our view of the finalized L1 block number to catch up to the
    /// proposal.
    async fn wait_for_finalized_block(
        &self,
        l1_client: &L1Client,
    ) -> Result<(), ProposalValidationError> {
        let proposed_finalized = self.proposal.header.l1_finalized();

        if let Some(proposed_finalized) = proposed_finalized {
            let finalized = l1_client
                .wait_for_finalized_block(proposed_finalized.number())
                .await;

            if finalized != proposed_finalized {
                return Err(ProposalValidationError::InvalidL1Finalized);
            }
        }

        Ok(())
    }

    /// Ensure that L1 Head on proposal is not decreasing.
    fn validate_l1_head(&self) -> Result<(), ProposalValidationError> {
        self.proposal.validate_l1_head(self.parent.l1_head())?;
        Ok(())
    }
    /// Validate basic numerical soundness and builder accounts by
    /// verifying signatures. Signatures are identified by index of fee `Vec`.
    fn validate_builder_fee(&self) -> Result<(), ProposalValidationError> {
        // TODO move logic from stand alone fn to here.
        if let Err(err) = validate_builder_fee(self.proposal.header, self.view_number) {
            return Err(ProposalValidationError::BuilderValidationError(err));
        }
        Ok(())
    }
    /// Validates proposals [`ChainConfig`] against expectation by comparing commitments.
    fn validate_chain_config(&self) -> Result<(), ProposalValidationError> {
        self.proposal
            .validate_chain_config(&self.expected_chain_config)?;
        Ok(())
    }
    /// Validate that proposal block size does not exceed configured
    /// `ChainConfig.max_block_size`.
    fn validate_block_size(&self) -> Result<(), ProposalValidationError> {
        let block_size = self.proposal.block_size as u64;
        if block_size > *self.expected_chain_config.max_block_size {
            return Err(ProposalValidationError::MaxBlockSizeExceeded {
                max_block_size: self.expected_chain_config.max_block_size,
                block_size: block_size.into(),
            });
        }
        Ok(())
    }
    /// Validate that [`FeeAmount`] (or sum of fees for Marketplace Version) is
    /// sufficient for block size.
    fn validate_fee(&self) -> Result<(), ProposalValidationError> {
        // TODO this should be updated to `base_fee * bundle_size` when we have
        // VID per bundle or namespace.
        let Some(amount) = self.proposal.header.fee_info().amount() else {
            return Err(ProposalValidationError::SomeFeeAmountOutOfRange);
        };

        if amount < self.expected_chain_config.base_fee * U256::from(self.proposal.block_size) {
            return Err(ProposalValidationError::InsufficientFee {
                max_block_size: self.expected_chain_config.max_block_size,
                base_fee: self.expected_chain_config.base_fee,
                proposed_fee: amount,
            });
        }
        Ok(())
    }
    /// Validate that proposal height is `parent_height + 1`.
    fn validate_height(&self) -> Result<(), ProposalValidationError> {
        let parent_header = self.parent;
        if self.proposal.header.height() != parent_header.height() + 1 {
            return Err(ProposalValidationError::InvalidHeight {
                parent_height: parent_header.height(),
                proposal_height: self.proposal.header.height(),
            });
        }
        Ok(())
    }
    /// Validate timestamp is not decreasing relative to parent and is
    /// within a given tolerance of system time. Tolerance is
    /// currently 12 seconds. This value may be moved to configuration
    /// in the future. Do this check first so we don't add unnecessary drift.
    fn validate_timestamp(&self) -> Result<(), ProposalValidationError> {
        self.proposal
            .validate_timestamp_non_dec(self.parent.timestamp())?;

        // Validate timestamp hasn't drifted too much from system time.
        let system_time: u64 = OffsetDateTime::now_utc().unix_timestamp() as u64;
        self.proposal.validate_timestamp_drift(system_time)?;

        Ok(())
    }
    /// Validate [`BlockMerkleTree`] by comparing proposed commitment
    /// that stored in [`ValidatedState`].
    fn validate_block_merkle_tree(&self) -> Result<(), ProposalValidationError> {
        let block_merkle_tree_root = self.state.block_merkle_tree.commitment();
        self.proposal
            .validate_block_merkle_tree(block_merkle_tree_root)?;

        Ok(())
    }

    /// Validate [`RewardMerkleTree`] by comparing proposed commitment
    /// against that stored in [`ValidatedState`].
    fn validate_reward_merkle_tree(&self) -> Result<(), ProposalValidationError> {
        let reward_merkle_tree_root = self.state.reward_merkle_tree.commitment();
        if self.proposal.header.reward_merkle_tree_root() != reward_merkle_tree_root {
            return Err(ProposalValidationError::InvalidRewardRoot {
                expected_root: reward_merkle_tree_root,
                proposal_root: self.proposal.header.reward_merkle_tree_root(),
            });
        }

        Ok(())
    }

    /// Validate [`FeeMerkleTree`] by comparing proposed commitment
    /// against that stored in [`ValidatedState`].
    fn validate_fee_merkle_tree(&self) -> Result<(), ProposalValidationError> {
        let fee_merkle_tree_root = self.state.fee_merkle_tree.commitment();
        if self.proposal.header.fee_merkle_tree_root() != fee_merkle_tree_root {
            return Err(ProposalValidationError::InvalidFeeRoot {
                expected_root: fee_merkle_tree_root,
                proposal_root: self.proposal.header.fee_merkle_tree_root(),
            });
        }

        Ok(())
    }
    /// Proxy to [`super::NsTable::validate()`].
    fn validate_namespace_table(&self) -> Result<(), ProposalValidationError> {
        self.proposal
            .header
            .ns_table()
            // Should be safe since `u32` will always fit in a `usize`.
            .validate(&PayloadByteLen(self.proposal.block_size as usize))
            .map_err(ProposalValidationError::from)
    }
}

#[cfg(any(test, feature = "testing"))]
impl ValidatedState {
    pub fn forget(&self) -> Self {
        Self {
            fee_merkle_tree: FeeMerkleTree::from_commitment(self.fee_merkle_tree.commitment()),
            block_merkle_tree: BlockMerkleTree::from_commitment(
                self.block_merkle_tree.commitment(),
            ),
            reward_merkle_tree: RewardMerkleTree::from_commitment(
                self.reward_merkle_tree.commitment(),
            ),
            chain_config: ResolvableChainConfig::from(self.chain_config.commit()),
        }
    }
}

impl From<NsTableValidationError> for ProposalValidationError {
    fn from(err: NsTableValidationError) -> Self {
        Self::InvalidNsTable(err)
    }
}

impl From<ProposalValidationError> for BlockError {
    fn from(err: ProposalValidationError) -> Self {
        tracing::error!("Invalid Block Header: {err:#}");
        BlockError::InvalidBlockHeader(err.to_string())
    }
}

impl From<MerkleTreeError> for FeeError {
    fn from(item: MerkleTreeError) -> Self {
        Self::MerkleTreeError(item)
    }
}

/// Validate builder accounts by verifying signatures. All fees are
/// verified against signature by index.
fn validate_builder_fee(
    proposed_header: &Header,
    view_number: u64,
) -> Result<(), BuilderValidationError> {
    let version = proposed_header.version();

    // TODO since we are iterating, should we include account/amount in errors?
    for (fee_info, signature) in proposed_header
        .fee_info()
        .iter()
        .zip(proposed_header.builder_signature())
    {
        // check that `amount` fits in a u64
        fee_info
            .amount()
            .as_u64()
            .ok_or(BuilderValidationError::FeeAmountOutOfRange(fee_info.amount))?;

        // Verify signatures.

        // TODO Marketplace signatures are placeholders for now. In
        // finished Marketplace signatures will cover the full
        // transaction.
        if version >= MarketplaceVersion::VERSION {
            tracing::debug!("Validating (Marketplace) sequencing fee signature.");
            fee_info
                .account()
                .validate_sequencing_fee_signature_marketplace(
                    &signature,
                    fee_info.amount().as_u64().unwrap(),
                    view_number,
                )
                .then_some(())
                .ok_or(BuilderValidationError::InvalidBuilderSignature)?;
        } else if !fee_info.account().validate_fee_signature(
            &signature,
            fee_info.amount().as_u64().unwrap(),
            proposed_header.metadata(),
        ) && !fee_info
            .account()
            .validate_fee_signature_with_vid_commitment(
                &signature,
                fee_info.amount().as_u64().unwrap(),
                proposed_header.metadata(),
                &proposed_header.payload_commitment(),
            )
        {
            return Err(BuilderValidationError::InvalidBuilderSignature);
        }
    }

    Ok(())
}

impl ValidatedState {
    /// Updates state with [`Header`] proposal.
    ///   * Clones and updates [`ValidatedState`] (avoiding mutation).
    ///   * Resolves [`ChainConfig`].
    ///   * Performs catchup.
    ///   * Charges fees.
    pub async fn apply_header(
        &self,
        instance: &NodeState,
        peers: &impl StateCatchup,
        parent_leaf: &Leaf2,
        proposed_header: &Header,
        version: Version,
        view_number: ViewNumber,
    ) -> anyhow::Result<(Self, Delta)> {
        // Clone state to avoid mutation. Consumer can take update
        // through returned value.
        let mut validated_state = self.clone();
        validated_state.apply_upgrade(instance, version);

        // TODO double check there is not some possibility we are
        // validating proposal values against ChainConfig of the proposal.
        let chain_config = validated_state
            .get_chain_config(instance, peers, &proposed_header.chain_config())
            .await?;

        if Some(chain_config) != validated_state.chain_config.resolve() {
            validated_state.chain_config = chain_config.into();
        }

        let l1_deposits = get_l1_deposits(
            instance,
            proposed_header,
            parent_leaf,
            chain_config.fee_contract,
        )
        .await;

        // Find missing fee state entries. We will need to use the builder account which is paying a
        // fee and the recipient account which is receiving it, plus any counts receiving deposits
        // in this block.
        let missing_accounts = self.forgotten_accounts(
            [chain_config.fee_recipient]
                .into_iter()
                .chain(proposed_header.fee_info().accounts())
                .chain(l1_deposits.accounts()),
        );

        let parent_height = parent_leaf.height();
        let parent_view = parent_leaf.view_number();

        // Ensure merkle tree has frontier
        if self.need_to_fetch_blocks_mt_frontier() {
            tracing::info!(
                parent_height,
                ?parent_view,
                "fetching block frontier from peers"
            );
            peers
                .remember_blocks_merkle_tree(
                    instance,
                    parent_height,
                    parent_view,
                    &mut validated_state.block_merkle_tree,
                )
                .await?;
        }

        // Fetch missing fee state entries
        if !missing_accounts.is_empty() {
            tracing::info!(
                parent_height,
                ?parent_view,
                ?missing_accounts,
                "fetching missing accounts from peers"
            );

            let missing_account_proofs = peers
                .fetch_accounts(
                    instance,
                    parent_height,
                    parent_view,
                    validated_state.fee_merkle_tree.commitment(),
                    missing_accounts,
                )
                .await?;

            // Remember the fee state entries
            for proof in missing_account_proofs.iter() {
                proof
                    .remember(&mut validated_state.fee_merkle_tree)
                    .expect("proof previously verified");
            }
        }

        let mut delta = Delta::default();
        validated_state.apply_proposal(&mut delta, parent_leaf, l1_deposits);

        validated_state.charge_fees(
            &mut delta,
            proposed_header.fee_info(),
            chain_config.fee_recipient,
        )?;

        if version >= EpochVersion::version()
            && !first_two_epochs(parent_leaf.height() + 1, instance).await?
        {
            let validator =
                find_validator_info(instance, &mut validated_state, parent_leaf, view_number)
                    .await?;

            // apply rewards
            validated_state
                .distribute_rewards(&mut delta, validator)
                .context("failed to distribute rewards")?;
        }

        Ok((validated_state, delta))
    }

    /// Updates the `ValidatedState` if a protocol upgrade has occurred.
    pub(crate) fn apply_upgrade(&mut self, instance: &NodeState, version: Version) {
        // Check for protocol upgrade based on sequencer version
        if version <= instance.current_version {
            return;
        }

        let Some(upgrade) = instance.upgrades.get(&version) else {
            return;
        };

        let cf = match upgrade.upgrade_type {
            UpgradeType::Fee { chain_config } => chain_config,
            UpgradeType::Marketplace { chain_config } => chain_config,
            UpgradeType::Epoch { chain_config } => chain_config,
        };

        self.chain_config = cf.into();
    }

    /// Retrieves the `ChainConfig`.
    ///
    ///  Returns the `NodeState` `ChainConfig` if the `ValidatedState` `ChainConfig` commitment matches the `NodeState` `ChainConfig`` commitment.
    ///  If the commitments do not match, it returns the `ChainConfig` available in either `ValidatedState` or proposed header.
    ///  If neither has the `ChainConfig`, it fetches the config from the peers.
    ///
    /// Returns an error if it fails to fetch the `ChainConfig` from the peers.
    pub(crate) async fn get_chain_config(
        &self,
        instance: &NodeState,
        peers: &impl StateCatchup,
        header_cf: &ResolvableChainConfig,
    ) -> anyhow::Result<ChainConfig> {
        let state_cf = self.chain_config;

        if state_cf.commit() == instance.chain_config.commit() {
            return Ok(instance.chain_config);
        }

        let cf = match (state_cf.resolve(), header_cf.resolve()) {
            (Some(cf), _) => cf,
            (_, Some(cf)) if cf.commit() == state_cf.commit() => cf,
            (_, Some(_)) | (None, None) => peers.fetch_chain_config(state_cf.commit()).await?,
        };

        Ok(cf)
    }
}

fn _apply_full_transactions(
    validated_state: &mut ValidatedState,
    full_network_txs: Vec<FullNetworkTx>,
) -> Result<(), ExecutionError> {
    full_network_txs
        .iter()
        .try_for_each(|tx| tx.execute(validated_state))
}

pub async fn get_l1_deposits(
    instance: &NodeState,
    header: &Header,
    parent_leaf: &Leaf2,
    fee_contract_address: Option<Address>,
) -> Vec<FeeInfo> {
    if let (Some(addr), Some(block_info)) = (fee_contract_address, header.l1_finalized()) {
        instance
            .l1_client
            .get_finalized_deposits(
                addr,
                parent_leaf
                    .block_header()
                    .l1_finalized()
                    .map(|block_info| block_info.number),
                block_info.number,
            )
            .await
    } else {
        vec![]
    }
}

impl HotShotState<SeqTypes> for ValidatedState {
    type Error = BlockError;
    type Instance = NodeState;

    type Time = ViewNumber;

    type Delta = Delta;
    fn on_commit(&self) {}
    /// Validate parent against known values (from state) and validate
    /// proposal descends from parent. Returns updated `ValidatedState`.
    #[tracing::instrument(
        skip_all,
        fields(
            node_id = instance.node_id,
            view = ?parent_leaf.view_number(),
            height = parent_leaf.height(),
        ),
    )]
    async fn validate_and_apply_header(
        &self,
        instance: &Self::Instance,
        parent_leaf: &Leaf2,
        proposed_header: &Header,
        payload_byte_len: u32,
        version: Version,
        view_number: u64,
    ) -> Result<(Self, Self::Delta), Self::Error> {
        let (validated_state, delta) = self
            // TODO We can add this logic to `ValidatedTransition` or do something similar to that here.
            .apply_header(
                instance,
                &instance.state_catchup,
                parent_leaf,
                proposed_header,
                version,
                ViewNumber::new(view_number),
            )
            .await
            .map_err(|e| BlockError::FailedHeaderApply(e.to_string()))?;

        // Validate the proposal.
        let validated_state = ValidatedTransition::new(
            validated_state,
            parent_leaf.block_header(),
            Proposal::new(proposed_header, payload_byte_len),
            view_number,
        )
        .validate()?
        .wait_for_l1(&instance.l1_client)
        .await?
        .state;

        // log successful progress about once in 10 - 20 seconds,
        // TODO: we may want to make this configurable
        if parent_leaf.view_number().u64() % 10 == 0 {
            tracing::info!("validated and applied new header");
        }
        Ok((validated_state, delta))
    }
    /// Construct the state with the given block header.
    ///
    /// This can also be used to rebuild the state for catchup.
    fn from_header(block_header: &Header) -> Self {
        let fee_merkle_tree = if block_header.fee_merkle_tree_root().size() == 0 {
            // If the commitment tells us that the tree is supposed to be empty, it is convenient to
            // just create an empty tree, rather than a commitment-only tree.
            FeeMerkleTree::new(FEE_MERKLE_TREE_HEIGHT)
        } else {
            FeeMerkleTree::from_commitment(block_header.fee_merkle_tree_root())
        };
        let block_merkle_tree = if block_header.block_merkle_tree_root().size() == 0 {
            // If the commitment tells us that the tree is supposed to be empty, it is convenient to
            // just create an empty tree, rather than a commitment-only tree.
            BlockMerkleTree::new(BLOCK_MERKLE_TREE_HEIGHT)
        } else {
            BlockMerkleTree::from_commitment(block_header.block_merkle_tree_root())
        };

        let reward_merkle_tree = if block_header.reward_merkle_tree_root().size() == 0 {
            RewardMerkleTree::new(REWARD_MERKLE_TREE_HEIGHT)
        } else {
            RewardMerkleTree::from_commitment(block_header.reward_merkle_tree_root())
        };

        Self {
            fee_merkle_tree,
            block_merkle_tree,
            reward_merkle_tree,
            chain_config: block_header.chain_config(),
        }
    }
    /// Construct a genesis validated state.
    #[must_use]
    fn genesis(instance: &Self::Instance) -> (Self, Self::Delta) {
        (instance.genesis_state.clone(), Delta::default())
    }
}

// Required for TestableState
#[cfg(any(test, feature = "testing"))]
impl std::fmt::Display for ValidatedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:#?}")
    }
}

#[cfg(any(test, feature = "testing"))]
impl hotshot_types::traits::states::TestableState<SeqTypes> for ValidatedState {
    fn create_random_transaction(
        _state: Option<&Self>,
        rng: &mut dyn rand::RngCore,
        _padding: u64,
    ) -> crate::Transaction {
        crate::Transaction::random(rng)
    }
}

impl MerklizedState<SeqTypes, { Self::ARITY }> for BlockMerkleTree {
    type Key = Self::Index;
    type Entry = Commitment<Header>;
    type T = Sha3Node;
    type Commit = Self::Commitment;
    type Digest = Sha3Digest;

    fn state_type() -> &'static str {
        "block_merkle_tree"
    }

    fn header_state_commitment_field() -> &'static str {
        "block_merkle_tree_root"
    }

    fn tree_height() -> usize {
        BLOCK_MERKLE_TREE_HEIGHT
    }

    fn insert_path(
        &mut self,
        key: Self::Key,
        proof: &MerkleProof<Self::Entry, Self::Key, Self::T, { Self::ARITY }>,
    ) -> anyhow::Result<()> {
        let Some(elem) = proof.elem() else {
            bail!("BlockMerkleTree does not support non-membership proofs");
        };
        self.remember(key, elem, proof)?;
        Ok(())
    }
}

impl MerklizedState<SeqTypes, { Self::ARITY }> for FeeMerkleTree {
    type Key = Self::Index;
    type Entry = Self::Element;
    type T = Sha3Node;
    type Commit = Self::Commitment;
    type Digest = Sha3Digest;

    fn state_type() -> &'static str {
        "fee_merkle_tree"
    }

    fn header_state_commitment_field() -> &'static str {
        "fee_merkle_tree_root"
    }

    fn tree_height() -> usize {
        FEE_MERKLE_TREE_HEIGHT
    }

    fn insert_path(
        &mut self,
        key: Self::Key,
        proof: &MerkleProof<Self::Entry, Self::Key, Self::T, { Self::ARITY }>,
    ) -> anyhow::Result<()> {
        match proof.elem() {
            Some(elem) => self.remember(key, elem, proof)?,
            None => self.non_membership_remember(key, proof)?,
        }
        Ok(())
    }
}

impl MerklizedState<SeqTypes, { Self::ARITY }> for RewardMerkleTree {
    type Key = Self::Index;
    type Entry = Self::Element;
    type T = Sha3Node;
    type Commit = Self::Commitment;
    type Digest = Sha3Digest;

    fn state_type() -> &'static str {
        "reward_merkle_tree"
    }

    fn header_state_commitment_field() -> &'static str {
        "reward_merkle_tree_root"
    }

    fn tree_height() -> usize {
        REWARD_MERKLE_TREE_HEIGHT
    }

    fn insert_path(
        &mut self,
        key: Self::Key,
        proof: &MerkleProof<Self::Entry, Self::Key, Self::T, { Self::ARITY }>,
    ) -> anyhow::Result<()> {
        match proof.elem() {
            Some(elem) => self.remember(key, elem, proof)?,
            None => self.non_membership_remember(key, proof)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use hotshot::{helpers::initialize_logging, traits::BlockPayload};
    use hotshot_query_service::{testing::mocks::MockVersions, Resolvable};
    use hotshot_types::{
        data::vid_commitment,
        traits::{node_implementation::Versions, signature_key::BuilderSignatureKey, EncodeBytes},
    };
    use sequencer_utils::ser::FromStringOrInteger;
    use tracing::debug;
    use vbs::version::StaticVersionType;

    use super::*;
    use crate::{
        eth_signature_key::{BuilderSignature, EthKeyPair},
        v0_1, v0_2, v0_3,
        v0_99::{self, BidTx},
        BlockSize, FeeAccountProof, FeeMerkleProof, Leaf, Payload, Transaction,
    };

    impl Transaction {
        async fn into_mock_header(self) -> (Header, u32) {
            let instance = NodeState::mock_v2();
            let (payload, metadata) =
                Payload::from_transactions([self], &instance.genesis_state, &instance)
                    .await
                    .unwrap();

            let builder_commitment = payload.builder_commitment(&metadata);
            let payload_bytes = payload.encode();

            let payload_commitment = vid_commitment::<MockVersions>(
                &payload_bytes,
                &metadata.encode(),
                1,
                <MockVersions as Versions>::Base::VERSION,
            );

            let header =
                Header::genesis(&instance, payload_commitment, builder_commitment, metadata);

            let header = header.sign();

            (header, payload.byte_len().0 as u32)
        }
    }
    impl Header {
        /// Build a new header from parent.
        fn next(self) -> Self {
            match self {
                Header::V1(_) => panic!("You called `Header.next()` on unimplemented version (v1)"),
                Header::V2(parent) => Header::V2(v0_2::Header {
                    height: parent.height + 1,
                    timestamp: OffsetDateTime::now_utc().unix_timestamp() as u64,
                    ..parent.clone()
                }),
                Header::V3(parent) => Header::V3(v0_3::Header {
                    height: parent.height + 1,
                    timestamp: OffsetDateTime::now_utc().unix_timestamp() as u64,
                    ..parent.clone()
                }),
                Header::V99(_) => {
                    panic!("You called `Header.next()` on unimplemented version (v3)")
                },
            }
        }
        /// Replaces builder signature w/ invalid one.
        fn sign(&self) -> Self {
            let key_pair = EthKeyPair::random();
            let fee_info = FeeInfo::new(key_pair.fee_account(), 1);

            let sig = FeeAccount::sign_fee(
                &key_pair,
                fee_info.amount().as_u64().unwrap(),
                self.metadata(),
            )
            .unwrap();

            match self {
                Header::V1(_) => panic!("You called `Header.sign()` on unimplemented version (v1)"),
                Header::V2(header) => Header::V2(v0_2::Header {
                    fee_info,
                    builder_signature: Some(sig),
                    ..header.clone()
                }),
                Header::V3(header) => Header::V3(v0_3::Header {
                    fee_info,
                    builder_signature: Some(sig),
                    ..header.clone()
                }),
                Header::V99(_) => {
                    panic!("You called `Header.sign()` on unimplemented version (v3)")
                },
            }
        }

        /// Replaces builder signature w/ invalid one.
        fn invalid_builder_signature(&self) -> Self {
            let key_pair = EthKeyPair::random();
            let key_pair2 = EthKeyPair::random();
            let fee_info = FeeInfo::new(key_pair.fee_account(), 1);

            let sig = FeeAccount::sign_fee(
                &key_pair2,
                fee_info.amount().as_u64().unwrap(),
                self.metadata(),
            )
            .unwrap();

            match self {
                Header::V1(_) => panic!(
                    "You called `Header.invalid_builder_signature()` on unimplemented version (v1)"
                ),
                Header::V2(parent) => Header::V2(v0_2::Header {
                    fee_info,
                    builder_signature: Some(sig),
                    ..parent.clone()
                }),
                Header::V3(parent) => Header::V3(v0_3::Header {
                    fee_info,
                    builder_signature: Some(sig),
                    ..parent.clone()
                }),
                Header::V99(_) => panic!(
                    "You called `Header.invalid_builder_signature()` on unimplemented version (v3)"
                ),
            }
        }
    }

    impl<'a> ValidatedTransition<'a> {
        fn mock(instance: NodeState, parent: &'a Header, proposal: Proposal<'a>) -> Self {
            let expected_chain_config = instance.chain_config;

            Self {
                state: instance.genesis_state,
                expected_chain_config,
                parent,
                proposal,
                view_number: 1,
            }
        }
    }

    pub fn mock_full_network_txs(key: Option<EthKeyPair>) -> Vec<FullNetworkTx> {
        // if no key is supplied, use `test_key_pair`. Since default `BidTxBody` is
        // signed with `test_key_pair`, it will verify successfully
        let key = key.unwrap_or_else(FeeAccount::test_key_pair);
        vec![FullNetworkTx::Bid(BidTx::mock(key))]
    }

    #[test]
    #[ignore]
    // TODO Currently we have some mismatch causing tests using
    // `Leaf::genesis` to generate a a Header to
    // fail. `NodeState::mock` is setting version to `v1` resulting in
    // an empty `bid_recipient` field on chain_config. We need a way to
    // pass in desired version, or so some other change before this can be enabled.
    fn test_apply_full_tx() {
        let mut state = ValidatedState::default();
        let txs = mock_full_network_txs(None);
        // Default key can be verified b/c it is the same that signs the mock tx
        _apply_full_transactions(&mut state, txs).unwrap();

        // Tx will be invalid if it is signed by a different key than
        // set in `account` field.
        let key = FeeAccount::generated_from_seed_indexed([1; 32], 0).1;
        let invalid = mock_full_network_txs(Some(key));
        let err = _apply_full_transactions(&mut state, invalid).unwrap_err();
        assert_eq!(ExecutionError::InvalidSignature, err);
    }

    #[test]
    fn test_fee_proofs() {
        initialize_logging();

        let mut tree = ValidatedState::default().fee_merkle_tree;
        let account1 = Address::random();
        let account2 = Address::default();
        tracing::info!(%account1, %account2);

        let balance1 = U256::from(100);
        tree.update(FeeAccount(account1), FeeAmount(balance1))
            .unwrap();

        // Membership proof.
        let (proof1, balance) = FeeAccountProof::prove(&tree, account1).unwrap();
        tracing::info!(?proof1, %balance);
        assert_eq!(balance, balance1);
        assert!(matches!(proof1.proof, FeeMerkleProof::Presence(_)));
        assert_eq!(proof1.verify(&tree.commitment()).unwrap(), balance1);

        // Non-membership proof.
        let (proof2, balance) = FeeAccountProof::prove(&tree, account2).unwrap();
        tracing::info!(?proof2, %balance);
        assert_eq!(balance, U256::ZERO);
        assert!(matches!(proof2.proof, FeeMerkleProof::Absence(_)));
        assert_eq!(proof2.verify(&tree.commitment()).unwrap(), U256::ZERO);

        // Test forget/remember. We cannot generate proofs in a completely sparse tree:
        let mut tree = FeeMerkleTree::from_commitment(tree.commitment());
        assert!(FeeAccountProof::prove(&tree, account1).is_none());
        assert!(FeeAccountProof::prove(&tree, account2).is_none());
        // After remembering the proofs, we can generate proofs again:
        proof1.remember(&mut tree).unwrap();
        proof2.remember(&mut tree).unwrap();
        FeeAccountProof::prove(&tree, account1).unwrap();
        FeeAccountProof::prove(&tree, account2).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_l1_head() {
        initialize_logging();

        // Setup.
        let tx = Transaction::of_size(10);
        let (header, block_size) = tx.into_mock_header().await;

        // Success Case
        let proposal = Proposal::new(&header, block_size);
        // Note we are using the same header for parent and proposal,
        // this may be OK depending on what we are testing.
        ValidatedTransition::mock(NodeState::mock_v2(), &header, proposal)
            .validate_l1_head()
            .unwrap();

        // Error Case
        let proposal = Proposal::new(&header, block_size);
        let err = proposal.validate_l1_head(u64::MAX).unwrap_err();
        assert_eq!(ProposalValidationError::DecrementingL1Head, err);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_builder_fee() {
        initialize_logging();

        // Setup.
        let instance = NodeState::mock();
        let tx = Transaction::of_size(20);
        let (header, block_size) = tx.into_mock_header().await;

        // Success Case
        let proposal = Proposal::new(&header, block_size);
        ValidatedTransition::mock(instance.clone(), &header, proposal)
            .validate_builder_fee()
            .unwrap();

        // Error Case
        let header = header.invalid_builder_signature();
        let proposal = Proposal::new(&header, block_size);
        let err = ValidatedTransition::mock(instance, &header, proposal)
            .validate_builder_fee()
            .unwrap_err();

        tracing::info!(%err, "task failed successfully");
        assert_eq!(
            ProposalValidationError::BuilderValidationError(
                BuilderValidationError::InvalidBuilderSignature
            ),
            err
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_chain_config() {
        initialize_logging();

        // Setup.
        let instance = NodeState::mock();
        let tx = Transaction::of_size(20);
        let (header, block_size) = tx.into_mock_header().await;

        // Success Case
        let proposal = Proposal::new(&header, block_size);
        ValidatedTransition::mock(instance.clone(), &header, proposal)
            .validate_chain_config()
            .unwrap();

        // Error Case
        let proposal = Proposal::new(&header, block_size);
        let expected_chain_config = ChainConfig {
            max_block_size: BlockSize(3333),
            ..instance.chain_config
        };
        let err = proposal
            .validate_chain_config(&expected_chain_config)
            .unwrap_err();

        tracing::info!(%err, "task failed successfully");

        assert_eq!(
            ProposalValidationError::InvalidChainConfig {
                expected: Box::new(expected_chain_config),
                proposal: Box::new(header.chain_config())
            },
            err
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_max_block_size() {
        initialize_logging();
        const MAX_BLOCK_SIZE: usize = 10;

        // Setup.
        let state = ValidatedState::default();
        let expected_chain_config = ChainConfig {
            max_block_size: BlockSize::from_integer(MAX_BLOCK_SIZE as u64).unwrap(),
            ..state.chain_config.resolve().unwrap()
        };
        let instance = NodeState::mock().with_chain_config(expected_chain_config);
        let tx = Transaction::of_size(20);
        let (header, block_size) = tx.into_mock_header().await;

        // Error Case
        let proposal = Proposal::new(&header, block_size);
        let err = ValidatedTransition::mock(instance.clone(), &header, proposal)
            .validate_block_size()
            .unwrap_err();

        tracing::info!(%err, "task failed successfully");
        assert_eq!(
            ProposalValidationError::MaxBlockSizeExceeded {
                max_block_size: instance.chain_config.max_block_size,
                block_size: BlockSize::from_integer(block_size as u64).unwrap()
            },
            err
        );

        // Success Case
        let proposal = Proposal::new(&header, 1);
        ValidatedTransition::mock(instance, &header, proposal)
            .validate_block_size()
            .unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_base_fee() {
        initialize_logging();
        // Setup
        let tx = Transaction::of_size(20);
        let (header, block_size) = tx.into_mock_header().await;
        let state = ValidatedState::default();
        let instance = NodeState::mock_v2().with_chain_config(ChainConfig {
            base_fee: 1000.into(), // High expected base fee
            ..state.chain_config.resolve().unwrap()
        });

        let proposal = Proposal::new(&header, block_size);
        let err = ValidatedTransition::mock(instance.clone(), &header, proposal)
            .validate_fee()
            .unwrap_err();

        // Validation fails because the genesis fee (0) is too low.
        tracing::info!(%err, "task failed successfully");
        assert_eq!(
            ProposalValidationError::InsufficientFee {
                max_block_size: instance.chain_config.max_block_size,
                base_fee: instance.chain_config.base_fee,
                proposed_fee: header.fee_info().amount().unwrap()
            },
            err
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_height() {
        initialize_logging();
        // Setup
        let instance = NodeState::mock_v2();
        let tx = Transaction::of_size(10);
        let (parent, block_size) = tx.into_mock_header().await;

        let proposal = Proposal::new(&parent, block_size);
        let err = ValidatedTransition::mock(instance.clone(), &parent, proposal)
            .validate_height()
            .unwrap_err();

        // Validation fails because the proposal is using same default.
        tracing::info!(%err, "task failed successfully");
        assert_eq!(
            ProposalValidationError::InvalidHeight {
                parent_height: parent.height(),
                proposal_height: parent.height()
            },
            err
        );

        // Success case. Increment height on proposal.
        let mut header = parent.clone();
        *header.height_mut() += 1;
        let proposal = Proposal::new(&header, block_size);

        ValidatedTransition::mock(instance, &parent, proposal)
            .validate_height()
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_timestamp_non_dec() {
        initialize_logging();
        let tx = Transaction::of_size(10);
        let (parent, block_size) = tx.into_mock_header().await;

        // Error case
        let proposal = Proposal::new(&parent, block_size);
        let proposal_timestamp = proposal.header.timestamp();
        let err = proposal.validate_timestamp_non_dec(u64::MAX).unwrap_err();

        // Validation fails because the proposal is using same default.
        tracing::info!(%err, "task failed successfully");
        assert_eq!(
            ProposalValidationError::DecrementingTimestamp {
                proposal_timestamp,
                parent_timestamp: u64::MAX,
            },
            err
        );

        // Success case (genesis timestamp is `0`).
        let proposal = Proposal::new(&parent, block_size);
        proposal.validate_timestamp_non_dec(0).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_timestamp_drift() {
        initialize_logging();
        // Setup
        let instance = NodeState::mock_v2();
        let (parent, block_size) = Transaction::of_size(10).into_mock_header().await;

        let header = parent.clone();
        // Error case.
        let proposal = Proposal::new(&header, block_size);
        let proposal_timestamp = header.timestamp();

        let mock_time = OffsetDateTime::now_utc().unix_timestamp() as u64;
        // TODO
        let err = ValidatedTransition::mock(instance.clone(), &parent, proposal)
            .validate_timestamp()
            .unwrap_err();

        tracing::info!(%err, "task failed successfully");
        assert_eq!(
            ProposalValidationError::InvalidTimestampDrift {
                proposal: proposal_timestamp,
                system: mock_time,
                diff: mock_time
            },
            err
        );

        let mock_time: u64 = OffsetDateTime::now_utc().unix_timestamp() as u64;
        let mut header = parent.clone();
        *header.timestamp_mut() = mock_time - 13;
        let proposal = Proposal::new(&header, block_size);

        let err = proposal.validate_timestamp_drift(mock_time).unwrap_err();
        tracing::info!(%err, "task failed successfully");
        assert_eq!(
            ProposalValidationError::InvalidTimestampDrift {
                proposal: mock_time - 13,
                system: mock_time,
                diff: 13
            },
            err
        );

        // Success cases.
        let mut header = parent.clone();
        *header.timestamp_mut() = mock_time;
        let proposal = Proposal::new(&header, block_size);
        proposal.validate_timestamp_drift(mock_time).unwrap();

        *header.timestamp_mut() = mock_time - 11;
        let proposal = Proposal::new(&header, block_size);
        proposal.validate_timestamp_drift(mock_time).unwrap();

        *header.timestamp_mut() = mock_time - 12;
        let proposal = Proposal::new(&header, block_size);
        proposal.validate_timestamp_drift(mock_time).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_fee_root() {
        initialize_logging();
        // Setup
        let instance = NodeState::mock_v2();
        let (header, block_size) = Transaction::of_size(10).into_mock_header().await;

        // Success case.
        let proposal = Proposal::new(&header, block_size);
        ValidatedTransition::mock(instance.clone(), &header, proposal)
            .validate_fee_merkle_tree()
            .unwrap();

        // Error case.
        let proposal = Proposal::new(&header, block_size);

        let mut fee_merkle_tree = instance.genesis_state.fee_merkle_tree;
        fee_merkle_tree
            .update_with(FeeAccount::default(), |_| Some(100.into()))
            .unwrap();

        let err = proposal
            .validate_block_merkle_tree(fee_merkle_tree.commitment())
            .unwrap_err();

        tracing::info!(%err, "task failed successfully");
        assert_eq!(
            ProposalValidationError::InvalidBlockRoot {
                expected_root: fee_merkle_tree.commitment(),
                proposal_root: header.block_merkle_tree_root(),
            },
            err
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_block_root() {
        initialize_logging();
        // Setup.
        let instance = NodeState::mock_v2();
        let (header, block_size) = Transaction::of_size(10).into_mock_header().await;

        // Success case.
        let proposal = Proposal::new(&header, block_size);
        ValidatedTransition::mock(instance.clone(), &header, proposal)
            .validate_block_merkle_tree()
            .unwrap();

        // Error case.
        let proposal = Proposal::new(&header, block_size);
        let mut block_merkle_tree = instance.genesis_state.block_merkle_tree;
        block_merkle_tree.push(header.commitment()).unwrap();
        block_merkle_tree
            .push(header.clone().next().commitment())
            .unwrap();

        let err = proposal
            .validate_block_merkle_tree(block_merkle_tree.commitment())
            .unwrap_err();

        tracing::info!(%err, "task failed successfully");
        assert_eq!(
            ProposalValidationError::InvalidBlockRoot {
                expected_root: block_merkle_tree.commitment(),
                proposal_root: proposal.header.block_merkle_tree_root(),
            },
            err
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validation_ns_table() {
        use NsTableValidationError::InvalidFinalOffset;

        initialize_logging();
        // Setup.
        let tx = Transaction::of_size(10);
        let (header, block_size) = tx.into_mock_header().await;

        // Success case.
        let proposal = Proposal::new(&header, block_size);
        ValidatedTransition::mock(NodeState::mock_v2(), &header, proposal)
            .validate_namespace_table()
            .unwrap();

        // Error case
        let proposal = Proposal::new(&header, 40);
        let err = ValidatedTransition::mock(NodeState::mock_v2(), &header, proposal)
            .validate_namespace_table()
            .unwrap_err();
        tracing::info!(%err, "task failed successfully");
        // TODO NsTable has other error variants, but these should be
        // tested in unit tests of `NsTable.validate()`.
        assert_eq!(
            ProposalValidationError::InvalidNsTable(InvalidFinalOffset),
            err
        );
    }

    #[test]
    fn test_charge_fee() {
        initialize_logging();
        let src = FeeAccount::generated_from_seed_indexed([0; 32], 0).0;
        let dst = FeeAccount::generated_from_seed_indexed([0; 32], 1).0;
        let amt = FeeAmount::from(1);

        let fee_info = FeeInfo::new(src, amt);

        let new_state = || {
            let mut state = ValidatedState::default();
            state.prefund_account(src, amt);
            state
        };

        tracing::info!("test successful fee");
        let mut state = new_state();
        state.charge_fee(fee_info, dst).unwrap();
        assert_eq!(state.balance(src), Some(0.into()));
        assert_eq!(state.balance(dst), Some(amt));

        tracing::info!("test insufficient balance");
        let err = state.charge_fee(fee_info, dst).unwrap_err();
        assert_eq!(state.balance(src), Some(0.into()));
        assert_eq!(state.balance(dst), Some(amt));
        assert_eq!(
            FeeError::InsufficientFunds {
                balance: None,
                amount: amt
            },
            err
        );

        tracing::info!("test src not in memory");
        let mut state = new_state();
        state.fee_merkle_tree.forget(src).expect_ok().unwrap();
        assert_eq!(
            FeeError::MerkleTreeError(MerkleTreeError::ForgottenLeaf),
            state.charge_fee(fee_info, dst).unwrap_err()
        );

        tracing::info!("test dst not in memory");
        let mut state = new_state();
        state.prefund_account(dst, amt);
        state.fee_merkle_tree.forget(dst).expect_ok().unwrap();
        assert_eq!(
            FeeError::MerkleTreeError(MerkleTreeError::ForgottenLeaf),
            state.charge_fee(fee_info, dst).unwrap_err()
        );
    }

    #[test]
    fn test_fee_amount_serde_json_as_decimal() {
        let amt = FeeAmount::from(123);
        let serialized = serde_json::to_string(&amt).unwrap();

        // The value is serialized as a decimal string.
        assert_eq!(serialized, "\"123\"");

        // Deserialization produces the original value
        let deserialized: FeeAmount = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, amt);
    }

    #[test]
    fn test_fee_amount_from_units() {
        for (unit, multiplier) in [
            ("wei", 1),
            ("gwei", 1_000_000_000),
            ("eth", 1_000_000_000_000_000_000),
        ] {
            let amt: FeeAmount = serde_json::from_str(&format!("\"1 {unit}\"")).unwrap();
            assert_eq!(amt, multiplier.into());
        }
    }

    #[test]
    fn test_fee_amount_serde_json_from_hex() {
        // For backwards compatibility, fee amounts can also be deserialized from a 0x-prefixed hex
        // string.
        let amt: FeeAmount = serde_json::from_str("\"0x123\"").unwrap();
        assert_eq!(amt, FeeAmount::from(0x123));
    }

    #[test]
    fn test_fee_amount_serde_json_from_number() {
        // For convenience, fee amounts can also be deserialized from a JSON number.
        let amt: FeeAmount = serde_json::from_str("123").unwrap();
        assert_eq!(amt, FeeAmount::from(123));
    }

    #[test]
    fn test_fee_amount_serde_bincode_unchanged() {
        // For non-human-readable formats, FeeAmount just serializes as the underlying U256.
        // note: for backward compat, it has to be the same as ethers' U256 instead of alloy's
        let n = ethers_core::types::U256::from(123);
        let amt = FeeAmount(U256::from(123));
        assert_eq!(
            bincode::serialize(&n).unwrap(),
            bincode::serialize(&amt).unwrap(),
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_builder_fee() {
        initialize_logging();
        let max_block_size = 10;

        let validated_state = ValidatedState::default();
        let instance_state = NodeState::mock().with_chain_config(ChainConfig {
            base_fee: 1000.into(), // High base fee
            max_block_size: max_block_size.into(),
            ..validated_state.chain_config.resolve().unwrap()
        });

        let parent: Leaf2 =
            Leaf::genesis::<MockVersions>(&instance_state.genesis_state, &instance_state)
                .await
                .into();
        let header = parent.block_header().clone();
        let metadata = parent.block_header().metadata();

        debug!("{:?}", header.version());

        let key_pair = EthKeyPair::random();
        let account = key_pair.fee_account();

        let data = header.fee_info()[0].amount().as_u64().unwrap();
        let sig = FeeAccount::sign_builder_message(&key_pair, &data.to_be_bytes()).unwrap();

        // ensure the signature is indeed valid
        account
            .validate_builder_signature(&sig, &data.to_be_bytes())
            .then_some(())
            .unwrap();

        // test v1 sig
        let sig = FeeAccount::sign_fee(&key_pair, data, metadata).unwrap();

        let header = match header {
            Header::V1(header) => Header::V1(v0_1::Header {
                builder_signature: Some(sig),
                fee_info: FeeInfo::new(account, data),
                ..header
            }),
            Header::V2(header) => Header::V2(v0_2::Header {
                builder_signature: Some(sig),
                fee_info: FeeInfo::new(account, data),
                ..header
            }),
            Header::V3(header) => Header::V3(v0_3::Header {
                builder_signature: Some(sig),
                fee_info: FeeInfo::new(account, data),
                ..header
            }),
            Header::V99(header) => Header::V99(v0_99::Header {
                builder_signature: vec![sig],
                fee_info: vec![FeeInfo::new(account, data)],
                ..header
            }),
        };

        validate_builder_fee(&header, *parent.view_number() + 1).unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_builder_fee_marketplace() {
        initialize_logging();
        let max_block_size = 10;

        let validated_state = ValidatedState::default();
        let instance_state = NodeState::mock_v99().with_chain_config(ChainConfig {
            base_fee: 1000.into(), // High base fee
            max_block_size: max_block_size.into(),
            ..validated_state.chain_config.resolve().unwrap()
        });

        let parent: Leaf2 =
            Leaf::genesis::<MockVersions>(&instance_state.genesis_state, &instance_state)
                .await
                .into();
        let header = parent.block_header().clone();

        debug!("{:?}", header.version());

        let key_pair = EthKeyPair::random();
        let account = key_pair.fee_account();

        let data = header.fee_info()[0].amount().as_u64().unwrap();

        // test v3 sig
        let sig =
            FeeAccount::sign_sequencing_fee_marketplace(&key_pair, data, *parent.view_number() + 1)
                .unwrap();
        // test dedicated marketplace validation function
        account
            .validate_sequencing_fee_signature_marketplace(&sig, data, *parent.view_number() + 1)
            .then_some(())
            .unwrap();

        let header = match header {
            Header::V1(header) => Header::V1(v0_1::Header {
                builder_signature: Some(sig),
                fee_info: FeeInfo::new(account, data),
                ..header
            }),
            Header::V2(header) => Header::V2(v0_2::Header {
                builder_signature: Some(sig),
                fee_info: FeeInfo::new(account, data),
                ..header
            }),
            Header::V3(header) => Header::V3(v0_3::Header {
                builder_signature: Some(sig),
                fee_info: FeeInfo::new(account, data),
                ..header
            }),
            Header::V99(header) => Header::V99(v0_99::Header {
                builder_signature: vec![sig],
                fee_info: vec![FeeInfo::new(account, data)],
                ..header
            }),
        };

        let sig: Vec<BuilderSignature> = header.builder_signature();
        let fee = header.fee_info()[0].amount().as_u64().unwrap();

        // assert expectations
        account
            .validate_sequencing_fee_signature_marketplace(&sig[0], fee, *parent.view_number() + 1)
            .then_some(())
            .unwrap();

        validate_builder_fee(&header, *parent.view_number() + 1).unwrap();
    }
}
