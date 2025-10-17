use std::{collections::VecDeque, marker::PhantomData, sync::Arc, time::Duration};

use async_broadcast::{broadcast, Sender as BroadcastSender};
use async_lock::RwLock;
use committable::{Commitment, CommitmentBoundsArkless, Committable};
use hotshot::{
    traits::BlockPayload,
    types::{BLSPubKey, SignatureKey},
};
use hotshot_builder_shared::{
    block::{BuilderStateId, ParentBlockReferences},
    testing::constants::{
        TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD, TEST_MAX_TX_NUM, TEST_PROTOCOL_MAX_BLOCK_SIZE,
    },
};
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
    node_types::{TestTypes, TestVersions},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_types::{
    data::{
        vid_commitment, DaProposal2, Leaf2, QuorumProposal2, QuorumProposalWrapper, ViewNumber,
    },
    message::Proposal,
    simple_certificate::{QuorumCertificate2, SimpleCertificate, SuccessThreshold},
    simple_vote::QuorumData2,
    traits::{
        node_implementation::{ConsensusTime, Versions},
        EncodeBytes,
    },
    utils::{BuilderCommitment, EpochTransitionIndicator},
};
use vbs::version::StaticVersionType;

use crate::{
    builder_state::{
        BuilderState, DAProposalInfo, DaProposalMessage, MessageType, QuorumProposalMessage,
    },
    service::{GlobalState, ReceivedTransaction},
};

mod basic_test;
pub mod finalization_test;

pub async fn create_builder_state<V: Versions>(
    channel_capacity: usize,
    num_storage_nodes: usize,
) -> (
    BroadcastSender<MessageType<TestTypes>>,
    Arc<RwLock<GlobalState<TestTypes>>>,
    BuilderState<TestTypes, V>,
) {
    // set up the broadcast channels
    let (bootstrap_sender, bootstrap_receiver) =
        broadcast::<MessageType<TestTypes>>(channel_capacity);
    let (_decide_sender, decide_receiver) = broadcast::<MessageType<TestTypes>>(channel_capacity);
    let (da_sender, da_receiver) = broadcast::<MessageType<TestTypes>>(channel_capacity);
    let (_quorum_sender, quorum_proposal_receiver) =
        broadcast::<MessageType<TestTypes>>(channel_capacity);
    let (senders, _receivers) = broadcast::<MessageType<TestTypes>>(channel_capacity);
    let (tx_sender, tx_receiver) =
        broadcast::<Arc<ReceivedTransaction<TestTypes>>>(channel_capacity);

    let genesis_vid_commitment =
        vid_commitment::<V>(&[], &[], num_storage_nodes, <V as Versions>::Base::VERSION);
    let genesis_builder_commitment = BuilderCommitment::from_bytes([]);

    // instantiate the global state
    let global_state = Arc::new(RwLock::new(GlobalState::<TestTypes>::new(
        bootstrap_sender,
        tx_sender.clone(),
        da_sender.clone(),
        genesis_vid_commitment,
        ViewNumber::genesis(),
        ViewNumber::genesis(),
        TEST_MAX_BLOCK_SIZE_INCREMENT_PERIOD,
        TEST_PROTOCOL_MAX_BLOCK_SIZE,
        num_storage_nodes,
        TEST_MAX_TX_NUM,
    )));

    // instantiate the bootstrap builder state
    let builder_state = BuilderState::new(
        ParentBlockReferences {
            view_number: ViewNumber::new(0),
            vid_commitment: genesis_vid_commitment,
            leaf_commit: Commitment::<Leaf2<TestTypes>>::default_commitment_no_preimage(),
            builder_commitment: genesis_builder_commitment,
            // Unused in old legacy builder:
            last_nonempty_view: None,
            tx_count: 0,
        },
        decide_receiver.clone(),
        da_receiver.clone(),
        quorum_proposal_receiver.clone(),
        bootstrap_receiver,
        tx_receiver,
        VecDeque::new(),
        global_state.clone(),
        Duration::from_millis(100),
        1,
        Arc::new(TestInstanceState::default()),
        Duration::from_millis(100),
        Arc::new(TestValidatedState::default()),
    );

    (senders, global_state, builder_state)
}

/// get transactions submitted in previous rounds, [] for genesis
/// and simulate the block built from those
pub async fn calc_proposal_msg<V: Versions>(
    num_storage_nodes: usize,
    round: usize,
    prev_quorum_proposal: Option<QuorumProposalWrapper<TestTypes>>,
    transactions: Vec<TestTransaction>,
) -> (
    QuorumProposalWrapper<TestTypes>,
    QuorumProposalMessage<TestTypes>,
    DaProposalMessage<TestTypes>,
    BuilderStateId<TestTypes>,
) {
    // get transactions submitted in previous rounds, [] for genesis
    // and simulate the block built from those
    let num_transactions = transactions.len() as u64;
    let encoded_transactions = TestTransaction::encode(&transactions);
    let block_payload = TestBlockPayload { transactions };
    let metadata = TestMetadata { num_transactions };
    let block_vid_commitment = vid_commitment::<V>(
        &encoded_transactions,
        &metadata.encode(),
        num_storage_nodes,
        <V as Versions>::Base::VERSION,
    );
    let block_builder_commitment =
        <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(
            &block_payload,
            &metadata,
        );

    // generate key for leader of this round
    let seed = [round as u8; 32];
    let (pub_key, private_key) = BLSPubKey::generated_from_seed_indexed(seed, round as u64);

    // Prepare the DA proposal message
    let da_proposal_message: DaProposalMessage<TestTypes> = {
        let da_proposal = DaProposal2 {
            encoded_transactions: encoded_transactions.clone().into(),
            metadata: TestMetadata {
                num_transactions: encoded_transactions.len() as u64,
            },
            view_number: ViewNumber::new(round as u64),
            epoch: None,
            epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
        };
        DaProposalMessage::<TestTypes> {
            proposal: Arc::new(da_proposal),
        }
    };

    let block_header = TestBlockHeader {
        block_number: round as u64,
        payload_commitment: block_vid_commitment,
        builder_commitment: block_builder_commitment,
        timestamp: round as u64,
        timestamp_millis: round as u64 * 1_000,
        metadata,
        random: 1, // arbitrary
    };

    let justify_qc = match prev_quorum_proposal.as_ref() {
        None => {
            QuorumCertificate2::<TestTypes>::genesis::<TestVersions>(
                &TestValidatedState::default(),
                &TestInstanceState::default(),
            )
            .await
        },
        Some(prev_proposal) => {
            let prev_justify_qc = prev_proposal.justify_qc();
            let prev_leaf = Leaf2::from_quorum_proposal(prev_proposal);
            let quorum_data = QuorumData2::<TestTypes> {
                leaf_commit: prev_leaf.commit(),
                epoch: None,
                block_number: Some(prev_leaf.height()),
            };

            // form a justify qc
            SimpleCertificate::<TestTypes, QuorumData2<TestTypes>, SuccessThreshold>::new(
                quorum_data.clone(),
                quorum_data.commit(),
                prev_proposal.view_number(),
                prev_justify_qc.signatures.clone(),
                PhantomData,
            )
        },
    };

    tracing::debug!("Iteration: {round} justify_qc: {justify_qc:?}");

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

    let quorum_signature =
        <TestTypes as hotshot_types::traits::node_implementation::NodeType>::SignatureKey::sign(
            &private_key,
            block_vid_commitment.as_ref(),
        )
        .expect("Failed to sign payload commitment while preparing Quorum proposal");

    let quorum_proposal_msg = QuorumProposalMessage::<TestTypes> {
        proposal: Arc::new(Proposal {
            data: quorum_proposal.clone(),
            signature: quorum_signature,
            _pd: PhantomData,
        }),
        sender: pub_key,
    };

    let builder_state_id = BuilderStateId {
        parent_commitment: block_vid_commitment,
        parent_view: ViewNumber::new(round as u64),
    };
    (
        quorum_proposal,
        quorum_proposal_msg,
        da_proposal_message,
        builder_state_id,
    )
}

pub async fn calc_builder_commitment(
    da_proposal_message: DaProposalMessage<TestTypes>,
) -> (BuilderCommitment, DAProposalInfo<TestTypes>) {
    // If the respective builder state exists to handle the request
    let proposal = da_proposal_message.proposal.clone();
    // get the view number and encoded txns from the da_proposal_data
    let view_number = proposal.view_number;
    let encoded_txns = &proposal.encoded_transactions;

    let metadata = &proposal.metadata;
    // form a block payload from the encoded transactions
    let block_payload =
        <TestBlockPayload as BlockPayload<TestTypes>>::from_bytes(encoded_txns, metadata);
    // get the builder commitment from the block payload
    let payload_builder_commitment =
        <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(&block_payload, metadata);
    // form the DA proposal info
    let da_proposal_info = DAProposalInfo {
        view_number,
        proposal,
    };
    (payload_builder_commitment, da_proposal_info)
}
