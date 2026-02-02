use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
    iter::once,
    str::FromStr,
};

use alloy::primitives::{
    utils::{parse_units, ParseUnits},
    Address, B256, U256,
};
use anyhow::{bail, ensure, Context};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
};
use hotshot::types::BLSPubKey;
use hotshot_contract_adapter::reward::RewardProofSiblings;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{election::Membership, node_implementation::ConsensusTime},
    utils::epoch_from_block_number,
};
use jf_merkle_tree_compat::{
    prelude::MerkleNode, ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme,
    LookupResult, MerkleTreeScheme, PersistentUniversalMerkleTreeScheme, ToTraversalPath,
    UniversalMerkleTreeScheme,
};
use num_traits::CheckedSub;
use sequencer_utils::{
    impl_serde_from_string_or_integer, impl_to_fixed_bytes, ser::FromStringOrInteger,
};
use tokio::task::JoinHandle;
use vbs::version::StaticVersionType;

use super::{
    v0_3::{RewardAmount, Validator, COMMISSION_BASIS_POINTS},
    v0_4::{
        RewardAccountProofV2, RewardAccountQueryDataV2, RewardAccountV2, RewardMerkleCommitmentV2,
        RewardMerkleProofV2, RewardMerkleTreeV2,
    },
    Leaf2, NodeState, ValidatedState,
};
use crate::{
    eth_signature_key::EthKeyPair,
    v0_3::{
        RewardAccountProofV1, RewardAccountV1, RewardMerkleCommitmentV1, RewardMerkleProofV1,
        RewardMerkleTreeV1,
    },
    v0_4::{Delta, REWARD_MERKLE_TREE_V2_ARITY, REWARD_MERKLE_TREE_V2_HEIGHT},
    v0_6::LeaderCounts,
    DrbAndHeaderUpgradeVersion, EpochRewardVersion, EpochVersion, FeeAccount, SeqTypes,
};

impl_serde_from_string_or_integer!(RewardAmount);
impl_to_fixed_bytes!(RewardAmount, U256);

impl From<u64> for RewardAmount {
    fn from(amt: u64) -> Self {
        Self(U256::from(amt))
    }
}

impl CheckedSub for RewardAmount {
    fn checked_sub(&self, v: &Self) -> Option<Self> {
        self.0.checked_sub(v.0).map(RewardAmount)
    }
}

impl FromStr for RewardAmount {
    type Err = <U256 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl FromStringOrInteger for RewardAmount {
    type Binary = U256;
    type Integer = u64;

    fn from_binary(b: Self::Binary) -> anyhow::Result<Self> {
        Ok(Self(b))
    }

    fn from_integer(i: Self::Integer) -> anyhow::Result<Self> {
        Ok(i.into())
    }

    fn from_string(s: String) -> anyhow::Result<Self> {
        // For backwards compatibility, we have an ad hoc parser for WEI amounts
        // represented as hex strings.
        if let Some(s) = s.strip_prefix("0x") {
            return Ok(Self(s.parse()?));
        }

        // Strip an optional non-numeric suffix, which will be interpreted as a unit.
        let (base, unit) = s
            .split_once(char::is_whitespace)
            .unwrap_or((s.as_str(), "wei"));
        match parse_units(base, unit)? {
            ParseUnits::U256(n) => Ok(Self(n)),
            ParseUnits::I256(_) => bail!("amount cannot be negative"),
        }
    }

    fn to_binary(&self) -> anyhow::Result<Self::Binary> {
        Ok(self.0)
    }

    fn to_string(&self) -> anyhow::Result<String> {
        Ok(format!("{self}"))
    }
}

impl RewardAmount {
    pub fn as_u64(&self) -> Option<u64> {
        if self.0 <= U256::from(u64::MAX) {
            Some(self.0.to::<u64>())
        } else {
            None
        }
    }
}

impl From<[u8; 20]> for RewardAccountV1 {
    fn from(bytes: [u8; 20]) -> Self {
        Self(Address::from(bytes))
    }
}

impl AsRef<[u8]> for RewardAccountV1 {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<const ARITY: usize> ToTraversalPath<ARITY> for RewardAccountV1 {
    fn to_traversal_path(&self, height: usize) -> Vec<usize> {
        self.0
            .as_slice()
            .iter()
            .take(height)
            .map(|i| *i as usize)
            .collect()
    }
}

impl RewardAccountV2 {
    /// Return inner `Address`
    pub fn address(&self) -> Address {
        self.0
    }
    /// Return byte slice representation of inner `Address` type
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
    /// Return array containing underlying bytes of inner `Address` type
    pub fn to_fixed_bytes(self) -> [u8; 20] {
        self.0.into_array()
    }
    pub fn test_key_pair() -> EthKeyPair {
        EthKeyPair::from_mnemonic(
            "test test test test test test test test test test test junk",
            0u32,
        )
        .unwrap()
    }
}

impl RewardAccountV1 {
    /// Return inner `Address`
    pub fn address(&self) -> Address {
        self.0
    }
    /// Return byte slice representation of inner `Address` type
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
    /// Return array containing underlying bytes of inner `Address` type
    pub fn to_fixed_bytes(self) -> [u8; 20] {
        self.0.into_array()
    }
    pub fn test_key_pair() -> EthKeyPair {
        EthKeyPair::from_mnemonic(
            "test test test test test test test test test test test junk",
            0u32,
        )
        .unwrap()
    }
}

impl FromStr for RewardAccountV2 {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl FromStr for RewardAccountV1 {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Valid for RewardAmount {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl Valid for RewardAccountV2 {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl Valid for RewardAccountV1 {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl CanonicalSerialize for RewardAmount {
    fn serialize_with_mode<W: std::io::prelude::Write>(
        &self,
        mut writer: W,
        _compress: Compress,
    ) -> Result<(), SerializationError> {
        Ok(writer.write_all(&self.to_fixed_bytes())?)
    }

    fn serialized_size(&self, _compress: Compress) -> usize {
        core::mem::size_of::<U256>()
    }
}
impl CanonicalDeserialize for RewardAmount {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        let mut bytes = [0u8; core::mem::size_of::<U256>()];
        reader.read_exact(&mut bytes)?;
        let value = U256::from_le_slice(&bytes);
        Ok(Self(value))
    }
}

impl CanonicalSerialize for RewardAccountV2 {
    fn serialize_with_mode<W: std::io::prelude::Write>(
        &self,
        mut writer: W,
        _compress: Compress,
    ) -> Result<(), SerializationError> {
        Ok(writer.write_all(self.0.as_slice())?)
    }

    fn serialized_size(&self, _compress: Compress) -> usize {
        core::mem::size_of::<Address>()
    }
}
impl CanonicalDeserialize for RewardAccountV2 {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        let mut bytes = [0u8; core::mem::size_of::<Address>()];
        reader.read_exact(&mut bytes)?;
        let value = Address::from_slice(&bytes);
        Ok(Self(value))
    }
}

impl CanonicalSerialize for RewardAccountV1 {
    fn serialize_with_mode<W: std::io::prelude::Write>(
        &self,
        mut writer: W,
        _compress: Compress,
    ) -> Result<(), SerializationError> {
        Ok(writer.write_all(self.0.as_slice())?)
    }

    fn serialized_size(&self, _compress: Compress) -> usize {
        core::mem::size_of::<Address>()
    }
}
impl CanonicalDeserialize for RewardAccountV1 {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        let mut bytes = [0u8; core::mem::size_of::<Address>()];
        reader.read_exact(&mut bytes)?;
        let value = Address::from_slice(&bytes);
        Ok(Self(value))
    }
}

impl From<[u8; 20]> for RewardAccountV2 {
    fn from(bytes: [u8; 20]) -> Self {
        Self(Address::from(bytes))
    }
}

impl AsRef<[u8]> for RewardAccountV2 {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl<const ARITY: usize> ToTraversalPath<ARITY> for RewardAccountV2 {
    fn to_traversal_path(&self, height: usize) -> Vec<usize> {
        let mut result = vec![0; height];

        // Convert 20-byte address to U256
        let mut value = U256::from_be_slice(self.0.as_slice());

        // Extract digits using modulo and division (LSB first)
        for item in result.iter_mut().take(height) {
            let digit = (value % U256::from(ARITY)).to::<usize>();
            *item = digit;
            value /= U256::from(ARITY);
        }

        result
    }
}

impl RewardAccountProofV2 {
    pub fn presence(
        pos: FeeAccount,
        proof: <RewardMerkleTreeV2 as MerkleTreeScheme>::MembershipProof,
    ) -> Self {
        Self {
            account: pos.into(),
            proof: RewardMerkleProofV2::Presence(proof),
        }
    }

    pub fn absence(
        pos: RewardAccountV2,
        proof: <RewardMerkleTreeV2 as UniversalMerkleTreeScheme>::NonMembershipProof,
    ) -> Self {
        Self {
            account: pos.into(),
            proof: RewardMerkleProofV2::Absence(proof),
        }
    }

    pub fn prove(tree: &RewardMerkleTreeV2, account: Address) -> Option<(Self, U256)> {
        match tree.universal_lookup(RewardAccountV2(account)) {
            LookupResult::Ok(balance, proof) => Some((
                Self {
                    account,
                    proof: RewardMerkleProofV2::Presence(proof),
                },
                balance.0,
            )),
            LookupResult::NotFound(proof) => Some((
                Self {
                    account,
                    proof: RewardMerkleProofV2::Absence(proof),
                },
                U256::ZERO,
            )),
            LookupResult::NotInMemory => None,
        }
    }

    pub fn verify(&self, comm: &RewardMerkleCommitmentV2) -> anyhow::Result<U256> {
        match &self.proof {
            RewardMerkleProofV2::Presence(proof) => {
                ensure!(
                    RewardMerkleTreeV2::verify(comm, RewardAccountV2(self.account), proof)?.is_ok(),
                    "invalid proof"
                );
                Ok(proof
                    .elem()
                    .context("presence proof is missing account balance")?
                    .0)
            },
            RewardMerkleProofV2::Absence(proof) => {
                let tree = RewardMerkleTreeV2::from_commitment(comm);
                ensure!(
                    RewardMerkleTreeV2::non_membership_verify(
                        tree.commitment(),
                        RewardAccountV2(self.account),
                        proof
                    )?,
                    "invalid proof"
                );
                Ok(U256::ZERO)
            },
        }
    }

    pub fn remember(&self, tree: &mut RewardMerkleTreeV2) -> anyhow::Result<()> {
        match &self.proof {
            RewardMerkleProofV2::Presence(proof) => {
                tree.remember(
                    RewardAccountV2(self.account),
                    proof
                        .elem()
                        .context("presence proof is missing account balance")?,
                    proof,
                )?;
                Ok(())
            },
            RewardMerkleProofV2::Absence(proof) => {
                tree.non_membership_remember(RewardAccountV2(self.account), proof)?;
                Ok(())
            },
        }
    }
}

impl TryInto<RewardProofSiblings> for RewardAccountProofV2 {
    type Error = anyhow::Error;

    /// Generate a Solidity-compatible proof for this account
    ///
    /// The proof is returned without leaf value. The caller is expected to
    /// obtain the leaf value from the jellyfish proof (Self).
    fn try_into(self) -> anyhow::Result<RewardProofSiblings> {
        // NOTE: rustfmt fails to format this file if the nesting is too deep.
        let proof = if let RewardMerkleProofV2::Presence(proof) = &self.proof {
            proof
        } else {
            bail!("only presence proofs supported")
        };

        let path = ToTraversalPath::<REWARD_MERKLE_TREE_V2_ARITY>::to_traversal_path(
            &RewardAccountV2(self.account),
            REWARD_MERKLE_TREE_V2_HEIGHT,
        );

        if path.len() != REWARD_MERKLE_TREE_V2_HEIGHT {
            bail!("Invalid proof: unexpected path length: {}", path.len());
        };

        let siblings: [B256; REWARD_MERKLE_TREE_V2_HEIGHT] = proof
            .proof
            .iter()
            .enumerate()
            .skip(1) // Skip the leaf node (first element)
            .filter_map(|(level_idx, node)| match node {
                MerkleNode::Branch { children, .. } => {
                    // Use the path to determine which sibling we need
                    let path_direction = path
                        .get(level_idx - 1)
                        .copied()
                        .expect("exists");
                    let sibling_idx = if path_direction == 0 { 1 } else { 0 };
                    if sibling_idx >= children.len() {
                        panic!(
                            "Invalid proof: index={sibling_idx} length={}",
                            children.len()
                        );
                    };

                    match children[sibling_idx].as_ref() {
                        MerkleNode::Empty => Some(B256::ZERO),
                        MerkleNode::Leaf { value, .. } => {
                            let bytes = value.as_ref();
                            Some(B256::from_slice(bytes))
                        }
                        MerkleNode::Branch { value, .. } => {
                            let bytes = value.as_ref();
                            Some(B256::from_slice(bytes))
                        }
                        MerkleNode::ForgettenSubtree { value } => {
                            let bytes = value.as_ref();
                            Some(B256::from_slice(bytes))
                        }
                    }
                }
                _ => None,
            })
            .collect::<Vec<B256>>().try_into().map_err(|err: Vec<_>| {
                panic!("Invalid proof length: {:?}, this should never happen", err.len())
            })
            .unwrap();

        Ok(siblings.into())
    }
}

impl RewardAccountProofV1 {
    pub fn presence(
        pos: FeeAccount,
        proof: <RewardMerkleTreeV1 as MerkleTreeScheme>::MembershipProof,
    ) -> Self {
        Self {
            account: pos.into(),
            proof: RewardMerkleProofV1::Presence(proof),
        }
    }

    pub fn absence(
        pos: RewardAccountV1,
        proof: <RewardMerkleTreeV1 as UniversalMerkleTreeScheme>::NonMembershipProof,
    ) -> Self {
        Self {
            account: pos.into(),
            proof: RewardMerkleProofV1::Absence(proof),
        }
    }

    pub fn prove(tree: &RewardMerkleTreeV1, account: Address) -> Option<(Self, U256)> {
        match tree.universal_lookup(RewardAccountV1(account)) {
            LookupResult::Ok(balance, proof) => Some((
                Self {
                    account,
                    proof: RewardMerkleProofV1::Presence(proof),
                },
                balance.0,
            )),
            LookupResult::NotFound(proof) => Some((
                Self {
                    account,
                    proof: RewardMerkleProofV1::Absence(proof),
                },
                U256::ZERO,
            )),
            LookupResult::NotInMemory => None,
        }
    }

    pub fn verify(&self, comm: &RewardMerkleCommitmentV1) -> anyhow::Result<U256> {
        match &self.proof {
            RewardMerkleProofV1::Presence(proof) => {
                ensure!(
                    RewardMerkleTreeV1::verify(comm, RewardAccountV1(self.account), proof)?.is_ok(),
                    "invalid proof"
                );
                Ok(proof
                    .elem()
                    .context("presence proof is missing account balance")?
                    .0)
            },
            RewardMerkleProofV1::Absence(proof) => {
                let tree = RewardMerkleTreeV1::from_commitment(comm);
                ensure!(
                    RewardMerkleTreeV1::non_membership_verify(
                        tree.commitment(),
                        RewardAccountV1(self.account),
                        proof
                    )?,
                    "invalid proof"
                );
                Ok(U256::ZERO)
            },
        }
    }

    pub fn remember(&self, tree: &mut RewardMerkleTreeV1) -> anyhow::Result<()> {
        match &self.proof {
            RewardMerkleProofV1::Presence(proof) => {
                tree.remember(
                    RewardAccountV1(self.account),
                    proof
                        .elem()
                        .context("presence proof is missing account balance")?,
                    proof,
                )?;
                Ok(())
            },
            RewardMerkleProofV1::Absence(proof) => {
                tree.non_membership_remember(RewardAccountV1(self.account), proof)?;
                Ok(())
            },
        }
    }
}

impl From<(RewardAccountProofV2, U256)> for RewardAccountQueryDataV2 {
    fn from((proof, balance): (RewardAccountProofV2, U256)) -> Self {
        Self { balance, proof }
    }
}

#[derive(Clone, Debug)]
pub struct ComputedRewards {
    leader_address: Address,
    // leader commission reward
    leader_commission: RewardAmount,
    // delegator rewards
    delegators: Vec<(Address, RewardAmount)>,
}

impl ComputedRewards {
    pub fn new(
        delegators: Vec<(Address, RewardAmount)>,
        leader_address: Address,
        leader_commission: RewardAmount,
    ) -> Self {
        Self {
            delegators,
            leader_address,
            leader_commission,
        }
    }

    pub fn leader_commission(&self) -> &RewardAmount {
        &self.leader_commission
    }

    pub fn delegators(&self) -> &Vec<(Address, RewardAmount)> {
        &self.delegators
    }

    // chains delegation rewards and leader commission reward
    pub fn all_rewards(self) -> Vec<(Address, RewardAmount)> {
        self.delegators
            .into_iter()
            .chain(once((self.leader_address, self.leader_commission)))
            .collect()
    }
}

pub struct RewardDistributor {
    validator: Validator<BLSPubKey>,
    block_reward: RewardAmount,
    total_distributed: RewardAmount,
}

impl RewardDistributor {
    pub fn new(
        validator: Validator<BLSPubKey>,
        block_reward: RewardAmount,
        total_distributed: RewardAmount,
    ) -> Self {
        Self {
            validator,
            block_reward,
            total_distributed,
        }
    }

    pub fn validator(&self) -> Validator<BLSPubKey> {
        self.validator.clone()
    }

    pub fn block_reward(&self) -> RewardAmount {
        self.block_reward
    }

    pub fn total_distributed(&self) -> RewardAmount {
        self.total_distributed
    }

    pub fn update_rewards_delta(&self, delta: &mut Delta) -> anyhow::Result<()> {
        // Update delta rewards
        delta
            .rewards_delta
            .insert(RewardAccountV2(self.validator().account));
        delta.rewards_delta.extend(
            self.validator()
                .delegators
                .keys()
                .map(|d| RewardAccountV2(*d)),
        );

        Ok(())
    }

    pub fn update_reward_balance<P>(
        tree: &mut P,
        account: &P::Index,
        amount: P::Element,
    ) -> anyhow::Result<()>
    where
        P: PersistentUniversalMerkleTreeScheme,
        P: MerkleTreeScheme<Element = RewardAmount>,
        P::Index: Borrow<<P as MerkleTreeScheme>::Index> + std::fmt::Display,
    {
        let mut err = None;
        *tree = tree.persistent_update_with(account.clone(), |balance| {
            let balance = balance.copied();
            match balance.unwrap_or_default().0.checked_add(amount.0) {
                Some(updated) => Some(updated.into()),
                None => {
                    err = Some(format!("overflowed reward balance for account {account}"));
                    balance
                },
            }
        })?;

        if let Some(error) = err {
            tracing::warn!(error);
            bail!(error)
        }

        Ok(())
    }

    pub fn apply_rewards(
        &mut self,
        version: vbs::version::Version,
        state: &mut ValidatedState,
    ) -> anyhow::Result<()> {
        let computed_rewards = self.compute_rewards()?;

        if version <= EpochVersion::version() {
            for (address, reward) in computed_rewards.all_rewards() {
                Self::update_reward_balance(
                    &mut state.reward_merkle_tree_v1,
                    &RewardAccountV1(address),
                    reward,
                )?;
                tracing::debug!(%address, %reward, "applied v1 rewards");
            }
        } else {
            for (address, reward) in computed_rewards.all_rewards() {
                Self::update_reward_balance(
                    &mut state.reward_merkle_tree_v2,
                    &RewardAccountV2(address),
                    reward,
                )?;
                tracing::debug!(%address, %reward, "applied v2 rewards");
            }
        }

        self.total_distributed += self.block_reward();

        Ok(())
    }

    /// Computes the reward in a block for the validator and its delegators
    /// based on the commission rate, individual delegator stake, and total block reward.
    ///
    /// The block reward is distributed among the delegators first based on their stake,
    /// with the remaining amount from the block reward given to the validator as the commission.
    /// Any minor discrepancies due to rounding off errors are adjusted in the leader reward
    /// to ensure the total reward is exactly equal to block reward.
    pub fn compute_rewards(&self) -> anyhow::Result<ComputedRewards> {
        ensure!(
            self.validator.commission <= COMMISSION_BASIS_POINTS,
            "commission must not exceed {COMMISSION_BASIS_POINTS}"
        );

        let mut rewards = Vec::new();

        let total_reward = self.block_reward.0;
        let delegators_ratio_basis_points = U256::from(COMMISSION_BASIS_POINTS)
            .checked_sub(U256::from(self.validator.commission))
            .context("overflow")?;
        let delegators_reward = delegators_ratio_basis_points
            .checked_mul(total_reward)
            .context("overflow")?;

        // Distribute delegator rewards
        let total_stake = self.validator.stake;
        let mut delegators_total_reward_distributed = U256::from(0);
        for (delegator_address, delegator_stake) in &self.validator.delegators {
            let delegator_reward = RewardAmount::from(
                (delegator_stake
                    .checked_mul(delegators_reward)
                    .context("overflow")?
                    .checked_div(total_stake)
                    .context("overflow")?)
                .checked_div(U256::from(COMMISSION_BASIS_POINTS))
                .context("overflow")?,
            );

            delegators_total_reward_distributed += delegator_reward.0;

            rewards.push((*delegator_address, delegator_reward));
        }

        let leader_commission = total_reward
            .checked_sub(delegators_total_reward_distributed)
            .context("overflow")?;

        Ok(ComputedRewards::new(
            rewards,
            self.validator.account,
            leader_commission.into(),
        ))
    }
}

/// Distributes the block reward for a given block height
///
/// Rewards are only distributed if the block belongs to an epoch beyond the second epoch.
///
/// The function also calculates the appropriate reward (fixed or dynamic) based
/// on the protocol version.
pub async fn distribute_block_reward(
    instance_state: &NodeState,
    validated_state: &mut ValidatedState,
    parent_leaf: &Leaf2,
    view_number: ViewNumber,
    version: vbs::version::Version,
) -> anyhow::Result<Option<RewardDistributor>> {
    let height = parent_leaf.height() + 1;

    let epoch_height = instance_state
        .epoch_height
        .context("epoch height not found")?;
    let epoch = EpochNumber::new(epoch_from_block_number(height, epoch_height));
    let coordinator = instance_state.coordinator.clone();
    let first_epoch = {
        coordinator
            .membership()
            .read()
            .await
            .first_epoch()
            .context("The first epoch was not set.")?
    };

    // Rewards are distributed only if the current epoch is not the first or second epoch
    // this is because we don't have stake table from the contract for the first two epochs
    if epoch <= first_epoch + 1 {
        return Ok(None);
    }

    // Determine who the block leader is for this view and ensure missing block
    // rewards are fetched from peers if needed.

    let leader = get_leader_and_fetch_missing_rewards(
        instance_state,
        validated_state,
        parent_leaf,
        view_number,
    )
    .await?;

    let parent_header = parent_leaf.block_header();

    // Initialize the total rewards distributed so far in this block.
    let mut previously_distributed = parent_header.total_reward_distributed().unwrap_or_default();

    // Decide whether to use a fixed or dynamic block reward.
    let block_reward = if version >= DrbAndHeaderUpgradeVersion::version() {
        instance_state
            .block_reward(EpochNumber::new(*epoch))
            .await
            .with_context(|| format!("block reward is None for epoch {epoch}"))?
    } else {
        instance_state.fixed_block_reward().await?
    };

    // If we are in the DRB + header upgrade
    // and the parent block is from V3 (which does not have a previously distributed reward field),
    // we need to recompute the previously distributed rewards
    // using the fixed block reward and the number of blocks in which fixed reward was distributed
    if version >= DrbAndHeaderUpgradeVersion::version()
        && parent_header.version() == EpochVersion::version()
    {
        ensure!(
            instance_state.epoch_start_block != 0,
            "epoch_start_block is zero"
        );

        let fixed_block_reward = instance_state.fixed_block_reward().await?;

        // Compute the first block where rewards start being distributed.
        // Rewards begin only after the first two epochs
        // Example:
        //   epoch_height = 10, first_epoch = 1
        // first_reward_block = 21
        let first_reward_block = (*first_epoch + 1) * epoch_height + 1;
        // We only compute fixed reward distribured so far
        // once the current block
        // is beyond the first rewardable block.
        if height > first_reward_block {
            // If v4 upgrade started at block 101, and first_reward_block is 21:
            // total_distributed = (101 - 21) * fixed_block_reward
            let blocks = height.checked_sub(first_reward_block).with_context(|| {
                format!("height ({height}) - first_reward_block ({first_reward_block}) underflowed")
            })?;
            previously_distributed = U256::from(blocks)
                .checked_mul(fixed_block_reward.0)
                .with_context(|| {
                    format!(
                        "overflow during total_distributed calculation: blocks={blocks}, \
                         fixed_block_reward={}",
                        fixed_block_reward.0
                    )
                })?
                .into();
        }
    }

    if block_reward.0.is_zero() {
        tracing::info!("block reward is zero. height={height}. epoch={epoch}");
        return Ok(None);
    }

    let mut reward_distributor =
        RewardDistributor::new(leader, block_reward, previously_distributed);

    reward_distributor.apply_rewards(version, validated_state)?;

    Ok(Some(reward_distributor))
}

pub async fn get_leader_and_fetch_missing_rewards(
    instance_state: &NodeState,
    validated_state: &mut ValidatedState,
    parent_leaf: &Leaf2,
    view: ViewNumber,
) -> anyhow::Result<Validator<BLSPubKey>> {
    let parent_height = parent_leaf.height();
    let parent_view = parent_leaf.view_number();
    let new_height = parent_height + 1;

    let epoch_height = instance_state
        .epoch_height
        .context("epoch height not found")?;
    if epoch_height == 0 {
        bail!("epoch height is 0. can not catchup reward accounts");
    }
    let epoch = EpochNumber::new(epoch_from_block_number(new_height, epoch_height));

    let coordinator = instance_state.coordinator.clone();

    let epoch_membership = coordinator.membership_for_epoch(Some(epoch)).await?;
    let membership = epoch_membership.coordinator.membership().read().await;

    let leader: BLSPubKey = membership
        .leader(view, Some(epoch))
        .context(format!("leader for epoch {epoch:?} not found"))?;

    tracing::debug!("Selected leader: {leader} for view {view} and epoch {epoch}");

    let validator = membership
        .get_validator_config(&epoch, leader)
        .context("validator not found")?;
    drop(membership);

    let mut reward_accounts = HashSet::new();
    reward_accounts.insert(validator.account.into());
    let delegators = validator
        .delegators
        .keys()
        .cloned()
        .map(|a| a.into())
        .collect::<Vec<RewardAccountV2>>();

    reward_accounts.extend(delegators.clone());

    let parent_header = parent_leaf.block_header();

    if parent_header.version() <= EpochVersion::version() {
        let accts: HashSet<_> = reward_accounts
            .into_iter()
            .map(RewardAccountV1::from)
            .collect();
        let missing_reward_accts = validated_state.forgotten_reward_accounts_v1(accts);

        if !missing_reward_accts.is_empty() {
            tracing::warn!(
                parent_height,
                ?parent_view,
                ?missing_reward_accts,
                "fetching missing v1 reward accounts from peers"
            );

            let missing_account_proofs = instance_state
                .state_catchup
                .fetch_reward_accounts_v1(
                    instance_state,
                    parent_height,
                    parent_view,
                    validated_state.reward_merkle_tree_v1.commitment(),
                    missing_reward_accts,
                )
                .await?;

            for proof in missing_account_proofs.iter() {
                proof
                    .remember(&mut validated_state.reward_merkle_tree_v1)
                    .expect("proof previously verified");
            }
        }
    } else {
        let missing_reward_accts = validated_state.forgotten_reward_accounts_v2(reward_accounts);
        let reward_merkle_tree_root = validated_state.reward_merkle_tree_v2.commitment();
        if !missing_reward_accts.is_empty() {
            tracing::warn!(
                parent_height,
                ?parent_view,
                ?missing_reward_accts,
                %reward_merkle_tree_root,
                "fetching missing reward accounts from peers"
            );

            let missing_account_proofs = instance_state
                .state_catchup
                .fetch_reward_accounts_v2(
                    instance_state,
                    parent_height,
                    parent_view,
                    reward_merkle_tree_root,
                    missing_reward_accts,
                )
                .await?;

            for proof in missing_account_proofs.iter() {
                proof
                    .remember(&mut validated_state.reward_merkle_tree_v2)
                    .expect("proof previously verified");
            }
        }
    }

    Ok(validator)
}

/// Result of epoch rewards calculation.
#[derive(Debug, Clone)]
pub struct EpochRewardsResult {
    /// The epoch this calculation was for.
    pub epoch: EpochNumber,
    /// The updated reward merkle tree with all rewards applied.
    pub reward_tree: RewardMerkleTreeV2,
    /// Total amount of rewards distributed in this epoch.
    pub total_distributed: RewardAmount,
    /// Set of all reward accounts that were modified.
    pub changed_accounts: HashSet<RewardAccountV2>,
    /// Timestamp when this result was inserted into the cache.
    pub inserted_at: std::time::Instant,
}

/// Manages epoch-based reward calculations in the background.
#[derive(Debug, Default)]
pub struct EpochRewardsCalculator {
    /// Cached results by epoch
    pub results: HashMap<EpochNumber, EpochRewardsResult>,
    /// Pending calculations by epoch
    pending: HashMap<EpochNumber, JoinHandle<anyhow::Result<EpochRewardsResult>>>,
}

impl EpochRewardsCalculator {
    pub fn new() -> Self {
        Self {
            results: HashMap::new(),
            pending: HashMap::new(),
        }
    }

    /// Check if we have a cached result for epoch.
    pub fn has_result(&self, epoch: EpochNumber) -> bool {
        self.results.contains_key(&epoch)
    }

    /// Check if calculation is in progress for epoch.
    pub fn is_calculating(&self, epoch: EpochNumber) -> bool {
        self.pending.contains_key(&epoch)
    }

    /// Get result for epoch, awaiting pending calculation if needed.
    pub async fn get_result(&mut self, epoch: EpochNumber) -> Option<EpochRewardsResult> {
        if let Some(result) = self.results.get(&epoch) {
            return Some(result.clone());
        }

        // Await pending calculation if exists
        if let Some(handle) = self.pending.remove(&epoch) {
            match handle.await {
                Ok(Ok(result)) => {
                    tracing::info!(%epoch, total = %result.total_distributed.0, "epoch rewards calculation completed");
                    self.results.insert(epoch, result.clone());
                    return Some(result);
                },
                Ok(Err(e)) => {
                    tracing::error!(%epoch, error = %e, "epoch rewards calculation failed");
                },
                Err(e) => {
                    tracing::error!(%epoch, error = %e, "epoch rewards task panicked");
                },
            }
        }

        None
    }

    /// Start a background task that calculates epoch rewards.
    /// Does nothing if calculation is already done or in progress for this epoch.
    ///
    pub fn spawn_background_task(
        &mut self,
        epoch: EpochNumber,
        epoch_height: u64,
        reward_tree: RewardMerkleTreeV2,
        instance_state: NodeState,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
        leader_counts: Option<LeaderCounts>,
    ) {
        if self.results.contains_key(&epoch) {
            tracing::debug!(%epoch, "calculation already completed, skipping");
            return;
        }
        if self.pending.contains_key(&epoch) {
            tracing::debug!(%epoch, "calculation already in progress, skipping");
            return;
        }

        tracing::info!(
            %epoch,
            has_leader_counts = leader_counts.is_some(),
            "starting background epoch rewards task"
        );

        let handle = tokio::spawn(async move {
            Self::fetch_and_calculate(
                epoch,
                epoch_height,
                reward_tree,
                instance_state,
                coordinator,
                leader_counts,
            )
            .await
        });
        self.pending.insert(epoch, handle);
    }

    async fn fetch_and_calculate(
        epoch: EpochNumber,
        epoch_height: u64,
        mut reward_tree: RewardMerkleTreeV2,
        instance_state: NodeState,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
        leader_counts: Option<LeaderCounts>,
    ) -> anyhow::Result<EpochRewardsResult> {
        let epoch_last_block_height = (*epoch) * epoch_height;

        tracing::info!(
            %epoch,
            epoch_last_block_height,
            has_leader_counts = leader_counts.is_some(),
            "fetch_and_calculate: starting"
        );

        // Get leader_counts for this epoch if not provided
        let leader_counts = if let Some(lc) = leader_counts {
            lc
        } else {
            // Fetch the header at the last block of the epoch
            let header = instance_state
                .state_catchup
                .as_ref()
                .fetch_header(epoch_last_block_height)
                .await
                .with_context(|| {
                    format!(
                        "failed to fetch header at height {epoch_last_block_height} for epoch \
                         {epoch}"
                    )
                })?;

            tracing::info!(
                %epoch,
                header_height = header.height(),
                header_version = %header.version(),
                header_reward_merkle_tree_root = %header.reward_merkle_tree_root(),
                "fetch_and_calculate: fetched header"
            );

            anyhow::ensure!(
                header.version() >= EpochRewardVersion::version(),
                "header version {} is pre-V6, cannot calculate rewards",
                header.version()
            );

            // Validate the reward merkle tree against the header commitment.
            // If they don't match, use an empty tree so the rebuild logic
            // will fetch all accounts from peers.
            // This should never happen though because we are passing parent reward merkle tree
            // which is only changed once per epoch
            let expected_root = header.reward_merkle_tree_root().right();
            let actual_root = reward_tree.commitment();
            if expected_root != Some(actual_root) {
                tracing::warn!(
                    %epoch,
                    ?expected_root,
                    ?actual_root,
                    "reward merkle tree root mismatch, using empty tree"
                );
                reward_tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
            }

            *header
                .leader_counts()
                .expect("V6+ header must have leader_counts")
        };

        // Ensure stake table is available for this epoch
        if let Err(err) = coordinator.membership_for_epoch(Some(epoch)).await {
            tracing::info!(%epoch, "stake table missing for epoch, triggering catchup: {err:#}");
            coordinator
                .wait_for_catchup(epoch)
                .await
                .context(format!("failed to catch up for epoch={epoch}"))?;
        }

        let membership = coordinator.membership().read().await;
        let validators: Vec<_> = membership
            .active_validators(&epoch)?
            .into_values()
            .collect();
        let block_reward = membership
            .epoch_block_reward(epoch)
            .context("block reward not found for epoch")?;
        drop(membership);

        tracing::info!(
            %epoch,
            num_validators = validators.len(),
            %block_reward,
            "fetch_and_calculate: got validators and block_reward"
        );

        // Check if we're missing accounts that need to be in the tree
        let accounts_to_update: Vec<_> = leader_counts
            .iter()
            .enumerate()
            .filter(|(_, &count)| count > 0)
            .flat_map(|(index, _)| {
                validators.get(index).into_iter().flat_map(|v| {
                    std::iter::once(RewardAccountV2(v.account))
                        .chain(v.delegators.keys().map(|d| RewardAccountV2(*d)))
                })
            })
            .collect();

        let missing_accounts: Vec<_> = accounts_to_update
            .iter()
            .filter(|account| reward_tree.lookup(**account).expect_not_in_memory().is_ok())
            .cloned()
            .collect();

        // If we have missing accounts, fetch all reward accounts from peers and rebuild the tree
        if !missing_accounts.is_empty() {
            tracing::info!(
                %epoch,
                num_missing = missing_accounts.len(),
                "missing accounts detected, fetching all reward accounts from peers"
            );

            tracing::info!(
                %epoch,
                epoch_last_block_height,
                "fetching all reward accounts from peers to rebuild tree"
            );

            // Fetch all reward accounts from peers (paginated)
            let mut all_accounts = Vec::new();
            let mut offset = 0u64;
            let limit = 10_000u64;

            loop {
                let accounts = instance_state
                    .state_catchup
                    .as_ref()
                    .fetch_all_reward_accounts(epoch_last_block_height, offset, limit)
                    .await
                    .with_context(|| {
                        format!(
                            "failed to fetch reward accounts at height {epoch_last_block_height}, \
                             offset {offset}"
                        )
                    })?;

                let count = accounts.len();
                all_accounts.extend(accounts);

                if (count as u64) < limit {
                    break;
                }
                offset += limit;
            }

            tracing::info!(
                %epoch,
                num_accounts = all_accounts.len(),
                "fetched all reward accounts, rebuilding tree"
            );

            // Rebuild the tree from scratch with all the accounts
            let kv_pairs: Vec<(RewardAccountV2, RewardAmount)> = all_accounts;
            reward_tree = RewardMerkleTreeV2::from_kv_set(REWARD_MERKLE_TREE_V2_HEIGHT, kv_pairs)
                .context("failed to rebuild reward merkle tree from accounts")?;

            tracing::info!(
                %epoch,
                reward_tree_commitment = %reward_tree.commitment(),
                "reward tree rebuilt successfully"
            );
        }

        tracing::info!(
            %epoch,
            reward_tree_commitment = %reward_tree.commitment(),
            "starting final epoch calculation"
        );

        Self::calculate_all_rewards(epoch, leader_counts, reward_tree, block_reward, validators)
            .await
    }

    /// Calculate all rewards for the epoch and update the reward tree.
    pub async fn calculate_all_rewards(
        epoch: EpochNumber,
        leader_counts: LeaderCounts,
        mut reward_tree: RewardMerkleTreeV2,
        block_reward: RewardAmount,
        validators: Vec<Validator<BLSPubKey>>,
    ) -> anyhow::Result<EpochRewardsResult> {
        let mut total_distributed = U256::ZERO;
        let mut changed_accounts = HashSet::new();

        for (index, &count) in leader_counts.iter().enumerate() {
            if count == 0 {
                continue;
            }

            let validator = validators.get(index).context(format!(
                "validator not found for leader index {index} with count {count} in epoch {epoch}"
            ))?;

            // validator_reward = count * block_reward
            let validator_reward = block_reward
                .0
                .checked_mul(U256::from(count))
                .context("overflow in validator reward calculation")?;

            if validator_reward.is_zero() {
                continue;
            }

            changed_accounts.insert(RewardAccountV2(validator.account));
            changed_accounts.extend(validator.delegators.keys().map(|d| RewardAccountV2(*d)));

            let distributor = RewardDistributor::new(
                validator.clone(),
                RewardAmount(validator_reward),
                Default::default(),
            );

            let computed_rewards = distributor.compute_rewards()?;

            for (address, reward) in computed_rewards.all_rewards() {
                RewardDistributor::update_reward_balance(
                    &mut reward_tree,
                    &RewardAccountV2(address),
                    reward,
                )?;
                tracing::debug!(%epoch, %address, %reward, "applied epoch reward");
            }

            total_distributed += validator_reward;
        }

        tracing::info!(
            %epoch,
            total_distributed = %total_distributed,
            num_changed_accounts = changed_accounts.len(),
            "epoch rewards calculation complete"
        );

        Ok(EpochRewardsResult {
            epoch,
            reward_tree,
            total_distributed: RewardAmount(total_distributed),
            changed_accounts,
            inserted_at: std::time::Instant::now(),
        })
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;

    // TODO: current tests are just sanity checks, we need more.

    #[test]
    fn test_reward_calculation_sanity_checks() {
        // This test verifies that the total rewards distributed match the block reward. Due to
        // rounding effects in distribution, the validator may receive a slightly higher amount
        // because the remainder after delegator distribution is sent to the validator.

        let validator = Validator::mock();
        let mut distributor = RewardDistributor::new(
            validator,
            RewardAmount(U256::from(1902000000000000000_u128)),
            U256::ZERO.into(),
        );
        let rewards = distributor.compute_rewards().unwrap();
        let total = |rewards: ComputedRewards| {
            rewards
                .all_rewards()
                .iter()
                .fold(U256::ZERO, |acc, (_, r)| acc + r.0)
        };
        assert_eq!(total(rewards.clone()), distributor.block_reward.0);

        distributor.validator.commission = 0;
        let rewards = distributor.compute_rewards().unwrap();
        assert_eq!(total(rewards.clone()), distributor.block_reward.0);

        distributor.validator.commission = 10000;
        let rewards = distributor.compute_rewards().unwrap();
        assert_eq!(total(rewards.clone()), distributor.block_reward.0);
        let leader_commission = rewards.leader_commission();
        assert_eq!(*leader_commission, distributor.block_reward);

        distributor.validator.commission = 10001;
        assert!(distributor
            .compute_rewards()
            .err()
            .unwrap()
            .to_string()
            .contains("must not exceed"));
    }

    #[test]
    fn test_compute_rewards_validator_commission() {
        let validator = Validator::mock();
        let mut distributor = RewardDistributor::new(
            validator.clone(),
            RewardAmount(U256::from(1902000000000000000_u128)),
            U256::ZERO.into(),
        );
        distributor.validator.commission = 0;

        let rewards = distributor.compute_rewards().unwrap();

        let leader_commission = rewards.leader_commission();
        let percentage =
            leader_commission.0 * U256::from(COMMISSION_BASIS_POINTS) / distributor.block_reward.0;
        assert_eq!(percentage, U256::ZERO);

        // 3%
        distributor.validator.commission = 300;

        let rewards = distributor.compute_rewards().unwrap();
        let leader_commission = rewards.leader_commission();
        let percentage =
            leader_commission.0 * U256::from(COMMISSION_BASIS_POINTS) / distributor.block_reward.0;
        println!("percentage: {percentage:?}");
        assert_eq!(percentage, U256::from(300));

        //100%
        distributor.validator.commission = 10000;

        let rewards = distributor.compute_rewards().unwrap();
        let leader_commission = rewards.leader_commission();
        assert_eq!(*leader_commission, distributor.block_reward);
    }
}
