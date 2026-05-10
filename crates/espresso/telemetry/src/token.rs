use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, anyhow};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use jf_signature::{
    SignatureScheme,
    bls_over_bn254::{BLSOverBN254CurveSignatureScheme, SignKey, Signature, VerKey},
};
use serde::{Deserialize, Serialize};
use tagged_base64::TaggedBase64;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TokenParseError {
    #[error("invalid JWT format, expected header.payload.signature")]
    InvalidFormat,
    #[error("unsupported JWT: alg={alg}, typ={typ}")]
    UnsupportedAlgorithm { alg: String, typ: String },
    #[error("invalid header: {0}")]
    InvalidHeader(String),
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
    #[error("invalid signature: {0}")]
    InvalidSignature(String),
}

#[derive(Debug, Error)]
pub enum TokenVerifyError {
    #[error("BLS signature verification failed")]
    InvalidSignature,
    #[error("token timestamp is in the future")]
    FutureTimestamp,
    #[error("token expired: age {age}s exceeds max {max_age}s")]
    Expired { age: u64, max_age: u64 },
}

/// JWT header for BLS-BN254 tokens.
#[derive(Serialize, Deserialize)]
struct Header {
    alg: String,
    typ: String,
}

/// JWT payload with standard claims.
#[derive(Serialize, Deserialize)]
struct Payload {
    sub: String,
    iat: u64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    node_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    company_name: Option<String>,
}

/// Parsed but unverified JWT token with BLS-BN254 signature.
pub struct UnauthenticatedToken {
    pubkey: VerKey,
    payload: Payload,
    signing_input: String,
    signature: Signature,
}

impl UnauthenticatedToken {
    pub fn pubkey_str(&self) -> &str {
        &self.payload.sub
    }

    pub fn iat(&self) -> u64 {
        self.payload.iat
    }

    pub fn node_name(&self) -> Option<&str> {
        self.payload.node_name.as_deref()
    }

    pub fn company_name(&self) -> Option<&str> {
        self.payload.company_name.as_deref()
    }
}

/// A token whose BLS signature and timestamp have been verified.
/// Only constructable via `UnauthenticatedToken::verify`.
#[derive(Debug)]
pub struct Token {
    pubkey_str: String,
    node_name: Option<String>,
    company_name: Option<String>,
}

impl Token {
    pub fn pubkey_str(&self) -> &str {
        &self.pubkey_str
    }

    pub fn node_name(&self) -> Option<&str> {
        self.node_name.as_deref()
    }

    pub fn company_name(&self) -> Option<&str> {
        self.company_name.as_deref()
    }
}

impl UnauthenticatedToken {
    /// Generate a JWT: sign the current timestamp with the BLS key.
    pub fn generate(signing_key: &SignKey) -> anyhow::Result<Self> {
        Self::generate_with(signing_key, None, None)
    }

    /// Generate a JWT with optional `node_name` and `company_name` claims.
    pub fn generate_with(
        signing_key: &SignKey,
        node_name: Option<&str>,
        company_name: Option<&str>,
    ) -> anyhow::Result<Self> {
        let pubkey = VerKey::from(signing_key);
        let pubkey_str = TaggedBase64::from(&pubkey).to_string();
        let iat = now_unix_secs();

        let header = Header {
            alg: "BLS-BN254".to_string(),
            typ: "JWT".to_string(),
        };
        let payload = Payload {
            sub: pubkey_str,
            iat,
            node_name: node_name.map(str::to_owned),
            company_name: company_name.map(str::to_owned),
        };

        let signing_input = format!(
            "{}.{}",
            B64.encode(serde_json::to_vec(&header)?),
            B64.encode(serde_json::to_vec(&payload)?)
        );

        let signature = BLSOverBN254CurveSignatureScheme::sign(
            &(),
            signing_key,
            signing_input.as_bytes(),
            &mut rand::thread_rng(),
        )?;

        Ok(Self {
            pubkey,
            payload,
            signing_input,
            signature,
        })
    }

    /// Parse a JWT string (header.payload.signature).
    pub fn parse(s: &str) -> Result<Self, TokenParseError> {
        let parts: Vec<&str> = s.splitn(3, '.').collect();
        if parts.len() != 3 {
            return Err(TokenParseError::InvalidFormat);
        }

        let header_bytes = B64
            .decode(parts[0])
            .map_err(|e| TokenParseError::InvalidHeader(e.to_string()))?;
        let header: Header = serde_json::from_slice(&header_bytes)
            .map_err(|e| TokenParseError::InvalidHeader(e.to_string()))?;
        if header.alg != "BLS-BN254" || header.typ != "JWT" {
            return Err(TokenParseError::UnsupportedAlgorithm {
                alg: header.alg,
                typ: header.typ,
            });
        }

        let payload_bytes = B64
            .decode(parts[1])
            .map_err(|e| TokenParseError::InvalidPayload(e.to_string()))?;
        let payload: Payload = serde_json::from_slice(&payload_bytes)
            .map_err(|e| TokenParseError::InvalidPayload(e.to_string()))?;

        let pubkey: VerKey = TaggedBase64::parse(&payload.sub)
            .map_err(|e| TokenParseError::InvalidPayload(e.to_string()))?
            .try_into()
            .map_err(|_| TokenParseError::InvalidPayload("invalid BLS public key".into()))?;

        let sig_bytes = B64
            .decode(parts[2])
            .map_err(|e| TokenParseError::InvalidSignature(e.to_string()))?;
        let signature = Signature::deserialize_compressed(&sig_bytes[..])
            .map_err(|e| TokenParseError::InvalidSignature(e.to_string()))?;

        let signing_input = format!("{}.{}", parts[0], parts[1]);

        Ok(Self {
            pubkey,
            payload,
            signing_input,
            signature,
        })
    }

    /// Encode as JWT string (header.payload.signature).
    pub fn encode(&self) -> String {
        let mut sig_bytes = Vec::new();
        self.signature
            .serialize_compressed(&mut sig_bytes)
            .expect("signature serialization should not fail");

        format!("{}.{}", self.signing_input, B64.encode(&sig_bytes))
    }

    /// Verify signature and timestamp, returning an authenticated Token on success.
    pub fn verify(self, max_age_secs: u64) -> Result<Token, TokenVerifyError> {
        BLSOverBN254CurveSignatureScheme::verify(
            &(),
            &self.pubkey,
            self.signing_input.as_bytes(),
            &self.signature,
        )
        .map_err(|_| TokenVerifyError::InvalidSignature)?;

        let now = now_unix_secs();
        if self.payload.iat > now + 60 {
            return Err(TokenVerifyError::FutureTimestamp);
        }
        let age = now.saturating_sub(self.payload.iat);
        if age > max_age_secs {
            return Err(TokenVerifyError::Expired {
                age,
                max_age: max_age_secs,
            });
        }

        Ok(Token {
            pubkey_str: self.payload.sub,
            node_name: self.payload.node_name,
            company_name: self.payload.company_name,
        })
    }
}

pub fn parse_bls_signing_key(s: &str) -> anyhow::Result<SignKey> {
    let tb = TaggedBase64::parse(s.trim()).context("invalid tagged-base64")?;
    let key: SignKey = tb
        .try_into()
        .map_err(|_| anyhow!("failed to convert tagged-base64 to SignKey"))?;
    Ok(key)
}

pub fn load_bls_signing_key(path: &Path) -> anyhow::Result<SignKey> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read BLS key file: {}", path.display()))?;
    let key_str = contents
        .lines()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| anyhow!("BLS key file is empty"))?;
    parse_bls_signing_key(key_str)
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_signing_key() -> SignKey {
        BLSOverBN254CurveSignatureScheme::key_gen(&(), &mut rand::thread_rng())
            .unwrap()
            .0
    }

    #[test]
    fn roundtrip_encode_parse() {
        let sk = gen_signing_key();
        let token = UnauthenticatedToken::generate(&sk).unwrap();
        let encoded = token.encode();

        assert_eq!(encoded.matches('.').count(), 2);

        let parsed = UnauthenticatedToken::parse(&encoded).unwrap();
        let authed = parsed.verify(60).unwrap();
        assert!(!authed.pubkey_str().is_empty());
    }

    #[test]
    fn jwt_header_contains_alg() {
        let sk = gen_signing_key();
        let token = UnauthenticatedToken::generate(&sk).unwrap();
        let encoded = token.encode();
        let header_b64 = encoded.split('.').next().unwrap();
        let header_json = B64.decode(header_b64).unwrap();
        let header: serde_json::Value = serde_json::from_slice(&header_json).unwrap();
        assert_eq!(header["alg"], "BLS-BN254");
        assert_eq!(header["typ"], "JWT");
    }

    #[test]
    fn verify_valid_token() {
        let sk = gen_signing_key();
        let token = UnauthenticatedToken::generate(&sk).unwrap();
        token.verify(60).unwrap();
    }

    fn sign_jwt(sk: &SignKey, iat: u64) -> String {
        let pubkey_str = TaggedBase64::from(&VerKey::from(sk)).to_string();
        let header = Header {
            alg: "BLS-BN254".into(),
            typ: "JWT".into(),
        };
        let payload = Payload {
            sub: pubkey_str,
            iat,
            node_name: None,
            company_name: None,
        };
        let signing_input = format!(
            "{}.{}",
            B64.encode(serde_json::to_vec(&header).unwrap()),
            B64.encode(serde_json::to_vec(&payload).unwrap())
        );
        let sig = BLSOverBN254CurveSignatureScheme::sign(
            &(),
            sk,
            signing_input.as_bytes(),
            &mut rand::thread_rng(),
        )
        .unwrap();
        let mut sig_bytes = Vec::new();
        sig.serialize_compressed(&mut sig_bytes).unwrap();
        format!("{}.{}", signing_input, B64.encode(&sig_bytes))
    }

    #[test]
    fn verify_expired_token() {
        let sk = gen_signing_key();
        let jwt = sign_jwt(&sk, 1000);
        let err = UnauthenticatedToken::parse(&jwt)
            .unwrap()
            .verify(60)
            .unwrap_err();
        assert!(matches!(err, TokenVerifyError::Expired { .. }));
    }

    #[test]
    fn verify_future_dated_token() {
        let sk = gen_signing_key();
        let jwt = sign_jwt(&sk, now_unix_secs() + 10000);
        let err = UnauthenticatedToken::parse(&jwt)
            .unwrap()
            .verify(86400)
            .unwrap_err();
        assert!(matches!(err, TokenVerifyError::FutureTimestamp));
    }

    #[test]
    fn parse_invalid_format() {
        assert!(matches!(
            UnauthenticatedToken::parse("garbage"),
            Err(TokenParseError::InvalidFormat)
        ));
        assert!(matches!(
            UnauthenticatedToken::parse("only.two"),
            Err(TokenParseError::InvalidFormat)
        ));
    }

    #[test]
    fn generate_without_claims_payload_is_byte_compat() {
        // Decode the payload segment and check it has only `sub` + `iat`.
        // This proves omitted claims aren't serialized, so existing tokens
        // remain bytewise compatible.
        let sk = gen_signing_key();
        let token = UnauthenticatedToken::generate(&sk).unwrap();
        let encoded = token.encode();
        let payload_b64 = encoded.split('.').nth(1).unwrap();
        let payload_bytes = B64.decode(payload_b64).unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).unwrap();
        let obj = payload.as_object().unwrap();
        assert_eq!(obj.len(), 2, "expected only sub+iat, got {obj:?}");
        assert!(obj.contains_key("sub"));
        assert!(obj.contains_key("iat"));
    }

    #[test]
    fn generate_with_both_claims_roundtrip() {
        let sk = gen_signing_key();
        let token =
            UnauthenticatedToken::generate_with(&sk, Some("node-01"), Some("acme")).unwrap();
        let encoded = token.encode();

        let parsed = UnauthenticatedToken::parse(&encoded).unwrap();
        let authed = parsed.verify(60).unwrap();
        assert_eq!(authed.node_name(), Some("node-01"));
        assert_eq!(authed.company_name(), Some("acme"));
    }

    #[test]
    fn generate_with_node_name_only() {
        let sk = gen_signing_key();
        let token = UnauthenticatedToken::generate_with(&sk, Some("node-42"), None).unwrap();
        let encoded = token.encode();

        let payload_b64 = encoded.split('.').nth(1).unwrap();
        let payload_bytes = B64.decode(payload_b64).unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).unwrap();
        let obj = payload.as_object().unwrap();
        assert!(!obj.contains_key("company_name"));

        let authed = UnauthenticatedToken::parse(&encoded)
            .unwrap()
            .verify(60)
            .unwrap();
        assert_eq!(authed.node_name(), Some("node-42"));
        assert_eq!(authed.company_name(), None);
    }
}
