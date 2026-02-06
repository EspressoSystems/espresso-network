use std::time::Duration;

use anyhow::Result;

use crate::{
    common::{load_genesis_file, NativeDemo, TestRequirements},
    smoke::assert_native_demo_works,
};

/// Checks if the native works if started on the DRB and Header version
#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_drb_header_base() -> Result<()> {
    let genesis_path = "data/genesis/demo-drb-header.toml";
    let genesis = load_genesis_file(genesis_path)?;

    let _child = NativeDemo::run(
        None,
        Some(vec![(
            "ESPRESSO_SEQUENCER_GENESIS_FILE".to_string(),
            // process compose runs from the root of the repo
            genesis_path.to_string(),
        )]),
    );

    // Sanity check that the demo is working
    assert_native_demo_works(Default::default()).await?;

    let epoch_length = genesis.epoch_height.expect("epoch_height set in genesis");

    // Version 0.4 supports rewards - currently don't have a good way to know how long we expect it
    // to take until the prover has finalized the state on L1. These limits are somewhat arbitrary.
    //
    // The prover needs to at least have enough time to send a proof for a block in epoch 3 because
    // no rewards are distributed until the end of epoch 2.
    //
    // TODO: monitor the prover making progress on L1 and adjust the test accordingly.
    let reward_claim_deadline_block_height = (epoch_length * 3 + 10).max(500);

    // Run for a least 3 epochs plus a few blocks to confirm we can make progress once
    // we are using the stake table from the contract.
    // Ensure we run long enough to check rewards
    let expected_block_height = (epoch_length * 3 + 10).max(reward_claim_deadline_block_height);

    let progress_requirements = TestRequirements {
        block_height_increment: expected_block_height,
        txn_count_increment: 2 * expected_block_height,
        global_timeout: Duration::from_secs(expected_block_height as u64 * 3),
        reward_claim_deadline_block_height: Some(reward_claim_deadline_block_height),
        ..Default::default()
    };

    assert_native_demo_works(progress_requirements).await?;

    Ok(())
}
