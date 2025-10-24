use std::{collections::BTreeMap, fmt::Debug, ops::Bound};

use hotshot_types::{
    drb::DrbResult,
    traits::{
        node_implementation::NodeType,
        signature_key::{
            LCV1StateSignatureKey, LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey,
            StateSignatureKey,
        },
    },
    PeerConfig,
};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TestStakeTableEntry<
    PubKey: SignatureKey,
    StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
> {
    pub signature_key: PubKey,
    pub stake_table_entry: <PubKey as SignatureKey>::StakeTableEntry,
    pub state_ver_key: StatePubKey,
}

impl<TYPES: NodeType> From<PeerConfig<TYPES>>
    for TestStakeTableEntry<TYPES::SignatureKey, TYPES::StateSignatureKey>
{
    fn from(peer_config: PeerConfig<TYPES>) -> Self {
        Self {
            signature_key: SignatureKey::public_key(&peer_config.stake_table_entry),
            stake_table_entry: peer_config.stake_table_entry,
            state_ver_key: peer_config.state_ver_key,
        }
    }
}

impl<TYPES: NodeType> From<TestStakeTableEntry<TYPES::SignatureKey, TYPES::StateSignatureKey>>
    for PeerConfig<TYPES>
{
    fn from(
        test_stake_table_entry: TestStakeTableEntry<TYPES::SignatureKey, TYPES::StateSignatureKey>,
    ) -> Self {
        PeerConfig {
            stake_table_entry: test_stake_table_entry.stake_table_entry,
            state_ver_key: test_stake_table_entry.state_ver_key,
        }
    }
}

// Map from first epoch to DA committee stake table entries
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TestDaCommittees<
    PubKey: SignatureKey,
    StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
>(BTreeMap<u64, Vec<TestStakeTableEntry<PubKey, StatePubKey>>>);

impl<
        PubKey: SignatureKey,
        StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
    > TestDaCommittees<PubKey, StatePubKey>
{
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn add(
        &mut self,
        first_epoch: u64,
        committee: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
    ) {
        self.0.insert(first_epoch, committee);
    }

    pub fn get(&self, epoch: Option<u64>) -> Option<Vec<TestStakeTableEntry<PubKey, StatePubKey>>> {
        if let Some(e) = epoch {
            // returns the greatest key smaller than or equal to `e`
            self.0
                .range((Bound::Included(&0), Bound::Included(&e)))
                .last()
                .map(|(_, committee)| committee)
                .cloned()
        } else {
            None
        }
    }
}

impl<
        PubKey: SignatureKey,
        StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
    > Default for TestDaCommittees<PubKey, StatePubKey>
{
    fn default() -> Self {
        Self::new()
    }
}

pub trait TestStakeTable<
    PubKey: SignatureKey,
    StatePubKey: StateSignatureKey + LCV1StateSignatureKey + LCV2StateSignatureKey + LCV3StateSignatureKey,
>: Debug + std::marker::Send + std::marker::Sync
{
    fn new(
        quorum_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
        da_members: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
    ) -> Self;

    fn stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>>;

    fn full_stake_table(&self) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>>;

    fn da_stake_table(&self, epoch: Option<u64>) -> Vec<TestStakeTableEntry<PubKey, StatePubKey>>;

    fn stake(
        &self,
        pub_key: PubKey,
        epoch: Option<u64>,
    ) -> Option<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.stake_table(epoch)
            .iter()
            .find(|entry| entry.signature_key == pub_key)
            .cloned()
    }

    fn da_stake(
        &self,
        pub_key: PubKey,
        epoch: Option<u64>,
    ) -> Option<TestStakeTableEntry<PubKey, StatePubKey>> {
        self.da_stake_table(epoch)
            .iter()
            .find(|entry| entry.signature_key == pub_key)
            .cloned()
    }

    fn lookup_leader(&self, view_number: u64, epoch: Option<u64>) -> anyhow::Result<PubKey>;

    fn has_stake_table(&self, epoch: u64) -> bool;

    fn has_randomized_stake_table(&self, epoch: u64) -> anyhow::Result<bool>;

    fn add_epoch_root(&mut self, epoch: u64);

    fn add_drb_result(&mut self, epoch: u64, drb_result: DrbResult);

    fn set_first_epoch(&mut self, epoch: u64, initial_drb_result: DrbResult);

    fn first_epoch(&self) -> Option<u64>;

    fn get_epoch_drb(&self, epoch: u64) -> anyhow::Result<DrbResult>;

    fn add_da_committee(
        &mut self,
        first_epoch: u64,
        committee: Vec<TestStakeTableEntry<PubKey, StatePubKey>>,
    );
}
