// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::time::Duration;

use hotshot_example_types::node_types::{
    EpochsTestVersions, Libp2pImpl, MemoryImpl, PushCdnImpl, StableQuorumFilterConfig,
    TestTypesRandomizedCommitteeMembers,
};
use hotshot_macros::cross_tests;
use hotshot_testing::{
    block_builder::SimpleBuilderImplementation,
    test_builder::{TestDescription, TimingData},
};

cross_tests!(
    TestName: test_epoch_success,
    Impls: [MemoryImpl, Libp2pImpl, PushCdnImpl],
    Types: [
        TestTypesRandomizedCommitteeMembers<StableQuorumFilterConfig<123, 2>, StableQuorumFilterConfig<123, 2>>,                 // Overlap =  F
    ],
    Versions: [EpochsTestVersions],
    Ignore: false,
    Metadata: {
        let mut metadata = TestDescription::default().set_num_nodes(14, 14);
        metadata.timing_data = TimingData {
            next_view_timeout: 12000,
            view_sync_timeout: Duration::from_millis(4000),
            ..Default::default()
        };
        metadata
    },
);
