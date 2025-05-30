pub use async_broadcast::broadcast;
pub use hotshot::traits::election::static_committee::StaticCommittee;
pub use hotshot_types::{
    data::{EpochNumber, Leaf, ViewNumber},
    message::Proposal,
    signature_key::BLSPubKey,
    simple_certificate::{QuorumCertificate2, SimpleCertificate, SuccessThreshold},
    traits::{
        block_contents::BlockPayload,
        node_implementation::{ConsensusTime, NodeType},
    },
};
use vbs::version::StaticVersionType;

pub use crate::builder_state::{BuilderState, MessageType};
/// The following tests are performed:
#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, hash::Hash, marker::PhantomData, sync::Arc, time::Duration};

    use async_lock::RwLock;
    use committable::{Commitment, CommitmentBoundsArkless, Committable};
    use hotshot::types::SignatureKey;
    use hotshot_builder_api::v0_2::data_source::BuilderDataSource;
    use hotshot_example_types::{
        block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
        node_types::TestVersions,
        state_types::{TestInstanceState, TestValidatedState},
    };
    use hotshot_types::{
        data::{vid_commitment, DaProposal2, Leaf2, QuorumProposal2, QuorumProposalWrapper},
        signature_key::{BuilderKey, SchnorrPubKey},
        simple_vote::QuorumData2,
        traits::{block_contents::BlockHeader, node_implementation::Versions, EncodeBytes},
        utils::{BuilderCommitment, EpochTransitionIndicator},
    };
    use marketplace_builder_shared::{
        block::ParentBlockReferences,
        testing::constants::{
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD, TEST_MAX_TX_NUM,
            TEST_NUM_NODES_IN_VID_COMPUTATION, TEST_PROTOCOL_MAX_BLOCK_SIZE,
        },
    };
    use serde::{Deserialize, Serialize};
    use sha2::{Digest, Sha256};
    use tokio::time::{error::Elapsed, timeout};
    use tracing_subscriber::EnvFilter;

    use super::*;
    use crate::{
        builder_state::{
            DaProposalMessage, DecideMessage, QuorumProposalMessage, TransactionSource,
        },
        service::{handle_received_txns, GlobalState, ProxyGlobalState, ReceivedTransaction},
    };
    /// This test simulates multiple builder states receiving messages from the channels and processing them
    #[tokio::test]
    //#[instrument]
    async fn test_builder() {
        // Setup logging
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
        tracing::info!("Testing the builder core with multiple messages from the channels");
        #[derive(
            Copy,
            Clone,
            Debug,
            Default,
            Hash,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Serialize,
            Deserialize,
        )]
        struct TestTypes;
        impl NodeType for TestTypes {
            type View = ViewNumber;
            type Epoch = EpochNumber;
            type BlockHeader = TestBlockHeader;
            type BlockPayload = TestBlockPayload;
            type SignatureKey = BLSPubKey;
            type Transaction = TestTransaction;
            type ValidatedState = TestValidatedState;
            type InstanceState = TestInstanceState;
            type Membership = StaticCommittee<Self>;
            type BuilderSignatureKey = BuilderKey;
            type StateSignatureKey = SchnorrPubKey;
        }
        // no of test messages to send
        let num_test_messages = 5;
        let multiplication_factor = 5;
        const NUM_NODES_IN_VID_COMPUTATION: usize = 4;

        // settingup the broadcast channels i.e [From hostshot: (tx, decide, da, quorum, )], [From api:(req - broadcast, res - mpsc channel) ]
        let (decide_sender, decide_receiver) =
            broadcast::<MessageType<TestTypes>>(num_test_messages * multiplication_factor);
        let (da_sender, da_receiver) =
            broadcast::<MessageType<TestTypes>>(num_test_messages * multiplication_factor);
        let (quorum_sender, quorum_proposal_receiver) =
            broadcast::<MessageType<TestTypes>>(num_test_messages * multiplication_factor);
        let (bootstrap_sender, bootstrap_receiver) =
            broadcast::<MessageType<TestTypes>>(num_test_messages * multiplication_factor);
        let (tx_sender, tx_receiver) = broadcast::<Arc<ReceivedTransaction<TestTypes>>>(
            num_test_messages * multiplication_factor,
        );
        let tx_queue = VecDeque::new();
        // generate the keys for the builder
        let seed = [201_u8; 32];
        let (builder_pub_key, builder_private_key) =
            BLSPubKey::generated_from_seed_indexed(seed, 2011_u64);
        // instantiate the global state also
        let initial_commitment = vid_commitment::<TestVersions>(
            &[],
            &[],
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            <TestVersions as Versions>::Base::VERSION,
        );
        let global_state = Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
            bootstrap_sender,
            tx_sender.clone(),
            initial_commitment,
            ViewNumber::new(0),
            ViewNumber::new(0),
            TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
            TEST_PROTOCOL_MAX_BLOCK_SIZE,
            TEST_NUM_NODES_IN_VID_COMPUTATION,
            TEST_MAX_TX_NUM,
        )));

        let bootstrap_builder_state = BuilderState::<TestTypes, TestVersions>::new(
            ParentBlockReferences {
                view_number: ViewNumber::new(0),
                vid_commitment: initial_commitment,
                leaf_commit: Commitment::<Leaf2<TestTypes>>::default_commitment_no_preimage(),
                builder_commitment: BuilderCommitment::from_bytes([]),
                // Unused in old legacy builder:
                last_nonempty_view: None,
                tx_count: 0,
            },
            decide_receiver.clone(),
            da_receiver.clone(),
            quorum_proposal_receiver.clone(),
            bootstrap_receiver,
            tx_receiver,
            tx_queue,
            global_state.clone(),
            Duration::from_millis(100),
            1,
            Arc::new(TestInstanceState::default()),
            Duration::from_millis(100),
            Arc::new(TestValidatedState::default()),
        );

        // Kick off async look for the bootstrap builder state
        bootstrap_builder_state.event_loop();

        let proxy_global_state = ProxyGlobalState::new(
            global_state.clone(),
            (builder_pub_key, builder_private_key),
            Duration::from_millis(100),
        );

        // to store all the sent messages
        // storing response messages
        let mut previous_commitment = initial_commitment;
        let mut previous_view = ViewNumber::new(0);
        let mut previous_quorum_proposal = {
            let previous_jc = QuorumCertificate2::<TestTypes>::genesis::<TestVersions>(
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await;

            QuorumProposalWrapper::<TestTypes> {
                proposal: QuorumProposal2::<TestTypes> {
                    block_header: TestBlockHeader {
                        block_number: 0,
                        payload_commitment: previous_commitment,
                        builder_commitment: BuilderCommitment::from_bytes([]),
                        timestamp: 0,
                        metadata: TestMetadata {
                            num_transactions: 0,
                        },
                        random: 1, // arbitrary
                    },
                    view_number: ViewNumber::new(0),
                    justify_qc: previous_jc.clone(),
                    upgrade_certificate: None,
                    view_change_evidence: None,
                    next_epoch_justify_qc: None,
                    next_drb_result: None,
                    epoch: None,
                    state_cert: None,
                },
            }
        };

        // generate num_test messages for each type and send it to the respective channels;
        for round in 0..num_test_messages as u32 {
            // Submit Transactions to the Builder
            {
                // Prepare the transaction message
                let tx = TestTransaction::new(vec![round as u8]);

                let tx_vec = vec![tx];
                assert_eq!(
                    handle_received_txns(
                        &tx_sender,
                        tx_vec.clone(),
                        TransactionSource::HotShot,
                        u64::MAX,
                    )
                    .await
                    .into_iter()
                    .map(|res| res.unwrap())
                    .collect::<Vec<_>>(),
                    tx_vec.iter().map(|tx| tx.commit()).collect::<Vec<_>>(),
                    "handle_received_txns should have the commits for all transactions submitted",
                );
            }

            // Request available blocks from the builder
            {
                let (leader_pub, leader_priv) =
                    BLSPubKey::generated_from_seed_indexed(seed, round as u64);

                let commitment_signature =
                    <BLSPubKey as SignatureKey>::sign(&leader_priv, previous_commitment.as_ref())
                        .unwrap();

                let available_blocks = proxy_global_state
                    .available_blocks(
                        &previous_commitment,
                        previous_view.u64(),
                        leader_pub,
                        &commitment_signature,
                    )
                    .await
                    .unwrap();

                // The available blocks should **NOT** be empty
                assert!(
                    available_blocks.len() == 1,
                    "available blocks should return a single entry"
                );
                assert!(
                    available_blocks[0].offered_fee >= 1,
                    "offered fee should be greater than 1"
                );

                let block_hash = available_blocks[0].block_hash.clone();

                // Let's claim this block, and this block header
                let block_hash_signature =
                    <BLSPubKey as SignatureKey>::sign(&leader_priv, block_hash.as_ref()).unwrap();

                let claimed_block = proxy_global_state
                    .claim_block_with_num_nodes(
                        &block_hash,
                        previous_view.u64(),
                        leader_pub,
                        &block_hash_signature,
                        // Increment to test whether `num_nodes` is updated properly.
                        TEST_NUM_NODES_IN_VID_COMPUTATION + 1,
                    )
                    .await
                    .unwrap();

                let _claimed_block_header = proxy_global_state
                    .claim_block_header_input(
                        &block_hash,
                        previous_view.u64(),
                        leader_pub,
                        &block_hash_signature,
                    )
                    .await
                    .unwrap();

                // Create the proposals from the transactions contained within
                // the claim_block result.

                let proposed_transactions = claimed_block.block_payload.transactions.clone();
                assert_eq!(
                    proposed_transactions.len(),
                    1,
                    "there should be one transaction in the proposed block"
                );

                let encoded_transactions = TestTransaction::encode(&proposed_transactions);

                // Prepare the DA proposal message
                let da_proposal_message = {
                    let da_proposal = DaProposal2 {
                        encoded_transactions: encoded_transactions.clone().into(),
                        metadata: TestMetadata {
                            num_transactions: encoded_transactions.len() as u64,
                        },
                        view_number: ViewNumber::new(round as u64),
                        epoch: None, // TODO: check if this is okay
                        epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
                    };
                    let encoded_transactions_hash = Sha256::digest(&encoded_transactions);
                    let seed = [round as u8; 32];
                    let (pub_key, private_key) =
                        BLSPubKey::generated_from_seed_indexed(seed, round as u64);
                    let da_signature =
                <TestTypes as hotshot_types::traits::node_implementation::NodeType>::SignatureKey::sign(
                    &private_key,
                    &encoded_transactions_hash,
                )
                .expect("Failed to sign encoded tx hash while preparing da proposal");

                    DaProposalMessage::<TestTypes> {
                        proposal: Arc::new(Proposal {
                            data: da_proposal,
                            signature: da_signature.clone(),
                            _pd: PhantomData,
                        }),
                        sender: pub_key,
                    }
                };

                // Prepare the Quorum proposal message
                // calculate the vid commitment over the encoded_transactions
                let quorum_certificate_message = {
                    let block_payload = claimed_block.block_payload.clone();
                    let metadata = claimed_block.metadata;

                    tracing::debug!(
                        "Encoded transactions: {:?} Num nodes:{}",
                        encoded_transactions,
                        NUM_NODES_IN_VID_COMPUTATION
                    );

                    let block_payload_commitment = vid_commitment::<TestVersions>(
                        &encoded_transactions,
                        &metadata.encode(),
                        NUM_NODES_IN_VID_COMPUTATION,
                        <TestVersions as Versions>::Base::VERSION,
                    );

                    tracing::debug!(
                        "Block Payload vid commitment: {:?}",
                        block_payload_commitment
                    );

                    let builder_commitment =
                        <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(
                            &block_payload,
                            &metadata,
                        );

                    let block_header = TestBlockHeader {
                        block_number: round as u64,
                        payload_commitment: block_payload_commitment,
                        builder_commitment,
                        timestamp: round as u64,
                        metadata,
                        random: 1, // arbitrary
                    };

                    let justify_qc = {
                        let previous_justify_qc = previous_quorum_proposal.justify_qc().clone();
                        // metadata
                        let _metadata = <TestBlockHeader as BlockHeader<TestTypes>>::metadata(
                            previous_quorum_proposal.block_header(),
                        );
                        let leaf = Leaf2::from_quorum_proposal(&previous_quorum_proposal);

                        let q_data = QuorumData2::<TestTypes> {
                            leaf_commit: leaf.commit(),
                            epoch: None, // TODO: check if this is okay
                            block_number: Some(leaf.height()),
                        };

                        let previous_quorum_view_number =
                            previous_quorum_proposal.view_number().u64();
                        let view_number = if previous_quorum_view_number == 0
                            && previous_justify_qc.view_number.u64() == 0
                        {
                            ViewNumber::new(0)
                        } else {
                            ViewNumber::new(1 + previous_justify_qc.view_number.u64())
                        };
                        // form a justify qc
                        SimpleCertificate::<TestTypes, QuorumData2<TestTypes>, SuccessThreshold>::new(
                            q_data.clone(),
                            q_data.commit(),
                            view_number,
                            previous_justify_qc.signatures.clone(),
                            PhantomData,
                        )
                    };

                    tracing::debug!("Iteration: {} justify_qc: {:?}", round, justify_qc);

                    let quorum_proposal = QuorumProposalWrapper::<TestTypes> {
                        proposal: QuorumProposal2::<TestTypes> {
                            block_header,
                            view_number: ViewNumber::new(round as u64),
                            justify_qc: justify_qc.clone(),
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
                            quorum_proposal.block_header(),
                        );

                    let quorum_signature = <BLSPubKey as SignatureKey>::sign(
                        &leader_priv,
                        payload_vid_commitment.as_ref(),
                    )
                    .expect("Failed to sign payload commitment while preparing Quorum proposal");

                    QuorumProposalMessage::<TestTypes> {
                        proposal: Arc::new(Proposal {
                            data: quorum_proposal.clone(),
                            signature: quorum_signature,
                            _pd: PhantomData,
                        }),
                        sender: leader_pub,
                    }
                };

                // Prepare the Decide Message
                // The Decide is mainly for cleanup of old BuilderStates.
                // This may not be necessary for this test
                let decide_message = {
                    let leaf = match round {
                        0 => Leaf::genesis::<TestVersions>(
                            &TestValidatedState::default(),
                            &TestInstanceState::default(),
                        )
                        .await
                        .into(),
                        _ => {
                            let block_payload = BlockPayload::<TestTypes>::from_bytes(
                                &encoded_transactions,
                                <TestBlockHeader as BlockHeader<TestTypes>>::metadata(
                                    quorum_certificate_message.proposal.data.block_header(),
                                ),
                            );
                            let mut current_leaf = Leaf2::from_quorum_proposal(
                                &quorum_certificate_message.proposal.data,
                            );
                            current_leaf
                                .fill_block_payload::<TestVersions>(
                                    block_payload,
                                    NUM_NODES_IN_VID_COMPUTATION,
                                    <TestVersions as Versions>::Base::VERSION,
                                )
                                .unwrap();
                            current_leaf
                        },
                    };

                    DecideMessage::<TestTypes> {
                        latest_decide_view_number: leaf.view_number(),
                    }
                };

                // Increment the view and the previous commitment
                previous_commitment = quorum_certificate_message
                    .proposal
                    .data
                    .block_header()
                    .payload_commitment;
                previous_view = quorum_certificate_message.proposal.data.view_number();
                previous_quorum_proposal =
                    quorum_certificate_message.proposal.as_ref().data.clone();

                // Broadcast the DA proposal
                da_sender
                    .broadcast(MessageType::DaProposalMessage(da_proposal_message))
                    .await
                    .unwrap();

                // Broadcast the Quorum Certificate
                quorum_sender
                    .broadcast(MessageType::QuorumProposalMessage(
                        quorum_certificate_message,
                    ))
                    .await
                    .unwrap();

                // Send the Decide Message
                decide_sender
                    .broadcast(MessageType::DecideMessage(decide_message))
                    .await
                    .unwrap();
            }
        }

        // We cloned these receivers to ensure that progress was being made
        // by the Builder processes.  Using these broadcast receivers we can
        // verify the number of messages received in this entire process, as
        // well as the order of them, should we be so inclined.

        // There should be num_test_messages messages in the receivers
        let mut da_receiver = da_receiver;
        let mut quorum_proposal_receiver = quorum_proposal_receiver;
        let mut decide_receiver = decide_receiver;
        for _ in 0..num_test_messages {
            da_receiver.recv().await.unwrap();
            quorum_proposal_receiver.recv().await.unwrap();
            decide_receiver.recv().await.unwrap();
        }

        // There should not be any other messages to receive
        let Err(Elapsed { .. }) = timeout(Duration::from_millis(100), da_receiver.recv()).await
        else {
            panic!("There should not be any more messages in the da_receiver");
        };
        let Err(Elapsed { .. }) =
            timeout(Duration::from_millis(100), quorum_proposal_receiver.recv()).await
        else {
            panic!("There should not be any more messages in the da_receiver");
        };
        let Err(Elapsed { .. }) = timeout(Duration::from_millis(100), decide_receiver.recv()).await
        else {
            panic!("There should not be any more messages in the da_receiver");
        };

        // Verify `num_nodes`.
        assert_eq!(
            global_state.read_arc().await.num_nodes,
            TEST_NUM_NODES_IN_VID_COMPUTATION + 1
        );
    }
}
