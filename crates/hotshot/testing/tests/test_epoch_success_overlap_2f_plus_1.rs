// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use hotshot_example_types::node_types::{
    EpochsTestVersions, Libp2pImpl, MemoryImpl, PushCdnImpl, StableQuorumFilterConfig,
    TestTypesRandomizedCommitteeMembers,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{block_builder::SimpleBuilderImplementation, test_builder::TestDescription};

cross_tests!(
    TestName: test_epoch_success,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [
        TestTypesRandomizedCommitteeMembers<StableQuorumFilterConfig<123, 5>, StableQuorumFilterConfig<123, 5>>,                 // Overlap = 2F+1
    ],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        TestDescription::default().set_num_nodes(14, 14)
    },
);
