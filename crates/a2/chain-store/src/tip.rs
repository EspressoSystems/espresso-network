//! The tip of the chain as a reader of the store sees it: the newest heights without their
//! payloads.
//!
//! One [`Follower`] per process reads each new height from the store once and keeps the newest
//! [`KEPT`] heights in memory, where every waiting read and stream finds them. However many
//! clients wait at the tip, the store is read once per height and change. The follower publishes
//! what it holds as an immutable [`Tip`] on a `watch`, so readers take no lock.
//!
//! A tip holds no payloads, only what describes them ([`Parts::payload_info`]), so it stays small
//! whatever the size of the blocks. Over a backend that keeps a tip itself, such as the store
//! service, the follower takes the backend's updates instead of reading the store: the store
//! service reads each height once for all of its readers.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use futures::{StreamExt, stream::BoxStream};
use tokio::{sync::watch, time::sleep};

use crate::{ChainStore, Item, Parts, Record, TipUpdate};

/// How many of the newest heights a tip keeps.
pub const KEPT: u64 = 128;

/// How long the follower waits before reading the store again after a read failed.
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// A stored height: its item and the parts the store held of it when it was read, but the
/// payload, as decoded and as stored.
#[derive(Debug)]
pub struct Entry {
    pub item: Item,
    pub parts: Parts,
    pub record: Record,
}

impl Entry {
    fn of(record: Record) -> anyhow::Result<Self> {
        Ok(Self {
            item: record.item()?,
            parts: record.parts()?,
            record,
        })
    }
}

/// What the follower last learned of the store.
#[derive(Debug, Default)]
pub struct Tip {
    /// Whether the follower has learned of the store yet.
    pub read: bool,

    /// The newest stored height, if any.
    pub newest: Option<u64>,

    /// Every height before it is pruned.
    pub floor: u64,

    /// The stored heights from [`Tip::start`] on.
    pub heights: BTreeMap<u64, Arc<Entry>>,
}

impl Tip {
    /// The first height the tip covers: a height from it on that the tip lacks is not stored.
    pub fn start(&self) -> Option<u64> {
        self.newest
            .map(|newest| (newest + 1).saturating_sub(KEPT).max(self.floor))
    }
}

/// Whether `parts` has everything a reader of the tip waits for: what describes the payload,
/// which is stored with it, and the VID common data.
pub fn complete(parts: &Parts) -> bool {
    parts.payload_info.is_some() && parts.vid_common.is_some()
}

/// The follower of `store`, and the tips it publishes.
pub fn follower(store: ChainStore) -> (Follower, watch::Receiver<Arc<Tip>>) {
    let (tip, tips) = watch::channel(Arc::new(Tip::default()));
    (Follower { store, tip }, tips)
}

/// Keeps the newest heights of a store, and publishes them whenever they change.
#[derive(Debug)]
pub struct Follower {
    store: ChainStore,
    tip: watch::Sender<Arc<Tip>>,
}

impl Follower {
    /// Follows the store until its changes end: by the backend's updates if it sends them, else
    /// by reading the store whenever the chain in it changes.
    pub async fn run(self) {
        loop {
            let from = self.tip.borrow().start().unwrap_or(0);
            match self.store.tip_updates(from).await {
                Ok(Some(updates)) => {
                    if let Err(err) = self.take(updates).await {
                        tracing::warn!("following the store's tip failed: {err:#}");
                    }
                },
                Ok(None) => return self.read_on_changes().await,
                Err(err) => tracing::warn!("asking for the store's tip failed: {err:#}"),
            }
            sleep(RETRY_DELAY).await;
        }
    }

    /// Takes the backend's updates until they end, and publishes the tip after each update of the
    /// bounds, which closes the heights sent before it.
    async fn take(
        &self,
        mut updates: BoxStream<'static, anyhow::Result<TipUpdate>>,
    ) -> anyhow::Result<()> {
        let mut heights = self.tip.borrow().heights.clone();
        while let Some(update) = updates.next().await {
            match update? {
                TipUpdate::Height {
                    height,
                    item,
                    parts,
                } => {
                    let record = Record {
                        height,
                        item,
                        parts,
                    };
                    heights.insert(height, Arc::new(Entry::of(record)?));
                },
                TipUpdate::Bounds { newest, floor } => {
                    let mut tip = Tip {
                        read: true,
                        newest,
                        floor,
                        heights: BTreeMap::new(),
                    };
                    if let (Some(start), Some(newest)) = (tip.start(), newest) {
                        tip.heights = heights
                            .range(start..=newest)
                            .map(|(h, e)| (*h, e.clone()))
                            .collect();
                    }
                    heights = tip.heights.clone();
                    self.tip.send_replace(Arc::new(tip));
                },
            }
        }
        Ok(())
    }

    /// Reads the newest heights whenever the chain in the store changes.
    async fn read_on_changes(self) {
        let mut changes = self.store.changes();
        loop {
            // Marked seen before reading, so a write after the read is not missed.
            changes.borrow_and_update();
            if let Err(err) = self.read().await {
                tracing::warn!("reading the tip of the store failed: {err:#}");
                sleep(RETRY_DELAY).await;
                continue;
            }
            if changes.changed().await.is_err() {
                return;
            }
        }
    }

    /// Reads the newest heights and publishes them. A height kept with every part a reader
    /// waits for is taken as it is; the others, new or not yet complete, are read again. The
    /// height after the newest kept is read together with the newest and the floor, since it is
    /// almost always the one the change brought.
    async fn read(&self) -> anyhow::Result<()> {
        let current = self.tip.borrow().clone();
        let next = current.newest.map_or(0, |newest| newest + 1);
        let (newest, floor, next_record) = tokio::try_join!(
            self.store.newest(),
            self.store.floor(),
            self.store.summary(next),
        )?;
        let mut tip = Tip {
            read: true,
            newest,
            floor,
            heights: BTreeMap::new(),
        };
        let mut next_record = Some(next_record);
        if let (Some(start), Some(newest)) = (tip.start(), tip.newest) {
            for height in start..=newest {
                if let Some(entry) = current.heights.get(&height)
                    && complete(&entry.parts)
                {
                    tip.heights.insert(height, entry.clone());
                    continue;
                }
                let record = match height == next {
                    true => next_record.take().flatten(),
                    false => self.store.summary(height).await?,
                };
                if let Some(record) = record {
                    tip.heights.insert(height, Arc::new(Entry::of(record)?));
                }
            }
        }
        self.tip.send_replace(Arc::new(tip));
        Ok(())
    }
}

/// The updates that bring a reader that holds `sent` up to `tip`: each height from `from` on that
/// is new or has new parts, then the bounds. Notes what it returns in `sent`, and forgets heights
/// before the tip's start.
pub fn updates(tip: &Tip, from: u64, sent: &mut BTreeMap<u64, usize>) -> Vec<TipUpdate> {
    let mut updates = Vec::new();
    for (height, entry) in tip.heights.range(from..) {
        let parts = entry.record.parts.len();
        if sent.get(height) == Some(&parts) {
            continue;
        }
        sent.insert(*height, parts);
        updates.push(TipUpdate::Height {
            height: *height,
            item: entry.record.item.clone(),
            parts: entry.record.parts.clone(),
        });
    }
    if let Some(start) = tip.start() {
        sent.retain(|height, _| *height >= start);
    }
    updates.push(TipUpdate::Bounds {
        newest: tip.newest,
        floor: tip.floor,
    });
    updates
}

#[cfg(test)]
mod tests {
    use std::{
        ops::Range,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use bytes::Bytes;
    use espresso_types::{Leaf2, NodeState, Payload, SeqTypes, ValidatedState};
    use hotshot_query_service_types::availability::VidCommonQueryData;
    use hotshot_types::{
        simple_certificate::QuorumCertificate2,
        traits::{BlockPayload, EncodeBytes},
    };
    use tokio::task::JoinSet;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;
    use crate::{AppWrite, Backend, Column, KeyBounds, Part, Row, TableName, mem::MemBackend};

    /// How often the record of each height was read.
    type Reads = Arc<Mutex<BTreeMap<u64, usize>>>;

    fn reads(reads: &Reads, height: u64) -> usize {
        reads.lock().unwrap().get(&height).copied().unwrap_or(0)
    }

    /// A store in memory that counts the reads of each height's record.
    #[derive(Default)]
    struct Counting {
        inner: MemBackend,
        reads: Reads,
    }

    #[async_trait]
    impl Backend for Counting {
        async fn put(&self, row: Row, parts: Vec<(Part, Bytes)>) -> anyhow::Result<()> {
            self.inner.put(row, parts).await
        }

        async fn put_parts(&self, height: u64, parts: Vec<(Part, Bytes)>) -> anyhow::Result<()> {
            self.inner.put_parts(height, parts).await
        }

        async fn item(&self, height: u64) -> anyhow::Result<Option<Bytes>> {
            self.inner.item(height).await
        }

        async fn part(&self, height: u64, part: Part) -> anyhow::Result<Option<Bytes>> {
            self.inner.part(height, part).await
        }

        async fn record(
            &self,
            height: u64,
            payload: bool,
        ) -> anyhow::Result<Option<(Bytes, Vec<(Part, Bytes)>)>> {
            *self.reads.lock().unwrap().entry(height).or_default() += 1;
            self.inner.record(height, payload).await
        }

        async fn height_of(&self, column: Column, value: &str) -> anyhow::Result<Option<u64>> {
            self.inner.height_of(column, value).await
        }

        async fn newest(&self) -> anyhow::Result<Option<u64>> {
            self.inner.newest().await
        }

        async fn floor(&self) -> anyhow::Result<u64> {
            self.inner.floor().await
        }

        async fn missing_items(&self, from: u64, limit: usize) -> anyhow::Result<Vec<Range<u64>>> {
            self.inner.missing_items(from, limit).await
        }

        async fn newest_gap(&self) -> anyhow::Result<Option<Range<u64>>> {
            self.inner.newest_gap().await
        }

        async fn missing_parts(
            &self,
            part: Part,
            from: u64,
            limit: usize,
        ) -> anyhow::Result<Vec<u64>> {
            self.inner.missing_parts(part, from, limit).await
        }

        async fn app_get(&self, table: &TableName, key: &[u8]) -> anyhow::Result<Option<Bytes>> {
            self.inner.app_get(table, key).await
        }

        async fn app_entries(
            &self,
            table: &TableName,
            keys: KeyBounds<'_>,
        ) -> anyhow::Result<Vec<(Bytes, Bytes)>> {
            self.inner.app_entries(table, keys).await
        }

        async fn app_write(&self, writes: Vec<AppWrite>) -> anyhow::Result<()> {
            self.inner.app_write(writes).await
        }

        async fn prune_below(&self, height: u64) -> anyhow::Result<()> {
            self.inner.prune_below(height).await
        }

        async fn lease(&self, name: &str, holder: &str, ttl: Duration) -> anyhow::Result<bool> {
            self.inner.lease(name, holder, ttl).await
        }

        fn changes(&self) -> watch::Receiver<u64> {
            self.inner.changes()
        }
    }

    /// The genesis item moved to `height`, with a payload and VID common data, or none.
    async fn block(height: u64, with_parts: bool) -> (Item, Parts) {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let mut leaf = Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await;
        *leaf.block_header_mut().height_mut() = height;
        let cert1 =
            QuorumCertificate2::genesis(&state, &instance, Upgrade::trivial(NEW_PROTOCOL_VERSION))
                .await;
        let parts = match with_parts {
            true => Parts {
                payload: Some(Payload::empty().0.encode().to_vec().into()),
                vid_common: Some(
                    VidCommonQueryData::<SeqTypes>::genesis(
                        &state,
                        &instance,
                        NEW_PROTOCOL_VERSION,
                    )
                    .await
                    .common,
                ),
                ..Default::default()
            },
            false => Parts::default(),
        };
        (Item { leaf, cert1 }, parts)
    }

    /// Waits until the follower has published a tip with `height` complete.
    async fn until_complete(tips: &mut watch::Receiver<Arc<Tip>>, height: u64) -> Arc<Entry> {
        let tip = tips
            .wait_for(|tip| tip.heights.get(&height).is_some_and(|e| complete(&e.parts)))
            .await
            .unwrap()
            .clone();
        tip.heights[&height].clone()
    }

    /// Readers waiting at the tip cost one read of the new height, however many there are, and
    /// a height kept complete is not read again; one still incomplete is, until it is complete.
    /// The tip holds no payloads, only their info.
    #[tokio::test]
    async fn reads_each_height_once_for_every_reader() {
        let counting = Counting::default();
        let counts = counting.reads.clone();
        let store = ChainStore::new(counting);
        let (follower, mut tips) = follower(store.clone());
        let mut tasks = JoinSet::new();
        tasks.spawn(follower.run());

        let (item, parts) = block(0, false).await;
        store.put(&item, &parts).await.unwrap();
        tips.wait_for(|tip| tip.heights.contains_key(&0))
            .await
            .unwrap();

        let mut readers = JoinSet::new();
        for _ in 0..20 {
            let mut tips = tips.clone();
            readers.spawn(async move { until_complete(&mut tips, 1).await });
        }
        let (item, parts) = block(1, true).await;
        store.put(&item, &parts).await.unwrap();
        while let Some(entry) = readers.join_next().await {
            let entry = entry.unwrap();
            assert_eq!(entry.parts.payload, None, "no payload in the tip");
            assert!(entry.parts.payload_info.is_some());
        }
        assert_eq!(reads(&counts, 1), 1, "one read for every reader");
        let height_0 = reads(&counts, 0);
        assert!(height_0 >= 2, "an incomplete height is read again");

        let (_, parts) = block(0, true).await;
        store.put_parts(0, &parts).await.unwrap();
        until_complete(&mut tips, 0).await;
        let (item, parts) = block(2, true).await;
        store.put(&item, &parts).await.unwrap();
        until_complete(&mut tips, 2).await;
        assert_eq!(reads(&counts, 1), 1, "a complete height is kept as it is");
        assert_eq!(reads(&counts, 0), height_0 + 1, "read once more, complete");
    }

    /// Over the store service, a follower takes the service's updates: new heights, and heights
    /// that get their parts later, without their payloads.
    #[tokio::test]
    async fn follows_the_store_service() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let backend: Arc<dyn Backend> = Arc::new(MemBackend::default());
        let (service, service_follower) = crate::service::server(backend.clone());
        let mut tasks = JoinSet::new();
        tasks.spawn(service_follower);
        tasks.spawn(async move {
            let _ = tonic::transport::Server::builder()
                .add_service(service)
                .serve_with_incoming(tonic::transport::server::TcpIncoming::from(listener))
                .await;
        });
        let writer = ChainStore::shared(backend);
        let remote = ChainStore::new(crate::remote::RemoteBackend::connect(&url).await.unwrap());
        let (follower, mut tips) = follower(remote);
        tasks.spawn(follower.run());

        let (item, _) = block(0, false).await;
        writer.put(&item, &Parts::default()).await.unwrap();
        tips.wait_for(|tip| tip.heights.contains_key(&0))
            .await
            .unwrap();
        let (_, parts) = block(0, true).await;
        writer.put_parts(0, &parts).await.unwrap();
        let entry = until_complete(&mut tips, 0).await;
        assert_eq!(entry.parts.payload, None);
        let payload = parts.payload.unwrap();
        assert_eq!(
            entry.parts.payload_info.map(|info| info.size),
            Some(payload.len() as u64)
        );
        assert!(
            entry
                .record
                .parts
                .iter()
                .all(|(part, _)| *part != Part::Payload),
            "the payload is not sent"
        );
        assert_eq!(tips.borrow().newest, Some(0));
    }
}
