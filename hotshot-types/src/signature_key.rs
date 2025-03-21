// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Types and structs for the hotshot signature keys

use ark_serialize::SerializationError;
use bitvec::{slice::BitSlice, vec::BitVec};
use digest::generic_array::GenericArray;
use jf_signature::{
    bls_over_bn254::{BLSOverBN254CurveSignatureScheme, KeyPair, SignKey, VerKey},
    SignatureError, SignatureScheme,
};
use primitive_types::U256;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use tracing::instrument;

use crate::{
    qc::{BitVectorQc, QcParams},
    stake_table::StakeTableEntry,
    traits::{
        qc::QuorumCertificateScheme,
        signature_key::{BuilderSignatureKey, PrivateSignatureKey, SignatureKey},
    },
};

/// BLS private key used to sign a message
pub type BLSPrivKey = SignKey;
/// BLS public key used to verify a signature
pub type BLSPubKey = VerKey;
/// Public parameters for BLS signature scheme
pub type BLSPublicParam = ();

impl PrivateSignatureKey for BLSPrivKey {
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Ok(Self::from_bytes(bytes))
    }

    fn to_tagged_base64(&self) -> Result<tagged_base64::TaggedBase64, tagged_base64::Tb64Error> {
        self.to_tagged_base64()
    }
}

impl SignatureKey for BLSPubKey {
    type PrivateKey = BLSPrivKey;
    type StakeTableEntry = StakeTableEntry<VerKey>;
    type QcParams =
        QcParams<BLSPubKey, <BLSOverBN254CurveSignatureScheme as SignatureScheme>::PublicParameter>;
    type PureAssembledSignatureType =
        <BLSOverBN254CurveSignatureScheme as SignatureScheme>::Signature;
    type QcType = (Self::PureAssembledSignatureType, BitVec);
    type SignError = SignatureError;

    #[instrument(skip(self))]
    fn validate(&self, signature: &Self::PureAssembledSignatureType, data: &[u8]) -> bool {
        // This is the validation for QC partial signature before append().
        BLSOverBN254CurveSignatureScheme::verify(&(), self, data, signature).is_ok()
    }

    fn sign(
        sk: &Self::PrivateKey,
        data: &[u8],
    ) -> Result<Self::PureAssembledSignatureType, Self::SignError> {
        BitVectorQc::<BLSOverBN254CurveSignatureScheme>::sign(
            &(),
            sk,
            data,
            &mut rand::thread_rng(),
        )
    }

    fn from_private(private_key: &Self::PrivateKey) -> Self {
        BLSPubKey::from(private_key)
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![];
        ark_serialize::CanonicalSerialize::serialize_compressed(self, &mut buf)
            .expect("Serialization should not fail.");
        buf
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, SerializationError> {
        ark_serialize::CanonicalDeserialize::deserialize_compressed(bytes)
    }

    fn generated_from_seed_indexed(seed: [u8; 32], index: u64) -> (Self, Self::PrivateKey) {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&seed);
        hasher.update(&index.to_le_bytes());
        let new_seed = *hasher.finalize().as_bytes();
        let kp = KeyPair::generate(&mut ChaCha20Rng::from_seed(new_seed));
        (kp.ver_key(), kp.sign_key_ref().clone())
    }

    fn stake_table_entry(&self, stake: u64) -> Self::StakeTableEntry {
        StakeTableEntry {
            stake_key: *self,
            stake_amount: U256::from(stake),
        }
    }

    fn public_key(entry: &Self::StakeTableEntry) -> Self {
        entry.stake_key
    }

    fn public_parameter(
        stake_entries: Vec<Self::StakeTableEntry>,
        threshold: U256,
    ) -> Self::QcParams {
        QcParams {
            stake_entries,
            threshold,
            agg_sig_pp: (),
        }
    }

    fn check(
        real_qc_pp: &Self::QcParams,
        data: &[u8],
        qc: &Self::QcType,
    ) -> Result<(), SignatureError> {
        let msg = GenericArray::from_slice(data);
        BitVectorQc::<BLSOverBN254CurveSignatureScheme>::check(real_qc_pp, msg, qc).map(|_| ())
    }

    fn sig_proof(signature: &Self::QcType) -> (Self::PureAssembledSignatureType, BitVec) {
        signature.clone()
    }

    fn assemble(
        real_qc_pp: &Self::QcParams,
        signers: &BitSlice,
        sigs: &[Self::PureAssembledSignatureType],
    ) -> Self::QcType {
        BitVectorQc::<BLSOverBN254CurveSignatureScheme>::assemble(real_qc_pp, signers, sigs)
            .expect("this assembling shouldn't fail")
    }

    fn genesis_proposer_pk() -> Self {
        let kp = KeyPair::generate(&mut ChaCha20Rng::from_seed([0u8; 32]));
        kp.ver_key()
    }
}

// Currently implement builder signature key for BLS
// So copy pasta here, but actually Sequencer will implement the same trait for ethereum types
/// Builder signature key
pub type BuilderKey = BLSPubKey;

impl BuilderSignatureKey for BuilderKey {
    type BuilderPrivateKey = BLSPrivKey;
    type BuilderSignature = <BLSOverBN254CurveSignatureScheme as SignatureScheme>::Signature;
    type SignError = SignatureError;

    fn sign_builder_message(
        private_key: &Self::BuilderPrivateKey,
        data: &[u8],
    ) -> Result<Self::BuilderSignature, Self::SignError> {
        BitVectorQc::<BLSOverBN254CurveSignatureScheme>::sign(
            &(),
            private_key,
            data,
            &mut rand::thread_rng(),
        )
    }

    fn validate_builder_signature(&self, signature: &Self::BuilderSignature, data: &[u8]) -> bool {
        BLSOverBN254CurveSignatureScheme::verify(&(), self, data, signature).is_ok()
    }

    fn generated_from_seed_indexed(seed: [u8; 32], index: u64) -> (Self, Self::BuilderPrivateKey) {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&seed);
        hasher.update(&index.to_le_bytes());
        let new_seed = *hasher.finalize().as_bytes();
        let kp = KeyPair::generate(&mut ChaCha20Rng::from_seed(new_seed));
        (kp.ver_key(), kp.sign_key_ref().clone())
    }
}
