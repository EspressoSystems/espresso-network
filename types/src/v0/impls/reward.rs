use std::{collections::HashSet, iter::once, str::FromStr};

use alloy::primitives::{
    utils::{parse_units, ParseUnits},
    Address, U256,
};
use anyhow::{bail, ensure, Context};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
};
use committable::{Commitment, Committable, RawCommitmentBuilder};
use hotshot::types::BLSPubKey;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    traits::{election::Membership, node_implementation::ConsensusTime},
    utils::epoch_from_block_number,
};
use jf_merkle_tree::{
    ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme, LookupResult,
    MerkleCommitment, MerkleTreeScheme, PersistentUniversalMerkleTreeScheme, ToTraversalPath,
    UniversalMerkleTreeScheme,
};
use num_traits::CheckedSub;
use sequencer_utils::{
    impl_serde_from_string_or_integer, impl_to_fixed_bytes, ser::FromStringOrInteger,
};
use vbs::version::StaticVersionType;

use super::{
    v0_1::{
        RewardAccount, RewardAccountProof, RewardAccountQueryData, RewardAmount, RewardInfo,
        RewardMerkleCommitment, RewardMerkleProof, RewardMerkleTree, COMMISSION_BASIS_POINTS,
    },
    v0_3::Validator,
    Leaf2, NodeState, ValidatedState,
};
use crate::{
    eth_signature_key::EthKeyPair, Delta, DrbAndHeaderUpgradeVersion, EpochVersion, FeeAccount,
};

impl Committable for RewardInfo {
    fn commit(&self) -> Commitment<Self> {
        RawCommitmentBuilder::new(&Self::tag())
            .fixed_size_field("account", &self.account.to_fixed_bytes())
            .fixed_size_field("amount", &self.amount.to_fixed_bytes())
            .finalize()
    }
    fn tag() -> String {
        "REWARD_INFO".into()
    }
}

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
        // For backwards compatibility, we have an ad hoc parser for WEI amounts represented as hex
        // strings.
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
impl RewardAccount {
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

impl FromStr for RewardAccount {
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

impl Valid for RewardAccount {
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
impl CanonicalSerialize for RewardAccount {
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
impl CanonicalDeserialize for RewardAccount {
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

impl ToTraversalPath<256> for RewardAccount {
    fn to_traversal_path(&self, height: usize) -> Vec<usize> {
        self.0
            .as_slice()
            .iter()
            .take(height)
            .map(|i| *i as usize)
            .collect()
    }
}

#[allow(dead_code)]
impl RewardAccountProof {
    pub fn presence(
        pos: FeeAccount,
        proof: <RewardMerkleTree as MerkleTreeScheme>::MembershipProof,
    ) -> Self {
        Self {
            account: pos.into(),
            proof: RewardMerkleProof::Presence(proof),
        }
    }

    pub fn absence(
        pos: RewardAccount,
        proof: <RewardMerkleTree as UniversalMerkleTreeScheme>::NonMembershipProof,
    ) -> Self {
        Self {
            account: pos.into(),
            proof: RewardMerkleProof::Absence(proof),
        }
    }

    pub fn prove(tree: &RewardMerkleTree, account: Address) -> Option<(Self, U256)> {
        match tree.universal_lookup(RewardAccount(account)) {
            LookupResult::Ok(balance, proof) => Some((
                Self {
                    account,
                    proof: RewardMerkleProof::Presence(proof),
                },
                balance.0,
            )),
            LookupResult::NotFound(proof) => Some((
                Self {
                    account,
                    proof: RewardMerkleProof::Absence(proof),
                },
                U256::ZERO,
            )),
            LookupResult::NotInMemory => None,
        }
    }

    pub fn verify(&self, comm: &RewardMerkleCommitment) -> anyhow::Result<U256> {
        match &self.proof {
            RewardMerkleProof::Presence(proof) => {
                ensure!(
                    RewardMerkleTree::verify(comm.digest(), RewardAccount(self.account), proof)?
                        .is_ok(),
                    "invalid proof"
                );
                Ok(proof
                    .elem()
                    .context("presence proof is missing account balance")?
                    .0)
            },
            RewardMerkleProof::Absence(proof) => {
                let tree = RewardMerkleTree::from_commitment(comm);
                ensure!(
                    tree.non_membership_verify(RewardAccount(self.account), proof)?,
                    "invalid proof"
                );
                Ok(U256::ZERO)
            },
        }
    }

    pub fn remember(&self, tree: &mut RewardMerkleTree) -> anyhow::Result<()> {
        match &self.proof {
            RewardMerkleProof::Presence(proof) => {
                tree.remember(
                    RewardAccount(self.account),
                    proof
                        .elem()
                        .context("presence proof is missing account balance")?,
                    proof,
                )?;
                Ok(())
            },
            RewardMerkleProof::Absence(proof) => {
                tree.non_membership_remember(RewardAccount(self.account), proof)?;
                Ok(())
            },
        }
    }
}

impl From<(RewardAccountProof, U256)> for RewardAccountQueryData {
    fn from((proof, balance): (RewardAccountProof, U256)) -> Self {
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
            .insert(RewardAccount(self.validator().account));
        delta.rewards_delta.extend(
            self.validator()
                .delegators
                .keys()
                .map(|d| RewardAccount(*d)),
        );

        Ok(())
    }

    pub fn apply_rewards(
        &mut self,
        mut reward_state: RewardMerkleTree,
    ) -> anyhow::Result<RewardMerkleTree> {
        let mut update_balance = |account: &RewardAccount, amount: RewardAmount| {
            let mut err = None;
            reward_state = reward_state.persistent_update_with(account, |balance| {
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
            Ok::<(), anyhow::Error>(())
        };
        let computed_rewards = self.compute_rewards()?;
        for (address, reward) in computed_rewards.all_rewards() {
            update_balance(&RewardAccount(address), reward)?;
            tracing::debug!("applied rewards address={address} reward={reward}",);
        }

        self.total_distributed += self.block_reward();

        Ok(reward_state)
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
/// The function also calculates the appropriate reward (fixed or dynamic) based on the protocol version.
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

    // Determine who the block leader is for this view and ensure missing block rewards are fetched from peers if needed.

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
        let block_reward = instance_state
            .block_reward(Some(EpochNumber::new(*epoch)))
            .await
            .with_context(|| format!("block reward is None for epoch {epoch}"))?;

        // If the current block is the start block of the new v4 version,
        // we use *fixed block reward* for calculating the total rewards distributed so far.
        if parent_header.version() == EpochVersion::version() {
            ensure!(
                instance_state.epoch_start_block != 0,
                "epoch_start_block is zero"
            );

            let fixed_block_reward = instance_state
                .block_reward(None)
                .await
                .with_context(|| format!("block reward is None for epoch {epoch}"))?;

            // Compute the first block where rewards start being distributed.
            // Rewards begin only after the first two epochs
            // Example:
            //   epoch_height = 10, first_epoch = 1
            // first_reward_block = 31
            let first_reward_block = (*first_epoch + 2) * epoch_height + 1;

            // If v4 upgrade started at block 101, and first_reward_block is 31:
            // total_distributed = (101 - 31) * fixed_block_reward
            let blocks = height
                .checked_sub(first_reward_block)
                .context("height - epoch_start_block underflowed")?;

            previously_distributed = U256::from(blocks)
                .checked_mul(fixed_block_reward.0)
                .context("overflow during total_distributed calculation")?
                .into();
        }

        block_reward
    } else {
        instance_state
            .block_reward(None)
            .await
            .with_context(|| format!("fixed block reward is None for epoch {epoch}"))?
    };

    let mut reward_distributor =
        RewardDistributor::new(leader, block_reward, previously_distributed.into());

    let reward_state =
        reward_distributor.apply_rewards(validated_state.reward_merkle_tree.clone())?;
    validated_state.reward_merkle_tree = reward_state;

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
        .collect::<Vec<RewardAccount>>();

    reward_accounts.extend(delegators.clone());
    let missing_reward_accts = validated_state.forgotten_reward_accounts(reward_accounts);

    if !missing_reward_accts.is_empty() {
        tracing::warn!(
            parent_height,
            ?parent_view,
            ?missing_reward_accts,
            "fetching missing reward accounts from peers"
        );

        let missing_account_proofs = instance_state
            .state_catchup
            .fetch_reward_accounts(
                instance_state,
                parent_height,
                parent_view,
                validated_state.reward_merkle_tree.commitment(),
                missing_reward_accts,
            )
            .await?;

        for proof in missing_account_proofs.iter() {
            proof
                .remember(&mut validated_state.reward_merkle_tree)
                .expect("proof previously verified");
        }
    }
    Ok(validator)
}

#[cfg(test)]
pub mod tests {

    use super::*;

    // TODO: current tests are just sanity checks, we need more.

    #[test]
    fn test_reward_calculation_sanity_checks() {
        // This test verifies that the total rewards distributed match the block reward.
        // Due to rounding effects in distribution, the validator may receive a slightly higher amount
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
        assert_eq!(total(rewards.clone()), distributor.block_reward.into());

        distributor.validator.commission = 0;
        let rewards = distributor.compute_rewards().unwrap();
        assert_eq!(total(rewards.clone()), distributor.block_reward.into());

        distributor.validator.commission = 10000;
        let rewards = distributor.compute_rewards().unwrap();
        assert_eq!(total(rewards.clone()), distributor.block_reward.into());
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
