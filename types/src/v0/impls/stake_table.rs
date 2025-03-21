use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    num::NonZeroU64,
    sync::Arc,
};

use alloy::{
    primitives::{Address, U256},
    rpc::types::Log,
};
use anyhow::{bail, Context};
use async_lock::RwLock;
use committable::Committable;
use contract_bindings_alloy::staketable::StakeTable::{
    ConsensusKeysUpdated, Delegated, Undelegated, ValidatorExit, ValidatorRegistered,
};
use ethers_conv::{ToAlloy, ToEthers};
use hotshot::types::{BLSPubKey, SignatureKey as _};
use hotshot_contract_adapter::stake_table::{bls_alloy_to_jf2, edward_bn254point_to_state_ver};
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
use thiserror::Error;

use super::{
    traits::{MembershipPersistence, StateCatchup},
    v0_3::{DAMembers, Validator},
    v0_99::ChainConfig,
    Header, L1Client, Leaf2, PubKey, SeqTypes,
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
pub fn from_l1_events<I: Iterator<Item = StakeTableEvent>>(
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
                let stake_table_key = bls_alloy_to_jf2(blsVk.clone());
                let state_ver_key = edward_bn254point_to_state_ver(schnorrVk.clone());
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
                // Add delegator to the set
                validator_entry.delegators.insert(delegator, amount);
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
                let bls = bls_alloy_to_jf2(blsVK);
                let state_ver_key = edward_bn254point_to_state_ver(schnorrVK);

                validator.stake_table_key = bls;
                validator.state_ver_key = state_ver_key;
            },
        }
    }

    validate_validators(&mut validators)?;

    Ok(validators)
}

fn validate_validators(
    validators: &mut IndexMap<Address, Validator<BLSPubKey>>,
) -> anyhow::Result<()> {
    for (address, validator) in validators.clone() {
        // Ensure validator has at least one delegator
        if validator.delegators.is_empty() {
            tracing::info!("Validator {address:?} does not have any delegator");
            validators.shift_remove(&address);
        }

        // Ensure total stake is not zero
        if validator.stake.is_zero() {
            tracing::info!("Validator {address:?} does not have any stake");
            validators.shift_remove(&address);
        }
    }

    Ok(())
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

    // #[cfg(any(test, feature = "testing"))]
    // pub fn mock() -> Self {
    //     StakeTables::new(StakeTable::mock(3), DAMembers::mock(3))
    // }
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

    chain_config: ChainConfig,

    /// Randomized committees, filled when we receive the DrbResult
    randomized_committees: BTreeMap<Epoch, RandomizedCommittee<StakeTableEntry<PubKey>>>,

    /// Peers for catching up the stake table
    #[debug(skip)]
    peers: Arc<dyn StateCatchup>,

    /// Methods for stake table persistence.
    #[debug(skip)]
    persistence: Arc<dyn MembershipPersistence>,
}

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
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct EpochCommittee {
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
        let stake_table: IndexMap<BLSPubKey, PeerConfig<BLSPubKey>> = validators
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
        let eligible_leaders: Vec<PeerConfig<BLSPubKey>> =
            stake_table.clone().into_iter().map(|(_, l)| l).collect();
        let randomized_committee = generate_stake_cdf(
            stake_table
                .clone()
                .into_iter()
                .map(|(_, l)| l.stake_table_entry)
                .collect(),
            [0u8; 32],
        );
        self.randomized_committees
            .insert(epoch, randomized_committee);
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
        Ok(validators.get(&address).unwrap().clone())
    }

    // We need a constructor to match our concrete type.
    pub fn new_stake(
        // TODO remove `new` from trait and rename this to `new`.
        // https://github.com/EspressoSystems/HotShot/commit/fcb7d54a4443e29d643b3bbc53761856aef4de8b
        committee_members: Vec<PeerConfig<PubKey>>,
        da_members: Vec<PeerConfig<PubKey>>,
        l1_client: L1Client,
        chain_config: ChainConfig,
        peers: Arc<dyn StateCatchup>,
        persistence: impl MembershipPersistence,
    ) -> Self {
        // For each member, get the stake table entry
        let stake_table: Vec<_> = committee_members
            .iter()
            .filter(|&peer_config| {
                peer_config.stake_table_entry.stake() > ethers::types::U256::zero()
            })
            .cloned()
            .collect();

        // For each eligible leader, get the stake table entry
        let eligible_leaders = stake_table.clone();

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
            eligible_leaders: eligible_leaders.clone(),
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

        let randomized_committee = generate_stake_cdf(
            eligible_leaders
                .clone()
                .into_iter()
                .map(|l| l.stake_table_entry)
                .collect(),
            [0u8; 32],
        );
        let mut randomized_committees = BTreeMap::new();
        for epoch in Epoch::genesis().u64()..=2 {
            map.insert(Epoch::new(epoch), epoch_committee.clone());
            randomized_committees.insert(Epoch::new(epoch), randomized_committee.clone());
        }
        Self {
            non_epoch_committee: members,
            state: map,
            l1_client,
            chain_config,
            randomized_committees,
            peers,
            persistence: Arc::new(persistence),
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

    /// Get the stake table by epoch. Try to load from DB and fall back to fetching from l1.
    async fn get_stake_table_by_epoch(
        &self,
        epoch: Epoch,
        contract_address: Address,
        l1_block: u64,
    ) -> Result<IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>, GetStakeTablesError>
    {
        if let Some(stake_tables) = self
            .persistence
            .load_stake(epoch)
            .await
            .map_err(GetStakeTablesError::PersistenceLoadError)?
        {
            Ok(stake_tables)
        } else {
            self.l1_client
                .get_stake_table(contract_address, l1_block)
                .await
                .map_err(GetStakeTablesError::L1ClientFetchError)
        }
    }
}

#[derive(Error, Debug)]
/// Error representing fail cases for retrieving the stake table.
enum GetStakeTablesError {
    #[error("Error loading from persistence: {0}")]
    PersistenceLoadError(anyhow::Error),
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
        let chain_config = get_chain_config(self.chain_config, &self.peers, &block_header)
            .await
            .ok()?;

        let contract_address = chain_config.stake_table_contract;

        if contract_address.is_none() {
            tracing::error!("No stake table contract address found in Chain config");
        }

        let address = contract_address?;

        let stake_tables = self
            .get_stake_table_by_epoch(epoch, address.to_alloy(), block_header.height())
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

    fn has_epoch(&self, epoch: Epoch) -> bool {
        self.state.contains_key(&epoch)
    }

    async fn get_epoch_root_and_drb(
        membership: Arc<RwLock<Self>>,
        block_height: u64,
        epoch_height: u64,
        epoch: Epoch,
    ) -> anyhow::Result<(Header, DrbResult)> {
        let peers = membership.read().await.peers.clone();
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
        let epoch_committee = self.state.get(&Epoch::genesis()).unwrap().clone();
        self.state.insert(epoch, epoch_committee.clone());
        self.state.insert(epoch + 1, epoch_committee);
        self.add_drb_result(epoch, initial_drb_result);
        self.add_drb_result(epoch + 1, initial_drb_result);
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
#[cfg(any(test, feature = "testing"))]
impl super::v0_3::StakeTable {
    /// Generate a `StakeTable` with `n` members.
    pub fn mock(n: u64) -> Self {
        [..n]
            .iter()
            .map(|_| PeerConfig::default())
            .collect::<Vec<PeerConfig<PubKey>>>()
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
            .collect::<Vec<PeerConfig<PubKey>>>()
            .into()
    }
}

#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use contract_bindings_alloy::staketable::{EdOnBN254::EdOnBN254Point, BN254::G2Point};
    use ethers_conv::ToAlloy as _;
    use hotshot_contract_adapter::stake_table::{bls_jf_to_alloy2, ParsedEdOnBN254Point};
    use hotshot_types::light_client::StateKeyPair;
    use rand::{Rng as _, RngCore as _};

    use super::*;

    // TODO: current tests are just sanity checks, we need more.

    pub struct TestValidator {
        pub account: Address,
        pub bls_vk: G2Point,
        pub schnorr_vk: EdOnBN254Point,
        pub commission: u16,
    }

    impl TestValidator {
        pub fn random() -> Self {
            let rng = &mut rand::thread_rng();
            let mut seed = [0u8; 32];
            rng.fill_bytes(&mut seed);

            let (bls_vk, _) = BLSPubKey::generated_from_seed_indexed(seed, 0);
            let schnorr_vk: ParsedEdOnBN254Point =
                StateKeyPair::generate_from_seed_indexed(seed, 0)
                    .ver_key()
                    .to_affine()
                    .into();

            Self {
                account: Address::random(),
                bls_vk: bls_jf_to_alloy2(bls_vk),
                schnorr_vk: EdOnBN254Point {
                    x: schnorr_vk.x.to_alloy(),
                    y: schnorr_vk.y.to_alloy(),
                },
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
            for _i in 0..=100 {
                let stake: u64 = rng.gen_range(0..10000);
                delegators.insert(Address::random(), alloy::primitives::U256::from(stake));
                validator_stake += alloy::primitives::U256::from(stake);
            }

            let stake_table_key = bls_alloy_to_jf2(val.bls_vk.clone());
            let state_ver_key = edward_bn254point_to_state_ver(val.schnorr_vk.clone());

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
                blsVk: val.bls_vk.clone(),
                schnorrVk: val.schnorr_vk.clone(),
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
                blsVK: val_new_keys.bls_vk.clone(),
                schnorrVK: val_new_keys.schnorr_vk.clone(),
            }
            .into(),
            Undelegated {
                delegator,
                validator: val.account,
                amount: U256::from(7),
            }
            .into(),
        ]
        .to_vec();

        let st = from_l1_events(events.iter().cloned())?;
        let st_val = st.get(&val.account).unwrap();
        assert_eq!(st_val.stake, U256::from(3));
        assert_eq!(st_val.commission, val.commission);
        assert_eq!(st_val.delegators.len(), 1);
        assert_eq!(*st_val.delegators.get(&delegator).unwrap(), U256::from(3));

        events.push(
            ValidatorExit {
                validator: val.account,
            }
            .into(),
        );

        let st = from_l1_events(events.iter().cloned())?;
        assert_eq!(st.len(), 0);

        Ok(())
    }

    #[test]
    fn test_from_l1_events_failures() -> anyhow::Result<()> {
        let val = TestValidator::random();
        let delegator = Address::random();

        let register: StakeTableEvent = ValidatorRegistered {
            account: val.account,
            blsVk: val.bls_vk.clone(),
            schnorrVk: val.schnorr_vk.clone(),
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
            blsVK: val.bls_vk.clone(),
            schnorrVK: val.schnorr_vk.clone(),
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
            let res = from_l1_events(events.iter().cloned());
            assert!(
                res.is_err(),
                "events {:?}, not a valid sequencer of events",
                res
            );
        }
        Ok(())
    }
}
