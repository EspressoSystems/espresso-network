use ethers::{
    core::k256::ecdsa::{SigningKey, VerifyingKey},
    signers::{
        coins_bip39::{English, Mnemonic},
        LocalWallet, WalletError,
    },
    types::{Address, Signature},
    utils::public_key_to_address,
};
use hotshot_types::traits::signature_key::BuilderSignatureKey;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use std::{
    fmt::{Display, Formatter},
    hash::Hash,
};

use crate::state::FeeAccount;

// Newtype because type doesn't implement Hash, Display, SerDe, Ord, PartialOrd
#[derive(PartialEq, Eq, Clone)]
pub struct EthKeyPair {
    signing_key: SigningKey,
    fee_account: FeeAccount,
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

impl EthKeyPair {
    pub fn from_mnemonic(
        phrase: impl AsRef<str>,
        index: impl Into<u32>,
    ) -> Result<Self, WalletError> {
        let index: u32 = index.into();
        let mnemonic = Mnemonic::<English>::new_from_phrase(phrase.as_ref())?;
        let derivation_path = format!("m/44'/60'/0'/0/{index}");
        let derived_priv_key =
            mnemonic.derive_key(derivation_path.as_str(), /* password */ None)?;
        let signing_key: &SigningKey = derived_priv_key.as_ref();
        Ok(signing_key.clone().into())
    }

    pub fn fee_account(&self) -> FeeAccount {
        self.fee_account
    }

    pub fn address(&self) -> Address {
        self.fee_account.address()
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

#[derive(Clone, Debug, Snafu)]
pub struct SigningError;

impl BuilderSignatureKey for FeeAccount {
    type BuilderPrivateKey = EthKeyPair;
    type BuilderSignature = Signature;
    type SignError = SigningError;

    fn validate_builder_signature(&self, signature: &Self::BuilderSignature, data: &[u8]) -> bool {
        signature.verify(data, self.address()).is_ok()
    }

    fn sign_builder_message(
        private_key: &Self::BuilderPrivateKey,
        data: &[u8],
    ) -> Result<Self::BuilderSignature, Self::SignError> {
        let wallet = LocalWallet::from_bytes(&private_key.signing_key.to_bytes()).unwrap();
        let message_hash = ethers::utils::hash_message(data);
        wallet.sign_hash(message_hash).map_err(|_| SigningError)
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
    use super::*;
    use hotshot_types::traits::signature_key::BuilderSignatureKey;

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
            "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"
                .parse()
                .unwrap()
        );
        let key1 = EthKeyPair::from_mnemonic(mnemonic, 1u32).unwrap();
        assert_eq!(
            key1.address(),
            "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
                .parse()
                .unwrap()
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

        // Recovery fails if signed with other key
        let other_key = FeeAccount::generated_from_seed_indexed([0u8; 32], 1).1;
        let sig = FeeAccount::sign_builder_message(&other_key, msg).unwrap();
        assert!(!key.fee_account().validate_builder_signature(&sig, msg));

        // Recovery fails if another message was signed
        let sig = FeeAccount::sign_builder_message(&key, b"hello world XYZ").unwrap();
        assert!(!key.fee_account().validate_builder_signature(&sig, msg));
    }
}
