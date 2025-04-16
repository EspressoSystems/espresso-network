// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{collections::HashMap, time::Duration};

use hotshot_example_types::{
    node_types::{MarketplaceTestVersions, MarketplaceUpgradeTestVersions, MemoryImpl},
    state_types::TestTypes,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    test_builder::{
        nonempty_block_limit, nonempty_block_threshold, BuilderChange, BuilderDescription,
        TestDescription,
    },
};
use vec1::vec1;

// Test marketplace with the auction solver and fallback builder down
// Requires no nonempty blocks be committed
cross_tests!(
    TestName: test_marketplace_solver_down,
    Impls: [MemoryImpl],
    Types: [TestTypes],
    Versions: [MarketplaceTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            fallback_builder:
              BuilderDescription {
                changes: HashMap::from([(0, BuilderChange::Down)])
              },
            validate_transactions: nonempty_block_limit((0,100)),
            start_solver: false,
            ..TestDescription::default()
        };

        metadata.test_config.epoch_height = 0;

        metadata
    },
);

// Test marketplace upgrade with no builder changes
// Upgrade is proposed in view 5 and completes in view 25.
// Requires 80% nonempty blocks
cross_tests!(
    TestName: test_marketplace_upgrade,
    Impls: [MemoryImpl],
    Types: [TestTypes],
    Versions: [MarketplaceUpgradeTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            upgrade_view: Some(5),
            validate_transactions: nonempty_block_threshold((40,50)),
            ..TestDescription::default()
        };

        metadata.test_config.epoch_height = 0;

        metadata
    },
);

// Test marketplace with both regular builders down but solver + fallback builder up
// Requires 80% nonempty blocks
cross_tests!(
    TestName: test_marketplace_builders_down,
    Impls: [MemoryImpl],
    Types: [TestTypes],
    Versions: [MarketplaceTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            builders: vec1![
              BuilderDescription {
                changes: HashMap::from([(0, BuilderChange::Down)])
              },
              BuilderDescription {
                changes: HashMap::from([(0, BuilderChange::Down)])
              }
            ],
            validate_transactions: nonempty_block_threshold((35,50)),
            ..TestDescription::default()
        };

        metadata.test_config.epoch_height = 0;

        metadata
    },
);

// Test marketplace with the fallback and one regular builder down
// Requires at least 80% of blocks to be nonempty
cross_tests!(
    TestName: test_marketplace_fallback_builder_down,
    Impls: [MemoryImpl],
    Types: [TestTypes],
    Versions: [MarketplaceTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(60),
                                             },
                                         ),
            builders: vec1![
              BuilderDescription {
                changes: HashMap::from([(0, BuilderChange::Down)])
              },
              BuilderDescription::default(),
            ],
            fallback_builder:
              BuilderDescription {
                changes: HashMap::from([(0, BuilderChange::Down)])
              },
            validate_transactions: nonempty_block_threshold((40,50)),
            ..TestDescription::default()
        };

        metadata.test_config.epoch_height = 0;

        metadata
    },
);
