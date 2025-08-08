use std::sync::Arc;

use async_broadcast::broadcast;
use hotshot_builder_shared::testing::{
    consensus::SimulatedChainState,
    constants::{TEST_NUM_NODES_IN_VID_COMPUTATION, TEST_PROTOCOL_MAX_BLOCK_SIZE},
};
use hotshot_example_types::{block_types::TestTransaction, state_types::TestInstanceState};
use tracing_test::traced_test;

use crate::{
    service::{BuilderConfig, GlobalState, ALLOW_EMPTY_BLOCK_PERIOD},
    testing::TestServiceWrapper,
};

// How many times consensus will re-try getting available blocks
const NUM_RETRIES: usize = 5;

/// [test_empty_block_rate] is a test to ensure that if we don't have any
/// transactions being submitted, that the builder will continue it's current
/// behavior of not proposing empty blocks.
///
/// |> Note: this test simulates how consensus interacts with the Builder in a
/// |> very basic way.  When consensus asks for available blocks, and the
/// |> Builder returns an error that indicates that it does not have any blocks
/// |> to propose, consensus will retry a few times before giving up. As a
/// |> result the number of times that consensus has to ask the Builder for
/// |> block is an integral part of this test.
#[tokio::test]
async fn test_empty_block_rate() {
    // Number of views to simulate
    const NUM_ROUNDS: u64 = 5;
    const {
        assert!(NUM_ROUNDS > ALLOW_EMPTY_BLOCK_PERIOD);
    }

    let global_state = GlobalState::new(
        BuilderConfig::test(),
        TestInstanceState::default(),
        TEST_PROTOCOL_MAX_BLOCK_SIZE,
        TEST_NUM_NODES_IN_VID_COMPUTATION,
    );

    let (event_stream_sender, event_stream) = broadcast(1024);
    let test_service =
        TestServiceWrapper::new(Arc::clone(&global_state), event_stream_sender.clone()).await;
    global_state.start_event_loop(event_stream);

    let mut chain_state = SimulatedChainState::new(event_stream_sender);

    // Simulate NUM_ROUNDS of consensus. First we submit the transactions for this round to the builder,
    // then construct DA and Quorum Proposals based on what we received from builder in the previous round
    // and request a new bundle.
    #[allow(clippy::needless_range_loop)] // intent is clearer this way
    for _ in 0..NUM_ROUNDS {
        let builder_state_id = chain_state.simulate_consensus_round(None).await;

        // get response
        for _ in 0..NUM_RETRIES {
            let available_blocks = test_service
                .get_available_blocks(&builder_state_id)
                .await
                .unwrap();
            assert!(
                available_blocks.is_empty(),
                "Builder shouldn't be building empty block without recent blocks with transactions"
            );
        }
    }
}

/// [test_eager_block_rate] is a test that ensures that the builder will propose
/// empty blocks, if consensus indicates a proposal included transactions.
///
/// It checks initially that it does not propose any empty blocks in round 0.
/// It checks that it proposes a block with transactions in round 1, which
/// gets included by consensus.
/// It then checks that the next `allow_empty_block_period` rounds return empty
/// blocks without the need to retry.
/// It then checks that the remaining rounds will not propose any empty blocks.
///
/// |> Note: this test simulates how consensus interacts with the Builder in a
/// |> very basic way.  When consensus asks for available blocks, and the
/// |> Builder returns an error that indicates that it does not have any blocks
/// |> to propose, consensus will retry a few times before giving up. As a
/// |> result the number of times that consensus has to ask the Builder for
/// |> block is an integral part of this test.
#[tokio::test]
#[traced_test]
async fn test_eager_block_rate() {
    let global_state = GlobalState::new(
        BuilderConfig::test(),
        TestInstanceState::default(),
        TEST_PROTOCOL_MAX_BLOCK_SIZE,
        TEST_NUM_NODES_IN_VID_COMPUTATION,
    );

    let (event_stream_sender, event_stream) = broadcast(1024);
    let test_service =
        TestServiceWrapper::new(Arc::clone(&global_state), event_stream_sender.clone()).await;
    global_state.start_event_loop(event_stream);

    let mut chain_state = SimulatedChainState::new(event_stream_sender);

    // First round, shouldn't be building empty blocks
    {
        let builder_state_id = chain_state.simulate_consensus_round(None).await;

        // get response
        for _ in 0..NUM_RETRIES {
            let available_blocks = test_service
                .get_available_blocks(&builder_state_id)
                .await
                .unwrap();
            assert!(
                available_blocks.is_empty(),
                "Builder shouldn't be building empty block without recent blocks with transactions"
            );
        }
    }

    // Second round, simulate a transaction has been proposed. Now builder should be proposing empty blocks.
    {
        let builder_state_id = chain_state
            .simulate_consensus_round(Some(vec![TestTransaction::default()]))
            .await;

        // get response
        for _ in 0..NUM_RETRIES {
            let available_blocks = test_service
                .get_available_blocks(&builder_state_id)
                .await
                .unwrap();
            assert_eq!(
                available_blocks.first().unwrap().block_size,
                0,
                "Builder should be building empty blocks: we've had a transaction included this \
                 round"
            );
        }
    }

    // Second round, simulate a transaction has been proposed. Now builder should be proposing empty blocks.
    for _ in 0..ALLOW_EMPTY_BLOCK_PERIOD {
        let builder_state_id = chain_state
            .simulate_consensus_round(Some(vec![TestTransaction::default()]))
            .await;

        // get response
        for _ in 0..NUM_RETRIES {
            let available_blocks = test_service
                .get_available_blocks(&builder_state_id)
                .await
                .unwrap();
            assert_eq!(
                available_blocks.first().unwrap().block_size,
                0,
                "Builder should be building empty blocks: we've had a transaction included \
                 recently"
            );
        }
    }

    {
        let builder_state_id = chain_state.simulate_consensus_round(None).await;

        // get response
        for _ in 0..NUM_RETRIES {
            let available_blocks = test_service
                .get_available_blocks(&builder_state_id)
                .await
                .unwrap();
            assert!(
                available_blocks.is_empty(),
                "Builder shouldn't be building empty block without recent blocks with transactions"
            );
        }
    }
}
