use std::time::{Duration, Instant};

use alloy::primitives::U256;
use anyhow::Result;
use espresso_types::SeqTypes;
use hotshot_types::{
    stake_table::StakeTableEntry,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    PeerConfig,
};
use sequencer::api::data_source::StakeTableWithEpochNumber;
use tagged_base64::TaggedBase64;
use url::Url;

use crate::{
    common::{load_genesis_file, NativeDemo, TestRequirements},
    smoke::assert_native_demo_works,
};

/// Checks if the native works if started on the PoS/Epoch version
#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_pos_base() -> Result<()> {
    let genesis_path = "data/genesis/demo-pos-base.toml";
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
    // Run for a least 3 epochs plus a few blocks to confirm we can make progress once
    // we are using the stake table from the contract.
    let expected_block_height = epoch_length * 3 + 10;

    let pos_progress_requirements = TestRequirements {
        block_height_increment: expected_block_height,
        txn_count_increment: 2 * expected_block_height,
        global_timeout: Duration::from_secs(expected_block_height as u64 * 3),
        ..Default::default()
    };
    assert_native_demo_works(pos_progress_requirements).await?;

    Ok(())
}

/// Checks if the native works if started on the DRB and Header version
#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_drb_header() -> Result<()> {
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
    // Run for a least 3 epochs plus a few blocks to confirm we can make progress once
    // we are using the stake table from the contract.
    let expected_block_height = epoch_length * 3 + 10;

    let progress_requirements = TestRequirements {
        block_height_increment: expected_block_height,
        txn_count_increment: 2 * expected_block_height,
        global_timeout: Duration::from_secs(expected_block_height as u64 * 3),
        ..Default::default()
    };
    assert_native_demo_works(progress_requirements).await?;

    Ok(())
}

/// Checks if dynamic DA committees work as expected
#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_da_committee() -> Result<()> {
    let genesis_path = "data/genesis/demo-da-committees.toml";
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

    // Step through the committees defined in demo-da-committees
    for committee in genesis
        .da_committees
        .expect("da_committees not set in genesis")
    {
        assert_da_stake_table(
            Default::default(),
            committee.start_epoch,
            &committee
                .committee
                .iter()
                .map(|member| member)
                .collect::<Vec<&PeerConfig<SeqTypes>>>(),
        )
        .await?;
    }

    let epoch_length = genesis
        .epoch_height
        .expect("epoch_height not set in genesis");
    // Run for a least 3 epochs plus a few blocks to confirm we can make progress once
    // we are using the stake table from the contract.
    let expected_block_height = epoch_length * 21 + 10; // Make sure we're past epoch 21

    let pos_progress_requirements = TestRequirements {
        block_height_increment: expected_block_height,
        txn_count_increment: 2 * expected_block_height,
        global_timeout: Duration::from_secs(expected_block_height as u64 * 3),
        ..Default::default()
    };
    assert_native_demo_works(pos_progress_requirements).await?;

    Ok(())
}

async fn assert_da_stake_table(
    requirements: TestRequirements,
    start_epoch: u64,
    entries: &[&PeerConfig<SeqTypes>],
) -> Result<()> {
    let start = Instant::now();
    let sequencer_api_port = dotenvy::var("ESPRESSO_SEQUENCER1_API_PORT")?;
    let sequencer_url: Url = format!("http://localhost:{sequencer_api_port}").parse()?;
    let da_stake_table_url = format!("{sequencer_url}v1/node/da-stake-table/current");
    println!("Fetching da stake table from: {}", da_stake_table_url);

    let mut step = 0;
    loop {
        // Timeout if tests take too long.
        if start.elapsed() > requirements.global_timeout * 30 {
            panic!(
                "Timeout waiting for block height, transaction count, and light client updates to \
                 increase."
            );
        }

        // Attempt to read da stake table, up to 5 times with 1 second delays
        let http_client = reqwest::Client::new();
        let mut attempt = 0;
        let da_stake_table = loop {
            attempt += 1;
            match http_client
                .get(&da_stake_table_url)
                .header("Accept", "application/json")
                .send()
                .await
                .and_then(|r| r.error_for_status())
            {
                Ok(response) => {
                    match response.json::<StakeTableWithEpochNumber<SeqTypes>>().await {
                        Ok(data) => {
                            break data;
                        },
                        Err(e) if attempt == 5 => {
                            panic!("Failed to parse da stake table response: {}", e);
                        },
                        Err(_) => {},
                    }
                },
                Err(e) if attempt == 5 => {
                    panic!("Request for da stake table failed: {}", e);
                },
                Err(_) => {},
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        };

        step += 1;
        if step > 30 {
            // Only show every 30 seconds
            step = 0;
            println!(
                "Retrieved DA stake table for epoch {:?}, {} members: {:?}",
                da_stake_table.epoch,
                da_stake_table.stake_table.len(),
                da_stake_table.stake_table
            );
        }

        let Some(response_epoch) = da_stake_table.epoch else {
            if step == 0 {
                println!("DA stake table epoch is None, waiting...");
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };

        if *response_epoch < start_epoch {
            if step == 0 {
                println!(
                    "DA stake table epoch {} less than start epoch {}, waiting...",
                    response_epoch, start_epoch
                );
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        let mut expected = entries
            .iter()
            .cloned()
            .cloned()
            .collect::<Vec<PeerConfig<SeqTypes>>>();
        expected.sort_by_key(|e| e.stake_table_entry.stake_key.clone());
        let mut actual = da_stake_table.stake_table.clone();
        actual.sort_by_key(|e| e.stake_table_entry.stake_key.clone());

        assert_eq!(
            expected, actual,
            "Expected DA stake table to match. Expected: {expected:?}, Actual: {actual:?}"
        );

        return Ok(());
    }
}
