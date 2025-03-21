use std::time::Instant;

use anyhow::Result;
use futures::StreamExt;

use crate::common::{NativeDemo, TestConfig};

/// We allow for no change in state across this many consecutive iterations.
const MAX_STATE_NOT_INCREMENTING: u8 = 1;
/// We allow for no new transactions across this many consecutive iterations.
const MAX_TXNS_NOT_INCREMENTING: u8 = 5;

pub async fn assert_native_demo_works() -> Result<()> {
    let start = Instant::now();
    dotenvy::dotenv()?;

    let testing = TestConfig::new().await.unwrap();

    println!("Waiting on readiness");
    let _ = testing.readiness().await?;

    let initial = testing.test_state().await;
    println!("Initial State: {}", initial);

    let mut sub = testing
        .espresso
        .subscribe_blocks(initial.block_height.unwrap())
        .await?;

    let mut last = initial.clone();
    let mut state_retries = 0;
    let mut txn_retries = 0;
    while (sub.next().await).is_some() {
        let new = testing.test_state().await;
        println!("New State:{}", new);

        if initial.builder_balance + initial.recipient_balance
            != new.builder_balance + new.recipient_balance
        {
            panic!("Balance not conserved");
        }

        // Timeout if tests take too long.
        if start.elapsed().as_secs() as f64 > testing.timeout {
            panic!("Timeout waiting for block height, transaction count, and light client updates to increase.");
        }

        // test that we progress EXPECTED_BLOCK_HEIGHT blocks from where we started
        if new.block_height.unwrap()
            >= testing.expected_block_height() + initial.block_height.unwrap()
        {
            println!("Reached {} block(s)!", testing.expected_block_height());
            if new.txn_count - initial.txn_count < 1 {
                panic!("Did not receive transactions");
            }
            break;
        }

        if new <= last {
            if state_retries > MAX_STATE_NOT_INCREMENTING {
                panic!("Chain state did not increment.");
            }
            state_retries += 1;
            println!("Chain state did not increment, trying again.");
        } else {
            // If state is incrementing reset the counter.
            state_retries = 0;
        }

        if new.txn_count <= last.txn_count {
            if txn_retries >= MAX_TXNS_NOT_INCREMENTING {
                panic!("No new transactions.");
            }
            txn_retries += 1;
            println!("Transactions did not increment, trying again.");
        } else {
            // If transactions are incrementing reset the counter.
            txn_retries = 0;
        }

        last = new;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_basic() -> Result<()> {
    let _child = NativeDemo::run(None);
    assert_native_demo_works().await
}
