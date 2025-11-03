// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use alloy::primitives::U256;
use hotshot_example_types::node_types::{
    DaCommitteeTestVersions, MemoryImpl, TestTypes, TestTypesRandomizedLeader,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    completion_task::{CompletionTaskDescription, TimeBasedCompletionTaskDescription},
    overall_safety_task::OverallSafetyPropertiesDescription,
    test_builder::TestDescription,
};
use hotshot_types::ValidatorConfig;
use vbs::version::Version;

cross_tests!(
    TestName: test_da_committees_downhalf,
    Impls: [MemoryImpl],
    Types: [TestTypes, TestTypesRandomizedLeader],
    Versions: [DaCommitteeTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(120),
                                             },
                                         ),
            ..TestDescription::default()
        };

        let node_configs = [
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                0,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                1,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                2,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                3,
                U256::from(1),
                true,
            ),
        ];

        metadata.test_config.epoch_height = 50;
        metadata.test_config.da_committees.push(hotshot_types::VersionedDaCommittee {
            start_version: Version{major: 0, minor: 4},
            start_epoch: 0,
            committee: vec![
                node_configs[0].public_config(),
                node_configs[1].public_config(),
                node_configs[2].public_config(),
                node_configs[3].public_config(),
            ],
        });
        metadata.test_config.da_committees.push(hotshot_types::VersionedDaCommittee {
            start_version: Version{major: 0, minor: 4},
            start_epoch: 2,
            committee: vec![
                node_configs[2].public_config(),
                node_configs[3].public_config(),
            ],
        });

        metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 200,
            ..Default::default()
        };

        metadata
    },
);

cross_tests!(
    TestName: test_da_committees_uphalf,
    Impls: [MemoryImpl],
    Types: [TestTypes, TestTypesRandomizedLeader],
    Versions: [DaCommitteeTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(120),
                                             },
                                         ),
            ..TestDescription::default()
        };

        let node_configs = [
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                0,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                1,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                2,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                3,
                U256::from(1),
                true,
            ),
        ];

        metadata.test_config.epoch_height = 50;
        metadata.test_config.da_committees.push(hotshot_types::VersionedDaCommittee {
            start_version: Version{major: 0, minor: 4},
            start_epoch: 0,
            committee: vec![
                node_configs[1].public_config(),
                node_configs[2].public_config(),
            ],
        });
        metadata.test_config.da_committees.push(hotshot_types::VersionedDaCommittee {
            start_version: Version{major: 0, minor: 4},
            start_epoch: 2,
            committee: vec![
                node_configs[0].public_config(),
                node_configs[1].public_config(),
                node_configs[2].public_config(),
                node_configs[3].public_config(),
            ],
        });

        metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 200,
            ..Default::default()
        };

        metadata
    },
);

cross_tests!(
    TestName: test_da_committees_changehalf,
    Impls: [MemoryImpl],
    Types: [TestTypes, TestTypesRandomizedLeader],
    Versions: [DaCommitteeTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(120),
                                             },
                                         ),
            ..TestDescription::default()
        };

        let node_configs = [
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                0,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                1,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                2,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                3,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                4,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                5,
                U256::from(1),
                true,
            ),
        ];

        metadata.test_config.epoch_height = 50;
        metadata.test_config.da_committees.push(hotshot_types::VersionedDaCommittee {
            start_version: Version{major: 0, minor: 4},
            start_epoch: 0,
            committee: vec![
                node_configs[0].public_config(),
                node_configs[1].public_config(),
                node_configs[2].public_config(),
                node_configs[3].public_config(),
            ],
        });
        metadata.test_config.da_committees.push(hotshot_types::VersionedDaCommittee {
            start_version: Version{major: 0, minor: 4},
            start_epoch: 2,
            committee: vec![
                node_configs[2].public_config(),
                node_configs[3].public_config(),
                node_configs[4].public_config(),
                node_configs[5].public_config(),
            ],
        });

        metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 200,
            ..Default::default()
        };

        metadata
    },
);

cross_tests!(
    TestName: test_da_committees_changehalf_small,
    Impls: [MemoryImpl],
    Types: [TestTypes, TestTypesRandomizedLeader],
    Versions: [DaCommitteeTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription {
            // allow more time to pass in CI
            completion_task_description: CompletionTaskDescription::TimeBasedCompletionTaskBuilder(
                                             TimeBasedCompletionTaskDescription {
                                                 duration: Duration::from_secs(120),
                                             },
                                         ),
            ..TestDescription::default()
        };

        let node_configs = [
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                0,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                1,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                2,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                3,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                4,
                U256::from(1),
                true,
            ),
            ValidatorConfig::generated_from_seed_indexed(
                [0u8; 32],
                5,
                U256::from(1),
                true,
            ),
        ];

        metadata.test_config.epoch_height = 50;
        metadata.test_config.da_committees.push(hotshot_types::VersionedDaCommittee {
            start_version: Version{major: 0, minor: 4},
            start_epoch: 0,
            committee: vec![
                node_configs[0].public_config(),
                node_configs[1].public_config(),
            ],
        });
        metadata.test_config.da_committees.push(hotshot_types::VersionedDaCommittee {
            start_version: Version{major: 0, minor: 4},
            start_epoch: 2,
            committee: vec![
                node_configs[1].public_config(),
                node_configs[2].public_config(),
            ],
        });

        metadata.overall_safety_properties = OverallSafetyPropertiesDescription {
            num_successful_views: 200,
            ..Default::default()
        };

        metadata
    },
);
