use alloy::primitives::U256;
use ark_ed_on_bn254::EdwardsConfig;
use ark_std::rand::{CryptoRng, RngCore};
use espresso_types::SeqTypes;
use hotshot_types::{
    stake_table::{HSStakeTable, StakeTableEntry},
    PeerConfig,
};
use jf_signature::{
    bls_over_bn254::{BLSOverBN254CurveSignatureScheme, VerKey as BLSVerKey},
    schnorr::SchnorrSignatureScheme,
    SignatureScheme,
};

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
