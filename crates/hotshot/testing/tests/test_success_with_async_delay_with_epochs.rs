// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::collections::HashMap;

use hotshot_example_types::{
    node_types::{
        CombinedImpl, EpochsTestVersions, Libp2pImpl, PushCdnImpl, TestTwoStakeTablesTypes,
        TestTypes,
    },
    testable_delay::{DelayConfig, DelayOptions, DelaySettings, SupportedTraitTypesForAsyncDelay},
};
use hotshot_macros::cross_tests;
use hotshot_testing::{block_builder::SimpleBuilderImplementation, test_builder::TestDescription};

cross_tests!(
    TestName: test_success_with_async_delay_with_epochs,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestTypes, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default();

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
        let mut config_map = HashMap::new();
        for i in 0..10 {
            config_map.insert(i as u64, config.clone());
        }
        metadata.async_delay_config = config_map;
        metadata
    },
);

cross_tests!(
    TestName: test_success_with_one_long_delay_with_epochs,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default();

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
        let mut config_map = HashMap::new();
        for i in 0..10 {
            config_map.insert(i as u64, config.clone());
        }
        metadata.async_delay_config = config_map;

        let bad_delay_settings = DelaySettings {
            delay_option: DelayOptions::Fixed,
            min_time_in_milliseconds: 4000,
            max_time_in_milliseconds: 4000,
            fixed_time_in_milliseconds: 4000,
        };
        let mut bad_config = DelayConfig::default();
        bad_config.add_setting(SupportedTraitTypesForAsyncDelay::ValidatedState, &bad_delay_settings);

        metadata.async_delay_config.insert(4, bad_config);

        metadata
    },
);
