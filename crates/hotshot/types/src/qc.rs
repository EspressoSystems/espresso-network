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
use digest::generic_array::GenericArray;
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
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Hash)]
#[serde(bound(deserialize = ""))]
pub struct QcParams<K: SignatureKey, P: for<'a> Deserialize<'a>> {
    /// the stake table (snapshot) this QC is verified against
    pub stake_entries: Vec<StakeTableEntry<K>>,
    /// threshold for the accumulated "weight" of votes to form a QC
    pub threshold: U256,
    /// public parameter for the aggregated signature scheme
    pub agg_sig_pp: P,
}

impl<A> QuorumCertificateScheme<A> for BitVectorQc<A>
where
    A: AggregateableSignatureSchemes + Serialize + for<'a> Deserialize<'a>,
    A::VerificationKey: SignatureKey,
{
    type QcProverParams = QcParams<A::VerificationKey, A::PublicParameter>;

    // TODO: later with SNARKs we'll use a smaller verifier parameter
    type QcVerifierParams = QcParams<A::VerificationKey, A::PublicParameter>;

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
        qc_pp: &Self::QcProverParams,
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
                        if *b {
                            acc + entry.stake_amount
                        } else {
                            acc
                        }
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
        qc_vp: &Self::QcVerifierParams,
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
                        if *b {
                            acc + entry.stake_amount
                        } else {
                            acc
                        }
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
        qc_vp: &Self::QcVerifierParams,
        message: &GenericArray<<A>::MessageUnit, Self::MessageLength>,
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

        Self::check(qc_vp, message, qc)?;

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
        bls_over_bn254::{BLSOverBN254CurveSignatureScheme, KeyPair},
        SignatureScheme,
    };
    use vbs::{version::StaticVersion, BinarySerializer, Serializer};

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
                stake_entries: vec![entry1, entry2, entry3],
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

            assert_eq!(
                qc_pp,
                Serializer::<Version>::deserialize(
                    &Serializer::<Version>::serialize(&qc_pp).unwrap()
                )
                .unwrap()
            );

            // bad paths
            // number of signatures unmatch
            assert!(BitVectorQc::<$aggsig>::assemble(
                &qc_pp,
                signers.as_bitslice(),
                &[sig2.clone()]
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
}
