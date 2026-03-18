// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Implementation for `BitVectorQc` that uses BLS signature + Bit vector.
//! See more details in hotshot paper.

use alloy::primitives::U256;
use ark_std::{
    fmt::Debug,
    format,
    marker::PhantomData,
    rand::{CryptoRng, RngCore},
    vec,
    vec::Vec,
};
use bitvec::prelude::*;
use generic_array::GenericArray;
use jf_signature::{AggregateableSignatureSchemes, SignatureError};
use serde::{Deserialize, Serialize};
use typenum::U32;

use crate::{
    stake_table::StakeTableEntry,
    traits::{qc::QuorumCertificateScheme, signature_key::SignatureKey},
};

/// An implementation of QC using BLS signature and a bit-vector.
#[derive(Serialize, Deserialize)]
pub struct BitVectorQc<A: AggregateableSignatureSchemes + Serialize + for<'a> Deserialize<'a>>(
    PhantomData<A>,
);

/// Public parameters of [`BitVectorQc`]
#[derive(PartialEq, Debug, Clone, Hash)]
pub struct QcParams<'a, K: SignatureKey, P> {
    /// the stake table (snapshot) this QC is verified against
    pub stake_entries: &'a [StakeTableEntry<K>],
    /// threshold for the accumulated "weight" of votes to form a QC
    pub threshold: U256,
    /// public parameter for the aggregated signature scheme
    pub agg_sig_pp: P,
}

impl<A> QuorumCertificateScheme<A> for BitVectorQc<A>
where
    A: AggregateableSignatureSchemes,
    A::VerificationKey: SignatureKey,
{
    type QcProverParams<'a> = QcParams<'a, A::VerificationKey, A::PublicParameter>;

    // TODO: later with SNARKs we'll use a smaller verifier parameter
    type QcVerifierParams<'a> = QcParams<'a, A::VerificationKey, A::PublicParameter>;

    type Qc = (A::Signature, BitVec);
    type MessageLength = U32;
    type QuorumSize = U256;

    /// Sign a message with the signing key
    fn sign<R: CryptoRng + RngCore, M: AsRef<[A::MessageUnit]>>(
        pp: &A::PublicParameter,
        sk: &A::SigningKey,
        msg: M,
        prng: &mut R,
    ) -> Result<A::Signature, SignatureError> {
        A::sign(pp, sk, msg, prng)
    }

    fn assemble(
        qc_pp: &Self::QcProverParams<'_>,
        signers: &BitSlice,
        sigs: &[A::Signature],
    ) -> Result<Self::Qc, SignatureError> {
        if signers.len() != qc_pp.stake_entries.len() {
            return Err(SignatureError::ParameterError(format!(
                "bit vector len {} != the number of stake entries {}",
                signers.len(),
                qc_pp.stake_entries.len(),
            )));
        }
        let total_weight: U256 =
            qc_pp
                .stake_entries
                .iter()
                .zip(signers.iter())
                .fold(
                    U256::ZERO,
                    |acc, (entry, b)| {
                        if *b { acc + entry.stake_amount } else { acc }
                    },
                );
        if total_weight < qc_pp.threshold {
            return Err(SignatureError::ParameterError(format!(
                "total_weight {} less than threshold {}",
                total_weight, qc_pp.threshold,
            )));
        }
        let mut ver_keys = vec![];
        for (entry, b) in qc_pp.stake_entries.iter().zip(signers.iter()) {
            if *b {
                ver_keys.push(entry.stake_key.clone());
            }
        }
        if ver_keys.len() != sigs.len() {
            return Err(SignatureError::ParameterError(format!(
                "the number of ver_keys {} != the number of partial signatures {}",
                ver_keys.len(),
                sigs.len(),
            )));
        }
        let sig = A::aggregate(&qc_pp.agg_sig_pp, &ver_keys[..], sigs)?;

        Ok((sig, signers.into()))
    }

    fn check(
        qc_vp: &Self::QcVerifierParams<'_>,
        message: &GenericArray<A::MessageUnit, Self::MessageLength>,
        qc: &Self::Qc,
    ) -> Result<Self::QuorumSize, SignatureError> {
        let (sig, signers) = qc;
        if signers.len() != qc_vp.stake_entries.len() {
            return Err(SignatureError::ParameterError(format!(
                "signers bit vector len {} != the number of stake entries {}",
                signers.len(),
                qc_vp.stake_entries.len(),
            )));
        }
        let total_weight: U256 =
            qc_vp
                .stake_entries
                .iter()
                .zip(signers.iter())
                .fold(
                    U256::ZERO,
                    |acc, (entry, b)| {
                        if *b { acc + entry.stake_amount } else { acc }
                    },
                );
        if total_weight < qc_vp.threshold {
            return Err(SignatureError::ParameterError(format!(
                "total_weight {} less than threshold {}",
                total_weight, qc_vp.threshold,
            )));
        }
        let mut ver_keys = vec![];
        for (entry, b) in qc_vp.stake_entries.iter().zip(signers.iter()) {
            if *b {
                ver_keys.push(entry.stake_key.clone());
            }
        }
        A::multi_sig_verify(&qc_vp.agg_sig_pp, &ver_keys[..], message, sig)?;

        Ok(total_weight)
    }

    fn trace(
        qc_vp: &Self::QcVerifierParams<'_>,
        message: &GenericArray<<A>::MessageUnit, Self::MessageLength>,
        qc: &Self::Qc,
    ) -> Result<Vec<<A>::VerificationKey>, SignatureError> {
        Self::check(qc_vp, message, qc)?;
        Self::signers(qc_vp, qc)
    }

    fn signers(
        qc_vp: &Self::QcVerifierParams<'_>,
        qc: &Self::Qc,
    ) -> Result<Vec<<A>::VerificationKey>, SignatureError> {
        let (_sig, signers) = qc;
        if signers.len() != qc_vp.stake_entries.len() {
            return Err(SignatureError::ParameterError(format!(
                "signers bit vector len {} != the number of stake entries {}",
                signers.len(),
                qc_vp.stake_entries.len(),
            )));
        }

        let signer_pks: Vec<_> = qc_vp
            .stake_entries
            .iter()
            .zip(signers.iter())
            .filter(|(_, b)| **b)
            .map(|(pk, _)| pk.stake_key.clone())
            .collect();
        Ok(signer_pks)
    }
}

#[cfg(test)]
mod tests {
    use jf_signature::{
        SignatureScheme,
        bls_over_bn254::{BLSOverBN254CurveSignatureScheme, KeyPair},
    };
    use vbs::{BinarySerializer, Serializer, version::StaticVersion};

    use super::*;
    type Version = StaticVersion<0, 1>;

    macro_rules! test_quorum_certificate {
        ($aggsig:tt) => {
            let mut rng = jf_utils::test_rng();
            let agg_sig_pp = $aggsig::param_gen(Some(&mut rng)).unwrap();
            let key_pair1 = KeyPair::generate(&mut rng);
            let key_pair2 = KeyPair::generate(&mut rng);
            let key_pair3 = KeyPair::generate(&mut rng);
            let entry1 = StakeTableEntry {
                stake_key: key_pair1.ver_key(),
                stake_amount: U256::from(3u8),
            };
            let entry2 = StakeTableEntry {
                stake_key: key_pair2.ver_key(),
                stake_amount: U256::from(5u8),
            };
            let entry3 = StakeTableEntry {
                stake_key: key_pair3.ver_key(),
                stake_amount: U256::from(7u8),
            };
            let qc_pp = QcParams {
                stake_entries: &[entry1, entry2, entry3],
                threshold: U256::from(10u8),
                agg_sig_pp,
            };
            let msg = [72u8; 32];
            let sig1 =
                BitVectorQc::<$aggsig>::sign(&agg_sig_pp, key_pair1.sign_key_ref(), &msg, &mut rng)
                    .unwrap();
            let sig2 =
                BitVectorQc::<$aggsig>::sign(&agg_sig_pp, key_pair2.sign_key_ref(), &msg, &mut rng)
                    .unwrap();
            let sig3 =
                BitVectorQc::<$aggsig>::sign(&agg_sig_pp, key_pair3.sign_key_ref(), &msg, &mut rng)
                    .unwrap();

            // happy path
            let signers = bitvec![0, 1, 1];
            let qc = BitVectorQc::<$aggsig>::assemble(
                &qc_pp,
                signers.as_bitslice(),
                &[sig2.clone(), sig3.clone()],
            )
            .unwrap();
            assert!(BitVectorQc::<$aggsig>::check(&qc_pp, &msg.into(), &qc).is_ok());
            assert_eq!(
                BitVectorQc::<$aggsig>::trace(&qc_pp, &msg.into(), &qc).unwrap(),
                vec![key_pair2.ver_key(), key_pair3.ver_key()],
            );

            // Check the QC and the QcParams can be serialized / deserialized
            assert_eq!(
                qc,
                Serializer::<Version>::deserialize(&Serializer::<Version>::serialize(&qc).unwrap())
                    .unwrap()
            );

            // bad paths
            // number of signatures unmatch
            assert!(BitVectorQc::<$aggsig>::assemble(
                &qc_pp,
                signers.as_bitslice(),
                std::slice::from_ref(&sig2)
            )
            .is_err());
            // total weight under threshold
            let active_bad = bitvec![1, 1, 0];
            assert!(BitVectorQc::<$aggsig>::assemble(
                &qc_pp,
                active_bad.as_bitslice(),
                &[sig1.clone(), sig2.clone()]
            )
            .is_err());
            // wrong bool vector length
            let active_bad_2 = bitvec![0, 1, 1, 0];
            assert!(BitVectorQc::<$aggsig>::assemble(
                &qc_pp,
                active_bad_2.as_bitslice(),
                &[sig2, sig3],
            )
            .is_err());

            assert!(BitVectorQc::<$aggsig>::check(
                &qc_pp,
                &msg.into(),
                &(qc.0.clone(), active_bad)
            )
            .is_err());
            assert!(BitVectorQc::<$aggsig>::check(
                &qc_pp,
                &msg.into(),
                &(qc.0.clone(), active_bad_2)
            )
            .is_err());
            let bad_msg = [70u8; 32];
            assert!(BitVectorQc::<$aggsig>::check(&qc_pp, &bad_msg.into(), &qc).is_err());

            let bad_sig = &sig1;
            assert!(
                BitVectorQc::<$aggsig>::check(&qc_pp, &msg.into(), &(bad_sig.clone(), qc.1))
                    .is_err()
            );
        };
    }
    #[test]
    fn test_quorum_certificate() {
        test_quorum_certificate!(BLSOverBN254CurveSignatureScheme);
    }

    /// State returned by the [`three_node_setup`] helper.
    struct ThreeNodeSetup {
        key_pair1: KeyPair,
        key_pair2: KeyPair,
        key_pair3: KeyPair,
        entries: Vec<
            StakeTableEntry<<BLSOverBN254CurveSignatureScheme as SignatureScheme>::VerificationKey>,
        >,
        sig1: <BLSOverBN254CurveSignatureScheme as SignatureScheme>::Signature,
        sig2: <BLSOverBN254CurveSignatureScheme as SignatureScheme>::Signature,
        sig3: <BLSOverBN254CurveSignatureScheme as SignatureScheme>::Signature,
    }

    /// Helper that assembles a 3-node QC setup reused across signers tests.
    ///
    /// Callers build `QcParams { stake_entries: &setup.entries, threshold, agg_sig_pp: () }`
    /// locally so that the borrow of `entries` stays within the caller's stack frame.
    fn three_node_setup() -> ThreeNodeSetup {
        let mut rng = jf_utils::test_rng();
        let key_pair1 = KeyPair::generate(&mut rng);
        let key_pair2 = KeyPair::generate(&mut rng);
        let key_pair3 = KeyPair::generate(&mut rng);
        let entries = vec![
            StakeTableEntry {
                stake_key: key_pair1.ver_key(),
                stake_amount: U256::from(1u8),
            },
            StakeTableEntry {
                stake_key: key_pair2.ver_key(),
                stake_amount: U256::from(1u8),
            },
            StakeTableEntry {
                stake_key: key_pair3.ver_key(),
                stake_amount: U256::from(1u8),
            },
        ];
        let msg = [42u8; 32];
        let sig1 = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::sign(
            &(),
            key_pair1.sign_key_ref(),
            msg,
            &mut rng,
        )
        .unwrap();
        let sig2 = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::sign(
            &(),
            key_pair2.sign_key_ref(),
            msg,
            &mut rng,
        )
        .unwrap();
        let sig3 = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::sign(
            &(),
            key_pair3.sign_key_ref(),
            msg,
            &mut rng,
        )
        .unwrap();
        ThreeNodeSetup {
            key_pair1,
            key_pair2,
            key_pair3,
            entries,
            sig1,
            sig2,
            sig3,
        }
    }

    #[test]
    fn test_signers_extracts_correct_keys() {
        let setup = three_node_setup();
        let qc_pp = QcParams {
            stake_entries: &setup.entries,
            threshold: U256::from(2u8),
            agg_sig_pp: (),
        };
        // Nodes 2 and 3 sign (bitvec [0, 1, 1])
        let signers_bv = bitvec![0, 1, 1];
        let qc = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::assemble(
            &qc_pp,
            signers_bv.as_bitslice(),
            &[setup.sig2, setup.sig3],
        )
        .unwrap();
        let result = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::signers(&qc_pp, &qc).unwrap();
        assert_eq!(
            result,
            vec![setup.key_pair2.ver_key(), setup.key_pair3.ver_key()]
        );
    }

    #[test]
    fn test_signers_different_subset() {
        let setup = three_node_setup();
        let qc_pp = QcParams {
            stake_entries: &setup.entries,
            threshold: U256::from(2u8),
            agg_sig_pp: (),
        };
        // Nodes 1 and 3 sign (bitvec [1, 0, 1])
        let signers_bv = bitvec![1, 0, 1];
        let qc = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::assemble(
            &qc_pp,
            signers_bv.as_bitslice(),
            &[setup.sig1, setup.sig3],
        )
        .unwrap();
        let result = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::signers(&qc_pp, &qc).unwrap();
        assert_eq!(
            result,
            vec![setup.key_pair1.ver_key(), setup.key_pair3.ver_key()]
        );
    }

    #[test]
    fn test_signers_all_participants() {
        let setup = three_node_setup();
        let qc_pp = QcParams {
            stake_entries: &setup.entries,
            threshold: U256::from(2u8),
            agg_sig_pp: (),
        };
        let signers_bv = bitvec![1, 1, 1];
        let qc = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::assemble(
            &qc_pp,
            signers_bv.as_bitslice(),
            &[setup.sig1, setup.sig2, setup.sig3],
        )
        .unwrap();
        let result = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::signers(&qc_pp, &qc).unwrap();
        assert_eq!(
            result,
            vec![
                setup.key_pair1.ver_key(),
                setup.key_pair2.ver_key(),
                setup.key_pair3.ver_key()
            ]
        );
    }

    #[test]
    fn test_signers_no_participants() {
        // signers() does NOT check the threshold - it just reads the bitvec.
        // We build the (sig, bitvec) tuple directly to avoid the threshold check in assemble().
        let setup = three_node_setup();
        let qc_pp = QcParams {
            stake_entries: &setup.entries,
            threshold: U256::from(2u8),
            agg_sig_pp: (),
        };
        // Use sig1 as the dummy aggregated signature; signers() only reads the bitvec.
        let empty_bv = bitvec![0, 0, 0];
        let qc = (setup.sig1, empty_bv);
        let result = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::signers(&qc_pp, &qc).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_signers_does_not_check_message() {
        // signers() only reads the bitvec, it does NOT verify the message.
        // So calling it with a QC assembled over message A but passing a different message is fine.
        let setup = three_node_setup();
        let qc_pp = QcParams {
            stake_entries: &setup.entries,
            threshold: U256::from(2u8),
            agg_sig_pp: (),
        };
        let signers_bv = bitvec![0, 1, 1];
        // Assemble QC over the "real" message (msg = [42u8; 32] from three_node_setup)
        let qc = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::assemble(
            &qc_pp,
            signers_bv.as_bitslice(),
            &[setup.sig2, setup.sig3],
        )
        .unwrap();
        // signers() succeeds regardless of any message - it only reads the bitvec.
        let result = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::signers(&qc_pp, &qc).unwrap();
        assert_eq!(
            result,
            vec![setup.key_pair2.ver_key(), setup.key_pair3.ver_key()]
        );
    }

    #[test]
    fn test_signers_bitvec_length_mismatch() {
        let setup = three_node_setup();
        let qc_pp = QcParams {
            stake_entries: &setup.entries,
            threshold: U256::from(2u8),
            agg_sig_pp: (),
        };
        // qc_pp has 3 stake entries but we create a QC with a bitvec of length 4.
        let wrong_bv = bitvec![0, 1, 1, 0];
        // Use sig1 as a plausible Signature value; signers() won't verify it.
        let qc_bad = (setup.sig1, wrong_bv);
        let result = BitVectorQc::<BLSOverBN254CurveSignatureScheme>::signers(&qc_pp, &qc_bad);
        assert!(result.is_err());
    }
}
