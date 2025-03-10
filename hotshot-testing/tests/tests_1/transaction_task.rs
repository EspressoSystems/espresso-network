use hotshot::tasks::task_state::CreateTaskState;
use hotshot_example_types::{
    block_types::TestMetadata,
    node_types::{MemoryImpl, TestConsecutiveLeaderTypes, TestVersions},
};
use hotshot_task_impls::{
    events::HotShotEvent, harness::run_harness, transactions::TransactionTaskState,
};
use hotshot_testing::helpers::build_system_handle;
use hotshot_types::{
    data::{null_block, EpochNumber, PackedBundle, ViewNumber},
    traits::node_implementation::{ConsensusTime, Versions},
};
use vbs::version::StaticVersionType;

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_transaction_task_leader_two_views_in_a_row() {
    hotshot::helpers::initialize_logging();

    // Build the API for node 2.
    let node_id = 2;
    let handle =
        build_system_handle::<TestConsecutiveLeaderTypes, MemoryImpl, TestVersions>(node_id)
            .await
            .0;

    let mut input = Vec::new();
    let mut output = Vec::new();

    let current_view = ViewNumber::new(4);
    input.push(HotShotEvent::ViewChange(
        current_view,
        Some(EpochNumber::new(1)),
    ));
    input.push(HotShotEvent::ViewChange(
        current_view + 1,
        Some(EpochNumber::new(1)),
    ));
    input.push(HotShotEvent::Shutdown);

    // current view
    let mut exp_packed_bundle = PackedBundle::new(
        vec![].into(),
        TestMetadata {
            num_transactions: 0,
        },
        current_view,
        Some(EpochNumber::new(1)),
        vec1::vec1![
            null_block::builder_fee::<TestConsecutiveLeaderTypes, TestVersions>(
                <TestVersions as Versions>::Base::VERSION,
                *ViewNumber::new(4),
            )
            .unwrap()
        ],
        None,
    );
    output.push(HotShotEvent::BlockRecv(exp_packed_bundle.clone()));

    // next view
    exp_packed_bundle.view_number = current_view + 1;
    output.push(HotShotEvent::BlockRecv(exp_packed_bundle));

    let transaction_state =
        TransactionTaskState::<TestConsecutiveLeaderTypes, MemoryImpl, TestVersions>::create_from(
            &handle,
        )
        .await;
    run_harness(input, output, transaction_state, false).await;
}
