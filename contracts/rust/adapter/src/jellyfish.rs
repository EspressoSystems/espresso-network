//! Helpers for connecting types between Jellyfish and Solidity.
//! Usually used during differential testing (via FFI).

use alloy::{
    hex::ToHexExt,
    primitives::{B256, U256},
};
use ark_bn254::{Bn254, Fq, Fr, G1Affine, G2Affine};
use ark_ec::{
    short_weierstrass::{Affine, SWCurveConfig},
    twisted_edwards::{self, TECurveConfig},
    AffineRepr,
};
use ark_ff::{Fp2, Fp2Config, MontFp, PrimeField};
use ark_std::{rand::Rng, UniformRand};
use jf_pcs::prelude::Commitment;
use jf_plonk::{
    constants::KECCAK256_STATE_SIZE,
    proof_system::structs::{OpenKey, Proof, ProofEvaluations, VerifyingKey},
    testing_apis::Challenges,
    transcript::SolidityTranscript,
};
use jf_utils::to_bytes;
use num_bigint::BigUint;
use num_traits::Num;

use crate::{field_to_u256, sol_types::*, u256_to_field};

// this is convention from BN256 precompile
impl Default for G1PointSol {
    fn default() -> Self {
        Self {
            x: U256::from(0),
            y: U256::from(0),
        }
    }
}
impl PartialEq for G1PointSol {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<P: SWCurveConfig> From<Affine<P>> for G1PointSol
where
    P::BaseField: PrimeField,
{
    fn from(p: Affine<P>) -> Self {
        if p.is_zero() {
            // this convention is from the BN precompile
            Self {
                x: U256::from(0),
                y: U256::from(0),
            }
        } else {
            Self {
                x: field_to_u256::<P::BaseField>(*p.x().unwrap()),
                y: field_to_u256::<P::BaseField>(*p.y().unwrap()),
            }
        }
    }
}

impl<P: SWCurveConfig> From<G1PointSol> for Affine<P>
where
    P::BaseField: PrimeField,
{
    fn from(p: G1PointSol) -> Self {
        if p == G1PointSol::default() {
            Self::default()
        } else {
            Self::new_unchecked(
                u256_to_field::<P::BaseField>(p.x),
                u256_to_field::<P::BaseField>(p.y),
            )
        }
    }
}

impl<P: SWCurveConfig<BaseField = Fp2<C>>, C> From<G2PointSol> for Affine<P>
where
    C: Fp2Config,
{
    fn from(p: G2PointSol) -> Self {
        Self::new_unchecked(
            Fp2::new(u256_to_field(p.x0), u256_to_field(p.x1)),
            Fp2::new(u256_to_field(p.y0), u256_to_field(p.y1)),
        )
    }
}

impl<P: SWCurveConfig<BaseField = Fp2<C>>, C> From<Affine<P>> for G2PointSol
where
    C: Fp2Config,
{
    fn from(p: Affine<P>) -> Self {
        Self {
            x0: field_to_u256(p.x().unwrap().c0),
            x1: field_to_u256(p.x().unwrap().c1),
            y0: field_to_u256(p.y().unwrap().c0),
            y1: field_to_u256(p.y().unwrap().c1),
        }
    }
}

impl<P: TECurveConfig> From<twisted_edwards::Affine<P>> for EdOnBN254PointSol
where
    P::BaseField: PrimeField,
{
    fn from(p: twisted_edwards::Affine<P>) -> Self {
        Self {
            x: field_to_u256::<P::BaseField>(*p.x().unwrap()),
            y: field_to_u256::<P::BaseField>(*p.y().unwrap()),
        }
    }
}

impl<P: TECurveConfig> From<EdOnBN254PointSol> for twisted_edwards::Affine<P>
where
    P::BaseField: PrimeField,
{
    fn from(p: EdOnBN254PointSol) -> Self {
        Self::new_unchecked(
            u256_to_field::<P::BaseField>(p.x),
            u256_to_field::<P::BaseField>(p.y),
        )
    }
}

// constant in hex string copied from hardcoded constants from solidity contracts

const COSET: [&str; 5] = [
    "1",
    "2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a",
    "1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb025",
    "2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a",
    "2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e881",
];

// H: G2Affine(x: Fp2, y:Fp2), x = x0 + u * x1, y = y0 + u * y1
const H: [&str; 4] = [
    "1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed", // x0
    "198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2", // x1
    "12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa", // y0
    "090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b", // y1
];

// See notes about `const H` above.
const BETA_H: [&str; 4] = [
    "0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b0",
    "260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c1",
    "22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e55",
    "04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe4",
];

// TODO: (alex) change to simply using `MontFp!("0x..")` after
// <https://github.com/arkworks-rs/algebra/pull/635> is on a tag release
// Return cosets coefficients for circuits over BN254.
pub fn coset_k() -> Vec<Fr> {
    vec![
        Fr::from(BigUint::from_str_radix(COSET[0], 16).unwrap()),
        Fr::from(BigUint::from_str_radix(COSET[1], 16).unwrap()),
        Fr::from(BigUint::from_str_radix(COSET[2], 16).unwrap()),
        Fr::from(BigUint::from_str_radix(COSET[3], 16).unwrap()),
        Fr::from(BigUint::from_str_radix(COSET[4], 16).unwrap()),
    ]
}

/// Returns `OpenKeys` for KZG10 over BN254 curve from Aztec's SRS
pub fn open_key() -> OpenKey<Bn254> {
    let g = G1Affine::new_unchecked(MontFp!("1"), MontFp!("2"));
    let h = G2Affine::new(
        Fp2::new(
            Fq::from(BigUint::from_str_radix(H[0], 16).unwrap()),
            Fq::from(BigUint::from_str_radix(H[1], 16).unwrap()),
        ),
        Fp2::new(
            Fq::from(BigUint::from_str_radix(H[2], 16).unwrap()),
            Fq::from(BigUint::from_str_radix(H[3], 16).unwrap()),
        ),
    );
    let beta_h = G2Affine::new(
        Fp2::new(
            Fq::from(BigUint::from_str_radix(BETA_H[0], 16).unwrap()),
            Fq::from(BigUint::from_str_radix(BETA_H[1], 16).unwrap()),
        ),
        Fp2::new(
            Fq::from(BigUint::from_str_radix(BETA_H[2], 16).unwrap()),
            Fq::from(BigUint::from_str_radix(BETA_H[3], 16).unwrap()),
        ),
    );

    OpenKey {
        g,
        h,
        beta_h,
        powers_of_g: vec![g],
        powers_of_h: vec![h, beta_h],
    }
}

impl From<SolidityTranscript> for TranscriptDataSol {
    fn from(t: SolidityTranscript) -> Self {
        let (state, transcript) = t.internal();
        Self {
            state: B256::from_slice(&state),
            transcript: transcript.into(),
        }
    }
}

impl From<TranscriptDataSol> for SolidityTranscript {
    fn from(t: TranscriptDataSol) -> Self {
        let mut state = [0u8; KECCAK256_STATE_SIZE];
        state.copy_from_slice(&t.state.0);
        Self::from_internal(state, t.transcript.to_vec())
    }
}

impl From<VerifyingKey<Bn254>> for VerifyingKeySol {
    fn from(vk: VerifyingKey<Bn254>) -> Self {
        let g2_bytes = to_bytes!(&vk.open_key.powers_of_h[1]).unwrap();
        assert!(g2_bytes.len() == 64);
        let mut g2_lsb = [0u8; 32];
        let mut g2_msb = [0u8; 32];
        g2_lsb.copy_from_slice(&g2_bytes[..32]);
        g2_msb.copy_from_slice(&g2_bytes[32..]);

        // since G2 point from the Aztec's SRS we use is fixed
        // remove these sanity check if using other SRS
        // generated via:
        // ```rust
        // let srs = ark_srs::kzg10::aztec20::setup(2u64.pow(6) as usize + 2).expect("Aztec SRS fail to load");
        // println!("{}", hex::encode(jf_utils::to_bytes!(&srs.beta_h).unwrap()));
        // ```
        assert_eq!(
            g2_lsb.encode_hex(),
            String::from("b0838893ec1f237e8b07323b0744599f4e97b598b3b589bcc2bc37b8d5c41801")
        );
        assert_eq!(
            g2_msb.encode_hex(),
            String::from("c18393c0fa30fe4e8b038e357ad851eae8de9107584effe7c7f1f651b2010e26")
        );

        Self {
            domainSize: U256::from(vk.domain_size),
            numInputs: U256::from(vk.num_inputs),
            sigma0: vk.sigma_comms[0].0.into(),
            sigma1: vk.sigma_comms[1].0.into(),
            sigma2: vk.sigma_comms[2].0.into(),
            sigma3: vk.sigma_comms[3].0.into(),
            sigma4: vk.sigma_comms[4].0.into(),
            q1: vk.selector_comms[0].0.into(),
            q2: vk.selector_comms[1].0.into(),
            q3: vk.selector_comms[2].0.into(),
            q4: vk.selector_comms[3].0.into(),
            qM12: vk.selector_comms[4].0.into(),
            qM34: vk.selector_comms[5].0.into(),
            qH1: vk.selector_comms[6].0.into(),
            qH2: vk.selector_comms[7].0.into(),
            qH3: vk.selector_comms[8].0.into(),
            qH4: vk.selector_comms[9].0.into(),
            qO: vk.selector_comms[10].0.into(),
            qC: vk.selector_comms[11].0.into(),
            qEcc: vk.selector_comms[12].0.into(),
            g2LSB: g2_lsb.into(),
            g2MSB: g2_msb.into(),
        }
    }
}

impl From<VerifyingKeySol> for VerifyingKey<Bn254> {
    fn from(vk: VerifyingKeySol) -> Self {
        let sigma_comms = vec![
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.sigma0)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.sigma1)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.sigma2)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.sigma3)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.sigma4)),
        ];

        let selector_comms = vec![
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.q1)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.q2)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.q3)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.q4)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.qM12)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.qM34)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.qH1)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.qH2)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.qH3)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.qH4)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.qO)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.qC)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(vk.qEcc)),
        ];

        Self {
            domain_size: vk.domainSize.to::<usize>(),
            num_inputs: vk.numInputs.to::<usize>(),
            sigma_comms,
            selector_comms,
            k: coset_k(),
            open_key: open_key(),
            is_merged: false,
            plookup_vk: None,
        }
    }
}

impl From<Proof<Bn254>> for PlonkProofSol {
    fn from(proof: Proof<Bn254>) -> Self {
        Self {
            wire0: proof.wires_poly_comms[0].0.into(),
            wire1: proof.wires_poly_comms[1].0.into(),
            wire2: proof.wires_poly_comms[2].0.into(),
            wire3: proof.wires_poly_comms[3].0.into(),
            wire4: proof.wires_poly_comms[4].0.into(),
            prodPerm: proof.prod_perm_poly_comm.0.into(),
            split0: proof.split_quot_poly_comms[0].0.into(),
            split1: proof.split_quot_poly_comms[1].0.into(),
            split2: proof.split_quot_poly_comms[2].0.into(),
            split3: proof.split_quot_poly_comms[3].0.into(),
            split4: proof.split_quot_poly_comms[4].0.into(),
            zeta: proof.opening_proof.0.into(),
            zetaOmega: proof.shifted_opening_proof.0.into(),
            wireEval0: field_to_u256(proof.poly_evals.wires_evals[0]),
            wireEval1: field_to_u256(proof.poly_evals.wires_evals[1]),
            wireEval2: field_to_u256(proof.poly_evals.wires_evals[2]),
            wireEval3: field_to_u256(proof.poly_evals.wires_evals[3]),
            wireEval4: field_to_u256(proof.poly_evals.wires_evals[4]),
            sigmaEval0: field_to_u256(proof.poly_evals.wire_sigma_evals[0]),
            sigmaEval1: field_to_u256(proof.poly_evals.wire_sigma_evals[1]),
            sigmaEval2: field_to_u256(proof.poly_evals.wire_sigma_evals[2]),
            sigmaEval3: field_to_u256(proof.poly_evals.wire_sigma_evals[3]),
            prodPermZetaOmegaEval: field_to_u256(proof.poly_evals.perm_next_eval),
        }
    }
}

impl From<PlonkProofSol> for Proof<Bn254> {
    fn from(proof: PlonkProofSol) -> Self {
        let wires_poly_comms = vec![
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.wire0)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.wire1)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.wire2)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.wire3)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.wire4)),
        ];
        let split_quot_poly_comms = vec![
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.split0)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.split1)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.split2)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.split3)),
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.split4)),
        ];
        let prod_perm_poly_comm =
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.prodPerm));
        let opening_proof = Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.zeta));
        let shifted_opening_proof =
            Commitment::from(<G1PointSol as Into<G1Affine>>::into(proof.zetaOmega));

        let wires_evals = vec![
            u256_to_field(proof.wireEval0),
            u256_to_field(proof.wireEval1),
            u256_to_field(proof.wireEval2),
            u256_to_field(proof.wireEval3),
            u256_to_field(proof.wireEval4),
        ];
        let wire_sigma_evals = vec![
            u256_to_field(proof.sigmaEval0),
            u256_to_field(proof.sigmaEval1),
            u256_to_field(proof.sigmaEval2),
            u256_to_field(proof.sigmaEval3),
        ];
        let perm_next_eval = u256_to_field(proof.prodPermZetaOmegaEval);

        Self {
            wires_poly_comms,
            prod_perm_poly_comm,
            split_quot_poly_comms,
            opening_proof,
            shifted_opening_proof,
            poly_evals: ProofEvaluations {
                wires_evals,
                wire_sigma_evals,
                perm_next_eval,
            },
            plookup_proof: None,
        }
    }
}

impl PlonkProofSol {
    /// return a dummy proof instance with random ProofEvaluations fields.
    pub fn dummy_with_rand_proof_evals<R: Rng>(rng: &mut R) -> Self {
        let zero = G1Affine::default();
        Self {
            wire0: zero.into(),
            wire1: zero.into(),
            wire2: zero.into(),
            wire3: zero.into(),
            wire4: zero.into(),
            prodPerm: zero.into(),
            split0: zero.into(),
            split1: zero.into(),
            split2: zero.into(),
            split3: zero.into(),
            split4: zero.into(),
            zeta: zero.into(),
            zetaOmega: zero.into(),
            wireEval0: field_to_u256(Fr::rand(rng)),
            wireEval1: field_to_u256(Fr::rand(rng)),
            wireEval2: field_to_u256(Fr::rand(rng)),
            wireEval3: field_to_u256(Fr::rand(rng)),
            wireEval4: field_to_u256(Fr::rand(rng)),
            sigmaEval0: field_to_u256(Fr::rand(rng)),
            sigmaEval1: field_to_u256(Fr::rand(rng)),
            sigmaEval2: field_to_u256(Fr::rand(rng)),
            sigmaEval3: field_to_u256(Fr::rand(rng)),
            prodPermZetaOmegaEval: field_to_u256(Fr::rand(rng)),
        }
    }

    /// return a dummy proof instance with all random fields
    pub fn dummy<R: Rng>(rng: &mut R) -> Self {
        let mut proof = Self::dummy_with_rand_proof_evals(rng);
        proof.wire0 = G1Affine::rand(rng).into();
        proof.wire1 = G1Affine::rand(rng).into();
        proof.wire2 = G1Affine::rand(rng).into();
        proof.wire3 = G1Affine::rand(rng).into();
        proof.wire4 = G1Affine::rand(rng).into();
        proof.prodPerm = G1Affine::rand(rng).into();
        proof.split0 = G1Affine::rand(rng).into();
        proof.split1 = G1Affine::rand(rng).into();
        proof.split2 = G1Affine::rand(rng).into();
        proof.split3 = G1Affine::rand(rng).into();
        proof.split4 = G1Affine::rand(rng).into();
        proof.zeta = G1Affine::rand(rng).into();
        proof.zetaOmega = G1Affine::rand(rng).into();
        proof
    }
}

impl From<Challenges<Fr>> for ChallengesSol {
    fn from(c: Challenges<Fr>) -> Self {
        let alpha_2 = c.alpha * c.alpha;
        Self {
            alpha: field_to_u256::<Fr>(c.alpha),
            alpha2: field_to_u256::<Fr>(alpha_2),
            alpha3: field_to_u256::<Fr>(c.alpha * alpha_2),
            beta: field_to_u256::<Fr>(c.beta),
            gamma: field_to_u256::<Fr>(c.gamma),
            zeta: field_to_u256::<Fr>(c.zeta),
            v: field_to_u256::<Fr>(c.v),
            u: field_to_u256::<Fr>(c.u),
        }
    }
}

impl From<ChallengesSol> for Challenges<Fr> {
    fn from(c: ChallengesSol) -> Self {
        Self {
            tau: None,
            alpha: u256_to_field(c.alpha),
            beta: u256_to_field(c.beta),
            gamma: u256_to_field(c.gamma),
            zeta: u256_to_field(c.zeta),
            v: u256_to_field(c.v),
            u: u256_to_field(c.u),
        }
    }
}

impl ChallengesSol {
    /// dummy challenges
    #[allow(dead_code)]
    pub fn dummy<R: Rng>(rng: &mut R) -> Self {
        let alpha = Fr::rand(rng);
        let alpha_2 = alpha * alpha;
        let alpha_3 = alpha * alpha_2;
        Self {
            alpha: field_to_u256(alpha),
            alpha2: field_to_u256(alpha_2),
            alpha3: field_to_u256(alpha_3),
            beta: field_to_u256(Fr::rand(rng)),
            gamma: field_to_u256(Fr::rand(rng)),
            zeta: field_to_u256(Fr::rand(rng)),
            v: field_to_u256(Fr::rand(rng)),
            u: field_to_u256(Fr::rand(rng)),
        }
    }
}
