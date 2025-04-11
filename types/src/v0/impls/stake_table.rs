use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::Arc,
};

use alloy::{
    primitives::{Address, U256},
    rpc::types::Log,
};
use anyhow::{bail, Context};
use async_lock::RwLock;
use committable::Committable;
use hotshot::types::{BLSPubKey, SchnorrPubKey, SignatureKey as _};
use hotshot_contract_adapter::sol_types::StakeTable::{
    ConsensusKeysUpdated, Delegated, Undelegated, ValidatorExit, ValidatorRegistered,
};
use hotshot_types::{
    data::{vid_disperse::VID_TARGET_TOTAL_STAKE, EpochNumber},
    drb::{
        election::{generate_stake_cdf, select_randomized_leader, RandomizedCommittee},
        DrbResult,
    },
    message::UpgradeLock,
    stake_table::StakeTableEntry,
    traits::{
        election::Membership,
        node_implementation::{ConsensusTime, NodeType},
        signature_key::StakeTableEntryType,
    },
    utils::verify_leaf_chain,
    PeerConfig,
};
use indexmap::IndexMap;
use thiserror::Error;

#[cfg(any(test, feature = "testing"))]
use super::v0_3::DAMembers;
use super::{
    traits::{MembershipPersistence, StateCatchup},
    v0_3::Validator,
    v0_99::ChainConfig,
    Header, L1Client, Leaf2, PubKey, SeqTypes,
};
use crate::{EpochVersion, SequencerVersions};

type Epoch = <SeqTypes as NodeType>::Epoch;

/// Extract all validators from L1 stake table events.
pub(crate) fn validators_from_l1_events<I: Iterator<Item = StakeTableEvent>>(
    events: I,
) -> anyhow::Result<IndexMap<Address, Validator<BLSPubKey>>> {
    let mut validators = IndexMap::new();
    let mut bls_keys = HashSet::new();
    let mut schnorr_keys = HashSet::new();
    for event in events {
        tracing::debug!("Processing stake table event: {:?}", event);
        match event {
            StakeTableEvent::Register(ValidatorRegistered {
                account,
                blsVk,
                schnorrVk,
                commission,
            }) => {
                // TODO(abdul): BLS and Schnorr signature keys verification
                let stake_table_key: BLSPubKey = blsVk.clone().into();
                let state_ver_key: SchnorrPubKey = schnorrVk.clone().into();
                // TODO(MA): The stake table contract currently enforces that each bls key is only used once. We will
                // move this check to the confirmation layer and remove it from the contract. Once we have the signature
                // check in this functions we can skip if a BLS key, or Schnorr key was previously used.
                if bls_keys.contains(&stake_table_key) {
                    bail!("bls key {} already used", stake_table_key.to_string());
                };

                // The contract does *not* enforce that each schnorr key is only used once.
                if schnorr_keys.contains(&state_ver_key) {
                    tracing::warn!("schnorr key {} already used", state_ver_key.to_string());
                };

                bls_keys.insert(stake_table_key);
                schnorr_keys.insert(state_ver_key.clone());

                match validators.entry(account) {
                    indexmap::map::Entry::Occupied(_occupied_entry) => {
                        bail!("validator {:#x} already registered", *account)
                    },
                    indexmap::map::Entry::Vacant(vacant_entry) => vacant_entry.insert(Validator {
                        account,
                        stake_table_key,
                        state_ver_key,
                        stake: U256::from(0_u64),
                        commission,
                        delegators: HashMap::default(),
                    }),
                };
            },
            StakeTableEvent::Deregister(exit) => {
                validators
                    .shift_remove(&exit.validator)
                    .with_context(|| format!("validator {:#x} not found", exit.validator))?;
            },
            StakeTableEvent::Delegate(delegated) => {
                let Delegated {
                    delegator,
                    validator,
                    amount,
                } = delegated;
                let validator_entry = validators
                    .get_mut(&validator)
                    .with_context(|| format!("validator {validator:#x} not found"))?;

                if amount.is_zero() {
                    tracing::warn!("delegator {delegator:?} has 0 stake");
                    continue;
                }
                // Increase stake
                validator_entry.stake += amount;
                // Insert the delegator with the given stake
                // or increase the stake if already present
                validator_entry
                    .delegators
                    .entry(delegator)
                    .and_modify(|stake| *stake += amount)
                    .or_insert(amount);
            },
            StakeTableEvent::Undelegate(undelegated) => {
                let Undelegated {
                    delegator,
                    validator,
                    amount,
                } = undelegated;
                let validator_entry = validators
                    .get_mut(&validator)
                    .with_context(|| format!("validator {validator:#x} not found"))?;

                validator_entry.stake = validator_entry
                    .stake
                    .checked_sub(amount)
                    .with_context(|| "stake is less than undelegated amount")?;

                let delegator_stake = validator_entry
                    .delegators
                    .get_mut(&delegator)
                    .with_context(|| format!("delegator {delegator:#x} not found"))?;
                *delegator_stake = delegator_stake
                    .checked_sub(amount)
                    .with_context(|| "delegator_stake is less than undelegated amount")?;

                if delegator_stake.is_zero() {
                    // if delegator stake is 0, remove from set
                    validator_entry.delegators.remove(&delegator);
                }
            },
            StakeTableEvent::KeyUpdate(update) => {
                let ConsensusKeysUpdated {
                    account,
                    blsVK,
                    schnorrVK,
                } = update;
                let validator = validators
                    .get_mut(&account)
                    .with_context(|| "validator {account:#x} not found")?;
                let bls = blsVK.into();
                let state_ver_key = schnorrVK.into();

                validator.stake_table_key = bls;
                validator.state_ver_key = state_ver_key;
            },
        }
    }

    Ok(validators)
}

/// Select active validators
///
/// Removes the validators without stake and selects the top 100 staked validators.
pub(crate) fn select_active_validator_set(
    validators: &mut IndexMap<Address, Validator<BLSPubKey>>,
) -> anyhow::Result<()> {
    // Remove invalid validators first
    validators.retain(|address, validator| {
        if validator.delegators.is_empty() {
            tracing::info!("Validator {address:?} does not have any delegator");
            return false;
        }

        if validator.stake.is_zero() {
            tracing::info!("Validator {address:?} does not have any stake");
            return false;
        }

        true
    });

    if validators.is_empty() {
        bail!("No valid validators found");
    }

    // Find the maximum stake
    let maximum_stake = validators
        .values()
        .map(|v| v.stake)
        .max()
        .context("Failed to determine max stake")?;

    let minimum_stake = maximum_stake
        .checked_div(U256::from(VID_TARGET_TOTAL_STAKE))
        .context("div err")?;

    // Collect validators that meet the minimum stake criteria
    let mut valid_stakers: Vec<_> = validators
        .iter()
        .filter(|(_, v)| v.stake >= minimum_stake)
        .map(|(addr, v)| (*addr, v.stake))
        .collect();

    // Sort by stake (descending order)
    valid_stakers.sort_by_key(|(_, stake)| std::cmp::Reverse(*stake));

    // Keep only the top 100 stakers
    if valid_stakers.len() > 100 {
        valid_stakers.truncate(100);
    }

    // Retain only the selected validators
    let selected_addresses: HashSet<_> = valid_stakers.iter().map(|(addr, _)| *addr).collect();
    validators.retain(|address, _| selected_addresses.contains(address));

    Ok(())
}

/// Extract the active validator set from the L1 stake table events.
pub(crate) fn active_validator_set_from_l1_events<I: Iterator<Item = StakeTableEvent>>(
    events: I,
) -> anyhow::Result<IndexMap<Address, Validator<BLSPubKey>>> {
    let mut validators = validators_from_l1_events(events)?;
    select_active_validator_set(&mut validators)?;
    Ok(validators)
}

#[derive(Clone, derive_more::From)]
pub enum StakeTableEvent {
    Register(ValidatorRegistered),
    Deregister(ValidatorExit),
    Delegate(Delegated),
    Undelegate(Undelegated),
    KeyUpdate(ConsensusKeysUpdated),
}

impl std::fmt::Debug for StakeTableEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StakeTableEvent::Register(event) => write!(f, "Register({:?})", event.account),
            StakeTableEvent::Deregister(event) => write!(f, "Deregister({:?})", event.validator),
            StakeTableEvent::Delegate(event) => write!(f, "Delegate({:?})", event.delegator),
            StakeTableEvent::Undelegate(event) => write!(f, "Undelegate({:?})", event.delegator),
            StakeTableEvent::KeyUpdate(event) => write!(f, "KeyUpdate({:?})", event.account),
        }
    }
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
                registration.into(),
            );
        }
        for (dereg, log) in deregistrations {
            map.insert(
                (
                    log.block_number.context("block number")?,
                    log.log_index.context("log index")?,
                ),
                dereg.into(),
            );
        }
        for (delegation, log) in delegations {
            map.insert(
                (
                    log.block_number.context("block number")?,
                    log.log_index.context("log index")?,
                ),
                delegation.into(),
            );
        }
        for (undelegated, log) in undelegated_events {
            map.insert(
                (
                    log.block_number.context("block number")?,
                    log.log_index.context("log index")?,
                ),
                undelegated.into(),
            );
        }

        for (update, log) in keys_update {
            map.insert(
                (
                    log.block_number.context("block number")?,
                    log.log_index.context("log index")?,
                ),
                update.into(),
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
    /// Verifiable `ChainConfig` holding contract address
    chain_config: ChainConfig,
    /// Randomized committees, filled when we receive the DrbResult
    randomized_committees: BTreeMap<Epoch, RandomizedCommittee<StakeTableEntry<PubKey>>>,
    /// Peers for catching up the stake table
    #[debug(skip)]
    peers: Arc<dyn StateCatchup>,
    /// Methods for stake table persistence.
    #[debug(skip)]
    persistence: Arc<dyn MembershipPersistence>,
    first_epoch: Option<Epoch>,
}

/// Holds Stake table and da stake
#[derive(Clone, Debug)]
struct NonEpochCommittee {
    /// The nodes eligible for leadership.
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<PeerConfig<SeqTypes>>,

    /// Keys for nodes participating in the network
    stake_table: Vec<PeerConfig<SeqTypes>>,

    /// Keys for DA members
    da_members: Vec<PeerConfig<SeqTypes>>,

    /// Stake entries indexed by public key, for efficient lookup.
    indexed_stake_table: HashMap<PubKey, PeerConfig<SeqTypes>>,

    /// DA entries indexed by public key, for efficient lookup.
    indexed_da_members: HashMap<PubKey, PeerConfig<SeqTypes>>,
}

/// Holds Stake table and da stake
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct EpochCommittee {
    /// The nodes eligible for leadership.
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<PeerConfig<SeqTypes>>,
    /// Keys for nodes participating in the network
    stake_table: IndexMap<PubKey, PeerConfig<SeqTypes>>,
    validators: IndexMap<Address, Validator<BLSPubKey>>,
    address_mapping: HashMap<BLSPubKey, Address>,
}

impl EpochCommittees {
    pub fn first_epoch(&self) -> Option<Epoch> {
        self.first_epoch
    }

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
        let stake_table: IndexMap<PubKey, PeerConfig<SeqTypes>> = validators
            .values()
            .map(|v| {
                address_mapping.insert(v.stake_table_key, v.account);
                (
                    v.stake_table_key,
                    PeerConfig {
                        stake_table_entry: BLSPubKey::stake_table_entry(
                            &v.stake_table_key,
                            v.stake,
                        ),
                        state_ver_key: v.state_ver_key.clone(),
                    },
                )
            })
            .collect();

        let eligible_leaders: Vec<PeerConfig<SeqTypes>> =
            stake_table.iter().map(|(_, l)| l.clone()).collect();

        self.state.insert(
            epoch,
            EpochCommittee {
                eligible_leaders,
                stake_table,
                validators,
                address_mapping,
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

        Ok(*mapping.get(&bls_key).context(format!(
            "failed to get ethereum address for bls key {bls_key:?}"
        ))?)
    }

    pub fn get_validator_config(
        &self,
        epoch: &Epoch,
        key: BLSPubKey,
    ) -> anyhow::Result<Validator<BLSPubKey>> {
        let address = self.address(epoch, key)?;
        let validators = self.validators(epoch)?;
        validators
            .get(&address)
            .context("validator not found")
            .cloned()
    }

    // We need a constructor to match our concrete type.
    pub fn new_stake(
        // TODO remove `new` from trait and rename this to `new`.
        // https://github.com/EspressoSystems/HotShot/commit/fcb7d54a4443e29d643b3bbc53761856aef4de8b
        committee_members: Vec<PeerConfig<SeqTypes>>,
        da_members: Vec<PeerConfig<SeqTypes>>,
        l1_client: L1Client,
        chain_config: ChainConfig,
        peers: Arc<dyn StateCatchup>,
        persistence: impl MembershipPersistence,
    ) -> Self {
        // For each member, get the stake table entry
        let stake_table: Vec<_> = committee_members
            .iter()
            .filter(|&peer_config| peer_config.stake_table_entry.stake() > U256::ZERO)
            .cloned()
            .collect();

        let eligible_leaders = stake_table.clone();
        // For each member, get the stake table entry
        let da_members: Vec<_> = da_members
            .iter()
            .filter(|&peer_config| peer_config.stake_table_entry.stake() > U256::ZERO)
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
            chain_config,
            randomized_committees: BTreeMap::new(),
            peers,
            persistence: Arc::new(persistence),
            first_epoch: None,
        }
    }

    pub async fn reload_stake(&mut self, limit: u64) {
        // Load the 50 latest stored stake tables
        let loaded_stake = match self.persistence.load_latest_stake(limit).await {
            Ok(Some(loaded)) => loaded,
            Ok(None) => {
                tracing::warn!("No stake table history found in persistence!");
                return;
            },
            Err(e) => {
                tracing::error!("Failed to load stake table history from persistence: {}", e);
                return;
            },
        };

        for (epoch, stake_table) in loaded_stake {
            self.update_stake_table(epoch, stake_table);
        }
    }

    fn get_stake_table(&self, epoch: &Option<Epoch>) -> Option<Vec<PeerConfig<SeqTypes>>> {
        if let Some(epoch) = epoch {
            self.state
                .get(epoch)
                .map(|committee| committee.stake_table.clone().into_values().collect())
        } else {
            Some(self.non_epoch_committee.stake_table.clone())
        }
    }

    /// Get the stake table by epoch. Try to load from DB and fall back to fetching from l1.
    async fn get_stake_table_from_l1(
        &self,
        contract_address: Address,
        l1_block: u64,
    ) -> Result<IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>, GetStakeTablesError>
    {
        self.l1_client
            .fetch_stake_table(contract_address, l1_block)
            .await
            .map_err(GetStakeTablesError::L1ClientFetchError)
    }
}

#[derive(Error, Debug)]
/// Error representing fail cases for retrieving the stake table.
enum GetStakeTablesError {
    #[error("Error fetching from L1: {0}")]
    L1ClientFetchError(anyhow::Error),
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
        _committee_members: Vec<PeerConfig<SeqTypes>>,
        _da_members: Vec<PeerConfig<SeqTypes>>,
    ) -> Self {
        panic!("This function has been replaced with new_stake()");
    }

    /// Get the stake table for the current view
    fn stake_table(&self, epoch: Option<Epoch>) -> Vec<PeerConfig<SeqTypes>> {
        self.get_stake_table(&epoch).unwrap_or_default()
    }
    /// Get the stake table for the current view
    fn da_stake_table(&self, _epoch: Option<Epoch>) -> Vec<PeerConfig<SeqTypes>> {
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

    /// Get the stake table entry for a public key
    fn stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> Option<PeerConfig<SeqTypes>> {
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
    fn da_stake(&self, pub_key: &PubKey, _epoch: Option<Epoch>) -> Option<PeerConfig<SeqTypes>> {
        // Only return the stake if it is above zero
        self.non_epoch_committee
            .indexed_da_members
            .get(pub_key)
            .cloned()
    }

    /// Check if a node has stake in the committee
    fn has_stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> bool {
        self.stake(pub_key, epoch)
            .map(|x| x.stake_table_entry.stake() > U256::ZERO)
            .unwrap_or_default()
    }

    /// Check if a node has stake in the committee
    fn has_da_stake(&self, pub_key: &PubKey, epoch: Option<Epoch>) -> bool {
        self.da_stake(pub_key, epoch)
            .map(|x| x.stake_table_entry.stake() > U256::ZERO)
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
    fn success_threshold(&self, epoch: Option<Epoch>) -> U256 {
        let total_stake = self.total_stake(epoch);
        let one = U256::ONE;
        let two = U256::from(2);
        let three = U256::from(3);
        if total_stake < U256::MAX / two {
            ((total_stake * two) / three) + one
        } else {
            ((total_stake / three) * two) + two
        }
    }

    /// Get the voting success threshold for the committee
    fn da_success_threshold(&self, epoch: Option<Epoch>) -> U256 {
        let total_stake = self.total_da_stake(epoch);
        let one = U256::ONE;
        let two = U256::from(2);
        let three = U256::from(3);

        if total_stake < U256::MAX / two {
            ((total_stake * two) / three) + one
        } else {
            ((total_stake / three) * two) + two
        }
    }

    /// Get the voting failure threshold for the committee
    fn failure_threshold(&self, epoch: Option<Epoch>) -> U256 {
        let total_stake = self.total_stake(epoch);
        let one = U256::ONE;
        let three = U256::from(3);

        (total_stake / three) + one
    }

    /// Get the voting upgrade threshold for the committee
    fn upgrade_threshold(&self, epoch: Option<Epoch>) -> U256 {
        let total_stake = self.total_stake(epoch);
        let nine = U256::from(9);
        let ten = U256::from(10);

        let normal_threshold = self.success_threshold(epoch);
        let higher_threshold = if total_stake < U256::MAX / nine {
            (total_stake * nine) / ten
        } else {
            (total_stake / ten) * nine
        };

        max(higher_threshold, normal_threshold)
    }

    #[allow(refining_impl_trait)]
    async fn add_epoch_root(
        &self,
        epoch: Epoch,
        block_header: Header,
    ) -> Option<Box<dyn FnOnce(&mut Self) + Send>> {
        let chain_config = get_chain_config(self.chain_config, &self.peers, &block_header)
            .await
            .ok()?;

        let Some(address) = chain_config.stake_table_contract else {
            tracing::error!("No stake table contract address found in Chain config");
            return None;
        };

        let Some(l1_finalized_block_info) = block_header.l1_finalized() else {
            tracing::error!("The epoch root for epoch {} is missing the L1 finalized block info. This is a fatal error. Consensus is blocked and will not recover.", epoch);
            return None;
        };

        let stake_tables = self
            .get_stake_table_from_l1(address, l1_finalized_block_info.number())
            .await
            .inspect_err(|e| {
                tracing::error!(?e, "`add_epoch_root`, error retrieving stake table");
            })
            .ok()?;

        if let Err(e) = self
            .persistence
            .store_stake(epoch, stake_tables.clone())
            .await
        {
            tracing::error!(?e, "`add_epoch_root`, error storing stake table");
        }

        Some(Box::new(move |committee: &mut Self| {
            committee.update_stake_table(epoch, stake_tables);
        }))
    }

    fn has_stake_table(&self, epoch: Epoch) -> bool {
        self.state.contains_key(&epoch)
    }

    fn has_randomized_stake_table(&self, epoch: Epoch) -> bool {
        match self.first_epoch {
            None => true,
            Some(first_epoch) => {
                if epoch < first_epoch {
                    self.state.contains_key(&epoch)
                } else {
                    self.randomized_committees.contains_key(&epoch)
                }
            },
        }
    }

    async fn get_epoch_root(
        membership: Arc<RwLock<Self>>,
        block_height: u64,
        epoch: Epoch,
    ) -> anyhow::Result<Leaf2> {
        let peers = membership.read().await.peers.clone();
        let stake_table = membership.read().await.stake_table(Some(epoch)).clone();
        let success_threshold = membership.read().await.success_threshold(Some(epoch));
        // Fetch leaves from peers
        let leaf: Leaf2 = peers
            .fetch_leaf(block_height, stake_table.clone(), success_threshold)
            .await?;

        Ok(leaf)
    }

    async fn get_epoch_drb(
        membership: Arc<RwLock<Self>>,
        block_height: u64,
        epoch: Epoch,
    ) -> anyhow::Result<DrbResult> {
        let peers = membership.read().await.peers.clone();
        let stake_table = membership.read().await.stake_table(Some(epoch)).clone();
        let success_threshold = membership.read().await.success_threshold(Some(epoch));

        tracing::debug!(
            "Getting DRB for epoch {:?}, block height {:?}",
            epoch,
            block_height
        );
        let mut drb_leaf_chain = peers.try_fetch_leaves(1, block_height).await?;

        drb_leaf_chain.sort_by_key(|l| l.view_number());
        let leaf_chain = drb_leaf_chain.into_iter().rev().collect();
        let drb_leaf = verify_leaf_chain(
            leaf_chain,
            stake_table.clone(),
            success_threshold,
            block_height,
            &UpgradeLock::<SeqTypes, SequencerVersions<EpochVersion, EpochVersion>>::new(),
        )
        .await?;

        let Some(drb) = drb_leaf.next_drb_result else {
            tracing::error!(
          "We received a leaf that should contain a DRB result, but the DRB result is missing: {:?}",
          drb_leaf
        );

            bail!("DRB leaf is missing the DRB result.");
        };

        Ok(drb)
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
        self.first_epoch = Some(epoch);

        let epoch_committee = self.state.get(&Epoch::genesis()).unwrap().clone();
        self.state.insert(epoch, epoch_committee.clone());
        self.state.insert(epoch + 1, epoch_committee);
        self.add_drb_result(epoch, initial_drb_result);
        self.add_drb_result(epoch + 1, initial_drb_result);
    }
}
/// Retrieve and verify `ChainConfig`
// TODO move to appropriate object (Header?)
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

#[cfg(any(test, feature = "testing"))]
impl super::v0_3::StakeTable {
    /// Generate a `StakeTable` with `n` members.
    pub fn mock(n: u64) -> Self {
        [..n]
            .iter()
            .map(|_| PeerConfig::default())
            .collect::<Vec<PeerConfig<SeqTypes>>>()
            .into()
    }
}

#[cfg(any(test, feature = "testing"))]
impl DAMembers {
    /// Generate a `DaMembers` (alias committee) with `n` members.
    pub fn mock(n: u64) -> Self {
        [..n]
            .iter()
            .map(|_| PeerConfig::default())
            .collect::<Vec<PeerConfig<SeqTypes>>>()
            .into()
    }
}

#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use hotshot_contract_adapter::sol_types::{EdOnBN254PointSol, G2PointSol};
    use hotshot_types::light_client::StateKeyPair;
    use rand::{Rng as _, RngCore as _};

    use super::*;

    // TODO: current tests are just sanity checks, we need more.

    pub struct TestValidator {
        pub account: Address,
        pub bls_vk: G2PointSol,
        pub schnorr_vk: EdOnBN254PointSol,
        pub commission: u16,
    }

    impl TestValidator {
        pub fn random() -> Self {
            let rng = &mut rand::thread_rng();
            let mut seed = [0u8; 32];
            rng.fill_bytes(&mut seed);

            let (bls_vk, _) = BLSPubKey::generated_from_seed_indexed(seed, 0);
            let schnorr_vk: EdOnBN254PointSol = StateKeyPair::generate_from_seed_indexed(seed, 0)
                .ver_key()
                .to_affine()
                .into();

            Self {
                account: Address::random(),
                bls_vk: bls_vk.to_affine().into(),
                schnorr_vk,
                commission: rng.gen_range(0..10000),
            }
        }
    }

    impl Validator<BLSPubKey> {
        pub fn mock() -> Validator<BLSPubKey> {
            let val = TestValidator::random();
            let rng = &mut rand::thread_rng();
            let mut seed = [1u8; 32];
            rng.fill_bytes(&mut seed);
            let mut validator_stake = alloy::primitives::U256::from(0);
            let mut delegators = HashMap::new();
            for _i in 0..=5000 {
                let stake: u64 = rng.gen_range(0..10000);
                delegators.insert(Address::random(), alloy::primitives::U256::from(stake));
                validator_stake += alloy::primitives::U256::from(stake);
            }

            let stake_table_key = val.bls_vk.clone().into();
            let state_ver_key = val.schnorr_vk.clone().into();

            Validator {
                account: val.account,
                stake_table_key,
                state_ver_key,
                stake: validator_stake,
                commission: val.commission,
                delegators,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::Address;
    use sequencer_utils::test_utils::setup_test;

    use super::*;
    use crate::v0::impls::testing::*;

    #[test]
    fn test_from_l1_events() -> anyhow::Result<()> {
        setup_test();
        // Build a stake table with one DA node and one consensus node.
        let val = TestValidator::random();
        let val_new_keys = TestValidator::random();
        let delegator = Address::random();
        let mut events: Vec<StakeTableEvent> = [
            ValidatorRegistered {
                account: val.account,
                blsVk: val.bls_vk.clone().into(),
                schnorrVk: val.schnorr_vk.clone().into(),
                commission: val.commission,
            }
            .into(),
            Delegated {
                delegator,
                validator: val.account,
                amount: U256::from(10),
            }
            .into(),
            ConsensusKeysUpdated {
                account: val.account,
                blsVK: val_new_keys.bls_vk.clone().into(),
                schnorrVK: val_new_keys.schnorr_vk.clone().into(),
            }
            .into(),
            Undelegated {
                delegator,
                validator: val.account,
                amount: U256::from(7),
            }
            .into(),
            // delegate to the same validator again
            Delegated {
                delegator,
                validator: val.account,
                amount: U256::from(5),
            }
            .into(),
        ]
        .to_vec();

        let st = active_validator_set_from_l1_events(events.iter().cloned())?;
        let st_val = st.get(&val.account).unwrap();
        // final staked amount should be 10 (delegated) - 7 (undelegated) + 5 (Delegated)
        assert_eq!(st_val.stake, U256::from(8));
        assert_eq!(st_val.commission, val.commission);
        assert_eq!(st_val.delegators.len(), 1);
        // final delegated amount should be 10 (delegated) - 7 (undelegated) + 5 (Delegated)
        assert_eq!(*st_val.delegators.get(&delegator).unwrap(), U256::from(8));

        events.push(
            ValidatorExit {
                validator: val.account,
            }
            .into(),
        );

        // This should fail because the validator has exited and no longer exists in the stake table.
        assert!(active_validator_set_from_l1_events(events.iter().cloned()).is_err());

        Ok(())
    }

    #[test]
    fn test_from_l1_events_failures() -> anyhow::Result<()> {
        let val = TestValidator::random();
        let delegator = Address::random();

        let register: StakeTableEvent = ValidatorRegistered {
            account: val.account,
            blsVk: val.bls_vk.clone().into(),
            schnorrVk: val.schnorr_vk.clone().into(),
            commission: val.commission,
        }
        .into();
        let delegate: StakeTableEvent = Delegated {
            delegator,
            validator: val.account,
            amount: U256::from(10),
        }
        .into();
        let key_update: StakeTableEvent = ConsensusKeysUpdated {
            account: val.account,
            blsVK: val.bls_vk.clone().into(),
            schnorrVK: val.schnorr_vk.clone().into(),
        }
        .into();
        let undelegate: StakeTableEvent = Undelegated {
            delegator,
            validator: val.account,
            amount: U256::from(7),
        }
        .into();

        let exit: StakeTableEvent = ValidatorExit {
            validator: val.account,
        }
        .into();

        let cases = [
            vec![exit],
            vec![undelegate.clone()],
            vec![delegate.clone()],
            vec![key_update],
            vec![register.clone(), register.clone()],
            vec![register, delegate, undelegate.clone(), undelegate],
        ];

        for events in cases.iter() {
            let res = active_validator_set_from_l1_events(events.iter().cloned());
            assert!(
                res.is_err(),
                "events {:?}, not a valid sequencer of events",
                res
            );
        }
        Ok(())
    }

    #[test]
    fn test_validators_selection() {
        let mut validators = IndexMap::new();
        let mut highest_stake = alloy::primitives::U256::ZERO;

        for _i in 0..3000 {
            let validator = Validator::mock();
            validators.insert(validator.account, validator.clone());

            if validator.stake > highest_stake {
                highest_stake = validator.stake;
            }
        }

        let minimum_stake = highest_stake / U256::from(VID_TARGET_TOTAL_STAKE);

        select_active_validator_set(&mut validators).expect("Failed to select validators");
        assert!(
            validators.len() <= 100,
            "validators len is {}, expected at most 100",
            validators.len()
        );

        let mut selected_validators_highest_stake = alloy::primitives::U256::ZERO;
        // Ensure every validator in the final selection is above or equal to minimum stake
        for (address, validator) in &validators {
            assert!(
                validator.stake >= minimum_stake,
                "Validator {:?} has stake below minimum: {}",
                address,
                validator.stake
            );

            if validator.stake > selected_validators_highest_stake {
                selected_validators_highest_stake = validator.stake;
            }
        }
    }
}
