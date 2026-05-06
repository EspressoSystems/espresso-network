use std::fmt;

use cliquenet::x25519::{InvalidKeypair, InvalidPublicKey, InvalidSecretKey};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use tagged_base64::{TaggedBase64, Tb64Error};

use crate::traits::signature_key::{PrivateSignatureKey, SignatureKey};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Keypair(cliquenet::x25519::Keypair);

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PublicKey(cliquenet::x25519::PublicKey);

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretKey(cliquenet::x25519::SecretKey);

impl Keypair {
    pub fn generate() -> Result<Self, InvalidKeypair> {
        cliquenet::x25519::Keypair::generate().map(Self)
    }

    pub fn generated_from_seed_indexed(seed: [u8; 32], index: u64) -> Result<Self, InvalidKeypair> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&seed);
        hasher.update(&index.to_be_bytes());
        let mut rng = ChaCha20Rng::from_seed(*hasher.finalize().as_bytes());
        let seed: [u8; 32] = rng.r#gen();
        cliquenet::x25519::Keypair::from_seed(seed).map(Self)
    }

    pub fn derive_from<K: SignatureKey>(k: &K::PrivateKey) -> Result<Self, InvalidSecretKey> {
        let seed = blake3::derive_key("signing key -> x25519 key", &k.to_bytes());
        let skey = SecretKey::try_from(seed)?;
        Ok(skey.into())
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.public_key())
    }

    pub fn secret_key(&self) -> SecretKey {
        SecretKey(self.0.secret_key())
    }
}

impl PublicKey {
    pub fn as_bytes(&self) -> [u8; 32] {
        self.0.as_bytes()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl SecretKey {
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.public_key())
    }

    pub fn as_bytes(&self) -> [u8; 32] {
        self.0.as_bytes()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Keypair> for cliquenet::x25519::Keypair {
    fn from(k: Keypair) -> Self {
        k.0
    }
}

impl From<PublicKey> for cliquenet::x25519::PublicKey {
    fn from(k: PublicKey) -> Self {
        k.0
    }
}

impl From<cliquenet::x25519::PublicKey> for PublicKey {
    fn from(k: cliquenet::x25519::PublicKey) -> Self {
        Self(k)
    }
}

impl From<SecretKey> for Keypair {
    fn from(k: SecretKey) -> Self {
        Self(k.0.into())
    }
}

impl From<&SecretKey> for Keypair {
    fn from(k: &SecretKey) -> Self {
        Self::from(k.clone())
    }
}

impl From<SecretKey> for PublicKey {
    fn from(k: SecretKey) -> Self {
        k.public_key()
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretKey")
    }
}

impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Keypair")
            .field("public_key", &self.public_key())
            .field("secret_key", &"SecretKey")
            .finish()
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", bs58::encode(&self.as_bytes()).into_string())
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl TryFrom<&[u8]> for PublicKey {
    type Error = InvalidPublicKey;

    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(cliquenet::x25519::PublicKey::try_from(s)?))
    }
}

impl TryFrom<&[u8]> for SecretKey {
    type Error = InvalidSecretKey;

    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(cliquenet::x25519::SecretKey::try_from(s)?))
    }
}

impl TryFrom<[u8; 32]> for SecretKey {
    type Error = InvalidSecretKey;

    fn try_from(a: [u8; 32]) -> Result<Self, Self::Error> {
        Ok(Self(cliquenet::x25519::SecretKey::try_from(a)?))
    }
}

impl TryFrom<&str> for PublicKey {
    type Error = InvalidPublicKey;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(Self(cliquenet::x25519::PublicKey::try_from(s)?))
    }
}

impl TryFrom<&str> for SecretKey {
    type Error = InvalidSecretKey;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(Self(cliquenet::x25519::SecretKey::try_from(s)?))
    }
}

const X25519_SECRET_KEY: &str = "X25519_SK";

impl TryFrom<TaggedBase64> for SecretKey {
    type Error = Tb64Error;

    fn try_from(tb: TaggedBase64) -> Result<Self, Self::Error> {
        if tb.tag() != X25519_SECRET_KEY {
            return Err(Tb64Error::InvalidTag);
        }
        Self::try_from(tb.as_ref()).map_err(|_| Tb64Error::InvalidData)
    }
}

impl TryFrom<SecretKey> for TaggedBase64 {
    type Error = Tb64Error;

    fn try_from(k: SecretKey) -> Result<Self, Self::Error> {
        TaggedBase64::new(X25519_SECRET_KEY, &k.as_bytes()[..])
    }
}
