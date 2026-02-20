// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Types and structs for the hotshot signature keys

use alloy::primitives::U256;
use ark_serialize::SerializationError;
use bitvec::{slice::BitSlice, vec::BitVec};
use generic_array::GenericArray;
use jf_signature::{
    bls_over_bn254::{BLSOverBN254CurveSignatureScheme, KeyPair, SignKey, VerKey},
    SignatureError, SignatureScheme,
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use tracing::instrument;

use crate::{
    light_client::{CircuitField, LightClientState, StakeTableState},
    qc::{BitVectorQc, QcParams},
    stake_table::StakeTableEntry,
    traits::{
        qc::QuorumCertificateScheme,
        signature_key::{
            BuilderSignatureKey, LCV1StateSignatureKey, LCV2StateSignatureKey,
            LCV3StateSignatureKey, PrivateSignatureKey, SignatureKey, StateSignatureKey,
        },
    },
};

/// BLS private key used to sign a consensus message
pub type BLSPrivKey = SignKey;
/// BLS public key used to verify a consensus signature
pub type BLSPubKey = VerKey;
/// BLS key pair used to sign and verify a consensus message
pub type BLSKeyPair = KeyPair;
/// Public parameters for BLS signature scheme
pub type BLSPublicParam = ();
/// BLS signature type for consensus votes
pub type BLSSignature = jf_signature::bls_over_bn254::Signature;

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
    type QcParams<'a> = QcParams<
        'a,
        BLSPubKey,
        <BLSOverBN254CurveSignatureScheme as SignatureScheme>::PublicParameter,
    >;
    type PureAssembledSignatureType =
        <BLSOverBN254CurveSignatureScheme as SignatureScheme>::Signature;
    type VerificationKeyType = Self;
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

    fn stake_table_entry(&self, stake: U256) -> Self::StakeTableEntry {
        StakeTableEntry {
            stake_key: *self,
            stake_amount: stake,
        }
    }

    fn public_key(entry: &Self::StakeTableEntry) -> Self {
        entry.stake_key
    }

    fn public_parameter(
        stake_entries: &'_ [Self::StakeTableEntry],
        threshold: U256,
    ) -> Self::QcParams<'_> {
        QcParams {
            stake_entries,
            threshold,
            agg_sig_pp: (),
        }
    }

    fn check(
        real_qc_pp: &Self::QcParams<'_>,
        data: &[u8],
        qc: &Self::QcType,
    ) -> Result<(), SignatureError> {
        let msg = GenericArray::from_slice(data);
        BitVectorQc::<BLSOverBN254CurveSignatureScheme>::check(real_qc_pp, msg, qc).map(|_| ())
    }

    fn signers(
        real_qc_pp: &Self::QcParams<'_>,
        qc: &Self::QcType,
    ) -> Result<Vec<Self>, SignatureError> {
        BitVectorQc::<BLSOverBN254CurveSignatureScheme>::signers(real_qc_pp, qc)
    }

    fn sig_proof(signature: &Self::QcType) -> (Self::PureAssembledSignatureType, BitVec) {
        signature.clone()
    }

    fn assemble(
        real_qc_pp: &Self::QcParams<'_>,
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

    fn to_verification_key(&self) -> Self::VerificationKeyType {
        *self
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

pub type SchnorrPubKey = jf_signature::schnorr::VerKey<ark_ed_on_bn254::EdwardsConfig>;
pub type SchnorrPrivKey = jf_signature::schnorr::SignKey<ark_ed_on_bn254::Fr>;
pub type SchnorrSignatureScheme =
    jf_signature::schnorr::SchnorrSignatureScheme<ark_ed_on_bn254::EdwardsConfig>;

impl PrivateSignatureKey for SchnorrPrivKey {
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

impl StateSignatureKey for SchnorrPubKey {
    type StatePrivateKey = SchnorrPrivKey;

    type StateSignature = jf_signature::schnorr::Signature<ark_ed_on_bn254::EdwardsConfig>;

    type SignError = SignatureError;

    fn generated_from_seed_indexed(seed: [u8; 32], index: u64) -> (Self, Self::StatePrivateKey) {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&seed);
        hasher.update(&index.to_le_bytes());
        let new_seed = *hasher.finalize().as_bytes();
        let kp = jf_signature::schnorr::KeyPair::generate(&mut ChaCha20Rng::from_seed(new_seed));
        (kp.ver_key(), kp.sign_key())
    }
}

impl LCV1StateSignatureKey for SchnorrPubKey {
    fn sign_state(
        sk: &Self::StatePrivateKey,
        light_client_state: &LightClientState,
    ) -> Result<Self::StateSignature, Self::SignError> {
        let state_msg: [_; 3] = light_client_state.into();
        SchnorrSignatureScheme::sign(&(), sk, state_msg, &mut rand::thread_rng())
    }

    fn verify_state_sig(
        &self,
        signature: &Self::StateSignature,
        light_client_state: &LightClientState,
    ) -> bool {
        let state_msg: [_; 3] = light_client_state.into();
        SchnorrSignatureScheme::verify(&(), self, state_msg, signature).is_ok()
    }
}

impl LCV2StateSignatureKey for SchnorrPubKey {
    fn sign_state(
        sk: &Self::StatePrivateKey,
        light_client_state: &LightClientState,
        next_stake_table_state: &StakeTableState,
    ) -> Result<Self::StateSignature, Self::SignError> {
        let mut msg = Vec::with_capacity(7);
        let state_msg: [_; 3] = light_client_state.into();
        msg.extend_from_slice(&state_msg);
        let adv_st_state_msg: [_; 4] = (*next_stake_table_state).into();
        msg.extend_from_slice(&adv_st_state_msg);
        SchnorrSignatureScheme::sign(&(), sk, msg, &mut rand::thread_rng())
    }

    fn verify_state_sig(
        &self,
        signature: &Self::StateSignature,
        light_client_state: &LightClientState,
        next_stake_table_state: &StakeTableState,
    ) -> bool {
        let mut msg = Vec::with_capacity(7);
        let state_msg: [_; 3] = light_client_state.into();
        msg.extend_from_slice(&state_msg);
        let adv_st_state_msg: [_; 4] = (*next_stake_table_state).into();
        msg.extend_from_slice(&adv_st_state_msg);
        SchnorrSignatureScheme::verify(&(), self, msg, signature).is_ok()
    }
}

impl LCV3StateSignatureKey for SchnorrPubKey {
    /// Sign the light client state
    /// The input `msg` should be the keccak256 hash of ABI encodings of the light client state,
    /// next stake table state, and the auth root.
    fn sign_state(
        private_key: &Self::StatePrivateKey,
        msg: CircuitField,
    ) -> Result<Self::StateSignature, Self::SignError> {
        SchnorrSignatureScheme::sign(&(), private_key, [msg], &mut rand::thread_rng())
    }

    /// Verify the light client state signature
    /// The input `msg` should be the keccak256 hash of ABI encodings of the light client state,
    /// next stake table state, and the auth root.
    fn verify_state_sig(&self, signature: &Self::StateSignature, msg: CircuitField) -> bool {
        SchnorrSignatureScheme::verify(&(), self, [msg], signature).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;
    use bitvec::prelude::*;
    use jf_signature::bls_over_bn254::{BLSOverBN254CurveSignatureScheme, KeyPair};

    use super::BLSPubKey;
    use crate::{
        qc::BitVectorQc,
        stake_table::StakeTableEntry,
        traits::{qc::QuorumCertificateScheme, signature_key::SignatureKey},
    };

    #[test]
    fn test_to_verification_key_is_identity() {
        let mut rng = jf_utils::test_rng();
        let kp = KeyPair::generate(&mut rng);
        let pub_key: BLSPubKey = kp.ver_key();
        // For BLSPubKey, VerificationKeyType = Self, so this should be a copy.
        assert_eq!(pub_key.to_verification_key(), pub_key);
    }

    #[test]
    fn test_bls_signers_correct_keys() {
        let mut rng = jf_utils::test_rng();
        let kp1 = KeyPair::generate(&mut rng);
        let kp2 = KeyPair::generate(&mut rng);
        let kp3 = KeyPair::generate(&mut rng);
        let entries: Vec<StakeTableEntry<BLSPubKey>> = vec![
            StakeTableEntry {
                stake_key: kp1.ver_key(),
                stake_amount: U256::from(1u8),
            },
            StakeTableEntry {
                stake_key: kp2.ver_key(),
                stake_amount: U256::from(1u8),
            },
            StakeTableEntry {
                stake_key: kp3.ver_key(),
                stake_amount: U256::from(1u8),
            },
        ];
        // Use BLSPubKey::public_parameter to create QcParams (threshold = 2, 1 stake each).
        let qc_pp = BLSPubKey::public_parameter(&entries, U256::from(2u8));
        let msg = [55u8; 32];
        let sig1 = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::sign(
            &(),
            kp1.sign_key_ref(),
            msg,
            &mut rng,
        )
        .unwrap();
        let sig2 = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::sign(
            &(),
            kp2.sign_key_ref(),
            msg,
            &mut rng,
        )
        .unwrap();
        // nodes 0 and 1 sign (bitvec [1, 1, 0])
        let signers_bv = bitvec![1, 1, 0];
        let qc = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::assemble(
            &qc_pp,
            signers_bv.as_bitslice(),
            &[sig1, sig2],
        )
        .unwrap();
        let result = BLSPubKey::signers(&qc_pp, &qc).unwrap();
        assert_eq!(result, vec![kp1.ver_key(), kp2.ver_key()]);
    }

    #[test]
    fn test_bls_signers_bitvec_mismatch() {
        let mut rng = jf_utils::test_rng();
        let kp1 = KeyPair::generate(&mut rng);
        let kp2 = KeyPair::generate(&mut rng);
        let kp3 = KeyPair::generate(&mut rng);
        let entries: Vec<StakeTableEntry<BLSPubKey>> = vec![
            StakeTableEntry {
                stake_key: kp1.ver_key(),
                stake_amount: U256::from(1u8),
            },
            StakeTableEntry {
                stake_key: kp2.ver_key(),
                stake_amount: U256::from(1u8),
            },
            StakeTableEntry {
                stake_key: kp3.ver_key(),
                stake_amount: U256::from(1u8),
            },
        ];
        let qc_pp = BLSPubKey::public_parameter(&entries, U256::from(2u8));

        // Build a QC with a bitvec of length 2 (should be 3).
        let wrong_bv = bitvec![1, 1];
        let sig = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::sign(
            &(),
            kp1.sign_key_ref(),
            [0u8; 32],
            &mut rng,
        )
        .unwrap();
        let qc_bad = (sig, wrong_bv);
        let result = BLSPubKey::signers(&qc_pp, &qc_bad);
        assert!(result.is_err());
    }
}
