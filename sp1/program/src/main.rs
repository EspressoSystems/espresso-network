#![cfg_attr(target_os = "zkvm", no_main)]
#[cfg(target_os = "zkvm")]
sp1_zkvm::entrypoint!(main);

#[cfg(target_os = "zkvm")]
mod atomics;

#[cfg(target_os = "zkvm")]
use alloy::primitives::U256;
#[cfg(target_os = "zkvm")]
use espresso_types::{AuthenticatedValidatorMap, MOCK_SEQUENCER_VERSIONS};
#[cfg(target_os = "zkvm")]
use sha2::{Digest, Sha256};

#[cfg(target_os = "zkvm")]
pub fn main() {
    let leaf_bytes = sp1_zkvm::io::read_vec();
    let stake_table_json = sp1_zkvm::io::read_vec();
    let threshold_bytes = sp1_zkvm::io::read_vec();

    let leaf_data = espresso_sp1_program::decode_leaf(&leaf_bytes).expect("leaf decodes");
    let validators: AuthenticatedValidatorMap =
        serde_json::from_slice(&stake_table_json).expect("stake table decodes");
    let stake_table = espresso_sp1_program::stake_table_from_validators(&validators);
    let success_threshold = U256::from_be_slice(&threshold_bytes);

    let (height, commit) = espresso_sp1_program::verify_leaf(
        &leaf_data,
        stake_table,
        success_threshold,
        MOCK_SEQUENCER_VERSIONS,
    )
    .expect("leaf verifies");

    // The stake table and threshold are unconstrained host inputs; committing
    // them binds the journal to the committee the QC was verified against.
    let stake_table_digest: [u8; 32] = Sha256::digest(&stake_table_json).into();

    sp1_zkvm::io::commit(&height);
    sp1_zkvm::io::commit_slice(commit.as_ref());
    sp1_zkvm::io::commit_slice(&stake_table_digest);
    sp1_zkvm::io::commit_slice(&success_threshold.to_be_bytes::<32>());
}

#[cfg(not(target_os = "zkvm"))]
fn main() {}
