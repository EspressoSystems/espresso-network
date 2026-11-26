//! SLH-DSA (FIPS 205) keys for post-quantum signatures.
//!
//! Proof of concept: the parameter set is SLH-DSA-SHA2-128s until the final set is chosen.

use std::fmt;

use fips205::{
    slh_dsa_sha2_128s as params,
    traits::{KeyGen, SerDes, Signer, Verifier},
};
use serde::{Deserialize, Serialize};

/// Length of an SLH-DSA public key in bytes.
pub const PUBLIC_KEY_LEN: usize = params::PK_LEN;

/// Length of an SLH-DSA signature in bytes.
pub const SIGNATURE_LEN: usize = params::SIG_LEN;

/// An SLH-DSA public key.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PublicKey([u8; PUBLIC_KEY_LEN]);

/// An SLH-DSA signature.
#[derive(Clone, PartialEq, Eq)]
pub struct Signature(Box<[u8; SIGNATURE_LEN]>);

/// An SLH-DSA key pair.
pub struct Keypair {
    public: PublicKey,
    secret: params::PrivateKey,
}

/// Errors from parsing SLH-DSA keys and signatures, and from verification.
#[derive(Debug, thiserror::Error)]
pub enum SlhDsaError {
    #[error("SLH-DSA public key must be {PUBLIC_KEY_LEN} bytes, got {0}")]
    PublicKeyLength(usize),
    #[error("SLH-DSA signature must be {SIGNATURE_LEN} bytes, got {0}")]
    SignatureLength(usize),
    #[error("invalid SLH-DSA signature")]
    InvalidSignature,
}

impl Keypair {
    /// A new random key pair.
    pub fn generate() -> Self {
        let (public, secret) =
            params::KG::try_keygen().expect("the OS random number generator works");
        Self {
            public: PublicKey(public.into_bytes()),
            secret,
        }
    }

    pub fn public_key(&self) -> PublicKey {
        self.public
    }

    /// Sign `message` with the FIPS 205 domain separation string `context`.
    ///
    /// # Panics
    ///
    /// If `context` is longer than 255 bytes, or the OS random number generator fails.
    pub fn sign(&self, context: &[u8], message: &[u8]) -> Signature {
        let signature = self
            .secret
            .try_sign(message, context, true)
            .expect("the context is at most 255 bytes and the OS random number generator works");
        Signature(Box::new(signature))
    }
}

impl PublicKey {
    /// Check `signature` over `message` with the domain separation string `context`.
    pub fn verify(
        &self,
        context: &[u8],
        message: &[u8],
        signature: &Signature,
    ) -> Result<(), SlhDsaError> {
        // Any byte string of the right length decodes as a public key.
        let key =
            params::PublicKey::try_from_bytes(self.0).map_err(|_| SlhDsaError::InvalidSignature)?;
        if key.verify(message, &signature.0, context) {
            Ok(())
        } else {
            Err(SlhDsaError::InvalidSignature)
        }
    }

    pub fn as_bytes(&self) -> &[u8; PUBLIC_KEY_LEN] {
        &self.0
    }
}

impl TryFrom<&[u8]> for PublicKey {
    type Error = SlhDsaError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let key = bytes
            .try_into()
            .map_err(|_| SlhDsaError::PublicKeyLength(bytes.len()))?;
        Ok(Self(key))
    }
}

impl Signature {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl TryFrom<&[u8]> for Signature {
    type Error = SlhDsaError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let signature = Box::<[u8; SIGNATURE_LEN]>::try_from(bytes.to_vec().into_boxed_slice())
            .map_err(|_| SlhDsaError::SignatureLength(bytes.len()))?;
        Ok(Self(signature))
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SlhDsaPublicKey(0x")?;
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        write!(f, ")")
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SlhDsaSignature({} bytes)", self.0.len())
    }
}

impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlhDsaKeypair")
            .field("public", &self.public)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTEXT: &[u8] = b"slh-dsa test context";
    const MESSAGE: &[u8] = b"slh-dsa test message";

    #[test]
    fn generate_gives_distinct_key_pairs() {
        let keypair = Keypair::generate();
        let other = Keypair::generate();

        assert_eq!(keypair.public_key(), keypair.public_key());
        assert_ne!(keypair.public_key(), other.public_key());
    }

    /// A signature verifies with the public key of the signing key pair, also after a round trip
    /// through its bytes.
    #[test]
    fn verify_accepts_signature_from_matching_key() {
        let keypair = Keypair::generate();
        let public_key = keypair.public_key();
        let signature = keypair.sign(CONTEXT, MESSAGE);

        assert!(public_key.verify(CONTEXT, MESSAGE, &signature).is_ok());

        let bytes = signature.as_bytes().to_vec();
        assert_eq!(bytes.len(), SIGNATURE_LEN);
        let parsed = Signature::try_from(bytes.as_slice()).map_err(|e| e.to_string());
        assert!(parsed.is_ok(), "{:?}", parsed.err());
        let parsed = parsed.unwrap();
        assert!(parsed == signature);
        assert!(public_key.verify(CONTEXT, MESSAGE, &parsed).is_ok());
    }

    #[test]
    fn verify_rejects_other_key_context_message_or_signature() {
        let keypair = Keypair::generate();
        let public_key = keypair.public_key();
        let signature = keypair.sign(CONTEXT, MESSAGE);
        assert!(public_key.verify(CONTEXT, MESSAGE, &signature).is_ok());

        let other_key = Keypair::generate().public_key();
        assert_invalid(other_key.verify(CONTEXT, MESSAGE, &signature));
        assert_invalid(public_key.verify(b"other context", MESSAGE, &signature));
        assert_invalid(public_key.verify(b"", MESSAGE, &signature));
        assert_invalid(public_key.verify(CONTEXT, b"other message", &signature));
        assert_invalid(public_key.verify(CONTEXT, b"", &signature));

        for index in [0, SIGNATURE_LEN / 2, SIGNATURE_LEN - 1] {
            let mut bytes = signature.as_bytes().to_vec();
            bytes[index] ^= 0x01;
            let tampered = Signature::try_from(bytes.as_slice())
                .map_err(|e| e.to_string())
                .expect("a signature of the right length parses");
            assert_invalid(public_key.verify(CONTEXT, MESSAGE, &tampered));
        }
    }

    #[test]
    fn public_key_from_bytes_round_trips() {
        let bytes: [u8; PUBLIC_KEY_LEN] = std::array::from_fn(|i| i as u8);

        let key = PublicKey::try_from(bytes.as_slice()).map_err(|e| e.to_string());
        assert!(key.is_ok(), "{:?}", key.err());
        assert_eq!(key.unwrap().as_bytes(), &bytes);
    }

    #[test]
    fn public_key_from_bytes_rejects_wrong_length() {
        for len in [0, 1, PUBLIC_KEY_LEN - 1, PUBLIC_KEY_LEN + 1, 64] {
            let bytes = vec![7u8; len];
            let result = PublicKey::try_from(bytes.as_slice()).map(|_| ());
            assert!(
                matches!(result, Err(SlhDsaError::PublicKeyLength(n)) if n == len),
                "length {len}: {result:?}"
            );
        }
    }

    #[test]
    fn signature_from_bytes_round_trips() {
        let bytes: Vec<u8> = (0..SIGNATURE_LEN).map(|i| (i % 251) as u8).collect();

        let signature = Signature::try_from(bytes.as_slice()).map_err(|e| e.to_string());
        assert!(signature.is_ok(), "{:?}", signature.err());
        assert_eq!(signature.unwrap().as_bytes(), bytes.as_slice());
    }

    #[test]
    fn signature_from_bytes_rejects_wrong_length() {
        for len in [0, PUBLIC_KEY_LEN, SIGNATURE_LEN - 1, SIGNATURE_LEN + 1] {
            let bytes = vec![7u8; len];
            let result = Signature::try_from(bytes.as_slice()).map(|_| ());
            assert!(
                matches!(result, Err(SlhDsaError::SignatureLength(n)) if n == len),
                "length {len}: {result:?}"
            );
        }
    }

    #[test]
    fn public_key_serde_round_trip() {
        let key = Keypair::generate().public_key();

        let encoded = bincode::serialize(&key).unwrap();
        let decoded: PublicKey = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded, key);

        let json = serde_json::to_string(&key).unwrap();
        let decoded: PublicKey = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, key);
    }

    fn assert_invalid(result: Result<(), SlhDsaError>) {
        assert!(
            matches!(result, Err(SlhDsaError::InvalidSignature)),
            "{result:?}"
        );
    }
}
