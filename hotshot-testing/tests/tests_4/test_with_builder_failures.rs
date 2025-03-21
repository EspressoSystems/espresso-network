// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{MemoryImpl, PushCdnImpl, TestTypes, TestVersions};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    test_builder::{BuilderChange, BuilderDescription, TestDescription},
    txn_task::TxnTaskDescription,
};

// Test one node leaving the network.
cross_tests!(
    TestName: test_with_builder_failures,
    Impls: [MemoryImpl, PushCdnImpl],
    Types: [TestTypes],
    Versions: [TestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default_multiple_rounds();
        metadata.test_config.epoch_height = 0;
        // Every block should contain at least one transaction - builders are never offline
        // simultaneously
        metadata.overall_safety_properties.transaction_threshold = 1;
        // Generate a lot of transactions so that freshly restarted builders still have
        // transactions
        metadata.txn_description = TxnTaskDescription::RoundRobinTimeBased(Duration::from_millis(1));
        metadata.test_config.epoch_height = 0;

        // Two builders running as follows:
        // view 1st  2nd
        // 0    Up   Down
        // 1    Up   Up
        // 2    Down Up
        // 3    Up   Up
        // 4    Up   Down
        // 5    Up   Up
        // 6    Down Up
        // 7    Up   Up
        // ...
        //
        // We give each builder a view of uptime before making it the only available builder so that it
        // has time to initialize

        // First builder will always respond with available blocks, but will periodically fail claim calls
        let first_builder = (0..metadata.overall_safety_properties.num_successful_views as u64).filter_map(|view_num| {
            match view_num % 4 {
                2 => Some((view_num, BuilderChange::FailClaims(true))),
                3 => Some((view_num, BuilderChange::FailClaims(false))),
                _ => None,
            }
        }).collect();
        // Second builder will periodically be completely down
        #[allow(clippy::unnecessary_filter_map)] // False positive
        let second_builder = (0..metadata.overall_safety_properties.num_successful_views as u64).filter_map(|view_num| {
            match view_num % 4 {
                0 => Some((view_num, BuilderChange::Down)),
                1 => Some((view_num, BuilderChange::Up)),
                _ => None,
            }
        }).collect();

        metadata.builders = vec1::vec1![
            BuilderDescription {
               changes: first_builder,
            },
            BuilderDescription {
               changes: second_builder,
            },
        ];
        metadata
    }
);
