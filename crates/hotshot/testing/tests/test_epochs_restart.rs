// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{
    CombinedImpl, EpochsTestVersions, PushCdnImpl, TestTwoStakeTablesTypes, TestTypes,
    TestTypesRandomizedLeader,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::{TestDescription, TimingData},
};

cross_tests!(
    TestName: test_all_restart_epochs,
    Impls: [CombinedImpl, PushCdnImpl],
    Types: [TestTypes, TestTypesRandomizedLeader, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
      let timing_data = TimingData {
          next_view_timeout: 5000,
          ..Default::default()
      };
      let mut metadata = TestDescription::default().set_num_nodes(20,20);
      let mut catchup_nodes = vec![];

      for i in 0..20 {
          catchup_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(0),
          })
      }

      metadata.timing_data = timing_data;

      metadata.spinning_properties = SpinningTaskDescription {
          // Restart all the nodes in view 10
          node_changes: vec![(10, catchup_nodes)],
      };
      metadata.view_sync_properties =
          hotshot_testing::view_sync_task::ViewSyncTaskDescription::Threshold(0, 20);

      metadata.completion_task_description =
          CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
              TimeBasedCompletionTaskDescription {
                  duration: Duration::from_secs(60),
              },
          );
      metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
          // Make sure we keep committing rounds after the catchup, but not the full 50.
          num_successful_views: 22,
          expected_view_failures: vec![10],
          possible_view_failures: vec![8, 9, 11, 12],
          decide_timeout: Duration::from_secs(60),
          ..Default::default()
      };

      metadata
    },
);

cross_tests!(
    TestName: test_all_restart_one_da_with_epochs,
    Impls: [CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
      let timing_data = TimingData {
          next_view_timeout: 5000,
          ..Default::default()
      };
      let mut metadata = TestDescription::default().set_num_nodes(20,2);

      let mut catchup_nodes = vec![];
      for i in 0..20 {
          catchup_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(0),
          })
      }

      metadata.timing_data = timing_data;

      metadata.spinning_properties = SpinningTaskDescription {
          // Restart all the nodes in view 10
          node_changes: vec![(10, catchup_nodes)],
      };
      metadata.view_sync_properties =
          hotshot_testing::view_sync_task::ViewSyncTaskDescription::Threshold(0, 20);

      metadata.completion_task_description =
          CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
              TimeBasedCompletionTaskDescription {
                  duration: Duration::from_secs(60),
              },
          );
      metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
          // Make sure we keep committing rounds after the catchup, but not the full 50.
          num_successful_views: 22,
          expected_view_failures: vec![10],
          possible_view_failures: vec![8, 9, 11, 12],
          decide_timeout: Duration::from_secs(60),
          ..Default::default()
      };

      metadata
    },
);

cross_tests!(
    TestName: test_staggered_restart_with_epochs_1,
    Impls: [CombinedImpl],
    Types: [TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
      let mut metadata = TestDescription::default().set_num_nodes(20,4);

      let mut down_da_nodes = vec![];
      for i in 2..4 {
          down_da_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(10),
          });
      }

      let mut down_regular_nodes = vec![];
      for i in 4..20 {
          down_regular_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(0),
          });
      }
      // restart the last da so it gets the new libp2p routing table
      for i in 0..2 {
          down_regular_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(0),
          });
      }

      metadata.spinning_properties = SpinningTaskDescription {
          node_changes: vec![(10, down_da_nodes), (20, down_regular_nodes)],
      };
      metadata.view_sync_properties =
          hotshot_testing::view_sync_task::ViewSyncTaskDescription::Threshold(0, 50);

      // Give the test some extra time because we are purposely timing out views
      metadata.completion_task_description =
          CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
              TimeBasedCompletionTaskDescription {
                  duration: Duration::from_secs(140),
              },
          );
      metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
          // Make sure we keep committing rounds after the catchup, but not the full 50.
          num_successful_views: 22,
          expected_view_failures: vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
          possible_view_failures: vec![8, 9, 10, 21, 22, 23, 24],
          decide_timeout: Duration::from_secs(120),
          ..Default::default()
      };

      metadata
    },
);

cross_tests!(
    TestName: test_staggered_restart_with_epochs_2,
    Impls: [CombinedImpl],
    Types: [TestTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
      let mut metadata = TestDescription::default().set_num_nodes(20,4);

      let mut down_da_nodes = vec![];
      for i in 2..4 {
          down_da_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(10),
          });
      }

      let mut down_regular_nodes = vec![];
      for i in 4..20 {
          down_regular_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(0),
          });
      }
      // restart the last da so it gets the new libp2p routing table
      for i in 0..2 {
          down_regular_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(0),
          });
      }

      metadata.spinning_properties = SpinningTaskDescription {
          node_changes: vec![(10, down_da_nodes), (20, down_regular_nodes)],
      };
      metadata.view_sync_properties =
          hotshot_testing::view_sync_task::ViewSyncTaskDescription::Threshold(0, 50);

      // Give the test some extra time because we are purposely timing out views
      metadata.completion_task_description =
          CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
              TimeBasedCompletionTaskDescription {
                  duration: Duration::from_secs(240),
              },
          );
      metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
          // Make sure we keep committing rounds after the catchup, but not the full 50.
          num_successful_views: 22,
          expected_view_failures: vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
          possible_view_failures: vec![8, 9, 10, 21, 22, 23, 24],
          decide_timeout: Duration::from_secs(120),
          ..Default::default()
      };

      metadata
    },
);

fn create_node_change(
    idx: usize,
    view: u64,
    view_up: u64,
    change: &mut Vec<(u64, Vec<ChangeNode>)>,
) {
    assert!(view_up >= view);
    let views_down = view_up - view;
    change.push((
        view,
        vec![ChangeNode {
            idx,
            updown: NodeAction::RestartDown(views_down),
        }],
    ));
}
cross_tests!(
    TestName: test_staggered_restart_double_restart,
    Impls: [CombinedImpl],
    Types: [TestTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
      let mut metadata = TestDescription::default().set_num_nodes(10,1);

      let mut node_changes = vec![];
      // idx, down view, up view
      create_node_change(1, 6, 7, &mut node_changes);
      create_node_change(2, 6, 7, &mut node_changes);
      create_node_change(3, 7, 7, &mut node_changes);
      create_node_change(4, 10, 10, &mut node_changes);
      create_node_change(5, 7, 7, &mut node_changes);
      create_node_change(6, 7, 7, &mut node_changes);
      // KILL 3 NODES until well after view sync
      create_node_change(7, 1, 15, &mut node_changes);
      create_node_change(8, 1, 15, &mut node_changes);
      create_node_change(9, 1, 15, &mut node_changes);


      metadata.spinning_properties = SpinningTaskDescription {
          node_changes,
      };
      metadata.view_sync_properties =
          hotshot_testing::view_sync_task::ViewSyncTaskDescription::Threshold(0, 50);

      // Give the test some extra time because we are purposely timing out views
      metadata.completion_task_description =
          CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
              TimeBasedCompletionTaskDescription {
                  duration: Duration::from_secs(240),
              },
          );
      metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
          // Make sure we keep committing rounds after the catchup, but not the full 50.
          num_successful_views: 22,
          expected_view_failures: vec![8,9],
          possible_view_failures: vec![5,6,7,10,11,12,13,14,15,16,17, 18, 19],
          decide_timeout: Duration::from_secs(60),
          ..Default::default()
      };

      metadata
    },
);
