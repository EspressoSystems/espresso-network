// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Minimal compatibility over public key signatures

// data is serialized as big-endian for signing purposes
#![forbid(clippy::little_endian_bytes)]

use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

use alloy::primitives::U256;
use ark_serialize::SerializationError;
use bitvec::prelude::*;
use committable::Committable;
use jf_signature::SignatureError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tagged_base64::{TaggedBase64, Tb64Error};

use super::EncodeBytes;
use crate::{
    bundle::Bundle, light_client::LightClientStateMsg, traits::node_implementation::NodeType,
    utils::BuilderCommitment,
};

/// Type representing stake table entries in a `StakeTable`
pub trait StakeTableEntryType<K> {
    /// Get the stake value
    fn stake(&self) -> U256;
    /// Get the public key
    fn public_key(&self) -> K;
}

/// Trait for abstracting private signature key
pub trait PrivateSignatureKey:
    Send + Sync + Sized + Clone + Debug + Eq + Hash + for<'a> TryFrom<&'a TaggedBase64>
{
    /// Serialize the private key into bytes
    fn to_bytes(&self) -> Vec<u8>;

    /// Deserialize the private key from bytes
    /// # Errors
    /// If deserialization fails.
    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self>;

    /// Serialize the private key into TaggedBase64 blob.
    /// # Errors
    /// If serialization fails.
    fn to_tagged_base64(&self) -> Result<TaggedBase64, Tb64Error>;
}

/// Trait for abstracting public key signatures
/// Self is the public key type
pub trait SignatureKey:
    Send
    + Sync
    + Clone
    + Sized
    + Debug
    + Hash
    + Serialize
    + for<'a> Deserialize<'a>
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Display
    + for<'a> TryFrom<&'a TaggedBase64>
    + Into<TaggedBase64>
{
    /// The private key type for this signature algorithm
    type PrivateKey: PrivateSignatureKey;
    /// The type of the entry that contain both public key and stake value
    type StakeTableEntry: StakeTableEntryType<Self>
        + Send
        + Sync
        + Sized
        + Clone
        + Debug
        + Hash
        + Eq
        + Serialize
        + for<'a> Deserialize<'a>;
    /// The type of the quorum certificate parameters used for assembled signature
    type QcParams: Send + Sync + Sized + Clone + Debug + Hash;
    /// The type of the assembled signature, without `BitVec`
    type PureAssembledSignatureType: Send
        + Sync
        + Sized
        + Clone
        + Debug
        + Hash
        + PartialEq
        + Eq
        + Serialize
        + for<'a> Deserialize<'a>
        + Into<TaggedBase64>
        + for<'a> TryFrom<&'a TaggedBase64>;
    /// The type of the assembled qc: assembled signature + `BitVec`
    type QcType: Send
        + Sync
        + Sized
        + Clone
        + Debug
        + Hash
        + PartialEq
        + Eq
        + Serialize
        + for<'a> Deserialize<'a>;

    /// Type of error that can occur when signing data
    type SignError: std::error::Error + Send + Sync;

    // Signature type represented as a vec/slice of bytes to let the implementer handle the nuances
    // of serialization, to avoid Cryptographic pitfalls
    /// Validate a signature
    fn validate(&self, signature: &Self::PureAssembledSignatureType, data: &[u8]) -> bool;

    /// Produce a signature
    /// # Errors
    /// If unable to sign the data with the key
    fn sign(
        private_key: &Self::PrivateKey,
        data: &[u8],
    ) -> Result<Self::PureAssembledSignatureType, Self::SignError>;

    /// Produce a public key from a private key
    fn from_private(private_key: &Self::PrivateKey) -> Self;
    /// Serialize a public key to bytes
    fn to_bytes(&self) -> Vec<u8>;
    /// Deserialize a public key from bytes
    /// # Errors
    ///
    /// Will return `Err` if deserialization fails
    fn from_bytes(bytes: &[u8]) -> Result<Self, SerializationError>;

    /// Generate a new key pair
    fn generated_from_seed_indexed(seed: [u8; 32], index: u64) -> (Self, Self::PrivateKey);

    /// get the stake table entry from the public key and stake value
    fn stake_table_entry(&self, stake: U256) -> Self::StakeTableEntry;

    /// only get the public key from the stake table entry
    fn public_key(entry: &Self::StakeTableEntry) -> Self;

    /// get the public parameter for the assembled signature checking
    fn public_parameter(
        stake_entries: Vec<Self::StakeTableEntry>,
        threshold: U256,
    ) -> Self::QcParams;

    /// check the quorum certificate for the assembled signature, returning `Ok(())` if it is valid.
    ///
    /// # Errors
    /// Returns an error if the signature key fails to validate
    fn check(
        real_qc_pp: &Self::QcParams,
        data: &[u8],
        qc: &Self::QcType,
    ) -> Result<(), SignatureError>;

    /// get the assembled signature and the `BitVec` separately from the assembled signature
    fn sig_proof(signature: &Self::QcType) -> (Self::PureAssembledSignatureType, BitVec);

    /// assemble the signature from the partial signature and the indication of signers in `BitVec`
    fn assemble(
        real_qc_pp: &Self::QcParams,
        signers: &BitSlice,
        sigs: &[Self::PureAssembledSignatureType],
    ) -> Self::QcType;

    /// generates the genesis public key. Meant to be dummy/filler
    #[must_use]
    fn genesis_proposer_pk() -> Self;
}

/// Builder Signature Key trait with minimal requirements
pub trait BuilderSignatureKey:
    Send
    + Sync
    + Clone
    + Sized
    + Debug
    + Hash
    + Serialize
    + DeserializeOwned
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
    + Display
{
    /// The type of the keys builder would use to sign its messages
    type BuilderPrivateKey: PrivateSignatureKey;

    /// The type of the signature builder would use to sign its messages
    type BuilderSignature: Send
        + Sync
        + Sized
        + Clone
        + Debug
        + Eq
        + Serialize
        + for<'a> Deserialize<'a>
        + Hash;

    /// Type of error that can occur when signing data
    type SignError: std::error::Error + Send + Sync;

    /// validate the message with the builder's public key
    fn validate_builder_signature(&self, signature: &Self::BuilderSignature, data: &[u8]) -> bool;

    /// validate signature over sequencing fee information
    /// with the builder's public key
    fn validate_fee_signature<Metadata: EncodeBytes>(
        &self,
        signature: &Self::BuilderSignature,
        fee_amount: u64,
        metadata: &Metadata,
    ) -> bool {
        self.validate_builder_signature(signature, &aggregate_fee_data(fee_amount, metadata))
    }

    /// validate signature over sequencing fee information
    /// with the builder's public key (marketplace version)
    fn validate_sequencing_fee_signature_marketplace(
        &self,
        signature: &Self::BuilderSignature,
        fee_amount: u64,
        view_number: u64,
    ) -> bool {
        self.validate_builder_signature(
            signature,
            &aggregate_fee_data_marketplace(fee_amount, view_number),
        )
    }

    /// validate the bundle's signature using the builder's public key
    fn validate_bundle_signature<TYPES: NodeType<BuilderSignatureKey = Self>>(
        &self,
        bundle: Bundle<TYPES>,
    ) -> bool where {
        let commitments = bundle
            .transactions
            .iter()
            .flat_map(|txn| <[u8; 32]>::from(txn.commit()))
            .collect::<Vec<u8>>();

        self.validate_builder_signature(&bundle.signature, &commitments)
    }

    /// validate signature over block information with the builder's public key
    fn validate_block_info_signature(
        &self,
        signature: &Self::BuilderSignature,
        block_size: u64,
        fee_amount: u64,
        payload_commitment: &BuilderCommitment,
    ) -> bool {
        self.validate_builder_signature(
            signature,
            &aggregate_block_info_data(block_size, fee_amount, payload_commitment),
        )
    }

    /// sign the message with the builder's private key
    /// # Errors
    /// If unable to sign the data with the key
    fn sign_builder_message(
        private_key: &Self::BuilderPrivateKey,
        data: &[u8],
    ) -> Result<Self::BuilderSignature, Self::SignError>;

    /// sign sequencing fee offer
    /// # Errors
    /// If unable to sign the data with the key
    fn sign_fee<Metadata: EncodeBytes>(
        private_key: &Self::BuilderPrivateKey,
        fee_amount: u64,
        metadata: &Metadata,
    ) -> Result<Self::BuilderSignature, Self::SignError> {
        Self::sign_builder_message(private_key, &aggregate_fee_data(fee_amount, metadata))
    }

    /// sign fee offer (marketplace version)
    /// # Errors
    /// If unable to sign the data with the key
    fn sign_sequencing_fee_marketplace(
        private_key: &Self::BuilderPrivateKey,
        fee_amount: u64,
        view_number: u64,
    ) -> Result<Self::BuilderSignature, Self::SignError> {
        Self::sign_builder_message(
            private_key,
            &aggregate_fee_data_marketplace(fee_amount, view_number),
        )
    }

    /// sign transactions (marketplace version)
    /// # Errors
    /// If unable to sign the data with the key
    fn sign_bundle<TYPES: NodeType>(
        private_key: &Self::BuilderPrivateKey,
        transactions: &[TYPES::Transaction],
    ) -> Result<Self::BuilderSignature, Self::SignError> {
        let commitments = transactions
            .iter()
            .flat_map(|txn| <[u8; 32]>::from(txn.commit()))
            .collect::<Vec<u8>>();

        Self::sign_builder_message(private_key, &commitments)
    }

    /// sign information about offered block
    /// # Errors
    /// If unable to sign the data with the key
    fn sign_block_info(
        private_key: &Self::BuilderPrivateKey,
        block_size: u64,
        fee_amount: u64,
        payload_commitment: &BuilderCommitment,
    ) -> Result<Self::BuilderSignature, Self::SignError> {
        Self::sign_builder_message(
            private_key,
            &aggregate_block_info_data(block_size, fee_amount, payload_commitment),
        )
    }

    /// Generate a new key pair
    fn generated_from_seed_indexed(seed: [u8; 32], index: u64) -> (Self, Self::BuilderPrivateKey);
}

/// Aggregate all inputs used for signature over fee data
fn aggregate_fee_data<Metadata: EncodeBytes>(fee_amount: u64, metadata: &Metadata) -> Vec<u8> {
    let mut fee_info = Vec::new();
    fee_info.extend_from_slice(fee_amount.to_be_bytes().as_ref());
    fee_info.extend_from_slice(metadata.encode().as_ref());
    fee_info
}

/// Aggregate all inputs used for signature over fee data
fn aggregate_fee_data_marketplace(fee_amount: u64, view_number: u64) -> Vec<u8> {
    let mut fee_info = Vec::new();
    fee_info.extend_from_slice(fee_amount.to_be_bytes().as_ref());
    fee_info.extend_from_slice(view_number.to_be_bytes().as_ref());
    fee_info
}

/// Aggregate all inputs used for signature over block info
fn aggregate_block_info_data(
    block_size: u64,
    fee_amount: u64,
    payload_commitment: &BuilderCommitment,
) -> Vec<u8> {
    let mut block_info = Vec::new();
    block_info.extend_from_slice(block_size.to_be_bytes().as_ref());
    block_info.extend_from_slice(fee_amount.to_be_bytes().as_ref());
    block_info.extend_from_slice(payload_commitment.as_ref());
    block_info
}

/// Light client state signature key with minimal requirements
pub trait StateSignatureKey:
    Send
    + Sync
    + Clone
    + Sized
    + Debug
    + Hash
    + Serialize
    + for<'a> Deserialize<'a>
    + PartialEq
    + Eq
    + Display
    + for<'a> TryFrom<&'a TaggedBase64>
    + Into<TaggedBase64>
{
    /// The private key type
    type StatePrivateKey: PrivateSignatureKey;

    /// The type of the signature
    type StateSignature: Send
        + Sync
        + Sized
        + Clone
        + Debug
        + Eq
        + Serialize
        + for<'a> Deserialize<'a>
        + Hash;

    /// Type of error that can occur when signing data
    type SignError: std::error::Error + Send + Sync;

    /// Sign the light client state
    fn sign_state(
        private_key: &Self::StatePrivateKey,
        state: &LightClientStateMsg,
    ) -> Result<Self::StateSignature, Self::SignError>;

    /// Verify the light client state signature
    fn verify_state_sig(
        &self,
        signature: &Self::StateSignature,
        state: &LightClientStateMsg,
    ) -> bool;

    /// Generate a new key pair
    fn generated_from_seed_indexed(seed: [u8; 32], index: u64) -> (Self, Self::StatePrivateKey);
}
