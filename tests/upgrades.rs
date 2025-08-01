use std::time::Duration;

use anyhow::Result;
use espresso_types::{DrbAndHeaderUpgradeVersion, EpochVersion, FeeVersion, UpgradeMode};
use futures::{future::join_all, StreamExt};
use sequencer::Genesis;
use vbs::version::StaticVersionType;

use crate::{
    common::{load_genesis_file, NativeDemo, TestConfig, TestRequirements},
    smoke::assert_native_demo_works,
};

async fn assert_upgrade_happens<Base, Target>(genesis: &Genesis) -> Result<()>
where
    Base: StaticVersionType,
    Target: StaticVersionType,
{
    dotenvy::dotenv()?;

    // The requirements passed to `TestConfig` are ignored here.
    let testing = TestConfig::new(Default::default()).await.unwrap();
    println!("Testing upgrade {testing:?}");

    let base_version = Base::version();
    let upgrade_version = Target::version();

    println!("Waiting on readiness");
    let _ = testing.readiness().await?;

    let initial = testing.test_state().await;
    println!("Initial State:{initial}");

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

        let wait_until_view = match genesis.upgrades[&upgrade_version].mode.clone() {
            // It usually takes about 120 blocks to get to the upgrade, wait a bit longer.
            UpgradeMode::View(upgrade) => upgrade.start_proposing_view + 200,
            UpgradeMode::Time(_time_based_upgrade) => {
                unimplemented!("Time based upgrade not supported yet")
            },
        };

        if header.height() > wait_until_view {
            panic!("Waited until {wait_until_view} but upgrade did not happen");
        }
    }

    Ok(())
}

async fn run_upgrade_test<Base, Target>(genesis_path: &str) -> Result<()>
where
    Base: StaticVersionType,
    Target: StaticVersionType,
{
    let genesis = load_genesis_file(genesis_path)?;
    let _demo = NativeDemo::run(
        None,
        Some(vec![(
            "ESPRESSO_SEQUENCER_PROCESS_COMPOSE_GENESIS_FILE".to_string(),
            genesis_path.to_string(),
        )]),
    )?;

    assert_native_demo_works(Default::default()).await?;
    assert_upgrade_happens::<Base, Target>(&genesis).await?;

    let epoch_length = genesis.epoch_height.expect("epoch_height set in genesis");
    // Run for a least 3 epochs plus a few blocks to confirm we can make progress once
    // we are using the stake table from the contract.
    let expected_block_height = epoch_length * 3 + 10;

    // verify native demo continues to work after upgrade
    let progress_requirements = TestRequirements {
        block_height_increment: expected_block_height,
        txn_count_increment: 2 * expected_block_height,
        global_timeout: Duration::from_secs(expected_block_height as u64 * 3),
        ..Default::default()
    };
    assert_native_demo_works(progress_requirements).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_pos_upgrade() -> Result<()> {
    run_upgrade_test::<FeeVersion, EpochVersion>("data/genesis/demo-pos.toml").await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_drb_header_upgrade() -> Result<()> {
    run_upgrade_test::<EpochVersion, DrbAndHeaderUpgradeVersion>(
        "data/genesis/demo-drb-header-upgrade.toml",
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_fee_to_drb_header_upgrade() -> Result<()> {
    run_upgrade_test::<FeeVersion, DrbAndHeaderUpgradeVersion>(
        "data/genesis/demo-fee-to-drb-header-upgrade.toml",
    )
    .await
}
