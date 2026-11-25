//! Membership in the application tables of a store.
//!
//! Consensus keeps the stake tables it builds, the DRB results and epoch roots they follow from,
//! and the L1 stake-table events it reads in its own store, next to its safety state, so that it
//! needs no SQL. Repair keeps the same in the chain store, where the API
//! instances read the stake tables, and a restart does not read L1 again. Per-epoch data is kept for the most recent [`RETAINED_EPOCHS`] epochs; older epochs
//! come from the API services, which keep history. The events are kept in full, since a stake
//! table is computed from all events before its epoch.

use std::{collections::BTreeSet, sync::LazyLock};

use anyhow::{Context, bail};
use app_tables::{AppTables, AppWrite, TableName};
use async_trait::async_trait;
use bytes::Bytes;
use espresso_types::{
    SeqTypes,
    traits::StakeTuple,
    v0_3::{EventKey, StakeTableEvent},
};
use hotshot_types::{
    data::EpochNumber,
    drb::{DrbInput, DrbResult},
    initializer::InitializerEpochInfo,
};

use crate::entities::{EpochMembership, MembershipEntities, StakeEvents};

/// How many of the latest epochs the store keeps stake tables, DRB data and epoch roots for.
pub const RETAINED_EPOCHS: u64 = espresso_types::RECENT_STAKE_TABLES_LIMIT;

/// Epoch to its stake table, block reward and stake table hash.
static STAKE: LazyLock<TableName> = LazyLock::new(|| TableName::of("stake"));

/// Epoch to every validator registered on L1 then, active or not.
static VALIDATORS: LazyLock<TableName> = LazyLock::new(|| TableName::of("validators"));

/// Epoch to its DRB result.
static DRB_RESULTS: LazyLock<TableName> = LazyLock::new(|| TableName::of("drb-results"));

/// Epoch to the progress of its DRB computation.
static DRB_INPUTS: LazyLock<TableName> = LazyLock::new(|| TableName::of("drb-inputs"));

/// Epoch to the header of its root block.
static EPOCH_ROOTS: LazyLock<TableName> = LazyLock::new(|| TableName::of("epoch-roots"));

/// `(L1 block, log index)` to a stake-table event.
static EVENTS: LazyLock<TableName> = LazyLock::new(|| TableName::of("stake-events"));

/// The L1 block up to which [`EVENTS`] holds every event, under [`EVENTS_UNTIL`].
static EVENTS_META: LazyLock<TableName> = LazyLock::new(|| TableName::of("stake-events-meta"));
const EVENTS_UNTIL: &[u8] = b"until";

/// The per-epoch tables, which keep only the latest epochs.
static PER_EPOCH: [&LazyLock<TableName>; 5] =
    [&STAKE, &VALIDATORS, &DRB_RESULTS, &DRB_INPUTS, &EPOCH_ROOTS];

/// The membership tables of a store: the consensus store of a consensus node, or the chain store
/// of an API service.
#[derive(Clone, Debug)]
pub struct MembershipStore<A> {
    store: A,
}

impl<A: AppTables> MembershipStore<A> {
    pub fn new(store: A) -> Self {
        Self { store }
    }

    async fn get<V: serde::de::DeserializeOwned + Send + 'static>(
        &self,
        table: &TableName,
        epoch: EpochNumber,
    ) -> anyhow::Result<Option<V>> {
        let bytes = self.store.get(table, &epoch_key(epoch)).await?;
        bytes
            .map(|b| bincode::deserialize(&b))
            .transpose()
            .with_context(|| format!("decoding {table} of epoch {epoch}"))
    }

    async fn write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()> {
        self.store.write(writes).await
    }

    /// The L1 block up to which the store holds every stake-table event.
    async fn events_until(&self) -> anyhow::Result<Option<u64>> {
        match self.store.get(&EVENTS_META, EVENTS_UNTIL).await? {
            Some(bytes) => {
                Ok(Some(u64::from_be_bytes(bytes[..].try_into().map_err(
                    |_| anyhow::anyhow!("malformed stake-event watermark"),
                )?)))
            },
            None => Ok(None),
        }
    }

    /// The DRB results the store holds, each with the root of its epoch if the store has it, for
    /// a restarted node to begin with.
    pub async fn start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>> {
        let results = self.store.entries(&DRB_RESULTS, &[], None).await?;
        let mut info = Vec::with_capacity(results.len());
        for (key, value) in results {
            let epoch = epoch_of(&key)?;
            let drb_result: DrbResult = bincode::deserialize(&value)
                .with_context(|| format!("decoding the DRB result of epoch {epoch}"))?;
            let block_header = self.get(&EPOCH_ROOTS, epoch).await?;
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
impl<A: AppTables + Clone + Send + Sync + 'static> MembershipEntities for MembershipStore<A> {
    async fn epoch(&self, epoch: EpochNumber) -> anyhow::Result<EpochMembership> {
        let validators = match self.store.get(&VALIDATORS, &epoch_key(epoch)).await? {
            Some(bytes) => Some(
                serde_json::from_slice(&bytes)
                    .with_context(|| format!("validators of epoch {epoch}"))?,
            ),
            None => None,
        };
        Ok(EpochMembership {
            stake: self.get(&STAKE, epoch).await?,
            validators,
            drb_result: self.get(&DRB_RESULTS, epoch).await?,
            root: self.get(&EPOCH_ROOTS, epoch).await?,
        })
    }

    async fn epochs(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, EpochMembership)>> {
        let mut epochs = BTreeSet::new();
        for table in [&*STAKE, &*VALIDATORS, &*DRB_RESULTS, &*EPOCH_ROOTS] {
            for (key, _) in self.store.entries(table, &[], None).await? {
                epochs.insert(epoch_of(&key)?);
            }
        }
        let mut latest = Vec::new();
        for epoch in epochs.into_iter().rev().take(limit) {
            latest.push((epoch, self.epoch(epoch).await?));
        }
        Ok(latest)
    }

    async fn latest_stakes(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, StakeTuple)>> {
        let entries = self.store.entries(&STAKE, &[], None).await?;
        entries
            .into_iter()
            .rev()
            .take(limit)
            .map(|(key, value)| {
                let epoch = epoch_of(&key)?;
                let stake = bincode::deserialize(&value)
                    .with_context(|| format!("stake of epoch {epoch}"))?;
                Ok((epoch, stake))
            })
            .collect()
    }

    async fn put_epoch(&self, epoch: EpochNumber, update: EpochMembership) -> anyhow::Result<()> {
        let mut writes = Vec::new();
        if let Some(stake) = &update.stake {
            writes.push(put(&STAKE, epoch, bincode::serialize(stake)?));
            if let Some(oldest) = (*epoch + 1).checked_sub(RETAINED_EPOCHS) {
                let before = Bytes::copy_from_slice(&epoch_key(EpochNumber::new(oldest)));
                writes.extend(PER_EPOCH.map(|table| AppWrite::DeleteBefore {
                    table: (**table).clone(),
                    key: before.clone(),
                }));
            }
        }
        if let Some(validators) = &update.validators {
            writes.push(put(&VALIDATORS, epoch, serde_json::to_vec(validators)?));
        }
        if let Some(drb_result) = &update.drb_result {
            writes.push(put(&DRB_RESULTS, epoch, bincode::serialize(drb_result)?));
        }
        if let Some(root) = &update.root {
            writes.push(put(&EPOCH_ROOTS, epoch, bincode::serialize(root)?));
        }
        self.write(writes).await
    }

    async fn events(&self, from: u64, to: u64) -> anyhow::Result<StakeEvents> {
        let Some(until) = self.events_until().await? else {
            return Ok(StakeEvents::default());
        };
        let last = until.min(to);
        let entries = self
            .store
            .entries(
                &EVENTS,
                &event_key(from, 0),
                last.checked_add(1)
                    .map(|next| event_key(next, 0))
                    .as_ref()
                    .map(|key| &key[..]),
            )
            .await?;
        let mut events = Vec::new();
        for (k, v) in entries {
            let (block, index) = decode_event_key(&k)?;
            let event: StakeTableEvent = serde_json::from_slice(&v)
                .with_context(|| format!("stake-table event at {block}/{index}"))?;
            events.push(((block, index), event));
        }
        Ok(StakeEvents {
            until: Some(until),
            events,
        })
    }

    async fn put_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        if self.events_until().await? > Some(l1_finalized) {
            return Ok(());
        }
        let mut writes = events
            .into_iter()
            .map(|((block, index), event)| {
                Ok(AppWrite::Put {
                    table: EVENTS.clone(),
                    key: Bytes::copy_from_slice(&event_key(block, index)),
                    value: serde_json::to_vec(&event)?.into(),
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        writes.push(AppWrite::Put {
            table: EVENTS_META.clone(),
            key: Bytes::from_static(EVENTS_UNTIL),
            value: Bytes::copy_from_slice(&l1_finalized.to_be_bytes()),
        });
        self.write(writes).await
    }

    async fn clear_stake_tables(&self) -> anyhow::Result<()> {
        self.write(
            [
                &*EVENTS,
                &*EVENTS_META,
                &*VALIDATORS,
                &*DRB_RESULTS,
                &*EPOCH_ROOTS,
            ]
            .map(|table| AppWrite::Clear {
                table: table.clone(),
            })
            .into(),
        )
        .await
    }

    async fn drb_input(&self, epoch: u64) -> anyhow::Result<Option<DrbInput>> {
        self.get(&DRB_INPUTS, EpochNumber::new(epoch)).await
    }

    async fn put_drb_input(&self, input: DrbInput) -> anyhow::Result<()> {
        let epoch = EpochNumber::new(input.epoch);
        if let Some(stored) = self.get::<DrbInput>(&DRB_INPUTS, epoch).await? {
            if stored.difficulty_level != input.difficulty_level {
                tracing::error!("Overwriting {stored:?} in storage with {input:?}");
            } else if stored.iteration >= input.iteration {
                bail!("DrbInput in storage {stored:?} is more recent than {input:?}");
            }
        }
        self.write(vec![put(&DRB_INPUTS, epoch, bincode::serialize(&input)?)])
            .await
    }
}

fn put(table: &TableName, epoch: EpochNumber, value: Vec<u8>) -> AppWrite {
    AppWrite::Put {
        table: table.clone(),
        key: Bytes::copy_from_slice(&epoch_key(epoch)),
        value: value.into(),
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
    use std::sync::Arc;

    use consensus::store::{ConsensusStore, MemStore};
    use espresso_types::traits::{EventsPersistenceRead, MembershipPersistence};
    use hotshot_types::traits::storage::Storage;

    use super::*;
    use crate::Persistence;

    fn store() -> MembershipStore<ConsensusStore<SeqTypes, MemStore>> {
        MembershipStore::new(ConsensusStore::new(Arc::new(MemStore::new())))
    }

    /// Runs each check on the consensus store and on the chain store, which must behave alike.
    macro_rules! on_both_stores {
        ($($check:ident),*) => {
            mod on_consensus_store {
                $(
                    #[tokio::test]
                    async fn $check() {
                        let store = super::store();
                        super::$check(store).await;
                    }
                )*
            }

            mod on_chain_store {
                use chain_store::{ChainStore, mem::MemBackend};

                $(
                    #[tokio::test]
                    async fn $check() {
                        let store = super::MembershipStore::new(ChainStore::new(MemBackend::default()));
                        super::$check(store).await;
                    }
                )*
            }
        };
    }

    on_both_stores!(
        keeps_the_latest_epochs,
        event_watermark,
        start_epoch_info_pairs_drb_results_with_roots,
        drb_progress_only_advances
    );

    /// Storing a stake table forgets the per-epoch data of epochs that fall out of the retained
    /// window, and the latest stake tables come newest first.
    async fn keeps_the_latest_epochs<A: AppTables + Clone + Send + Sync + 'static>(
        store: MembershipStore<A>,
    ) {
        let store = Persistence(store);
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
    async fn event_watermark<A: AppTables + Clone + Send + Sync + 'static>(
        store: MembershipStore<A>,
    ) {
        let store = Persistence(store);
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
    async fn start_epoch_info_pairs_drb_results_with_roots<
        A: AppTables + Clone + Send + Sync + 'static,
    >(
        store: MembershipStore<A>,
    ) {
        assert!(store.start_epoch_info().await.unwrap().is_empty());
        for epoch in [3, 4] {
            Storage::store_drb_result(
                &Persistence(store.clone()),
                EpochNumber::new(epoch),
                [epoch as u8; 32],
            )
            .await
            .unwrap();
        }
        let info = store.start_epoch_info().await.unwrap();
        let epochs: Vec<_> = info.iter().map(|i| (*i.epoch, i.drb_result[0])).collect();
        assert_eq!(epochs, vec![(3, 3), (4, 4)]);
        assert!(info.iter().all(|i| i.block_header.is_none()));
    }

    /// DRB progress is not replaced by older progress of the same computation.
    async fn drb_progress_only_advances<A: AppTables + Clone + Send + Sync + 'static>(
        store: MembershipStore<A>,
    ) {
        let store = Persistence(store);
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
