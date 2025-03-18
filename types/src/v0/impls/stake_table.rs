use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::NonZeroU64,
    ops::Add,
    sync::Arc,
};

use alloy::{
    primitives::{Address, U256},
    rpc::types::Log,
};
use anyhow::Context;
use async_trait::async_trait;
use contract_bindings_alloy::{
    permissionedstaketable::PermissionedStakeTable::StakersUpdated,
    staketable::StakeTable::{
        ConsensusKeysUpdated, Delegated, Undelegated, ValidatorExit, ValidatorRegistered,
    },
};
use ethers_conv::{ToAlloy, ToEthers};
use hotshot::types::{BLSPubKey, SignatureKey as _};
use hotshot_contract_adapter::stake_table::{
    bls_alloy_to_jf2, edward_bn254point_to_state_ver, NodeInfoJf, ParsedEdOnBN254Point,
};
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
use itertools::Itertools;
use thiserror::Error;

use super::{
    traits::StateCatchup,
    v0_3::{DAMembers, Delegator, StakeTable, Validator},
    Header, L1Client, Leaf2, NodeState, PubKey, SeqTypes,
};

type Epoch = <SeqTypes as NodeType>::Epoch;

/// Create the consensus and DA stake tables from L1 events
///
/// This is a pure function, to make it easily testable.
///
/// We expect have at most a few hundred EVM events in the
/// PermissionedStakeTable contract over the liftetime of the contract so it
/// should not significantly affect performance to fetch all events and
/// perform the computation in this functions once per epoch.
pub fn from_l1_events(
    registered: Vec<(ValidatorRegistered, Log)>,
    deregistered: Vec<(ValidatorExit, Log)>,
    delegated: Vec<(Delegated, Log)>,
    undelegated: Vec<(Undelegated, Log)>,
    keys_update: Vec<(ConsensusKeysUpdated, Log)>,
) -> anyhow::Result<IndexMap<Address, Validator<BLSPubKey>>> {
    let events = StakeTableEvent::sort_events(
        registered,
        deregistered,
        delegated,
        undelegated,
        keys_update,
    )?;

    let mut validators = IndexMap::new();
    let mut bls_keys = HashSet::new();
    let mut schnor_keys = HashSet::new();
    for event in events.values() {
        match event {
            StakeTableEvent::Register(ValidatorRegistered {
                account,
                blsVk,
                schnorrVk,
                commission,
            }) => {
                let stake_table_key = bls_alloy_to_jf2(blsVk.clone());
                let state_ver_key = edward_bn254point_to_state_ver(schnorrVk.clone());
                if bls_keys.contains(&stake_table_key) {
                    tracing::warn!("bls key {stake_table_key:?} already used");
                    continue;
                }

                if schnor_keys.contains(&state_ver_key) {
                    tracing::warn!("schnor verifying key {state_ver_key:?} already used");
                    continue;
                }

                bls_keys.insert(stake_table_key);
                schnor_keys.insert(state_ver_key.clone());

                validators.insert(
                    *account,
                    Validator {
                        account: *account,
                        stake_table_key,
                        state_ver_key,
                        stake: U256::from(0_u64),
                        commission: *commission,
                        delegators: HashMap::default(),
                    },
                );
            },
            StakeTableEvent::Deregister(exit) => {
                validators.shift_remove(&exit.validator);
            },
            StakeTableEvent::Delegate(delegator) => {
                let account = delegator.delegator;
                let validator = delegator.validator;
                let amount = delegator.amount;

                if let Some(validator_entry) = validators.get_mut(&validator) {
                    // Increase stake
                    validator_entry.stake += amount;
                    // Add delegator to the set
                    validator_entry.delegators.insert(account, amount);
                }
            },
            StakeTableEvent::Undelegate(undelegated) => {
                if let Some(validator_entry) = validators.get_mut(&undelegated.validator) {
                    validator_entry.stake = validator_entry
                        .stake
                        .checked_sub(undelegated.amount)
                        .context("stake is less than undelegated amount")?;

                    let delegator = undelegated.delegator;

                    let delegator_stake = validator_entry
                        .delegators
                        .get_mut(&delegator)
                        .context(format!("delegator {delegator:?} not found"))?;
                    *delegator_stake = delegator_stake.checked_sub(undelegated.amount).unwrap();

                    if delegator_stake.is_zero() {
                        // if delegator stake is 0, remove from set
                        validator_entry.delegators.remove(&undelegated.delegator);
                    }
                }
            },
            StakeTableEvent::KeyUpdate(update) => {
                let validator = validators.get_mut(&update.account);
                if let Some(validator) = validator {
                    let bls = bls_alloy_to_jf2(update.blsVK.clone());
                    let state_ver_key = edward_bn254point_to_state_ver(update.schnorrVK.clone());

                    validator.stake_table_key = bls;
                    validator.state_ver_key = state_ver_key;
                }
            },
        }
    }

    Ok(validators)
}

pub enum StakeTableEvent {
    Register(ValidatorRegistered),
    Deregister(ValidatorExit),
    Delegate(Delegated),
    Undelegate(Undelegated),
    KeyUpdate(ConsensusKeysUpdated),
}

impl StakeTableEvent {
    pub fn sort_events(
        registrations: Vec<(ValidatorRegistered, Log)>,
        deregistrations: Vec<(ValidatorExit, Log)>,
        delegations: Vec<(Delegated, Log)>,
        undelegated_events: Vec<(Undelegated, Log)>,
        keys_update: Vec<(ConsensusKeysUpdated, Log)>,
    ) -> anyhow::Result<BTreeMap<(u64, u64), StakeTableEvent>> {
        let mut map = BTreeMap::new();
        for (registration, log) in registrations {
            map.insert(
                (
                    log.block_number.context("block number")?,
                    log.log_index.context("log index")?,
                ),
                StakeTableEvent::Register(registration),
            );
        }
        for (dereg, log) in deregistrations {
            map.insert(
                (
                    log.block_number.context("block number")?,
                    log.log_index.context("log index")?,
                ),
                StakeTableEvent::Deregister(dereg),
            );
        }
        for (delegation, log) in delegations {
            map.insert(
                (
                    log.block_number.context("block number")?,
                    log.log_index.context("log index")?,
                ),
                StakeTableEvent::Delegate(delegation),
            );
        }
        for (undelegated, log) in undelegated_events {
            map.insert(
                (
                    log.block_number.context("block number")?,
                    log.log_index.context("log index")?,
                ),
                StakeTableEvent::Undelegate(undelegated),
            );
        }

        for (update, log) in keys_update {
            map.insert(
                (
                    log.block_number.context("block number")?,
                    log.log_index.context("log index")?,
                ),
                StakeTableEvent::KeyUpdate(update),
            );
        }
        Ok(map)
    }
}

#[derive(Clone, derive_more::derive::Debug)]
/// Type to describe DA and Stake memberships
pub struct EpochCommittees {
    /// Committee used when we're in pre-epoch state
    non_epoch_committee: NonEpochCommittee,

    /// Holds Stake table and da stake
    state: HashMap<Epoch, EpochCommittee>,

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

#[derive(Clone)]
enum StakeTableChange {
    Add(ValidatorRegistered),
    Remove(ValidatorExit),
}

impl PartialEq for StakeTableChange {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (StakeTableChange::Add(a), StakeTableChange::Add(b)) => a.account == b.account,
            (StakeTableChange::Remove(a), StakeTableChange::Remove(b)) => {
                a.validator == b.validator
            },
            _ => false,
        }
    }
}

#[derive(Clone)]
enum DelegationChange {
    Add(Delegated),
    Remove(Undelegated),
}

impl PartialEq for DelegationChange {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DelegationChange::Add(a), DelegationChange::Add(b)) => {
                a.validator == b.validator && a.amount == b.amount && a.delegator == b.delegator
            },
            (DelegationChange::Remove(a), DelegationChange::Remove(b)) => {
                a.delegator == b.delegator && a.validator == b.validator && a.amount == b.amount
            },
            _ => false,
        }
    }
}
// impl StakeTableChange {
//     pub(crate) fn key(&self) -> BLSPubKey {
//         match self {
//             StakeTableChange::Add(validator) => bls_alloy_to_jf2(validator.blsVk)
//             StakeTableChange::Remove(validator) =>  bls_alloy_to_jf2(validator.blsVk)
//         }
//     }
// }

/// Holds Stake table and da stake
#[derive(Clone, Debug)]
struct NonEpochCommittee {
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

/// Holds Stake table and da stake
#[derive(Clone, Debug)]
struct EpochCommittee {
    /// The nodes eligible for leadership.
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<PeerConfig<PubKey>>,
    /// Keys for nodes participating in the network
    stake_table: IndexMap<PubKey, PeerConfig<PubKey>>,
    validators: IndexMap<Address, Validator<BLSPubKey>>,
    address_mapping: HashMap<BLSPubKey, Address>,
}

impl EpochCommittees {
    /// Updates `Self.stake_table` with stake_table for
    /// `Self.contract_address` at `l1_block_height`. This is intended
    /// to be called before calling `self.stake()` so that
    /// `Self.stake_table` only needs to be updated once in a given
    /// life-cycle but may be read from many times.
    fn update_stake_table(
        &mut self,
        epoch: EpochNumber,
        validators: IndexMap<Address, Validator<BLSPubKey>>,
    ) {
        let mut address_mapping = HashMap::new();
        let stake_table = validators
            .values()
            .map(|v| {
                address_mapping.insert(v.stake_table_key, v.account);
                (
                    v.stake_table_key,
                    PeerConfig {
                        stake_table_entry: BLSPubKey::stake_table_entry(
                            &v.stake_table_key,
                            v.stake.to_ethers().as_u64(),
                        ),
                        state_ver_key: v.state_ver_key.clone(),
                    },
                )
            })
            .collect();

        self.state.insert(
            epoch,
            EpochCommittee {
                eligible_leaders: self.non_epoch_committee.eligible_leaders.clone(),
                stake_table,
                validators,
                address_mapping: HashMap::new(),
            },
        );
    }

    pub fn validators(
        &self,
        epoch: &Epoch,
    ) -> anyhow::Result<IndexMap<Address, Validator<BLSPubKey>>> {
        Ok(self
            .state
            .get(epoch)
            .context("state for found")?
            .validators
            .clone())
    }

    pub fn address(&self, epoch: &Epoch, bls_key: BLSPubKey) -> anyhow::Result<Address> {
        let mapping = self
            .state
            .get(epoch)
            .context("state for found")?
            .address_mapping
            .clone();

        Ok(*mapping.get(&bls_key).unwrap())
    }

    pub fn get_validator_config(
        &self,
        epoch: &Epoch,
        key: BLSPubKey,
    ) -> anyhow::Result<Validator<BLSPubKey>> {
        let address = self.address(epoch, key)?;
        let validators = self.validators(epoch)?;
        Ok(validators.get(&address).unwrap().clone())
    }

    // We need a constructor to match our concrete type.
    pub fn new_stake(
        // TODO remove `new` from trait and rename this to `new`.
        // https://github.com/EspressoSystems/HotShot/commit/fcb7d54a4443e29d643b3bbc53761856aef4de8b
        committee_members: Vec<PeerConfig<PubKey>>,
        da_members: Vec<PeerConfig<PubKey>>,
        l1_client: L1Client,
        contract_address: Option<Address>,
        peers: Arc<dyn StateCatchup>,
    ) -> Self {
        // For each eligible leader, get the stake table entry
        let eligible_leaders: Vec<_> = committee_members
            .iter()
            .filter(|&peer_config| {
                peer_config.stake_table_entry.stake() > ethers::types::U256::zero()
            })
            .cloned()
            .collect();

        // For each member, get the stake table entry
        let stake_table: Vec<_> = committee_members
            .iter()
            .filter(|&peer_config| {
                peer_config.stake_table_entry.stake() > ethers::types::U256::zero()
            })
            .cloned()
            .collect();

        // For each member, get the stake table entry
        let da_members: Vec<_> = da_members
            .iter()
            .filter(|&peer_config| {
                peer_config.stake_table_entry.stake() > ethers::types::U256::zero()
            })
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

        let members = NonEpochCommittee {
            eligible_leaders,
            stake_table,
            da_members,
            indexed_stake_table,
            indexed_da_members,
        };

        let mut map = HashMap::new();
        let epoch_committee = EpochCommittee {
            eligible_leaders: members.eligible_leaders.clone(),
            stake_table: members
                .stake_table
                .iter()
                .map(|x| (PubKey::public_key(&x.stake_table_entry), x.clone()))
                .collect(),
            validators: Default::default(),
            address_mapping: HashMap::new(),
        };
        map.insert(Epoch::genesis(), epoch_committee.clone());
        // TODO: remove this, workaround for hotshot asking for stake tables from epoch 1
        map.insert(Epoch::genesis() + 1u64, epoch_committee.clone());

        Self {
            non_epoch_committee: members,
            state: map,
            l1_client,
            contract_address,
            randomized_committees: BTreeMap::new(),
            peers: Some(peers),
            initial_drb_result: None,
        }
    }
    fn get_stake_table(&self, epoch: &Option<Epoch>) -> Option<Vec<PeerConfig<PubKey>>> {
        if let Some(epoch) = epoch {
            self.state
                .get(epoch)
                .map(|committee| committee.stake_table.clone().into_values().collect())
        } else {
            Some(self.non_epoch_committee.stake_table.clone())
        }
    }

    fn leaders(&self, epoch: &Option<Epoch>) -> Option<Vec<PeerConfig<PubKey>>> {
        if let Some(epoch) = epoch {
            self.state
                .get(epoch)
                .map(|committee| committee.eligible_leaders.clone())
        } else {
            Some(self.non_epoch_committee.eligible_leaders.clone())
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
        self.get_stake_table(&epoch).unwrap_or_default()
    }
    /// Get the stake table for the current view
    fn da_stake_table(&self, _epoch: Option<Epoch>) -> Vec<PeerConfig<PubKey>> {
        self.non_epoch_committee.da_members.clone()
    }

    /// Get all members of the committee for the current view
    fn committee_members(
        &self,
        _view_number: <SeqTypes as NodeType>::View,
        epoch: Option<Epoch>,
    ) -> BTreeSet<PubKey> {
        let stake_table = self.stake_table(epoch);
        stake_table
            .iter()
            .map(|x| PubKey::public_key(&x.stake_table_entry))
            .collect()
    }

    /// Get all members of the committee for the current view
    fn da_committee_members(
        &self,
        _view_number: <SeqTypes as NodeType>::View,
        _epoch: Option<Epoch>,
    ) -> BTreeSet<PubKey> {
        self.non_epoch_committee
            .indexed_da_members
            .clone()
            .into_keys()
            .collect()
    }

    /// Get all eligible leaders of the committee for the current view
    fn committee_leaders(
        &self,
        _view_number: <SeqTypes as NodeType>::View,
        epoch: Option<Epoch>,
    ) -> BTreeSet<PubKey> {
        self.leaders(&epoch)
            .unwrap()
            .iter()
            .map(|x| PubKey::public_key(&x.stake_table_entry))
            .collect()
    }

    /// Get the stake table entry for a public key
    fn stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> Option<PeerConfig<PubKey>> {
        // Only return the stake if it is above zero
        if let Some(epoch) = epoch {
            self.state
                .get(&epoch)
                .and_then(|h| h.stake_table.get(pub_key))
                .cloned()
        } else {
            self.non_epoch_committee
                .indexed_stake_table
                .get(pub_key)
                .cloned()
        }
    }

    /// Get the DA stake table entry for a public key
    fn da_stake(&self, pub_key: &PubKey, _epoch: Option<Epoch>) -> Option<PeerConfig<PubKey>> {
        // Only return the stake if it is above zero
        self.non_epoch_committee
            .indexed_da_members
            .get(pub_key)
            .cloned()
    }

    /// Check if a node has stake in the committee
    fn has_stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> bool {
        self.stake(pub_key, epoch)
            .map(|x| x.stake_table_entry.stake() > ethers::types::U256::zero())
            .unwrap_or_default()
    }

    /// Check if a node has stake in the committee
    fn has_da_stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> bool {
        self.da_stake(pub_key, epoch)
            .map(|x| x.stake_table_entry.stake() > ethers::types::U256::zero())
            .unwrap_or_default()
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
            Ok(PubKey::public_key(&res.stake_table_entry))
        }
    }

    /// Get the total number of nodes in the committee
    fn total_nodes(&self, epoch: Option<Epoch>) -> usize {
        self.stake_table(epoch).len()
    }

    /// Get the total number of DA nodes in the committee
    fn da_total_nodes(&self, epoch: Option<Epoch>) -> usize {
        self.da_stake_table(epoch).len()
    }

    /// Get the voting success threshold for the committee
    fn success_threshold(&self, epoch: Option<Epoch>) -> NonZeroU64 {
        let quorum_len = self.stake_table(epoch).len();
        NonZeroU64::new(((quorum_len as u64 * 2) / 3) + 1).unwrap()
    }

    /// Get the voting success threshold for the committee
    fn da_success_threshold(&self, epoch: Option<Epoch>) -> NonZeroU64 {
        let da_len = self.da_stake_table(epoch).len();
        NonZeroU64::new(((da_len as u64 * 2) / 3) + 1).unwrap()
    }

    /// Get the voting failure threshold for the committee
    fn failure_threshold(&self, epoch: Option<Epoch>) -> NonZeroU64 {
        let quorum_len = self.stake_table(epoch).len();

        NonZeroU64::new(((quorum_len as u64) / 3) + 1).unwrap()
    }

    /// Get the voting upgrade threshold for the committee
    fn upgrade_threshold(&self, epoch: Option<Epoch>) -> NonZeroU64 {
        let quorum_len = self.total_nodes(epoch);

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
            .get_stake_table(address, block_header.height())
            .await
            .ok()
            .map(|stake_table| -> Box<dyn FnOnce(&mut Self) + Send> {
                Box::new(move |committee: &mut Self| {
                    committee.update_stake_table(epoch, stake_table);
                })
            })
    }

    fn has_epoch(&self, epoch: Epoch) -> bool {
        self.state.contains_key(&epoch)
    }

    async fn get_epoch_root_and_drb(
        &self,
        block_height: u64,
        epoch_height: u64,
        epoch: Epoch,
    ) -> anyhow::Result<(Header, DrbResult)> {
        let Some(ref peers) = self.peers else {
            anyhow::bail!("No Peers Configured for Catchup");
        };
        // Fetch leaves from peers
        let leaf: Leaf2 = peers
            .fetch_leaf(block_height, self, epoch, epoch_height)
            .await?;
        //DRB height is decided in the next epoch's last block
        let drb_height = block_height + epoch_height + 3;
        let drb_leaf = peers
            .fetch_leaf(drb_height, self, epoch, epoch_height)
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
        // self.state.insert(epoch, self.non_epoch_committee.clone());
        // self.state
        //     .insert(epoch + 1, self.non_epoch_committee.clone());
        // self.initial_drb_result = Some((epoch + 2, initial_drb_result));
    }
}

#[cfg(test)]
mod tests {
    use contract_bindings_alloy::permissionedstaketable::PermissionedStakeTable::NodeInfo;

    use super::*;

    // #[test]
    // fn test_stake_table_from_l1_events() {
    //     let mut rng = rand::thread_rng();

    //     // Build a stake table with one DA node and one consensus node.
    //     let mut da_node = NodeInfoJf::random(&mut rng);
    //     da_node.da = true;
    //     let mut consensus_node = NodeInfoJf::random(&mut rng);
    //     consensus_node.da = false;
    //     let added: Vec<NodeInfo> = vec![da_node.clone().into(), consensus_node.clone().into()];
    //     let mut updates = vec![StakersUpdated {
    //         removed: vec![],
    //         added,
    //     }];

    //     let st = from_l1_events(updates.clone());

    //     // The DA stake table contains the DA node only
    //     assert_eq!(st.da_members.0.len(), 1);
    //     assert_eq!(
    //         st.da_members.0[0].stake_table_entry.stake_key,
    //         da_node.stake_table_key
    //     );

    //     // The consensus stake table contains both nodes
    //     assert_eq!(st.stake_table.0.len(), 2);
    //     assert_eq!(
    //         st.stake_table.0[0].stake_table_entry.stake_key,
    //         da_node.stake_table_key
    //     );
    //     assert_eq!(
    //         st.stake_table.0[1].stake_table_entry.stake_key,
    //         consensus_node.stake_table_key
    //     );

    //     // Simulate making the consensus node a DA node. This is accomplished by
    //     // sending a transaction removes and re-adds the same node with updated
    //     // DA status.
    //     let mut new_da_node = consensus_node.clone();
    //     new_da_node.da = true;
    //     updates.push(StakersUpdated {
    //         removed: vec![consensus_node.stake_table_key_alloy()],
    //         added: vec![new_da_node.clone().into()],
    //     });
    //     let st = StakeTables::from_l1_events(updates.clone());

    //     // The DA stake stable now contains both nodes
    //     assert_eq!(st.da_members.0.len(), 2);
    //     assert_eq!(
    //         st.da_members.0[0].stake_table_entry.stake_key,
    //         da_node.stake_table_key
    //     );
    //     assert_eq!(
    //         st.da_members.0[1].stake_table_entry.stake_key,
    //         new_da_node.stake_table_key
    //     );

    //     // The consensus stake stable (still) contains both nodes
    //     assert_eq!(st.stake_table.0.len(), 2);
    //     assert_eq!(
    //         st.stake_table.0[0].stake_table_entry.stake_key,
    //         da_node.stake_table_key
    //     );
    //     assert_eq!(
    //         st.stake_table.0[1].stake_table_entry.stake_key,
    //         new_da_node.stake_table_key
    //     );

    //     // Simulate removing the second node
    //     updates.push(StakersUpdated {
    //         removed: vec![new_da_node.stake_table_key_alloy()],
    //         added: vec![],
    //     });
    //     let st = StakeTables::from_l1_events(updates);

    //     // The DA stake table contains only the original DA node
    //     assert_eq!(st.da_members.0.len(), 1);
    //     assert_eq!(
    //         st.da_members.0[0].stake_table_entry.stake_key,
    //         da_node.stake_table_key
    //     );

    //     // The consensus stake table also contains only the original DA node
    //     assert_eq!(st.stake_table.0.len(), 1);
    //     assert_eq!(
    //         st.stake_table.0[0].stake_table_entry.stake_key,
    //         da_node.stake_table_key
    //     );
    // }

    // TODO: test that repeatedly removes and adds more nodes

    // TODO: the contract prevents removing nodes that aren't stakers and adding
    //       stakers multiple times, but should the rust code handle it too?
}
