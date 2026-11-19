//! Membership data in the consensus store.
//!
//! Consensus keeps the stake tables it builds, the DRB results and epoch roots they follow from,
//! and the L1 stake-table events it reads in its own store, next to its safety state, so that it
//! needs no SQL. Per-epoch data is kept for the most recent [`RETAINED_EPOCHS`] epochs; older
//! epochs come from the API services, which keep history. The events are kept in full, since a
//! stake table is computed from all events before its epoch.

use std::collections::BTreeSet;

use alloy::primitives::Address;
use anyhow::{Context, bail};
use async_trait::async_trait;
use consensus::store::{AppWrite, ConsensusStore};
use espresso_types::{
    AuthenticatedValidatorMap, Header, PubKey, RegisteredValidatorMap, SeqTypes, StakeTableHash,
    traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
    v0_3::{EventKey, IndexedStake, RegisteredValidator, RewardAmount, StakeTableEvent},
};
use hotshot_types::{
    data::{
        DaProposal2, EpochNumber, QuorumProposalWrapper, VidCommitment, VidDisperseShare,
        ViewNumber,
    },
    drb::{DrbInput, DrbResult},
    event::HotShotAction,
    initializer::InitializerEpochInfo,
    message::Proposal,
    simple_certificate::{LightClientStateUpdateCertificateV2, UpgradeCertificate},
    traits::storage::Storage,
};
use indexmap::IndexMap;

/// How many of the latest epochs the store keeps stake tables, DRB data and epoch roots for.
pub const RETAINED_EPOCHS: u64 = espresso_types::RECENT_STAKE_TABLES_LIMIT;

/// Epoch to its stake table, block reward and stake table hash.
const STAKE: &str = "stake";

/// Epoch to every validator registered on L1 then, active or not.
const VALIDATORS: &str = "validators";

/// Epoch to its DRB result.
const DRB_RESULTS: &str = "drb-results";

/// Epoch to the progress of its DRB computation.
const DRB_INPUTS: &str = "drb-inputs";

/// Epoch to the header of its root block.
const EPOCH_ROOTS: &str = "epoch-roots";

/// `(L1 block, log index)` to a stake-table event.
const EVENTS: &str = "stake-events";

/// The L1 block up to which [`EVENTS`] holds every event, under [`EVENTS_UNTIL`].
const EVENTS_META: &str = "stake-events-meta";
const EVENTS_UNTIL: &[u8] = b"until";

/// The per-epoch tables, which keep only the latest epochs.
const PER_EPOCH: [&str; 5] = [STAKE, VALIDATORS, DRB_RESULTS, DRB_INPUTS, EPOCH_ROOTS];

/// The membership tables of a consensus store.
#[derive(Clone)]
pub struct MembershipStore {
    store: ConsensusStore<SeqTypes>,
}

impl MembershipStore {
    pub fn new(store: ConsensusStore<SeqTypes>) -> Self {
        Self { store }
    }

    /// Runs `f` on the store on a blocking thread.
    async fn blocking<R: Send + 'static>(
        &self,
        f: impl FnOnce(&ConsensusStore<SeqTypes>) -> anyhow::Result<R> + Send + 'static,
    ) -> anyhow::Result<R> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || f(&store)).await?
    }

    async fn get<V: serde::de::DeserializeOwned + Send + 'static>(
        &self,
        table: &'static str,
        epoch: EpochNumber,
    ) -> anyhow::Result<Option<V>> {
        let bytes = self
            .blocking(move |s| s.app_get(table, &epoch_key(epoch)))
            .await?;
        bytes
            .map(|b| bincode::deserialize(&b))
            .transpose()
            .with_context(|| format!("decoding {table} of epoch {epoch}"))
    }

    async fn write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()> {
        self.blocking(move |s| s.app_write(&writes)).await
    }

    /// The DRB results the store holds, each with the root of its epoch if the store has it, for
    /// a restarted node to begin with.
    pub async fn start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>> {
        let results = self.blocking(|s| s.app_entries(DRB_RESULTS, &[])).await?;
        let mut info = Vec::with_capacity(results.len());
        for (key, value) in results {
            let epoch = epoch_of(&key)?;
            let drb_result: DrbResult = bincode::deserialize(&value)
                .with_context(|| format!("decoding the DRB result of epoch {epoch}"))?;
            let block_header = self.get(EPOCH_ROOTS, epoch).await?;
            info.push(InitializerEpochInfo {
                epoch,
                drb_result,
                block_header,
            });
        }
        Ok(info)
    }
}

#[async_trait]
impl MembershipPersistence for MembershipStore {
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        self.get(STAKE, epoch).await
    }

    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        let entries = self.blocking(|s| s.app_entries(STAKE, &[])).await?;
        let mut stakes = entries
            .into_iter()
            .rev()
            .take(limit as usize)
            .map(|(k, v)| {
                let epoch = epoch_of(&k)?;
                let (validators, reward, hash): StakeTuple =
                    bincode::deserialize(&v).with_context(|| format!("stake of epoch {epoch}"))?;
                Ok((epoch, (validators, reward), hash))
            })
            .collect::<anyhow::Result<Vec<IndexedStake>>>()?;
        stakes.sort_by_key(|(epoch, ..)| std::cmp::Reverse(*epoch));
        Ok(Some(stakes))
    }

    async fn stored_stake_epochs(&self) -> anyhow::Result<BTreeSet<EpochNumber>> {
        let entries = self.blocking(|s| s.app_entries(STAKE, &[])).await?;
        entries.iter().map(|(k, _)| epoch_of(k)).collect()
    }

    async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        self.get(DRB_RESULTS, epoch).await
    }

    async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        self.get(EPOCH_ROOTS, epoch).await
    }

    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: Header,
    ) -> anyhow::Result<()> {
        self.write(vec![put(
            EPOCH_ROOTS,
            epoch,
            bincode::serialize(&block_header)?,
        )])
        .await
    }

    /// Stores the stake table of `epoch` and forgets the per-epoch data of epochs more than
    /// [`RETAINED_EPOCHS`] before it.
    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        let stake: StakeTuple = (stake, block_reward, stake_table_hash);
        let mut writes = vec![put(STAKE, epoch, bincode::serialize(&stake)?)];
        if let Some(oldest) = (*epoch + 1).checked_sub(RETAINED_EPOCHS) {
            let before = epoch_key(EpochNumber::new(oldest)).to_vec();
            writes.extend(PER_EPOCH.map(|table| AppWrite::DeleteBefore {
                table,
                key: before.clone(),
            }));
        }
        self.write(writes).await
    }

    /// Stores `events` and that every event up to `l1_finalized` is stored, unless the store
    /// already holds events up to a later block.
    async fn store_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        let mut writes = events
            .into_iter()
            .map(|((block, index), event)| {
                Ok(AppWrite::Put {
                    table: EVENTS,
                    key: event_key(block, index).to_vec(),
                    value: serde_json::to_vec(&event)?,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        writes.push(AppWrite::Put {
            table: EVENTS_META,
            key: EVENTS_UNTIL.to_vec(),
            value: l1_finalized.to_be_bytes().to_vec(),
        });
        self.blocking(move |s| {
            if events_until(s)? > Some(l1_finalized) {
                return Ok(());
            }
            s.app_write(&writes)
        })
        .await
    }

    async fn load_events(
        &self,
        from_l1_block: u64,
        to_l1_block: u64,
    ) -> anyhow::Result<(
        Option<EventsPersistenceRead>,
        Vec<(EventKey, StakeTableEvent)>,
    )> {
        let (until, entries) = self
            .blocking(move |s| {
                let until = events_until(s)?;
                let entries = s.app_entries(EVENTS, &event_key(from_l1_block, 0))?;
                Ok((until, entries))
            })
            .await?;
        let Some(until) = until else {
            return Ok((None, Vec::new()));
        };
        let last = until.min(to_l1_block);
        let mut events = Vec::new();
        for (k, v) in entries {
            let (block, index) = decode_event_key(&k)?;
            if block > last {
                break;
            }
            let event: StakeTableEvent = serde_json::from_slice(&v)
                .with_context(|| format!("stake-table event at {block}/{index}"))?;
            events.push(((block, index), event));
        }
        let read = if last == to_l1_block {
            EventsPersistenceRead::Complete
        } else {
            EventsPersistenceRead::UntilL1Block(last)
        };
        Ok((Some(read), events))
    }

    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        self.write(vec![
            AppWrite::Clear { table: EVENTS },
            AppWrite::Clear { table: EVENTS_META },
            AppWrite::Clear { table: VALIDATORS },
            AppWrite::Clear { table: DRB_RESULTS },
            AppWrite::Clear { table: EPOCH_ROOTS },
        ])
        .await
    }

    async fn store_all_validators(
        &self,
        epoch: EpochNumber,
        all_validators: IndexMap<Address, RegisteredValidator<PubKey>>,
    ) -> anyhow::Result<()> {
        self.write(vec![put(
            VALIDATORS,
            epoch,
            serde_json::to_vec(&all_validators)?,
        )])
        .await
    }

    async fn load_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        let Some(bytes) = self
            .blocking(move |s| s.app_get(VALIDATORS, &epoch_key(epoch)))
            .await?
        else {
            bail!("no validators stored for epoch {epoch}");
        };
        let map: RegisteredValidatorMap = serde_json::from_slice(&bytes)
            .with_context(|| format!("validators of epoch {epoch}"))?;
        let mut validators: Vec<_> = map.into_values().collect();
        validators.sort_by_key(|v| v.account);
        Ok(validators
            .into_iter()
            .skip(offset as usize)
            .take(limit as usize)
            .collect())
    }
}

/// The DRB storage of the membership coordinator. Consensus keeps everything else this trait
/// covers in its own tables, so those methods fail.
#[async_trait]
impl Storage<SeqTypes> for MembershipStore {
    async fn append_vid(
        &self,
        _: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!("consensus keeps VID shares in its own tables")
    }

    async fn append_da2(
        &self,
        _: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        _: VidCommitment,
    ) -> anyhow::Result<()> {
        bail!("consensus keeps payloads in its own tables")
    }

    async fn append_proposal_wrapper(
        &self,
        _: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!("consensus keeps proposals in its own tables")
    }

    async fn record_action(&self, _: ViewNumber, _: HotShotAction) -> anyhow::Result<()> {
        bail!("consensus keeps its actions in its own tables")
    }

    async fn update_state_cert(
        &self,
        _: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        bail!("consensus keeps state certificates in its own tables")
    }

    async fn update_decided_upgrade_certificate(
        &self,
        _: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!("consensus keeps the upgrade certificate in its own tables")
    }

    async fn store_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        self.write(vec![put(
            DRB_RESULTS,
            epoch,
            bincode::serialize(&drb_result)?,
        )])
        .await
    }

    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: Header,
    ) -> anyhow::Result<()> {
        MembershipPersistence::store_epoch_root(self, epoch, block_header).await
    }

    /// Refuses to replace the progress of the same computation with older progress.
    async fn store_drb_input(&self, drb_input: DrbInput) -> anyhow::Result<()> {
        let epoch = EpochNumber::new(drb_input.epoch);
        if let Some(stored) = self.get::<DrbInput>(DRB_INPUTS, epoch).await? {
            if stored.difficulty_level != drb_input.difficulty_level {
                tracing::error!("Overwriting {stored:?} in storage with {drb_input:?}");
            } else if stored.iteration >= drb_input.iteration {
                bail!("DrbInput in storage {stored:?} is more recent than {drb_input:?}");
            }
        }
        self.write(vec![put(
            DRB_INPUTS,
            epoch,
            bincode::serialize(&drb_input)?,
        )])
        .await
    }

    async fn load_drb_input(&self, epoch: u64) -> anyhow::Result<DrbInput> {
        self.get(DRB_INPUTS, EpochNumber::new(epoch))
            .await?
            .with_context(|| format!("no DRB progress stored for epoch {epoch}"))
    }
}

fn put(table: &'static str, epoch: EpochNumber, value: Vec<u8>) -> AppWrite {
    AppWrite::Put {
        table,
        key: epoch_key(epoch).to_vec(),
        value,
    }
}

/// The L1 block up to which the store holds every stake-table event.
fn events_until(store: &ConsensusStore<SeqTypes>) -> anyhow::Result<Option<u64>> {
    match store.app_get(EVENTS_META, EVENTS_UNTIL)? {
        Some(bytes) => {
            Ok(Some(u64::from_be_bytes(bytes.try_into().map_err(
                |_| anyhow::anyhow!("malformed stake-event watermark"),
            )?)))
        },
        None => Ok(None),
    }
}

fn epoch_key(epoch: EpochNumber) -> [u8; 8] {
    epoch.u64().to_be_bytes()
}

fn epoch_of(key: &[u8]) -> anyhow::Result<EpochNumber> {
    let bytes: [u8; 8] = key.try_into().context("malformed epoch key")?;
    Ok(EpochNumber::new(u64::from_be_bytes(bytes)))
}

fn event_key(block: u64, index: u64) -> [u8; 16] {
    let mut key = [0; 16];
    key[..8].copy_from_slice(&block.to_be_bytes());
    key[8..].copy_from_slice(&index.to_be_bytes());
    key
}

fn decode_event_key(key: &[u8]) -> anyhow::Result<EventKey> {
    let bytes: [u8; 16] = key.try_into().context("malformed event key")?;
    let (block, index) = bytes.split_at(8);
    Ok((
        u64::from_be_bytes(block.try_into()?),
        u64::from_be_bytes(index.try_into()?),
    ))
}

#[cfg(test)]
mod tests {
    use consensus::store::Import;

    use super::*;

    async fn store() -> (tempfile::TempDir, MembershipStore) {
        let dir = tempfile::tempdir().unwrap();
        let (store, _) = ConsensusStore::open(dir.path().join("store"), async || {
            Ok(Import {
                proposals: Vec::new(),
                locked_qc: None,
                boundary_qc: None,
                acted_view: None,
                restart_view: None,
                upgrade_cert: None,
                state_cert: None,
                payloads: Vec::new(),
                anchor: None,
                app: Vec::new(),
            })
        })
        .await
        .unwrap();
        (dir, MembershipStore::new(store))
    }

    /// Storing a stake table forgets the per-epoch data of epochs that fall out of the retained
    /// window, and the latest stake tables come newest first.
    #[tokio::test]
    async fn keeps_the_latest_epochs() {
        let (_dir, store) = store().await;
        let newest = RETAINED_EPOCHS + 2;
        for epoch in 1..=newest {
            let epoch = EpochNumber::new(epoch);
            store
                .store_stake(epoch, Default::default(), None, None)
                .await
                .unwrap();
            Storage::store_drb_result(&store, epoch, [epoch.u64() as u8; 32])
                .await
                .unwrap();
        }
        let oldest = newest + 1 - RETAINED_EPOCHS;
        for epoch in 1..oldest {
            let epoch = EpochNumber::new(epoch);
            assert!(store.load_stake(epoch).await.unwrap().is_none(), "{epoch}");
            assert!(
                MembershipPersistence::load_drb_result(&store, epoch)
                    .await
                    .unwrap()
                    .is_none(),
                "{epoch}"
            );
        }
        assert!(
            MembershipPersistence::load_drb_result(&store, EpochNumber::new(oldest))
                .await
                .unwrap()
                .is_some()
        );
        let latest = store.load_latest_stake(3).await.unwrap().unwrap();
        let epochs: Vec<_> = latest.iter().map(|(epoch, ..)| epoch.u64()).collect();
        assert_eq!(epochs, vec![newest, newest - 1, newest - 2]);
    }

    /// The event watermark only moves forward, and a read past it says how far it got.
    #[tokio::test]
    async fn event_watermark() {
        let (_dir, store) = store().await;
        assert_eq!(store.load_events(0, 10).await.unwrap().0, None);

        store.store_events(10, Vec::new()).await.unwrap();
        store.store_events(5, Vec::new()).await.unwrap();
        assert_eq!(
            store.load_events(0, 20).await.unwrap().0,
            Some(EventsPersistenceRead::UntilL1Block(10))
        );
        assert_eq!(
            store.load_events(0, 8).await.unwrap().0,
            Some(EventsPersistenceRead::Complete)
        );
    }

    /// A restarted node begins with the DRB results stored, and the roots stored with them.
    #[tokio::test]
    async fn start_epoch_info_pairs_drb_results_with_roots() {
        let (_dir, store) = store().await;
        assert!(store.start_epoch_info().await.unwrap().is_empty());
        for epoch in [3, 4] {
            Storage::store_drb_result(&store, EpochNumber::new(epoch), [epoch as u8; 32])
                .await
                .unwrap();
        }
        let info = store.start_epoch_info().await.unwrap();
        let epochs: Vec<_> = info.iter().map(|i| (*i.epoch, i.drb_result[0])).collect();
        assert_eq!(epochs, vec![(3, 3), (4, 4)]);
        assert!(info.iter().all(|i| i.block_header.is_none()));
    }

    /// DRB progress is not replaced by older progress of the same computation.
    #[tokio::test]
    async fn drb_progress_only_advances() {
        let (_dir, store) = store().await;
        let input = |iteration| DrbInput {
            epoch: 3,
            iteration,
            value: [0; 32],
            difficulty_level: 10,
        };
        store.store_drb_input(input(5)).await.unwrap();
        store.store_drb_input(input(4)).await.unwrap_err();
        assert_eq!(store.load_drb_input(3).await.unwrap().iteration, 5);
        store.load_drb_input(4).await.unwrap_err();
    }
}
