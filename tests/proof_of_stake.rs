use std::time::{Duration, Instant};

use anyhow::Result;
use espresso_types::SeqTypes;
use sequencer::api::data_source::StakeTableWithEpochNumber;
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

/// Checks if the native works if started on the PoS/Epoch version
#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_da_committee() -> Result<()> {
    /*     use hotshot_types::traits::signature_key::SignatureKey;
    for i in 0..5 {
        let key1 = hotshot_types::signature_key::BLSPubKey::generated_from_seed_indexed([0; 32], i).1.to_tagged_base64().unwrap();
        let kp = hotshot_types::light_client::StateKeyPair::generate_from_seed_indexed([0; 32], i).sign_key_ref().to_tagged_base64().unwrap();
        println!("Iteration {i}");
        println!(" - BLS: {key1}");
        println!(" - Schnorr: {kp}");
    }
    assert!(false);*/

    let genesis_path = "data/genesis/demo-da-committees.toml";
    let genesis = load_genesis_file(genesis_path)?;

    let _child = NativeDemo::run(
        None,
        Some(vec![
            (
                "ESPRESSO_SEQUENCER_GENESIS_FILE".to_string(),
                // process compose runs from the root of the repo
                genesis_path.to_string(),
            ),
            // These keys are generated with generated_from_seed_indexed and are the private side to the keys in demo-da-committees.toml
            // TODO: Grab the default keys from .env and make public keys to put in demo-da-committees instead
            (
                "ESPRESSO_DEMO_SEQUENCER_STAKING_PRIVATE_KEY_0".to_string(),
                "BLS_SIGNING_KEY~lNDh4Pn-pTAyzyprOAFdXHwhrKhEwqwtMtkD3CZF4x3o".to_string(),
            ),
            (
                "ESPRESSO_DEMO_SEQUENCER_STATE_PRIVATE_KEY_0".to_string(),
                "SCHNORR_SIGNING_KEY~HpvL0GKuLCeVkbpyRWh8XGhpSgDAel5Ehq181Qp2nAFD".to_string(),
            ),
            (
                "ESPRESSO_DEMO_SEQUENCER_STAKING_PRIVATE_KEY_1".to_string(),
                "BLS_SIGNING_KEY~-DO72m_SFl6NQMYknm05FYpPEklkeqz-B3g2mFdbuS83".to_string(),
            ),
            (
                "ESPRESSO_DEMO_SEQUENCER_STATE_PRIVATE_KEY_1".to_string(),
                "SCHNORR_SIGNING_KEY~45YyRVukvS11jD742ESpdofgvNram9qXEcEbWJMZnAII".to_string(),
            ),
            (
                "ESPRESSO_DEMO_SEQUENCER_STAKING_PRIVATE_KEY_2".to_string(),
                "BLS_SIGNING_KEY~LY0x6w5BheYvEI3ro3g39NU-qwoYQRKc4ObCqc1yoC4S".to_string(),
            ),
            (
                "ESPRESSO_DEMO_SEQUENCER_STATE_PRIVATE_KEY_2".to_string(),
                "SCHNORR_SIGNING_KEY~MsqAFOzgc5RUvoB3sVRLKJmcgCST-x_fThnhiU0tTwEN".to_string(),
            ),
            (
                "ESPRESSO_DEMO_SEQUENCER_STAKING_PRIVATE_KEY_3".to_string(),
                "BLS_SIGNING_KEY~w4jERAaQfBdCdmlStEgj8PfIJJOWmCvbsL2wckpTfCbo".to_string(),
            ),
            (
                "ESPRESSO_DEMO_SEQUENCER_STATE_PRIVATE_KEY_3".to_string(),
                "SCHNORR_SIGNING_KEY~_vCBzmTgY32OZIkteql1y2knVqI7Jx68GvU_2117ggB4".to_string(),
            ),
            (
                "ESPRESSO_DEMO_SEQUENCER_STAKING_PRIVATE_KEY_4".to_string(),
                "BLS_SIGNING_KEY~FTAq-zib6oUVGSOdIlgntYB1IelS0vK6icYW8Z8OUySv".to_string(),
            ),
            (
                "ESPRESSO_DEMO_SEQUENCER_STATE_PRIVATE_KEY_4".to_string(),
                "SCHNORR_SIGNING_KEY~O-7qlIsA5O9lD5tdwAqkit-AksJQ_hBAXAni_GCqTgVt".to_string(),
            ),
        ]),
    );

    // Sanity check that the demo is working

    assert_native_demo_works(Default::default()).await?;
    assert_da_stake_table(Default::default()).await?;

    let epoch_length = genesis.epoch_height.expect("epoch_height set in genesis");
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

async fn assert_da_stake_table(requirements: TestRequirements) -> Result<()> {
    let start = Instant::now();
    let sequencer_api_port = dotenvy::var("ESPRESSO_SEQUENCER1_API_PORT")?;
    let sequencer_url: Url = format!("http://localhost:{sequencer_api_port}").parse()?;
    let da_stake_table_url = format!("{sequencer_url}v1/node/stake-table/current");
    println!("Fetching da stake table from: {}", da_stake_table_url);

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
        println!(
            "Retrieved DA stake table for epoch {:?}: {:?}",
            da_stake_table.epoch, da_stake_table.stake_table
        );

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
