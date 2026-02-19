use std::time::Instant;

use alloy::primitives::U256;
use anyhow::Result;
use futures::StreamExt;

use crate::common::{NativeDemo, TestRequirements, TestRuntime, TestState};

pub async fn assert_native_demo_works(requirements: TestRequirements) -> Result<()> {
    dotenvy::dotenv()?;

    println!("{:#?}", requirements);

    let mut runtime = TestRuntime::from_requirements(requirements.clone()).await?;
    let start = Instant::now();

    let initial = runtime.test_state().await;
    println!("Initial State: {initial}");

    let client = client::SequencerClient::new(runtime.config.sequencer_api_url.clone());
    let mut sub = client
        .subscribe_blocks(initial.block_height.unwrap())
        .await?;

    let old = initial.clone();
    let mut blocks_without_tx = 0;
    let mut iteration = 0u64;
    let mut prev_printed = None;

    loop {
        match tokio::time::timeout(requirements.block_timeout, sub.next()).await {
            Ok(Some(_header)) => {},
            Ok(None) => panic!("Unexpected end of block stream"),
            Err(_) => panic!("No new blocks after {:?}", requirements.block_timeout),
        };

        let new = runtime.test_state().await;
        iteration += 1;

        let significant_change = prev_printed.as_ref().is_none_or(|prev: &TestState| {
            new.light_client_finalized_block_height != prev.light_client_finalized_block_height
                || new.rewards_claimed != prev.rewards_claimed
        });
        let power_of_two = iteration.is_power_of_two();

        if significant_change || power_of_two {
            println!("State (block {}):{new}", new.block_height.unwrap());
            prev_printed = Some(new.clone());
        }

        let num_new_tx = new.txn_count - old.txn_count;
        if num_new_tx == 0 {
            blocks_without_tx += new.block_height.unwrap() - old.block_height.unwrap();
            if blocks_without_tx > requirements.block_height_increment {
                panic!(
                    "Found {} blocks without txns",
                    requirements.max_consecutive_blocks_without_tx
                );
            }
        } else {
            blocks_without_tx = 0;
        }

        if initial.builder_balance + initial.recipient_balance
            != new.builder_balance + new.recipient_balance
        {
            panic!("Balance not conserved");
        }

        if start.elapsed() > requirements.global_timeout {
            panic!(
                "Timeout waiting for block height, transaction count, and light client updates to \
                 increase."
            );
        }

        if new.block_height.unwrap() < runtime.expected_block_height() {
            continue;
        }

        if new.txn_count < runtime.expected_txn_count() {
            continue;
        }

        if let Some(deadline_height) = requirements.reward_claim_deadline_block_height {
            let new = runtime.test_state().await;
            if new.block_height.unwrap() >= deadline_height {
                assert!(new.rewards_claimed > U256::ZERO);
                println!(
                    "Rewards claimed: {} at block {}",
                    new.rewards_claimed,
                    new.block_height.unwrap()
                );
            }
        }

        break;
    }
    println!("Final State: {}", runtime.test_state().await);
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_native_demo_base() -> Result<()> {
    let _child = NativeDemo::run(None, None);
    assert_native_demo_works(Default::default()).await
}
