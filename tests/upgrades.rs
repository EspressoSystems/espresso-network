use std::time::Duration;

use anyhow::Result;
use espresso_types::{EpochVersion, FeeVersion};
use futures::{future::join_all, StreamExt};
use vbs::version::StaticVersionType;

use crate::{
    common::{NativeDemo, TestConfig, TestRequirements},
    smoke::assert_native_demo_works,
};

// The number of blocks we will wait for the upgrade to complete before we panic. Needs to be large
// than "start_proposing_view" set in the "demo-pos.toml" genesis file.
//
// With the current config
//
// epoch_height = 200
// epoch_start_block = 321
// start_proposing_view = 200
//
// the upgrade happens usually at height 316.
const POS_UPGRADE_WAIT_UNTIL_HEIGHT: u64 = 350;
const MIN_BLOCK_INCREMENT_WITH_POS: u64 = 400; // 2 epochs

async fn assert_pos_upgrade_happens() -> Result<()> {
    dotenvy::dotenv()?;

    // TODO we don't really want to use the requirements here
    let testing = TestConfig::new(Default::default()).await.unwrap();
    println!("Testing upgrade {:?}", testing);

    let base_version = FeeVersion::version();
    let upgrade_version = EpochVersion::version();

    println!("Waiting on readiness");
    let _ = testing.readiness().await?;

    let initial = testing.test_state().await;
    println!("Initial State:{}", initial);

    let clients = testing.sequencer_clients;

    // Test is limited to those sequencers with correct modules
    // enabled. It would be less fragile if we could discover them.
    let subscriptions = join_all(clients.iter().map(|c| c.subscribe_headers(0)))
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

    let mut stream = futures::stream::iter(subscriptions).flatten_unordered(None);

    while let Some(header) = stream.next().await {
        let header = header.unwrap();
        println!(
            "block: height={}, version={}",
            header.height(),
            header.version()
        );

        // TODO is it possible to discover the view at which upgrade should be finished?
        // First few views should be `Base` version.
        if header.height() <= 20 {
            assert_eq!(header.version(), base_version);
        }

        if header.version() == upgrade_version {
            println!("header version matched! height={:?}", header.height());
            break;
        }

        if header.height() > POS_UPGRADE_WAIT_UNTIL_HEIGHT {
            panic!("Exceeded maximum block height. Upgrade should have finished by now :(");
        }
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_pos_upgrade() -> Result<()> {
    let _demo = NativeDemo::run(
        None,
        Some(vec![(
            "ESPRESSO_SEQUENCER_PROCESS_COMPOSE_GENESIS_FILE".to_string(),
            "data/genesis/demo-pos.toml".to_string(),
        )]),
    )?;

    assert_native_demo_works(Default::default()).await?;
    assert_pos_upgrade_happens().await?;

    // verify native demo continues to work after upgrade
    let requirements = TestRequirements {
        block_height_increment: MIN_BLOCK_INCREMENT_WITH_POS,
        txn_count_increment: 100, // a bit arbitrary but we wait for many blocks
        timeout: Duration::from_secs(MIN_BLOCK_INCREMENT_WITH_POS * 2),
    };
    assert_native_demo_works(requirements).await?;

    Ok(())
}
