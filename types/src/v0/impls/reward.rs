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

use super::{
    v0_1::{
        RewardAccount, RewardAccountProof, RewardAccountQueryData, RewardAmount, RewardInfo,
        RewardMerkleCommitment, RewardMerkleProof, RewardMerkleTree, COMMISSION_BASIS_POINTS,
    },
    v0_3::Validator,
    Leaf2, NodeState, ValidatedState,
};
use crate::{eth_signature_key::EthKeyPair, Delta, FeeAccount};

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
}

impl RewardDistributor {
    pub fn new(validator: Validator<BLSPubKey>, block_reward: RewardAmount) -> Self {
        Self {
            validator,
            block_reward,
        }
    }

    pub fn validator(&self) -> Validator<BLSPubKey> {
        self.validator.clone()
    }

    pub fn block_reward(&self) -> RewardAmount {
        self.block_reward
    }

    pub fn distribute(&self, state: &mut ValidatedState, delta: &mut Delta) -> anyhow::Result<()> {
        let reward_state = self.apply_rewards(state.reward_merkle_tree.clone())?;
        state.reward_merkle_tree = reward_state;

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
        &self,
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

        ensure!(self.block_reward.0 > U256::ZERO, "block reward is zero");

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
        let mut delegators_rewards_distributed = U256::from(0);
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

            delegators_rewards_distributed += delegator_reward.0;

            rewards.push((*delegator_address, delegator_reward));
        }

        let leader_commission = total_reward
            .checked_sub(delegators_rewards_distributed)
            .context("overflow")?;

        Ok(ComputedRewards::new(
            rewards,
            self.validator.account,
            leader_commission.into(),
        ))
    }
}
/// Checks whether the given height belongs to the first or second epoch. or
/// the Genesis epoch (EpochNumber::new(0))
///
/// Rewards are not distributed for these epochs because the stake table
/// is built from the contract only when `add_epoch_root()` is called
/// by HotShot, which happens starting from the third epoch.
pub async fn first_two_epochs(height: u64, instance_state: &NodeState) -> anyhow::Result<bool> {
    let epoch_height = instance_state
        .epoch_height
        .context("epoch height not found")?;
    let epoch = EpochNumber::new(epoch_from_block_number(height, epoch_height));
    let coordinator = instance_state.coordinator.clone();
    let first_epoch = coordinator
        .membership()
        .read()
        .await
        .first_epoch()
        .context("The first epoch was not set.")?;

    Ok(epoch <= first_epoch + 1)
}

pub async fn find_validator_info(
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
