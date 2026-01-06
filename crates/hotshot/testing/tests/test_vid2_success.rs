// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use hotshot_example_types::node_types::{
    Libp2pImpl, MemoryImpl, PushCdnImpl, TestTypes, Vid2TestVersions,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{block_builder::SimpleBuilderImplementation, test_builder::TestDescription};

cross_tests!(
    TestName: test_vid2_success,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [
        TestTypes,
    ],
    Versions: [Vid2TestVersions],
    Ignore: false,
    Metadata: {
        TestDescription::default().set_num_nodes(5, 5)
    },
);
