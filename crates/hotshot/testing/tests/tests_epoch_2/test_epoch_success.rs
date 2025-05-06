// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{
        EpochsTestVersions, Libp2pImpl, MemoryImpl,
        PushCdnImpl, RandomOverlapQuorumFilterConfig, StableQuorumFilterConfig, TestTypes, TestTypesEpochCatchupTypes,
        TestTypesRandomizedCommitteeMembers, TestTypesRandomizedLeader,
    };
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    test_builder::TestDescription,
};

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
