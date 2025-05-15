// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{
    CombinedImpl, EpochsTestVersions, Libp2pImpl, PushCdnImpl, TestTwoStakeTablesTypes, TestTypes,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    test_builder::TestDescription,
};

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
