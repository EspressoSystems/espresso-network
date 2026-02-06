use std::{cmp::Ordering, fmt};

use ed25519_compact::x25519;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Keypair {
    pair: x25519::KeyPair,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct PublicKey {
    key: x25519::PublicKey,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SecretKey {
    key: x25519::SecretKey,
}

impl Keypair {
    pub fn generate() -> Result<Self, InvalidKeypair> {
        let pair = x25519::KeyPair::generate();
        if pair.validate().is_err() {
            return Err(InvalidKeypair(()));
        }
        Ok(Self { pair })
    }

    pub fn public_key(&self) -> PublicKey {
        PublicKey { key: self.pair.pk }
    }

    pub fn secret_key(&self) -> SecretKey {
        SecretKey {
            key: self.pair.sk.clone(),
        }
    }
}

impl PublicKey {
    pub fn as_bytes(&self) -> [u8; 32] {
        *self.key
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.key[..]
    }
}

impl SecretKey {
    pub fn public_key(&self) -> PublicKey {
        let key = self.key.recover_public_key().expect("valid public key");
        PublicKey { key }
    }

    pub fn as_bytes(&self) -> [u8; 32] {
        *self.key
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.key[..]
    }
}

impl From<SecretKey> for Keypair {
    fn from(k: SecretKey) -> Self {
        let p = k.public_key();
        Self {
            pair: x25519::KeyPair {
                sk: k.key,
                pk: p.key,
            },
        }
    }
}

impl From<SecretKey> for PublicKey {
    fn from(k: SecretKey) -> Self {
        k.public_key()
    }
}

impl Ord for PublicKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key[..].cmp(&other.key[..])
    }
}

impl PartialOrd for PublicKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let key = x25519::PublicKey::from_slice(value).map_err(|_| InvalidPublicKey(()))?;
        Ok(Self { key })
    }
}

impl TryFrom<&[u8]> for SecretKey {
    type Error = InvalidSecretKey;

    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        let k = x25519::SecretKey::from_slice(s).map_err(|_| InvalidSecretKey(()))?;
        if k.recover_public_key().is_err() {
            return Err(InvalidSecretKey(()));
        }
        Ok(Self { key: k })
    }
}

impl TryFrom<&str> for PublicKey {
    type Error = InvalidPublicKey;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        bs58::decode(s)
            .into_vec()
            .map_err(|_| InvalidPublicKey(()))
            .and_then(|v| PublicKey::try_from(v.as_slice()))
    }
}

impl TryFrom<&str> for SecretKey {
    type Error = InvalidSecretKey;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        bs58::decode(s)
            .into_vec()
            .map_err(|_| InvalidSecretKey(()))
            .and_then(|v| SecretKey::try_from(v.as_slice()))
    }
}

impl From<[u8; 32]> for SecretKey {
    fn from(bytes: [u8; 32]) -> Self {
        SecretKey {
            key: x25519::SecretKey::new(bytes),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid keypair")]
pub struct InvalidKeypair(());

#[derive(Debug, thiserror::Error)]
#[error("invalid secret key")]
pub struct InvalidSecretKey(());

#[derive(Debug, thiserror::Error)]
#[error("invalid public key")]
pub struct InvalidPublicKey(());
