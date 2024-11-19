use super::{L1Client, NodeState};
use ethers::{abi::Address, types::U256};
use hotshot::types::SignatureKey;
use hotshot_types::{
    traits::{
        election::Membership, network::Topic, node_implementation::NodeType,
        signature_key::StakeTableEntryType,
    },
    PeerConfig,
};
use std::{cmp::max, collections::BTreeMap, num::NonZeroU64, str::FromStr};
use thiserror::Error;
use url::Url;

#[derive(Clone, Debug)]
pub struct StakeCommittee<T: NodeType> {
    /// The nodes eligible for leadership.
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<<T::SignatureKey as SignatureKey>::StakeTableEntry>,
    /// The nodes on the committee and their stake
    stake_table: Vec<<T::SignatureKey as SignatureKey>::StakeTableEntry>,

    /// The nodes on the committee and their stake, indexed by public key
    indexed_stake_table:
        BTreeMap<T::SignatureKey, <T::SignatureKey as SignatureKey>::StakeTableEntry>,

    /// Number of blocks in an epoch
    epoch_size: u64,

    /// Address of StakeTable contract (proxy address)
    contract_address: Option<Address>,

    /// L1 provider
    provider: L1Client,

    /// The network topic of the committee
    committee_topic: Topic,
}

impl<TYPES: NodeType> StakeCommittee<TYPES> {
    /// Updates `Self.stake_table` with stake_table for
    /// `Self.contract_address` at `l1_block_height`. This is intended
    /// to be called before calling `self.stake()` so that
    /// `Self.stake_table` only needs to be updated once in a given
    /// life-cycle but may be read from many times.
    // TODO we could just subscribe to L1 updates w/ recent `l1Client`
    // changes, but current structure of `Membership` isn't ideal (we
    // would have a subscription per field).
    async fn update_stake_table(&mut self, l1_block_height: u64) {
        let table: Vec<<<TYPES as NodeType>::SignatureKey as SignatureKey>::StakeTableEntry> = self
            .provider
            .get_stake_table::<TYPES>(l1_block_height, self.contract_address.unwrap())
            .await
            .clone();
        self.stake_table = table;
    }
    // We need a constructor to match our concrete type.
    pub fn new_stake(
        // TODO remove `new` from trait and rename this to `new`.
        // https://github.com/EspressoSystems/HotShot/commit/fcb7d54a4443e29d643b3bbc53761856aef4de8b
        // committee_members are stake holders
        committee_members: Vec<PeerConfig<<TYPES as NodeType>::SignatureKey>>,
        da_members: Vec<PeerConfig<<TYPES as NodeType>::SignatureKey>>,
        committee_topic: Topic,
        instance_state: &NodeState,
        epoch_size: u64,
    ) -> Self {
        // For each eligible leader, get the stake table entry
        let eligible_leaders: Vec<<TYPES::SignatureKey as SignatureKey>::StakeTableEntry> =
            eligible_leaders
                .iter()
                .map(|member| member.stake_table_entry.clone())
                .filter(|entry| entry.stake() > U256::zero())
                .collect();

        // For each member, get the stake table entry
        let members: Vec<<TYPES::SignatureKey as SignatureKey>::StakeTableEntry> =
            committee_members
                .iter()
                .map(|member| member.stake_table_entry.clone())
                .filter(|entry| entry.stake() > U256::zero())
                .collect();

        // Index the stake table by public key
        let indexed_stake_table: BTreeMap<
            TYPES::SignatureKey,
            <TYPES::SignatureKey as SignatureKey>::StakeTableEntry,
        > = members
            .iter()
            .map(|entry| (TYPES::SignatureKey::public_key(entry), entry.clone()))
            .collect();

        Self {
            eligible_leaders,
            stake_table: members,
            indexed_stake_table,
            epoch_size,
            provider: instance_state.l1_client.clone(),
            contract_address: instance_state.chain_config.stake_table_contract,
            committee_topic,
        }
    }
}

#[derive(Error, Debug)]
#[error("Could not lookup leader")] // TODO error variants? message?
pub struct LeaderLookupError;

impl<TYPES: NodeType> Membership<TYPES> for StakeCommittee<TYPES> {
    type Error = LeaderLookupError;

    /// DO NOT USE. Dummy constructor to comply w/ trait.
    fn new(
        // TODO remove `new` from trait and remove this fn as well.
        // https://github.com/EspressoSystems/HotShot/commit/fcb7d54a4443e29d643b3bbc53761856aef4de8b
        eligible_leaders: Vec<PeerConfig<<TYPES as NodeType>::SignatureKey>>,
        committee_members: Vec<PeerConfig<TYPES::SignatureKey>>,
        committee_topic: Topic,
    ) -> Self {
        // For each eligible leader, get the stake table entry
        let eligible_leaders: Vec<<TYPES::SignatureKey as SignatureKey>::StakeTableEntry> =
            eligible_leaders
                .iter()
                .map(|member| member.stake_table_entry.clone())
                .filter(|entry| entry.stake() > U256::zero())
                .collect();

        // For each member, get the stake table entry
        let members: Vec<<TYPES::SignatureKey as SignatureKey>::StakeTableEntry> =
            committee_members
                .iter()
                .map(|member| member.stake_table_entry.clone())
                .filter(|entry| entry.stake() > U256::zero())
                .collect();

        // Index the stake table by public key
        let indexed_stake_table: BTreeMap<
            TYPES::SignatureKey,
            <TYPES::SignatureKey as SignatureKey>::StakeTableEntry,
        > = members
            .iter()
            .map(|entry| (TYPES::SignatureKey::public_key(entry), entry.clone()))
            .collect();

        Self {
            eligible_leaders,
            stake_table: members,
            indexed_stake_table,
            epoch_size: 12, // TODO get the real number from config (I think)
            provider: L1Client::http(Url::from_str("http:://ab.b").unwrap()),
            contract_address: None,
            committee_topic,
        }
    }
    /// Get the stake table for the current view
    fn stake_table(
        &self,
        _epoch: <TYPES as NodeType>::Epoch,
    ) -> Vec<<<TYPES as NodeType>::SignatureKey as SignatureKey>::StakeTableEntry> {
        self.stake_table.clone()
    }

    /// Get all members of the committee for the current view
    fn committee_members(
        &self,
        _view_number: <TYPES as NodeType>::View,
        _epoch: <TYPES as NodeType>::Epoch,
    ) -> std::collections::BTreeSet<<TYPES as NodeType>::SignatureKey> {
        self.stake_table
            .iter()
            .map(TYPES::SignatureKey::public_key)
            .collect()
    }

    /// Get all eligible leaders of the committee for the current view
    fn committee_leaders(
        &self,
        _view_number: <TYPES as NodeType>::View,
        _epoch: <TYPES as NodeType>::Epoch,
    ) -> std::collections::BTreeSet<<TYPES as NodeType>::SignatureKey> {
        self.eligible_leaders
            .iter()
            .map(TYPES::SignatureKey::public_key)
            .collect()
    }

    /// Get the stake table entry for a public key
    fn stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        _epoch: <TYPES as NodeType>::Epoch,
    ) -> Option<<TYPES::SignatureKey as SignatureKey>::StakeTableEntry> {
        // Only return the stake if it is above zero
        self.indexed_stake_table.get(pub_key).cloned()
    }

    /// Check if a node has stake in the committee
    fn has_stake(
        &self,
        pub_key: &<TYPES as NodeType>::SignatureKey,
        _epoch: <TYPES as NodeType>::Epoch,
    ) -> bool {
        self.indexed_stake_table
            .get(pub_key)
            .is_some_and(|x| x.stake() > U256::zero())
    }

    /// Get the network topic for the committee
    fn committee_topic(&self) -> Topic {
        self.committee_topic.clone()
    }

    /// Index the vector of public keys with the current view number
    fn lookup_leader(
        &self,
        view_number: TYPES::View,
        _epoch: <TYPES as NodeType>::Epoch,
    ) -> Result<TYPES::SignatureKey, Self::Error> {
        let index = *view_number as usize % self.eligible_leaders.len();
        let res = self.eligible_leaders[index].clone();
        Ok(TYPES::SignatureKey::public_key(&res))
    }

    /// Get the total number of nodes in the committee
    fn total_nodes(&self, _epoch: <TYPES as NodeType>::Epoch) -> usize {
        self.stake_table.len()
    }

    /// Get the voting success threshold for the committee
    fn success_threshold(&self) -> NonZeroU64 {
        NonZeroU64::new(((self.stake_table.len() as u64 * 2) / 3) + 1).unwrap()
    }

    /// Get the voting failure threshold for the committee
    fn failure_threshold(&self) -> NonZeroU64 {
        NonZeroU64::new(((self.stake_table.len() as u64) / 3) + 1).unwrap()
    }

    /// Get the voting upgrade threshold for the committee
    fn upgrade_threshold(&self) -> NonZeroU64 {
        NonZeroU64::new(max(
            (self.stake_table.len() as u64 * 9) / 10,
            ((self.stake_table.len() as u64 * 2) / 3) + 1,
        ))
        .unwrap()
    }
}
