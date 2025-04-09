use std::{
    fmt::{Display, Formatter},
    hash::Hash,
};

use alloy::{
    primitives::{Address, PrimitiveSignature},
    signers::{
        self,
        k256::ecdsa::{SigningKey, VerifyingKey},
        local::{
            coins_bip39::{English, Mnemonic},
            PrivateKeySigner,
        },
        utils::public_key_to_address,
        SignerSync,
    },
};
use alloy_compat::ethers_serde;
use derive_more::*;
use hotshot_types::traits::signature_key::{BuilderSignatureKey, PrivateSignatureKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::FeeAccount;

// Newtype because type doesn't implement Hash, Display, SerDe, Ord, PartialOrd
#[derive(PartialEq, Eq, Clone)]
pub struct EthKeyPair {
    signing_key: SigningKey,
    fee_account: FeeAccount,
}

impl TryFrom<&tagged_base64::TaggedBase64> for EthKeyPair {
    type Error = tagged_base64::Tb64Error;

    fn try_from(value: &tagged_base64::TaggedBase64) -> Result<Self, Self::Error> {
        // Make sure the tag is correct
        if value.tag() != "ETH_KEY_PAIR" {
            return Err(tagged_base64::Tb64Error::InvalidTag);
        }

        // Convert the bytes to a signing key
        let bytes = value.value();
        let signing_key =
            SigningKey::from_slice(&bytes).map_err(|_| tagged_base64::Tb64Error::InvalidData)?;

        // Convert the signing key to an EthKeyPair
        Ok(signing_key.into())
    }
}

impl From<SigningKey> for EthKeyPair {
    fn from(signing_key: SigningKey) -> Self {
        let fee_account = public_key_to_address(&VerifyingKey::from(&signing_key)).into();
        EthKeyPair {
            signing_key,
            fee_account,
        }
    }
}

impl PrivateSignatureKey for EthKeyPair {
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let signing_key =
            SigningKey::from_slice(bytes).map_err(|_| tagged_base64::Tb64Error::InvalidData)?;

        Ok(signing_key.into())
    }

    fn to_bytes(&self) -> Vec<u8> {
        self.signing_key.to_bytes().to_vec()
    }

    fn to_tagged_base64(&self) -> Result<tagged_base64::TaggedBase64, tagged_base64::Tb64Error> {
        tagged_base64::TaggedBase64::new("ETH_KEY_PAIR", &self.signing_key.to_bytes())
    }
}

impl EthKeyPair {
    pub fn from_mnemonic(
        phrase: impl AsRef<str>,
        index: impl Into<u32>,
    ) -> Result<Self, signers::local::LocalSignerError> {
        let index: u32 = index.into();
        let mnemonic = Mnemonic::<English>::new_from_phrase(phrase.as_ref())?;
        let derivation_path = format!("m/44'/60'/0'/0/{index}");
        let derived_priv_key =
            mnemonic.derive_key(derivation_path.as_str(), /* password */ None)?;
        let signing_key: &SigningKey = derived_priv_key.as_ref();
        Ok(signing_key.clone().into())
    }

    pub fn random() -> EthKeyPair {
        SigningKey::random(&mut rand::thread_rng()).into()
    }

    pub fn fee_account(&self) -> FeeAccount {
        self.fee_account
    }

    pub fn address(&self) -> Address {
        self.fee_account.address()
    }

    pub fn signer(&self) -> PrivateKeySigner {
        PrivateKeySigner::from(self.signing_key.clone())
    }
}

impl Hash for EthKeyPair {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.signing_key.to_bytes().hash(state);
    }
}

// Always display the address, not the private key
impl Display for EthKeyPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "EthKeyPair(address={:?})", self.address())
    }
}

impl std::fmt::Debug for EthKeyPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Serialize for EthKeyPair {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.signing_key.to_bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EthKeyPair {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = <[u8; 32]>::deserialize(deserializer)?;
        Ok(EthKeyPair::from(
            SigningKey::from_slice(&bytes).map_err(serde::de::Error::custom)?,
        ))
    }
}

impl PartialOrd for EthKeyPair {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EthKeyPair {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.signing_key
            .as_nonzero_scalar()
            .cmp(other.signing_key.as_nonzero_scalar())
    }
}

#[derive(self::Debug, Error)]
#[error("Failed to sign builder message")]
pub struct SigningError(#[from] signers::Error);

/// signature type for the builder
#[derive(
    self::Debug, Clone, Copy, Hash, Deref, PartialEq, Eq, From, Into, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct BuilderSignature(#[serde(with = "ethers_serde::signature")] pub PrimitiveSignature);

impl BuilderSignatureKey for FeeAccount {
    type BuilderPrivateKey = EthKeyPair;
    type BuilderSignature = BuilderSignature;
    type SignError = SigningError;

    fn validate_builder_signature(&self, signature: &Self::BuilderSignature, data: &[u8]) -> bool {
        signature.recover_address_from_msg(data).unwrap() == self.address()
    }

    fn sign_builder_message(
        private_key: &Self::BuilderPrivateKey,
        data: &[u8],
    ) -> Result<Self::BuilderSignature, Self::SignError> {
        let wallet = private_key.signer();
        let message_hash = alloy::primitives::eip191_hash_message(data);
        let sig = wallet
            .sign_hash_sync(&message_hash)
            .map_err(SigningError::from)?
            .into();
        Ok(sig)
    }

    fn generated_from_seed_indexed(seed: [u8; 32], index: u64) -> (Self, Self::BuilderPrivateKey) {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&seed);
        hasher.update(&index.to_le_bytes());
        let new_seed = *hasher.finalize().as_bytes();
        let signing_key = EthKeyPair::from(SigningKey::from_slice(&new_seed).unwrap());
        (signing_key.fee_account(), signing_key)
    }
}

#[cfg(test)]
mod tests {
    use hotshot_types::traits::signature_key::BuilderSignatureKey;

    use super::*;

    impl EthKeyPair {
        pub fn for_test() -> Self {
            FeeAccount::generated_from_seed_indexed([0u8; 32], 0).1
        }
    }

    #[test]
    fn test_fmt() {
        let key = EthKeyPair::for_test();
        let expected = "EthKeyPair(address=0xb0cfa4e5893107e2995974ef032957752bb526e9)";
        assert_eq!(format!("{}", key), expected);
        assert_eq!(format!("{:?}", key), expected);
    }

    #[test]
    fn test_derivation_from_mnemonic() {
        let mnemonic = "test test test test test test test test test test test junk";
        let key0 = EthKeyPair::from_mnemonic(mnemonic, 0u32).unwrap();
        assert_eq!(
            key0.address(),
            Address::parse_checksummed("0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266", None).unwrap()
        );
        let key1 = EthKeyPair::from_mnemonic(mnemonic, 1u32).unwrap();
        assert_eq!(
            key1.address(),
            Address::parse_checksummed("0x70997970C51812dc3A010C7d01b50e0d17dc79C8", None).unwrap()
        );
    }

    #[test]
    fn test_key_serde() {
        let key = EthKeyPair::for_test();
        let serialized = bincode::serialize(&key).unwrap();
        let deserialized: EthKeyPair = bincode::deserialize(&serialized).unwrap();
        assert_eq!(key, deserialized);
    }

    #[test]
    fn test_signing_and_recovery() {
        // Recovery works
        let key = EthKeyPair::for_test();
        let msg = b"hello world";
        let sig = FeeAccount::sign_builder_message(&key, msg).unwrap();
        assert!(key.fee_account().validate_builder_signature(&sig, msg));

        // Validation fails if signed with other key.
        let other_key = FeeAccount::generated_from_seed_indexed([0u8; 32], 1).1;
        let sig = FeeAccount::sign_builder_message(&other_key, msg).unwrap();
        assert!(!key.fee_account().validate_builder_signature(&sig, msg));

        // Validation fails if another message was signed
        let sig = FeeAccount::sign_builder_message(&key, b"hello world XYZ").unwrap();
        assert!(!key.fee_account().validate_builder_signature(&sig, msg));
    }
}
