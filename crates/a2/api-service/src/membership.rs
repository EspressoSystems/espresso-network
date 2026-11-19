//! The stake tables of the chain, which the API service derives as consensus does: the genesis
//! file gives the stake table of the first epochs, and each later one comes from the stake table
//! contract's events on L1 up to the root block of the epoch two before. The root blocks come from
//! the store, so the stake tables follow what the store holds.
//!
//! The API service needs them only to check what the chain does not commit to: the `Certificate2`
//! that decided a block directly, and the state certificate of an epoch root.

use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU64,
    ops::Range,
    sync::Arc,
};

use alloy::primitives::{Address, U256};
use anyhow::{Context, bail, ensure};
use async_lock::Mutex as AsyncMutex;
use async_trait::async_trait;
use committable::Committable;
use espresso_genesis::Genesis;
use espresso_types::{
    AuthenticatedValidatorMap, EpochCommittees, Header, L1ClientOptions, Leaf2, PubKey, SeqTypes,
    StakeTableHash,
    traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
    v0_3::{EventKey, Fetcher, IndexedStake, RegisteredValidator, RewardAmount, StakeTableEvent},
};
use hotshot_types::{
    ValidatorConfig,
    data::{
        DaProposal2, EpochNumber, QuorumProposalWrapper, VidCommitment, VidDisperseShare,
        ViewNumber,
    },
    drb::{DrbInput, DrbResult},
    epoch_membership::EpochMembershipCoordinator,
    event::HotShotAction,
    message::{Proposal, UpgradeLock},
    simple_certificate::{Certificate2, LightClientStateUpdateCertificateV2, UpgradeCertificate},
    stake_table::HSStakeTable,
    traits::{
        election::{Membership as _, MembershipSnapshot as _},
        storage::Storage,
    },
    utils::{epoch_from_block_number, is_epoch_root, root_block_in_epoch},
};
use indexmap::IndexMap;
use light_client::consensus::quorum::{Quorum, StakeTable, StakeTableQuorum};
use url::Url;
use versions::Upgrade;

use crate::{null_catchup::NullStateCatchup, state_cert::validate_state_cert};

/// The stake tables of the chain, as far as the store's epoch roots reach.
#[derive(Clone)]
pub struct Membership {
    coordinator: EpochMembershipCoordinator<SeqTypes>,
    epoch_height: u64,
}

impl Membership {
    /// The membership of the chain `genesis` starts, reading stake table events from the L1 at
    /// `l1`.
    pub async fn new(genesis: &Genesis, l1: Vec<Url>) -> anyhow::Result<Self> {
        let first_epoch = genesis.first_epoch()?;
        let epoch_height = genesis.epoch_height.unwrap_or_default();
        // The API service has no stake, so any key serves to read the genesis stake table.
        let key =
            ValidatorConfig::<SeqTypes>::generated_from_seed_indexed([0; 32], 0, U256::ONE, false)
                .public_key;
        let committee = genesis
            .network_config(&key)
            .context("the genesis file has no stake table")?
            .config;
        let l1_client = L1ClientOptions::default()
            .connect(l1)
            .context("connecting to L1")?;
        l1_client.spawn_tasks().await;
        let store = MemMembership::default();
        let fetcher = Fetcher::new(
            Arc::new(NullStateCatchup::default()),
            Arc::new(AsyncMutex::new(store.clone())),
            l1_client,
            genesis.chain_config,
        );
        fetcher.spawn_update_loop().await;
        let block_reward = fetcher.fetch_fixed_block_reward().await.ok();
        let committees = EpochCommittees::new(
            committee.known_nodes_with_stake,
            committee.known_da_nodes,
            block_reward,
            fetcher,
            epoch_height,
            first_epoch,
        )?;
        let epoch_length =
            NonZeroU64::new(epoch_height).context("genesis needs a positive epoch_height")?;
        Ok(Self {
            coordinator: EpochMembershipCoordinator::new(committees, epoch_length, &store),
            epoch_height,
        })
    }

    pub fn epoch_height(&self) -> u64 {
        self.epoch_height
    }

    /// The first epoch, which runs on the genesis stake table.
    pub fn first_epoch(&self) -> EpochNumber {
        self.coordinator.membership().first_epoch()
    }

    /// The epoch of the block at `height`.
    pub fn epoch_of(&self, height: u64) -> EpochNumber {
        EpochNumber::new(epoch_from_block_number(height, self.epoch_height))
    }

    /// The heights of the blocks of `epoch`.
    pub fn heights_of(&self, epoch: EpochNumber) -> Range<u64> {
        epoch_heights(epoch, self.epoch_height)
    }

    /// The height of the root block inside `epoch`, which gives the stake table of the epoch two
    /// later.
    pub fn root_of(&self, epoch: EpochNumber) -> u64 {
        root_block_in_epoch(*epoch, self.epoch_height)
    }

    /// Derives the stake table of the epoch two after `root`'s, from L1. `preceding` is the root of
    /// the epoch before `root`'s, if the store has it; without it the block reward assumes the
    /// block time, which the checks here do not depend on.
    pub async fn install_root(
        &self,
        root: Header,
        preceding: Option<Header>,
    ) -> anyhow::Result<()> {
        ensure!(
            is_epoch_root(root.height(), self.epoch_height),
            "block {} is no epoch root",
            root.height()
        );
        self.coordinator
            .membership()
            .install_epoch_roots(preceding, vec![root], &self.coordinator)
            .await
            .map_err(|err| anyhow::anyhow!("{err}"))
    }

    /// The stake table of `epoch`, if it is known.
    pub fn stake_table(&self, epoch: EpochNumber) -> Option<HSStakeTable<SeqTypes>> {
        let snapshot = self.coordinator.membership().snapshot(epoch)?;
        Some(HSStakeTable::from_iter(snapshot.stake_table()))
    }

    /// Checks that `cert2` decides `leaf`, and has a quorum of the stake table of `leaf`'s epoch.
    pub async fn check_cert2(
        &self,
        cert2: &Certificate2<SeqTypes>,
        leaf: &Leaf2,
    ) -> anyhow::Result<()> {
        let height = leaf.height();
        ensure!(
            cert2.data.block_number == height && cert2.data.leaf_commit == leaf.commit(),
            "a cert2 of block {} for block {height}",
            cert2.data.block_number
        );
        let epoch = self.epoch_of(height);
        ensure!(
            cert2.data.epoch == epoch,
            "a cert2 of epoch {} for a block of epoch {epoch}",
            cert2.data.epoch
        );
        let Some(table) = self.stake_table(epoch) else {
            bail!("no stake table of epoch {epoch}");
        };
        let table = Arc::new(StakeTable::from(table));
        StakeTableQuorum::new((table.clone(), table), self.epoch_height)
            .verify_cert2(cert2, leaf.block_header().version())
            .await
    }

    /// Checks that `cert`, the state certificate of the epoch root with `header`, has the
    /// signatures of the stake table of its epoch.
    pub fn check_state_cert(
        &self,
        cert: &LightClientStateUpdateCertificateV2<SeqTypes>,
        header: &Header,
    ) -> anyhow::Result<()> {
        let epoch = self.epoch_of(header.height());
        let Some(table) = self.stake_table(epoch) else {
            bail!("no stake table of epoch {epoch}");
        };
        // The block's version is the one its certificate was signed under.
        let lock = UpgradeLock::new(Upgrade::trivial(header.version()));
        validate_state_cert(cert, &table, epoch, self.epoch_height, &lock)
    }
}

/// The heights of the blocks of `epoch`, in epochs of `epoch_height` blocks: epoch 1 has blocks 1
/// up to `epoch_height`.
fn epoch_heights(epoch: EpochNumber, epoch_height: u64) -> Range<u64> {
    let end = *epoch * epoch_height + 1;
    end.saturating_sub(epoch_height)..end
}

/// What membership persists, in memory: a restarted API service derives it again from L1 and the
/// store.
#[derive(Clone, Debug, Default)]
struct MemMembership {
    inner: Arc<std::sync::Mutex<Kept>>,
}

#[derive(Debug, Default)]
struct Kept {
    stakes: BTreeMap<EpochNumber, StakeTuple>,
    epoch_roots: BTreeMap<EpochNumber, Header>,
    drb_results: BTreeMap<EpochNumber, DrbResult>,
    drb_inputs: BTreeMap<u64, DrbInput>,
    events: BTreeMap<EventKey, StakeTableEvent>,
    events_until: Option<u64>,
    validators: BTreeMap<EpochNumber, IndexMap<Address, RegisteredValidator<PubKey>>>,
}

impl MemMembership {
    fn with<R>(&self, f: impl FnOnce(&mut Kept) -> R) -> R {
        f(&mut self.inner.lock().unwrap())
    }
}

#[async_trait]
impl MembershipPersistence for MemMembership {
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        Ok(self.with(|kept| kept.stakes.get(&epoch).cloned()))
    }

    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        Ok(Some(self.with(|kept| {
            kept.stakes
                .iter()
                .rev()
                .take(limit as usize)
                .map(|(epoch, (validators, reward, hash))| {
                    (*epoch, (validators.clone(), *reward), *hash)
                })
                .collect()
        })))
    }

    async fn stored_stake_epochs(&self) -> anyhow::Result<BTreeSet<EpochNumber>> {
        Ok(self.with(|kept| kept.stakes.keys().copied().collect()))
    }

    async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        Ok(self.with(|kept| kept.drb_results.get(&epoch).copied()))
    }

    async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        Ok(self.with(|kept| kept.epoch_roots.get(&epoch).cloned()))
    }

    async fn store_epoch_root(&self, epoch: EpochNumber, header: Header) -> anyhow::Result<()> {
        self.with(|kept| kept.epoch_roots.insert(epoch, header));
        Ok(())
    }

    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        self.with(|kept| {
            let held = kept.stakes.get(&epoch);
            let reward = block_reward.or(held.and_then(|(_, reward, _)| *reward));
            let hash = stake_table_hash.or(held.and_then(|(_, _, hash)| *hash));
            kept.stakes.insert(epoch, (stake, reward, hash));
        });
        Ok(())
    }

    /// Keeps the later of the stored and the given `l1_finalized`, as the consensus store does.
    async fn store_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        self.with(|kept| {
            if kept.events_until > Some(l1_finalized) {
                return;
            }
            kept.events.extend(events);
            kept.events_until = Some(l1_finalized);
        });
        Ok(())
    }

    async fn load_events(
        &self,
        from_l1_block: u64,
        to_l1_block: u64,
    ) -> anyhow::Result<(
        Option<EventsPersistenceRead>,
        Vec<(EventKey, StakeTableEvent)>,
    )> {
        Ok(self.with(|kept| {
            let Some(until) = kept.events_until else {
                return (None, Vec::new());
            };
            let last = until.min(to_l1_block);
            let events = kept
                .events
                .range((from_l1_block, 0)..)
                .take_while(|((block, _), _)| *block <= last)
                .map(|(key, event)| (*key, event.clone()))
                .collect();
            let read = if last == to_l1_block {
                EventsPersistenceRead::Complete
            } else {
                EventsPersistenceRead::UntilL1Block(last)
            };
            (Some(read), events)
        }))
    }

    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        self.with(|kept| {
            kept.events.clear();
            kept.events_until = None;
            kept.validators.clear();
            kept.drb_results.clear();
            kept.epoch_roots.clear();
        });
        Ok(())
    }

    async fn store_all_validators(
        &self,
        epoch: EpochNumber,
        all_validators: IndexMap<Address, RegisteredValidator<PubKey>>,
    ) -> anyhow::Result<()> {
        self.with(|kept| kept.validators.insert(epoch, all_validators));
        Ok(())
    }

    async fn load_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        self.with(|kept| {
            let Some(map) = kept.validators.get(&epoch) else {
                bail!("no validators stored for epoch {epoch}");
            };
            let mut validators: Vec<_> = map.values().cloned().collect();
            validators.sort_by_key(|v| v.account);
            Ok(validators
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect())
        })
    }
}

/// Membership keeps only DRB results and epoch roots here; the rest of the trait is what a
/// consensus node persists, which the API service has none of.
#[async_trait]
impl Storage<SeqTypes> for MemMembership {
    async fn append_vid(
        &self,
        _: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!("the API service stores no VID shares")
    }

    async fn append_da2(
        &self,
        _: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        _: VidCommitment,
    ) -> anyhow::Result<()> {
        bail!("the API service stores no DA proposals")
    }

    async fn append_proposal_wrapper(
        &self,
        _: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!("the API service stores no proposals")
    }

    async fn record_action(&self, _: ViewNumber, _: HotShotAction) -> anyhow::Result<()> {
        bail!("the API service takes no consensus actions")
    }

    async fn update_state_cert(
        &self,
        _: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        bail!("the API service keeps state certificates in its store")
    }

    async fn update_decided_upgrade_certificate(
        &self,
        _: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!("the API service stores no upgrade certificate")
    }

    async fn store_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        self.with(|kept| kept.drb_results.insert(epoch, drb_result));
        Ok(())
    }

    async fn store_epoch_root(&self, epoch: EpochNumber, header: Header) -> anyhow::Result<()> {
        MembershipPersistence::store_epoch_root(self, epoch, header).await
    }

    async fn store_drb_input(&self, drb_input: DrbInput) -> anyhow::Result<()> {
        self.with(|kept| kept.drb_inputs.insert(drb_input.epoch, drb_input));
        Ok(())
    }

    async fn load_drb_input(&self, epoch: u64) -> anyhow::Result<DrbInput> {
        self.with(|kept| kept.drb_inputs.get(&epoch).cloned())
            .with_context(|| format!("no DRB progress stored for epoch {epoch}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_heights_agree_with_the_epoch_of_each_height() {
        let epoch_height = 100;
        for epoch in [1, 2, 37] {
            let epoch = EpochNumber::new(epoch);
            let heights = epoch_heights(epoch, epoch_height);
            assert_eq!(heights.end - heights.start, epoch_height);
            for height in [heights.start, heights.end - 1] {
                assert_eq!(epoch_from_block_number(height, epoch_height), *epoch);
            }
            assert_eq!(
                epoch_from_block_number(heights.end, epoch_height),
                *epoch + 1
            );
            assert!(heights.contains(&root_block_in_epoch(*epoch, epoch_height)));
        }
    }
}
