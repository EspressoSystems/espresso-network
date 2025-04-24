// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use hotshot::tasks::task_state::CreateTaskState;
use hotshot_example_types::{
    block_types::TestMetadata,
    node_types::{MemoryImpl, TestTypes, TestVersions},
    state_types::TestValidatedState,
};
use hotshot_macros::{run_test, test_scripts};
use hotshot_task_impls::{events::HotShotEvent::*, quorum_proposal::QuorumProposalTaskState};
use hotshot_testing::{
    all_predicates,
    helpers::{build_payload_commitment, build_system_handle},
    predicates::event::{all_predicates, quorum_proposal_send},
    random,
    script::{Expectations, InputOrder, TaskScript},
    serial,
    view_generator::TestViewGenerator,
};
use hotshot_types::{
    data::{null_block, EpochNumber, Leaf2, ViewChangeEvidence2, ViewNumber},
    simple_vote::{TimeoutData2, ViewSyncFinalizeData2},
    traits::node_implementation::{ConsensusTime, Versions},
    utils::BuilderCommitment,
};
use sha2::Digest;
use vec1::vec1;

const TIMEOUT: Duration = Duration::from_millis(35);

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_proposal_task_quorum_proposal_view_1() {
    use hotshot_testing::script::{Expectations, TaskScript};
    use vbs::version::StaticVersionType;

    hotshot::helpers::initialize_logging();

    let node_id = 1;
    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id).await;

    let membership = handle.hotshot.membership_coordinator.clone();
    let epoch_1_mem = membership
        .membership_for_epoch(Some(EpochNumber::new(1)))
        .await
        .unwrap();
    let version = handle
        .hotshot
        .upgrade_lock
        .version_infallible(ViewNumber::new(node_id))
        .await;

    let payload_commitment = build_payload_commitment::<TestTypes, TestVersions>(
        &epoch_1_mem,
        ViewNumber::new(node_id),
        version,
    )
    .await;

    let mut generator =
        TestViewGenerator::<TestVersions>::generate(membership.clone(), node_key_map);

    let mut proposals = Vec::new();
    let mut leaders = Vec::new();
    let mut leaves = Vec::new();
    let mut vids = Vec::new();
    let mut vid_dispersals = Vec::new();
    let consensus = handle.hotshot.consensus();
    let mut consensus_writer = consensus.write().await;
    for view in (&mut generator).take(2).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaders.push(view.leader_public_key);
        leaves.push(view.leaf.clone());
        vids.push(view.vid_proposal.clone());
        vid_dispersals.push(view.vid_disperse.clone());

        // We don't have a `QuorumProposalRecv` task handler, so we'll just manually insert the proposals
        // to make sure they show up during tests.
        consensus_writer
            .update_leaf(
                Leaf2::from_quorum_proposal(&view.quorum_proposal.data),
                Arc::new(TestValidatedState::default()),
                None,
            )
            .unwrap();
    }

    // We must send the genesis cert here to initialize hotshot successfully.
    let num_storage_node = epoch_1_mem.total_nodes().await;
    let genesis_cert = proposals[0].data.justify_qc().clone();
    let builder_commitment = BuilderCommitment::from_raw_digest(sha2::Sha256::new().finalize());
    let builder_fee = null_block::builder_fee::<TestTypes, TestVersions>(
        num_storage_node,
        <TestVersions as Versions>::Base::VERSION,
        *ViewNumber::new(1),
    )
    .unwrap();
    drop(consensus_writer);

    let inputs = vec![
        serial![VidDisperseSend(
            vid_dispersals[0].clone(),
            handle.public_key()
        )],
        random![
            Qc2Formed(either::Left(genesis_cert.clone())),
            SendPayloadCommitmentAndMetadata(
                payload_commitment,
                builder_commitment,
                TestMetadata {
                    num_transactions: 0
                },
                ViewNumber::new(1),
                vec1![builder_fee.clone()],
                None,
            ),
        ],
    ];

    let expectations = vec![
        Expectations::from_outputs(vec![]),
        Expectations::from_outputs(all_predicates![quorum_proposal_send(),]),
    ];

    let quorum_proposal_task_state =
        QuorumProposalTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&handle).await;

    let mut script = TaskScript {
        timeout: TIMEOUT,
        state: quorum_proposal_task_state,
        expectations,
    };
    run_test![inputs, script].await;
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_proposal_task_quorum_proposal_view_gt_1() {
    use vbs::version::StaticVersionType;

    hotshot::helpers::initialize_logging();

    let node_id = 3;
    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id).await;

    let membership = handle.hotshot.membership_coordinator.clone();
    let epoch_1_mem = membership
        .membership_for_epoch(Some(EpochNumber::new(1)))
        .await
        .unwrap();
    let num_storage_node = epoch_1_mem.total_nodes().await;

    let mut generator =
        TestViewGenerator::<TestVersions>::generate(membership.clone(), node_key_map);

    let mut proposals = Vec::new();
    let mut leaders = Vec::new();
    let mut leaves = Vec::new();
    let mut vids = Vec::new();
    let mut vid_dispersals = Vec::new();
    let consensus = handle.hotshot.consensus();
    let mut consensus_writer = consensus.write().await;
    for view in (&mut generator).take(5).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaders.push(view.leader_public_key);
        leaves.push(view.leaf.clone());
        vids.push(view.vid_proposal.clone());
        vid_dispersals.push(view.vid_disperse.clone());

        // We don't have a `QuorumProposalRecv` task handler, so we'll just manually insert the proposals
        // to make sure they show up during tests.
        consensus_writer
            .update_leaf(
                Leaf2::from_quorum_proposal(&view.quorum_proposal.data),
                Arc::new(TestValidatedState::default()),
                None,
            )
            .unwrap();
    }

    // We need to handle the views where we aren't the leader to ensure that the states are
    // updated properly.
    let genesis_cert = proposals[0].data.justify_qc().clone();

    drop(consensus_writer);

    let builder_commitment = BuilderCommitment::from_raw_digest(sha2::Sha256::new().finalize());
    let builder_fee = null_block::builder_fee::<TestTypes, TestVersions>(
        num_storage_node,
        <TestVersions as Versions>::Base::VERSION,
        *ViewNumber::new(1),
    )
    .unwrap();

    let upgrade_lock = &handle.hotshot.upgrade_lock;
    let version_1 = upgrade_lock.version_infallible(ViewNumber::new(1)).await;
    let version_2 = upgrade_lock.version_infallible(ViewNumber::new(2)).await;
    let version_3 = upgrade_lock.version_infallible(ViewNumber::new(3)).await;
    let version_4 = upgrade_lock.version_infallible(ViewNumber::new(4)).await;
    let version_5 = upgrade_lock.version_infallible(ViewNumber::new(5)).await;

    let inputs = vec![
        random![
            Qc2Formed(either::Left(genesis_cert.clone())),
            SendPayloadCommitmentAndMetadata(
                build_payload_commitment::<TestTypes, TestVersions>(
                    &epoch_1_mem,
                    ViewNumber::new(1),
                    version_1,
                )
                .await,
                builder_commitment.clone(),
                TestMetadata {
                    num_transactions: 0
                },
                ViewNumber::new(1),
                vec1![builder_fee.clone()],
                None,
            ),
            VidDisperseSend(vid_dispersals[0].clone(), handle.public_key()),
        ],
        random![
            QuorumProposalPreliminarilyValidated(proposals[0].clone()),
            Qc2Formed(either::Left(proposals[1].data.justify_qc().clone())),
            SendPayloadCommitmentAndMetadata(
                build_payload_commitment::<TestTypes, TestVersions>(
                    &epoch_1_mem,
                    ViewNumber::new(2),
                    version_2,
                )
                .await,
                builder_commitment.clone(),
                proposals[0].data.block_header().metadata,
                ViewNumber::new(2),
                vec1![builder_fee.clone()],
                None,
            ),
            VidDisperseSend(vid_dispersals[1].clone(), handle.public_key()),
        ],
        random![
            QuorumProposalPreliminarilyValidated(proposals[1].clone()),
            Qc2Formed(either::Left(proposals[2].data.justify_qc().clone())),
            SendPayloadCommitmentAndMetadata(
                build_payload_commitment::<TestTypes, TestVersions>(
                    &epoch_1_mem,
                    ViewNumber::new(3),
                    version_3,
                )
                .await,
                builder_commitment.clone(),
                proposals[1].data.block_header().metadata,
                ViewNumber::new(3),
                vec1![builder_fee.clone()],
                None,
            ),
            VidDisperseSend(vid_dispersals[2].clone(), handle.public_key()),
        ],
        random![
            QuorumProposalPreliminarilyValidated(proposals[2].clone()),
            Qc2Formed(either::Left(proposals[3].data.justify_qc().clone())),
            SendPayloadCommitmentAndMetadata(
                build_payload_commitment::<TestTypes, TestVersions>(
                    &epoch_1_mem,
                    ViewNumber::new(4),
                    version_4,
                )
                .await,
                builder_commitment.clone(),
                proposals[2].data.block_header().metadata,
                ViewNumber::new(4),
                vec1![builder_fee.clone()],
                None,
            ),
            VidDisperseSend(vid_dispersals[3].clone(), handle.public_key()),
        ],
        random![
            QuorumProposalPreliminarilyValidated(proposals[3].clone()),
            Qc2Formed(either::Left(proposals[4].data.justify_qc().clone())),
            SendPayloadCommitmentAndMetadata(
                build_payload_commitment::<TestTypes, TestVersions>(
                    &epoch_1_mem,
                    ViewNumber::new(5),
                    version_5,
                )
                .await,
                builder_commitment,
                proposals[3].data.block_header().metadata,
                ViewNumber::new(5),
                vec1![builder_fee.clone()],
                None,
            ),
            VidDisperseSend(vid_dispersals[4].clone(), handle.public_key()),
        ],
    ];

    let expectations = vec![
        Expectations::from_outputs(vec![]),
        Expectations::from_outputs(vec![]),
        Expectations::from_outputs(all_predicates![quorum_proposal_send(),]),
        Expectations::from_outputs(vec![]),
        Expectations::from_outputs(vec![]),
    ];

    let quorum_proposal_task_state =
        QuorumProposalTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&handle).await;

    let mut script = TaskScript {
        timeout: TIMEOUT,
        state: quorum_proposal_task_state,
        expectations,
    };

    run_test![inputs, script].await;
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_proposal_task_qc_timeout() {
    use vbs::version::StaticVersionType;

    hotshot::helpers::initialize_logging();

    let node_id = 3;

    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id).await;
    let membership = handle.hotshot.membership_coordinator.clone();
    let epoch_1_mem = membership
        .membership_for_epoch(Some(EpochNumber::new(1)))
        .await
        .unwrap();
    let version = handle
        .hotshot
        .upgrade_lock
        .version_infallible(ViewNumber::new(node_id))
        .await;

    let payload_commitment = build_payload_commitment::<TestTypes, TestVersions>(
        &epoch_1_mem,
        ViewNumber::new(node_id),
        version,
    )
    .await;
    let builder_commitment = BuilderCommitment::from_raw_digest(sha2::Sha256::new().finalize());

    let mut generator =
        TestViewGenerator::<TestVersions>::generate(membership.clone(), node_key_map);

    let mut proposals = Vec::new();
    let mut leaders = Vec::new();
    let mut vids = Vec::new();
    let mut vid_dispersals = Vec::new();
    let mut leaves = Vec::new();
    for view in (&mut generator).take(1).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaders.push(view.leader_public_key);
        vids.push(view.vid_proposal.clone());
        vid_dispersals.push(view.vid_disperse.clone());
        leaves.push(view.leaf.clone());
    }
    let timeout_data = TimeoutData2 {
        view: ViewNumber::new(1),
        epoch: None,
    };
    generator.add_timeout(timeout_data);
    for view in (&mut generator).take(2).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaders.push(view.leader_public_key);
        vids.push(view.vid_proposal.clone());
        vid_dispersals.push(view.vid_disperse.clone());
        leaves.push(view.leaf.clone());
    }

    // Get the proposal cert out for the view sync input
    let cert = match proposals[1].data.view_change_evidence().clone().unwrap() {
        ViewChangeEvidence2::Timeout(tc) => tc,
        _ => panic!("Found a View Sync Cert when there should have been a Timeout cert"),
    };

    let num_storage_nodes = epoch_1_mem.total_nodes().await;
    let inputs = vec![random![
        Qc2Formed(either::Right(cert.clone())),
        SendPayloadCommitmentAndMetadata(
            payload_commitment,
            builder_commitment,
            TestMetadata {
                num_transactions: 0
            },
            ViewNumber::new(3),
            vec1![null_block::builder_fee::<TestTypes, TestVersions>(
                num_storage_nodes,
                <TestVersions as Versions>::Base::VERSION,
                *ViewNumber::new(3),
            )
            .unwrap()],
            None,
        ),
        VidDisperseSend(vid_dispersals[2].clone(), handle.public_key()),
    ]];

    let expectations = vec![Expectations::from_outputs(vec![quorum_proposal_send()])];

    let quorum_proposal_task_state =
        QuorumProposalTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&handle).await;

    let mut script = TaskScript {
        timeout: TIMEOUT,
        state: quorum_proposal_task_state,
        expectations,
    };
    run_test![inputs, script].await;
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_proposal_task_view_sync() {
    use hotshot_example_types::block_types::TestMetadata;
    use hotshot_types::data::null_block;
    use vbs::version::StaticVersionType;

    hotshot::helpers::initialize_logging();

    let node_id = 2;
    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id).await;

    let membership = handle.hotshot.membership_coordinator.clone();
    let epoch_1_mem = membership
        .membership_for_epoch(Some(EpochNumber::new(1)))
        .await
        .unwrap();
    let version = handle
        .hotshot
        .upgrade_lock
        .version_infallible(ViewNumber::new(node_id))
        .await;

    let payload_commitment = build_payload_commitment::<TestTypes, TestVersions>(
        &epoch_1_mem,
        ViewNumber::new(node_id),
        version,
    )
    .await;
    let builder_commitment = BuilderCommitment::from_raw_digest(sha2::Sha256::new().finalize());

    let mut generator =
        TestViewGenerator::<TestVersions>::generate(membership.clone(), node_key_map);

    let mut proposals = Vec::new();
    let mut leaders = Vec::new();
    let mut vids = Vec::new();
    let mut vid_dispersals = Vec::new();
    let mut leaves = Vec::new();
    for view in (&mut generator).take(1).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaders.push(view.leader_public_key);
        vids.push(view.vid_proposal.clone());
        vid_dispersals.push(view.vid_disperse.clone());
        leaves.push(view.leaf.clone());
    }

    let view_sync_finalize_data = ViewSyncFinalizeData2 {
        relay: 2,
        round: ViewNumber::new(node_id),
        epoch: None,
    };
    generator.add_view_sync_finalize(view_sync_finalize_data);
    for view in (&mut generator).take(2).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaders.push(view.leader_public_key);
        vids.push(view.vid_proposal.clone());
        vid_dispersals.push(view.vid_disperse.clone());
        leaves.push(view.leaf.clone());
    }

    // Get the proposal cert out for the view sync input
    let cert = match proposals[1].data.view_change_evidence().clone().unwrap() {
        ViewChangeEvidence2::ViewSync(vsc) => vsc,
        _ => panic!("Found a TC when there should have been a view sync cert"),
    };

    let num_storage_nodes = epoch_1_mem.total_nodes().await;
    let inputs = vec![random![
        ViewSyncFinalizeCertificateRecv(cert.clone()),
        SendPayloadCommitmentAndMetadata(
            payload_commitment,
            builder_commitment,
            TestMetadata {
                num_transactions: 0
            },
            ViewNumber::new(2),
            vec1![null_block::builder_fee::<TestTypes, TestVersions>(
                num_storage_nodes,
                <TestVersions as Versions>::Base::VERSION,
                *ViewNumber::new(2),
            )
            .unwrap()],
            None,
        ),
        VidDisperseSend(vid_dispersals[1].clone(), handle.public_key()),
    ]];

    let expectations = vec![Expectations::from_outputs(vec![quorum_proposal_send()])];

    let quorum_proposal_task_state =
        QuorumProposalTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&handle).await;

    let mut script = TaskScript {
        timeout: TIMEOUT,
        state: quorum_proposal_task_state,
        expectations,
    };
    run_test![inputs, script].await;
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_proposal_task_liveness_check() {
    use vbs::version::StaticVersionType;

    hotshot::helpers::initialize_logging();

    let node_id = 3;
    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id).await;

    let membership = handle.hotshot.membership_coordinator.clone();
    let epoch_1_mem = membership
        .membership_for_epoch(Some(EpochNumber::new(1)))
        .await
        .unwrap();

    let mut generator =
        TestViewGenerator::<TestVersions>::generate(membership.clone(), node_key_map);

    let mut proposals = Vec::new();
    let mut leaders = Vec::new();
    let mut leaves = Vec::new();
    let mut vids = Vec::new();
    let mut vid_dispersals = Vec::new();
    let consensus = handle.hotshot.consensus();
    let mut consensus_writer = consensus.write().await;
    for view in (&mut generator).take(5).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaders.push(view.leader_public_key);
        leaves.push(view.leaf.clone());
        vids.push(view.vid_proposal.clone());
        vid_dispersals.push(view.vid_disperse.clone());

        // We don't have a `QuorumProposalRecv` task handler, so we'll just manually insert the proposals
        // to make sure they show up during tests.
        consensus_writer
            .update_leaf(
                Leaf2::from_quorum_proposal(&view.quorum_proposal.data),
                Arc::new(TestValidatedState::default()),
                None,
            )
            .unwrap();
    }
    drop(consensus_writer);

    let num_storage_nodes = epoch_1_mem.total_nodes().await;
    let builder_commitment = BuilderCommitment::from_raw_digest(sha2::Sha256::new().finalize());
    let builder_fee = null_block::builder_fee::<TestTypes, TestVersions>(
        num_storage_nodes,
        <TestVersions as Versions>::Base::VERSION,
        *ViewNumber::new(1),
    )
    .unwrap();

    // We need to handle the views where we aren't the leader to ensure that the states are
    // updated properly.
    let genesis_cert = proposals[0].data.justify_qc().clone();

    let upgrade_lock = &handle.hotshot.upgrade_lock;
    let version_1 = upgrade_lock.version_infallible(ViewNumber::new(1)).await;
    let version_2 = upgrade_lock.version_infallible(ViewNumber::new(2)).await;
    let version_3 = upgrade_lock.version_infallible(ViewNumber::new(3)).await;
    let version_4 = upgrade_lock.version_infallible(ViewNumber::new(4)).await;
    let version_5 = upgrade_lock.version_infallible(ViewNumber::new(5)).await;

    let inputs = vec![
        random![
            Qc2Formed(either::Left(genesis_cert.clone())),
            SendPayloadCommitmentAndMetadata(
                build_payload_commitment::<TestTypes, TestVersions>(
                    &epoch_1_mem,
                    ViewNumber::new(1),
                    version_1,
                )
                .await,
                builder_commitment.clone(),
                TestMetadata {
                    num_transactions: 0
                },
                ViewNumber::new(1),
                vec1![builder_fee.clone()],
                None,
            ),
            VidDisperseSend(vid_dispersals[0].clone(), handle.public_key()),
        ],
        random![
            QuorumProposalPreliminarilyValidated(proposals[0].clone()),
            Qc2Formed(either::Left(proposals[1].data.justify_qc().clone())),
            SendPayloadCommitmentAndMetadata(
                build_payload_commitment::<TestTypes, TestVersions>(
                    &epoch_1_mem,
                    ViewNumber::new(2),
                    version_2,
                )
                .await,
                builder_commitment.clone(),
                proposals[0].data.block_header().metadata,
                ViewNumber::new(2),
                vec1![builder_fee.clone()],
                None,
            ),
            VidDisperseSend(vid_dispersals[1].clone(), handle.public_key()),
        ],
        random![
            QuorumProposalPreliminarilyValidated(proposals[1].clone()),
            Qc2Formed(either::Left(proposals[2].data.justify_qc().clone())),
            SendPayloadCommitmentAndMetadata(
                build_payload_commitment::<TestTypes, TestVersions>(
                    &epoch_1_mem,
                    ViewNumber::new(3),
                    version_3,
                )
                .await,
                builder_commitment.clone(),
                proposals[1].data.block_header().metadata,
                ViewNumber::new(3),
                vec1![builder_fee.clone()],
                None,
            ),
            VidDisperseSend(vid_dispersals[2].clone(), handle.public_key()),
        ],
        random![
            QuorumProposalPreliminarilyValidated(proposals[2].clone()),
            Qc2Formed(either::Left(proposals[3].data.justify_qc().clone())),
            SendPayloadCommitmentAndMetadata(
                build_payload_commitment::<TestTypes, TestVersions>(
                    &epoch_1_mem,
                    ViewNumber::new(4),
                    version_4,
                )
                .await,
                builder_commitment.clone(),
                proposals[2].data.block_header().metadata,
                ViewNumber::new(4),
                vec1![builder_fee.clone()],
                None,
            ),
            VidDisperseSend(vid_dispersals[3].clone(), handle.public_key()),
        ],
        random![
            QuorumProposalPreliminarilyValidated(proposals[3].clone()),
            Qc2Formed(either::Left(proposals[4].data.justify_qc().clone())),
            SendPayloadCommitmentAndMetadata(
                build_payload_commitment::<TestTypes, TestVersions>(
                    &epoch_1_mem,
                    ViewNumber::new(5),
                    version_5,
                )
                .await,
                builder_commitment,
                proposals[3].data.block_header().metadata,
                ViewNumber::new(5),
                vec1![builder_fee.clone()],
                None,
            ),
            VidDisperseSend(vid_dispersals[4].clone(), handle.public_key()),
        ],
    ];

    let expectations = vec![
        Expectations::from_outputs(vec![]),
        Expectations::from_outputs(vec![]),
        Expectations::from_outputs(all_predicates![quorum_proposal_send(),]),
        Expectations::from_outputs(vec![]),
        Expectations::from_outputs(vec![]),
    ];

    let quorum_proposal_task_state =
        QuorumProposalTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&handle).await;

    let mut script = TaskScript {
        timeout: TIMEOUT,
        state: quorum_proposal_task_state,
        expectations,
    };
    run_test![inputs, script].await;
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_proposal_task_with_incomplete_events() {
    hotshot::helpers::initialize_logging();

    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(2).await;
    let membership = handle.hotshot.membership_coordinator.clone();
    let mut generator = TestViewGenerator::<TestVersions>::generate(membership, node_key_map);

    let mut proposals = Vec::new();
    let mut leaders = Vec::new();
    let mut leaves = Vec::new();
    for view in (&mut generator).take(2).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaders.push(view.leader_public_key);
        leaves.push(view.leaf.clone());
    }

    // We run the task here at view 2, but this time we ignore the crucial piece of evidence: the
    // payload commitment and metadata. Instead we send only one of the three "OR" required fields.
    // This should result in the proposal failing to be sent.
    let inputs = vec![serial![QuorumProposalRecv(
        proposals[1].clone(),
        leaders[1]
    )]];

    let expectations = vec![Expectations::from_outputs(vec![])];

    let quorum_proposal_task_state =
        QuorumProposalTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&handle).await;

    let mut script = TaskScript {
        timeout: TIMEOUT,
        state: quorum_proposal_task_state,
        expectations,
    };
    run_test![inputs, script].await;
}
