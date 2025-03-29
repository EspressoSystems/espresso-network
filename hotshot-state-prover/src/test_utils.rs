use alloy::primitives::U256;
use ark_ed_on_bn254::EdwardsConfig;
use ark_std::rand::{CryptoRng, RngCore};
use hotshot_stake_table::vec_based::StakeTable;
use hotshot_types::traits::stake_table::StakeTableScheme;
use jf_signature::{
    bls_over_bn254::{BLSOverBN254CurveSignatureScheme, VerKey as BLSVerKey},
    schnorr::SchnorrSignatureScheme,
    SignatureScheme,
};

type F = ark_ed_on_bn254::Fq;
type SchnorrVerKey = jf_signature::schnorr::VerKey<EdwardsConfig>;
type SchnorrSignKey = jf_signature::schnorr::SignKey<ark_ed_on_bn254::Fr>;

/// Helper function for test
pub(crate) fn key_pairs_for_testing<R: CryptoRng + RngCore>(
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
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn stake_table_for_testing(
    capacity: usize,
    bls_keys: &[BLSVerKey],
    schnorr_keys: &[(SchnorrSignKey, SchnorrVerKey)],
) -> StakeTable<BLSVerKey, SchnorrVerKey, F> {
    let mut st = StakeTable::<BLSVerKey, SchnorrVerKey, F>::new(capacity);
    // Registering keys
    bls_keys
        .iter()
        .enumerate()
        .zip(schnorr_keys)
        .for_each(|((i, bls_key), (_, schnorr_key))| {
            st.register(*bls_key, U256::from((i + 1) as u32), schnorr_key.clone())
                .unwrap();
        });
    // Freeze the stake table
    st.advance();
    st.advance();
    st
}
