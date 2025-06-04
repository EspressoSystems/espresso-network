use std::collections::HashMap;

use alloy::primitives::U256;
use anyhow::Result;
use ark_bn254::Bn254;
use ark_ed_on_bn254::EdwardsConfig;
use ark_ff::PrimeField;
use ark_std::{
    rand::{rngs::StdRng, CryptoRng, Rng, RngCore},
    UniformRand,
};
use espresso_types::SeqTypes;
use hotshot_contract_adapter::{field_to_u256, jellyfish::open_key};
use hotshot_types::{
    light_client::{GenericLightClientState, GenericStakeTableState, LightClientState},
    stake_table::{HSStakeTable, StakeTableEntry},
    PeerConfig,
};
use itertools::izip;
use jf_pcs::prelude::UnivariateUniversalParams;
use jf_plonk::{
    proof_system::{PlonkKzgSnark, UniversalSNARK},
    transcript::SolidityTranscript,
};
use jf_relation::{Arithmetization, Circuit, PlonkCircuit};
use jf_signature::{
    bls_over_bn254::{BLSOverBN254CurveSignatureScheme, VerKey as BLSVerKey},
    schnorr::{SchnorrSignatureScheme, Signature},
    SignatureScheme,
};
use jf_utils::test_rng;

use crate::legacy::{
    circuit::GenericPublicInput, generate_state_update_proof, preprocess, Proof, VerifyingKey,
};

type F = ark_ed_on_bn254::Fq;
type SchnorrVerKey = jf_signature::schnorr::VerKey<EdwardsConfig>;
type SchnorrSignKey = jf_signature::schnorr::SignKey<ark_ed_on_bn254::Fr>;

/// Stake table capacity used for testing
pub const STAKE_TABLE_CAPACITY_FOR_TEST: usize = 10;

/// Mock for system parameter of `MockLedger`
pub struct MockSystemParam {
    /// max capacity of stake table
    st_cap: usize,
}

impl MockSystemParam {
    /// Init the system parameters (some fixed, some adjustable)
    pub fn init() -> Self {
        Self {
            st_cap: STAKE_TABLE_CAPACITY_FOR_TEST,
        }
    }
}

/// Mock of hotshot ledger for testing LightClient.sol functionalities only.
/// Its logic is completely divergent from a real light client or HotShot
pub struct MockLedger {
    pp: MockSystemParam,
    pub rng: StdRng,
    pub(crate) state: GenericLightClientState<F>,
    pub(crate) voting_st: HSStakeTable<SeqTypes>,
    key_archive: HashMap<BLSVerKey, SchnorrSignKey>,
}

impl MockLedger {
    /// Initialize the ledger with genesis state
    pub fn init(pp: MockSystemParam, num_validators: usize) -> Self {
        // credit: https://github.com/EspressoSystems/HotShot/blob/5554b7013b00e6034691b533299b44f3295fa10d/crates/hotshot-state-prover/src/lib.rs#L176
        let mut rng = test_rng();
        let (qc_keys, state_keys) = key_pairs_for_testing(num_validators, &mut rng);
        let mut key_archive = HashMap::new();
        for i in 0..qc_keys.len() {
            key_archive.insert(qc_keys[i], state_keys[i].0.clone());
        }
        let voting_st = stake_table_for_testing(&qc_keys, &state_keys);

        // arbitrary commitment values as they don't affect logic being tested
        let block_comm_root = F::from(1234);
        let genesis = LightClientState {
            view_number: 0,
            block_height: 0,
            block_comm_root,
        };

        Self {
            pp,
            rng,
            state: genesis,
            voting_st,
            key_archive,
        }
    }

    /// Elapse a view with a new finalized block
    pub fn elapse_with_block(&mut self) {
        self.state.view_number += 1;
        self.state.block_height += 1;
        self.state.block_comm_root = self.new_dummy_comm();
    }

    /// Elapse a view without a new finalized block
    /// (e.g. insufficient votes, malicious leaders or inconsecutive noterized views)
    pub fn elapse_without_block(&mut self) {
        self.state.view_number += 1;
    }

    /// Return the light client state and proof of consensus on this finalized state
    pub fn gen_state_proof(&mut self) -> (GenericPublicInput<F>, Proof) {
        let voting_st_state = self.voting_stake_table_state();

        let mut msg = Vec::with_capacity(7);
        let state_msg: [F; 3] = self.state.into();
        msg.extend_from_slice(&state_msg);

        let st: Vec<(BLSVerKey, U256, SchnorrVerKey)> = self
            .voting_st
            .iter()
            .map(|config| {
                (
                    config.stake_table_entry.stake_key,
                    config.stake_table_entry.stake_amount,
                    config.state_ver_key.clone(),
                )
            })
            .collect();
        let st_size = st.len();

        // find a quorum whose accumulated weights exceed threshold
        let mut bit_vec = vec![false; st_size];
        let mut total_weight = U256::from(0);
        while total_weight < field_to_u256(voting_st_state.threshold) {
            let signer_idx = self.rng.gen_range(0..st_size);
            // if already selected, skip to next random sample
            if bit_vec[signer_idx] {
                continue;
            }

            bit_vec[signer_idx] = true;
            total_weight += st[signer_idx].1;
        }

        let sigs = bit_vec
            .iter()
            .enumerate()
            .map(|(i, b)| {
                if *b {
                    SchnorrSignatureScheme::<EdwardsConfig>::sign(
                        &(),
                        self.key_archive.get(&st[i].0).unwrap(),
                        &msg,
                        &mut self.rng,
                    )
                } else {
                    Ok(Signature::<EdwardsConfig>::default())
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let srs = {
            // load SRS from Aztec's ceremony
            let srs = ark_srs::kzg10::aztec20::setup(2u64.pow(16) as usize + 2)
                .expect("Aztec SRS fail to load");
            // convert to Jellyfish type
            // TODO: (alex) use constructor instead https://github.com/EspressoSystems/jellyfish/issues/440
            UnivariateUniversalParams {
                powers_of_g: srs.powers_of_g,
                h: srs.h,
                beta_h: srs.beta_h,
                powers_of_h: vec![srs.h, srs.beta_h],
            }
        };
        let (pk, _) =
            preprocess(&srs, self.pp.st_cap).expect("Fail to preprocess state prover circuit");
        let stake_table_entries = st
            .into_iter()
            .map(|(_, stake_amount, schnorr_key)| (schnorr_key, stake_amount))
            .collect::<Vec<_>>();
        let (proof, pi) = generate_state_update_proof(
            &mut self.rng,
            &pk,
            &stake_table_entries,
            &bit_vec,
            &sigs,
            &self.state,
            &voting_st_state,
            self.pp.st_cap,
        )
        .expect("Fail to generate state proof");

        (pi, proof)
    }

    /// a malicious attack, generating a fake stake table full of adversarial stakers
    /// adv-controlled stakers signed the state and replace the stake table commitment with that of the fake one
    /// in an attempt to hijack the correct stake table.
    pub fn gen_state_proof_with_fake_stakers(
        &mut self,
    ) -> (GenericPublicInput<F>, Proof, GenericStakeTableState<F>) {
        let new_state = self.state;

        let (adv_qc_keys, adv_state_keys) = key_pairs_for_testing(self.pp.st_cap, &mut self.rng);
        let adv_st = stake_table_for_testing(&adv_qc_keys, &adv_state_keys);
        let adv_st_state = adv_st.commitment(self.pp.st_cap).unwrap();

        // replace new state with adversarial stake table commitment
        let mut msg = Vec::with_capacity(7);
        let state_msg: [F; 3] = new_state.into();
        msg.extend_from_slice(&state_msg);

        // every fake stakers sign on the adverarial new state
        let bit_vec = vec![true; self.pp.st_cap];
        let sigs = adv_state_keys
            .iter()
            .map(|(sk, _)| {
                SchnorrSignatureScheme::<EdwardsConfig>::sign(&(), sk, &msg, &mut self.rng)
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let srs = {
            // load SRS from Aztec's ceremony
            let srs = ark_srs::kzg10::aztec20::setup(2u64.pow(16) as usize + 2)
                .expect("Aztec SRS fail to load");
            // convert to Jellyfish type
            // TODO: (alex) use constructor instead https://github.com/EspressoSystems/jellyfish/issues/440
            UnivariateUniversalParams {
                powers_of_g: srs.powers_of_g,
                h: srs.h,
                beta_h: srs.beta_h,
                powers_of_h: vec![srs.h, srs.beta_h],
            }
        };
        let (pk, _) =
            preprocess(&srs, self.pp.st_cap).expect("Fail to preprocess state prover circuit");
        let stake_table_entries = adv_st
            .0
            .into_iter()
            .map(|config| (config.state_ver_key, config.stake_table_entry.stake_amount))
            .collect::<Vec<_>>();
        let (proof, pi) = generate_state_update_proof::<_, _, _, _>(
            &mut self.rng,
            &pk,
            &stake_table_entries,
            &bit_vec,
            &sigs,
            &new_state,
            &adv_st_state,
            self.pp.st_cap,
        )
        .expect("Fail to generate state proof");

        (pi, proof, adv_st_state)
    }

    /// Returns the stake table state for current voting
    pub fn voting_stake_table_state(&self) -> GenericStakeTableState<F> {
        self.voting_st
            .commitment(self.pp.st_cap)
            .expect("Failed to compute stake table commitment")
    }

    /// Returns the light client state
    pub fn light_client_state(&self) -> GenericLightClientState<F> {
        self.state
    }

    // return a dummy commitment value
    fn new_dummy_comm(&mut self) -> F {
        F::rand(&mut self.rng)
    }
}

/// Helper function for test
fn key_pairs_for_testing<R: CryptoRng + RngCore>(
    num_validators: usize,
    prng: &mut R,
) -> (Vec<BLSVerKey>, Vec<(SchnorrSignKey, SchnorrVerKey)>) {
    let bls_keys = (0..num_validators)
        .map(|_| {
            BLSOverBN254CurveSignatureScheme::key_gen(&(), prng)
                .unwrap()
                .1
        })
        .collect::<Vec<_>>();
    let schnorr_keys = (0..num_validators)
        .map(|_| SchnorrSignatureScheme::key_gen(&(), prng).unwrap())
        .collect::<Vec<_>>();
    (bls_keys, schnorr_keys)
}

/// Helper function for test
fn stake_table_for_testing(
    bls_keys: &[BLSVerKey],
    schnorr_keys: &[(SchnorrSignKey, SchnorrVerKey)],
) -> HSStakeTable<SeqTypes> {
    bls_keys
        .iter()
        .enumerate()
        .zip(schnorr_keys)
        .map(|((i, bls_key), (_, schnorr_key))| PeerConfig {
            stake_table_entry: StakeTableEntry {
                stake_key: *bls_key,
                stake_amount: U256::from((i + 1) as u32),
            },
            state_ver_key: schnorr_key.clone(),
        })
        .collect::<Vec<_>>()
        .into()
}

// modify from <https://github.com/EspressoSystems/cape/blob/main/contracts/rust/src/plonk_verifier/helpers.rs>
/// return list of (proof, ver_key, public_input, extra_msg, domain_size)
#[allow(clippy::type_complexity)]
pub fn gen_plonk_proof_for_test(
    num_proof: usize,
) -> Vec<(Proof, VerifyingKey, Vec<F>, Option<Vec<u8>>, usize)> {
    // 1. Simulate universal setup
    let rng = &mut jf_utils::test_rng();
    let srs = {
        let aztec_srs = ark_srs::kzg10::aztec20::setup(1024).expect("Aztec SRS fail to load");

        UnivariateUniversalParams {
            powers_of_g: aztec_srs.powers_of_g,
            h: aztec_srs.h,
            beta_h: aztec_srs.beta_h,
            powers_of_h: vec![aztec_srs.h, aztec_srs.beta_h],
        }
    };
    let open_key = open_key();
    assert_eq!(srs.h, open_key.h);
    assert_eq!(srs.beta_h, open_key.beta_h);
    assert_eq!(srs.powers_of_g[0], open_key.g);

    // 2. Create circuits
    let circuits = (0..num_proof)
        .map(|i| {
            let m = 2 + i / 3;
            let a0 = 1 + i % 3;
            gen_circuit_for_test::<F>(m, a0)
        })
        .collect::<Result<Vec<_>>>()
        .expect("Test circuits fail to create");
    let domain_sizes: Vec<usize> = circuits
        .iter()
        .map(|c| c.eval_domain_size().unwrap())
        .collect();

    // 3. Preprocessing
    let mut prove_keys = vec![];
    let mut ver_keys = vec![];
    for c in circuits.iter() {
        let (pk, vk) =
            PlonkKzgSnark::<Bn254>::preprocess(&srs, c).expect("Circuit preprocessing failed");
        prove_keys.push(pk);
        ver_keys.push(vk);
    }

    // 4. Proving
    let mut proofs = vec![];
    let mut extra_msgs = vec![];

    circuits.iter().zip(prove_keys.iter()).for_each(|(cs, pk)| {
        let extra_msg = Some(vec![]); // We set extra_msg="" for the contract tests to pass
        proofs.push(
            PlonkKzgSnark::<Bn254>::prove::<_, _, SolidityTranscript>(
                rng,
                cs,
                pk,
                extra_msg.clone(),
            )
            .unwrap(),
        );
        extra_msgs.push(extra_msg);
    });

    let public_inputs: Vec<Vec<F>> = circuits
        .iter()
        .map(|cs| cs.public_input().unwrap())
        .collect();

    izip!(proofs, ver_keys, public_inputs, extra_msgs, domain_sizes).collect()
}

// Different `m`s lead to different circuits.
// Different `a0`s lead to different witness values.
pub fn gen_circuit_for_test<F: PrimeField>(m: usize, a0: usize) -> Result<PlonkCircuit<F>> {
    let mut cs: PlonkCircuit<F> = PlonkCircuit::new_turbo_plonk();
    // Create variables
    let mut a = vec![];
    for i in a0..(a0 + 4 * m) {
        a.push(cs.create_variable(F::from(i as u64))?);
    }
    let b = [
        cs.create_public_variable(F::from(m as u64 * 2))?,
        cs.create_public_variable(F::from(a0 as u64 * 2 + m as u64 * 4 - 1))?,
    ];
    let c = cs.create_public_variable(
        (cs.witness(b[1])? + cs.witness(a[0])?) * (cs.witness(b[1])? - cs.witness(a[0])?),
    )?;

    // Create other public variables so that the number of public inputs is 11
    for i in 0..8u32 {
        cs.create_public_variable(F::from(i))?;
    }

    // Create gates:
    // 1. a0 + ... + a_{4*m-1} = b0 * b1
    // 2. (b1 + a0) * (b1 - a0) = c
    // 3. b0 = 2 * m
    let mut acc = cs.zero();
    a.iter().for_each(|&elem| acc = cs.add(acc, elem).unwrap());
    let b_mul = cs.mul(b[0], b[1])?;
    cs.enforce_equal(acc, b_mul)?;
    let b1_plus_a0 = cs.add(b[1], a[0])?;
    let b1_minus_a0 = cs.sub(b[1], a[0])?;
    cs.mul_gate(b1_plus_a0, b1_minus_a0, c)?;
    cs.enforce_constant(b[0], F::from(m as u64 * 2))?;

    // Finalize the circuit.
    cs.finalize_for_arithmetization()?;

    Ok(cs)
}
