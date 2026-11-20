//! The records that make the API services of the validators findable, as validators sign them.
//!
//! At the start of each epoch, every validator signs a record for that epoch with the URL of its
//! API service, and the Ed25519 key that service authenticates requests with if it has one. Its
//! consensus node sends the record to every other consensus node, and API services list the
//! records they get from their consensus node. A client checks each signature against the stake
//! table of the record's epoch, so it need not trust the service that lists them.

use hotshot_types::{
    data::EpochNumber,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
};
use serde::{Deserialize, Serialize};

/// The longest API URL a record may carry, in bytes.
pub const MAX_API_URL_LEN: usize = 2048;

/// Separates what validators sign for records from everything else they sign.
const DOMAIN: &[u8] = b"espresso-service-record-v1";

/// What a validator tells the network about its API service in an epoch.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ServiceRecord {
    /// The epoch whose stake table the validator is in.
    pub epoch: EpochNumber,

    /// The URL of the validator's API service.
    pub api_url: String,

    /// The Ed25519 public key the API service authenticates requests with, if any.
    pub api_key: Option<[u8; 32]>,

    /// Orders the records of one validator in an epoch: a higher sequence number replaces a lower
    /// one.
    pub sequence: u64,
}

impl ServiceRecord {
    /// The bytes a validator signs: a domain tag followed by the record's bincode encoding.
    pub fn signed_bytes(&self) -> Vec<u8> {
        let mut bytes = DOMAIN.to_vec();
        bytes.extend(bincode::serialize(self).expect("a record always encodes"));
        bytes
    }

    /// The record signed by the validator with `public_key`.
    pub fn sign<T: NodeType>(
        self,
        public_key: T::SignatureKey,
        private_key: &<T::SignatureKey as SignatureKey>::PrivateKey,
    ) -> Result<SignedServiceRecord<T>, <T::SignatureKey as SignatureKey>::SignError> {
        let signature = T::SignatureKey::sign(private_key, &self.signed_bytes())?;
        Ok(SignedServiceRecord {
            record: self,
            signer: public_key,
            signature,
        })
    }
}

/// A record with the signature of the validator it describes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(bound(deserialize = ""))]
pub struct SignedServiceRecord<T: NodeType> {
    pub record: ServiceRecord,
    pub signer: T::SignatureKey,
    pub signature: <T::SignatureKey as SignatureKey>::PureAssembledSignatureType,
}

impl<T: NodeType> SignedServiceRecord<T> {
    /// Whether the record is well formed and signed by its signer.
    pub fn is_valid(&self) -> bool {
        let url = &self.record.api_url;
        !url.is_empty()
            && url.len() <= MAX_API_URL_LEN
            && self
                .signer
                .validate(&self.signature, &self.record.signed_bytes())
    }
}
