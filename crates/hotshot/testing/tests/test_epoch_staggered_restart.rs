// Copyright (c) 2021-2025 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{
    CombinedImpl, EpochsTestVersions, TestTwoStakeTablesTypes, TestTypes,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::TestDescription,
};

cross_tests!(
    TestName: test_staggered_restart_transition_block,
    Impls: [CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
      let mut metadata = TestDescription::default().set_num_nodes(20,4);

      let mut down_da_nodes = vec![];
      for i in 2..4 {
          down_da_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(13),
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
          node_changes: vec![(7, down_da_nodes), (20, down_regular_nodes)],
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
          expected_view_failures: vec![8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
          possible_view_failures: vec![6, 7, 21, 22, 23, 24],
          decide_timeout: Duration::from_secs(120),
          ..Default::default()
      };

      metadata
    },
);

cross_tests!(
    TestName: test_staggered_restart_first_empty_block,
    Impls: [CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
      let mut metadata = TestDescription::default().set_num_nodes(20,4);

      let mut down_da_nodes = vec![];
      for i in 2..4 {
          down_da_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(12),
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
          node_changes: vec![(8, down_da_nodes), (20, down_regular_nodes)],
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
          expected_view_failures: vec![9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
          possible_view_failures: vec![7, 8, 21, 22, 23, 24],
          decide_timeout: Duration::from_secs(120),
          ..Default::default()
      };

      metadata
    },
);

cross_tests!(
    TestName: test_staggered_restart_first_block,
    Impls: [CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
      let mut metadata = TestDescription::default().set_num_nodes(20,4);

      let mut down_da_nodes = vec![];
      for i in 2..4 {
          down_da_nodes.push(ChangeNode {
              idx: i,
              updown: NodeAction::RestartDown(9),
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
          node_changes: vec![(11, down_da_nodes), (20, down_regular_nodes)],
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
          expected_view_failures: vec![12, 13, 14, 15, 16, 17, 18, 19, 20],
          possible_view_failures: vec![11, 21, 22, 23, 24],
          decide_timeout: Duration::from_secs(120),
          ..Default::default()
      };

      metadata
    },
);
