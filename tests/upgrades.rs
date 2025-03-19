use anyhow::Result;
use client::SequencerClient;
use dotenvy::var;
use espresso_types::{EpochVersion, FeeVersion, MarketplaceVersion};
use futures::{future::join_all, StreamExt};
use vbs::version::{StaticVersionType, Version};

use crate::{
    common::{test_stake_table_update, NativeDemo, TestConfig},
    smoke::assert_native_demo_works,
};

const SEQUENCER_BLOCKS_TIMEOUT: u64 = 200;

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_upgrade() -> Result<()> {
    dotenvy::dotenv()?;
    let compose = var("INTEGRATION_TEST_PROCESS_COMPOSE").ok();

    let _demo = NativeDemo::run(compose)?;

    assert_native_demo_works().await?;

    let testing = TestConfig::new().await.unwrap();

    let (base, upgrade) = match testing.sequencer_version {
        3 => (FeeVersion::version(), EpochVersion::version()),
        version if version > 3 => (FeeVersion::version(), MarketplaceVersion::version()),
        _ => panic!("Invalid sequencer version provided for upgrade test."),
    };

    println!("Waiting on readiness");
    let _ = testing.readiness().await?;

    let initial = testing.test_state().await;
    println!("Initial State:{}", initial);

    let clients = testing.sequencer_clients;
    let client = clients[0].clone();
    let height = test_header_version(clients.clone(), base, upgrade).await?;
    // check that atleast 50 blocks are produced after the upgrade
    test_blocks_production(clients.clone(), height, 10).await?;

    if upgrade == EpochVersion::version() {
        test_stake_table_update(clients.clone()).await?;
    }

    let height = client.get_height().await?;
    // check that atleast 50 blocks are produced after the stake table updates
    test_blocks_production(clients.clone(), height, 10).await?;

    // TODO assert transactions are incrementing
    Ok(())
}

async fn test_header_version(
    clients: Vec<SequencerClient>,
    base: Version,
    upgrade: Version,
) -> Result<u64> {
    // Test is limited to those sequencers with correct modules
    // enabled. It would be less fragile if we could discover them.
    let subscriptions = join_all(clients.iter().map(|c| c.subscribe_headers(0)))
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

    let mut stream = futures::stream::iter(subscriptions).flatten_unordered(None);
    let mut height = 0;
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
            assert_eq!(header.version(), base)
        }

        if header.version() == upgrade {
            println!("header version matched! height={:?}", header.height());
            height = header.height();
            break;
        }

        if header.height() > SEQUENCER_BLOCKS_TIMEOUT {
            panic!("Exceeded maximum block height. Upgrade should have finished by now :(");
        }
    }

    Ok(height)
}

async fn test_blocks_production(clients: Vec<SequencerClient>, from: u64, num: u64) -> Result<()> {
    let subscriptions = join_all(clients.iter().map(|c| c.subscribe_blocks(from)))
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

    let mut num_blocks = 0;

    for mut node in subscriptions {
        while let Some(block) = node.next().await {
            let _block = block.unwrap();
            num_blocks += 1;
            if num_blocks == num {
                break;
            }
        }

        num_blocks = 0;
    }

    Ok(())
}
