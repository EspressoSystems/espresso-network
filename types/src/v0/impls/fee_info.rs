use std::str::FromStr;

use alloy::primitives::{
    utils::{parse_units, ParseUnits},
    Address, U256,
};
use anyhow::{bail, ensure, Context};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
};
use committable::{Commitment, Committable, RawCommitmentBuilder};
use hotshot_contract_adapter::sol_types::Deposit;
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

/// Possible charge fee failures
#[derive(Error, Debug, Eq, PartialEq)]
pub enum FeeError {
    #[error("Insuficcient Funds: have {balance:?}, required {amount:?}")]
    InsufficientFunds {
        balance: Option<FeeAmount>,
        amount: FeeAmount,
    },
    #[error("Merkle Tree Error: {0}")]
    MerkleTreeError(MerkleTreeError),
}

impl FeeInfo {
    pub fn new(account: impl Into<FeeAccount>, amount: impl Into<FeeAmount>) -> Self {
        Self {
            account: account.into(),
            amount: amount.into(),
        }
    }
    /// The minimum fee paid by the given builder account for a proposed block.
    // TODO this function should take the block size as an input, we need to get this information
    // from HotShot.
    pub fn base_fee(account: FeeAccount) -> Self {
        Self {
            account,
            amount: FeeAmount::default(),
        }
    }

    pub fn genesis() -> Self {
        Self {
            account: Default::default(),
            amount: Default::default(),
        }
    }

    pub fn account(&self) -> FeeAccount {
        self.account
    }

    pub fn amount(&self) -> FeeAmount {
        self.amount
    }
    /// Get a `Vec<FeeInfo>` from `Vec<BuilderFee>`
    pub fn from_builder_fees(fees: Vec<BuilderFee<SeqTypes>>) -> Vec<FeeInfo> {
        fees.into_iter().map(FeeInfo::from).collect()
    }
}

impl IterableFeeInfo for Vec<FeeInfo> {
    /// Get sum of fee amounts
    fn amount(&self) -> Option<FeeAmount> {
        self.iter()
            // getting the u64 tests that the value fits
            .map(|fee_info| fee_info.amount.as_u64())
            .collect::<Option<Vec<u64>>>()
            .and_then(|amounts| amounts.iter().try_fold(0u64, |acc, n| acc.checked_add(*n)))
            .map(FeeAmount::from)
    }

    /// Get a `Vec` of all unique fee accounts
    fn accounts(&self) -> Vec<FeeAccount> {
        self.iter()
            .unique_by(|entry| &entry.account)
            .map(|entry| entry.account)
            .collect()
    }
}

impl IterableFeeInfo for Vec<BuilderFee<SeqTypes>> {
    /// Get sum of amounts
    fn amount(&self) -> Option<FeeAmount> {
        self.iter()
            .map(|fee_info| fee_info.fee_amount)
            .try_fold(0u64, |acc, n| acc.checked_add(n))
            .map(FeeAmount::from)
    }

    /// Get a `Vec` of all unique fee accounts
    fn accounts(&self) -> Vec<FeeAccount> {
        self.iter()
            .unique_by(|entry| &entry.fee_account)
            .map(|entry| entry.fee_account)
            .collect()
    }
}

impl From<BuilderFee<SeqTypes>> for FeeInfo {
    fn from(fee: BuilderFee<SeqTypes>) -> Self {
        Self {
            amount: fee.fee_amount.into(),
            account: fee.fee_account,
        }
    }
}

impl From<Deposit> for FeeInfo {
    fn from(item: Deposit) -> Self {
        Self {
            amount: item.amount.into(),
            account: item.user.into(),
        }
    }
}

impl Committable for FeeInfo {
    fn commit(&self) -> Commitment<Self> {
        RawCommitmentBuilder::new(&Self::tag())
            .fixed_size_field("account", &self.account.to_fixed_bytes())
            .fixed_size_field("amount", &self.amount.to_fixed_bytes())
            .finalize()
    }
    fn tag() -> String {
        "FEE_INFO".into()
    }
}

impl_serde_from_string_or_integer!(FeeAmount);
impl_to_fixed_bytes!(FeeAmount, U256);

impl From<u64> for FeeAmount {
    fn from(amt: u64) -> Self {
        Self(U256::from(amt))
    }
}

impl From<FeeAmount> for MonetaryValue {
    fn from(value: FeeAmount) -> Self {
        MonetaryValue::eth(value.0.to::<u128>() as i128)
    }
}

impl CheckedSub for FeeAmount {
    fn checked_sub(&self, v: &Self) -> Option<Self> {
        self.0.checked_sub(v.0).map(FeeAmount)
    }
}

impl FromStr for FeeAmount {
    type Err = <U256 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl FromStringOrInteger for FeeAmount {
    type Binary = ethers_core::types::U256;
    type Integer = u64;

    fn from_binary(b: Self::Binary) -> anyhow::Result<Self> {
        Ok(Self(U256::from_limbs(b.0)))
    }

    fn from_integer(i: Self::Integer) -> anyhow::Result<Self> {
        Ok(i.into())
    }

    fn from_string(s: String) -> anyhow::Result<Self> {
        // For backwards compatibility, we have an ad hoc parser for WEI amounts represented as hex
        // strings.
        if let Some(s) = s.strip_prefix("0x") {
            return Ok(Self(U256::from_str_radix(s, 16)?));
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
        Ok(ethers_core::types::U256(self.0.into_limbs()))
    }

    fn to_string(&self) -> anyhow::Result<String> {
        Ok(format!("{self}"))
    }
}

impl FeeAmount {
    pub fn as_u64(&self) -> Option<u64> {
        if self.0 <= U256::from(u64::MAX) {
            Some(self.0.to::<u64>())
        } else {
            None
        }
    }
}
impl FeeAccount {
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

impl FromStr for FeeAccount {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Valid for FeeAmount {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl Valid for FeeAccount {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl CanonicalSerialize for FeeAmount {
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
impl CanonicalDeserialize for FeeAmount {
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
impl CanonicalSerialize for FeeAccount {
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
impl CanonicalDeserialize for FeeAccount {
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

impl ToTraversalPath<256> for FeeAccount {
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
impl FeeAccountProof {
    pub fn presence(
        pos: FeeAccount,
        proof: <FeeMerkleTree as MerkleTreeScheme>::MembershipProof,
    ) -> Self {
        Self {
            account: pos.into(),
            proof: FeeMerkleProof::Presence(proof),
        }
    }

    pub fn absence(
        pos: FeeAccount,
        proof: <FeeMerkleTree as UniversalMerkleTreeScheme>::NonMembershipProof,
    ) -> Self {
        Self {
            account: pos.into(),
            proof: FeeMerkleProof::Absence(proof),
        }
    }

    pub fn prove(tree: &FeeMerkleTree, account: Address) -> Option<(Self, U256)> {
        match tree.universal_lookup(FeeAccount(account)) {
            LookupResult::Ok(balance, proof) => Some((
                Self {
                    account,
                    proof: FeeMerkleProof::Presence(proof),
                },
                balance.0,
            )),
            LookupResult::NotFound(proof) => Some((
                Self {
                    account,
                    proof: FeeMerkleProof::Absence(proof),
                },
                U256::ZERO,
            )),
            LookupResult::NotInMemory => None,
        }
    }

    pub fn verify(&self, comm: &FeeMerkleCommitment) -> anyhow::Result<U256> {
        match &self.proof {
            FeeMerkleProof::Presence(proof) => {
                ensure!(
                    FeeMerkleTree::verify(comm.digest(), FeeAccount(self.account), proof)?.is_ok(),
                    "invalid proof"
                );
                Ok(proof
                    .elem()
                    .context("presence proof is missing account balance")?
                    .0)
            },
            FeeMerkleProof::Absence(proof) => {
                let tree = FeeMerkleTree::from_commitment(comm);
                ensure!(
                    tree.non_membership_verify(FeeAccount(self.account), proof)?,
                    "invalid proof"
                );
                Ok(U256::ZERO)
            },
        }
    }

    pub fn remember(&self, tree: &mut FeeMerkleTree) -> anyhow::Result<()> {
        match &self.proof {
            FeeMerkleProof::Presence(proof) => {
                tree.remember(
                    FeeAccount(self.account),
                    proof
                        .elem()
                        .context("presence proof is missing account balance")?,
                    proof,
                )?;
                Ok(())
            },
            FeeMerkleProof::Absence(proof) => {
                tree.non_membership_remember(FeeAccount(self.account), proof)?;
                Ok(())
            },
        }
    }
}

impl From<(FeeAccountProof, U256)> for AccountQueryData {
    fn from((proof, balance): (FeeAccountProof, U256)) -> Self {
        Self { balance, proof }
    }
}

/// Get a partial snapshot of the given fee state, which contains only the specified accounts.
///
/// Fails if one of the requested accounts is not represented in the original `state`.
pub fn retain_accounts(
    state: &FeeMerkleTree,
    accounts: impl IntoIterator<Item = FeeAccount>,
) -> anyhow::Result<FeeMerkleTree> {
    let mut snapshot = FeeMerkleTree::from_commitment(state.commitment());
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

#[cfg(test)]
mod test {
    use super::{Address, IterableFeeInfo};
    use crate::{FeeAccount, FeeAmount, FeeInfo};

    #[test]
    fn test_iterable_fee_info() {
        let addr = Address::default();
        let fee = FeeInfo::new(addr, FeeAmount::from(1));
        let fees = vec![fee, fee, fee];
        // check the sum of amounts
        let sum = fees.amount().unwrap();
        assert_eq!(FeeAmount::from(3), sum);

        // check accounts collector
        let accounts = fees.accounts();
        assert_eq!(vec![FeeAccount::from(Address::default())], accounts);
    }
}
