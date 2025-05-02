// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#![allow(unused_imports)]

use std::time::Duration;

use futures::StreamExt;
use hotshot::{tasks::task_state::CreateTaskState, types::SystemContextHandle};
use hotshot_example_types::{
    block_types::{TestMetadata, TestTransaction},
    node_types::{MemoryImpl, TestTypes, TestVersions},
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_macros::{run_test, test_scripts};
use hotshot_task_impls::{
    consensus::ConsensusTaskState, events::HotShotEvent::*,
    quorum_proposal::QuorumProposalTaskState, upgrade::UpgradeTaskState,
};
use hotshot_testing::{
    all_predicates,
    helpers::{build_fake_view_with_leaf, build_payload_commitment, vid_share},
    predicates::{event::*, upgrade_with_proposal::*},
    random,
    script::{Expectations, InputOrder, TaskScript},
    serial,
    view_generator::TestViewGenerator,
};
use hotshot_types::{
    data::{null_block, EpochNumber, Leaf2, ViewNumber},
    simple_vote::UpgradeProposalData,
    traits::{
        election::Membership,
        node_implementation::{ConsensusTime, Versions},
        ValidatedState,
    },
    utils::BuilderCommitment,
    vote::HasViewNumber,
};
use sha2::Digest;
use vbs::version::{StaticVersionType, Version};
use vec1::vec1;

const TIMEOUT: Duration = Duration::from_millis(35);

#[tokio::test(flavor = "multi_thread")]
/// Test that we correctly form and include an `UpgradeCertificate` when receiving votes.
async fn test_upgrade_task_with_proposal() {
    use std::sync::Arc;

    use hotshot_testing::helpers::build_system_handle;

    hotshot::helpers::initialize_logging();

    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(3).await;

    let other_handles = futures::future::join_all((0..=9).map(build_system_handle)).await;

    let old_version = Version { major: 0, minor: 1 };
    let new_version = Version { major: 0, minor: 2 };

    let upgrade_data: UpgradeProposalData<TestTypes> = UpgradeProposalData {
        old_version,
        new_version,
        decide_by: ViewNumber::new(4),
        new_version_hash: [0u8; 12].to_vec(),
        old_version_last_view: ViewNumber::new(5),
        new_version_first_view: ViewNumber::new(7),
    };

    let mut proposals = Vec::new();
    let mut votes = Vec::new();
    let mut dacs = Vec::new();
    let mut vid_dispersals = Vec::new();
    let mut leaders = Vec::new();
    let mut leaves = Vec::new();
    let mut views = Vec::new();
    let consensus = handle.hotshot.consensus();
    let mut consensus_writer = consensus.write().await;

    let membership = handle.hotshot.membership_coordinator.clone();
    let epoch_1_mem = membership
        .membership_for_epoch(Some(EpochNumber::new(1)))
        .await
        .unwrap();
    let num_storage_nodes = epoch_1_mem.total_nodes().await;

    let mut generator =
        TestViewGenerator::<TestVersions>::generate(membership.clone(), node_key_map);

    for view in (&mut generator).take(1).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        votes.push(view.create_quorum_vote(&handle).await);
        dacs.push(view.da_certificate.clone());
        vid_dispersals.push(view.vid_disperse.clone());
        leaders.push(view.leader_public_key);
        views.push(view.clone());
        consensus_writer
            .update_leaf(
                Leaf2::from_quorum_proposal(&view.quorum_proposal.data),
                Arc::new(TestValidatedState::default()),
                None,
            )
            .unwrap();
    }

    generator.add_upgrade(upgrade_data.clone());

    for view in generator.take(4).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        votes.push(view.create_quorum_vote(&handle).await);
        dacs.push(view.da_certificate.clone());
        vid_dispersals.push(view.vid_disperse.clone());
        leaders.push(view.leader_public_key);
        leaves.push(view.leaf.clone());
        views.push(view.clone());
        consensus_writer
            .update_leaf(
                Leaf2::from_quorum_proposal(&view.quorum_proposal.data),
                Arc::new(TestValidatedState::default()),
                None,
            )
            .unwrap();
    }
    drop(consensus_writer);

    let genesis_cert = proposals[0].data.justify_qc().clone();
    let builder_commitment = BuilderCommitment::from_raw_digest(sha2::Sha256::new().finalize());
    let builder_fee = null_block::builder_fee::<TestTypes, TestVersions>(
        num_storage_nodes,
        <TestVersions as Versions>::Base::VERSION,
        *ViewNumber::new(1),
    )
    .unwrap();

    let mut upgrade_votes = Vec::new();

    for handle in other_handles {
        upgrade_votes.push(
            views[2]
                .create_upgrade_vote(upgrade_data.clone(), &handle.0)
                .await,
        );
    }

    let proposal_state =
        QuorumProposalTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&handle).await;
    let upgrade_state = UpgradeTaskState::<TestTypes, TestVersions>::create_from(&handle).await;

    let upgrade_vote_recvs: Vec<_> = upgrade_votes.into_iter().map(UpgradeVoteRecv).collect();

    let upgrade_lock = &upgrade_state.upgrade_lock;
    let version_1 = upgrade_lock.version_infallible(ViewNumber::new(1)).await;
    let version_2 = upgrade_lock.version_infallible(ViewNumber::new(2)).await;
    let version_3 = upgrade_lock.version_infallible(ViewNumber::new(3)).await;

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
        InputOrder::Random(upgrade_vote_recvs),
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
    ];

    let mut proposal_script = TaskScript {
        timeout: TIMEOUT,
        state: proposal_state,
        expectations: vec![
            Expectations::from_outputs(vec![]),
            Expectations::from_outputs(vec![]),
            Expectations::from_outputs(vec![]),
            Expectations::from_outputs(all_predicates![
                quorum_proposal_send_with_upgrade_certificate::<TestTypes>()
            ]),
        ],
    };

    let mut upgrade_script = TaskScript {
        timeout: TIMEOUT,
        state: upgrade_state,
        expectations: vec![
            Expectations::from_outputs(vec![]),
            Expectations::from_outputs(vec![]),
            Expectations {
                output_asserts: vec![upgrade_certificate_formed::<TestTypes>()],
                task_state_asserts: vec![],
            },
            Expectations::from_outputs(vec![]),
        ],
    };

    run_test![inputs, proposal_script, upgrade_script].await;
}
