use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use async_broadcast::broadcast;
use committable::Committable;
use hotshot_builder_api::v0_1::builder::TransactionStatus;
use hotshot_example_types::{block_types::TestTransaction, state_types::TestInstanceState};
use hotshot_types::{
    data::{VidCommitment, ViewNumber},
    traits::node_implementation::ConsensusTime,
};
use marketplace_builder_shared::{
    block::{BlockId, BuilderStateId},
    testing::{consensus::SimulatedChainState, constants::TEST_NUM_NODES_IN_VID_COMPUTATION},
};
use tracing_test::traced_test;

use crate::{
    block_size_limits::BlockSizeLimits,
    service::{BuilderConfig, GlobalState},
    testing::TestServiceWrapper,
};

/// This tests simulates size limits being decreased lower than our capacity
/// and then checks that size limits return to protocol maximum over time
#[tokio::test]
#[traced_test]
async fn block_size_increment() {
    // Number of views we'll need to simulate to reach protocol max block size
    // Basically compound interest formula solved for time
    let num_rounds: u64 =
        (((PROTOCOL_MAX_BLOCK_SIZE / BlockSizeLimits::MAX_BLOCK_SIZE_FLOOR) as f64).ln()
            / (1.0 + 1f64 / BlockSizeLimits::MAX_BLOCK_SIZE_CHANGE_DIVISOR as f64).ln())
        .ceil() as u64;

    // Max block size for this test. Relatively low
    // so that we don't spend a lot of rounds in this test
    // in this test
    const PROTOCOL_MAX_BLOCK_SIZE: u64 = BlockSizeLimits::MAX_BLOCK_SIZE_FLOOR * 3;

    let mut cfg = BuilderConfig::test();
    // We don't want to delay increments for this test
    cfg.max_block_size_increment_period = Duration::ZERO;
    let global_state = GlobalState::new(
        cfg,
        TestInstanceState::default(),
        PROTOCOL_MAX_BLOCK_SIZE,
        TEST_NUM_NODES_IN_VID_COMPUTATION,
    );

    // Manually set the limits
    global_state.block_size_limits.mutable_state.store(
        crate::block_size_limits::MutableState {
            max_block_size: BlockSizeLimits::MAX_BLOCK_SIZE_FLOOR,
            last_block_size_increment: coarsetime::Instant::now().as_ticks(),
        },
        Ordering::Relaxed,
    );

    let (event_stream_sender, event_stream) = broadcast(1024);
    let test_service =
        TestServiceWrapper::new(Arc::clone(&global_state), event_stream_sender.clone()).await;
    Arc::clone(&global_state).start_event_loop(event_stream);

    // set up state to track between simulated consensus rounds
    let mut chain_state = SimulatedChainState::new(event_stream_sender.clone());

    // Simulate NUM_ROUNDS of consensus. First we submit the transactions for this round to the builder,
    // then construct DA and Quorum Proposals based on what we received from builder in the previous round
    // and request a new bundle.
    #[allow(clippy::needless_range_loop)] // intent is clearer this way
    for round in 0..num_rounds {
        // We should still be climbing
        assert_ne!(
            global_state
                .block_size_limits
                .mutable_state
                .load(Ordering::Relaxed)
                .max_block_size,
            PROTOCOL_MAX_BLOCK_SIZE,
            "On round {round}/{num_rounds} we shouldn't be back to PROTOCOL_MAX_BLOCK_SIZE yet"
        );

        // simulate transaction being submitted to the builder
        test_service
            .submit_transactions(vec![TestTransaction::default()])
            .await;

        // get transactions submitted in previous rounds, [] for genesis
        // and simulate the block built from those
        let builder_state_id = chain_state.simulate_consensus_round(None).await;

        // Get response. Called through
        let mut available_states = test_service
            .get_available_blocks(&builder_state_id)
            .await
            .unwrap();

        if let Some(block_info) = available_states.pop() {
            let block_id = BlockId {
                hash: block_info.block_hash,
                view: builder_state_id.parent_view,
            };
            // Get header input, this should trigger block size limits increment
            test_service
                .claim_block_header_input(&block_id)
                .await
                .expect("Failed to claim header input");
        }
    }

    // We should've returned to protocol max block size
    assert_eq!(
        global_state
            .block_size_limits
            .mutable_state
            .load(Ordering::Relaxed)
            .max_block_size,
        PROTOCOL_MAX_BLOCK_SIZE
    )
}

#[tokio::test]
#[traced_test]
async fn huge_transactions() {
    // Max block size for this test. As low as possible
    // so that we don't spend a lot of time in this test
    const PROTOCOL_MAX_BLOCK_SIZE: u64 = BlockSizeLimits::MAX_BLOCK_SIZE_FLOOR;
    // Number of big transactions to send to the builder
    const N_BIG_TRANSACTIONS: usize = 10;

    let global_state = GlobalState::new(
        BuilderConfig::test(),
        TestInstanceState::default(),
        PROTOCOL_MAX_BLOCK_SIZE,
        TEST_NUM_NODES_IN_VID_COMPUTATION,
    );
    let (event_stream_sender, event_stream) = broadcast(1024);
    let test_service =
        TestServiceWrapper::new(Arc::clone(&global_state), event_stream_sender).await;
    Arc::clone(&global_state).start_event_loop(event_stream);

    let almost_too_big = TestTransaction::new(vec![0u8; PROTOCOL_MAX_BLOCK_SIZE as usize]);
    let too_big = TestTransaction::new(vec![0u8; PROTOCOL_MAX_BLOCK_SIZE as usize + 1]);
    let too_big_commitment = too_big.commit();

    test_service
        .submit_transactions_private(vec![almost_too_big.clone(); N_BIG_TRANSACTIONS])
        .await
        .unwrap();

    test_service
        .submit_transactions_private(vec![too_big])
        .await
        .unwrap_err();

    // Should also update the tx status
    assert!(matches!(
        test_service
            .proxy_global_state
            .coordinator
            .tx_status(&too_big_commitment),
        TransactionStatus::Rejected { .. }
    ));

    // Builder shouldn't exceed the maximum block size, so transactions
    // should be included one-by-one
    assert_eq!(
        vec![almost_too_big],
        test_service
            .get_transactions(&BuilderStateId {
                parent_view: ViewNumber::genesis(),
                parent_commitment: VidCommitment::default(),
            })
            .await
    )
}
