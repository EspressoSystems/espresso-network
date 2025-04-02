// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{
    CombinedImpl, EpochsTestVersions, Libp2pImpl, MemoryImpl, PushCdnImpl,
    TestConsecutiveLeaderTypes, TestTwoStakeTablesTypes, TestTypes,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::TestDescription,
    view_sync_task::ViewSyncTaskDescription,
};

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
        metadata.overall_safety_properties.expected_view_failures = vec![5, 11, 17, 23, 29];
        metadata.overall_safety_properties.possible_view_failures = vec![4, 10, 16, 22, 28];
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
            8,
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
        metadata.overall_safety_properties.expected_view_failures = vec![7, 8, 9, 17, 18, 19];
        metadata.overall_safety_properties.possible_view_failures = vec![6, 16];
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
