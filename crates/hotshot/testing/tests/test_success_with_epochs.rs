// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use hotshot_example_types::node_types::{
    CombinedImpl, EpochsTestVersions, Libp2pImpl, PushCdnImpl, TestTwoStakeTablesTypes, TestTypes,
    TestTypesRandomizedLeader,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{block_builder::SimpleBuilderImplementation, test_builder::TestDescription};

cross_tests!(
    TestName: test_success_with_epochs,
    Impls: [Libp2pImpl, PushCdnImpl, CombinedImpl],
    Types: [TestTypes, TestTypesRandomizedLeader, TestTwoStakeTablesTypes],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default();

        metadata.test_config.epoch_height = 10;

        metadata
    },
);
