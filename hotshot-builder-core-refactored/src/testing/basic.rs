use std::{sync::Arc, time::Duration};

use async_broadcast::broadcast;
use hotshot::types::{EventType, SignatureKey};
use hotshot_builder_api::v0_1::data_source::BuilderDataSource;
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
    node_types::{TestTypes, TestVersions},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_types::{
    data::{Leaf2, QuorumProposal2, QuorumProposalWrapper, VidCommitment, ViewNumber},
    event::LeafInfo,
    simple_certificate::QuorumCertificate2,
    traits::{
        block_contents::BlockHeader,
        node_implementation::{ConsensusTime, NodeType},
    },
    utils::BuilderCommitment,
};
use marketplace_builder_shared::{
    error::Error,
    testing::{
        consensus::SimulatedChainState,
        constants::{TEST_NUM_NODES_IN_VID_COMPUTATION, TEST_PROTOCOL_MAX_BLOCK_SIZE},
    },
};
use tokio::time::sleep;
use tracing_test::traced_test;

use crate::{
    service::{BuilderConfig, GlobalState, ProxyGlobalState},
    testing::{assert_eq_generic_err, sign, TestServiceWrapper, MOCK_LEADER_KEYS},
};

/// This test simulates consensus performing as expected and builder processing a number
/// of transactions
#[tokio::test]
#[traced_test]
async fn test_builder() {
    // Number of views to simulate
    const NUM_ROUNDS: usize = 5;
    // Number of transactions to submit per round
    const NUM_TXNS_PER_ROUND: usize = 4;

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

    // Transactions to send
    let all_transactions = (0..NUM_ROUNDS)
        .map(|round| {
            (0..NUM_TXNS_PER_ROUND)
                .map(|tx_num| TestTransaction::new(vec![round as u8, tx_num as u8]))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // set up state to track between simulated consensus rounds
    let mut prev_proposed_transactions: Option<Vec<TestTransaction>> = None;
    let mut transaction_history = Vec::new();

    let mut chain_state = SimulatedChainState::new(event_stream_sender.clone());

    // Simulate NUM_ROUNDS of consensus. First we submit the transactions for this round to the builder,
    // then construct DA and Quorum Proposals based on what we received from builder in the previous round
    // and request a new bundle.
    #[allow(clippy::needless_range_loop)] // intent is clearer this way
    for round in 0..NUM_ROUNDS {
        // simulate transaction being submitted to the builder
        test_service
            .submit_transactions(all_transactions[round].clone())
            .await;

        // get transactions submitted in previous rounds, [] for genesis
        // and simulate the block built from those
        let builder_state_id = chain_state
            .simulate_consensus_round(prev_proposed_transactions)
            .await;

        // get response
        let transactions = test_service.get_transactions(&builder_state_id).await;

        // in the next round we will use received transactions to simulate
        // the block being proposed
        prev_proposed_transactions = Some(transactions.clone());
        // save transactions to history
        transaction_history.extend(transactions);
    }

    // we should've served all transactions submitted, and in correct order
    assert_eq!(
        transaction_history,
        all_transactions.into_iter().flatten().collect::<Vec<_>>()
    );
}

// This test checks that builder prunes saved blocks on decide
#[tokio::test]
#[traced_test]
async fn test_pruning() {
    // Number of views to simulate
    const NUM_ROUNDS: usize = 10;
    // View number of decide event
    const DECIDE_VIEW: u64 = 5;
    // Number of transactions to submit per round
    const NUM_TXNS_PER_ROUND: usize = 4;

    let global_state = GlobalState::new(
        BuilderConfig::test(),
        TestInstanceState::default(),
        TEST_PROTOCOL_MAX_BLOCK_SIZE,
        TEST_NUM_NODES_IN_VID_COMPUTATION,
    );

    let (event_stream_sender, event_stream) = broadcast(1024);
    let test_service =
        TestServiceWrapper::new(Arc::clone(&global_state), event_stream_sender.clone()).await;
    Arc::clone(&global_state).start_event_loop(event_stream);

    // Transactions to send
    let all_transactions = (0..NUM_ROUNDS)
        .map(|round| {
            (0..NUM_TXNS_PER_ROUND)
                .map(|tx_num| TestTransaction::new(vec![round as u8, tx_num as u8]))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // set up state to track between simulated consensus rounds
    let mut prev_proposed_transactions: Option<Vec<TestTransaction>> = None;
    let mut transaction_history = Vec::new();

    let mut chain_state = SimulatedChainState::new(event_stream_sender.clone());

    // Simulate NUM_ROUNDS of consensus. First we submit the transactions for this round to the builder,
    // then construct DA and Quorum Proposals based on what we received from builder in the previous round
    // and request a new bundle.
    #[allow(clippy::needless_range_loop)] // intent is clearer this way
    for round in 0..NUM_ROUNDS {
        // All tiered maps shouldn't be pruned
        assert_eq!(
            *global_state
                .block_store
                .read()
                .await
                .blocks
                .lowest_view()
                .unwrap_or(ViewNumber::genesis()),
            0,
        );
        assert_eq!(
            *global_state
                .block_store
                .read()
                .await
                .block_cache
                .lowest_view()
                .unwrap_or(ViewNumber::genesis()),
            0,
        );
        assert_eq!(*global_state.coordinator.lowest_view().await, 0);

        // simulate transaction being submitted to the builder
        test_service
            .submit_transactions(all_transactions[round].clone())
            .await;

        // get transactions submitted in previous rounds, [] for genesis
        // and simulate the block built from those
        let builder_state_id = chain_state
            .simulate_consensus_round(prev_proposed_transactions)
            .await;

        // get response
        let transactions = test_service.get_transactions(&builder_state_id).await;

        // in the next round we will use received transactions to simulate
        // the block being proposed
        prev_proposed_transactions = Some(transactions.clone());
        // save transactions to history
        transaction_history.extend(transactions);
    }

    // Send a bogus decide event. The only thing we care about is the leaf's view number,
    // everything else is boilerplate.

    let mock_qc =
        QuorumCertificate2::genesis::<TestVersions>(&Default::default(), &Default::default()).await;
    let leaf = Leaf2::from_quorum_proposal(&QuorumProposalWrapper {
        proposal: QuorumProposal2 {
            block_header: <TestBlockHeader as BlockHeader<TestTypes>>::genesis::<TestVersions>(
                &Default::default(),
                TestBlockPayload::genesis(),
                &TestMetadata {
                    num_transactions: 0,
                },
            ),
            view_number: ViewNumber::new(DECIDE_VIEW),
            justify_qc: mock_qc.clone(),
            upgrade_certificate: None,
            view_change_evidence: None,
            next_epoch_justify_qc: None,
            next_drb_result: None,
            epoch: None,
            state_cert: None,
        },
    });
    event_stream_sender
        .broadcast(hotshot::types::Event {
            view_number: ViewNumber::new(NUM_ROUNDS as u64),
            event: EventType::Decide {
                leaf_chain: Arc::new(vec![LeafInfo {
                    leaf,
                    state: Arc::new(TestValidatedState::default()),
                    delta: None,
                    vid_share: None,
                    state_cert: None,
                }]),
                qc: Arc::new(mock_qc),
                block_size: None,
            },
        })
        .await
        .unwrap();

    // Give builder time to handle decide event
    sleep(Duration::from_millis(100)).await;

    // Tiered maps should be pruned now
    assert_eq!(
        *global_state
            .block_store
            .read()
            .await
            .blocks
            .lowest_view()
            .unwrap(),
        DECIDE_VIEW
    );
    assert_eq!(
        *global_state
            .block_store
            .read()
            .await
            .block_cache
            .lowest_view()
            .unwrap(),
        DECIDE_VIEW
    );
    assert_eq!(*global_state.coordinator.lowest_view().await, DECIDE_VIEW);
}

#[tokio::test]
#[traced_test]
async fn test_signature_checks() {
    let expected_signing_keys = &MOCK_LEADER_KEYS;
    let wrong_signing_key =
        <<TestTypes as NodeType>::SignatureKey as SignatureKey>::generated_from_seed_indexed(
            [0; 32], 1,
        )
        .1;

    // Signature over wrong data by expected signing key
    let signature_over_bogus_data = sign(&[42]);

    // Sign correct data with unexpected key
    let sign_with_wrong_key = |data| {
        <<TestTypes as NodeType>::SignatureKey as SignatureKey>::sign(&wrong_signing_key, data)
            .unwrap()
    };

    let builder_commitment = BuilderCommitment::from_bytes([]);
    let vid_commitment = VidCommitment::default();

    let global_state = GlobalState::<TestTypes>::new(
        BuilderConfig::test(),
        TestInstanceState::default(),
        TEST_PROTOCOL_MAX_BLOCK_SIZE,
        TEST_NUM_NODES_IN_VID_COMPUTATION,
    );

    let test_service = ProxyGlobalState(global_state);

    // Available blocks
    {
        // Verification  should fail if signature is over incorrect data
        let err = test_service
            .available_blocks(
                &vid_commitment,
                0,
                expected_signing_keys.0,
                &signature_over_bogus_data,
            )
            .await
            .expect_err("Signature verification should've failed");

        assert_eq_generic_err(err, Error::SignatureValidation);

        // Verification  should also fail if signature is over correct data but by incorrect key
        let err = test_service
            .available_blocks(
                &vid_commitment,
                0,
                expected_signing_keys.0,
                &sign_with_wrong_key(vid_commitment.as_ref()),
            )
            .await
            .expect_err("Signature verification should've failed");

        assert_eq_generic_err(err, Error::SignatureValidation);
    }

    // Claim block
    {
        // Verification  should fail if signature is over incorrect data
        let err = test_service
            .claim_block(
                &builder_commitment,
                0,
                expected_signing_keys.0,
                &signature_over_bogus_data,
            )
            .await
            .expect_err("Signature verification should've failed");

        assert_eq_generic_err(err, Error::SignatureValidation);

        // Verification  should also fail if signature is over correct data but by incorrect key
        let err = test_service
            .claim_block(
                &builder_commitment,
                0,
                expected_signing_keys.0,
                &sign_with_wrong_key(builder_commitment.as_ref()),
            )
            .await
            .expect_err("Signature verification should've failed");

        assert_eq_generic_err(err, Error::SignatureValidation);
    }

    // Claim block header input
    {
        // Verification  should fail if signature is over incorrect data
        let err = test_service
            .claim_block_header_input(
                &builder_commitment,
                0,
                expected_signing_keys.0,
                &signature_over_bogus_data,
            )
            .await
            .expect_err("Signature verification should've failed");

        assert_eq_generic_err(err, Error::SignatureValidation);

        // Verification  should also fail if signature is over correct data but by incorrect key
        let err = test_service
            .claim_block_header_input(
                &builder_commitment,
                0,
                expected_signing_keys.0,
                &sign_with_wrong_key(builder_commitment.as_ref()),
            )
            .await
            .expect_err("Signature verification should've failed");

        assert_eq_generic_err(err, Error::SignatureValidation);
    }
}
