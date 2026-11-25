//! The store in the same process.

use std::{
    ops::{Range, RangeInclusive},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use chain_api::{BlocksFrontier, RewardSet, SentState, StateRow, Tables};
use chain_store::{
    ChainStore, Item, Lookup, Part, Parts, Record, TipUpdate, Totals,
    tip::{self, Follower, Tip},
};
use consensus_api::discovery::SignedServiceRecord;
use espresso_types::{
    ChainConfig, FeeAccount, FeeAmount, SeqTypes,
    traits::StakeTuple,
    v0_3::{EventKey, StakeTableEvent},
};
use futures::stream::BoxStream;
use hotshot_types::{
    PeerConfig,
    data::EpochNumber,
    drb::DrbInput,
    simple_certificate::{Certificate2, LightClientStateUpdateCertificateV2},
};
use membership::{EpochMembership, MembershipEntities, MembershipStore, StakeEvents};
use tokio::sync::watch;

use crate::{INLINE_LIMIT, PayloadAt, PayloadData, PayloadStatus, Store};

/// A [`ChainStore`], with its tables and membership. Clones share the store and its tip.
#[derive(Clone, Debug)]
pub struct LocalStore {
    store: ChainStore,

    /// What blocks and parts are written through: the store, or the same store uploading
    /// payloads at once rather than staging them; see [`Self::writing_through`].
    writer: ChainStore,
    tables: Tables,
    membership: MembershipStore<ChainStore>,

    /// The newest heights, which every follower of the store's tip shares.
    tips: watch::Receiver<Arc<Tip>>,

    /// The largest payload, or range of one, answered with its bytes when it could be answered
    /// with where to fetch them.
    inline_limit: u64,
}

impl LocalStore {
    /// `store`, with the follower of its newest heights, which must run as long as the store's
    /// tip is followed.
    pub fn new(store: ChainStore) -> (Self, Follower) {
        let (follower, tips) = tip::follower(store.clone());
        let local = Self {
            tables: Tables::new(store.clone()),
            membership: MembershipStore::new(store.clone()),
            writer: store.clone(),
            store,
            tips,
            inline_limit: INLINE_LIMIT,
        };
        (local, follower)
    }

    /// The store, writing the blocks and parts its callers put through `writer`, the same store
    /// over the same backend. A storage instance stages the payloads of the decide log, but has
    /// its callers' payloads, which repair fetched, uploaded at once: a staged payload counts as
    /// missing until it is uploaded, and staging is private to one instance.
    pub fn writing_through(self, writer: ChainStore) -> Self {
        Self { writer, ..self }
    }

    /// The store, answering payloads, or ranges of them, of more than `bytes` with where to
    /// fetch them, if clients can reach the object store.
    pub fn with_inline_limit(self, bytes: u64) -> Self {
        Self {
            inline_limit: bytes,
            ..self
        }
    }

    /// `store`, with its follower running on the current runtime until the store's changes end.
    pub fn spawn(store: ChainStore) -> Arc<Self> {
        let (local, follower) = Self::new(store);
        tokio::spawn(follower.run());
        Arc::new(local)
    }

    /// Where clients fetch the payload `at` names, if the store's payloads live where clients
    /// reach them, the payload is uploaded, and the answer, all of it or `range`, is too large to
    /// send inline.
    async fn payload_url(
        &self,
        at: &PayloadAt,
        range: Option<&Range<u64>>,
    ) -> anyhow::Result<Option<String>> {
        if range.is_some_and(|range| range.end.saturating_sub(range.start) <= self.inline_limit) {
            return Ok(None);
        }
        let Some(payloads) = self.store.payload_store() else {
            return Ok(None);
        };
        let commitment = match at {
            PayloadAt::Commitment(commitment) => commitment.clone(),
            PayloadAt::Height(height) => match self.store.item(*height).await? {
                Some(item) => item.leaf.payload_commitment().to_string(),
                None => return Ok(None),
            },
        };
        // A payload kept in memory is sent from there.
        if self.store.is_recent(&commitment) {
            return Ok(None);
        }
        let Some(url) = payloads.url(&commitment) else {
            return Ok(None);
        };
        let Some(record) = self.store.payload_record(&commitment).await? else {
            return Ok(None);
        };
        let large = range.is_some() || record.info.size > self.inline_limit;
        Ok((record.uploaded && large).then_some(url))
    }
}

#[async_trait]
impl Store for LocalStore {
    async fn newest(&self) -> anyhow::Result<Option<u64>> {
        self.store.newest().await
    }

    async fn floor(&self) -> anyhow::Result<u64> {
        self.store.floor().await
    }

    async fn tip_updates(
        &self,
        from: u64,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<TipUpdate>>> {
        Ok(tip::updates_of(self.tips.clone(), from))
    }

    async fn record(&self, height: u64, payload: bool) -> anyhow::Result<Option<Record>> {
        match payload {
            true => self.store.record(height).await,
            false => self.store.summary(height).await,
        }
    }

    async fn item(&self, height: u64) -> anyhow::Result<Option<Item>> {
        self.store.item(height).await
    }

    async fn cert2(&self, height: u64) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        self.store.cert2(height).await
    }

    async fn state_cert(
        &self,
        height: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        self.store.state_cert(height).await
    }

    async fn locate(&self, lookup: &Lookup) -> anyhow::Result<Option<u64>> {
        self.store.height_of(lookup).await
    }

    async fn payload(
        &self,
        at: &PayloadAt,
        range: Option<Range<u64>>,
    ) -> anyhow::Result<Option<PayloadData>> {
        // A store that keeps its payloads in its database knows them by height only.
        let by_height;
        let at = match at {
            PayloadAt::Commitment(commitment) if self.store.payload_store().is_none() => {
                let Ok(commitment) = commitment.parse() else {
                    return Ok(None);
                };
                let lookup = Lookup::Payload(commitment);
                match self.store.height_of(&lookup).await? {
                    Some(height) => {
                        by_height = PayloadAt::Height(height);
                        &by_height
                    },
                    None => return Ok(None),
                }
            },
            at => at,
        };
        if let Some(url) = self.payload_url(at, range.as_ref()).await? {
            return Ok(Some(PayloadData::Url(url)));
        }
        let bytes = match (at, range) {
            (PayloadAt::Height(height), None) => self.store.payload(*height).await?,
            (PayloadAt::Height(height), Some(range)) => {
                self.store.payload_range(*height, range).await?
            },
            (PayloadAt::Commitment(commitment), None) => {
                self.store.payload_by_commitment(commitment).await?
            },
            (PayloadAt::Commitment(commitment), Some(range)) => {
                self.store
                    .payload_range_by_commitment(commitment, range)
                    .await?
            },
        };
        Ok(bytes.map(PayloadData::Inline))
    }

    async fn payload_status(&self, commitment: &str) -> anyhow::Result<PayloadStatus> {
        let has = self.store.has_payload(commitment).await?;
        let held = has || self.store.holds_payload(commitment).await?;
        Ok(PayloadStatus { has, held })
    }

    async fn transaction_position(&self, hash: &str) -> anyhow::Result<Option<(u64, u64)>> {
        let Ok(hash) = hash.parse() else {
            return Ok(None);
        };
        self.store.transaction_position(&hash).await
    }

    async fn totals(
        &self,
        heights: RangeInclusive<u64>,
        namespace: Option<u32>,
    ) -> anyhow::Result<Totals> {
        self.store.totals(heights, namespace).await
    }

    async fn missing_items(&self, from: u64, limit: usize) -> anyhow::Result<Vec<Range<u64>>> {
        self.store.missing_items(from, limit).await
    }

    async fn newest_gap(&self) -> anyhow::Result<Option<Range<u64>>> {
        self.store.newest_gap().await
    }

    async fn missing_parts(&self, part: Part, from: u64, limit: usize) -> anyhow::Result<Vec<u64>> {
        self.store.missing_parts(part, from, limit).await
    }

    async fn put_block(&self, item: &Item, parts: &Parts) -> anyhow::Result<()> {
        self.writer.put(item, parts).await
    }

    async fn put_parts(&self, height: u64, parts: &Parts) -> anyhow::Result<()> {
        self.writer.put_parts(height, parts).await
    }

    async fn lease(&self, name: &str, holder: &str, ttl: Duration) -> anyhow::Result<bool> {
        self.store.lease(name, holder, ttl).await
    }

    async fn state(&self, height: u64) -> anyhow::Result<Option<StateRow>> {
        self.tables.state(height).await
    }

    async fn states(&self) -> anyhow::Result<Vec<(u64, StateRow)>> {
        self.tables.states().await
    }

    async fn rewards(&self, at: u64) -> anyhow::Result<Option<RewardSet>> {
        self.tables.rewards(at).await
    }

    async fn sent_state(&self, height: u64) -> anyhow::Result<Option<SentState>> {
        self.tables.sent_state(height).await
    }

    async fn put_state(
        &self,
        height: u64,
        frontier: Option<BlocksFrontier>,
        chain_config: ChainConfig,
        rewards: Option<RewardSet>,
    ) -> anyhow::Result<bool> {
        self.tables
            .put_state(height, frontier, chain_config, rewards.as_ref())
            .await
    }

    async fn chain_config(&self, commitment: &str) -> anyhow::Result<Option<ChainConfig>> {
        self.tables.chain_config(commitment).await
    }

    async fn put_chain_config(&self, config: &ChainConfig) -> anyhow::Result<()> {
        self.tables.put_chain_config(config).await
    }

    async fn fee_accounts(&self) -> anyhow::Result<Option<Vec<(FeeAccount, FeeAmount)>>> {
        self.tables.fee_accounts().await
    }

    async fn put_fee_accounts(&self, accounts: &[(FeeAccount, FeeAmount)]) -> anyhow::Result<()> {
        self.tables.put_fee_accounts(accounts).await
    }

    async fn stake_table(
        &self,
        epoch: EpochNumber,
    ) -> anyhow::Result<Option<Vec<PeerConfig<SeqTypes>>>> {
        self.tables.stake_table(epoch).await
    }

    async fn service_records(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Option<(EpochNumber, Vec<SignedServiceRecord<SeqTypes>>)>> {
        self.tables.service_records(epoch).await
    }

    async fn epoch(&self, epoch: EpochNumber) -> anyhow::Result<EpochMembership> {
        self.membership.epoch(epoch).await
    }

    async fn epochs(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, EpochMembership)>> {
        self.membership.epochs(limit).await
    }

    async fn latest_stakes(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, StakeTuple)>> {
        self.membership.latest_stakes(limit).await
    }

    async fn put_epoch(&self, epoch: EpochNumber, update: EpochMembership) -> anyhow::Result<()> {
        self.membership.put_epoch(epoch, update).await
    }

    async fn events(&self, from: u64, to: u64) -> anyhow::Result<StakeEvents> {
        self.membership.events(from, to).await
    }

    async fn put_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        self.membership.put_events(l1_finalized, events).await
    }

    async fn clear_stake_tables(&self) -> anyhow::Result<()> {
        self.membership.clear_stake_tables().await
    }

    async fn drb_input(&self, epoch: u64) -> anyhow::Result<Option<DrbInput>> {
        self.membership.drb_input(epoch).await
    }

    async fn put_drb_input(&self, input: DrbInput) -> anyhow::Result<()> {
        self.membership.put_drb_input(input).await
    }
}
