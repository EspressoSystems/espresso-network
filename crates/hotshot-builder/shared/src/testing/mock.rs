//! A collection of generator functions for mock data used in tests
use std::{marker::PhantomData, sync::Arc, time::Duration};

use async_broadcast::broadcast;
use committable::{Commitment, Committable};
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestTransaction},
    node_types::{TestTypes, TestVersions},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_types::{
    data::{
        random_commitment, vid_commitment, DaProposal2, Leaf, Leaf2, QuorumProposal2,
        QuorumProposalWrapper, ViewNumber,
    },
    event::LeafInfo,
    message::UpgradeLock,
    simple_certificate::{QuorumCertificate, QuorumCertificate2},
    simple_vote::{QuorumData2, VersionedVoteData},
    traits::{
        block_contents::GENESIS_VID_NUM_STORAGE_NODES,
        node_implementation::{ConsensusTime, NodeType, Versions},
        BlockPayload, EncodeBytes,
    },
    utils::{BuilderCommitment, EpochTransitionIndicator},
    vid::advz::advz_scheme,
};
use jf_vid::VidScheme;
use rand::{distributions::Standard, thread_rng, Rng};
use vbs::version::StaticVersionType;

use super::constants::{TEST_CHANNEL_BUFFER_SIZE, TEST_NUM_NODES_IN_VID_COMPUTATION};
use crate::{block::ParentBlockReferences, state::BuilderState};

pub fn transaction() -> TestTransaction {
    TestTransaction::new(
        thread_rng()
            .sample_iter(Standard)
            .take(100)
            .collect::<Vec<_>>(),
    )
}

pub async fn decide_leaf_chain(decided_view: u64) -> Arc<Vec<LeafInfo<TestTypes>>> {
    decide_leaf_chain_with_transactions(decided_view, vec![transaction()]).await
}

pub async fn decide_leaf_chain_with_transactions(
    decided_view: u64,
    transactions: Vec<TestTransaction>,
) -> Arc<Vec<LeafInfo<TestTypes>>> {
    let (da_proposal, quorum_proposal) =
        proposals_with_transactions(decided_view, transactions).await;
    let mut leaf = Leaf2::from_quorum_proposal(&quorum_proposal);
    let payload = <TestBlockPayload as BlockPayload<TestTypes>>::from_bytes(
        &da_proposal.encoded_transactions,
        &da_proposal.metadata,
    );
    leaf.fill_block_payload_unchecked(payload);
    Arc::new(vec![LeafInfo {
        leaf,
        state: Default::default(),
        delta: None,
        vid_share: None,
        state_cert: None,
    }])
}

/// Create mock pair of DA and Quorum proposals
pub async fn proposals(view: u64) -> (DaProposal2<TestTypes>, QuorumProposalWrapper<TestTypes>) {
    let transaction = transaction();
    proposals_with_transactions(view, vec![transaction]).await
}

/// Create mock pair of DA and Quorum proposals with given transactions
pub async fn proposals_with_transactions(
    view: u64,
    transactions: Vec<TestTransaction>,
) -> (DaProposal2<TestTypes>, QuorumProposalWrapper<TestTypes>) {
    let epoch = None;
    let view_number = <TestTypes as NodeType>::View::new(view);
    let upgrade_lock = UpgradeLock::<TestTypes, TestVersions>::new();
    let validated_state = TestValidatedState::default();
    let instance_state = TestInstanceState::default();

    let (payload, metadata) = <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
        transactions.clone(),
        &validated_state,
        &instance_state,
    )
    .await
    .unwrap();
    let encoded_transactions = TestTransaction::encode(&transactions);

    let header = TestBlockHeader::new(
        &Leaf::<TestTypes>::genesis::<TestVersions>(&Default::default(), &Default::default())
            .await
            .into(),
        vid_commitment::<TestVersions>(
            &encoded_transactions,
            &metadata.encode(),
            GENESIS_VID_NUM_STORAGE_NODES,
            <TestVersions as Versions>::Base::VERSION,
        ),
        <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(&payload, &metadata),
        metadata,
    );

    let genesis_qc = QuorumCertificate::<TestTypes>::genesis::<TestVersions>(
        &TestValidatedState::default(),
        &TestInstanceState::default(),
    )
    .await
    .to_qc2();
    let parent_proposal = QuorumProposalWrapper {
        proposal: QuorumProposal2 {
            block_header: header,
            view_number: ViewNumber::new(view_number.saturating_sub(1)),
            justify_qc: genesis_qc,
            upgrade_certificate: None,
            view_change_evidence: None,
            next_drb_result: None,
            next_epoch_justify_qc: None,
            epoch,
            state_cert: None,
        },
    };
    let leaf = Leaf2::from_quorum_proposal(&parent_proposal);

    let quorum_data = QuorumData2 {
        leaf_commit: leaf.commit(),
        epoch,
        block_number: Some(leaf.height()),
    };

    let versioned_data = VersionedVoteData::<_, _, _>::new_infallible(
        quorum_data.clone(),
        view_number,
        &upgrade_lock,
    )
    .await;

    let commitment = Commitment::from_raw(versioned_data.commit().into());

    let justify_qc =
        QuorumCertificate2::new(quorum_data, commitment, view_number, None, PhantomData);

    (
        DaProposal2 {
            encoded_transactions: encoded_transactions.into(),
            metadata,
            view_number,
            epoch,
            epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
        },
        QuorumProposalWrapper {
            proposal: QuorumProposal2 {
                block_header: leaf.block_header().clone(),
                view_number,
                justify_qc,
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                next_epoch_justify_qc: None,
                epoch,
                state_cert: None,
            },
        },
    )
}

pub fn builder_state(view: u64) -> Arc<BuilderState<TestTypes>> {
    let references = parent_references(view);
    let (_, receiver) = broadcast(TEST_CHANNEL_BUFFER_SIZE);
    BuilderState::new(
        references,
        Duration::from_secs(1),
        receiver,
        TestValidatedState::default(),
    )
}

/// Generate references for given view number with random
/// commitments for use in testing code
pub fn parent_references(view: u64) -> ParentBlockReferences<TestTypes> {
    let rng = &mut thread_rng();
    ParentBlockReferences {
        view_number: <TestTypes as NodeType>::View::new(view),
        leaf_commit: random_commitment(rng),
        vid_commitment: hotshot_types::data::VidCommitment::V0(
            advz_scheme(TEST_NUM_NODES_IN_VID_COMPUTATION)
                .commit_only(rng.sample_iter(Standard).take(100).collect::<Vec<_>>())
                .unwrap(),
        ),
        builder_commitment: BuilderCommitment::from_bytes(
            rng.sample_iter(Standard).take(32).collect::<Vec<_>>(),
        ),
        tx_count: rng.gen(),
        last_nonempty_view: None,
    }
}
