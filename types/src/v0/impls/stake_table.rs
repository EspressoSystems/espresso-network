use super::{
    traits::StateCatchup,
    v0_3::{DAMembers, StakeTable, StakeTables},
    v0_99::ChainConfig,
    Header, L1Client, NodeState, PubKey, SeqTypes,
};

use async_trait::async_trait;
use committable::Committable;
use contract_bindings_alloy::permissionedstaketable::PermissionedStakeTable::StakersUpdated;
use ethers::types::U256;
use ethers_conv::ToAlloy;
use hotshot::types::{BLSPubKey, SignatureKey as _};
use hotshot_contract_adapter::stake_table::{bls_alloy_to_jf, NodeInfoJf};
use hotshot_types::{
    data::EpochNumber,
    drb::{
        election::{generate_stake_cdf, select_randomized_leader, RandomizedCommittee},
        DrbResult,
    },
    stake_table::StakeTableEntry,
    traits::{
        election::Membership,
        node_implementation::{ConsensusTime, NodeType},
        signature_key::StakeTableEntryType,
    },
    PeerConfig,
};
use indexmap::IndexMap;
use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet, HashMap},
    num::NonZeroU64,
    sync::Arc,
};
use thiserror::Error;

type Epoch = <SeqTypes as NodeType>::Epoch;

impl StakeTables {
    pub fn new(stake_table: StakeTable, da_members: DAMembers) -> Self {
        Self {
            stake_table,
            da_members,
        }
    }

    /// Create the consensus and DA stake tables from L1 events
    ///
    /// This is a pure function, to make it easily testable.
    ///
    /// We expect have at most a few hundred EVM events in the
    /// PermissionedStakeTable contract over the liftetime of the contract so it
    /// should not significantly affect performance to fetch all events and
    /// perform the computation in this functions once per epoch.
    pub fn from_l1_events(updates: Vec<StakersUpdated>) -> Self {
        let mut index_map = IndexMap::new();

        for event in updates {
            for key in event.removed {
                let change = StakeTableChange::Remove(bls_alloy_to_jf(key));
                index_map.insert(change.key(), change);
            }
            for node_info in event.added {
                let change = StakeTableChange::Add(node_info.into());
                index_map.insert(change.key(), change);
            }
        }

        let mut da_members = Vec::new();
        let mut stake_table = Vec::new();

        for change in index_map.values() {
            if let StakeTableChange::Add(node_info_jf) = change {
                let entry: StakeTableEntry<PubKey> = node_info_jf.clone().into();
                stake_table.push(entry.clone());
                if change.is_da() {
                    da_members.push(entry);
                }
            }
        }

        tracing::error!("DA={da_members:?}");
        tracing::error!("ST={stake_table:?}");

        Self::new(stake_table.into(), da_members.into())
    }
}

#[derive(derive_more::Debug, Clone)]
/// Type to describe DA and Stake memberships
pub struct EpochCommittees {
    /// Committee used when we're in pre-epoch state
    non_epoch_committee: Committee,

    /// Holds Stake table and da stake
    state: HashMap<Epoch, Committee>,

    /// Number of blocks in an epoch
    _epoch_size: u64,

    /// L1 provider
    l1_client: L1Client,

    chain_config: ChainConfig,
    #[debug("{}", peers.name())]
    pub peers: Arc<dyn StateCatchup>,

    /// Randomized committees, filled when we receive the DrbResult
    randomized_committees: BTreeMap<Epoch, RandomizedCommittee<StakeTableEntry<PubKey>>>,
}

#[derive(Debug, Clone, PartialEq)]
enum StakeTableChange {
    Add(NodeInfoJf),
    Remove(BLSPubKey),
}

impl StakeTableChange {
    pub(crate) fn key(&self) -> BLSPubKey {
        match self {
            StakeTableChange::Add(node_info) => node_info.stake_table_key,
            StakeTableChange::Remove(key) => *key,
        }
    }

    pub(crate) fn is_da(&self) -> bool {
        match self {
            StakeTableChange::Add(node_info) => node_info.da,
            StakeTableChange::Remove(_) => false,
        }
    }
}

/// Holds Stake table and da stake
#[derive(Clone, Debug)]
struct Committee {
    /// The nodes eligible for leadership.
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<StakeTableEntry<PubKey>>,

    /// Keys for nodes participating in the network
    stake_table: Vec<StakeTableEntry<PubKey>>,

    /// Keys for DA members
    da_members: Vec<StakeTableEntry<PubKey>>,

    /// Stake entries indexed by public key, for efficient lookup.
    indexed_stake_table: HashMap<PubKey, StakeTableEntry<PubKey>>,

    /// DA entries indexed by public key, for efficient lookup.
    indexed_da_members: HashMap<PubKey, StakeTableEntry<PubKey>>,
}

impl EpochCommittees {
    /// Updates `Self.stake_table` with stake_table for
    /// `Self.contract_address` at `l1_block_height`. This is intended
    /// to be called before calling `self.stake()` so that
    /// `Self.stake_table` only needs to be updated once in a given
    /// life-cycle but may be read from many times.
    fn update_stake_table(&mut self, epoch: EpochNumber, st: StakeTables) -> Committee {
        // This works because `get_stake_table` is fetching *all*
        // update events and building the table for us. We will need
        // more subtlety when start fetching only the events since last update.

        let stake_table = st.stake_table.0.clone();

        let da_members = st.da_members.0.clone();

        let indexed_stake_table: HashMap<PubKey, _> = st
            .stake_table
            .0
            .iter()
            .map(|entry| (PubKey::public_key(entry), entry.clone()))
            .collect();

        let indexed_da_members: HashMap<PubKey, _> = st
            .da_members
            .0
            .iter()
            .map(|entry| (PubKey::public_key(entry), entry.clone()))
            .collect();

        let eligible_leaders: Vec<_> = st
            .stake_table
            .0
            .into_iter()
            .filter(|entry| entry.stake() > U256::zero())
            .collect();

        let randomized_committee = generate_stake_cdf(eligible_leaders.clone(), [0u8; 32]);

        let committee = Committee {
            eligible_leaders,
            stake_table,
            da_members,
            indexed_stake_table,
            indexed_da_members,
        };

        self.state.insert(epoch, committee.clone());
        self.state.insert(epoch + 1, committee.clone());
        self.state.insert(epoch + 2, committee.clone());
        self.randomized_committees
            .insert(epoch, randomized_committee.clone());
        self.randomized_committees
            .insert(epoch + 1, randomized_committee.clone());
        self.randomized_committees
            .insert(epoch + 1, randomized_committee.clone());

        committee
    }

    // We need a constructor to match our concrete type.
    pub fn new_stake(
        // TODO remove `new` from trait and rename this to `new`.
        // https://github.com/EspressoSystems/HotShot/commit/fcb7d54a4443e29d643b3bbc53761856aef4de8b
        committee_members: Vec<PeerConfig<PubKey>>,
        da_members: Vec<PeerConfig<PubKey>>,
        instance_state: &NodeState,
        epoch_size: u64,
    ) -> Self {
        // For each eligible leader, get the stake table entry
        let eligible_leaders: Vec<_> = committee_members
            .iter()
            .map(|member| member.stake_table_entry.clone())
            .filter(|entry| entry.stake() > U256::zero())
            .collect();

        // For each member, get the stake table entry
        let stake_table: Vec<_> = committee_members
            .iter()
            .map(|member| member.stake_table_entry.clone())
            .filter(|entry| entry.stake() > U256::zero())
            .collect();

        // For each member, get the stake table entry
        let da_members: Vec<_> = da_members
            .iter()
            .map(|member| member.stake_table_entry.clone())
            .filter(|entry| entry.stake() > U256::zero())
            .collect();

        // Index the stake table by public key
        let indexed_stake_table: HashMap<PubKey, _> = stake_table
            .iter()
            .map(|entry| (PubKey::public_key(entry), entry.clone()))
            .collect();

        // Index the stake table by public key
        let indexed_da_members: HashMap<PubKey, _> = da_members
            .iter()
            .map(|entry| (PubKey::public_key(entry), entry.clone()))
            .collect();
        let randomized_committee = generate_stake_cdf(eligible_leaders.clone(), [0u8; 32]);

        let members = Committee {
            eligible_leaders,
            stake_table,
            da_members,
            indexed_stake_table,
            indexed_da_members,
        };

        let mut randomized_committees = BTreeMap::new();

        // TODO: remove this, workaround for hotshot asking for stake tables from epoch 1 and 2
        let mut map = HashMap::new();
        for epoch in Epoch::genesis().u64()..=50 {
            map.insert(Epoch::new(epoch), members.clone());
            randomized_committees.insert(Epoch::new(epoch), randomized_committee.clone());
        }

        Self {
            non_epoch_committee: members,
            state: map,
            _epoch_size: epoch_size,
            l1_client: instance_state.l1_client.clone(),
            chain_config: instance_state.chain_config,
            peers: instance_state.peers.clone(),
            randomized_committees,
        }
    }

    fn state(&self, epoch: &Option<Epoch>) -> Option<&Committee> {
        if let Some(epoch) = epoch {
            self.state.get(epoch)
        } else {
            Some(&self.non_epoch_committee)
        }
    }
}

#[derive(Error, Debug)]
#[error("Could not lookup leader")] // TODO error variants? message?
pub struct LeaderLookupError;

#[async_trait]
impl Membership<SeqTypes> for EpochCommittees {
    type Error = LeaderLookupError;

    // DO NOT USE. Dummy constructor to comply w/ trait.
    fn new(
        // TODO remove `new` from trait and remove this fn as well.
        // https://github.com/EspressoSystems/HotShot/commit/fcb7d54a4443e29d643b3bbc53761856aef4de8b
        _committee_members: Vec<PeerConfig<PubKey>>,
        _da_members: Vec<PeerConfig<PubKey>>,
    ) -> Self {
        panic!("EpochCommittees::new() called. This function has been replaced with new_stake()");
    }
    /// Get the stake table for the current view
    fn stake_table(&self, epoch: Option<Epoch>) -> Vec<StakeTableEntry<PubKey>> {
        let st = if let Some(st) = self.state(&epoch) {
            st.stake_table.clone()
        } else {
            vec![]
        };

        tracing::debug!("stake table = {st:?}");
        st
    }
    /// Get the stake table for the current view
    fn da_stake_table(&self, epoch: Option<Epoch>) -> Vec<StakeTableEntry<PubKey>> {
        let da = if let Some(sc) = self.state(&epoch) {
            sc.da_members.clone()
        } else {
            vec![]
        };

        tracing::debug!("da members = {da:?}");
        da
    }

    /// Get all members of the committee for the current view
    fn committee_members(
        &self,
        _view_number: <SeqTypes as NodeType>::View,
        epoch: Option<Epoch>,
    ) -> BTreeSet<PubKey> {
        let committee = if let Some(sc) = self.state(&epoch) {
            sc.indexed_stake_table.clone().into_keys().collect()
        } else {
            BTreeSet::new()
        };

        tracing::debug!("committee={committee:?}");

        committee
    }

    /// Get all members of the committee for the current view
    fn da_committee_members(
        &self,
        _view_number: <SeqTypes as NodeType>::View,
        epoch: Option<Epoch>,
    ) -> BTreeSet<PubKey> {
        let da = if let Some(sc) = self.state(&epoch) {
            sc.indexed_da_members.clone().into_keys().collect()
        } else {
            BTreeSet::new()
        };
        tracing::debug!("da committee={da:?}");

        da
    }

    /// Get all eligible leaders of the committee for the current view
    fn committee_leaders(
        &self,
        _view_number: <SeqTypes as NodeType>::View,
        epoch: Option<Epoch>,
    ) -> BTreeSet<PubKey> {
        let committee_leaders = self
            .state(&epoch)
            .unwrap()
            .eligible_leaders
            .iter()
            .map(PubKey::public_key)
            .collect();

        tracing::debug!("committee_leaders={committee_leaders:?}");
        committee_leaders
    }

    /// Get the stake table entry for a public key
    fn stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> Option<StakeTableEntry<PubKey>> {
        // Only return the stake if it is above zero
        self.state(&epoch)
            .and_then(|h| h.indexed_stake_table.get(pub_key).cloned())
    }

    /// Get the DA stake table entry for a public key
    fn da_stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> Option<StakeTableEntry<PubKey>> {
        // Only return the stake if it is above zero
        self.state(&epoch)
            .and_then(|h| h.indexed_da_members.get(pub_key).cloned())
    }

    /// Check if a node has stake in the committee
    fn has_stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> bool {
        self.state(&epoch)
            .and_then(|h| h.indexed_stake_table.get(pub_key))
            .is_some_and(|x| x.stake() > U256::zero())
    }

    /// Check if a node has stake in the committee
    fn has_da_stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> bool {
        self.state(&epoch)
            .and_then(|h| h.indexed_da_members.get(pub_key))
            .is_some_and(|x| x.stake() > U256::zero())
    }

    /// Index the vector of public keys with the current view number
    fn lookup_leader(
        &self,
        view_number: <SeqTypes as NodeType>::View,
        epoch: Option<Epoch>,
    ) -> Result<PubKey, Self::Error> {
        if let Some(epoch) = epoch {
            let Some(randomized_committee) = self.randomized_committees.get(&epoch) else {
                tracing::error!(
                    "We are missing the randomized committee for epoch {}",
                    epoch
                );
                return Err(LeaderLookupError);
            };

            Ok(PubKey::public_key(&select_randomized_leader(
                randomized_committee,
                *view_number,
            )))
        } else {
            let leaders = &self.non_epoch_committee.eligible_leaders;

            let index = *view_number as usize % leaders.len();
            let res = leaders[index].clone();
            Ok(PubKey::public_key(&res))
        }
    }

    /// Get the total number of nodes in the committee
    fn total_nodes(&self, epoch: Option<Epoch>) -> usize {
        self.state(&epoch)
            .map(|sc| sc.stake_table.len())
            .unwrap_or_default()
    }

    /// Get the total number of DA nodes in the committee
    fn da_total_nodes(&self, epoch: Option<Epoch>) -> usize {
        self.state(&epoch)
            .map(|sc: &Committee| sc.da_members.len())
            .unwrap_or_default()
    }

    /// Get the voting success threshold for the committee
    fn success_threshold(&self, epoch: Option<Epoch>) -> NonZeroU64 {
        let quorum_len = self.state(&epoch).unwrap().stake_table.len();
        NonZeroU64::new(((quorum_len as u64 * 2) / 3) + 1).unwrap()
    }

    /// Get the voting success threshold for the committee
    fn da_success_threshold(&self, epoch: Option<Epoch>) -> NonZeroU64 {
        let da_len = self.state(&epoch).unwrap().da_members.len();
        NonZeroU64::new(((da_len as u64 * 2) / 3) + 1).unwrap()
    }

    /// Get the voting failure threshold for the committee
    fn failure_threshold(&self, epoch: Option<Epoch>) -> NonZeroU64 {
        let quorum_len = self.state(&epoch).unwrap().stake_table.len();

        NonZeroU64::new(((quorum_len as u64) / 3) + 1).unwrap()
    }

    /// Get the voting upgrade threshold for the committee
    fn upgrade_threshold(&self, epoch: Option<Epoch>) -> NonZeroU64 {
        let quorum_len = self.state(&epoch).unwrap().indexed_stake_table.len();

        NonZeroU64::new(max(
            (quorum_len as u64 * 9) / 10,
            ((quorum_len as u64 * 2) / 3) + 1,
        ))
        .unwrap()
    }

    async fn add_epoch_root(
        &self,
        epoch: Epoch,
        block_header: Header,
    ) -> Option<Box<dyn FnOnce(&mut Self) + Send>> {
        let chain_config = get_chain_config(self.chain_config, &self.peers, &block_header)
            .await
            .ok()?;

        let contract_address = chain_config.stake_table_contract;

        if contract_address.is_none() {
            tracing::error!("No stake table contract address found in Chain config");
        }

        let address = contract_address?;

        self.l1_client
            .get_stake_table(address.to_alloy(), block_header.l1_head())
            .await
            .ok()
            .map(|stake_table| -> Box<dyn FnOnce(&mut Self) + Send> {
                Box::new(move |committee: &mut Self| {
                    let _ = committee.update_stake_table(epoch, stake_table);
                })
            })
    }

    fn add_drb_result(&mut self, epoch: Epoch, drb: DrbResult) {
        let Some(raw_stake_table) = self.state.get(&epoch) else {
            tracing::error!("add_drb_result({}, {:?}) was called, but we do not yet have the stake table for epoch {}", epoch, drb, epoch);
            return;
        };

        let randomized_committee =
            generate_stake_cdf(raw_stake_table.eligible_leaders.clone(), drb);

        self.randomized_committees
            .insert(epoch, randomized_committee);
    }
}

pub(crate) async fn get_chain_config(
    chain_config: ChainConfig,
    peers: &impl StateCatchup,
    header: &Header,
) -> anyhow::Result<ChainConfig> {
    let header_cf = header.chain_config();
    if chain_config.commit() == header_cf.commit() {
        return Ok(chain_config);
    }

    let cf = match header_cf.resolve() {
        Some(cf) => cf,
        None => peers
            .fetch_chain_config(header_cf.commit())
            .await
            .map_err(|err| {
                tracing::error!("failed to get chain_config from peers. err: {err:?}");
                err
            })?,
    };

    Ok(cf)
}

#[cfg(test)]
mod tests {
    use contract_bindings_alloy::permissionedstaketable::PermissionedStakeTable::NodeInfo;

    use super::*;

    #[test]
    fn test_stake_table_from_l1_events() {
        let mut rng = rand::thread_rng();

        // Build a stake table with one DA node and one consensus node.
        let mut da_node = NodeInfoJf::random(&mut rng);
        da_node.da = true;
        let mut consensus_node = NodeInfoJf::random(&mut rng);
        consensus_node.da = false;
        let added: Vec<NodeInfo> = vec![da_node.clone().into(), consensus_node.clone().into()];
        let mut updates = vec![StakersUpdated {
            removed: vec![],
            added,
        }];

        let st = StakeTables::from_l1_events(updates.clone());

        // The DA stake table contains the DA node only
        assert_eq!(st.da_members.0.len(), 1);
        assert_eq!(st.da_members.0[0].stake_key, da_node.stake_table_key);

        // The consensus stake table contains both nodes
        assert_eq!(st.stake_table.0.len(), 2);
        assert_eq!(st.stake_table.0[0].stake_key, da_node.stake_table_key);
        assert_eq!(
            st.stake_table.0[1].stake_key,
            consensus_node.stake_table_key
        );

        // Simulate making the consensus node a DA node. This is accomplished by
        // sending a transaction removes and re-adds the same node with updated
        // DA status.
        let mut new_da_node = consensus_node.clone();
        new_da_node.da = true;
        updates.push(StakersUpdated {
            removed: vec![consensus_node.stake_table_key_alloy()],
            added: vec![new_da_node.clone().into()],
        });
        let st = StakeTables::from_l1_events(updates.clone());

        // The DA stake stable now contains both nodes
        assert_eq!(st.da_members.0.len(), 2);
        assert_eq!(st.da_members.0[0].stake_key, da_node.stake_table_key);
        assert_eq!(st.da_members.0[1].stake_key, new_da_node.stake_table_key);

        // The consensus stake stable (still) contains both nodes
        assert_eq!(st.stake_table.0.len(), 2);
        assert_eq!(st.stake_table.0[0].stake_key, da_node.stake_table_key);
        assert_eq!(st.stake_table.0[1].stake_key, new_da_node.stake_table_key);

        // Simulate removing the second node
        updates.push(StakersUpdated {
            removed: vec![new_da_node.stake_table_key_alloy()],
            added: vec![],
        });
        let st = StakeTables::from_l1_events(updates);

        // The DA stake table contains only the original DA node
        assert_eq!(st.da_members.0.len(), 1);
        assert_eq!(st.da_members.0[0].stake_key, da_node.stake_table_key);

        // The consensus stake table also contains only the original DA node
        assert_eq!(st.stake_table.0.len(), 1);
        assert_eq!(st.stake_table.0[0].stake_key, da_node.stake_table_key);
    }

    // TODO: test that repeatedly removes and adds more nodes

    // TODO: the contract prevents removing nodes that aren't stakers and adding
    //       stakers multiple times, but should the rust code handle it too?
}
