// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::HashMap;

use hotshot_example_types::{
    node_types::{
        Libp2pImpl, MemoryImpl, PushCdnImpl, TestConsecutiveLeaderTypes, TestTypes,
        TestTypesRandomizedLeader, TestVersions,
    },
    testable_delay::{DelayConfig, DelayOptions, DelaySettings, SupportedTraitTypesForAsyncDelay},
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    test_builder::TestDescription,
    view_sync_task::ViewSyncTaskDescription,
};

cross_tests!(
    TestName: test_success,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTypes, TestTypesRandomizedLeader],
    Versions: [TestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default();

        metadata.test_config.epoch_height = 0;

        metadata
    },
);

cross_tests!(
    TestName: test_success_with_async_delay,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTypes],
    Versions: [TestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default();
        metadata.test_config.epoch_height = 0;

        metadata.overall_safety_properties.num_successful_views = 0;
        let mut config = DelayConfig::default();
        let delay_settings = DelaySettings {
            delay_option: DelayOptions::Random,
            min_time_in_milliseconds: 10,
            max_time_in_milliseconds: 100,
            fixed_time_in_milliseconds: 0,
        };
        config.add_settings_for_all_types(delay_settings);
        let mut config_map = HashMap::new();
        for i in 0..10 {
            config_map.insert(i as u64, config.clone());
        }
        metadata.async_delay_config = config_map;
        metadata
    },
);

cross_tests!(
    TestName: test_success_with_async_delay_2,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestTypes],
    Versions: [TestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default();
        metadata.test_config.epoch_height = 0;

        metadata.overall_safety_properties.num_successful_views = 10;
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
        let mut config_map = HashMap::new();
        for i in 0..10 {
            config_map.insert(i as u64, config.clone());
        }
        metadata.async_delay_config = config_map;
        metadata
    },
);

cross_tests!(
    TestName: test_with_double_leader_no_failures,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [TestConsecutiveLeaderTypes],
    Versions: [TestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default_more_nodes().set_num_nodes(12,12);
        metadata.test_config.epoch_height = 0;
        metadata.test_config.num_bootstrap = 10;

        metadata.view_sync_properties = ViewSyncTaskDescription::Threshold(0, 0);

        metadata
    }
);
