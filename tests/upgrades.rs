use std::time::Duration;

use anyhow::Result;
use espresso_types::UpgradeMode;
use futures::{future::join_all, StreamExt};
use hotshot_types::utils::epoch_from_block_number;
use sequencer::Genesis;
use versions::{Upgrade, DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_VERSION, FEE_VERSION};

use crate::{
    common::{load_genesis_file, NativeDemo, TestRequirements, TestRuntime},
    smoke::assert_native_demo_works,
};

async fn assert_upgrade_happens(genesis: &Genesis, upgrade: Upgrade) -> Result<()> {
    dotenvy::dotenv()?;

    let mut runtime = TestRuntime::from_requirements(Default::default())
        .await
        .unwrap();
    println!("Testing upgrade {runtime:?}");

    let initial = runtime.test_state().await;
    println!("Initial State:{initial}");

    let clients = runtime.config.sequencer_clients.clone();

    // Test is limited to those sequencers with correct modules
    // enabled. It would be less fragile if we could discover them.
    let subscriptions = join_all(clients.iter().map(|c| c.subscribe_headers(0)))
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;

    let mut stream = futures::stream::iter(subscriptions).flatten_unordered(None);
    let mut iteration = 0u64;

    while let Some(header) = stream.next().await {
        let header = header.unwrap();
        iteration += 1;

        if iteration.is_power_of_two() {
            println!(
                "block: height={}, version={}",
                header.height(),
                header.version()
            );
        }

        // TODO is it possible to discover the view at which upgrade should be finished?
        // First few views should be `Base` version.
        if header.height() <= 20 {
            assert_eq!(header.version(), upgrade.base);
        }

        if header.version() == upgrade.target {
            println!("header version matched! height={:?}", header.height());
            break;
        }

        let wait_until_view = match genesis.upgrades[&upgrade.target].mode.clone() {
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

async fn run_upgrade_test(genesis_path: &str, upgrade: Upgrade) -> Result<()> {
    let genesis = load_genesis_file(genesis_path)?;
    let _demo = NativeDemo::run(
        None,
        Some(vec![(
            "ESPRESSO_SEQUENCER_GENESIS_FILE".to_string(),
            genesis_path.to_string(),
        )]),
    )?;

    assert_native_demo_works(Default::default()).await?;
    assert_upgrade_happens(&genesis, upgrade).await?;

    let epoch_length = genesis.epoch_height.expect("epoch_height set in genesis");
    let epoch_start_block = genesis.epoch_start_block.unwrap_or(1);

    let first_epoch = epoch_from_block_number(epoch_start_block, epoch_length);
    let first_reward_block = (first_epoch + 1) * epoch_length + 1;

    let first_reward_block = if upgrade.target >= DRB_AND_HEADER_UPGRADE_VERSION {
        Some(first_reward_block)
    } else {
        None
    };

    let expected_block_height = (first_epoch + 2) * epoch_length + 10;

    println!("Upgrade test config:");
    println!("  epoch_start_block: {epoch_start_block}");
    println!("  epoch_length: {epoch_length}");
    println!("  first_epoch: {first_epoch}");
    println!("  first_reward_block: {first_reward_block:?}");
    println!("  expected_block_height: {expected_block_height}");

    let progress_requirements = TestRequirements {
        block_height_increment: expected_block_height,
        txn_count_increment: 2 * expected_block_height,
        global_timeout: Duration::from_secs(expected_block_height as u64 * 6),
        first_reward_block,
        ..Default::default()
    };

    assert_native_demo_works(progress_requirements).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_pos_upgrade() -> Result<()> {
    run_upgrade_test(
        "data/genesis/demo-pos.toml",
        Upgrade::new(FEE_VERSION, EPOCH_VERSION),
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_drb_header_upgrade() -> Result<()> {
    run_upgrade_test(
        "data/genesis/demo-drb-header-upgrade.toml",
        Upgrade::new(EPOCH_VERSION, DRB_AND_HEADER_UPGRADE_VERSION),
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_fee_to_drb_header_upgrade() -> Result<()> {
    run_upgrade_test(
        "data/genesis/demo-fee-to-drb-header-upgrade.toml",
        Upgrade::new(FEE_VERSION, DRB_AND_HEADER_UPGRADE_VERSION),
    )
    .await
}
