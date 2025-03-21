// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::{
    node_types::{
        CombinedImpl, EpochUpgradeTestVersions, EpochsTestVersions, Libp2pImpl, MemoryImpl,
        PushCdnImpl, RandomOverlapQuorumFilterConfig, StableQuorumFilterConfig,
        TestConsecutiveLeaderTypes, TestTwoStakeTablesTypes, TestTypes, TestTypesEpochCatchupTypes,
        TestTypesRandomizedCommitteeMembers, TestTypesRandomizedLeader,
    },
    testable_delay::{DelayConfig, DelayOptions, DelaySettings, SupportedTraitTypesForAsyncDelay},
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::{TestDescription, TimingData},
    view_sync_task::ViewSyncTaskDescription,
};

cross_tests!(
    TestName: test_success_with_epochs,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestTypes, TestTypesRandomizedLeader, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            ..TestDescription::default()
        };

        metadata.test_config.epoch_height = 10;

        metadata
    },
);

cross_tests!(
    TestName: test_epoch_success,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [
        TestTypes,
        TestTypesEpochCatchupTypes,
        TestTypesRandomizedLeader,
        TestTypesRandomizedCommitteeMembers<StableQuorumFilterConfig<123, 2>>,                 // Overlap =  F
        TestTypesRandomizedCommitteeMembers<StableQuorumFilterConfig<123, 3>>,                 // Overlap =  F+1
        TestTypesRandomizedCommitteeMembers<StableQuorumFilterConfig<123, 4>>,                 // Overlap = 2F
        TestTypesRandomizedCommitteeMembers<StableQuorumFilterConfig<123, 5>>,                 // Overlap = 2F+1
        TestTypesRandomizedCommitteeMembers<StableQuorumFilterConfig<123, 6>>,                 // Overlap = 3F
        TestTypesRandomizedCommitteeMembers<RandomOverlapQuorumFilterConfig<123, 4, 7, 0, 2>>, // Overlap = Dynamic
    ],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            ..TestDescription::default().set_num_nodes(14, 14)
        }
    },
);

cross_tests!(
    TestName: test_success_with_async_delay_with_epochs,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            ..TestDescription::default()
        };

        metadata.test_config.epoch_height = 10;
        metadata.overall_safety_properties.num_successful_views = 50;
        let mut config = DelayConfig::default();
        let delay_settings = DelaySettings {
            delay_option: DelayOptions::Random,
            min_time_in_milliseconds: 10,
            max_time_in_milliseconds: 100,
            fixed_time_in_milliseconds: 0,
        };
        config.add_settings_for_all_types(delay_settings);
        metadata.async_delay_config = config;
        metadata
    },
);

cross_tests!(
    TestName: test_success_with_async_delay_2_with_epochs,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            ..TestDescription::default()
        };

        metadata.test_config.epoch_height = 10;
        metadata.overall_safety_properties.num_successful_views = 30;
        let mut config = DelayConfig::default();
        let mut delay_settings = DelaySettings {
            delay_option: DelayOptions::Random,
            min_time_in_milliseconds: 10,
            max_time_in_milliseconds: 100,
            fixed_time_in_milliseconds: 15,
        };
        config.add_setting(SupportedTraitTypesForAsyncDelay::Storage, &delay_settings);

        delay_settings.delay_option = DelayOptions::Fixed;
        config.add_setting(SupportedTraitTypesForAsyncDelay::BlockHeader, &delay_settings);

        delay_settings.delay_option = DelayOptions::Random;
        delay_settings.min_time_in_milliseconds = 5;
        delay_settings.max_time_in_milliseconds = 20;
        config.add_setting(SupportedTraitTypesForAsyncDelay::ValidatedState, &delay_settings);
        metadata.async_delay_config = config;
        metadata
    },
);

cross_tests!(
    TestName: test_with_double_leader_no_failures_with_epochs,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestConsecutiveLeaderTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default_more_nodes().set_num_nodes(12,12);
        metadata.test_config.num_bootstrap = 10;
        metadata.test_config.epoch_height = 10;

        metadata.view_sync_properties = ViewSyncTaskDescription::Threshold(0, 0);

        metadata
    }
);

cross_tests!(
    TestName: test_epoch_end,
    Impls: [CombinedImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_millis(100000),
                },
            ),
            ..TestDescription::default()
        }.set_num_nodes(11,11);

        metadata.test_config.epoch_height = 10;

        metadata
    },
);

// Test to make sure we can decide in just 3 views
// This test fails with the old decide rule
cross_tests!(
    TestName: test_shorter_decide,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                TimeBasedCompletionTaskDescription {
                    duration: Duration::from_millis(100000),
                },
            ),
            ..TestDescription::default()
        };
        // after the first 3 leaders the next leader is down. It's a hack to make sure we decide in
        // 3 views or else we get a timeout
        let dead_nodes = vec![
            ChangeNode {
                idx: 4,
                updown: NodeAction::Down,
            },

        ];
        metadata.test_config.epoch_height = 10;
        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(1, dead_nodes)]
        };
        metadata.overall_safety_properties.num_successful_views = 1;
        metadata
    },
);

cross_tests!(
    TestName: test_epoch_upgrade,
    Impls: [MemoryImpl],
    Types: [TestTypes, TestTypesRandomizedLeader],
    // TODO: we need some test infrastructure + Membership trait fixes to get this to work with:
    // Types: [TestTypes, TestTypesRandomizedLeader, TestTwoStakeTablesTypes],
    Versions: [EpochUpgradeTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(120),
                                             },
                                         ),
            upgrade_view: Some(5),
            ..TestDescription::default()
        };

        // Keep going until the 2nd epoch transition
        metadata.overall_safety_properties.num_successful_views = 110;
        metadata.test_config.epoch_height = 50;

        metadata
    },
);

cross_tests!(
    TestName: test_with_failures_2_with_epochs,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default_more_nodes().set_num_nodes(12,12);
        metadata.test_config.epoch_height = 10;
        let dead_nodes = vec![
            ChangeNode {
                idx: 10,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 11,
                updown: NodeAction::Down,
            },
        ];

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(5, dead_nodes)]
        };

        // Make sure we keep committing rounds after the bad leaders, but not the full 50 because of the numerous timeouts
        metadata.overall_safety_properties.num_successful_views = 20;
        metadata.overall_safety_properties.expected_view_failures = vec![4, 5, 10, 11, 17, 22, 23, 28, 29, 34, 35];
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(20);

        metadata
    }
);

cross_tests!(
    TestName: test_with_double_leader_failures_with_epochs,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestConsecutiveLeaderTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default_more_nodes().set_num_nodes(12,12);
        let dead_nodes = vec![
            ChangeNode {
                idx: 5,
                updown: NodeAction::Down,
            },
        ];

        // shutdown while node 5 is leader
        // we want to trigger `ViewSyncTrigger` during epoch transition
        // then ensure we do not fail again as next leader will be leader 2 views also
        let view_spin_node_down = 9;
        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(view_spin_node_down, dead_nodes)]
        };

        // node 5 is leader twice when we shut down
        metadata.overall_safety_properties.expected_view_failures = vec![
            view_spin_node_down,
            view_spin_node_down + 1,
            view_spin_node_down + 2
        ];
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(20);
        // Make sure we keep committing rounds after the bad leaders, but not the full 50 because of the numerous timeouts
        metadata.overall_safety_properties.num_successful_views = 13;

        // only turning off 1 node, so expected should be num_nodes_with_stake - 1
        let expected_nodes_in_view_sync = 11;
        metadata.view_sync_properties = ViewSyncTaskDescription::Threshold(expected_nodes_in_view_sync, expected_nodes_in_view_sync);

        metadata
    }
);

cross_tests!(
    TestName: test_with_failures_half_f_epochs_1,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default_more_nodes();
        let dead_nodes = vec![
            ChangeNode {
                idx: 17,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 18,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 19,
                updown: NodeAction::Down,
            },
        ];

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(5, dead_nodes)]
        };

        metadata.overall_safety_properties.expected_view_failures = vec![16, 17, 18, 19];
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(24);
        // Make sure we keep committing rounds after the bad leaders, but not the full 50 because of the numerous timeouts
        metadata.overall_safety_properties.num_successful_views = 19;
        metadata
    }
);

cross_tests!(
    TestName: test_with_failures_half_f_epochs_2,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default_more_nodes();
        metadata.test_config.epoch_height = 10;
        // The first 14 (i.e., 20 - f) nodes are in the DA committee and we may shutdown the
        // remaining 6 (i.e., f) nodes. We could remove this restriction after fixing the
        // following issue.
        let dead_nodes = vec![
            ChangeNode {
                idx: 17,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 18,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 19,
                updown: NodeAction::Down,
            },
        ];

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(5, dead_nodes)]
        };

        metadata.overall_safety_properties.expected_view_failures = vec![7, 8, 9, 18, 19];
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(20);
        // Make sure we keep committing rounds after the bad leaders, but not the full 50 because of the numerous timeouts
        metadata.overall_safety_properties.num_successful_views = 19;
        metadata
    }
);

cross_tests!(
    TestName: test_with_failures_f_epochs_1,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default_more_nodes();
        metadata.overall_safety_properties.expected_view_failures = vec![13, 14, 15, 16, 17, 18, 19];
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(60);
        // Make sure we keep committing rounds after the bad leaders, but not the full 50 because of the numerous timeouts
        metadata.overall_safety_properties.num_successful_views = 15;
        let dead_nodes = vec![
            ChangeNode {
                idx: 14,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 15,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 16,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 17,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 18,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 19,
                updown: NodeAction::Down,
            },
        ];

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(5, dead_nodes)]
        };

        metadata
    }
);

cross_tests!(
    TestName: test_with_failures_f_epochs_2,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default_more_nodes();
        metadata.overall_safety_properties.expected_view_failures = vec![6, 7, 8, 9, 17, 18, 19];
        metadata.overall_safety_properties.decide_timeout = Duration::from_secs(60);
        // Make sure we keep committing rounds after the bad leaders, but not the full 50 because of the numerous timeouts
        metadata.overall_safety_properties.num_successful_views = 15;
        let dead_nodes = vec![
            ChangeNode {
                idx: 14,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 15,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 16,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 17,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 18,
                updown: NodeAction::Down,
            },
            ChangeNode {
                idx: 19,
                updown: NodeAction::Down,
            },
        ];

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(5, dead_nodes)]
        };

        metadata
    }
);

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
          possible_view_failures: vec![9, 11, 12],
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
          possible_view_failures: vec![9, 11, 12],
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
              updown: NodeAction::RestartDown(20),
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
          node_changes: vec![(10, down_da_nodes), (30, down_regular_nodes)],
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
          expected_view_failures: vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33],
          possible_view_failures: vec![10, 34],
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
              updown: NodeAction::RestartDown(20),
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
          node_changes: vec![(10, down_da_nodes), (30, down_regular_nodes)],
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
          expected_view_failures: vec![11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31],
          possible_view_failures: vec![10, 32],
          decide_timeout: Duration::from_secs(120),
          ..Default::default()
      };

      metadata
    },
);

// A run where the CDN crashes part-way through, epochs enabled.
cross_tests!(
    TestName: test_combined_network_cdn_crash_with_epochs,
    Impls: [CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let timing_data = TimingData {
            next_view_timeout: 10_000,
            ..Default::default()
        };

        let overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 35,
            decide_timeout: Duration::from_secs(8),
            ..Default::default()
        };

        let completion_task_description = CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(220),
            },
        );

        let mut metadata = TestDescription::default_multiple_rounds();
        metadata.timing_data = timing_data;
        metadata.overall_safety_properties = overall_safety_properties;
        metadata.completion_task_description = completion_task_description;

        let mut all_nodes = vec![];
        for node in 0..metadata.test_config.num_nodes_with_stake.into() {
            all_nodes.push(ChangeNode {
                idx: node,
                updown: NodeAction::NetworkDown,
            });
        }

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(5, all_nodes)],
        };

        metadata
    },
);

// A run where the CDN crashes partway through
// and then comes back up
cross_tests!(
    TestName: test_combined_network_reup_with_epochs,
    Impls: [CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let timing_data = TimingData {
            next_view_timeout: 10_000,
            ..Default::default()
        };

        let overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 35,
            decide_timeout: Duration::from_secs(10),
            ..Default::default()
        };

        let completion_task_description = CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(220),
            },
        );

        let mut metadata = TestDescription::default_multiple_rounds();
        metadata.timing_data = timing_data;
        metadata.overall_safety_properties = overall_safety_properties;
        metadata.completion_task_description = completion_task_description;

        let mut all_down = vec![];
        let mut all_up = vec![];
        for node in 0..metadata.test_config.num_nodes_with_stake.into() {
            all_down.push(ChangeNode {
                idx: node,
                updown: NodeAction::NetworkDown,
            });
            all_up.push(ChangeNode {
                idx: node,
                updown: NodeAction::NetworkUp,
            });
        }

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(13, all_up), (5, all_down)],
        };

        metadata
    },
);

// A run where half of the nodes disconnect from the CDN
cross_tests!(
    TestName: test_combined_network_half_dc_with_epochs,
    Impls: [CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let timing_data = TimingData {
            next_view_timeout: 10_000,
            ..Default::default()
        };

        let overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 35,
            decide_timeout: Duration::from_secs(10),
            ..Default::default()
        };

        let completion_task_description = CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
            TimeBasedCompletionTaskDescription {
                duration: Duration::from_secs(220),
            },
        );

        let mut metadata = TestDescription::default_multiple_rounds();
        metadata.timing_data = timing_data;
        metadata.overall_safety_properties = overall_safety_properties;
        metadata.completion_task_description = completion_task_description;

        let mut half = vec![];
        for node in 0..usize::from(metadata.test_config.num_nodes_with_stake) / 2 {
            half.push(ChangeNode {
                idx: node,
                updown: NodeAction::NetworkDown,
            });
        }

        metadata.spinning_properties = SpinningTaskDescription {
            node_changes: vec![(5, half)],
        };

        metadata
    },
);
