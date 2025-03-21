// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use hotshot::tasks::task_state::CreateTaskState;
use hotshot_example_types::node_types::{MemoryImpl, TestTypes, TestVersions};
use hotshot_task_impls::{
    events::HotShotEvent, harness::run_harness, view_sync::ViewSyncTaskState,
};
use hotshot_testing::helpers::build_system_handle;
use hotshot_types::{
    data::ViewNumber, simple_vote::ViewSyncPreCommitData2,
    traits::node_implementation::ConsensusTime,
};

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_view_sync_task() {
    hotshot::helpers::initialize_logging();

    // Build the API for node 5.
    let handle = build_system_handle::<TestTypes, MemoryImpl, TestVersions>(5)
        .await
        .0;

    let vote_data = ViewSyncPreCommitData2 {
        relay: 0,
        round: <TestTypes as hotshot_types::traits::node_implementation::NodeType>::View::new(4),
        epoch: None,
    };
    let vote = hotshot_types::simple_vote::ViewSyncPreCommitVote2::<TestTypes>::create_signed_vote(
        vote_data,
        <TestTypes as hotshot_types::traits::node_implementation::NodeType>::View::new(4),
        hotshot_types::traits::consensus_api::ConsensusApi::public_key(&handle),
        hotshot_types::traits::consensus_api::ConsensusApi::private_key(&handle),
        &handle.hotshot.upgrade_lock,
    )
    .await
    .expect("Failed to create a ViewSyncPreCommitVote!");

    tracing::error!("Vote in test is {:?}", vote.clone());

    let mut input = Vec::new();
    let mut output = Vec::new();

    input.push(HotShotEvent::Timeout(ViewNumber::new(2), None));
    input.push(HotShotEvent::Timeout(ViewNumber::new(3), None));

    input.push(HotShotEvent::Shutdown);

    output.push(HotShotEvent::ViewChange(ViewNumber::new(3), None));
    output.push(HotShotEvent::ViewSyncPreCommitVoteSend(vote.clone()));

    let view_sync_state = ViewSyncTaskState::<TestTypes, TestVersions>::create_from(&handle).await;
    run_harness(input, output, view_sync_state, false).await;
}
