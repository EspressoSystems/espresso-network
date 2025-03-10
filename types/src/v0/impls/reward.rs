use std::str::FromStr;

use anyhow::{bail, ensure, Context};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
};
use committable::{Commitment, Committable, RawCommitmentBuilder};
use contract_bindings_alloy::feecontract::FeeContract::Deposit;
use contract_bindings_ethers::fee_contract::DepositFilter;
use ethers::{
    prelude::{Address, U256},
    utils::{parse_units, ParseUnits},
};
use ethers_conv::ToEthers;
use hotshot_query_service::explorer::MonetaryValue;
use hotshot_types::traits::block_contents::BuilderFee;
use itertools::Itertools;
use jf_merkle_tree::{
    ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme, LookupResult,
    MerkleCommitment, MerkleTreeError, MerkleTreeScheme, ToTraversalPath,
    UniversalMerkleTreeScheme,
};
use num_traits::CheckedSub;
use sequencer_utils::{
    impl_serde_from_string_or_integer, impl_to_fixed_bytes, ser::FromStringOrInteger,
};
use thiserror::Error;

use crate::{
    eth_signature_key::EthKeyPair, v0_99::IterableFeeInfo, AccountQueryData, FeeAccount,
    FeeAccountProof, FeeAmount, FeeInfo, FeeMerkleCommitment, FeeMerkleProof, FeeMerkleTree,
    SeqTypes,
};

use super::v0_1::{
    RewardAccount, RewardAccountProof, RewardAccountQueryData, RewardAmount, RewardInfo,
    RewardMerkleCommitment, RewardMerkleProof, RewardMerkleTree,
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
        Self(amt.into())
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
        if self.0 <= u64::MAX.into() {
            Some(self.0.as_u64())
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
        self.0.as_bytes()
    }
    /// Return array containing underlying bytes of inner `Address` type
    pub fn to_fixed_bytes(self) -> [u8; 20] {
        self.0.to_fixed_bytes()
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
        let value = U256::from_little_endian(&bytes);
        Ok(Self(value))
    }
}
impl CanonicalSerialize for RewardAccount {
    fn serialize_with_mode<W: std::io::prelude::Write>(
        &self,
        mut writer: W,
        _compress: Compress,
    ) -> Result<(), SerializationError> {
        Ok(writer.write_all(&self.0.to_fixed_bytes())?)
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
            .to_fixed_bytes()
            .into_iter()
            .take(height)
            .map(|i| i as usize)
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
                0.into(),
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
                Ok(0.into())
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

/// Get a partial snapshot of the given fee state, which contains only the specified accounts.
///
/// Fails if one of the requested accounts is not represented in the original `state`.
pub fn retain_accounts(
    state: &RewardMerkleTree,
    accounts: impl IntoIterator<Item = RewardAccount>,
) -> anyhow::Result<RewardMerkleTree> {
    let mut snapshot = RewardMerkleTree::from_commitment(state.commitment());
    for account in accounts {
        match state.universal_lookup(account) {
            LookupResult::Ok(elem, proof) => {
                // This remember cannot fail, since we just constructed a valid proof, and are
                // remembering into a tree with the same commitment.
                snapshot.remember(account, *elem, proof).unwrap();
            },
            LookupResult::NotFound(proof) => {
                // Likewise this cannot fail.
                snapshot.non_membership_remember(account, proof).unwrap()
            },
            LookupResult::NotInMemory => {
                bail!("missing account {account}");
            },
        }
    }

    Ok(snapshot)
}
