// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use hotshot::traits::{
    implementations::{CombinedNetworks, Libp2pNetwork, MemoryNetwork, PushCdnNetwork},
    NodeImplementation,
};
use hotshot_types::{
    constants::TEST_UPGRADE_CONSTANTS,
    data::{EpochNumber, ViewNumber},
    signature_key::{BLSPubKey, BuilderKey, SchnorrPubKey},
    traits::node_implementation::{NodeType, Versions},
    upgrade_config::UpgradeConstants,
};
use serde::{Deserialize, Serialize};
use vbs::version::StaticVersion;

pub use crate::membership::helpers::{RandomOverlapQuorumFilterConfig, StableQuorumFilterConfig};
use crate::{
    block_types::{TestBlockHeader, TestBlockPayload, TestTransaction},
    membership::{
        helpers::QuorumFilterConfig, randomized_committee::RandomizedStakeTable,
        randomized_committee_members::RandomizedCommitteeMembers, stake_table::TestStakeTable,
        static_committee::StaticStakeTable,
        static_committee_leader_two_views::StaticStakeTableLeaderForTwoViews,
        strict_membership::StrictMembership, two_static_committees::TwoStakeTables,
    },
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
/// filler struct to implement node type and allow us
/// to select our traits
pub struct TestTypes;
impl NodeType for TestTypes {
    const UPGRADE_CONSTANTS: UpgradeConstants = TEST_UPGRADE_CONSTANTS;

    type View = ViewNumber;
    type Epoch = EpochNumber;
    type BlockHeader = TestBlockHeader;
    type BlockPayload = TestBlockPayload;
    type SignatureKey = BLSPubKey;
    type Transaction = TestTransaction;
    type ValidatedState = TestValidatedState;
    type InstanceState = TestInstanceState;
    type Membership = StrictMembership<TestTypes, StaticStakeTable<BLSPubKey, SchnorrPubKey>>;
    type BuilderSignatureKey = BuilderKey;
    type StateSignatureKey = SchnorrPubKey;
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
/// filler struct to implement node type and allow us
/// to select our traits
pub struct TestTypesRandomizedLeader;
impl NodeType for TestTypesRandomizedLeader {
    const UPGRADE_CONSTANTS: UpgradeConstants = TEST_UPGRADE_CONSTANTS;

    type View = ViewNumber;
    type Epoch = EpochNumber;
    type BlockHeader = TestBlockHeader;
    type BlockPayload = TestBlockPayload;
    type SignatureKey = BLSPubKey;
    type Transaction = TestTransaction;
    type ValidatedState = TestValidatedState;
    type InstanceState = TestInstanceState;
    type Membership =
        StrictMembership<TestTypesRandomizedLeader, RandomizedStakeTable<BLSPubKey, SchnorrPubKey>>;
    type BuilderSignatureKey = BuilderKey;
    type StateSignatureKey = SchnorrPubKey;
}

#[derive(Debug, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct TestTypesEpochCatchupTypes<StakeTable: TestStakeTable<BLSPubKey, SchnorrPubKey>> {
    _pd: PhantomData<StakeTable>,
}

impl<StakeTable: TestStakeTable<BLSPubKey, SchnorrPubKey>> Default
    for TestTypesEpochCatchupTypes<StakeTable>
{
    fn default() -> Self {
        Self { _pd: PhantomData }
    }
}

impl<StakeTable: TestStakeTable<BLSPubKey, SchnorrPubKey>> Hash
    for TestTypesEpochCatchupTypes<StakeTable>
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self._pd.hash(state);
    }
}

impl<StakeTable: TestStakeTable<BLSPubKey, SchnorrPubKey>> PartialEq
    for TestTypesEpochCatchupTypes<StakeTable>
{
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<StakeTable: TestStakeTable<BLSPubKey, SchnorrPubKey>> Eq
    for TestTypesEpochCatchupTypes<StakeTable>
{
}

impl<StakeTable: TestStakeTable<BLSPubKey, SchnorrPubKey>> Copy
    for TestTypesEpochCatchupTypes<StakeTable>
{
}

impl<StakeTable: TestStakeTable<BLSPubKey, SchnorrPubKey>> Clone
    for TestTypesEpochCatchupTypes<StakeTable>
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<StakeTable: TestStakeTable<BLSPubKey, SchnorrPubKey> + 'static> NodeType
    for TestTypesEpochCatchupTypes<StakeTable>
{
    const UPGRADE_CONSTANTS: UpgradeConstants = TEST_UPGRADE_CONSTANTS;

    type View = ViewNumber;
    type Epoch = EpochNumber;
    type BlockHeader = TestBlockHeader;
    type BlockPayload = TestBlockPayload;
    type SignatureKey = BLSPubKey;
    type Transaction = TestTransaction;
    type ValidatedState = TestValidatedState;
    type InstanceState = TestInstanceState;
    type Membership = StrictMembership<TestTypesEpochCatchupTypes<StakeTable>, StakeTable>;
    type BuilderSignatureKey = BuilderKey;
    type StateSignatureKey = SchnorrPubKey;
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
/// filler struct to implement node type and allow us
/// to select our traits
pub struct TestTypesRandomizedCommitteeMembers<
    QuorumConfig: QuorumFilterConfig,
    DaConfig: QuorumFilterConfig,
> {
    _pd: PhantomData<QuorumConfig>,
    _dd: PhantomData<DaConfig>,
}

impl<QuorumConfig: QuorumFilterConfig, DaConfig: QuorumFilterConfig> NodeType
    for TestTypesRandomizedCommitteeMembers<QuorumConfig, DaConfig>
{
    const UPGRADE_CONSTANTS: UpgradeConstants = TEST_UPGRADE_CONSTANTS;

    type View = ViewNumber;
    type Epoch = EpochNumber;
    type BlockHeader = TestBlockHeader;
    type BlockPayload = TestBlockPayload;
    type SignatureKey = BLSPubKey;
    type Transaction = TestTransaction;
    type ValidatedState = TestValidatedState;
    type InstanceState = TestInstanceState;
    type Membership = StrictMembership<
        TestTypesRandomizedCommitteeMembers<QuorumConfig, DaConfig>,
        RandomizedCommitteeMembers<BLSPubKey, SchnorrPubKey, QuorumConfig, DaConfig>,
    >;
    type BuilderSignatureKey = BuilderKey;
    type StateSignatureKey = SchnorrPubKey;
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
/// filler struct to implement node type and allow us
/// to select our traits
pub struct TestConsecutiveLeaderTypes;
impl NodeType for TestConsecutiveLeaderTypes {
    const UPGRADE_CONSTANTS: UpgradeConstants = TEST_UPGRADE_CONSTANTS;

    type View = ViewNumber;
    type Epoch = EpochNumber;
    type BlockHeader = TestBlockHeader;
    type BlockPayload = TestBlockPayload;
    type SignatureKey = BLSPubKey;
    type Transaction = TestTransaction;
    type ValidatedState = TestValidatedState;
    type InstanceState = TestInstanceState;
    type Membership = StrictMembership<
        TestConsecutiveLeaderTypes,
        StaticStakeTableLeaderForTwoViews<BLSPubKey, SchnorrPubKey>,
    >;
    type BuilderSignatureKey = BuilderKey;
    type StateSignatureKey = SchnorrPubKey;
}

#[derive(
    Copy,
    Clone,
    Debug,
    Default,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
/// filler struct to implement node type and allow us
/// to select our traits
pub struct TestTwoStakeTablesTypes;
impl NodeType for TestTwoStakeTablesTypes {
    const UPGRADE_CONSTANTS: UpgradeConstants = TEST_UPGRADE_CONSTANTS;

    type View = ViewNumber;
    type Epoch = EpochNumber;
    type BlockHeader = TestBlockHeader;
    type BlockPayload = TestBlockPayload;
    type SignatureKey = BLSPubKey;
    type Transaction = TestTransaction;
    type ValidatedState = TestValidatedState;
    type InstanceState = TestInstanceState;
    type Membership =
        StrictMembership<TestTwoStakeTablesTypes, TwoStakeTables<BLSPubKey, SchnorrPubKey>>;
    type BuilderSignatureKey = BuilderKey;
    type StateSignatureKey = SchnorrPubKey;
}

/// The Push CDN implementation
#[derive(Clone, Debug, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub struct PushCdnImpl;

/// Memory network implementation
#[derive(Clone, Debug, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub struct MemoryImpl;

/// Libp2p network implementation
#[derive(Clone, Debug, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub struct Libp2pImpl;

/// Web server network implementation
#[derive(Clone, Debug, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub struct WebImpl;

/// Combined Network implementation (libp2p + web server)
#[derive(Clone, Debug, Deserialize, Serialize, Hash, Eq, PartialEq)]
pub struct CombinedImpl;

impl<TYPES: NodeType> NodeImplementation<TYPES> for PushCdnImpl {
    type Network = PushCdnNetwork<TYPES::SignatureKey>;
    type Storage = TestStorage<TYPES>;
}

impl<TYPES: NodeType> NodeImplementation<TYPES> for MemoryImpl {
    type Network = MemoryNetwork<TYPES::SignatureKey>;
    type Storage = TestStorage<TYPES>;
}

impl<TYPES: NodeType> NodeImplementation<TYPES> for CombinedImpl {
    type Network = CombinedNetworks<TYPES>;
    type Storage = TestStorage<TYPES>;
}

impl<TYPES: NodeType> NodeImplementation<TYPES> for Libp2pImpl {
    type Network = Libp2pNetwork<TYPES>;
    type Storage = TestStorage<TYPES>;
}

#[derive(Clone, Debug, Copy)]
pub struct TestVersions {}

impl Versions for TestVersions {
    type Base = StaticVersion<0, 1>;
    type Upgrade = StaticVersion<0, 2>;
    const UPGRADE_HASH: [u8; 32] = [
        1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
        0, 0,
    ];

    type Epochs = StaticVersion<0, 4>;
    type DrbAndHeaderUpgrade = StaticVersion<0, 5>;
}

#[derive(Clone, Debug, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EpochsTestVersions {}

impl Versions for EpochsTestVersions {
    type Base = StaticVersion<0, 3>;
    type Upgrade = StaticVersion<0, 3>;
    const UPGRADE_HASH: [u8; 32] = [
        1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
        0, 0,
    ];

    type Epochs = StaticVersion<0, 3>;
    type DrbAndHeaderUpgrade = StaticVersion<0, 5>;
}

#[derive(Clone, Debug, Copy)]
pub struct EpochUpgradeTestVersions {}

impl Versions for EpochUpgradeTestVersions {
    type Base = StaticVersion<0, 3>;
    type Upgrade = StaticVersion<0, 4>;
    const UPGRADE_HASH: [u8; 32] = [
        1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
        0, 0,
    ];

    type Epochs = StaticVersion<0, 4>;
    type DrbAndHeaderUpgrade = StaticVersion<0, 5>;
}

#[derive(Clone, Debug, Copy)]
pub struct DaCommitteeTestVersions {}

impl Versions for DaCommitteeTestVersions {
    type Base = StaticVersion<0, 2>;
    type Upgrade = StaticVersion<0, 4>;
    const UPGRADE_HASH: [u8; 32] = [
        1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
        0, 0,
    ];

    type Epochs = StaticVersion<0, 1>;
    type DrbAndHeaderUpgrade = StaticVersion<0, 1>;
}

#[cfg(test)]
mod tests {
    use committable::{Commitment, Committable};
    use hotshot_types::{
        impl_has_epoch,
        message::UpgradeLock,
        simple_vote::{HasEpoch, VersionedVoteData},
        traits::node_implementation::ConsensusTime,
        utils::{genesis_epoch_from_version, option_epoch_from_block_number},
    };
    use serde::{Deserialize, Serialize};

    use crate::node_types::{EpochsTestVersions, NodeType, TestTypes, TestVersions};
    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Hash, Eq)]
    /// Dummy data used for test
    struct TestData<TYPES: NodeType> {
        data: u64,
        epoch: Option<TYPES::Epoch>,
    }

    impl<TYPES: NodeType> Committable for TestData<TYPES> {
        fn commit(&self) -> Commitment<Self> {
            committable::RawCommitmentBuilder::new("Test data")
                .u64(self.data)
                .finalize()
        }
    }

    impl_has_epoch!(TestData<TYPES>);

    #[tokio::test(flavor = "multi_thread")]
    /// Test that the view number affects the commitment post-marketplace
    async fn test_versioned_commitment_includes_view() {
        let upgrade_lock = UpgradeLock::new();

        let data = TestData {
            data: 10,
            epoch: None,
        };

        let view_0 = <TestTypes as NodeType>::View::new(0);
        let view_1 = <TestTypes as NodeType>::View::new(1);

        let versioned_data_0 =
            VersionedVoteData::<TestTypes, TestData<TestTypes>, TestVersions>::new(
                data,
                view_0,
                &upgrade_lock,
            )
            .await
            .unwrap();
        let versioned_data_1 =
            VersionedVoteData::<TestTypes, TestData<TestTypes>, TestVersions>::new(
                data,
                view_1,
                &upgrade_lock,
            )
            .await
            .unwrap();

        let versioned_data_commitment_0: [u8; 32] = versioned_data_0.commit().into();
        let versioned_data_commitment_1: [u8; 32] = versioned_data_1.commit().into();

        assert!(
            versioned_data_commitment_0 != versioned_data_commitment_1,
            "left: {versioned_data_commitment_0:?}, right: {versioned_data_commitment_1:?}"
        );
    }

    #[test]
    fn test_option_epoch_from_block_number() {
        // block 0 is always epoch 0
        let epoch = option_epoch_from_block_number::<TestTypes>(true, 1, 10);
        assert_eq!(Some(<TestTypes as NodeType>::Epoch::new(1)), epoch);

        let epoch = option_epoch_from_block_number::<TestTypes>(true, 1, 10);
        assert_eq!(Some(<TestTypes as NodeType>::Epoch::new(1)), epoch);

        let epoch = option_epoch_from_block_number::<TestTypes>(true, 10, 10);
        assert_eq!(Some(<TestTypes as NodeType>::Epoch::new(1)), epoch);

        let epoch = option_epoch_from_block_number::<TestTypes>(true, 11, 10);
        assert_eq!(Some(<TestTypes as NodeType>::Epoch::new(2)), epoch);

        let epoch = option_epoch_from_block_number::<TestTypes>(true, 20, 10);
        assert_eq!(Some(<TestTypes as NodeType>::Epoch::new(2)), epoch);

        let epoch = option_epoch_from_block_number::<TestTypes>(true, 21, 10);
        assert_eq!(Some(<TestTypes as NodeType>::Epoch::new(3)), epoch);

        let epoch = option_epoch_from_block_number::<TestTypes>(true, 21, 0);
        assert_eq!(None, epoch);

        let epoch = option_epoch_from_block_number::<TestTypes>(false, 21, 10);
        assert_eq!(None, epoch);

        let epoch = option_epoch_from_block_number::<TestTypes>(false, 21, 0);
        assert_eq!(None, epoch);
    }

    #[test]
    fn test_genesis_epoch_from_version() {
        let epoch = genesis_epoch_from_version::<TestVersions, TestTypes>();
        assert_eq!(None, epoch);

        let epoch = genesis_epoch_from_version::<EpochsTestVersions, TestTypes>();
        assert_eq!(Some(<TestTypes as NodeType>::Epoch::new(1)), epoch);
    }
}
