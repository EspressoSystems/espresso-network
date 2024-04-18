use ark_bn254::Bn254;
use ark_ed_on_bn254::EdwardsConfig;
use ark_std::{
    borrow::Borrow,
    rand::{CryptoRng, RngCore},
};
use ethers::types::U256;
use hotshot_types::light_client::{CircuitField, LightClientState, PublicInput, StateVerKey};
use jf_plonk::{
    errors::PlonkError,
    proof_system::{PlonkKzgSnark, UniversalSNARK},
    transcript::SolidityTranscript,
};
use jf_primitives::signatures::schnorr::Signature;

/// BLS verification key, base field and Schnorr verification key
pub use hotshot_stake_table::vec_based::config::QCVerKey;
/// Proving key
pub type ProvingKey = jf_plonk::proof_system::structs::ProvingKey<Bn254>;
/// Verifying key
pub type VerifyingKey = jf_plonk::proof_system::structs::VerifyingKey<Bn254>;
/// Proof
pub type Proof = jf_plonk::proof_system::structs::Proof<Bn254>;
/// Universal SRS
pub type UniversalSrs = jf_plonk::proof_system::structs::UniversalSrs<Bn254>;

/// Given a SRS, returns the proving key and verifying key for state update
/// # Errors
/// Errors if unable to preprocess
#[allow(clippy::cast_possible_truncation)]
pub fn preprocess(
    srs: &UniversalSrs,
    stake_table_capacity: usize,
) -> Result<(ProvingKey, VerifyingKey), PlonkError> {
    let (circuit, _) = crate::circuit::build_for_preprocessing::<CircuitField, EdwardsConfig>(
        stake_table_capacity,
    )?;
    PlonkKzgSnark::preprocess(srs, &circuit)
}

/// Given a proving key and
/// - a list of stake table entries (`Vec<(BLSVerKey, Amount, SchnorrVerKey)>`)
/// - a list of schnorr signatures of the updated states (`Vec<SchnorrSignature>`), default if the node doesn't sign the state
/// - updated light client state (`(view_number, block_height, block_comm_root, fee_ledger_comm, stake_table_comm)`)
/// - a bit vector indicates the signers
/// - a quorum threshold
/// Returns error or a pair `(proof, public_inputs)` asserting that
/// - the signer's accumulated weight exceeds the quorum threshold
/// - the stake table corresponds to the one committed in the light client state
/// - all schnorr signatures over the light client state are valid
/// # Errors
/// Errors if unable to generate proof
/// # Panics
/// if the stake table is not up to date
#[allow(clippy::too_many_arguments)]
pub fn generate_state_update_proof<STIter, R, BitIter, SigIter>(
    rng: &mut R,
    pk: &ProvingKey,
    stake_table_entries: STIter,
    signer_bit_vec: BitIter,
    signatures: SigIter,
    lightclient_state: &LightClientState,
    threshold: &U256,
    stake_table_capacity: usize,
) -> Result<(Proof, PublicInput), PlonkError>
where
    STIter: IntoIterator,
    STIter::Item: Borrow<(StateVerKey, U256)>,
    STIter::IntoIter: ExactSizeIterator,
    R: CryptoRng + RngCore,
    BitIter: IntoIterator,
    BitIter::Item: Borrow<bool>,
    BitIter::IntoIter: ExactSizeIterator,
    SigIter: IntoIterator,
    SigIter::Item: Borrow<Signature<EdwardsConfig>>,
    SigIter::IntoIter: ExactSizeIterator,
{
    let signer_bit_vec = signer_bit_vec.into_iter().map(|b| {
        if *b.borrow() {
            CircuitField::from(1u64)
        } else {
            CircuitField::from(0u64)
        }
    });
    let (circuit, public_inputs) = crate::circuit::build::<_, _, _, _, _>(
        stake_table_entries,
        signer_bit_vec,
        signatures,
        lightclient_state,
        threshold,
        stake_table_capacity,
    )?;
    let proof = PlonkKzgSnark::<Bn254>::prove::<_, _, SolidityTranscript>(rng, &circuit, pk, None)?;
    Ok((proof, public_inputs))
}

#[cfg(test)]
mod tests {
    use super::{generate_state_update_proof, preprocess, CircuitField, UniversalSrs};
    use crate::{
        circuit::build_for_preprocessing,
        test_utils::{key_pairs_for_testing, stake_table_for_testing},
    };
    use ark_bn254::Bn254;
    use ark_ec::pairing::Pairing;
    use ark_ed_on_bn254::EdwardsConfig as Config;
    use ark_std::{
        rand::{CryptoRng, RngCore},
        One,
    };
    use ethers::types::U256;
    use hotshot_types::{
        light_client::GenericLightClientState,
        traits::stake_table::{SnapshotVersion, StakeTableScheme},
    };
    use jf_plonk::{
        proof_system::{PlonkKzgSnark, UniversalSNARK},
        transcript::SolidityTranscript,
    };
    use jf_primitives::{
        crhf::{VariableLengthRescueCRHF, CRHF},
        errors::PrimitivesError,
        signatures::{schnorr::Signature, SchnorrSignatureScheme, SignatureScheme},
    };
    use jf_relation::Circuit;
    use jf_utils::test_rng;

    const ST_CAPACITY: usize = 20;

    // FIXME(Chengyu): see <https://github.com/EspressoSystems/jellyfish/issues/249>
    #[allow(clippy::unnecessary_wraps)]
    fn universal_setup_for_testing<R>(
        max_degree: usize,
        rng: &mut R,
    ) -> Result<UniversalSrs, PrimitivesError>
    where
        R: RngCore + CryptoRng,
    {
        use ark_ec::{scalar_mul::fixed_base::FixedBase, CurveGroup};
        use ark_ff::PrimeField;
        use ark_std::{end_timer, start_timer, UniformRand};

        let setup_time = start_timer!(|| format!("KZG10::Setup with degree {}", max_degree));
        let beta = <Bn254 as Pairing>::ScalarField::rand(rng);
        let g = <Bn254 as Pairing>::G1::rand(rng);
        let h = <Bn254 as Pairing>::G2::rand(rng);

        let mut powers_of_beta = vec![<Bn254 as Pairing>::ScalarField::one()];

        let mut cur = beta;
        for _ in 0..max_degree {
            powers_of_beta.push(cur);
            cur *= &beta;
        }

        let window_size = FixedBase::get_mul_window_size(max_degree + 1);

        let scalar_bits = <Bn254 as Pairing>::ScalarField::MODULUS_BIT_SIZE as usize;
        let g_time = start_timer!(|| "Generating powers of G");
        // TODO: parallelization
        let g_table = FixedBase::get_window_table(scalar_bits, window_size, g);
        let powers_of_g = FixedBase::msm::<<Bn254 as Pairing>::G1>(
            scalar_bits,
            window_size,
            &g_table,
            &powers_of_beta,
        );
        end_timer!(g_time);

        let powers_of_g = <Bn254 as Pairing>::G1::normalize_batch(&powers_of_g);

        let h = h.into_affine();
        let beta_h = (h * beta).into_affine();

        let pp = UniversalSrs {
            powers_of_g,
            h,
            beta_h,
            powers_of_h: vec![h, beta_h],
        };
        end_timer!(setup_time);
        Ok(pp)
    }

    #[test]
    fn test_proof_generation() {
        let num_validators = 10;
        let mut prng = test_rng();

        let (bls_keys, schnorr_keys) = key_pairs_for_testing(num_validators, &mut prng);
        let st = stake_table_for_testing(ST_CAPACITY, &bls_keys, &schnorr_keys);

        let stake_table_entries = st
            .try_iter(SnapshotVersion::LastEpochStart)
            .unwrap()
            .map(|(_, stake_amount, schnorr_key)| (schnorr_key, stake_amount))
            .collect::<Vec<_>>();

        let block_comm_root = VariableLengthRescueCRHF::<CircuitField, 1>::evaluate(vec![
            CircuitField::from(1u32),
            CircuitField::from(2u32),
        ])
        .unwrap()[0];
        let fee_ledger_comm = VariableLengthRescueCRHF::<CircuitField, 1>::evaluate(vec![
            CircuitField::from(3u32),
            CircuitField::from(5u32),
        ])
        .unwrap()[0];

        let lightclient_state = GenericLightClientState {
            view_number: 100,
            block_height: 73,
            block_comm_root,
            fee_ledger_comm,
            stake_table_comm: st.commitment(SnapshotVersion::LastEpochStart).unwrap(),
        };
        let state_msg: [CircuitField; 7] = lightclient_state.clone().into();

        let sigs = schnorr_keys
            .iter()
            .map(|(key, _)| SchnorrSignatureScheme::<Config>::sign(&(), key, state_msg, &mut prng))
            .collect::<Result<Vec<_>, PrimitivesError>>()
            .unwrap();

        // bit vector with total weight 26
        let bit_vec = [
            true, true, true, false, true, true, false, false, true, false,
        ];
        let bit_masked_sigs = bit_vec
            .iter()
            .zip(sigs.iter())
            .map(|(bit, sig)| {
                if *bit {
                    sig.clone()
                } else {
                    Signature::<Config>::default()
                }
            })
            .collect::<Vec<_>>();

        // good path
        let num_gates =
            build_for_preprocessing::<CircuitField, ark_ed_on_bn254::EdwardsConfig>(ST_CAPACITY)
                .unwrap()
                .0
                .num_gates();
        let test_srs = universal_setup_for_testing(num_gates + 2, &mut prng).unwrap();
        ark_std::println!("Number of constraint in the circuit: {num_gates}");

        let result = preprocess(&test_srs, ST_CAPACITY);
        assert!(result.is_ok());
        let (pk, vk) = result.unwrap();

        let result = generate_state_update_proof::<_, _, _, _>(
            &mut prng,
            &pk,
            &stake_table_entries,
            &bit_vec,
            &bit_masked_sigs,
            &lightclient_state,
            &U256::from(26u32),
            ST_CAPACITY,
        );
        assert!(result.is_ok());

        let (proof, public_inputs) = result.unwrap();
        assert!(PlonkKzgSnark::<Bn254>::verify::<SolidityTranscript>(
            &vk,
            public_inputs.as_ref(),
            &proof,
            None
        )
        .is_ok());

        // minimum bad path, other bad cases are checked inside `circuit.rs`
        let result = generate_state_update_proof::<_, _, _, _>(
            &mut prng,
            &pk,
            &stake_table_entries,
            &bit_vec,
            &bit_masked_sigs,
            &lightclient_state,
            &U256::from(100u32),
            ST_CAPACITY,
        );
        assert!(result.is_err());
    }
}
