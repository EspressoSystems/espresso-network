use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Debug,
    num::NonZeroU64,
    sync::Arc,
};

use anyhow::Context;
use async_lock::RwLock;
use contract_bindings_alloy::permissionedstaketable::PermissionedStakeTable::StakersUpdated;
use ethers::types::{Address, U256};
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
use itertools::Itertools;
use thiserror::Error;

use super::{
    traits::StateCatchup,
    v0_3::{DAMembers, StakeTable, StakeTables},
    Header, L1Client, Leaf2, NodeState, PubKey, SeqTypes,
};

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
        let changes_per_node = updates
            .into_iter()
            .flat_map(|event| {
                event
                    .removed
                    .into_iter()
                    .map(|key| StakeTableChange::Remove(bls_alloy_to_jf(key)))
                    .chain(
                        event
                            .added
                            .into_iter()
                            .map(|node_info| StakeTableChange::Add(node_info.into())),
                    )
            })
            .group_by(|change| change.key());

        // If the last event for a stakers is `Added` the staker is currently
        // staking, if the last event is removed or (or the staker is not present)
        // they are not staking.
        let currently_staking = changes_per_node
            .into_iter()
            .map(|(_pub_key, deltas)| deltas.last().expect("deltas non-empty").clone())
            .filter_map(|change| match change {
                StakeTableChange::Add(node_info) => Some(node_info),
                StakeTableChange::Remove(_) => None,
            });

        let mut consensus_stake_table: Vec<PeerConfig<PubKey>> = vec![];
        let mut da_members: Vec<PeerConfig<PubKey>> = vec![];
        for node in currently_staking {
            consensus_stake_table.push(node.clone().into());
            if node.da {
                da_members.push(node.into());
            }
        }
        Self::new(consensus_stake_table.into(), da_members.into())
    }
}

#[derive(Clone, derive_more::derive::Debug)]
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

    /// Address of Stake Table Contract
    contract_address: Option<Address>,

    /// Randomized committees, filled when we receive the DrbResult
    randomized_committees: BTreeMap<Epoch, RandomizedCommittee<StakeTableEntry<PubKey>>>,

    /// Peers for catching up the stake table
    #[debug(skip)]
    peers: Option<Arc<dyn StateCatchup>>,
    /// Contains the epoch after which initial_drb_result will not be used (set_first_epoch.epoch + 2)
    /// And the DrbResult to use before that epoch
    initial_drb_result: Option<(Epoch, DrbResult)>,
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
}

/// Holds Stake table and da stake
#[derive(Clone, Debug)]
struct Committee {
    /// The nodes eligible for leadership.
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<PeerConfig<PubKey>>,

    /// Keys for nodes participating in the network
    stake_table: Vec<PeerConfig<PubKey>>,

    /// Keys for DA members
    da_members: Vec<PeerConfig<PubKey>>,

    /// Stake entries indexed by public key, for efficient lookup.
    indexed_stake_table: HashMap<PubKey, PeerConfig<PubKey>>,

    /// DA entries indexed by public key, for efficient lookup.
    indexed_da_members: HashMap<PubKey, PeerConfig<PubKey>>,
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
            .map(|peer_config| {
                (
                    PubKey::public_key(&peer_config.stake_table_entry),
                    peer_config.clone(),
                )
            })
            .collect();

        let indexed_da_members: HashMap<PubKey, _> = st
            .da_members
            .0
            .iter()
            .map(|peer_config| {
                (
                    PubKey::public_key(&peer_config.stake_table_entry),
                    peer_config.clone(),
                )
            })
            .collect();

        let eligible_leaders: Vec<_> = st
            .stake_table
            .0
            .into_iter()
            .filter(|peer_config| peer_config.stake_table_entry.stake() > U256::zero())
            .collect();

        let committee = Committee {
            eligible_leaders,
            stake_table,
            da_members,
            indexed_stake_table,
            indexed_da_members,
        };

        self.state.insert(epoch, committee.clone());
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
            .filter(|&peer_config| peer_config.stake_table_entry.stake() > U256::zero())
            .cloned()
            .collect();

        // For each member, get the stake table entry
        let stake_table: Vec<_> = committee_members
            .iter()
            .filter(|&peer_config| peer_config.stake_table_entry.stake() > U256::zero())
            .cloned()
            .collect();

        // For each member, get the stake table entry
        let da_members: Vec<_> = da_members
            .iter()
            .filter(|&peer_config| peer_config.stake_table_entry.stake() > U256::zero())
            .cloned()
            .collect();

        // Index the stake table by public key
        let indexed_stake_table: HashMap<PubKey, _> = stake_table
            .iter()
            .map(|peer_config| {
                (
                    PubKey::public_key(&peer_config.stake_table_entry),
                    peer_config.clone(),
                )
            })
            .collect();

        // Index the stake table by public key
        let indexed_da_members: HashMap<PubKey, _> = da_members
            .iter()
            .map(|peer_config| {
                (
                    PubKey::public_key(&peer_config.stake_table_entry),
                    peer_config.clone(),
                )
            })
            .collect();

        let members = Committee {
            eligible_leaders,
            stake_table,
            da_members,
            indexed_stake_table,
            indexed_da_members,
        };

        let mut map = HashMap::new();
        map.insert(Epoch::genesis(), members.clone());
        // TODO: remove this, workaround for hotshot asking for stake tables from epoch 1
        map.insert(Epoch::genesis() + 1u64, members.clone());

        Self {
            non_epoch_committee: members,
            state: map,
            _epoch_size: epoch_size,
            l1_client: instance_state.l1_client.clone(),
            contract_address: instance_state.chain_config.stake_table_contract,
            randomized_committees: BTreeMap::new(),
            peers: Some(instance_state.peers.clone()),
            initial_drb_result: None,
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

// #[async_trait]
impl Membership<SeqTypes> for EpochCommittees {
    type Error = LeaderLookupError;
    // DO NOT USE. Dummy constructor to comply w/ trait.
    fn new(
        // TODO remove `new` from trait and remove this fn as well.
        // https://github.com/EspressoSystems/HotShot/commit/fcb7d54a4443e29d643b3bbc53761856aef4de8b
        _committee_members: Vec<PeerConfig<PubKey>>,
        _da_members: Vec<PeerConfig<PubKey>>,
    ) -> Self {
        panic!("This function has been replaced with new_stake()");
    }

    /// Get the stake table for the current view
    fn stake_table(&self, epoch: Option<Epoch>) -> Vec<PeerConfig<PubKey>> {
        if let Some(st) = self.state(&epoch) {
            st.stake_table.clone()
        } else {
            vec![]
        }
    }
    /// Get the stake table for the current view
    fn da_stake_table(&self, epoch: Option<Epoch>) -> Vec<PeerConfig<PubKey>> {
        if let Some(sc) = self.state(&epoch) {
            sc.da_members.clone()
        } else {
            vec![]
        }
    }

    /// Get all members of the committee for the current view
    fn committee_members(
        &self,
        _view_number: <SeqTypes as NodeType>::View,
        epoch: Option<Epoch>,
    ) -> BTreeSet<PubKey> {
        if let Some(sc) = self.state(&epoch) {
            sc.indexed_stake_table.clone().into_keys().collect()
        } else {
            BTreeSet::new()
        }
    }

    /// Get all members of the committee for the current view
    fn da_committee_members(
        &self,
        _view_number: <SeqTypes as NodeType>::View,
        epoch: Option<Epoch>,
    ) -> BTreeSet<PubKey> {
        if let Some(sc) = self.state(&epoch) {
            sc.indexed_da_members.clone().into_keys().collect()
        } else {
            BTreeSet::new()
        }
    }

    /// Get the stake table entry for a public key
    fn stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> Option<PeerConfig<PubKey>> {
        // Only return the stake if it is above zero
        self.state(&epoch)
            .and_then(|h| h.indexed_stake_table.get(pub_key).cloned())
    }

    /// Get the DA stake table entry for a public key
    fn da_stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> Option<PeerConfig<PubKey>> {
        // Only return the stake if it is above zero
        self.state(&epoch)
            .and_then(|h| h.indexed_da_members.get(pub_key).cloned())
    }

    /// Check if a node has stake in the committee
    fn has_stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> bool {
        self.state(&epoch)
            .and_then(|h| h.indexed_stake_table.get(pub_key))
            .is_some_and(|x| x.stake_table_entry.stake() > U256::zero())
    }

    /// Check if a node has stake in the committee
    fn has_da_stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> bool {
        self.state(&epoch)
            .and_then(|h| h.indexed_da_members.get(pub_key))
            .is_some_and(|x| x.stake_table_entry.stake() > U256::zero())
    }

    /// Index the vector of public keys with the current view number
    fn lookup_leader(
        &self,
        view_number: <SeqTypes as NodeType>::View,
        epoch: Option<Epoch>,
    ) -> Result<PubKey, Self::Error> {
        // if we aren't at the initial epoch or epoch is None, we use the non-randomized leader
        if let Some((initial_epoch, _drb_result)) = self.initial_drb_result {
            if epoch.unwrap_or(Epoch::genesis()) < initial_epoch {
                let leaders = &self.non_epoch_committee.eligible_leaders;

                let index = *view_number as usize % leaders.len();
                let res = leaders[index].clone();
                return Ok(PubKey::public_key(&res.stake_table_entry));
            }
        }
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
            Ok(PubKey::public_key(&res.stake_table_entry))
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

    #[allow(refining_impl_trait)]
    async fn add_epoch_root(
        &self,
        epoch: Epoch,
        block_header: Header,
    ) -> Option<Box<dyn FnOnce(&mut Self) + Send>> {
        let address = self.contract_address?;
        self.l1_client
            .get_stake_table(address.to_alloy(), block_header.height())
            .await
            .ok()
            .map(|stake_table| -> Box<dyn FnOnce(&mut Self) + Send> {
                Box::new(move |committee: &mut Self| {
                    let _ = committee.update_stake_table(epoch, stake_table);
                })
            })
    }

    fn has_epoch(&self, epoch: Epoch) -> bool {
        self.state.contains_key(&epoch)
    }

    async fn get_epoch_root_and_drb(
        membership: Arc<RwLock<Self>>,
        block_height: u64,
        epoch_height: u64,
        epoch: Epoch,
    ) -> anyhow::Result<(Header, DrbResult)> {
        let Some(peers) = membership.read().await.peers.clone() else {
            anyhow::bail!("No Peers Configured for Catchup");
        };
        let stake_table = membership.read().await.stake_table(Some(epoch)).clone();
        let success_threshold = membership.read().await.success_threshold(Some(epoch));
        // Fetch leaves from peers
        let leaf: Leaf2 = peers
            .fetch_leaf(
                block_height,
                stake_table.clone(),
                success_threshold,
                epoch_height,
            )
            .await?;
        //DRB height is decided in the next epoch's last block
        let drb_height = block_height + epoch_height + 3;
        let drb_leaf = peers
            .fetch_leaf(drb_height, stake_table, success_threshold, epoch_height)
            .await?;

        Ok((
            leaf.block_header().clone(),
            drb_leaf
                .next_drb_result
                .context(format!("No DRB result on decided leaf at {drb_height}"))?,
        ))
    }

    fn add_drb_result(&mut self, epoch: Epoch, drb: DrbResult) {
        let Some(raw_stake_table) = self.state.get(&epoch) else {
            tracing::error!("add_drb_result({}, {:?}) was called, but we do not yet have the stake table for epoch {}", epoch, drb, epoch);
            return;
        };

        let leaders = raw_stake_table
            .eligible_leaders
            .clone()
            .into_iter()
            .map(|peer_config| peer_config.stake_table_entry)
            .collect::<Vec<_>>();
        let randomized_committee = generate_stake_cdf(leaders, drb);

        self.randomized_committees
            .insert(epoch, randomized_committee);
    }

    fn set_first_epoch(&mut self, epoch: Epoch, initial_drb_result: DrbResult) {
        self.state.insert(epoch, self.non_epoch_committee.clone());
        self.state
            .insert(epoch + 1, self.non_epoch_committee.clone());
        self.initial_drb_result = Some((epoch + 2, initial_drb_result));
    }
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
        assert_eq!(
            st.da_members.0[0].stake_table_entry.stake_key,
            da_node.stake_table_key
        );

        // The consensus stake table contains both nodes
        assert_eq!(st.stake_table.0.len(), 2);
        assert_eq!(
            st.stake_table.0[0].stake_table_entry.stake_key,
            da_node.stake_table_key
        );
        assert_eq!(
            st.stake_table.0[1].stake_table_entry.stake_key,
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
        assert_eq!(
            st.da_members.0[0].stake_table_entry.stake_key,
            da_node.stake_table_key
        );
        assert_eq!(
            st.da_members.0[1].stake_table_entry.stake_key,
            new_da_node.stake_table_key
        );

        // The consensus stake stable (still) contains both nodes
        assert_eq!(st.stake_table.0.len(), 2);
        assert_eq!(
            st.stake_table.0[0].stake_table_entry.stake_key,
            da_node.stake_table_key
        );
        assert_eq!(
            st.stake_table.0[1].stake_table_entry.stake_key,
            new_da_node.stake_table_key
        );

        // Simulate removing the second node
        updates.push(StakersUpdated {
            removed: vec![new_da_node.stake_table_key_alloy()],
            added: vec![],
        });
        let st = StakeTables::from_l1_events(updates);

        // The DA stake table contains only the original DA node
        assert_eq!(st.da_members.0.len(), 1);
        assert_eq!(
            st.da_members.0[0].stake_table_entry.stake_key,
            da_node.stake_table_key
        );

        // The consensus stake table also contains only the original DA node
        assert_eq!(st.stake_table.0.len(), 1);
        assert_eq!(
            st.stake_table.0[0].stake_table_entry.stake_key,
            da_node.stake_table_key
        );
    }

    // TODO: test that repeatedly removes and adds more nodes

    // TODO: the contract prevents removing nodes that aren't stakers and adding
    //       stakers multiple times, but should the rust code handle it too?
}
