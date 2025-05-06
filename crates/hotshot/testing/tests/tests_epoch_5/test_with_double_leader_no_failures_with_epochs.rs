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
    spinning_task::{ChangeNode, NodeAction, SpinningTaskDescription},
    test_builder::TestDescription,
    view_sync_task::ViewSyncTaskDescription,
};
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
