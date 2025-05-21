use std::{sync::Arc, time::Duration};

use async_broadcast::{broadcast, Sender};
use async_lock::RwLock;
use committable::Commitment;
use hotshot::{
    traits::BlockPayload,
    types::{BLSPubKey, SignatureKey},
};
use hotshot_builder_api::v0_1::{
    block_info::AvailableBlockInfo,
    builder::BuildError,
    data_source::{AcceptsTxnSubmits, BuilderDataSource},
};
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
    node_types::{TestTypes, TestVersions},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_types::{
    data::{vid_commitment, DaProposal2, QuorumProposal2, QuorumProposalWrapper, ViewNumber},
    message::Proposal,
    simple_certificate::QuorumCertificate2,
    traits::{
        block_contents::BlockHeader,
        node_implementation::{ConsensusTime, Versions},
        EncodeBytes,
    },
    utils::{BuilderCommitment, EpochTransitionIndicator},
};
use marketplace_builder_shared::{
    block::{BuilderStateId, ParentBlockReferences},
    testing::constants::{
        TEST_CHANNEL_BUFFER_SIZE, TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD, TEST_MAX_TX_NUM,
        TEST_NUM_CONSENSUS_RETRIES, TEST_NUM_NODES_IN_VID_COMPUTATION,
        TEST_PROTOCOL_MAX_BLOCK_SIZE,
    },
};
use sha2::{Digest, Sha256};
use vbs::version::StaticVersionType;

use super::basic_test::{BuilderState, MessageType};
use crate::{
    builder_state::{DaProposalMessage, QuorumProposalMessage, ALLOW_EMPTY_BLOCK_PERIOD},
    service::{GlobalState, ProxyGlobalState, ReceivedTransaction},
};

type TestSetup = (
    ProxyGlobalState<TestTypes>,
    async_broadcast::Sender<MessageType<TestTypes>>,
    async_broadcast::Sender<MessageType<TestTypes>>,
    async_broadcast::Sender<MessageType<TestTypes>>,
    async_broadcast::Sender<Arc<ReceivedTransaction<TestTypes>>>,
);

/// [`setup_builder_for_test`] sets up a test environment for the builder state.
/// It returns a tuple containing the proxy global state, the sender for decide
/// messages, the sender for data availability proposals,
pub fn setup_builder_for_test() -> TestSetup {
    let (req_sender, req_receiver) = broadcast(TEST_CHANNEL_BUFFER_SIZE);
    let (tx_sender, tx_receiver) = broadcast(TEST_CHANNEL_BUFFER_SIZE);

    let parent_commitment = vid_commitment::<TestVersions>(
        &[],
        &[],
        TEST_NUM_NODES_IN_VID_COMPUTATION,
        <TestVersions as Versions>::Base::VERSION,
    );
    let bootstrap_builder_state_id = BuilderStateId::<TestTypes> {
        parent_commitment,
        parent_view: ViewNumber::genesis(),
    };

    let global_state = Arc::new(RwLock::new(GlobalState::new(
        req_sender,
        tx_sender.clone(),
        bootstrap_builder_state_id.parent_commitment,
        bootstrap_builder_state_id.parent_view,
        bootstrap_builder_state_id.parent_view,
        TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
        TEST_PROTOCOL_MAX_BLOCK_SIZE,
        TEST_NUM_NODES_IN_VID_COMPUTATION,
        TEST_MAX_TX_NUM,
    )));

    let max_api_duration = Duration::from_millis(100);

    let proxy_global_state = ProxyGlobalState::new(
        global_state.clone(),
        BLSPubKey::generated_from_seed_indexed([1; 32], 0),
        max_api_duration,
    );

    let (decide_sender, decide_receiver) = broadcast(TEST_CHANNEL_BUFFER_SIZE);
    let (da_proposal_sender, da_proposal_receiver) = broadcast(TEST_CHANNEL_BUFFER_SIZE);
    let (quorum_proposal_sender, quorum_proposal_receiver) = broadcast(TEST_CHANNEL_BUFFER_SIZE);
    let bootstrap_builder_state = BuilderState::<TestTypes, TestVersions>::new(
        ParentBlockReferences {
            vid_commitment: parent_commitment,
            view_number: ViewNumber::genesis(),
            leaf_commit: Commitment::from_raw([0; 32]),
            builder_commitment: BuilderCommitment::from_bytes([0; 32]),
            // Unused in old legacy builder:
            last_nonempty_view: None,
            tx_count: 0,
        },
        decide_receiver,
        da_proposal_receiver,
        quorum_proposal_receiver,
        req_receiver,
        tx_receiver,
        Default::default(),
        global_state.clone(),
        Duration::from_millis(40),
        1,
        Default::default(),
        Duration::from_secs(1),
        Default::default(),
    );

    bootstrap_builder_state.event_loop();

    (
        proxy_global_state,
        decide_sender,
        da_proposal_sender,
        quorum_proposal_sender,
        tx_sender,
    )
}

/// [`process_available_blocks_round`] processes available rounds for a given
/// round. It returns the number of attempts made to get the available blocks
/// and the result of the available blocks.
///
/// By default Consensus will retry 3-4 times to get available blocks from the
/// Builder.
pub async fn process_available_blocks_round(
    proxy_global_state: &ProxyGlobalState<TestTypes>,
    builder_state_id: BuilderStateId<TestTypes>,
    round: u64,
) -> (
    usize,
    Result<Vec<AvailableBlockInfo<TestTypes>>, BuildError>,
) {
    let (leader_pub, leader_priv) = BLSPubKey::generated_from_seed_indexed([0; 32], round);

    let current_commit_signature = <BLSPubKey as SignatureKey>::sign(
        &leader_priv,
        builder_state_id.parent_commitment.as_ref(),
    )
    .unwrap();

    // Simulate Consensus retries

    let mut attempt = 0;
    loop {
        attempt += 1;

        let available_blocks_result = proxy_global_state
            .available_blocks(
                &builder_state_id.parent_commitment,
                builder_state_id.parent_view.u64(),
                leader_pub,
                &current_commit_signature,
            )
            .await;

        if available_blocks_result.is_ok() {
            return (attempt, available_blocks_result);
        }

        if attempt >= TEST_NUM_CONSENSUS_RETRIES {
            return (attempt, available_blocks_result);
        }
    }
}

/// [`progress_round_with_available_block_info`] is a helper function that
/// progresses the round with the information returned from a call to
/// [`process_available_blocks_round`]. This function simulates decide events
/// if the next call to [`ProxyGlobalState::available_blocks`] returns something
/// successfully rather than an error.
///
/// This is the workflow that happens if the builder has a block to propose,
/// and the block is included by consensus.
pub async fn progress_round_with_available_block_info(
    proxy_global_state: &ProxyGlobalState<TestTypes>,
    available_block_info: AvailableBlockInfo<TestTypes>,
    builder_state_id: BuilderStateId<TestTypes>,
    round: u64,
    da_proposal_sender: &Sender<MessageType<TestTypes>>,
    quorum_proposal_sender: &Sender<MessageType<TestTypes>>,
) -> BuilderStateId<TestTypes> {
    let (leader_pub, leader_priv) = BLSPubKey::generated_from_seed_indexed([0; 32], round);

    let signed_parent_commitment =
        <BLSPubKey as SignatureKey>::sign(&leader_priv, available_block_info.block_hash.as_ref())
            .unwrap();

    let claim_block_result = proxy_global_state
        .claim_block(
            &available_block_info.block_hash,
            builder_state_id.parent_view.u64(),
            leader_pub,
            &signed_parent_commitment,
        )
        .await
        .unwrap_or_else(|_| panic!("claim block should succeed for round {round}"));

    let _claim_block_header_result = proxy_global_state
        .claim_block_header_input(
            &available_block_info.block_hash,
            builder_state_id.parent_view.u64(),
            leader_pub,
            &signed_parent_commitment,
        )
        .await
        .unwrap_or_else(|_| panic!("claim block header input should succeed for round {round}"));

    progress_round_with_transactions(
        builder_state_id,
        claim_block_result.block_payload.transactions,
        round,
        da_proposal_sender,
        quorum_proposal_sender,
    )
    .await
}

/// [`progress_round_without_available_block_info`] is a helper function that
/// progresses the round without any available block information.
///
/// This is the workflow that happens if the builder does not have a block to
/// propose, and consensus must continue to progress without a block built by
/// any builder.
pub async fn progress_round_without_available_block_info(
    builder_state_id: BuilderStateId<TestTypes>,
    round: u64,
    da_proposal_sender: &Sender<MessageType<TestTypes>>,
    quorum_proposal_sender: &Sender<MessageType<TestTypes>>,
) -> BuilderStateId<TestTypes> {
    progress_round_with_transactions(
        builder_state_id,
        vec![],
        round,
        da_proposal_sender,
        quorum_proposal_sender,
    )
    .await
}

/// [`progress_round_with_transactions`] is a helper function that progress
/// consensus with the given list of transactions.
///
/// This function is used by [`progress_round_without_available_block_info`] and
/// by [`progress_round_with_available_block_info`] to progress the round with
/// the given transactions.
async fn progress_round_with_transactions(
    builder_state_id: BuilderStateId<TestTypes>,
    transactions: Vec<TestTransaction>,
    round: u64,
    da_proposal_sender: &Sender<MessageType<TestTypes>>,
    quorum_proposal_sender: &Sender<MessageType<TestTypes>>,
) -> BuilderStateId<TestTypes> {
    let (leader_pub, leader_priv) = BLSPubKey::generated_from_seed_indexed([0; 32], round);
    let encoded_transactions = TestTransaction::encode(&transactions);
    let next_view = builder_state_id.parent_view + 1;

    // Create and send the DA Proposals and Quorum Proposals
    {
        let encoded_transactions_hash = Sha256::digest(&encoded_transactions);
        let da_signature =
        <TestTypes as hotshot_types::traits::node_implementation::NodeType>::SignatureKey::sign(
            &leader_priv,
            &encoded_transactions_hash,
        )
        .expect("should sign encoded transactions hash successfully");

        let metadata = TestMetadata {
            num_transactions: transactions.len() as u64,
        };

        da_proposal_sender
            .broadcast(MessageType::DaProposalMessage(DaProposalMessage {
                proposal: Arc::new(Proposal {
                    data: DaProposal2::<TestTypes> {
                        encoded_transactions: encoded_transactions.clone().into(),
                        metadata,
                        view_number: next_view,
                        epoch: None, // TODO
                        epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
                    },
                    signature: da_signature,
                    _pd: Default::default(),
                }),
                sender: leader_pub,
            }))
            .await
            .expect("should broadcast DA Proposal successfully");

        let payload_commitment = vid_commitment::<TestVersions>(
            &encoded_transactions,
            &metadata.encode(),
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );

        let (block_payload, metadata) =
            <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
                transactions,
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await
            .unwrap();

        let builder_commitment = <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(
            &block_payload,
            &metadata,
        );

        let block_header = TestBlockHeader {
            block_number: round,
            payload_commitment,
            builder_commitment,
            timestamp: round,
            metadata,
            random: 0,
        };

        let qc_proposal = QuorumProposalWrapper::<TestTypes> {
            proposal: QuorumProposal2::<TestTypes> {
                block_header,
                view_number: next_view,
                justify_qc: QuorumCertificate2::<TestTypes>::genesis::<TestVersions>(
                    &TestValidatedState::default(),
                    &TestInstanceState::default(),
                )
                .await,
                upgrade_certificate: None,
                view_change_evidence: None,
                next_epoch_justify_qc: None,
                next_drb_result: None,
                epoch: None,
                state_cert: None,
            },
        };

        let payload_vid_commitment =
            <TestBlockHeader as BlockHeader<TestTypes>>::payload_commitment(
                qc_proposal.block_header(),
            );

        let qc_signature = <TestTypes as hotshot_types::traits::node_implementation::NodeType>::SignatureKey::sign(
                        &leader_priv,
                        payload_vid_commitment.as_ref(),
                        ).expect("Failed to sign payload commitment while preparing QC proposal");

        quorum_proposal_sender
            .broadcast(MessageType::QuorumProposalMessage(QuorumProposalMessage {
                proposal: Arc::new(Proposal {
                    data: qc_proposal.clone(),
                    signature: qc_signature,
                    _pd: Default::default(),
                }),
                sender: leader_pub,
            }))
            .await
            .expect("should broadcast QC Proposal successfully");

        BuilderStateId {
            parent_commitment: payload_vid_commitment,
            parent_view: next_view,
        }
    }
}

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
    let (proxy_global_state, _, da_proposal_sender, quorum_proposal_sender, _) =
        setup_builder_for_test();

    let mut current_builder_state_id = BuilderStateId::<TestTypes> {
        parent_commitment: vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        ),
        parent_view: ViewNumber::genesis(),
    };

    for round in 0..10 {
        let (attempts, available_available_blocks_result) = process_available_blocks_round(
            &proxy_global_state,
            current_builder_state_id.clone(),
            round,
        )
        .await;

        assert_eq!(
            attempts, TEST_NUM_CONSENSUS_RETRIES,
            "Consensus should retry {TEST_NUM_CONSENSUS_RETRIES} times to get available blocks"
        );
        assert!(available_available_blocks_result.is_err());

        current_builder_state_id = progress_round_without_available_block_info(
            current_builder_state_id,
            round,
            &da_proposal_sender,
            &quorum_proposal_sender,
        )
        .await;
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
/// It then checks that the remaining round up to 9 will not propose any empty
/// blocks.
///
/// |> Note: this test simulates how consensus interacts with the Builder in a
/// |> very basic way.  When consensus asks for available blocks, and the
/// |> Builder returns an error that indicates that it does not have any blocks
/// |> to propose, consensus will retry a few times before giving up. As a
/// |> result the number of times that consensus has to ask the Builder for
/// |> block is an integral part of this test.
#[tokio::test]
async fn test_eager_block_rate() {
    let (proxy_global_state, _, da_proposal_sender, quorum_proposal_sender, _) =
        setup_builder_for_test();

    let mut current_builder_state_id = BuilderStateId::<TestTypes> {
        parent_commitment: vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        ),
        parent_view: ViewNumber::genesis(),
    };

    // Round 0
    {
        let round = 0;
        let (attempts, available_available_blocks_result) = process_available_blocks_round(
            &proxy_global_state,
            current_builder_state_id.clone(),
            round,
        )
        .await;

        assert_eq!(
            attempts, TEST_NUM_CONSENSUS_RETRIES,
            "Consensus should retry {TEST_NUM_CONSENSUS_RETRIES} times to get available blocks for round {round}"
        );

        assert!(
            available_available_blocks_result.is_err(),
            "builder should not propose empty blocks for round {round}"
        );

        current_builder_state_id = progress_round_without_available_block_info(
            current_builder_state_id,
            round,
            &da_proposal_sender,
            &quorum_proposal_sender,
        )
        .await;
    }

    // Round 1, submit a single transaction, and advance the round
    {
        proxy_global_state
            .submit_txns(vec![TestTransaction::new(vec![1])])
            .await
            .expect("should submit transaction without issue");

        let round = 1;

        let (attempts, available_available_blocks_result) = process_available_blocks_round(
            &proxy_global_state,
            current_builder_state_id.clone(),
            round,
        )
        .await;

        assert_eq!(
            attempts, 1,
            "Consensus should not have needed to retry at all for round {round}"
        );

        assert!(
            available_available_blocks_result.is_ok(),
            "builder should be proposing empty blocks for round {round}"
        );

        current_builder_state_id = progress_round_with_available_block_info(
            &proxy_global_state,
            available_available_blocks_result.unwrap()[0].clone(),
            current_builder_state_id,
            round,
            &da_proposal_sender,
            &quorum_proposal_sender,
        )
        .await;
    }

    // rounds 2 through 2 + ALLOW_EMPTY_BLOCK_PERIOD - 1 should propose empty
    // blocks.
    for round in 2..(2 + ALLOW_EMPTY_BLOCK_PERIOD) {
        let (attempts, available_blocks_result) = process_available_blocks_round(
            &proxy_global_state,
            current_builder_state_id.clone(),
            round,
        )
        .await;

        assert_eq!(
            attempts, 1,
            "Consensus should not have needed to retry at all for round {round}"
        );

        assert!(
            available_blocks_result.is_ok(),
            "builder should be proposing empty blocks for round {round}"
        );

        let available_blocks_result = available_blocks_result.unwrap();

        assert_eq!(
            available_blocks_result[0].block_size, 0,
            "the block should be empty for round {round}"
        );

        current_builder_state_id = progress_round_with_available_block_info(
            &proxy_global_state,
            available_blocks_result[0].clone(),
            current_builder_state_id,
            round,
            &da_proposal_sender,
            &quorum_proposal_sender,
        )
        .await;
    }

    // rounds 2 + ALLOW_EMPTY_BLOCK_PERIOD through 9 should not propose empty
    for round in (2 + ALLOW_EMPTY_BLOCK_PERIOD)..10 {
        let (attempts, available_blocks_result) = process_available_blocks_round(
            &proxy_global_state,
            current_builder_state_id.clone(),
            round,
        )
        .await;

        assert_eq!(
            attempts, TEST_NUM_CONSENSUS_RETRIES,
            "Consensus should have retries {TEST_NUM_CONSENSUS_RETRIES} times for round {round}"
        );
        assert!(available_blocks_result.is_err());

        current_builder_state_id = progress_round_without_available_block_info(
            current_builder_state_id,
            round,
            &da_proposal_sender,
            &quorum_proposal_sender,
        )
        .await;
    }
}
