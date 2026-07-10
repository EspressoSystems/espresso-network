#![cfg_attr(target_os = "zkvm", no_main)]
#[cfg(target_os = "zkvm")]
sp1_zkvm::entrypoint!(main);

#[cfg(target_os = "zkvm")]
use sha2::{Digest, Sha256};

#[cfg(target_os = "zkvm")]
pub fn main() {
    let leaf_bytes = sp1_zkvm::io::read_vec();
    let stake_table_bytes = sp1_zkvm::io::read_vec();

    let leaf_data = espresso_sp1_program::parse_leaf(&leaf_bytes).expect("leaf decodes");
    let peers =
        espresso_sp1_program::parse_stake_table(&stake_table_bytes).expect("stake table decodes");

    let verified = espresso_sp1_program::verify_leaf(&leaf_data, peers).expect("leaf verifies");

    // The stake table is an unconstrained host input; committing its digest
    // binds the journal to the committee the QC was verified against.
    let stake_table_digest: [u8; 32] = Sha256::digest(&stake_table_bytes).into();

    sp1_zkvm::io::commit(&verified.height);
    sp1_zkvm::io::commit_slice(verified.commitment.as_ref());
    sp1_zkvm::io::commit_slice(&stake_table_digest);
    sp1_zkvm::io::commit_slice(&verified.threshold.to_be_bytes::<32>());
    sp1_zkvm::io::commit(&verified.epoch);
}

#[cfg(not(target_os = "zkvm"))]
fn main() {}
