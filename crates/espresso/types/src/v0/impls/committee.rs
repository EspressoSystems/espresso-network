use std::{
    collections::{BTreeMap, HashMap},
    ops::Bound,
    sync::Arc,
    time::Duration,
};

use alloy::primitives::{Address, U256};
use anyhow::{Context, bail};
use async_lock::Mutex as AsyncMutex;
use committable::Commitment;
use hotshot_types::{
    PeerConfig, PeerConnectInfo,
    data::{BlockNumber, EpochNumber, ViewNumber},
    drb::{
        DrbResult, INITIAL_DRB_RESULT,
        election::{RandomizedCommittee, generate_stake_cdf, select_randomized_leader},
    },
    epoch_membership::EpochMembershipCoordinator,
    signature_key::BLSPubKey,
    stake_table::StakeTableEntry,
    traits::{
        election::{
            EpochComponents, GenesisStakeTables, Membership, MembershipSnapshot,
            StakeTableAvailability,
        },
        signature_key::{SignatureKey as _, StakeTableEntryType},
    },
    utils::{epoch_from_block_number, root_block_in_epoch, transition_block_for_epoch},
};
#[cfg(feature = "node")]
use hotshot_types::{traits::block_contents::BlockHeader, utils::is_epoch_root};
use indexmap::IndexMap;
use parking_lot::RwLock;
use thiserror::Error;
use tracing::{debug, error, info, warn};
#[cfg(feature = "node")]
use versions::DRB_AND_HEADER_UPGRADE_VERSION;
use versions::EPOCH_VERSION;

use super::{
    AuthenticatedValidatorMap, RegisteredValidatorMap, StakeTableHash, StakeTableState,
    compute_block_reward,
};
use crate::{
    Header, Leaf2, PubKey, SeqTypes,
    traits::{MembershipPersistence, StakeTuple, StateCatchup},
    v0_3::{ASSUMED_BLOCK_TIME_SECONDS, AuthenticatedValidator, Fetcher, RewardAmount},
};

/// Number of recent per-epoch stake tables kept in memory.
///
/// `reload_stake` loads this many epochs from persistence on startup and
/// `prune_epochs` evicts anything older as the tip advances, so a
/// long-running node holds the same window as a freshly restarted one.
/// Evicted epochs are recovered on demand via the membership coordinator's
/// catchup.
pub const RECENT_STAKE_TABLES_LIMIT: u64 = 20;

/// Default for how long `load_epoch` may spend on each of its two
/// phases.
///
/// Catchup calls `load_epoch` for each epoch it walks and bounds each
/// call by its own `DEFAULT_CATCHUP_TIMEOUT` (300 s), so this is the tighter
/// of the two and is what a stalled store actually costs per epoch. Neither
/// bounds a walk as a whole.
const DEFAULT_STORAGE_READ_TIMEOUT: Duration = Duration::from_secs(60);

/// Type to describe DA and Stake memberships.
#[derive(Clone, Debug)]
pub struct EpochCommittees {
    inner: Arc<RwLock<Inner>>,
    fetcher: Arc<Fetcher>,
    #[cfg_attr(not(feature = "node"), allow(dead_code))]
    update_fixed_block_reward_lock: Arc<AsyncMutex<()>>,
    load_from_storage_lock: Arc<AsyncMutex<()>>,
    storage_read_timeout: Duration,
    epoch_height: BlockNumber,
}

#[derive(Debug)]
struct Inner {
    /// The stake tables of views before the first epoch.
    ///
    /// Built once at constructor time. `add_da_committee` does not change it.
    genesis: GenesisStakeTables<SeqTypes>,

    /// The DA committee of the epochs that no entry of `da_committees` covers.
    genesis_da_committee: Arc<DaCommittee>,

    /// Per-epoch snapshots.
    ///
    /// Each mutator rebuilds the snapshot for its affected epoch(s).
    snapshots: BTreeMap<EpochNumber, EpochSnapshot>,

    /// Holds the full validator candidate sets temporarily, until we store them.
    #[cfg_attr(not(feature = "node"), allow(dead_code))]
    all_validators: BTreeMap<EpochNumber, RegisteredValidatorMap>,

    /// DA committees, indexed by the first epoch in which they apply.
    ///
    /// Kept separate from `snapshots` because the lookup is a range query.
    da_committees: BTreeMap<EpochNumber, Arc<DaCommittee>>,

    /// Epochs below this have been evicted by `prune_epochs`.
    ///
    /// Only ever moves forward.
    prune_cutoff: EpochNumber,

    first_epoch: EpochNumber,

    /// Fixed block reward (used only in V3).
    ///
    /// Starting from V4, block reward is dynamic
    fixed_block_reward: Option<RewardAmount>,
}

#[derive(Clone, Debug)]
struct DaCommittee {
    committee: Vec<PeerConfig<SeqTypes>>,
    indexed_committee: HashMap<PubKey, PeerConfig<SeqTypes>>,
}

/// Holds Stake table and da stake
#[derive(Clone, Debug)]
struct EpochCommittee {
    /// The nodes eligible for leadership.
    ///
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<PeerConfig<SeqTypes>>,
    /// Keys for nodes participating in the network
    stake_table: IndexMap<PubKey, PeerConfig<SeqTypes>>,
    validators: AuthenticatedValidatorMap,
    address_mapping: HashMap<BLSPubKey, Address>,
    block_reward: Option<RewardAmount>,
    stake_table_hash: Option<StakeTableHash>,
    header: Option<Header>,
}

impl EpochCommittee {
    fn new(
        validators: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        hash: Option<StakeTableHash>,
        header: Option<Header>,
    ) -> Self {
        let mut address_mapping = HashMap::new();
        let stake_table: IndexMap<PubKey, PeerConfig<SeqTypes>> = validators
            .values()
            .map(|v| {
                let key = *v.stake_table_key();
                address_mapping.insert(key, v.account);
                (
                    key,
                    PeerConfig {
                        stake_table_entry: BLSPubKey::stake_table_entry(&key, v.stake),
                        state_ver_key: v.state_ver_key().clone(),
                        connect_info: v.x25519_key.and_then(|p| {
                            let a = v.p2p_addr.clone()?;
                            Some(PeerConnectInfo {
                                x25519_key: p,
                                p2p_addr: a,
                            })
                        }),
                    },
                )
            })
            .collect();

        let eligible_leaders: Vec<PeerConfig<SeqTypes>> =
            stake_table.iter().map(|(_, l)| l.clone()).collect();

        Self {
            eligible_leaders,
            stake_table,
            validators,
            address_mapping,
            block_reward,
            stake_table_hash: hash,
            header,
        }
    }
}

impl EpochCommittees {
    pub fn epoch_height(&self) -> BlockNumber {
        self.epoch_height
    }

    pub fn first_epoch(&self) -> EpochNumber {
        self.inner.read().first_epoch
    }

    pub fn fetcher(&self) -> &Fetcher {
        &self.fetcher
    }

    pub fn fixed_block_reward(&self) -> Option<RewardAmount> {
        self.inner.read().fixed_block_reward
    }

    /// Find the most recent stake-table entry for `key`.
    ///
    /// Scanns loaded epochs from highest to lowest and falling back to the
    /// genesis bootstrap committee.
    pub fn latest_peer_config(&self, key: &PubKey) -> Option<PeerConfig<SeqTypes>> {
        let inner = self.inner.read();
        for snap in inner.snapshots.values().rev() {
            if let Some(cfg) = snap.inner.committee.stake_table.get(key) {
                return Some(cfg.clone());
            }
        }
        inner.genesis.stake(key)
    }

    pub fn latest_account(&self, key: &PubKey) -> Option<Address> {
        let inner = self.inner.read();
        inner
            .snapshots
            .values()
            .rev()
            .find_map(|snap| snap.inner.committee.address_mapping.get(key).copied())
    }

    /// Fetch the fixed block reward and update it if its None.
    /// We used a fixed block reward for version v3
    /// Version v4 uses the dynamic block reward
    /// Assumes the stake table contract proxy address does not change
    #[cfg(feature = "node")]
    async fn fetch_and_update_fixed_block_reward(
        &self,
        epoch: EpochNumber,
    ) -> anyhow::Result<RewardAmount> {
        // Ensure there is only one `fetch_and_update_fixed_block_reward` at a time:
        let _guard = self.update_fixed_block_reward_lock.lock().await;

        // Clippy claims "temporary with significant `Drop` in `if let`
        // scrutinee will live until the end of the `if let` expression",
        // however this is incorrect. The 2024 edition changed the drop
        // scope of `if-let` expressions:
        //
        // https://doc.rust-lang.org/edition-guide/rust-2024/temporary-if-let-scope.html
        //
        // The read guard is dropped before `else`.
        #[allow(clippy::significant_drop_in_scrutinee)]
        if let Some(reward) = self.inner.read().fixed_block_reward {
            Ok(reward)
        } else {
            warn!(%epoch,
                "Block reward is None. attempting to fetch it from L1",
            );
            let block_reward =
                self.fetcher
                    .fetch_fixed_block_reward()
                    .await
                    .inspect_err(|err| {
                        error!(?epoch, ?err, "failed to fetch block_reward");
                    })?;
            self.inner.write().fixed_block_reward = Some(block_reward);
            Ok(block_reward)
        }
    }

    /// Calculates the dynamic block reward for a given block header within an epoch.
    ///
    /// The reward is based on a dynamic inflation rate computed from the current stake ratio (p),
    /// where `p = total_stake / total_supply`. The inflation function R(p) is defined piecewise:
    /// - If `p <= 0.01`: R(p) = 0.03 / sqrt(2 * 0.01)
    /// - Else: R(p) = 0.03 / sqrt(2 * p)
    async fn calculate_dynamic_block_reward(
        &self,
        epoch: EpochNumber,
        header: &Header,
        validators: &AuthenticatedValidatorMap,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
    ) -> anyhow::Result<Option<RewardAmount>> {
        let epoch_height = self.epoch_height;
        let current_epoch = epoch_from_block_number(header.height(), *epoch_height);
        let previous_epoch = current_epoch
            .checked_sub(1)
            .context("underflow: cannot get previous epoch when current_epoch is 0")?;
        debug!(?epoch, "previous_epoch={previous_epoch:?}");

        let first_epoch = *self.first_epoch();

        // Return early if previous epoch is not the first two epochs
        // and we don't have the stake table.
        if previous_epoch > first_epoch + 1 && self.snapshot(previous_epoch.into()).is_none() {
            warn!(?previous_epoch, "missing stake table for previous epoch");
            return Ok(None);
        }

        let previous_reward_distributed = header
            .total_reward_distributed()
            .context("Invalid block header: missing total_reward_distributed field")?;

        // Calculate total stake across all active validators
        let total_stake: U256 = validators.values().map(|v| v.stake).sum();
        let initial_supply = self.fetcher.initial_supply_or_fetch().await?;
        let total_supply = initial_supply
            .checked_add(previous_reward_distributed.0)
            .context("initial_supply + previous_reward_distributed overflow")?;

        // Calculate average block time over the last epoch
        let curr_ts = header.timestamp_millis_internal();
        debug!(?epoch, "curr_ts={curr_ts:?}");

        // If the node starts from epoch version V4, there is no previous epoch root available.
        // In this case, we assume a fixed average block time of 2000 milli seconds (2s)
        // for the first epoch in which reward id distributed
        let average_block_time_ms = if previous_epoch <= first_epoch + 1 {
            ASSUMED_BLOCK_TIME_SECONDS as u64 * 1000 // 2 seconds in milliseconds
        } else {
            // We are calculating rewards for epoch `epoch`, so the current epoch should be `epoch - 2`.
            // We need to calculate the average block time for the current epoch, so we need to know
            // the previous epoch root which is stored with epoch `epoch - 1`, i.e. the next epoch.
            let next_epoch = epoch
                .checked_sub(1)
                .context("underflow: cannot get next epoch when epoch is 0")?;
            self.load_epoch(EpochNumber::new(next_epoch), EpochComponents::SourceRoot)
                .await?;
            let prev_ts = match self.map_header(next_epoch, |h| h.timestamp_millis_internal()) {
                Some(ts) => ts,
                None => {
                    info!(
                        "Calculating rewards for epoch {}, we have no root leaf header for epoch \
                         - 1. Fetching from peers",
                        epoch
                    );

                    let root_height = header.height().checked_sub(*epoch_height).context(
                        "Epoch height is greater than block height. cannot compute previous epoch \
                         root height",
                    )?;

                    self.fetcher
                        .peers
                        .fetch_leaf(coordinator.clone(), root_height)
                        .await
                        .context("Epoch root leaf not found")?
                        .block_header()
                        .timestamp_millis_internal()
                },
            };

            let time_diff = curr_ts.checked_sub(prev_ts).context(
                "Current timestamp is earlier than previous. underflow in block time calculation",
            )?;

            time_diff
                .checked_div(*epoch_height)
                .context("Epoch height is zero. cannot compute average block time")?
        };
        info!(?epoch, %total_supply, %total_stake, %average_block_time_ms,
                       "dynamic block reward parameters");

        let block_reward =
            compute_block_reward(epoch, total_supply, total_stake, average_block_time_ms)?;

        Ok(Some(block_reward))
    }

    /// This function just returns the stored block reward in epoch committee
    pub fn epoch_block_reward(&self, epoch: EpochNumber) -> Option<RewardAmount> {
        self.inner
            .read()
            .epoch_committee(epoch)
            .and_then(|committee| committee.block_reward)
    }

    /// Get the index of a validator's BLS key in the epoch's stake table.
    /// Returns None if the validator is not in the stake table for this epoch.
    ///
    /// The index corresponds to the position in the `leader_counts` array in V6 headers.
    pub fn get_validator_index(&self, epoch: EpochNumber, bls_key: &PubKey) -> Option<usize> {
        self.inner
            .read()
            .epoch_committee(epoch)
            .and_then(|committee| committee.stake_table.get_index_of(bls_key))
    }

    pub fn active_validators(&self, e: EpochNumber) -> anyhow::Result<AuthenticatedValidatorMap> {
        self.inner.read().active_validators(e)
    }

    pub fn address(&self, e: EpochNumber, key: &BLSPubKey) -> anyhow::Result<Address> {
        self.inner.read().address(e, key)
    }

    pub fn get_validator_config(
        &self,
        epoch: EpochNumber,
        key: &BLSPubKey,
    ) -> anyhow::Result<AuthenticatedValidator<BLSPubKey>> {
        let inner = self.inner.read();
        let address = inner.address(epoch, key)?;
        let validators = inner.active_validators(epoch)?;
        validators
            .get(&address)
            .context("validator not found")
            .cloned()
    }

    /// The membership of a chain whose epochs start at `first_epoch`.
    ///
    /// `first_epoch` and `first_epoch + 1` run on the genesis stake table,
    /// `committee_members`, with the initial DRB result.
    ///
    /// # Errors
    ///
    /// Returns an error if the staked genesis members or the genesis DA
    /// members hold a key twice.
    pub fn new<B: Into<BlockNumber>>(
        committee_members: Vec<PeerConfig<SeqTypes>>,
        da_members: Vec<PeerConfig<SeqTypes>>,
        fixed_block_reward: Option<RewardAmount>,
        fetcher: Fetcher,
        epoch_height: B,
        first_epoch: EpochNumber,
    ) -> anyhow::Result<Self> {
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
        let indexed_da_members: HashMap<PubKey, _> = da_members
            .iter()
            .map(|peer_config| {
                (
                    PubKey::public_key(&peer_config.stake_table_entry),
                    peer_config.clone(),
                )
            })
            .collect();

        let genesis = GenesisStakeTables::new(stake_table.clone(), da_members.clone())?;
        let da_committee = Arc::new(DaCommittee {
            committee: da_members,
            indexed_committee: indexed_da_members,
        });

        let epoch_committee = Arc::new(EpochCommittee {
            eligible_leaders,
            stake_table: stake_table
                .iter()
                .map(|x| (PubKey::public_key(&x.stake_table_entry), x.clone()))
                .collect(),
            validators: Default::default(),
            address_mapping: HashMap::new(),
            block_reward: Default::default(),
            stake_table_hash: None,
            header: None,
        });

        let mut snapshots = BTreeMap::new();
        snapshots.insert(
            EpochNumber::genesis(),
            EpochSnapshot::new(
                EpochNumber::genesis(),
                first_epoch,
                epoch_committee.clone(),
                None,
                da_committee.clone(),
            ),
        );
        // TODO: remove this, workaround for hotshot asking for stake tables from epoch 1
        snapshots.insert(
            EpochNumber::genesis() + 1u64,
            EpochSnapshot::new(
                EpochNumber::genesis() + 1u64,
                first_epoch,
                epoch_committee,
                None,
                da_committee.clone(),
            ),
        );

        let committees = Self {
            inner: Arc::new(RwLock::new(Inner {
                genesis,
                genesis_da_committee: da_committee,
                da_committees: BTreeMap::new(),
                snapshots,
                all_validators: BTreeMap::new(),
                prune_cutoff: EpochNumber::genesis(),
                first_epoch,
                fixed_block_reward,
            })),
            fetcher: Arc::new(fetcher),
            update_fixed_block_reward_lock: Arc::new(AsyncMutex::new(())),
            load_from_storage_lock: Arc::new(AsyncMutex::new(())),
            storage_read_timeout: DEFAULT_STORAGE_READ_TIMEOUT,
            epoch_height: epoch_height.into(),
        };
        committees.seed_first_epoch(first_epoch);
        Ok(committees)
    }

    /// Override how long `load_epoch` may spend on each of its two
    /// phases. Mostly for tests.
    pub fn with_storage_read_timeout(mut self, timeout: Duration) -> Self {
        self.storage_read_timeout = timeout;
        self
    }

    #[cfg(feature = "node")]
    pub async fn reload_stake(&mut self, limit: u64) {
        match self.fetcher.fetch_fixed_block_reward().await {
            Ok(block_reward) => {
                info!("Fetched block reward: {block_reward}");
                self.inner.write().fixed_block_reward = Some(block_reward);
            },
            Err(err) => {
                warn!("Failed to fetch the block reward when reloading the stake tables: {err}");
            },
        }

        // Load the `limit` latest stored stake tables
        let loaded_stake = match self
            .fetcher
            .persistence
            .lock()
            .await
            .load_latest_stake(limit)
            .await
        {
            Ok(Some(loaded)) => loaded,
            Ok(None) => {
                warn!("No stake table history found in persistence!");
                return;
            },
            Err(e) => {
                error!("Failed to load stake table history from persistence: {e}");
                return;
            },
        };

        let first_epoch = self.inner.read().first_epoch;
        for (epoch, (validators, block_reward), stake_table_hash) in loaded_stake {
            // `new` seeded the epochs up to `first_epoch + 1` with the genesis stake table.
            if epoch <= first_epoch + 1 {
                continue;
            }
            let committee = EpochCommittee::new(validators, block_reward, stake_table_hash, None);
            self.inner
                .write()
                .put_epoch_committee(epoch, Arc::new(committee));
        }
    }

    async fn read_epoch_components(
        &self,
        persistence: &dyn MembershipPersistence,
        epoch: EpochNumber,
        components: EpochComponents,
    ) -> Result<(Option<StakeTuple>, Option<DrbResult>, Option<Header>), EpochCommitteesError> {
        let resident = self.snapshot(epoch);
        let stake = if resident.is_none() {
            persistence
                .load_stake(epoch)
                .await
                .map_err(|source| EpochCommitteesError::StakeTableStorage { epoch, source })?
        } else {
            None
        };
        if resident.is_none() && stake.is_none() {
            return Ok((None, None, None));
        }
        let drb = if matches!(
            components,
            EpochComponents::Leader | EpochComponents::AllPersisted
        ) && !resident.as_ref().is_some_and(|s| s.has_drb())
        {
            persistence
                .load_drb_result(epoch)
                .await
                .map_err(|source| EpochCommitteesError::DrbStorage { epoch, source })?
        } else {
            None
        };
        let header = if matches!(
            components,
            EpochComponents::SourceRoot | EpochComponents::AllPersisted
        ) && !resident
            .as_ref()
            .is_some_and(|s| s.inner.committee.header.is_some())
        {
            persistence
                .load_epoch_root(epoch)
                .await
                .map_err(|source| EpochCommitteesError::EpochRootStorage { epoch, source })?
        } else {
            None
        };
        Ok::<_, EpochCommitteesError>((stake, drb, header))
    }

    fn resident_components_present(&self, epoch: EpochNumber, components: EpochComponents) -> bool {
        self.snapshot(epoch)
            .is_some_and(|snapshot| match components {
                EpochComponents::StakeTable => true,
                EpochComponents::Leader => snapshot.has_drb(),
                EpochComponents::SourceRoot => snapshot.inner.committee.header.is_some(),
                EpochComponents::AllPersisted => {
                    snapshot.has_drb() && snapshot.inner.committee.header.is_some()
                },
            })
    }

    /// Get root leaf header for a given epoch
    fn map_header<E, F, R>(&self, epoch: E, f: F) -> Option<R>
    where
        E: Into<EpochNumber>,
        F: FnMut(&Header) -> R,
    {
        self.inner
            .read()
            .epoch_committee(epoch.into())
            .and_then(|committee| committee.header.as_ref().map(f))
    }

    /// Run `epoch` and `epoch + 1` on the genesis stake table with the initial DRB result.
    fn seed_first_epoch(&self, epoch: EpochNumber) {
        let rand_comm = Arc::new(
            self.randomized_committee(EpochNumber::genesis(), INITIAL_DRB_RESULT)
                .expect("committee exist at genesis"),
        );

        let mut inner = self.inner.write();
        let epoch_committee = inner
            .epoch_committee(EpochNumber::genesis())
            .expect("committee exists at genesis")
            .clone();

        // Build snapshots for `epoch` and `epoch + 1` with the genesis
        // stake table and the initial DRB result.
        inner.put_epoch_committee(epoch, epoch_committee.clone());
        inner.put_randomized_committee(epoch, rand_comm.clone());
        inner.put_epoch_committee(epoch + 1, epoch_committee);
        inner.put_randomized_committee(epoch + 1, rand_comm);
    }

    fn randomized_committee(
        &self,
        epoch: EpochNumber,
        drb: DrbResult,
    ) -> Option<RandomizedCommittee<StakeTableEntry<PubKey>>> {
        let inner = self.inner.read();
        let Some(raw_stake_table) = inner.epoch_committee(epoch) else {
            error!(
                "randomized_committee({epoch}, {drb:?}) was called, but we do not yet have the \
                 stake table for epoch {epoch}"
            );
            return None;
        };

        let leaders = raw_stake_table
            .eligible_leaders
            .clone()
            .into_iter()
            .map(|peer_config| peer_config.stake_table_entry)
            .collect::<Vec<_>>();

        Some(generate_stake_cdf(leaders, drb))
    }
}

/// returns the block reward for the given epoch.
///
/// Reward depends on the epoch root header version:
/// V3: Returns the fixed block reward as V3 only supports fixed reward
/// >= V4 : Returns the dynamic block reward
///
/// It also attempts catchup for the root header if not present in the committee,
/// and also for the stake table of the previous epoch
/// before computing the dynamic block reward
pub async fn fetch_and_calculate_block_reward(
    coordinator: EpochMembershipCoordinator<SeqTypes>,
    current_epoch: EpochNumber,
) -> anyhow::Result<RewardAmount> {
    let committee;
    let first_epoch;
    let fixed_block_reward;
    {
        let membership = coordinator.membership().inner.read();
        fixed_block_reward = membership.fixed_block_reward;

        committee = membership
            .epoch_committee(current_epoch)
            .context(format!("committee not found for epoch={current_epoch:?}"))?
            .clone();

        // Return early if committee has a reward already
        if let Some(reward) = committee.block_reward {
            return Ok(reward);
        }

        first_epoch = membership.first_epoch;
    }

    if *current_epoch <= *first_epoch + 1 {
        bail!(
            "epoch is in first two epochs: current_epoch={current_epoch}, \
             first_epoch={first_epoch}"
        );
    }

    if committee.header.is_none() {
        coordinator
            .membership()
            .load_epoch(current_epoch, EpochComponents::SourceRoot)
            .await?;
    }
    let header = match committee.header.clone().or_else(|| {
        coordinator
            .membership()
            .map_header(current_epoch, Clone::clone)
    }) {
        Some(header) => header,
        None => {
            let root_epoch = current_epoch.checked_sub(2).context(format!(
                "Epoch calculation underflow (current_epoch={current_epoch})"
            ))?;

            info!(?root_epoch, "catchup epoch root header");

            let leaf = coordinator
                .get_epoch_root(EpochNumber::new(root_epoch))
                .await
                .with_context(|| format!("Failed to get epoch root for root_epoch={root_epoch}"))?;
            leaf.block_header().clone()
        },
    };

    if header.version() <= EPOCH_VERSION {
        return fixed_block_reward.context(format!(
            "Fixed block reward not found for current_epoch={current_epoch}"
        ));
    }

    let prev_epoch_u64 = current_epoch.checked_sub(1).context(format!(
        "Underflow: cannot compute previous epoch when current_epoch={current_epoch}"
    ))?;

    let prev_epoch = EpochNumber::new(prev_epoch_u64);

    // If the previous epoch is not in the first two epochs,
    // there should be a stake table for it
    if *prev_epoch > *first_epoch + 1
        && let Err(err) = coordinator.stake_table_for_epoch(prev_epoch)
    {
        info!("failed to get membership for epoch={prev_epoch:?}: {err:#}");

        coordinator
            .wait_for_catchup(prev_epoch)
            .await
            .context(format!("failed to catch up for epoch={prev_epoch}"))?;
    }

    coordinator
        .membership()
        .calculate_dynamic_block_reward(current_epoch, &header, &committee.validators, &coordinator)
        .await
        .with_context(|| {
            format!("dynamic block reward calculation failed for epoch={current_epoch}")
        })?
        .with_context(|| format!("dynamic block reward returned None. epoch={current_epoch}"))
}

impl Membership<SeqTypes> for EpochCommittees {
    type Error = EpochCommitteesError;
    type Snapshot = EpochSnapshot;

    fn snapshot(&self, epoch: EpochNumber) -> Option<Self::Snapshot> {
        self.inner.read().snapshots.get(&epoch).cloned()
    }

    async fn load_epoch(
        &self,
        epoch: EpochNumber,
        components: EpochComponents,
    ) -> Result<StakeTableAvailability, Self::Error> {
        if self.resident_components_present(epoch, components) {
            return Ok(StakeTableAvailability::Present);
        }
        let locks = tokio::time::timeout(self.storage_read_timeout, async {
            let guard = self.load_from_storage_lock.lock().await;
            let persistence = self.fetcher.persistence.lock().await;
            (guard, persistence)
        })
        .await;
        let (_guard, persistence) = match locks {
            Ok(locks) => locks,
            Err(_) if self.resident_components_present(epoch, components) => {
                return Ok(StakeTableAvailability::Present);
            },
            Err(_) => return Err(EpochCommitteesError::StorageLockTimeout(epoch)),
        };
        let read = tokio::time::timeout(
            self.storage_read_timeout,
            self.read_epoch_components(&*persistence, epoch, components),
        )
        .await;
        let (stake, drb, header) = match read {
            Ok(result) => result?,
            Err(_) if self.resident_components_present(epoch, components) => {
                return Ok(StakeTableAvailability::Present);
            },
            Err(_) => return Err(EpochCommitteesError::StorageReadTimeout(epoch)),
        };
        drop(persistence);
        let committee = stake.map(|(validators, reward, hash)| {
            Arc::new(EpochCommittee::new(validators, reward, hash, None))
        });
        Ok(self
            .inner
            .write()
            .hydrate_epoch(epoch, committee, drb, header))
    }

    fn genesis_stake_tables(&self) -> GenesisStakeTables<SeqTypes> {
        self.inner.read().genesis.clone()
    }

    /// Adds the epoch committee and block reward for a given epoch,
    /// either by fetching from L1 or using local state if available.
    /// It also calculates and stores the block reward based on header version.
    #[cfg(not(feature = "node"))]
    async fn add_epoch_root(
        &self,
        _block_header: Header,
        _coordinator: &EpochMembershipCoordinator<SeqTypes>,
    ) -> Result<(), Self::Error> {
        // Fetching stake table events for the new epoch requires an L1 client.
        unimplemented!("add_epoch_root requires the node feature");
    }

    /// Adds the epoch committee and block reward for a given epoch,
    /// either by fetching from L1 or using local state if available.
    /// It also calculates and stores the block reward based on header version.
    #[cfg(feature = "node")]
    async fn add_epoch_root(
        &self,
        block_header: Header,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
    ) -> Result<(), Self::Error> {
        let block_number = block_header.block_number();

        let epoch_height = *self.epoch_height;

        let epoch = EpochNumber::new(epoch_from_block_number(block_number, epoch_height) + 2);

        info!(?epoch, "adding epoch root. height={:?}", block_number);

        if !is_epoch_root(block_number, epoch_height) {
            error!(
                "`add_epoch_root` was called with a block header that was not the root block for \
                 an epoch. This should never happen. Header:\n\n{block_header:?}"
            );
            return Err(Self::Error::NoRootBlock(block_number.into()));
        }

        // Stored ahead of the early return below so a failed write is retried by the next
        // `add_epoch_root` for this epoch. Downstream of `put_epoch_committee` the committee
        // is complete, which arms that early return and also stops catchup from re-entering
        // `fetch_stake_table`, so one dropped error would leave the header unpersisted.
        //
        // Keyed by `epoch`, unlike the previous-epoch stake table stored further down, and
        // not gated behind the stake table consistency check, which doesn't apply here.
        if let Err(e) = self
            .fetcher
            .persistence
            .lock()
            .await
            .store_epoch_root(epoch, block_header.clone())
            .await
        {
            error!(?e, ?epoch, "`add_epoch_root`, error storing epoch root");
        }

        let version = block_header.version();
        // Update the chain config if the block header contains a newer one.
        self.fetcher
            .update_chain_config(&block_header)
            .await
            .map_err(Self::Error::Fetcher)?;

        let mut block_reward = None;
        // Even if the current header is the root of the epoch which falls in the post upgrade
        // we use the fixed block reward
        if version == EPOCH_VERSION {
            let reward = self
                .fetch_and_update_fixed_block_reward(epoch)
                .await
                .map_err(Self::Error::Fetcher)?;
            block_reward = Some(reward);
        }

        self.load_epoch(epoch, EpochComponents::SourceRoot).await?;

        let epoch_committee = self.inner.read().epoch_committee(epoch).cloned();

        // TODO: If the stake table is missing should it be fetched by an unbounded
        // number of tasks?

        // If the epoch committee:
        // - exists and has a header stake table hash and block reward, return early.
        // - exists without a reward, reuse validators and update reward.
        // and fetch from L1 if the stake table hash is missing.
        // - doesn't exist, fetch it from L1.
        let (active_validators, all_validators, stake_table_hash) = match epoch_committee {
            Some(committee)
                if committee.block_reward.is_some()
                    && committee.header.is_some()
                    && committee.stake_table_hash.is_some() =>
            {
                info!(
                    ?epoch,
                    "committee already has block reward, header, and stake table hash; skipping \
                     add_epoch_root"
                );
                return Ok(());
            },

            Some(committee) => {
                if let Some(reward) = committee.block_reward {
                    block_reward = Some(reward);
                }

                if let Some(hash) = committee.stake_table_hash {
                    (committee.validators.clone(), Default::default(), Some(hash))
                } else {
                    // if stake table hash is missing then recalculate from events
                    info!(
                        "Stake table hash missing for epoch {epoch}. recalculating by fetching \
                         from l1."
                    );
                    let set = self
                        .fetcher
                        .fetch(epoch, &block_header)
                        .await
                        .map_err(Self::Error::Fetcher)?;
                    (
                        set.active_validators,
                        set.all_validators,
                        set.stake_table_hash,
                    )
                }
            },

            None => {
                info!("Stake table missing for epoch {epoch}. Fetching from L1.");
                let set = self
                    .fetcher
                    .fetch(epoch, &block_header)
                    .await
                    .map_err(Self::Error::Fetcher)?;
                (
                    set.active_validators,
                    set.all_validators,
                    set.stake_table_hash,
                )
            },
        };

        // If we are past the DRB+Header upgrade point,
        // and don't have block reward
        // calculate the dynamic block reward based on validator info and block header.
        if block_reward.is_none() && version >= DRB_AND_HEADER_UPGRADE_VERSION {
            info!(?epoch, "calculating dynamic block reward");
            let reward = self
                .calculate_dynamic_block_reward(
                    epoch,
                    &block_header,
                    &active_validators,
                    coordinator,
                )
                .await
                .map_err(Self::Error::Reward)?;

            info!(?epoch, "calculated dynamic block reward = {reward:?}");
            block_reward = reward;
        }

        let committee = EpochCommittee::new(
            active_validators.clone(),
            block_reward,
            stake_table_hash,
            Some(block_header.clone()),
        );

        let previous_epoch;
        let previous_committee;
        let previous_validators;
        {
            let mut inner = self.inner.write();
            inner.put_epoch_committee(epoch, Arc::new(committee));
            // previous_epoch is the epoch prior to `epoch`,
            // or the epoch immediately succeeding the block header
            previous_epoch = EpochNumber::new(epoch.saturating_sub(1));
            previous_committee = inner.epoch_committee(previous_epoch).cloned();
            // garbage collect the validator set
            inner.all_validators = inner.all_validators.split_off(&previous_epoch);
            // extract `all_validators` for the previous epoch
            previous_validators = inner.all_validators.remove(&previous_epoch);
            inner.all_validators.insert(epoch, all_validators.clone());
            inner.prune_epochs(epoch);
        }

        let persistence_lock = self.fetcher.persistence.lock().await;

        let decided_hash = block_header.next_stake_table_hash();

        // we store the information from the previous epoch's in-memory committeee
        // if the decided stake_table_hash is consistent with what we get
        //
        // in principle this is unnecessary and we could've stored these right away,
        // without offsetting the epoch. but the intention is to catch L1 provider issues
        // if there is a mismatch
        if let Some(previous_committee) = previous_committee {
            if decided_hash.is_none() || decided_hash == previous_committee.stake_table_hash {
                if let Err(e) = persistence_lock
                    .store_stake(
                        previous_epoch,
                        previous_committee.validators.clone(),
                        previous_committee.block_reward,
                        previous_committee.stake_table_hash,
                    )
                    .await
                {
                    error!(
                        ?e,
                        ?previous_epoch,
                        "`add_epoch_root`, error storing stake table"
                    );
                }

                if let Some(previous_validators) = previous_validators
                    && let Err(e) = persistence_lock
                        .store_all_validators(previous_epoch, previous_validators)
                        .await
                {
                    error!(?e, ?epoch, "`add_epoch_root`, error storing all validators");
                }
            } else {
                panic!(
                    "The decided block header's `next_stake_table_hash` does not match the hash \
                     of the stake table we have. This is an unrecoverable error likely due to \
                     issues with your L1 RPC provider. Decided:\n\n{:?}Actual:\n\n{:?}",
                    decided_hash, previous_committee.stake_table_hash
                );
            }
        }

        Ok(())
    }

    async fn get_epoch_root(
        &self,
        epoch: EpochNumber,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
    ) -> Result<Leaf2, Self::Error> {
        let block_height = root_block_in_epoch(*epoch, *self.epoch_height());
        let peers = self.fetcher.peers.clone();

        // the root block may not exist anywhere yet
        // Each attempt tries all peers once
        // `retry` only scales the per peer timeout
        for retry in 0..3 {
            match peers
                .try_fetch_leaf(retry, coordinator.clone(), block_height)
                .await
            {
                Ok(leaf) => return Ok(leaf),
                Err(err) => {
                    warn!(%epoch, block_height, retry, "failed to fetch epoch root leaf: {err:#}");
                },
            }
        }
        Err(Self::Error::Catchup(anyhow::anyhow!(
            "failed to fetch epoch root leaf for epoch {epoch} (block {block_height}) from peers"
        )))
    }

    async fn get_epoch_drb(
        &self,
        epoch: EpochNumber,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
    ) -> Result<DrbResult, Self::Error> {
        let peers = self.fetcher.peers.clone();

        // Try to retrieve the DRB result from an existing snapshot's randomized committee.
        if let Some(snap) = self.snapshot(epoch)
            && let Some(rand) = &snap.inner.randomized
        {
            return Ok(rand.drb_result());
        }

        // Otherwise, we try to fetch the epoch root leaf
        let previous_epoch = match epoch.checked_sub(1) {
            Some(epoch) => EpochNumber::new(epoch),
            None => {
                return self
                    .snapshot(epoch)
                    .and_then(|s| s.inner.randomized.as_ref().map(|r| r.drb_result()))
                    .ok_or_else(|| {
                        Self::Error::Message(format!(
                            "Missing randomized committee for epoch {epoch}"
                        ))
                    });
            },
        };

        let block_height = transition_block_for_epoch(*previous_epoch, *self.epoch_height());

        debug!(
            "Getting DRB for epoch {}, block height {}",
            epoch, block_height
        );
        let drb_leaf = peers
            .try_fetch_leaf(1, coordinator.clone(), block_height)
            .await
            .map_err(Self::Error::Catchup)?;

        let Some(drb) = drb_leaf.next_drb_result else {
            error!(
                "We received a leaf that should contain a DRB result, but the DRB result is \
                 missing: {:?}",
                drb_leaf
            );

            return Err(Self::Error::Message(
                "DRB leaf is missing the DRB result.".to_string(),
            ));
        };

        Ok(drb)
    }

    fn add_drb_result(&self, epoch: EpochNumber, drb: DrbResult) {
        info!("Adding DRB result {drb:?} to epoch {epoch}");
        if let Some(committee) = self.randomized_committee(epoch, drb) {
            self.inner
                .write()
                .put_randomized_committee(epoch, Arc::new(committee));
        }
    }

    fn first_epoch(&self) -> EpochNumber {
        self.inner.read().first_epoch
    }

    fn highest_known_epoch(&self) -> Option<EpochNumber> {
        self.inner.read().snapshots.keys().max().copied()
    }

    fn add_da_committee(&self, first_epoch: EpochNumber, committee: Vec<PeerConfig<SeqTypes>>) {
        let indexed_committee: HashMap<PubKey, _> = committee
            .iter()
            .map(|peer_config| {
                (
                    PubKey::public_key(&peer_config.stake_table_entry),
                    peer_config.clone(),
                )
            })
            .collect();

        let da_committee = Arc::new(DaCommittee {
            committee,
            indexed_committee,
        });

        let mut inner = self.inner.write();
        inner
            .da_committees
            .insert(first_epoch, da_committee.clone());

        // The DA committee inserted at `first_epoch` applies to every epoch
        // up to (but not including) the next `da_committees` key. Snapshots
        // for those epochs were built with whatever DA was current at the
        // time and must be rebuilt so reads of `da_stake_table()` etc.
        // reflect the new committee.
        let upper = inner
            .da_committees
            .range((Bound::Excluded(first_epoch), Bound::Unbounded))
            .next()
            .map(|(k, _)| *k);

        let range = if let Some(u) = upper {
            (Bound::Included(first_epoch), Bound::Excluded(u))
        } else {
            (Bound::Included(first_epoch), Bound::Unbounded)
        };

        let affected: Vec<EpochNumber> = inner.snapshots.range(range).map(|(k, _)| *k).collect();
        let first_epoch_field = inner.first_epoch;

        for epoch in affected {
            let Some(existing) = inner.snapshots.get(&epoch) else {
                continue;
            };
            let new_snapshot = EpochSnapshot::new(
                epoch,
                first_epoch_field,
                existing.inner.committee.clone(),
                existing.inner.randomized.clone(),
                da_committee.clone(),
            );
            inner.snapshots.insert(epoch, new_snapshot);
        }
    }
}

#[derive(Error, Debug)]
pub enum EpochCommitteesError {
    #[error("failed to load persisted stake table for epoch {epoch}: {source}")]
    StakeTableStorage {
        epoch: EpochNumber,
        #[source]
        source: anyhow::Error,
    },

    #[error("failed to load persisted DRB for epoch {epoch}: {source}")]
    DrbStorage {
        epoch: EpochNumber,
        #[source]
        source: anyhow::Error,
    },

    #[error("failed to load persisted epoch root for epoch {epoch}: {source}")]
    EpochRootStorage {
        epoch: EpochNumber,
        #[source]
        source: anyhow::Error,
    },

    #[error("membership storage lock acquisition timed out for epoch {0}")]
    StorageLockTimeout(EpochNumber),

    #[error("membership component read timed out for epoch {0}")]
    StorageReadTimeout(EpochNumber),

    #[error("could not lookup leader")]
    LeaderLookupError,

    #[error("block {0} is not the root block for an epoch")]
    NoRootBlock(BlockNumber),

    #[error("fetcher error: {0}")]
    Fetcher(#[source] anyhow::Error),

    #[error("{0}")]
    Message(String),

    #[error("state catchup error: {0}")]
    Catchup(#[source] anyhow::Error),

    #[error("reward calculation error: {0}")]
    Reward(#[source] anyhow::Error),
}

impl Inner {
    /// Merge against the state at installation time: storage I/O may have
    /// overlapped a decided root, DA update, or a newer supplied DRB.
    fn hydrate_epoch(
        &mut self,
        epoch: EpochNumber,
        loaded: Option<Arc<EpochCommittee>>,
        drb: Option<DrbResult>,
        header: Option<Header>,
    ) -> StakeTableAvailability {
        let Some(committee) = self.epoch_committee(epoch).cloned().or(loaded) else {
            return StakeTableAvailability::Absent;
        };
        let committee = if committee.header.is_none() && header.is_some() {
            Arc::new(EpochCommittee {
                header,
                ..(*committee).clone()
            })
        } else {
            committee
        };
        self.put_epoch_committee(epoch, committee.clone());
        if !self.snapshots[&epoch].has_drb()
            && let Some(drb) = drb
        {
            let leaders = committee
                .eligible_leaders
                .iter()
                .map(|peer| peer.stake_table_entry.clone())
                .collect();
            self.put_randomized_committee(epoch, Arc::new(generate_stake_cdf(leaders, drb)));
        }
        StakeTableAvailability::Present
    }

    /// The DA committee that applies to `epoch`, or the genesis DA committee
    /// when no explicit DA committee covers it.
    fn resolve_da_committee(&self, epoch: EpochNumber) -> Arc<DaCommittee> {
        // The greatest key ≤ `epoch` is the DA committee that applies.
        self.da_committees
            .range((Bound::Included(0.into()), Bound::Included(epoch)))
            .last()
            .map(|(_, committee)| committee.clone())
            .unwrap_or_else(|| self.genesis_da_committee.clone())
    }

    /// Borrow the per-epoch `EpochCommittee` if loaded.
    fn epoch_committee(&self, e: EpochNumber) -> Option<&Arc<EpochCommittee>> {
        self.snapshots.get(&e).map(|s| &s.inner.committee)
    }

    fn address(&self, e: EpochNumber, key: &BLSPubKey) -> anyhow::Result<Address> {
        self.epoch_committee(e)
            .context("state for found")?
            .address_mapping
            .get(key)
            .copied()
            .context(format!(
                "failed to get ethereum address for bls key {key}. epoch={e}"
            ))
    }

    fn active_validators(&self, e: EpochNumber) -> anyhow::Result<AuthenticatedValidatorMap> {
        Ok(self
            .epoch_committee(e)
            .context("state not found")?
            .validators
            .clone())
    }

    /// Rebuild (or insert) the snapshot for `epoch` carrying forward
    /// `randomized` from any existing snapshot for that epoch.
    fn put_epoch_committee(&mut self, epoch: EpochNumber, committee: Arc<EpochCommittee>) {
        let randomized = self
            .snapshots
            .get(&epoch)
            .and_then(|s| s.inner.randomized.clone());
        let da_committee = self.resolve_da_committee(epoch);
        let first_epoch = self.first_epoch;
        self.snapshots.insert(
            epoch,
            EpochSnapshot::new(epoch, first_epoch, committee, randomized, da_committee),
        );
    }

    /// Rebuild the snapshot for `epoch` with a new randomized committee,
    /// carrying forward the existing committee/da. No-op if no snapshot
    /// for `epoch` exists yet.
    fn put_randomized_committee(
        &mut self,
        epoch: EpochNumber,
        randomized: Arc<RandomizedCommittee<StakeTableEntry<PubKey>>>,
    ) {
        let Some(existing) = self.snapshots.get(&epoch).cloned() else {
            return;
        };
        let committee = existing.inner.committee.clone();
        let da_committee = existing.inner.da_committee.clone();
        let first_epoch = self.first_epoch;
        self.snapshots.insert(
            epoch,
            EpochSnapshot::new(
                epoch,
                first_epoch,
                committee,
                Some(randomized),
                da_committee,
            ),
        );
    }

    /// Evict snapshots more than [`RECENT_STAKE_TABLES_LIMIT`] epochs below
    /// `added`.
    ///
    /// The cutoff only ever moves forward: `add_epoch_root` also runs for old
    /// epochs during catchup, and a lagging insert must neither evict newer
    /// state nor evict itself before its readers had a chance to observe it.
    ///
    /// `da_committees` is exempt: it holds one entry per configured DA
    /// committee (seeded from config at startup and on upgrade certificates),
    /// so it is bounded, and `resolve_da_committee`'s greatest-key-below
    /// lookup needs the full history for re-inserted old epochs.
    #[cfg_attr(not(feature = "node"), allow(dead_code))]
    fn prune_epochs(&mut self, added: EpochNumber) {
        let cutoff = EpochNumber::new(added.saturating_sub(RECENT_STAKE_TABLES_LIMIT));
        if cutoff <= self.prune_cutoff {
            return;
        }
        self.prune_cutoff = cutoff;
        // The snapshots for `first_epoch` and `first_epoch + 1` (seeded by
        // `new`) are never evicted:
        let keep = self.first_epoch + 1u64;
        self.snapshots.retain(|e, _| *e <= keep || *e >= cutoff);
    }
}

/// A consistent per-epoch view of `EpochCommittees`.
///
/// Returned by [`Membership::snapshot`].
#[derive(Clone, Debug)]
pub struct EpochSnapshot {
    inner: Arc<EpochSnapshotInner>,
}

#[derive(Debug)]
struct EpochSnapshotInner {
    epoch: EpochNumber,
    first_epoch: EpochNumber,
    committee: Arc<EpochCommittee>,
    randomized: Option<Arc<RandomizedCommittee<StakeTableEntry<PubKey>>>>,
    da_committee: Arc<DaCommittee>,
}

impl EpochSnapshot {
    fn new(
        epoch: EpochNumber,
        first_epoch: EpochNumber,
        committee: Arc<EpochCommittee>,
        randomized: Option<Arc<RandomizedCommittee<StakeTableEntry<PubKey>>>>,
        da_committee: Arc<DaCommittee>,
    ) -> Self {
        Self {
            inner: Arc::new(EpochSnapshotInner {
                epoch,
                first_epoch,
                committee,
                randomized,
                da_committee,
            }),
        }
    }
}

impl EpochSnapshot {
    /// Index of `key` in this epoch's stake table, if present.
    pub fn validator_index(&self, key: &PubKey) -> Option<usize> {
        self.inner.committee.stake_table.get_index_of(key)
    }

    /// The full validator record (account, stake, delegators, etc.) for `key`.
    pub fn validator_config(
        &self,
        key: &BLSPubKey,
    ) -> anyhow::Result<&AuthenticatedValidator<BLSPubKey>> {
        let address = self
            .inner
            .committee
            .address_mapping
            .get(key)
            .context(format!(
                "failed to get ethereum address for bls key {key}. epoch={}",
                self.inner.epoch
            ))?;
        self.inner
            .committee
            .validators
            .get(address)
            .context("validator not found")
    }

    pub fn epoch_block_reward(&self) -> Option<RewardAmount> {
        self.inner.committee.block_reward
    }

    pub fn validators(&self) -> &AuthenticatedValidatorMap {
        &self.inner.committee.validators
    }
}

impl MembershipSnapshot<SeqTypes> for EpochSnapshot {
    type Error = EpochCommitteesError;
    type StakeTableHash = StakeTableState;

    fn epoch(&self) -> EpochNumber {
        self.inner.epoch
    }

    fn first_epoch(&self) -> EpochNumber {
        self.inner.first_epoch
    }

    fn has_drb(&self) -> bool {
        self.inner.randomized.is_some()
    }

    fn stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<SeqTypes>> + Send {
        self.inner.committee.stake_table.values()
    }

    fn da_stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<SeqTypes>> + Send {
        self.inner.da_committee.committee.iter()
    }

    fn committee_members(&self, _: ViewNumber) -> impl ExactSizeIterator<Item = &PubKey> + Send {
        self.inner.committee.stake_table.keys()
    }

    fn da_committee_members(&self, _: ViewNumber) -> impl ExactSizeIterator<Item = &PubKey> + Send {
        self.inner.da_committee.indexed_committee.keys()
    }

    fn stake(&self, key: &PubKey) -> Option<PeerConfig<SeqTypes>> {
        self.inner.committee.stake_table.get(key).cloned()
    }

    fn da_stake(&self, key: &PubKey) -> Option<PeerConfig<SeqTypes>> {
        self.inner.da_committee.indexed_committee.get(key).cloned()
    }

    fn has_stake(&self, key: &PubKey) -> bool {
        self.stake(key)
            .map(|x| x.stake_table_entry.stake() > U256::ZERO)
            .unwrap_or_default()
    }

    fn has_da_stake(&self, key: &PubKey) -> bool {
        self.da_stake(key)
            .map(|x| x.stake_table_entry.stake() > U256::ZERO)
            .unwrap_or_default()
    }

    /// Returns the leader's public key for a given view number in this epoch.
    ///
    /// # Errors
    ///
    /// Returns `LeaderLookupError` if the snapshot's epoch is before
    /// `first_epoch`, the randomized committee is missing, or the epoch has
    /// no eligible leader.
    fn lookup_leader(&self, view: ViewNumber) -> Result<PubKey, Self::Error> {
        let inner = &self.inner;
        let first_epoch = inner.first_epoch;
        if inner.epoch < first_epoch {
            error!(
                "leader requested for epoch {} before first epoch {first_epoch}",
                inner.epoch,
            );
            return Err(EpochCommitteesError::LeaderLookupError);
        }
        let Some(rand) = inner.randomized.as_deref() else {
            error!(
                "missing randomized committee for epoch {} in snapshot",
                inner.epoch,
            );
            return Err(EpochCommitteesError::LeaderLookupError);
        };
        // `select_randomized_leader` panics on an empty committee. `new` seeds
        // one when the genesis stake table has no staked member.
        if inner.committee.eligible_leaders.is_empty() {
            error!("no eligible leader for epoch {}", inner.epoch);
            return Err(EpochCommitteesError::LeaderLookupError);
        }
        Ok(PubKey::public_key(&select_randomized_leader(rand, *view)))
    }

    fn stake_table_hash(&self) -> Option<Commitment<Self::StakeTableHash>> {
        self.inner.committee.stake_table_hash
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    use committable::Committable;
    use hotshot_query_service::testing::mocks::MOCK_UPGRADE;
    use hotshot_types::{
        ValidatorConfig,
        traits::{BlockPayload, block_contents::BlockHeader},
    };
    use rstest::rstest;
    use tokio::{task::JoinSet, time::Duration};

    use super::*;
    use crate::{NodeState, Payload, Transaction, traits::MembershipPersistence};

    /// Wall-clock target each concurrency test runs for. Long enough to
    /// catch flaky races that one-shot tests would miss; short enough to
    /// be tolerable in CI.
    const TEST_DURATION: Duration = Duration::from_secs(5);

    fn build_committees(num_peers: u64) -> EpochCommittees {
        build_committees_with_fetcher(num_peers, Fetcher::mock(), EpochNumber::genesis())
    }

    fn build_committees_with_fetcher(
        num_peers: u64,
        fetcher: Fetcher,
        first_epoch: EpochNumber,
    ) -> EpochCommittees {
        let peers: Vec<PeerConfig<SeqTypes>> = (0..num_peers)
            .map(|i| {
                ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                    [42u8; 32],
                    i,
                    U256::from(100),
                    true,
                )
                .public_config()
            })
            .collect();
        EpochCommittees::new(peers.clone(), peers, None, fetcher, 100u64, first_epoch)
            .expect("genesis without duplicate keys")
    }

    /// Persistence stub holding stake tables for epochs 7 and 8, plus a DRB
    /// result and an epoch root header for epoch 7 only. `stored_headers`
    /// records headers written via `store_epoch_root`, independent of
    /// `header`. `fail_next_store` makes the next `store_epoch_root` fail
    /// once, then clears itself.
    #[derive(Debug, Default)]
    struct ReadControl {
        stake_reads: AtomicUsize,
        drb_reads: AtomicUsize,
        root_reads: AtomicUsize,
        fail_stake: AtomicBool,
        fail_root: AtomicBool,
        drb_gate: Option<Arc<tokio::sync::Notify>>,
        drb_started: tokio::sync::Notify,
    }

    #[derive(Debug)]
    struct MockStakeStore {
        control: Arc<ReadControl>,
        header: Header,
        stored_headers: std::sync::Mutex<HashMap<EpochNumber, Header>>,
        fail_next_store: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl crate::traits::MembershipPersistence for MockStakeStore {
        async fn load_stake(
            &self,
            epoch: EpochNumber,
        ) -> anyhow::Result<Option<crate::traits::StakeTuple>> {
            self.control.stake_reads.fetch_add(1, Ordering::SeqCst);
            if self.control.fail_stake.load(Ordering::SeqCst) {
                anyhow::bail!("injected stake read failure");
            }
            Ok((*epoch == 7 || *epoch == 8).then(|| (Default::default(), None, None)))
        }

        async fn load_latest_stake(
            &self,
            _limit: u64,
        ) -> anyhow::Result<Option<Vec<crate::v0_3::IndexedStake>>> {
            Ok(None)
        }

        async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
            self.control.drb_reads.fetch_add(1, Ordering::SeqCst);
            self.control.drb_started.notify_one();
            if let Some(gate) = &self.control.drb_gate {
                gate.notified().await;
            }
            Ok((*epoch == 7).then_some([7; 32]))
        }

        async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
            self.control.root_reads.fetch_add(1, Ordering::SeqCst);
            if self.control.fail_root.load(Ordering::SeqCst) {
                anyhow::bail!("injected root read failure");
            }
            if let Some(header) = self.stored_headers.lock().unwrap().get(&epoch) {
                return Ok(Some(header.clone()));
            }
            Ok((*epoch == 7).then(|| self.header.clone()))
        }

        async fn store_epoch_root(
            &self,
            epoch: EpochNumber,
            block_header: Header,
        ) -> anyhow::Result<()> {
            if self.fail_next_store.swap(false, Ordering::SeqCst) {
                anyhow::bail!("injected store_epoch_root failure");
            }
            self.stored_headers
                .lock()
                .unwrap()
                .insert(epoch, block_header);
            Ok(())
        }

        async fn store_stake(
            &self,
            _epoch: EpochNumber,
            _stake: AuthenticatedValidatorMap,
            _block_reward: Option<RewardAmount>,
            _stake_table_hash: Option<StakeTableHash>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_events(
            &self,
            _l1_finalized: u64,
            _events: Vec<(crate::v0_3::EventKey, crate::v0_3::StakeTableEvent)>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn load_events(
            &self,
            _from_l1_block: u64,
            _l1_finalized: u64,
        ) -> anyhow::Result<(
            Option<crate::traits::EventsPersistenceRead>,
            Vec<(crate::v0_3::EventKey, crate::v0_3::StakeTableEvent)>,
        )> {
            Ok((None, vec![]))
        }

        async fn delete_stake_tables(&self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_all_validators(
            &self,
            _epoch: EpochNumber,
            _all_validators: IndexMap<Address, crate::v0_3::RegisteredValidator<PubKey>>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn load_all_validators(
            &self,
            _epoch: EpochNumber,
            _offset: u64,
            _limit: u64,
        ) -> anyhow::Result<Vec<crate::v0_3::RegisteredValidator<PubKey>>> {
            Ok(vec![])
        }
    }

    async fn committees_with_read_control(control: Arc<ReadControl>) -> EpochCommittees {
        let store = MockStakeStore {
            control,
            header: build_epoch_root_header().await,
            stored_headers: Default::default(),
            fail_next_store: Default::default(),
        };
        let fetcher = Fetcher::new(
            Arc::new(crate::mock::MockStateCatchup::default()),
            Arc::new(AsyncMutex::new(store)),
            crate::L1Client::new(vec!["http://localhost:3331".parse().unwrap()]).unwrap(),
            crate::ChainConfig::default(),
        );
        build_committees_with_fetcher(4, fetcher, EpochNumber::genesis())
    }

    #[tokio::test]
    async fn table_ensure_and_pure_reads_do_not_wait_for_legacy_drb_catchup() {
        use hotshot_types::epoch_membership::ResidentEpochReadiness;

        let gate = Arc::new(tokio::sync::Notify::new());
        let control = Arc::new(ReadControl {
            drb_gate: Some(gate.clone()),
            fail_root: AtomicBool::new(true),
            ..Default::default()
        });
        let committees = committees_with_read_control(control.clone()).await;
        let coordinator = EpochMembershipCoordinator::<SeqTypes>::new(
            committees,
            100u64,
            &hotshot_example_types::storage_types::TestStorage::default(),
        );
        let epoch = EpochNumber::new(7);
        assert!(coordinator.snapshot(epoch).is_none());
        assert!(coordinator.leader_snapshot(epoch).is_none());
        assert_eq!(
            coordinator.resident_readiness(epoch),
            ResidentEpochReadiness::Unavailable
        );
        tokio::task::yield_now().await;
        assert_eq!(control.stake_reads.load(Ordering::SeqCst), 0);
        assert_eq!(control.drb_reads.load(Ordering::SeqCst), 0);

        let table = coordinator.ensure_stake_table(epoch).await.unwrap();
        assert!(!table.has_drb());
        assert_eq!(control.drb_reads.load(Ordering::SeqCst), 0);
        assert_eq!(control.root_reads.load(Ordering::SeqCst), 0);
        assert!(coordinator.membership_for_epoch(epoch).is_err());
        tokio::time::timeout(Duration::from_secs(1), control.drb_started.notified())
            .await
            .unwrap();
        assert_eq!(
            coordinator.resident_readiness(epoch),
            ResidentEpochReadiness::StakeTable
        );
        tokio::time::timeout(
            Duration::from_millis(100),
            coordinator.ensure_stake_table(epoch),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            tokio::time::timeout(
                Duration::from_millis(20),
                coordinator.ensure_leader_membership(epoch)
            )
            .await
            .is_err()
        );
        assert!(coordinator.leader_snapshot(epoch).is_none());

        gate.notify_one();
        let leader = coordinator.ensure_leader_membership(epoch).await.unwrap();
        assert!(leader.snapshot().has_drb());
        assert!(!table.has_drb());
        assert_eq!(control.drb_reads.load(Ordering::SeqCst), 1);
        // A source-root storage error must not prevent restoring a saved DRB.
        assert_eq!(control.root_reads.load(Ordering::SeqCst), 0);
        assert!(matches!(
            coordinator
                .membership()
                .load_epoch(epoch, EpochComponents::AllPersisted)
                .await,
            Err(EpochCommitteesError::EpochRootStorage { .. })
        ));
    }

    #[tokio::test]
    async fn persisted_absence_failure_and_timeout_remain_distinct() {
        let control = Arc::new(ReadControl::default());
        let committees = committees_with_read_control(control.clone())
            .await
            .with_storage_read_timeout(Duration::from_millis(20));
        let epoch = EpochNumber::new(9);
        assert_eq!(
            committees
                .load_epoch(epoch, EpochComponents::StakeTable)
                .await
                .unwrap(),
            StakeTableAvailability::Absent
        );
        control.fail_stake.store(true, Ordering::SeqCst);
        assert!(matches!(
            committees
                .load_epoch(epoch, EpochComponents::StakeTable)
                .await,
            Err(EpochCommitteesError::StakeTableStorage { .. })
        ));
        control.fail_stake.store(false, Ordering::SeqCst);
        let _busy = committees.fetcher.persistence.lock().await;
        assert!(matches!(
            committees
                .load_epoch(epoch, EpochComponents::StakeTable)
                .await,
            Err(EpochCommitteesError::StorageLockTimeout(timed_out)) if timed_out == epoch
        ));
    }

    #[tokio::test]
    async fn add_epoch_root_does_not_wait_for_persisted_drb() {
        let control = Arc::new(ReadControl {
            drb_gate: Some(Arc::new(tokio::sync::Notify::new())),
            ..Default::default()
        });
        let committees = committees_with_read_control(control.clone()).await;
        let epoch = EpochNumber::new(3);
        {
            let mut inner = committees.inner.write();
            let template = inner.epoch_committee(EpochNumber::genesis()).unwrap();
            let committee = EpochCommittee {
                block_reward: Some(RewardAmount::default()),
                stake_table_hash: Some(StakeTableState::default().commit()),
                header: None,
                ..(**template).clone()
            };
            inner.put_epoch_committee(epoch, Arc::new(committee));
        }
        let coordinator = EpochMembershipCoordinator::<SeqTypes>::new(
            committees,
            100u64,
            &hotshot_example_types::storage_types::TestStorage::default(),
        );
        let header = build_epoch_root_header().await;
        tokio::time::timeout(
            Duration::from_millis(100),
            coordinator.add_epoch_root(header),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(!coordinator.snapshot(epoch).unwrap().has_drb());
        assert_eq!(control.drb_reads.load(Ordering::SeqCst), 0);
        assert_eq!(control.root_reads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn source_root_hydration_does_not_require_drb() {
        let control = Arc::new(ReadControl {
            drb_gate: Some(Arc::new(tokio::sync::Notify::new())),
            ..Default::default()
        });
        let committees = committees_with_read_control(control.clone()).await;
        let epoch = EpochNumber::new(7);
        committees
            .load_epoch(epoch, EpochComponents::StakeTable)
            .await
            .unwrap();
        tokio::time::timeout(
            Duration::from_millis(100),
            committees.load_epoch(epoch, EpochComponents::SourceRoot),
        )
        .await
        .unwrap()
        .unwrap();
        let restored = committees.snapshot(epoch).unwrap();
        assert!(restored.inner.committee.header.is_some());
        assert!(!restored.has_drb());
        assert_eq!(control.drb_reads.load(Ordering::SeqCst), 0);
        assert_eq!(control.root_reads.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn stalled_drb_read_returns_typed_timeout_without_changing_table_readiness() {
        let control = Arc::new(ReadControl {
            drb_gate: Some(Arc::new(tokio::sync::Notify::new())),
            ..Default::default()
        });
        let committees = committees_with_read_control(control)
            .await
            .with_storage_read_timeout(Duration::from_millis(20));
        let epoch = EpochNumber::new(7);
        committees
            .load_epoch(epoch, EpochComponents::StakeTable)
            .await
            .unwrap();
        assert!(
            matches!(committees.load_epoch(epoch, EpochComponents::Leader).await,
            Err(EpochCommitteesError::StorageReadTimeout(timed_out)) if timed_out == epoch)
        );
        assert!(!committees.snapshot(epoch).unwrap().has_drb());
        assert_eq!(
            committees
                .load_epoch(epoch, EpochComponents::StakeTable)
                .await
                .unwrap(),
            StakeTableAvailability::Present
        );
    }

    #[tokio::test]
    async fn persisted_hydration_preserves_concurrent_table_drb_and_da_updates() {
        let gate = Arc::new(tokio::sync::Notify::new());
        let control = Arc::new(ReadControl {
            drb_gate: Some(gate.clone()),
            ..Default::default()
        });
        let committees = committees_with_read_control(control.clone()).await;
        let epoch = EpochNumber::new(7);
        committees
            .load_epoch(epoch, EpochComponents::StakeTable)
            .await
            .unwrap();
        let before = committees.snapshot(epoch).unwrap();
        let load = tokio::spawn({
            let committees = committees.clone();
            async move {
                committees
                    .load_epoch(epoch, EpochComponents::AllPersisted)
                    .await
            }
        });
        tokio::time::timeout(Duration::from_secs(1), control.drb_started.notified())
            .await
            .unwrap();
        // Replacing the table while storage awaits must preserve its new order.
        let replacement = build_committees(3)
            .inner
            .read()
            .epoch_committee(EpochNumber::genesis())
            .unwrap()
            .clone();
        committees
            .inner
            .write()
            .put_epoch_committee(epoch, replacement.clone());
        committees.add_drb_result(epoch, [9; 32]);
        committees.add_da_committee(epoch, vec![]);
        let concurrent = committees.snapshot(epoch).unwrap();
        gate.notify_one();
        assert_eq!(
            load.await.unwrap().unwrap(),
            StakeTableAvailability::Present
        );
        let hydrated = committees.snapshot(epoch).unwrap();
        assert!(Arc::ptr_eq(
            &hydrated.inner.randomized.as_ref().unwrap(),
            &concurrent.inner.randomized.as_ref().unwrap()
        ));
        assert_eq!(
            hydrated.inner.randomized.as_ref().unwrap().drb_result(),
            [9; 32]
        );
        assert_eq!(
            hydrated.stake_table().collect::<Vec<_>>(),
            concurrent.stake_table().collect::<Vec<_>>()
        );
        assert_eq!(hydrated.da_total_nodes(), 0);
        assert!(hydrated.inner.committee.header.is_some());
        assert!(!before.has_drb());
        assert!(before.inner.committee.header.is_none());
    }

    // `load_stake_table` materializes a snapshot from the persisted stake
    // table (e.g. after eviction by `prune_epochs`) without network catchup,
    // including the randomized committee and the epoch root header when
    // they are persisted.
    #[tokio::test]
    async fn restore_stake_table_from_persistence() {
        let store = MockStakeStore {
            control: Default::default(),
            header: build_epoch_root_header().await,
            stored_headers: Default::default(),
            fail_next_store: Default::default(),
        };
        let fetcher = Fetcher::new(
            Arc::new(crate::mock::MockStateCatchup::default()),
            Arc::new(AsyncMutex::new(store)),
            crate::L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            crate::ChainConfig::default(),
        );
        let committees = build_committees_with_fetcher(4, fetcher, EpochNumber::genesis());

        // Epoch 7 has a stake table, a DRB result and a root header in storage.
        let seven = EpochNumber::new(7);
        assert!(committees.snapshot(seven).is_none());
        assert!(committees.load_stake_table(seven).await);
        let snapshot = committees.snapshot(seven).unwrap();
        assert!(snapshot.has_drb());
        assert!(snapshot.inner.committee.header.is_some());
        // A second call is satisfied by the in-memory snapshot.
        assert!(committees.load_stake_table(seven).await);
        // Epoch 8 has a stake table but no DRB result and no root header.
        let eight = EpochNumber::new(8);
        assert!(committees.load_stake_table(eight).await);
        let snapshot = committees.snapshot(eight).unwrap();
        assert!(!snapshot.has_drb());
        assert!(snapshot.inner.committee.header.is_none());
        // Nothing persisted for epoch 9.
        assert!(!committees.load_stake_table(EpochNumber::new(9)).await);
    }

    #[tokio::test]
    async fn restore_missing_components_of_resident_stake_table() {
        let store = MockStakeStore {
            control: Default::default(),
            header: build_epoch_root_header().await,
            stored_headers: Default::default(),
            fail_next_store: Default::default(),
        };
        let fetcher = Fetcher::new(
            Arc::new(crate::mock::MockStateCatchup::default()),
            Arc::new(AsyncMutex::new(store)),
            crate::L1Client::new(vec!["http://localhost:3331".parse().unwrap()]).unwrap(),
            crate::ChainConfig::default(),
        );
        let committees = build_committees_with_fetcher(4, fetcher, EpochNumber::genesis());
        let epoch = EpochNumber::new(7);
        assert_eq!(
            committees
                .load_epoch(epoch, EpochComponents::StakeTable)
                .await
                .unwrap(),
            StakeTableAvailability::Present,
        );
        let captured = committees.snapshot(epoch).unwrap();
        assert!(!captured.has_drb());
        assert!(captured.inner.committee.header.is_none());

        assert!(committees.load_stake_table(epoch).await);
        let restored = committees.snapshot(epoch).unwrap();
        assert!(restored.has_drb());
        assert!(restored.inner.committee.header.is_some());
        assert!(!captured.has_drb());
        assert!(captured.inner.committee.header.is_none());
        assert_eq!(
            captured.stake_table().collect::<Vec<_>>(),
            restored.stake_table().collect::<Vec<_>>()
        );
    }

    // A caller that times out queueing for the storage locks must still
    // answer `true` when the epoch was loaded by someone else while it
    // queued: `load_stake_table`'s contract is "is a snapshot available",
    // and a false here sends catchup to peers for an epoch already in
    // memory. The held lock models the persistence lock's other users (the
    // L1 event fetcher) keeping it busy past the lock-phase budget.
    #[rstest]
    #[case(true)]
    #[case(false)]
    #[tokio::test]
    async fn won_race_reported_loaded_when_lock_phase_times_out(#[case] legacy: bool) {
        let store = MockStakeStore {
            control: Default::default(),
            header: build_epoch_root_header().await,
            stored_headers: Default::default(),
            fail_next_store: Default::default(),
        };
        let persistence = Arc::new(AsyncMutex::new(store));
        let fetcher = Fetcher::new(
            Arc::new(crate::mock::MockStateCatchup::default()),
            Arc::clone(&persistence) as Arc<AsyncMutex<dyn MembershipPersistence>>,
            crate::L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            crate::ChainConfig::default(),
        );
        let committees = build_committees_with_fetcher(4, fetcher, EpochNumber::genesis())
            .with_storage_read_timeout(Duration::from_millis(400));

        // Hold the persistence lock for the whole test, well past the
        // lock-phase budget.
        let _busy = persistence.lock().await;

        // The loader passes its entry checks (epoch 7 not loaded yet) and
        // parks queueing on the held persistence lock.
        let seven = EpochNumber::new(7);
        let loader = tokio::spawn({
            let committees = committees.clone();
            async move {
                if legacy {
                    committees.load_stake_table(seven).await
                } else {
                    committees
                        .load_epoch(seven, EpochComponents::StakeTable)
                        .await
                        .unwrap()
                        == StakeTableAvailability::Present
                }
            }
        });
        // Give the loader time to run its entry checks and park; inserting
        // before it enters would satisfy the entry check and prove nothing.
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Another task wins the race: epoch 7's snapshot appears while the
        // loader is still queued.
        {
            let mut inner = committees.inner.write();
            let template = inner
                .epoch_committee(EpochNumber::genesis())
                .expect("genesis committee exists")
                .clone();
            inner.put_epoch_committee(seven, template);
        }

        let loaded = loader.await.expect("loader panicked");
        assert!(
            loaded,
            "load_stake_table returned false for an epoch whose snapshot was loaded while it \
             queued for the persistence lock: catchup would re-fetch from peers what is already \
             in memory"
        );
    }

    // `prune_epochs` keeps a bounded window behind the newest decided epoch
    // plus the first-epoch seeds (catchup anchors), and its cutoff never
    // regresses when an older epoch is (re-)inserted during catchup.
    #[test]
    fn prune_epochs_keeps_recent_window() {
        let committees = build_committees(4);
        let mut inner = committees.inner.write();
        inner.first_epoch = EpochNumber::new(1);
        let template = inner
            .epoch_committee(EpochNumber::genesis())
            .expect("genesis committee exists")
            .clone();

        for e in 2..=50 {
            inner.put_epoch_committee(EpochNumber::new(e), template.clone());
            inner.prune_epochs(EpochNumber::new(e));
        }

        // The seeds for `first_epoch` and `first_epoch + 1` survive, then
        // nothing until the cutoff.
        let cutoff = EpochNumber::new(50 - RECENT_STAKE_TABLES_LIMIT);
        let expected: Vec<EpochNumber> = [1, 2]
            .into_iter()
            .chain(*cutoff..=50)
            .map(EpochNumber::new)
            .collect();
        assert_eq!(
            inner.snapshots.keys().copied().collect::<Vec<_>>(),
            expected
        );

        // A catchup insert below the cutoff is not evicted by its own
        // insertion; it goes once the tip advances past it.
        inner.put_epoch_committee(EpochNumber::new(5), template);
        inner.prune_epochs(EpochNumber::new(5));
        assert!(inner.snapshots.contains_key(&EpochNumber::new(5)));
        inner.prune_epochs(EpochNumber::new(51));
        assert!(!inner.snapshots.contains_key(&EpochNumber::new(5)));
    }

    // Concurrent reads must not panic or deadlock while a writer drives
    // real mutations on the same `Inner` lock.
    //
    // Per-call invariants (within a single method invocation) are
    // checked. Cross-call invariants are *not*: each public method
    // takes its own short-lived lock, so a sequence of two read calls
    // observes two snapshots in time and the writer can run between
    // them. See the `EpochCommittees` doc-comment for the rationale.
    //
    // To make the contention real, we pre-populate `inner.state` for
    // several extra epochs so the writer's `add_drb_result` calls
    // actually take the write lock. Without this they would early-exit
    // on the missing-state branch and never contend.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_reads_during_mutations() {
        let committees = build_committees(8);

        // Pre-populate snapshots for epochs 2..6 by cloning the genesis
        // committee. `add_drb_result(e)` is a no-op when no snapshot for
        // `e` exists, so without this the writer never takes the write
        // lock for those epochs.
        {
            let mut inner = committees.inner.write();
            let template = inner
                .epoch_committee(EpochNumber::genesis())
                .expect("genesis committee exists")
                .clone();
            for e in 2..6 {
                inner.put_epoch_committee(EpochNumber::new(e), template.clone());
            }
        }

        let stop = Arc::new(AtomicBool::new(false));
        let mut tasks = JoinSet::new();

        for _ in 0..8 {
            let c = committees.clone();
            let stop = Arc::clone(&stop);
            tasks.spawn(async move {
                let stable = EpochNumber::new(1);
                let mutating = EpochNumber::new(3);
                let view = ViewNumber::new(0);
                while !stop.load(Ordering::Relaxed) {
                    // Stable epoch — the writer never touches
                    // `inner.state[1]` or `inner.randomized_committees[1]`
                    // (both were set by `new` before this
                    // loop and stay unchanged thereafter), so the
                    // assertions below hold even across separate snapshots.
                    let stable_snap = c.snapshot(stable).expect("stable snapshot");
                    let len = stable_snap.stake_table().len();
                    assert_eq!(len, stable_snap.total_nodes());
                    let leader = stable_snap.lookup_leader(view).expect("leader");
                    assert!(
                        stable_snap.committee_members(view).any(|p| p == &leader),
                        "leader {leader:?} not in committee_members for stable epoch",
                    );
                    assert_eq!(c.first_epoch(), stable);

                    // Mutating epoch — the writer churns
                    // `randomized_committees[3]`. Just exercise the API
                    // path; the value can vary or be transiently absent
                    // between calls and that is the documented
                    // behaviour, not a bug.
                    let _ = c.snapshot(mutating);
                    if let Some(s) = c.snapshot(mutating) {
                        let _ = s.lookup_leader(view);
                    }
                    tokio::task::yield_now().await;
                }
            });
        }

        // Writer driving real mutations against fields the readers see.
        // Loops until the test signals stop, so the contention window
        // matches `TEST_DURATION`.
        tasks.spawn({
            let c = committees.clone();
            let stop = Arc::clone(&stop);
            async move {
                let extra: Vec<PeerConfig<SeqTypes>> = (0..3)
                    .map(|i| {
                        ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                            [99u8; 32],
                            i,
                            U256::from(50),
                            true,
                        )
                        .public_config()
                    })
                    .collect();
                let mut i: u64 = 0;
                while !stop.load(Ordering::Relaxed) {
                    // Pre-populated epochs 2..5 — these acquire the
                    // write lock and contend with reader read locks.
                    c.add_drb_result(EpochNumber::new(2 + (i % 4)), [(i % 256) as u8; 32]);
                    // Non-existent epoch — exercises the read-then-no-op
                    // branch of `add_drb_result` (read lock only).
                    c.add_drb_result(EpochNumber::new(10_000 + i), [0xAB; 32]);
                    if i.is_multiple_of(50) {
                        c.add_da_committee(i.into(), extra.clone());
                    }
                    if i.is_multiple_of(16) {
                        tokio::task::yield_now().await;
                    }
                    i += 1;
                }
            }
        });

        tokio::time::sleep(TEST_DURATION).await;
        stop.store(true, Ordering::Relaxed);
        while let Some(res) = tasks.join_next().await {
            res.expect("task panicked");
        }
    }

    // Many writer tasks hammer `add_drb_result` for the same epoch with
    // distinct DRBs. While they do, reader tasks call `lookup_leader`,
    // which must always succeed once the randomized committee is set —
    // the writer overwrites the entry but never removes it. After the
    // writers drain, the entry must still be present and queryable.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_add_drb_result_same_epoch() {
        let committees = build_committees(4);
        let epoch = EpochNumber::new(1);

        let stop = Arc::new(AtomicBool::new(false));
        let writes = Arc::new(AtomicUsize::new(0));
        let lookups = Arc::new(AtomicUsize::new(0));

        let mut writers = JoinSet::new();
        let mut readers = JoinSet::new();

        // Readers: lookup_leader must always succeed for an epoch
        // whose randomized committee has been populated, even while it
        // is being overwritten.
        for _ in 0..4 {
            let c = committees.clone();
            let stop = Arc::clone(&stop);
            let lookups = Arc::clone(&lookups);
            readers.spawn(async move {
                let view = ViewNumber::new(0);
                while !stop.load(Ordering::Relaxed) {
                    c.snapshot(epoch)
                        .expect("snapshot")
                        .lookup_leader(view)
                        .expect("randomized committee must remain present once set");
                    lookups.fetch_add(1, Ordering::Relaxed);
                    tokio::task::yield_now().await;
                }
            });
        }

        // Writers: each task overwrites the randomized committee with
        // a unique DRB derived from its task id and iteration. Loops
        // until stop so the contention window matches `TEST_DURATION`.
        for tid in 0..8u8 {
            let c = committees.clone();
            let stop = Arc::clone(&stop);
            let writes = Arc::clone(&writes);
            writers.spawn(async move {
                let mut i: u64 = 0;
                while !stop.load(Ordering::Relaxed) {
                    let mut drb = [tid; 32];
                    drb[0] = (i & 0xFF) as u8;
                    c.add_drb_result(epoch, drb);
                    writes.fetch_add(1, Ordering::Relaxed);
                    if i.is_multiple_of(16) {
                        tokio::task::yield_now().await;
                    }
                    i += 1;
                }
            });
        }

        tokio::time::sleep(TEST_DURATION).await;
        stop.store(true, Ordering::Relaxed);
        while let Some(res) = writers.join_next().await {
            res.expect("writer panicked");
        }
        while let Some(res) = readers.join_next().await {
            res.expect("reader panicked");
        }

        assert!(writes.load(Ordering::Relaxed) > 0, "writers never advanced",);
        assert!(
            lookups.load(Ordering::Relaxed) > 0,
            "readers never observed the randomized committee",
        );
        let snap = committees
            .snapshot(epoch)
            .expect("randomized committee must survive concurrent writes");
        assert!(snap.has_drb(), "randomized committee must remain present");
        let view = ViewNumber::new(0);
        let _leader = snap
            .lookup_leader(view)
            .expect("lookup_leader succeeds when randomized committee is present");
    }

    // Build an epoch-root header for `epoch_height = 100`. Block height
    // 95 satisfies `is_epoch_root(95, 100)` and produces target epoch 3
    // when passed to `add_epoch_root`.
    async fn build_epoch_root_header() -> Header {
        let instance = NodeState::mock_v2();
        let tx = Transaction::of_size(10);
        let (payload, _) = Payload::from_transactions(&[tx], &instance.genesis_state, &instance)
            .await
            .expect("payload");
        let metadata = payload.ns_table().clone();
        let header = Header::genesis(&instance, payload, &metadata, MOCK_UPGRADE.base);
        match header {
            Header::V2(mut h) => {
                h.height = 95;
                Header::V2(h)
            },
            other => panic!("expected V2 header from NodeState::mock_v2, got {other:?}"),
        }
    }

    // `add_epoch_root` mutates `state[epoch]` and `all_validators[epoch]`
    // inside one `inner.write()` block. A reader observing both fields
    // under one read lock must see them flip together: pre-state
    // (`state[epoch].header == None`, `all_validators[epoch]` absent)
    // or post-state (`header == Some(_)`, `all_validators[epoch]`
    // present). A torn snapshot in either direction would indicate the
    // mutations leaked outside the single write lock.
    //
    // We pre-populate `state[epoch]` with `block_reward` and
    // `stake_table_hash` set so `add_epoch_root` reuses the validators
    // already in memory and skips the L1 fetch (the mock fetcher
    // points at a non-existent RPC endpoint and would fail). This
    // still drives the inner.write block we want to verify.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn add_epoch_root_is_atomic() {
        let header = build_epoch_root_header().await;
        let target = EpochNumber::new(3);

        fn snapshot(c: &EpochCommittees, e: EpochNumber) -> (bool, bool) {
            let inner = c.inner.read();
            let header_set = inner
                .snapshots
                .get(&e)
                .map(|s| s.inner.committee.header.is_some())
                .unwrap_or(false);
            let all_validators_present = inner.all_validators.contains_key(&e);
            (header_set, all_validators_present)
        }

        // Each round is one race attempt against `add_epoch_root`. Loop
        // for `TEST_DURATION` to widen the window for catching a torn
        // observation across the inner.write block.
        let test_start = tokio::time::Instant::now();
        let mut round: u64 = 0;
        while test_start.elapsed() < TEST_DURATION {
            let committees = build_committees(4);

            // Pre-populate the snapshot for `target` with block_reward and
            // stake_table_hash but no header. This lands `add_epoch_root`
            // on the second match arm (no L1 fetch) but still drives
            // the inner.write() mutation.
            {
                let mut inner = committees.inner.write();
                let template = inner
                    .epoch_committee(EpochNumber::genesis())
                    .expect("genesis committee exists");
                let prefilled = EpochCommittee {
                    block_reward: Some(RewardAmount::default()),
                    stake_table_hash: Some(StakeTableState::default().commit()),
                    header: None,
                    eligible_leaders: template.eligible_leaders.clone(),
                    stake_table: template.stake_table.clone(),
                    validators: template.validators.clone(),
                    address_mapping: template.address_mapping.clone(),
                };
                inner.put_epoch_committee(target, Arc::new(prefilled));
            }

            let stop = Arc::new(AtomicBool::new(false));
            let post = Arc::new(AtomicUsize::new(0));

            let reader = {
                let c = committees.clone();
                let stop = Arc::clone(&stop);
                let post = Arc::clone(&post);
                tokio::spawn(async move {
                    while !stop.load(Ordering::Relaxed) {
                        match snapshot(&c, target) {
                            (false, false) => {}, // pre-state
                            (true, true) => {
                                post.fetch_add(1, Ordering::Relaxed);
                            },
                            torn => panic!(
                                "round {round}: torn snapshot for epoch {target}: header_set={}, \
                                 all_validators_present={}",
                                torn.0, torn.1,
                            ),
                        }
                        tokio::task::yield_now().await;
                    }
                })
            };

            // Brief warmup so the reader is in its loop before the
            // mutation lands.
            tokio::time::sleep(Duration::from_millis(2)).await;
            let coordinator = EpochMembershipCoordinator::<SeqTypes>::new(
                committees.clone(),
                *committees.epoch_height(),
                &hotshot_example_types::storage_types::TestStorage::default(),
            );
            coordinator
                .add_epoch_root(header.clone())
                .await
                .expect("add_epoch_root should succeed for the prefilled state");

            let deadline = tokio::time::Instant::now() + Duration::from_millis(200);
            while tokio::time::Instant::now() < deadline && post.load(Ordering::Relaxed) == 0 {
                tokio::task::yield_now().await;
            }
            stop.store(true, Ordering::Relaxed);
            reader.await.expect("reader panicked");
            assert!(
                post.load(Ordering::Relaxed) > 0,
                "round {round}: reader never observed post-state",
            );
            round += 1;
        }
        assert!(round > 0, "test loop never executed a round");
    }

    /// Build an `add_epoch_root` fixture for `header`'s target epoch. The
    /// epoch committee is prefilled so the call reuses the cached validators
    /// and skips the L1 fetch; `cached_header` decides whether that committee
    /// is already complete, i.e. whether the call takes the early return.
    fn epoch_root_fixture(
        header: &Header,
        cached_header: bool,
    ) -> (
        EpochCommittees,
        EpochMembershipCoordinator<SeqTypes>,
        EpochNumber,
        Arc<AtomicBool>,
    ) {
        let fail_next_store = Arc::<AtomicBool>::default();
        let store = MockStakeStore {
            control: Default::default(),
            header: header.clone(),
            stored_headers: Default::default(),
            fail_next_store: fail_next_store.clone(),
        };
        let fetcher = Fetcher::new(
            Arc::new(crate::mock::MockStateCatchup::default()),
            Arc::new(AsyncMutex::new(store)),
            crate::L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            crate::ChainConfig::default(),
        );
        let committees = build_committees_with_fetcher(4, fetcher, EpochNumber::genesis());
        let epoch_height = *committees.epoch_height();
        let epoch = EpochNumber::new(epoch_from_block_number(header.height(), epoch_height) + 2);

        {
            let mut inner = committees.inner.write();
            let template = inner
                .epoch_committee(EpochNumber::genesis())
                .expect("genesis committee exists");
            let prefilled = EpochCommittee {
                block_reward: Some(RewardAmount::default()),
                stake_table_hash: Some(StakeTableState::default().commit()),
                header: cached_header.then(|| header.clone()),
                eligible_leaders: template.eligible_leaders.clone(),
                stake_table: template.stake_table.clone(),
                validators: template.validators.clone(),
                address_mapping: template.address_mapping.clone(),
            };
            inner.put_epoch_committee(epoch, Arc::new(prefilled));
        }

        let coordinator = EpochMembershipCoordinator::<SeqTypes>::new(
            committees.clone(),
            epoch_height,
            &hotshot_example_types::storage_types::TestStorage::default(),
        );
        (committees, coordinator, epoch, fail_next_store)
    }

    // `add_epoch_root` is the only path by which a V0_6-native node (which
    // never calls the legacy `decide_epoch_root`/`Storage::store_epoch_root`)
    // persists the epoch root header. Regression test for that header being
    // silently dropped. The header version is irrelevant: the store happens
    // before any version-dependent branch. A cached header makes the committee
    // complete, which takes the early return; that path must persist too.
    #[rstest]
    #[case::incomplete_committee(false)]
    #[case::complete_committee(true)]
    #[tokio::test]
    async fn add_epoch_root_persists_epoch_root_header(#[case] cached_header: bool) {
        let header = build_epoch_root_header().await;
        let (committees, coordinator, epoch, _) = epoch_root_fixture(&header, cached_header);

        coordinator
            .add_epoch_root(header.clone())
            .await
            .expect("add_epoch_root should succeed for the prefilled state");

        let persistence = committees.fetcher.persistence.lock().await;
        assert_eq!(
            persistence.load_epoch_root(epoch).await.unwrap(),
            Some(header)
        );
    }

    // A failed `store_epoch_root` must be retried by the next `add_epoch_root`
    // for the epoch. The first call leaves the in-memory committee complete, so
    // the second takes the early return; a store placed after that return would
    // leave the header unpersisted for good.
    #[tokio::test]
    async fn add_epoch_root_retries_a_failed_store() {
        let header = build_epoch_root_header().await;
        let (committees, coordinator, epoch, fail_next_store) = epoch_root_fixture(&header, false);

        fail_next_store.store(true, Ordering::SeqCst);

        coordinator
            .add_epoch_root(header.clone())
            .await
            .expect("a dropped store must not fail `add_epoch_root`");
        assert_eq!(
            committees
                .fetcher
                .persistence
                .lock()
                .await
                .load_epoch_root(epoch)
                .await
                .unwrap(),
            None,
            "the injected failure should have dropped the write"
        );

        coordinator
            .add_epoch_root(header.clone())
            .await
            .expect("add_epoch_root should succeed for the complete committee");
        assert_eq!(
            committees
                .fetcher
                .persistence
                .lock()
                .await
                .load_epoch_root(epoch)
                .await
                .unwrap(),
            Some(header)
        );
    }

    // `add_da_committee` updates the per-epoch `da_committees` map and
    // must retroactively rebuild every loaded snapshot whose epoch
    // resolves to the new committee. Without the rebuild, snapshots
    // baked the old DA at construction time and reads would silently
    // observe stale data until `add_epoch_root` rebuilt them.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_da_committee_rebuilds_affected_snapshots() {
        fn da_keys(c: &EpochCommittees, e: EpochNumber) -> Vec<PubKey> {
            c.snapshot(e)
                .expect("snapshot")
                .da_stake_table()
                .map(|p| PubKey::public_key(&p.stake_table_entry))
                .collect()
        }

        // `EpochNumber::genesis()` is 1, so `new` keeps snapshots for
        // epochs 1 and 2, and seeds epochs 2 and 3 for `first_epoch = 2`,
        // all with the bootstrap DA.
        let committees = build_committees_with_fetcher(4, Fetcher::mock(), EpochNumber::new(2));

        let initial_e2 = da_keys(&committees, EpochNumber::new(2));
        let initial_e3 = da_keys(&committees, EpochNumber::new(3));

        let new_da: Vec<PeerConfig<SeqTypes>> = (0..2)
            .map(|i| {
                ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                    [123u8; 32],
                    i,
                    U256::from(50),
                    true,
                )
                .public_config()
            })
            .collect();
        let new_da_keys: Vec<PubKey> = new_da
            .iter()
            .map(|p| PubKey::public_key(&p.stake_table_entry))
            .collect();
        assert_ne!(
            new_da_keys, initial_e2,
            "test setup: new DA must differ from initial"
        );

        // Apply the new DA starting at epoch 2. Snapshot(2) and
        // snapshot(3) were seeded with the bootstrap DA; both must
        // observe the new DA after this call. Snapshot(1) lies before
        // `first_epoch=2`, so it stays on the bootstrap DA.
        committees.add_da_committee(EpochNumber::new(2), new_da);

        assert_eq!(
            da_keys(&committees, EpochNumber::new(2)),
            new_da_keys,
            "snapshot(2) must reflect the new DA",
        );
        assert_eq!(
            da_keys(&committees, EpochNumber::new(3)),
            new_da_keys,
            "snapshot(3) must reflect the new DA",
        );
        assert_eq!(
            da_keys(&committees, EpochNumber::new(1)),
            initial_e2,
            "snapshot(1) lies before first_epoch=2 and must keep the bootstrap DA",
        );
        // Sanity: the original epoch-2 and epoch-3 snapshots both used
        // the bootstrap DA before the call. If a future change to
        // `new` makes these diverge, this assertion will
        // catch it so the test setup can be revisited.
        assert_eq!(initial_e2, initial_e3);
    }

    // A `add_da_committee` call with a `first_epoch` greater than every
    // entry in an existing layered map must only rebuild snapshots in
    // its own range, not earlier ranges owned by other committees.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn add_da_committee_layered_does_not_rebuild_earlier_ranges() {
        fn da_keys(c: &EpochCommittees, e: EpochNumber) -> Vec<PubKey> {
            c.snapshot(e)
                .expect("snapshot")
                .da_stake_table()
                .map(|p| PubKey::public_key(&p.stake_table_entry))
                .collect()
        }

        let committees = build_committees(4);

        // Pre-populate snapshots for epochs 2..6 (`new` only seeds 1
        // and 2, but the range query needs more epochs to
        // exercise the layered case).
        {
            let mut inner = committees.inner.write();
            let template = inner
                .epoch_committee(EpochNumber::genesis())
                .expect("genesis committee exists")
                .clone();
            for e in 3..6 {
                inner.put_epoch_committee(EpochNumber::new(e), template.clone());
            }
        }

        let da_b: Vec<PeerConfig<SeqTypes>> = (0..2)
            .map(|i| {
                ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                    [200u8; 32],
                    i,
                    U256::from(50),
                    true,
                )
                .public_config()
            })
            .collect();
        let da_b_keys: Vec<PubKey> = da_b
            .iter()
            .map(|p| PubKey::public_key(&p.stake_table_entry))
            .collect();

        let da_c: Vec<PeerConfig<SeqTypes>> = (0..2)
            .map(|i| {
                ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                    [201u8; 32],
                    i,
                    U256::from(50),
                    true,
                )
                .public_config()
            })
            .collect();
        let da_c_keys: Vec<PubKey> = da_c
            .iter()
            .map(|p| PubKey::public_key(&p.stake_table_entry))
            .collect();

        // Insert C first at epoch 5, then B at epoch 3. Range [3, 5)
        // must be rebuilt with B; [5, ∞) must keep C.
        committees.add_da_committee(EpochNumber::new(5), da_c);
        committees.add_da_committee(EpochNumber::new(3), da_b);

        assert_eq!(da_keys(&committees, EpochNumber::new(3)), da_b_keys);
        assert_eq!(da_keys(&committees, EpochNumber::new(4)), da_b_keys);
        assert_eq!(
            da_keys(&committees, EpochNumber::new(5)),
            da_c_keys,
            "epoch 5 must keep C — it is outside B's range",
        );
    }

    fn peer(seed: u8, index: u64, stake: u64) -> PeerConfig<SeqTypes> {
        ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
            [seed; 32],
            index,
            U256::from(stake),
            true,
        )
        .public_config()
    }

    /// Genesis committee members: four with stake, and one without stake in
    /// the middle, which no stake table may hold. Returns all members and the
    /// staked ones.
    fn genesis_members() -> (Vec<PeerConfig<SeqTypes>>, Vec<PeerConfig<SeqTypes>>) {
        let staked: Vec<_> = (0..4).map(|i| peer(42, i, 100)).collect();
        let mut all = staked.clone();
        all.insert(2, peer(42, 4, 0));
        (all, staked)
    }

    fn committees_from_first_epoch(first_epoch: EpochNumber, fetcher: Fetcher) -> EpochCommittees {
        let (members, _) = genesis_members();
        EpochCommittees::new(members.clone(), members, None, fetcher, 100u64, first_epoch)
            .expect("genesis without duplicate keys")
    }

    /// Asserts that `epoch` runs on exactly the `staked` committee, has a DRB
    /// result, and picks its leaders from `staked`.
    fn assert_runs_on_committee(
        committees: &EpochCommittees,
        epoch: EpochNumber,
        staked: &[PeerConfig<SeqTypes>],
    ) {
        let snapshot = committees
            .snapshot(epoch)
            .unwrap_or_else(|| panic!("no snapshot for epoch {epoch}"));

        assert_eq!(
            snapshot.stake_table().len(),
            staked.len(),
            "epoch {epoch}: stake table size"
        );
        for peer in staked {
            let key = PubKey::public_key(&peer.stake_table_entry);
            assert_eq!(
                snapshot.stake(&key).as_ref(),
                Some(peer),
                "epoch {epoch}: stake table entry of {key}"
            );
        }

        assert!(snapshot.has_drb(), "epoch {epoch}: no DRB result");

        let keys: std::collections::HashSet<PubKey> = staked
            .iter()
            .map(|p| PubKey::public_key(&p.stake_table_entry))
            .collect();
        for view in 0..64 {
            let leader = snapshot
                .lookup_leader(ViewNumber::new(view))
                .unwrap_or_else(|err| panic!("epoch {epoch}, view {view}: no leader: {err}"));
            assert!(
                keys.contains(&leader),
                "epoch {epoch}, view {view}: leader {leader} is not in the committee"
            );
        }
    }

    // The constructor seeds `first_epoch` and `first_epoch + 1` with the
    // genesis committee and the initial DRB result, also when the chain does
    // not start its epochs at genesis.
    #[rstest]
    #[case::genesis(1)]
    #[case::later(5)]
    #[case::mainnet(275)]
    fn new_seeds_first_two_epochs_with_genesis_committee(#[case] first_epoch: u64) {
        let first_epoch = EpochNumber::new(first_epoch);
        let (_, staked) = genesis_members();
        let committees = committees_from_first_epoch(first_epoch, Fetcher::mock());

        assert_runs_on_committee(&committees, first_epoch, &staked);
        assert_runs_on_committee(&committees, first_epoch + 1, &staked);
    }

    // A snapshot rebuilt after construction must still know the first epoch:
    // without it, `lookup_leader` fails.
    #[rstest]
    #[case::genesis(1)]
    #[case::later(5)]
    fn add_da_committee_keeps_leaders_of_first_two_epochs(#[case] first_epoch: u64) {
        let first_epoch = EpochNumber::new(first_epoch);
        let (_, staked) = genesis_members();
        let committees = committees_from_first_epoch(first_epoch, Fetcher::mock());

        let other_da: Vec<_> = (0..2).map(|i| peer(123, i, 50)).collect();
        committees.add_da_committee(first_epoch, other_da.clone());

        for epoch in [first_epoch, first_epoch + 1] {
            assert_runs_on_committee(&committees, epoch, &staked);
            let da: Vec<_> = committees
                .snapshot(epoch)
                .expect("snapshot")
                .da_stake_table()
                .cloned()
                .collect();
            assert_eq!(da, other_da, "epoch {epoch}: DA committee");
        }
    }

    // A node adds the configured DA committees at startup, before it builds
    // the snapshots of later epochs. Each such snapshot takes the DA committee
    // with the greatest start epoch at or below its epoch, or the genesis DA
    // committee when no added committee starts at or below it.
    #[tokio::test]
    async fn snapshots_built_after_add_da_committee_take_the_da_committee_of_their_epoch() {
        let (_, staked) = genesis_members();
        let stored = AuthenticatedValidator::mock();
        let stored_table: AuthenticatedValidatorMap =
            [(stored.account, stored)].into_iter().collect();
        let fetcher = fetcher_with_latest_stake(
            (3..=8)
                .map(|e| (EpochNumber::new(e), (stored_table.clone(), None), None))
                .collect(),
        );
        let mut committees = committees_from_first_epoch(EpochNumber::genesis(), fetcher);

        let da_from_4: Vec<_> = (0..2).map(|i| peer(61, i, 50)).collect();
        let da_from_6: Vec<_> = (0..3).map(|i| peer(62, i, 50)).collect();
        let da_from_9: Vec<_> = (0..2).map(|i| peer(63, i, 50)).collect();
        committees.add_da_committee(EpochNumber::new(6), da_from_6.clone());
        committees.add_da_committee(EpochNumber::new(4), da_from_4.clone());
        committees.add_da_committee(EpochNumber::new(9), da_from_9);
        committees.reload_stake(RECENT_STAKE_TABLES_LIMIT).await;

        for (epoch, expected) in [
            (3, &staked),
            (4, &da_from_4),
            (5, &da_from_4),
            (6, &da_from_6),
            (7, &da_from_6),
            (8, &da_from_6),
        ] {
            let da: Vec<_> = committees
                .snapshot(EpochNumber::new(epoch))
                .unwrap_or_else(|| panic!("stored stake table for epoch {epoch} was not loaded"))
                .da_stake_table()
                .cloned()
                .collect();
            assert_eq!(&da, expected, "epoch {epoch}: DA committee");
        }
    }

    /// `count` committee members, none of them with stake.
    fn unstaked_members(count: u64) -> Vec<PeerConfig<SeqTypes>> {
        (0..count).map(|i| peer(42, i, 0)).collect()
    }

    // Without a staked member, the seeded first two epochs have no leader
    // candidates. A leader lookup there must fail, not panic.
    #[rstest]
    #[case::empty_from_genesis(0, EpochNumber::genesis())]
    #[case::empty_from_later(0, EpochNumber::new(5))]
    #[case::zero_stake_from_genesis(3, EpochNumber::genesis())]
    #[case::zero_stake_from_later(3, EpochNumber::new(5))]
    fn lookup_leader_errors_without_staked_members(
        #[case] members: u64,
        #[case] first_epoch: EpochNumber,
    ) {
        let members = unstaked_members(members);
        let committees = EpochCommittees::new(
            members.clone(),
            members,
            None,
            Fetcher::mock(),
            100u64,
            first_epoch,
        )
        .expect("genesis without duplicate keys");

        for epoch in [first_epoch, first_epoch + 1] {
            let snapshot = committees
                .snapshot(epoch)
                .unwrap_or_else(|| panic!("no snapshot for epoch {epoch}"));
            for view in [0, 1, 7, 1000] {
                let result = snapshot.lookup_leader(ViewNumber::new(view));
                assert!(
                    matches!(result, Err(EpochCommitteesError::LeaderLookupError)),
                    "epoch {epoch}, view {view}: expected LeaderLookupError, got {result:?}"
                );
            }
        }
    }

    // `NodeState::mock()` builds its membership without a staked member.
    #[tokio::test]
    async fn mock_coordinator_leader_errors_without_staked_members() {
        let coordinator = NodeState::mock().coordinator;
        let first_epoch = coordinator.membership().first_epoch();

        for epoch in [first_epoch, first_epoch + 1] {
            let membership = coordinator
                .membership_for_epoch(epoch)
                .unwrap_or_else(|err| panic!("epoch {epoch} is not seeded: {err:#}"));
            for view in [0, 1, 7, 1000] {
                let result = membership.leader(ViewNumber::new(view));
                assert!(
                    result.is_err(),
                    "epoch {epoch}, view {view}: expected an error, got {result:?}"
                );
            }
        }
    }

    /// Persistence stub whose `load_latest_stake` returns `stakes`. Every
    /// other read finds nothing, and every write is dropped.
    struct LatestStakeStore {
        stakes: Vec<crate::v0_3::IndexedStake>,
    }

    #[async_trait::async_trait]
    impl crate::traits::MembershipPersistence for LatestStakeStore {
        async fn load_stake(
            &self,
            _epoch: EpochNumber,
        ) -> anyhow::Result<Option<crate::traits::StakeTuple>> {
            Ok(None)
        }

        async fn load_latest_stake(
            &self,
            _limit: u64,
        ) -> anyhow::Result<Option<Vec<crate::v0_3::IndexedStake>>> {
            Ok(Some(self.stakes.clone()))
        }

        async fn load_drb_result(&self, _epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
            Ok(None)
        }

        async fn load_epoch_root(&self, _epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
            Ok(None)
        }

        async fn store_epoch_root(
            &self,
            _epoch: EpochNumber,
            _block_header: Header,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_stake(
            &self,
            _epoch: EpochNumber,
            _stake: AuthenticatedValidatorMap,
            _block_reward: Option<RewardAmount>,
            _stake_table_hash: Option<StakeTableHash>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_events(
            &self,
            _l1_finalized: u64,
            _events: Vec<(crate::v0_3::EventKey, crate::v0_3::StakeTableEvent)>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn load_events(
            &self,
            _from_l1_block: u64,
            _l1_finalized: u64,
        ) -> anyhow::Result<(
            Option<crate::traits::EventsPersistenceRead>,
            Vec<(crate::v0_3::EventKey, crate::v0_3::StakeTableEvent)>,
        )> {
            Ok((None, vec![]))
        }

        async fn delete_stake_tables(&self) -> anyhow::Result<()> {
            Ok(())
        }

        async fn store_all_validators(
            &self,
            _epoch: EpochNumber,
            _all_validators: IndexMap<Address, crate::v0_3::RegisteredValidator<PubKey>>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn load_all_validators(
            &self,
            _epoch: EpochNumber,
            _offset: u64,
            _limit: u64,
        ) -> anyhow::Result<Vec<crate::v0_3::RegisteredValidator<PubKey>>> {
            Ok(vec![])
        }
    }

    // On restart, storage can hold stake tables for the first two epochs.
    // They must not replace the genesis committee those epochs run on. A
    // stored table for a later epoch is loaded, and its snapshot knows the
    // first epoch once its DRB result arrives.
    #[rstest]
    #[case::genesis(1)]
    #[case::later(5)]
    #[tokio::test]
    async fn reload_stake_keeps_genesis_committee_of_first_two_epochs(#[case] first_epoch: u64) {
        let first_epoch = EpochNumber::new(first_epoch);
        let (_, staked) = genesis_members();

        let stored = AuthenticatedValidator::mock();
        let stored_key = *stored.stake_table_key();
        let stored_table: AuthenticatedValidatorMap =
            [(stored.account, stored)].into_iter().collect();
        let store = LatestStakeStore {
            stakes: [first_epoch, first_epoch + 1, first_epoch + 2]
                .into_iter()
                .map(|epoch| (epoch, (stored_table.clone(), None), None))
                .collect(),
        };
        let fetcher = Fetcher::new(
            Arc::new(crate::mock::MockStateCatchup::default()),
            Arc::new(AsyncMutex::new(store)),
            crate::L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            crate::ChainConfig::default(),
        );
        let mut committees = committees_from_first_epoch(first_epoch, fetcher);

        committees.reload_stake(RECENT_STAKE_TABLES_LIMIT).await;

        assert_runs_on_committee(&committees, first_epoch, &staked);
        assert_runs_on_committee(&committees, first_epoch + 1, &staked);

        let later = first_epoch + 2;
        let loaded: Vec<PubKey> = committees
            .snapshot(later)
            .unwrap_or_else(|| panic!("stored stake table for epoch {later} was not loaded"))
            .stake_table()
            .map(|p| PubKey::public_key(&p.stake_table_entry))
            .collect();
        assert_eq!(loaded, vec![stored_key], "epoch {later}: stake table");

        committees.add_drb_result(later, [7u8; 32]);
        let leader = committees
            .snapshot(later)
            .expect("snapshot")
            .lookup_leader(ViewNumber::new(0))
            .unwrap_or_else(|err| panic!("epoch {later}: no leader: {err}"));
        assert_eq!(leader, stored_key, "epoch {later}: leader");
    }

    // The coordinator serves stake tables from the first epoch on, and
    // rejects epochs before it.
    #[rstest]
    #[case::later(5)]
    #[case::mainnet(275)]
    #[tokio::test]
    async fn coordinator_serves_stake_tables_from_first_epoch(#[case] first_epoch: u64) {
        let before = EpochNumber::new(first_epoch - 1);
        let first_epoch = EpochNumber::new(first_epoch);
        let committees = committees_from_first_epoch(first_epoch, Fetcher::mock());
        let coordinator = EpochMembershipCoordinator::<SeqTypes>::new(
            committees,
            100u64,
            &hotshot_example_types::storage_types::TestStorage::default(),
        );

        for epoch in [first_epoch, first_epoch + 1] {
            coordinator
                .stake_table_for_epoch(epoch)
                .unwrap_or_else(|err| panic!("epoch {epoch}: no stake table: {err}"));
            coordinator
                .membership_for_epoch(epoch)
                .unwrap_or_else(|err| panic!("epoch {epoch}: no randomized stake table: {err}"));
        }
        assert!(
            coordinator.stake_table_for_epoch(before).is_err(),
            "epoch {before} is before the first epoch {first_epoch}"
        );
    }

    type SeqEpochMembership = hotshot_types::epoch_membership::EpochMembership<SeqTypes>;

    fn coordinator_for(committees: EpochCommittees) -> EpochMembershipCoordinator<SeqTypes> {
        EpochMembershipCoordinator::<SeqTypes>::new(
            committees,
            100u64,
            &hotshot_example_types::storage_types::TestStorage::default(),
        )
    }

    /// The membership of `epoch`, read once with each lookup of the
    /// coordinator. Panics if a lookup fails.
    fn lookups(
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        epoch: EpochNumber,
    ) -> [(&'static str, SeqEpochMembership); 2] {
        [
            (
                "stake_table_for_epoch",
                coordinator.stake_table_for_epoch(epoch),
            ),
            (
                "membership_for_epoch",
                coordinator.membership_for_epoch(epoch),
            ),
        ]
        .map(|(lookup, result)| {
            let membership = result.unwrap_or_else(|err| panic!("{lookup}({epoch:?}): {err}"));
            (lookup, membership)
        })
    }

    /// Genesis members in an order sorted neither by index nor by stake, with
    /// unequal stakes and two members without stake. Returns all members and
    /// the staked ones, in the given order.
    fn unordered_genesis_members() -> (Vec<PeerConfig<SeqTypes>>, Vec<PeerConfig<SeqTypes>>) {
        let all: Vec<_> = [
            (5, 300),
            (0, 0),
            (7, 100),
            (2, 500),
            (6, 0),
            (1, 200),
            (3, 400),
        ]
        .into_iter()
        .map(|(index, stake)| peer(42, index, stake))
        .collect();
        let staked = all
            .iter()
            .filter(|p| p.stake_table_entry.stake() > U256::ZERO)
            .cloned()
            .collect();
        (all, staked)
    }

    /// A fetcher whose persistence returns `stakes` as the latest stored
    /// stake tables.
    fn fetcher_with_latest_stake(stakes: Vec<crate::v0_3::IndexedStake>) -> Fetcher {
        Fetcher::new(
            Arc::new(crate::mock::MockStateCatchup::default()),
            Arc::new(AsyncMutex::new(LatestStakeStore { stakes })),
            crate::L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            crate::ChainConfig::default(),
        )
    }

    // Views before epochs were enabled run on the genesis stake table, as the
    // first epoch does.
    #[rstest]
    #[case::genesis(1)]
    #[case::later(5)]
    #[case::mainnet(275)]
    #[tokio::test]
    async fn genesis_stake_tables_match_first_epoch_stake_table(#[case] first_epoch: u64) {
        let first_epoch = EpochNumber::new(first_epoch);
        let (members, staked) = genesis_members();
        let committees = committees_from_first_epoch(first_epoch, Fetcher::mock());

        assert_genesis_reads_first_epoch(&committees, first_epoch, &members);
        // The genesis DA members are the genesis members, and `new` drops the
        // ones without stake.
        assert_genesis_da_committee(&committees.genesis_stake_tables(), &staked);
    }

    // VID shares and the light client stake table commitment depend on the
    // order of the stake table.
    #[rstest]
    #[case::genesis(1)]
    #[case::later(5)]
    #[case::mainnet(275)]
    #[tokio::test]
    async fn genesis_stake_tables_stake_table_is_staked_genesis_members_in_order(
        #[case] first_epoch: u64,
    ) {
        let first_epoch = EpochNumber::new(first_epoch);
        let (all, staked) = unordered_genesis_members();
        let unstaked: Vec<PubKey> = all
            .iter()
            .filter(|p| p.stake_table_entry.stake() == U256::ZERO)
            .map(|p| PubKey::public_key(&p.stake_table_entry))
            .collect();
        let committees =
            EpochCommittees::new(all.clone(), all, None, Fetcher::mock(), 100u64, first_epoch)
                .expect("genesis without duplicate keys");
        let genesis = committees.genesis_stake_tables();

        let stake_table: Vec<_> = genesis.stake_table().cloned().collect();
        assert_eq!(stake_table, staked, "genesis stake table");
        for key in &unstaked {
            assert!(
                genesis.stake(key).is_none(),
                "member {key} without stake is in the genesis stake table"
            );
        }
    }

    // Before epochs were enabled, the DA committee was the genesis one. A DA
    // committee that starts at the first epoch does not change that.
    #[rstest]
    #[case::genesis(1)]
    #[case::later(5)]
    #[case::mainnet(275)]
    #[tokio::test]
    async fn genesis_stake_tables_keep_genesis_da_committee_when_first_epoch_has_another(
        #[case] first_epoch: u64,
    ) {
        let first_epoch = EpochNumber::new(first_epoch);
        let (members, staked) = genesis_members();
        let committees = committees_from_first_epoch(first_epoch, Fetcher::mock());
        let other_da: Vec<_> = (0..2).map(|i| peer(123, i, 50)).collect();
        committees.add_da_committee(first_epoch, other_da.clone());

        let first = committees
            .snapshot(first_epoch)
            .unwrap_or_else(|| panic!("no snapshot for epoch {first_epoch}"));
        assert_snapshot_da_committee(first_epoch, &first, &other_da);
        assert_genesis_da_committee(&committees.genesis_stake_tables(), &staked);
        assert_genesis_reads_first_epoch(&committees, first_epoch, &members);
    }

    #[rstest]
    #[case::genesis(1)]
    #[case::later(5)]
    #[case::mainnet(275)]
    #[tokio::test]
    async fn genesis_stake_tables_ignore_stake_table_and_da_committee_of_later_epoch(
        #[case] first_epoch: u64,
    ) {
        let first_epoch = EpochNumber::new(first_epoch);
        let later = first_epoch + 2;
        let (_, staked) = genesis_members();

        let stored = AuthenticatedValidator::mock();
        let stored_key = *stored.stake_table_key();
        let stored_table: AuthenticatedValidatorMap =
            [(stored.account, stored)].into_iter().collect();
        let fetcher = fetcher_with_latest_stake(vec![(later, (stored_table, None), None)]);
        let mut committees = committees_from_first_epoch(first_epoch, fetcher);
        committees.reload_stake(RECENT_STAKE_TABLES_LIMIT).await;
        let later_da: Vec<_> = (0..2).map(|i| peer(77, i, 50)).collect();
        committees.add_da_committee(later, later_da.clone());

        let loaded = committees
            .snapshot(later)
            .unwrap_or_else(|| panic!("epoch {later}: no stake table"));
        let loaded_keys: Vec<PubKey> = loaded
            .stake_table()
            .map(|p| PubKey::public_key(&p.stake_table_entry))
            .collect();
        assert_eq!(loaded_keys, vec![stored_key], "epoch {later}: stake table");
        let loaded_da: Vec<_> = loaded.da_stake_table().cloned().collect();
        assert_eq!(loaded_da, later_da, "epoch {later}: DA committee");

        // The genesis DA members are all staked, so the genesis DA committee
        // is `staked` as well.
        let genesis = committees.genesis_stake_tables();
        let stake_table: Vec<_> = genesis.stake_table().cloned().collect();
        assert_eq!(stake_table, staked, "genesis stake table");
        let da: Vec<_> = genesis.da_stake_table().cloned().collect();
        assert_eq!(da, staked, "genesis DA committee");
    }

    // The DA committee is normally a subset of the stake table, so the other
    // table holds the repeated key once, which is fine.
    #[rstest]
    #[case::stake_table(false)]
    #[case::da_stake_table(true)]
    fn genesis_stake_tables_reject_key_twice(#[case] twice_in_da: bool) {
        let (_, staked) = genesis_members();
        let (twice, key) = with_key_twice(staked.clone());
        let (stake_table, da) = if twice_in_da {
            (staked, twice)
        } else {
            (twice, staked)
        };

        let Err(err) = GenesisStakeTables::<SeqTypes>::new(stake_table, da) else {
            panic!("accepted a table with key {key} twice");
        };
        let message = format!("{err:#}");
        assert!(
            message.contains(&key.to_string()),
            "error {message:?} does not name the key {key}"
        );
    }

    // `new` drops the members without stake before it builds the genesis
    // tables, so both entries of the repeated key have stake.
    #[rstest]
    #[case::stake_table(false)]
    #[case::da_stake_table(true)]
    #[tokio::test]
    async fn epoch_committees_reject_staked_genesis_key_twice(#[case] twice_in_da: bool) {
        let (all, _) = genesis_members();
        let (twice, key) = with_key_twice(all.clone());
        let (members, da) = if twice_in_da {
            (all, twice)
        } else {
            (twice, all)
        };

        let result = EpochCommittees::new(
            members,
            da,
            None,
            Fetcher::mock(),
            100u64,
            EpochNumber::new(5),
        );
        assert!(result.is_err(), "accepted a genesis with key {key} twice");
    }

    /// `members` with the key of `members[1]` once more at the end, with more
    /// stake. So the two entries differ, are not neighbours, and only their
    /// keys match. Returns the table and the repeated key.
    fn with_key_twice(
        mut members: Vec<PeerConfig<SeqTypes>>,
    ) -> (Vec<PeerConfig<SeqTypes>>, PubKey) {
        let mut twice = members[1].clone();
        twice.stake_table_entry.stake_amount += U256::from(200);
        let key = PubKey::public_key(&twice.stake_table_entry);
        members.push(twice);
        (members, key)
    }

    /// Asserts that every read of the genesis stake table gives the same
    /// answer as the snapshot of `first_epoch`. `members` are the keys to
    /// check `stake` for, next to one key that is not a member.
    fn assert_genesis_reads_first_epoch(
        committees: &EpochCommittees,
        first_epoch: EpochNumber,
        members: &[PeerConfig<SeqTypes>],
    ) {
        let genesis = committees.genesis_stake_tables();
        let first = committees
            .snapshot(first_epoch)
            .unwrap_or_else(|| panic!("no snapshot for epoch {first_epoch}"));
        let context = format!("genesis vs epoch {first_epoch}");

        let genesis_table: Vec<_> = genesis.stake_table().cloned().collect();
        let first_table: Vec<_> = first.stake_table().cloned().collect();
        assert_eq!(genesis_table, first_table, "{context}: stake table");
        assert_eq!(
            genesis.success_threshold(),
            first.success_threshold(),
            "{context}: success threshold"
        );

        let outsider = peer(99, 0, 100);
        for member in members.iter().chain(std::iter::once(&outsider)) {
            let key = PubKey::public_key(&member.stake_table_entry);
            let stake = genesis.stake(&key);
            assert_eq!(stake, first.stake(&key), "{context}: stake({key})");
        }
    }

    /// Asserts that the genesis DA stake table is `expected`, in order.
    fn assert_genesis_da_committee(
        genesis: &GenesisStakeTables<SeqTypes>,
        expected: &[PeerConfig<SeqTypes>],
    ) {
        let da: Vec<_> = genesis.da_stake_table().cloned().collect();
        assert_eq!(da, expected, "genesis DA stake table");
    }

    /// Asserts that every read of the DA committee of the snapshot of `epoch`
    /// gives `expected`, in order, and nothing else.
    fn assert_snapshot_da_committee(
        epoch: EpochNumber,
        snapshot: &EpochSnapshot,
        expected: &[PeerConfig<SeqTypes>],
    ) {
        let da: Vec<_> = snapshot.da_stake_table().cloned().collect();
        assert_eq!(da, expected, "epoch {epoch}: DA stake table");
        assert_eq!(
            snapshot.da_total_nodes(),
            expected.len(),
            "epoch {epoch}: DA total nodes"
        );
        let members: std::collections::HashSet<PubKey> = snapshot
            .da_committee_members(ViewNumber::new(0))
            .copied()
            .collect();
        assert_eq!(
            members,
            keys_of(expected),
            "epoch {epoch}: DA committee members"
        );

        for peer in expected {
            let key = PubKey::public_key(&peer.stake_table_entry);
            let da_stake = snapshot.da_stake(&key);
            assert_eq!(
                da_stake.as_ref(),
                Some(peer),
                "epoch {epoch}: da_stake({key})"
            );
            assert!(
                snapshot.has_da_stake(&key),
                "epoch {epoch}: has_da_stake({key})"
            );
        }
        let outsider = PubKey::public_key(&peer(99, 0, 100).stake_table_entry);
        assert!(
            !snapshot.has_da_stake(&outsider),
            "epoch {epoch}: has_da_stake of a key outside the DA committee"
        );
    }

    fn keys_of(peers: &[PeerConfig<SeqTypes>]) -> std::collections::HashSet<PubKey> {
        peers
            .iter()
            .map(|p| PubKey::public_key(&p.stake_table_entry))
            .collect()
    }

    /// One member for each of `stakes`, in order, with keys from `seed`.
    fn members_with_stakes(seed: u8, stakes: &[u64]) -> Vec<PeerConfig<SeqTypes>> {
        (0..)
            .zip(stakes)
            .map(|(index, &stake)| peer(seed, index, stake))
            .collect()
    }

    /// A member with `stake`, given in hexadecimal, so it may not fit in a
    /// `u64`.
    fn member_with_hex_stake(seed: u8, stake: &str) -> PeerConfig<SeqTypes> {
        ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
            [seed; 32],
            0,
            hex_u256(stake),
            true,
        )
        .public_config()
    }

    fn hex_u256(hex: &str) -> U256 {
        U256::from_str_radix(hex, 16).unwrap_or_else(|err| panic!("{hex}: {err}"))
    }

    /// Asserts the failure threshold and the DA success threshold of the
    /// first two epochs of a chain whose genesis stake table is `quorum` and
    /// whose genesis DA committee is `da`. Reads both from the epoch snapshot
    /// and from each lookup of the coordinator.
    fn assert_epoch_thresholds(
        quorum: Vec<PeerConfig<SeqTypes>>,
        da: Vec<PeerConfig<SeqTypes>>,
        failure: U256,
        da_success: U256,
    ) {
        let first_epoch = EpochNumber::new(5);
        let committees =
            EpochCommittees::new(quorum, da, None, Fetcher::mock(), 100u64, first_epoch)
                .expect("genesis without duplicate keys");
        let coordinator = coordinator_for(committees);

        for epoch in [first_epoch, first_epoch + 1] {
            let snapshot = coordinator
                .membership()
                .snapshot(epoch)
                .unwrap_or_else(|| panic!("no snapshot for epoch {epoch}"));
            assert_eq!(
                snapshot.failure_threshold(),
                failure,
                "snapshot({epoch}): failure threshold"
            );
            assert_eq!(
                snapshot.da_success_threshold(),
                da_success,
                "snapshot({epoch}): DA success threshold"
            );

            for (lookup, membership) in lookups(&coordinator, epoch) {
                assert_eq!(
                    membership.failure_threshold(),
                    failure,
                    "{lookup}({epoch}): failure threshold"
                );
                assert_eq!(
                    membership.da_success_threshold(),
                    da_success,
                    "{lookup}({epoch}): DA success threshold"
                );
            }
        }
    }

    // The failure threshold is one third of the quorum stake, rounded down,
    // plus one. The DA success threshold is two thirds of the DA stake,
    // rounded down, plus one. The quorum and DA totals differ in most cases,
    // so a threshold read from the wrong table fails the test.
    #[rstest]
    #[case::empty(&[], &[], 1, 1)]
    #[case::quorum_1_da_100(&[1], &[40, 60], 1, 67)]
    #[case::quorum_2_da_10(&[1, 1], &[1, 2, 3, 4], 1, 7)]
    #[case::quorum_3_da_4(&[1, 1, 1], &[4], 2, 3)]
    #[case::quorum_4_da_3(&[1, 3], &[1, 1, 1], 2, 3)]
    #[case::quorum_7_da_5(&[7], &[2, 3], 3, 4)]
    #[case::quorum_10_da_2(&[1, 2, 3, 4], &[2], 4, 2)]
    #[case::quorum_100_da_1(&[10, 20, 30, 40], &[1], 34, 1)]
    #[case::quorum_1000_da_11(&[100, 200, 300, 400], &[5, 6], 334, 8)]
    #[tokio::test]
    async fn epoch_failure_and_da_success_thresholds(
        #[case] quorum: &[u64],
        #[case] da: &[u64],
        #[case] failure: u64,
        #[case] da_success: u64,
    ) {
        assert_epoch_thresholds(
            members_with_stakes(42, quorum),
            members_with_stakes(123, da),
            U256::from(failure),
            U256::from(da_success),
        );
    }

    // Totals of at least `U256::MAX / 2` take another branch of the DA
    // success threshold, because doubling them overflows.
    #[rstest]
    // Quorum `U256::MAX`, DA `U256::MAX / 2 - 1`.
    #[case::da_just_below_half_of_max(
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "7ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe",
        "5555555555555555555555555555555555555555555555555555555555555556",
        "5555555555555555555555555555555555555555555555555555555555555555"
    )]
    // Quorum `U256::MAX / 2 - 1`, DA `U256::MAX / 2`.
    #[case::da_half_of_max(
        "7ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe",
        "7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "2aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
        "5555555555555555555555555555555555555555555555555555555555555556"
    )]
    // Quorum 1, DA `U256::MAX`.
    #[case::da_max(
        "1",
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "1",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaac"
    )]
    #[tokio::test]
    async fn epoch_failure_and_da_success_thresholds_near_u256_max(
        #[case] quorum: &str,
        #[case] da: &str,
        #[case] failure: &str,
        #[case] da_success: &str,
    ) {
        assert_epoch_thresholds(
            vec![member_with_hex_stake(42, quorum)],
            vec![member_with_hex_stake(123, da)],
            hex_u256(failure),
            hex_u256(da_success),
        );
    }
}
