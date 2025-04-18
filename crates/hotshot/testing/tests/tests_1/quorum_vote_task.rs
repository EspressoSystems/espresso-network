// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

#![allow(clippy::panic)]

use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use hotshot::tasks::task_state::CreateTaskState;
use hotshot_example_types::{
    node_types::{MemoryImpl, TestTypes, TestVersions},
    state_types::TestValidatedState,
};
use hotshot_macros::{run_test, test_scripts};
use hotshot_testing::{
    all_predicates,
    helpers::vid_share,
    predicates::event::all_predicates,
    random,
    script::{Expectations, InputOrder, TaskScript},
};
use hotshot_types::{
    data::{Leaf2, ViewNumber},
    traits::node_implementation::ConsensusTime,
};

const TIMEOUT: Duration = Duration::from_millis(35);

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_vote_task_success() {
    use hotshot_task_impls::{events::HotShotEvent::*, quorum_vote::QuorumVoteTaskState};
    use hotshot_testing::{
        helpers::build_system_handle,
        predicates::event::{exact, quorum_vote_send},
        view_generator::TestViewGenerator,
    };

    hotshot::helpers::initialize_logging();

    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(2).await;

    let membership = handle.hotshot.membership_coordinator.clone();

    let mut generator = TestViewGenerator::<TestVersions>::generate(membership, node_key_map);

    let mut proposals = Vec::new();
    let mut leaves = Vec::new();
    let mut dacs = Vec::new();
    let mut vids = Vec::new();
    let mut leaders = Vec::new();
    let consensus = handle.hotshot.consensus().clone();
    let mut consensus_writer = consensus.write().await;
    for view in (&mut generator).take(2).collect::<Vec<_>>().await {
        leaders.push(view.leader_public_key);
        proposals.push(view.quorum_proposal.clone());
        leaves.push(view.leaf.clone());
        dacs.push(view.da_certificate.clone());
        vids.push(view.vid_proposal.clone());
        consensus_writer
            .update_leaf(
                Leaf2::from_quorum_proposal(&view.quorum_proposal.data),
                Arc::new(TestValidatedState::default()),
                None,
            )
            .unwrap();
    }
    drop(consensus_writer);

    // Send the quorum proposal, DAC, VID share data, and validated state, in which case a dummy
    // vote can be formed and the view number will be updated.
    let inputs = vec![random![
        QuorumProposalValidated(proposals[1].clone(), leaves[0].clone()),
        DaCertificateRecv(dacs[1].clone()),
        VidShareRecv(leaders[1], vids[1].0[0].clone()),
    ]];

    let expectations = vec![Expectations::from_outputs(all_predicates![
        exact(DaCertificateValidated(dacs[1].clone())),
        exact(VidShareValidated(vids[1].0[0].clone())),
        exact(ViewChange(ViewNumber::new(3), None)),
        quorum_vote_send(),
    ])];

    let quorum_vote_state =
        QuorumVoteTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&handle).await;

    let mut script = TaskScript {
        timeout: TIMEOUT,
        state: quorum_vote_state,
        expectations,
    };
    run_test![inputs, script].await;
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_vote_task_miss_dependency() {
    use hotshot_task_impls::{events::HotShotEvent::*, quorum_vote::QuorumVoteTaskState};
    use hotshot_testing::{
        helpers::build_system_handle, predicates::event::exact, view_generator::TestViewGenerator,
    };

    hotshot::helpers::initialize_logging();

    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(2).await;

    let membership = handle.hotshot.membership_coordinator.clone();

    let mut generator = TestViewGenerator::<TestVersions>::generate(membership, node_key_map);

    let mut proposals = Vec::new();
    let mut leaders = Vec::new();
    let mut votes = Vec::new();
    let mut dacs = Vec::new();
    let mut vids = Vec::new();
    let mut leaves = Vec::new();
    let consensus = handle.hotshot.consensus().clone();
    let mut consensus_writer = consensus.write().await;
    for view in (&mut generator).take(5).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaders.push(view.leader_public_key);
        votes.push(view.create_quorum_vote(&handle).await);
        dacs.push(view.da_certificate.clone());
        vids.push(view.vid_proposal.clone());
        leaves.push(view.leaf.clone());

        consensus_writer
            .update_leaf(
                Leaf2::from_quorum_proposal(&view.quorum_proposal.data),
                Arc::new(TestValidatedState::default()),
                None,
            )
            .unwrap();
    }
    drop(consensus_writer);

    // Send two of quorum proposal, DAC, VID share data, in which case there's no vote.
    let inputs = vec![
        random![
            QuorumProposalValidated(proposals[1].clone(), leaves[0].clone()),
            VidShareRecv(leaders[1], vid_share(&vids[1].0, handle.public_key())),
        ],
        random![
            QuorumProposalValidated(proposals[2].clone(), leaves[1].clone()),
            DaCertificateRecv(dacs[2].clone()),
        ],
        random![
            DaCertificateRecv(dacs[3].clone()),
            VidShareRecv(leaders[3], vid_share(&vids[3].0, handle.public_key())),
        ],
    ];

    let expectations = vec![
        Expectations::from_outputs(all_predicates![exact(VidShareValidated(
            vids[1].0[0].clone()
        ))]),
        Expectations::from_outputs(all_predicates![exact(DaCertificateValidated(
            dacs[2].clone()
        ))]),
        Expectations::from_outputs(all_predicates![
            exact(DaCertificateValidated(dacs[3].clone())),
            exact(VidShareValidated(vids[3].0[0].clone())),
        ]),
    ];

    let quorum_vote_state =
        QuorumVoteTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&handle).await;

    let mut script = TaskScript {
        timeout: TIMEOUT,
        state: quorum_vote_state,
        expectations,
    };
    run_test![inputs, script].await;
}

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_quorum_vote_task_incorrect_dependency() {
    use hotshot_task_impls::{events::HotShotEvent::*, quorum_vote::QuorumVoteTaskState};
    use hotshot_testing::{
        helpers::build_system_handle, predicates::event::exact, view_generator::TestViewGenerator,
    };

    hotshot::helpers::initialize_logging();

    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(2).await;

    let membership = handle.hotshot.membership_coordinator.clone();

    let mut generator = TestViewGenerator::<TestVersions>::generate(membership, node_key_map);

    let mut proposals = Vec::new();
    let mut leaves = Vec::new();
    let mut dacs = Vec::new();
    let mut vids = Vec::new();
    let mut leaders = Vec::new();
    for view in (&mut generator).take(2).collect::<Vec<_>>().await {
        leaders.push(view.leader_public_key);
        proposals.push(view.quorum_proposal.clone());
        leaves.push(view.leaf.clone());
        dacs.push(view.da_certificate.clone());
        vids.push(view.vid_proposal.clone());
    }

    // Send the correct quorum proposal and DAC, and incorrect VID share data.
    let inputs = vec![random![
        QuorumProposalValidated(proposals[1].clone(), leaves[0].clone()),
        DaCertificateRecv(dacs[1].clone()),
        VidShareRecv(leaders[0], vids[0].0[0].clone()),
    ]];

    let expectations = vec![Expectations::from_outputs(all_predicates![
        exact(DaCertificateValidated(dacs[1].clone())),
        exact(VidShareValidated(vids[0].0[0].clone())),
    ])];

    let quorum_vote_state =
        QuorumVoteTaskState::<TestTypes, MemoryImpl, TestVersions>::create_from(&handle).await;

    let mut script = TaskScript {
        timeout: TIMEOUT,
        state: quorum_vote_state,
        expectations,
    };
    run_test![inputs, script].await;
}
