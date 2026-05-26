use std::{
    collections::{BTreeMap, HashMap},
    ops::Bound,
    sync::Arc,
};

use alloy::primitives::{Address, U256};
use anyhow::{Context, bail};
use async_lock::Mutex as AsyncMutex;
use committable::Commitment;
use hotshot::types::{BLSPubKey, SignatureKey as _};
use hotshot_types::{
    PeerConfig, PeerConnectInfo,
    data::{BlockNumber, EpochNumber, ViewNumber},
    drb::{
        DrbResult,
        election::{RandomizedCommittee, generate_stake_cdf, select_randomized_leader},
    },
    epoch_membership::EpochMembershipCoordinator,
    stake_table::{HSStakeTable, StakeTableEntry},
    traits::{
        block_contents::BlockHeader,
        election::{Membership, MembershipSnapshot, NonEpochMembershipSnapshot},
        signature_key::StakeTableEntryType,
    },
    utils::{
        epoch_from_block_number, is_epoch_root, root_block_in_epoch, transition_block_for_epoch,
    },
};
use indexmap::IndexMap;
use parking_lot::RwLock;
use thiserror::Error;
use tracing::{debug, error, info, warn};
use versions::{DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_VERSION};

use super::{
    AuthenticatedValidatorMap, RegisteredValidatorMap, StakeTableHash, StakeTableState,
    compute_block_reward,
};
use crate::{
    Header, Leaf2, PubKey, SeqTypes,
    traits::StateCatchup,
    v0_3::{ASSUMED_BLOCK_TIME_SECONDS, AuthenticatedValidator, Fetcher, RewardAmount},
};

/// Type to describe DA and Stake memberships.
#[derive(Clone, Debug)]
pub struct EpochCommittees {
    inner: Arc<RwLock<Inner>>,
    fetcher: Arc<Fetcher>,
    update_fixed_block_reward_lock: Arc<AsyncMutex<()>>,
    epoch_height: BlockNumber,
}

#[derive(Debug)]
struct Inner {
    /// Captured pre-epoch view.
    ///
    /// Built once at constructor time. `add_da_committee` operates on the
    /// per-epoch `da_committees` map and does not modify the pre-epoch view.
    non_epoch_snapshot: NonEpochSnapshot,

    /// Per-epoch snapshots.
    ///
    /// Each mutator rebuilds the snapshot for its affected epoch(s).
    snapshots: BTreeMap<EpochNumber, EpochSnapshot>,

    /// Holds the full validator candidate sets temporarily, until we store them.
    all_validators: BTreeMap<EpochNumber, RegisteredValidatorMap>,

    /// DA committees, indexed by the first epoch in which they apply.
    ///
    /// Kept separate from `snapshots` because the lookup is a range query.
    da_committees: BTreeMap<EpochNumber, Arc<DaCommittee>>,

    first_epoch: Option<EpochNumber>,

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

/// Pre-epoch stake-table state.
#[derive(Debug)]
struct NonEpochCommittee {
    /// The nodes eligible for leadership.
    ///
    /// NOTE: This is currently a hack because the DA leader needs to be the quorum
    /// leader but without voting rights.
    eligible_leaders: Vec<PeerConfig<SeqTypes>>,

    /// Keys for nodes participating in the network
    stake_table: Vec<PeerConfig<SeqTypes>>,

    /// Stake entries indexed by public key, for efficient lookup.
    indexed_stake_table: HashMap<PubKey, PeerConfig<SeqTypes>>,
}

/// Holds Stake table and da stake
#[derive(Debug)]
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
                        state_ver_key: v.state_ver_key.clone(),
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

    pub fn first_epoch(&self) -> Option<EpochNumber> {
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
        inner
            .non_epoch_snapshot
            .inner
            .committee
            .indexed_stake_table
            .get(key)
            .cloned()
    }

    /// Fetch the fixed block reward and update it if its None.
    /// We used a fixed block reward for version v3
    /// Version v4 uses the dynamic block reward
    /// Assumes the stake table contract proxy address does not change
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
    ) -> anyhow::Result<Option<RewardAmount>> {
        let epoch_height = self.epoch_height;
        let current_epoch = epoch_from_block_number(header.height(), *epoch_height);
        let previous_epoch = current_epoch
            .checked_sub(1)
            .context("underflow: cannot get previous epoch when current_epoch is 0")?;
        debug!(?epoch, "previous_epoch={previous_epoch:?}");

        let first_epoch = *self.first_epoch().context("first epoch is None")?;

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
        let initial_supply = *self.fetcher.initial_supply.read().await;
        let initial_supply = match initial_supply {
            Some(supply) => supply,
            None => self.fetcher.fetch_and_update_initial_supply().await?,
        };
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

                    let prev_snapshot = self
                        .snapshot(EpochNumber::new(previous_epoch))
                        .context("Stake table not found")?;
                    let prev_stake_table =
                        HSStakeTable(prev_snapshot.stake_table().cloned().collect());
                    let success_threshold = prev_snapshot.success_threshold();

                    self.fetcher
                        .peers
                        .fetch_leaf(root_height, prev_stake_table, success_threshold)
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

    // We need a constructor to match our concrete type.
    pub fn new_stake<B: Into<BlockNumber>>(
        // TODO remove `new` from trait and rename this to `new`.
        // https://github.com/EspressoSystems/HotShot/commit/fcb7d54a4443e29d643b3bbc53761856aef4de8b
        committee_members: Vec<PeerConfig<SeqTypes>>,
        da_members: Vec<PeerConfig<SeqTypes>>,
        fixed_block_reward: Option<RewardAmount>,
        fetcher: Fetcher,
        epoch_height: B,
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

        let da_committee = Arc::new(DaCommittee {
            committee: da_members,
            indexed_committee: indexed_da_members,
        });

        let members = Arc::new(NonEpochCommittee {
            eligible_leaders,
            stake_table,
            indexed_stake_table,
        });

        let non_epoch_snapshot = NonEpochSnapshot::new(members.clone(), da_committee.clone());

        let epoch_committee = Arc::new(EpochCommittee {
            eligible_leaders: members.eligible_leaders.clone(),
            stake_table: members
                .stake_table
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
                None,
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
                None,
                epoch_committee,
                None,
                da_committee,
            ),
        );

        Self {
            inner: Arc::new(RwLock::new(Inner {
                non_epoch_snapshot,
                da_committees: BTreeMap::new(),
                snapshots,
                all_validators: BTreeMap::new(),
                first_epoch: None,
                fixed_block_reward,
            })),
            fetcher: Arc::new(fetcher),
            update_fixed_block_reward_lock: Arc::new(AsyncMutex::new(())),
            epoch_height: epoch_height.into(),
        }
    }

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

        // Load the 50 latest stored stake tables
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

        for (epoch, (validators, block_reward), stake_table_hash) in loaded_stake {
            let committee = EpochCommittee::new(validators, block_reward, stake_table_hash, None);
            self.inner
                .write()
                .put_epoch_committee(epoch, Arc::new(committee));
        }
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

        first_epoch = membership.first_epoch.context(format!(
            "First epoch not initialized (current_epoch={current_epoch})"
        ))?;
    }

    if *current_epoch <= *first_epoch + 1 {
        bail!(
            "epoch is in first two epochs: current_epoch={current_epoch}, \
             first_epoch={first_epoch}"
        );
    }

    let header = match committee.header.clone() {
        Some(header) => header,
        None => {
            let root_epoch = current_epoch.checked_sub(2).context(format!(
                "Epoch calculation underflow (current_epoch={current_epoch})"
            ))?;

            info!(?root_epoch, "catchup epoch root header");

            let leaf = coordinator
                .membership()
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
        && let Err(err) = coordinator.stake_table_for_epoch(Some(prev_epoch))
    {
        info!("failed to get membership for epoch={prev_epoch:?}: {err:#}");

        coordinator
            .wait_for_catchup(prev_epoch)
            .await
            .context(format!("failed to catch up for epoch={prev_epoch}"))?;
    }

    coordinator
        .membership()
        .calculate_dynamic_block_reward(current_epoch, &header, &committee.validators)
        .await
        .with_context(|| {
            format!("dynamic block reward calculation failed for epoch={current_epoch}")
        })?
        .with_context(|| format!("dynamic block reward returned None. epoch={current_epoch}"))
}

impl Membership<SeqTypes> for EpochCommittees {
    type Error = EpochCommitteesError;
    type Snapshot = EpochSnapshot;
    type NonEpochSnapshot = NonEpochSnapshot;

    fn snapshot(&self, epoch: EpochNumber) -> Option<Self::Snapshot> {
        self.inner.read().snapshots.get(&epoch).cloned()
    }

    fn non_epoch_snapshot(&self) -> Self::NonEpochSnapshot {
        self.inner.read().non_epoch_snapshot.clone()
    }

    /// Adds the epoch committee and block reward for a given epoch,
    /// either by fetching from L1 or using local state if available.
    /// It also calculates and stores the block reward based on header version.
    async fn add_epoch_root(&self, block_header: Header) -> Result<(), Self::Error> {
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
                .calculate_dynamic_block_reward(epoch, &block_header, &active_validators)
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

    async fn get_epoch_root(&self, epoch: EpochNumber) -> Result<Leaf2, Self::Error> {
        let block_height = root_block_in_epoch(*epoch, *self.epoch_height());
        let peers = self.fetcher.peers.clone();
        let snapshot = self
            .snapshot(epoch)
            .ok_or_else(|| Self::Error::Message(format!("no committee for epoch={epoch}")))?;
        let stake_table = HSStakeTable(snapshot.stake_table().cloned().collect());
        let success_threshold = snapshot.success_threshold();
        let leaf: Leaf2 = peers
            .fetch_leaf(block_height, stake_table, success_threshold)
            .await
            .map_err(Self::Error::Catchup)?;
        Ok(leaf)
    }

    async fn get_epoch_drb(&self, epoch: EpochNumber) -> Result<DrbResult, Self::Error> {
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

        let prev_snapshot = self.snapshot(previous_epoch).ok_or_else(|| {
            Self::Error::Message(format!("no committee for previous_epoch={previous_epoch}"))
        })?;
        let stake_table = HSStakeTable(prev_snapshot.stake_table().cloned().collect());
        let success_threshold = prev_snapshot.success_threshold();

        let block_height = transition_block_for_epoch(*previous_epoch, *self.epoch_height());

        debug!(
            "Getting DRB for epoch {}, block height {}",
            epoch, block_height
        );
        let drb_leaf = peers
            .try_fetch_leaf(1, block_height, stake_table, success_threshold)
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

    fn set_first_epoch(&self, epoch: EpochNumber, initial_drb_result: DrbResult) {
        let rand_comm = Arc::new(
            self.randomized_committee(EpochNumber::genesis(), initial_drb_result)
                .expect("committee exist at genesis"),
        );

        let mut inner = self.inner.write();
        inner.first_epoch = Some(epoch);

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

    fn first_epoch(&self) -> Option<EpochNumber> {
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
    /// The DA committee that applies to `epoch`, or the non-epoch fallback
    /// when `epoch` is `None` or no explicit DA committee covers it.
    fn resolve_da_committee(&self, epoch: Option<EpochNumber>) -> Arc<DaCommittee> {
        if let Some(e) = epoch {
            // The greatest key ≤ `e` is the DA committee that applies.
            self.da_committees
                .range((Bound::Included(0.into()), Bound::Included(e)))
                .last()
                .map(|(_, committee)| committee.clone())
                .unwrap_or_else(|| self.non_epoch_snapshot.inner.da_committee.clone())
        } else {
            self.non_epoch_snapshot.inner.da_committee.clone()
        }
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
        let da_committee = self.resolve_da_committee(Some(epoch));
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
    first_epoch: Option<EpochNumber>,
    committee: Arc<EpochCommittee>,
    randomized: Option<Arc<RandomizedCommittee<StakeTableEntry<PubKey>>>>,
    da_committee: Arc<DaCommittee>,
}

impl EpochSnapshot {
    fn new(
        epoch: EpochNumber,
        first_epoch: Option<EpochNumber>,
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

    fn first_epoch(&self) -> Option<EpochNumber> {
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
    /// Returns `LeaderLookupError` if `first_epoch` is unset, the snapshot's
    /// epoch is before `first_epoch`, or the randomized committee is missing.
    fn lookup_leader(&self, view: ViewNumber) -> Result<PubKey, Self::Error> {
        let inner = &self.inner;
        let Some(first_epoch) = inner.first_epoch else {
            error!(
                "leader requested for epoch {} but first_epoch is unset",
                inner.epoch,
            );
            return Err(EpochCommitteesError::LeaderLookupError);
        };
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
        Ok(PubKey::public_key(&select_randomized_leader(rand, *view)))
    }

    fn stake_table_hash(&self) -> Option<Commitment<Self::StakeTableHash>> {
        self.inner.committee.stake_table_hash
    }
}

/// A consistent pre-epoch view of `EpochCommittees`.
///
/// Returned by [`Membership::non_epoch_snapshot`].
#[derive(Clone, Debug)]
pub struct NonEpochSnapshot {
    inner: Arc<NonEpochSnapshotInner>,
}

#[derive(Debug)]
struct NonEpochSnapshotInner {
    committee: Arc<NonEpochCommittee>,
    da_committee: Arc<DaCommittee>,
}

impl NonEpochSnapshot {
    fn new(committee: Arc<NonEpochCommittee>, da_committee: Arc<DaCommittee>) -> Self {
        Self {
            inner: Arc::new(NonEpochSnapshotInner {
                committee,
                da_committee,
            }),
        }
    }
}

impl NonEpochMembershipSnapshot<SeqTypes> for NonEpochSnapshot {
    type Error = EpochCommitteesError;

    fn stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<SeqTypes>> + Send + '_ {
        self.inner.committee.stake_table.iter()
    }

    fn da_stake_table(&self) -> impl ExactSizeIterator<Item = &PeerConfig<SeqTypes>> + Send + '_ {
        self.inner.da_committee.committee.iter()
    }

    fn committee_members(
        &self,
        _: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &PubKey> + Send + '_ {
        self.inner.committee.indexed_stake_table.keys()
    }

    fn da_committee_members(
        &self,
        _: ViewNumber,
    ) -> impl ExactSizeIterator<Item = &PubKey> + Send + '_ {
        self.inner.da_committee.indexed_committee.keys()
    }

    fn stake(&self, key: &PubKey) -> Option<PeerConfig<SeqTypes>> {
        self.inner.committee.indexed_stake_table.get(key).cloned()
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

    fn lookup_leader(&self, view: ViewNumber) -> Result<PubKey, Self::Error> {
        let leaders = &self.inner.committee.eligible_leaders;
        if leaders.is_empty() {
            return Err(EpochCommitteesError::LeaderLookupError);
        }
        let index = *view as usize % leaders.len();
        Ok(PubKey::public_key(&leaders[index].stake_table_entry))
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
    use tokio::{task::JoinSet, time::Duration};

    use super::*;
    use crate::{NodeState, Payload, Transaction};

    /// Wall-clock target each concurrency test runs for. Long enough to
    /// catch flaky races that one-shot tests would miss; short enough to
    /// be tolerable in CI.
    const TEST_DURATION: Duration = Duration::from_secs(5);

    fn build_committees(num_peers: u64) -> EpochCommittees {
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
        EpochCommittees::new_stake(peers.clone(), peers, None, Fetcher::mock(), 100u64)
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
        committees.set_first_epoch(EpochNumber::new(1), [0u8; 32]);

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
                    // (both were set by `set_first_epoch` before this
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
                    assert_eq!(c.first_epoch(), Some(stable));

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

    // A task concurrent with `set_first_epoch` must never see a
    // partially-applied state, since all mutations in `set_first_epoch`
    // happen under a single write lock.
    //
    // We can't verify this through the public API because each method
    // takes its own lock — between two reader calls the writer can run
    // to completion (a real TOCTOU window in the new locking model, not
    // a torn read). Instead we take a single-locked snapshot of all
    // affected fields directly and assert the snapshot is internally
    // consistent: either the pre-state (nothing set) or the post-state
    // (everything set together).
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn set_first_epoch_is_atomic() {
        // Snapshot all fields touched by `set_first_epoch` under a
        // single read lock so the reader observes one consistent state.
        fn snapshot(c: &EpochCommittees, e: EpochNumber) -> Snapshot {
            let inner = c.inner.read();
            Snapshot {
                first_epoch: inner.first_epoch,
                state_e: inner.snapshots.contains_key(&e),
                state_e1: inner.snapshots.contains_key(&(e + 1)),
                rand_e: inner
                    .snapshots
                    .get(&e)
                    .is_some_and(|s| s.inner.randomized.is_some()),
                rand_e1: inner
                    .snapshots
                    .get(&(e + 1))
                    .is_some_and(|s| s.inner.randomized.is_some()),
            }
        }

        #[derive(Debug)]
        struct Snapshot {
            first_epoch: Option<EpochNumber>,
            state_e: bool,
            state_e1: bool,
            rand_e: bool,
            rand_e1: bool,
        }

        let target = EpochNumber::new(10);

        // Concurrency bugs are flaky — loop until we've spent
        // `TEST_DURATION` widening the window for catching a torn
        // state. Each round is one race attempt against
        // `set_first_epoch`.
        let test_start = tokio::time::Instant::now();
        let mut round: u64 = 0;
        while test_start.elapsed() < TEST_DURATION {
            let committees = build_committees(4);
            let stop = Arc::new(AtomicBool::new(false));
            let post_observations = Arc::new(AtomicUsize::new(0));

            let reader = {
                let c = committees.clone();
                let stop = Arc::clone(&stop);
                let post = Arc::clone(&post_observations);
                tokio::spawn(async move {
                    while !stop.load(Ordering::Relaxed) {
                        let s = snapshot(&c, target);
                        match s.first_epoch {
                            None => assert!(
                                !s.state_e && !s.state_e1 && !s.rand_e && !s.rand_e1,
                                "torn snapshot: first_epoch=None but some target state present: \
                                 {s:?}",
                            ),
                            Some(e) => {
                                assert_eq!(e, target, "only target is ever set");
                                assert!(
                                    s.state_e && s.state_e1 && s.rand_e && s.rand_e1,
                                    "torn snapshot: first_epoch=Some but some target state \
                                     missing: {s:?}",
                                );
                                post.fetch_add(1, Ordering::Relaxed);
                            },
                        }
                        tokio::task::yield_now().await;
                    }
                })
            };

            // Brief warmup so the reader is in its loop.
            tokio::time::sleep(Duration::from_millis(2)).await;
            committees.set_first_epoch(target, [(round as u8) ^ 0xA5; 32]);

            // Wait until the reader has observed the post-state at least
            // once, with a generous timeout.
            let deadline = tokio::time::Instant::now() + Duration::from_millis(200);
            while tokio::time::Instant::now() < deadline
                && post_observations.load(Ordering::Relaxed) == 0
            {
                tokio::task::yield_now().await;
            }

            stop.store(true, Ordering::Relaxed);
            reader.await.expect("reader panicked");
            assert!(
                post_observations.load(Ordering::Relaxed) > 0,
                "round {round}: reader never observed post-set state",
            );
            round += 1;
        }
        assert!(round > 0, "test loop never executed a round");
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
        // `has_randomized_stake_table` and `lookup_leader` both
        // require `first_epoch` to be set.
        committees.set_first_epoch(epoch, [0u8; 32]);

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
        let (payload, _) = Payload::from_transactions([tx], &instance.genesis_state, &instance)
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
            committees
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

        // `EpochNumber::genesis()` is 1, so `new_stake` seeds snapshots
        // for epochs 1 and 2. `set_first_epoch(2, ...)` then rebuilds
        // snapshots for epochs 2 and 3, all with the bootstrap DA.
        let committees = build_committees(4);
        committees.set_first_epoch(EpochNumber::new(2), [0u8; 32]);

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
        // `set_first_epoch` makes these diverge, this assertion will
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
        committees.set_first_epoch(EpochNumber::new(1), [0u8; 32]);

        // Pre-populate snapshots for epochs 2..6 (set_first_epoch only
        // covers 1 and 2, but the range query needs more epochs to
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
}
