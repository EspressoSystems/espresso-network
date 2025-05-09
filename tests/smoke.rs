use std::time::Instant;

use anyhow::Result;
use futures::StreamExt;

use crate::common::{NativeDemo, TestConfig, TestRequirements};

/// We allow for no change in state across this many consecutive iterations.
const MAX_STATE_NOT_INCREMENTING: u8 = 1;
/// We allow for no new transactions across this many consecutive iterations.
const MAX_TXNS_NOT_INCREMENTING: u8 = 5;

pub async fn assert_native_demo_works(requirements: TestRequirements) -> Result<()> {
    let start = Instant::now();
    dotenvy::dotenv()?;

    let testing = TestConfig::new(requirements.clone()).await.unwrap();

    println!("Waiting on readiness");
    let _ = testing.readiness().await?;

    let initial = testing.test_state().await;
    println!("Initial State: {}", initial);

    let mut sub = testing
        .espresso
        .subscribe_blocks(initial.block_height.unwrap())
        .await?;

    while (sub.next().await).is_some() {
        let new = testing.test_state().await;
        println!("New State:{}", new);

        if initial.builder_balance + initial.recipient_balance
            != new.builder_balance + new.recipient_balance
        {
            panic!("Balance not conserved");
        }

        // Timeout if tests take too long.
        if start.elapsed() > requirements.timeout {
            panic!("Timeout waiting for block height, transaction count, and light client updates to increase.");
        }

        // test that we progress EXPECTED_BLOCK_HEIGHT blocks from where we started
        if new.block_height.unwrap() < testing.expected_block_height() {
            println!(
                "waiting for block height have={} want={}",
                new.block_height.unwrap(),
                testing.expected_block_height()
            );
            continue;
        }

        if new.txn_count - initial.txn_count < testing.expected_txn_count() {
            println!(
                "waiting for transaction count have={} want={}",
                new.txn_count - initial.txn_count,
                testing.expected_txn_count()
            );
            continue;
        }
        break;
    }
    println!("Final State: {}", testing.test_state().await);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_base() -> Result<()> {
    let _child = NativeDemo::run(None, None);
    assert_native_demo_works(Default::default()).await
}
