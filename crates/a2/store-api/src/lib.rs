//! What an API instance reads from and writes to its deployment's store.
//!
//! The API instances never reach the store's database or its object store. They get and put the
//! store's domain entities through a [`Store`]: [`LocalStore`] in the process that holds the
//! store, or [`RemoteStore`], a client of the [`server`] that the storage instances run over their
//! own [`LocalStore`]. The storage instances own how the entities are kept: the encodings, the
//! tables, and which writes commit together.

use std::{
    ops::{Range, RangeInclusive},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use chain_api::{BlocksFrontier, RewardSet, SentState, StateRow};
use chain_store::{Item, Lookup, Part, Parts, Record, TipUpdate, Totals};
use consensus_api::discovery::SignedServiceRecord;
use espresso_types::{
    ChainConfig, FeeAccount, FeeAmount, SeqTypes,
    traits::StakeTuple,
    v0_3::{EventKey, StakeTableEvent},
};
use futures::{StreamExt, stream::BoxStream};
use hotshot_types::{
    PeerConfig,
    data::EpochNumber,
    drb::DrbInput,
    simple_certificate::{Certificate2, LightClientStateUpdateCertificateV2},
};
use membership::{EpochMembership, MembershipEntities, StakeEvents};
use tokio::{sync::watch, time::sleep};

mod client;
mod local;
pub mod server;
#[cfg(test)]
mod tests;
mod wire;

pub use client::RemoteStore;
pub use local::LocalStore;

/// A deployment's store, by its domain entities.
#[async_trait]
pub trait Store: std::fmt::Debug + Send + Sync + 'static {
    /// The newest height with an item, if any.
    async fn newest(&self) -> anyhow::Result<Option<u64>>;

    /// The first height the store keeps; everything before it is pruned.
    async fn floor(&self) -> anyhow::Result<u64>;

    /// The newest heights from `from` on, without their payloads: each height that is new or has
    /// new parts, then the bounds, now and whenever they change. The stream ends only if the
    /// store can no longer be followed, and may have missed changes then.
    async fn tip_updates(
        &self,
        from: u64,
    ) -> anyhow::Result<BoxStream<'static, anyhow::Result<TipUpdate>>>;

    /// The item of `height` with the parts it has, read together; the payload only if `payload`.
    async fn record(&self, height: u64, payload: bool) -> anyhow::Result<Option<Record>>;

    /// The item of `height`.
    async fn item(&self, height: u64) -> anyhow::Result<Option<Item>>;

    /// The certificate that decided the block at `height` directly, if one did.
    async fn cert2(&self, height: u64) -> anyhow::Result<Option<Certificate2<SeqTypes>>>;

    /// The light client state certificate of the epoch whose root is the block at `height`.
    async fn state_cert(
        &self,
        height: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>>;

    /// The height of the block `lookup` names.
    async fn locate(&self, lookup: &Lookup) -> anyhow::Result<Option<u64>>;

    /// The payload `at` names, or only the bytes in `range` of it, once the store can serve it:
    /// the bytes, or where to fetch them, for more than its inline limit, [`INLINE_LIMIT`] by
    /// default, of an uploaded
    /// payload from a store whose payloads clients can reach. Callers ask only for ranges within
    /// the payload.
    async fn payload(
        &self,
        at: &PayloadAt,
        range: Option<Range<u64>>,
    ) -> anyhow::Result<Option<PayloadData>>;

    /// Whether the store can serve the payload whose VID commitment is `commitment`, and whether
    /// it holds it durably at all.
    async fn payload_status(&self, commitment: &str) -> anyhow::Result<PayloadStatus>;

    /// The height and position in its block of the transaction with hash `hash`, as tagged
    /// base64; the earliest if several blocks hold it.
    async fn transaction_position(&self, hash: &str) -> anyhow::Result<Option<(u64, u64)>>;

    /// The transactions and payload bytes of the stored heights in `heights`, of `namespace` only
    /// if given.
    async fn totals(
        &self,
        heights: RangeInclusive<u64>,
        namespace: Option<u32>,
    ) -> anyhow::Result<Totals>;

    /// Up to `limit` ranges of heights without an item, from `from` or the floor on, up to the
    /// newest height with one.
    async fn missing_items(&self, from: u64, limit: usize) -> anyhow::Result<Vec<Range<u64>>>;

    /// The newest range of heights without an item, from the floor on and before the newest
    /// height with one. Every height from its end up to the newest has an item.
    async fn newest_gap(&self) -> anyhow::Result<Option<Range<u64>>>;

    /// Up to `limit` heights from `from` on that have an item but not `part`. A payload counts as
    /// had once the store can serve it.
    async fn missing_parts(&self, part: Part, from: u64, limit: usize) -> anyhow::Result<Vec<u64>>;

    /// Stores `item` with `parts`, a payload among them, which the store describes and indexes
    /// itself. What is stored already is kept, so a write fills only what is missing, and nothing
    /// is written before the floor.
    async fn put_block(&self, item: &Item, parts: &Parts) -> anyhow::Result<()>;

    /// Stores `parts` of `height`, each unless the height has it or is before the floor. A
    /// payload needs the height's item to be stored first.
    async fn put_parts(&self, height: u64, parts: &Parts) -> anyhow::Result<()>;

    /// Takes or renews the lease `name` for `holder` for `ttl`, and says whether `holder` holds it
    /// now: it does unless another holder took it and its time has not run out. Leases tell the
    /// processes sharing the store which of them does a piece of work.
    async fn lease(&self, name: &str, holder: &str, ttl: Duration) -> anyhow::Result<bool>;

    /// What the state after `height` holds besides its header's roots, kept for the newest
    /// heights only.
    async fn state(&self, height: u64) -> anyhow::Result<Option<StateRow>>;

    /// Every kept [`StateRow`], with its height, in height order.
    async fn states(&self) -> anyhow::Result<Vec<(u64, StateRow)>>;

    /// The reward balances kept under `at`, a height a [`StateRow`] names.
    async fn rewards(&self, at: u64) -> anyhow::Result<Option<RewardSet>>;

    /// The state consensus sent for `height`, until a [`StateRow`] is made of it.
    async fn sent_state(&self, height: u64) -> anyhow::Result<Option<SentState>>;

    /// Stores the state after `height` besides its header's roots: `frontier`, `chain_config`,
    /// and `rewards`, every reward balance, or `None` if they are those of the state after
    /// `height - 1`; in one commit with forgetting what the new state makes obsolete. Whether it
    /// was stored: not if the store has no balances to take for `None`.
    async fn put_state(
        &self,
        height: u64,
        frontier: Option<BlocksFrontier>,
        chain_config: ChainConfig,
        rewards: Option<RewardSet>,
    ) -> anyhow::Result<bool>;

    /// The chain config whose commitment is `commitment`.
    async fn chain_config(&self, commitment: &str) -> anyhow::Result<Option<ChainConfig>>;

    async fn put_chain_config(&self, config: &ChainConfig) -> anyhow::Result<()>;

    /// The accounts of the fee tree, which does not change under the new protocol.
    async fn fee_accounts(&self) -> anyhow::Result<Option<Vec<(FeeAccount, FeeAmount)>>>;

    async fn put_fee_accounts(&self, accounts: &[(FeeAccount, FeeAmount)]) -> anyhow::Result<()>;

    /// The stake table of `epoch`.
    async fn stake_table(
        &self,
        epoch: EpochNumber,
    ) -> anyhow::Result<Option<Vec<PeerConfig<SeqTypes>>>>;

    /// The discovery records of `epoch`, or of the latest epoch with records if `epoch` is
    /// `None`, with the epoch they are for.
    async fn service_records(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Option<(EpochNumber, Vec<SignedServiceRecord<SeqTypes>>)>>;

    /// What the store keeps of the membership of `epoch`.
    async fn epoch(&self, epoch: EpochNumber) -> anyhow::Result<EpochMembership>;

    /// The latest `limit` epochs the store keeps membership of, latest first.
    async fn epochs(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, EpochMembership)>>;

    /// The stake tables of the latest `limit` epochs the store keeps one of, latest first.
    async fn latest_stakes(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, StakeTuple)>>;

    /// Stores what `update` has of the membership of `epoch`; see
    /// [`MembershipEntities::put_epoch`].
    async fn put_epoch(&self, epoch: EpochNumber, update: EpochMembership) -> anyhow::Result<()>;

    /// The L1 stake-table events of the blocks from `from` to `to`, both included.
    async fn events(&self, from: u64, to: u64) -> anyhow::Result<StakeEvents>;

    /// Stores L1 stake-table events; see [`MembershipEntities::put_events`].
    async fn put_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()>;

    /// Forgets the events and every epoch's validators, DRB result and root.
    async fn clear_stake_tables(&self) -> anyhow::Result<()>;

    /// The progress of the DRB computation of `epoch`.
    async fn drb_input(&self, epoch: u64) -> anyhow::Result<Option<DrbInput>>;

    /// Stores the progress of a DRB computation; an error if older than the progress stored.
    async fn put_drb_input(&self, input: DrbInput) -> anyhow::Result<()>;
}

/// The largest payload, or range of one, a store answers with its bytes when it could answer
/// with where to fetch them, unless configured otherwise; see [`LocalStore::with_inline_limit`].
pub const INLINE_LIMIT: u64 = 64 << 10;

/// A payload, or a range of one, as a store answers it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PayloadData {
    /// The bytes.
    Inline(Bytes),

    /// Where the bytes are, for the caller to fetch or to send its own clients to: the whole
    /// payload, of which a range request takes the range asked for.
    Url(String),
}

/// The membership a [`Store`] keeps, for [`membership::Persistence`].
#[derive(Clone, Debug)]
pub struct StoreMembership(pub Arc<dyn Store>);

#[async_trait]
impl MembershipEntities for StoreMembership {
    async fn epoch(&self, epoch: EpochNumber) -> anyhow::Result<EpochMembership> {
        self.0.epoch(epoch).await
    }

    async fn epochs(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, EpochMembership)>> {
        self.0.epochs(limit).await
    }

    async fn latest_stakes(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, StakeTuple)>> {
        self.0.latest_stakes(limit).await
    }

    async fn put_epoch(&self, epoch: EpochNumber, update: EpochMembership) -> anyhow::Result<()> {
        self.0.put_epoch(epoch, update).await
    }

    async fn events(&self, from: u64, to: u64) -> anyhow::Result<StakeEvents> {
        self.0.events(from, to).await
    }

    async fn put_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        self.0.put_events(l1_finalized, events).await
    }

    async fn clear_stake_tables(&self) -> anyhow::Result<()> {
        self.0.clear_stake_tables().await
    }

    async fn drb_input(&self, epoch: u64) -> anyhow::Result<Option<DrbInput>> {
        self.0.drb_input(epoch).await
    }

    async fn put_drb_input(&self, input: DrbInput) -> anyhow::Result<()> {
        self.0.put_drb_input(input).await
    }
}

/// A value that changes whenever the newest heights of `store` change, by its tip updates, and
/// also when the updates stop and are asked for again, since changes may have been missed then.
/// Follows the store until the receivers are dropped.
pub fn changes(store: Arc<dyn Store>) -> watch::Receiver<u64> {
    let (sender, changes) = watch::channel(0);
    tokio::spawn(async move {
        while !sender.is_closed() {
            match store.tip_updates(0).await {
                Ok(mut updates) => {
                    while let Some(Ok(update)) = updates.next().await {
                        if matches!(update, TipUpdate::Bounds { .. }) {
                            sender.send_modify(|n| *n += 1);
                        }
                        if sender.is_closed() {
                            return;
                        }
                    }
                },
                Err(err) => tracing::warn!("following the store's changes failed: {err:#}"),
            }
            sender.send_modify(|n| *n += 1);
            sleep(Duration::from_secs(1)).await;
        }
    });
    changes
}

/// What names a payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PayloadAt {
    /// The payload of the block at a height.
    Height(u64),

    /// The payload with a VID commitment, as tagged base64, decided or not.
    Commitment(String),
}

/// Whether a store has a payload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PayloadStatus {
    /// The store can serve it now.
    pub has: bool,

    /// The store holds it durably, uploaded or still staged, and will serve it once uploaded.
    pub held: bool,
}
