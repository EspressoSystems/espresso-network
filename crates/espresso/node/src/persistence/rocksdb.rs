//! RocksDB-backed consensus storage for nodes that do not run the query service.
//!
//! Every write is synced to the write-ahead log before it returns, so a vote or proposal that
//! consensus has acted on survives a crash. RocksDB groups concurrent synced writes into a single
//! WAL fsync, which lets the per-view writes consensus issues in parallel share one flush instead
//! of queueing behind a database-wide write lock.

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Weak},
    time::Instant,
};

use anyhow::{Context as _, bail};
use async_trait::async_trait;
use clap::Parser;
use espresso_types::{
    AuthenticatedValidatorMap, Header, Leaf2, NetworkConfig, PubKey, RegisteredValidatorMap,
    SeqTypes, StakeTableHash,
    traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
    v0::traits::{EventConsumer, PersistenceOptions, SequencerPersistence},
    v0_3::{EventKey, IndexedStake, RegisteredValidator, RewardAmount, StakeTableEvent},
};
use hotshot::InitializerEpochInfo;
use hotshot_libp2p_networking::network::behaviours::dht::store::persistent::{
    DhtPersistentStorage, SerializableRecord,
};
use hotshot_new_protocol::message::Certificate2;
use hotshot_types::{
    data::{
        DaProposal, DaProposal2, EpochNumber, QuorumProposalWrapper, VidCommitment,
        VidDisperseShare,
    },
    drb::{DrbInput, DrbResult},
    event::{HotShotAction, LeafInfo},
    message::{Proposal, convert_proposal},
    simple_certificate::{
        CertificatePair, LightClientStateUpdateCertificateV2, NextEpochQuorumCertificate2,
        QuorumCertificate2, UpgradeCertificate,
    },
    traits::metrics::Metrics,
    vote::HasViewNumber,
};
use rocksdb::{
    BlockBasedOptions, ColumnFamily, ColumnFamilyDescriptor, DB, DBCompressionType, Direction,
    IteratorMode, MergeOperands, WriteBatch, WriteOptions,
};
use serde::{Serialize, de::DeserializeOwned};
use tokio::sync::Mutex;

use crate::{
    RECENT_STAKE_TABLES_LIMIT, ViewNumber,
    persistence::{
        migrate_network_config,
        persistence_metrics::PersistenceMetricsValue,
        storage_probe::{self, StorageProbe},
    },
};

/// Options for RocksDB-backed persistence.
///
/// This backend stores consensus data only. It cannot back the query service, so a node that
/// serves the `query` module must use `storage-sql` or `storage-fs` instead.
///
/// Decided leaves are never replayed as decide events: the event consumer is not called, and
/// everything at or below a decided view is dropped as soon as it is decided, except the newest
/// decided leaf, which is the restart anchor.
#[derive(Parser, Clone, Debug)]
pub struct Options {
    /// Directory holding the RocksDB database.
    #[clap(long = "path", env = "ESPRESSO_NODE_ROCKSDB_PATH")]
    pub(crate) path: PathBuf,
}

impl Options {
    pub fn new(path: PathBuf) -> Options {
        Options { path }
    }
}

#[async_trait]
impl PersistenceOptions for Options {
    type Persistence = Persistence;

    // Nothing outlives its decide, so there is no retention window to configure.
    fn set_view_retention(&mut self, _: u64) {}

    async fn create(&mut self) -> anyhow::Result<Persistence> {
        std::fs::create_dir_all(&self.path).with_context(|| {
            format!(
                "failed to create storage directory '{}'",
                self.path.display()
            )
        })?;
        let probe = storage_probe::probe(&self.path, None).await?;
        let path = self.path.clone();
        let db = tokio::task::spawn_blocking(move || open_shared(&path))
            .await
            .context("failed to join the RocksDB open task")??;

        Ok(Persistence {
            db: Db(db),
            stake_events_lock: Arc::new(Mutex::new(())),
            drb_input_lock: Arc::new(Mutex::new(())),
            metrics: Arc::new(PersistenceMetricsValue::default()),
            probe,
        })
    }

    async fn reset(self) -> anyhow::Result<()> {
        let path = self.path;
        tokio::task::spawn_blocking(move || {
            DB::destroy(&rocksdb::Options::default(), &path)
                .with_context(|| format!("failed to destroy RocksDB at '{}'", path.display()))
        })
        .await
        .context("failed to join the RocksDB destroy task")?
    }
}

/// RocksDB-backed persistence.
#[derive(Clone, Debug)]
pub struct Persistence {
    db: Db,
    stake_events_lock: Arc<Mutex<()>>,
    drb_input_lock: Arc<Mutex<()>>,
    metrics: Arc<PersistenceMetricsValue>,
    probe: StorageProbe,
}

#[derive(Clone)]
struct Db(Arc<DB>);

impl fmt::Debug for Db {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Db").field(&self.0.path()).finish()
    }
}

impl Db {
    /// Run `f` on the blocking pool: RocksDB calls block on disk I/O and synced writes block on
    /// an fsync.
    async fn run<T, F>(&self, f: F) -> anyhow::Result<T>
    where
        F: FnOnce(&DB) -> anyhow::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let db = Arc::clone(&self.0);
        tokio::task::spawn_blocking(move || f(&db))
            .await
            .context("failed to join a RocksDB task")?
    }
}

#[derive(Clone, Copy, Debug)]
enum Cf {
    Meta,
    Leaves,
    VidShares,
    DaProposals,
    QuorumProposals,
    Cert2,
    StateCerts,
    FinalizedStateCerts,
    DrbInputs,
    DrbResults,
    EpochRoots,
    StakeTables,
    StakeEvents,
    Validators,
}

impl Cf {
    const ALL: [Cf; 14] = [
        Cf::Meta,
        Cf::Leaves,
        Cf::VidShares,
        Cf::DaProposals,
        Cf::QuorumProposals,
        Cf::Cert2,
        Cf::StateCerts,
        Cf::FinalizedStateCerts,
        Cf::DrbInputs,
        Cf::DrbResults,
        Cf::EpochRoots,
        Cf::StakeTables,
        Cf::StakeEvents,
        Cf::Validators,
    ];

    /// Column families keyed by view, dropped once their view is decided.
    const PER_VIEW: [Cf; 6] = [
        Cf::Leaves,
        Cf::VidShares,
        Cf::DaProposals,
        Cf::QuorumProposals,
        Cf::Cert2,
        Cf::StateCerts,
    ];

    fn name(self) -> &'static str {
        match self {
            Cf::Meta => "meta",
            Cf::Leaves => "decided_leaves",
            Cf::VidShares => "vid_shares",
            Cf::DaProposals => "da_proposals",
            Cf::QuorumProposals => "quorum_proposals",
            Cf::Cert2 => "decided_cert2",
            Cf::StateCerts => "state_certs",
            Cf::FinalizedStateCerts => "finalized_state_certs",
            Cf::DrbInputs => "drb_inputs",
            Cf::DrbResults => "drb_results",
            Cf::EpochRoots => "epoch_roots",
            Cf::StakeTables => "stake_tables",
            Cf::StakeEvents => "stake_table_events",
            Cf::Validators => "all_validators",
        }
    }

    fn options(self) -> rocksdb::Options {
        let mut opts = rocksdb::Options::default();
        opts.set_compression_type(DBCompressionType::Lz4);
        match self {
            Cf::Meta => opts.set_merge_operator_associative("keep_highest_view", keep_highest_view),
            // Payloads reach the maximum block size. Keeping them in blob files stops every
            // compaction from rewriting them before they are deleted at decide.
            Cf::VidShares | Cf::DaProposals | Cf::QuorumProposals => {
                opts.set_enable_blob_files(true);
                opts.set_min_blob_size(4096);
                opts.set_enable_blob_gc(true);
            },
            Cf::Leaves
            | Cf::Cert2
            | Cf::StateCerts
            | Cf::FinalizedStateCerts
            | Cf::DrbInputs
            | Cf::DrbResults
            | Cf::EpochRoots
            | Cf::StakeTables
            | Cf::StakeEvents
            | Cf::Validators => {},
        }
        opts
    }
}

/// Keys of the singleton records in [`Cf::Meta`].
mod meta {
    pub(super) const CONFIG: &[u8] = b"network_config";
    pub(super) const ACTED_VIEW: &[u8] = b"latest_acted_view";
    pub(super) const RESTART_VIEW: &[u8] = b"restart_view";
    pub(super) const HIGH_QC2: &[u8] = b"high_qc2";
    pub(super) const NEXT_EPOCH_QC: &[u8] = b"next_epoch_qc";
    pub(super) const EQC: &[u8] = b"eqc";
    pub(super) const UPGRADE_CERT: &[u8] = b"upgrade_certificate";
    pub(super) const DHT: &[u8] = b"libp2p_dht";
    pub(super) const STAKE_EVENTS_L1_BLOCK: &[u8] = b"stake_table_events_l1_block";
}

/// Exclusive upper bound for every key this backend writes, the longest being an epoch followed
/// by a validator address.
const KEY_UPPER_BOUND: [u8; 32] = [u8::MAX; 32];

/// Open the database at `path`, or reuse this process's open handle to it.
///
/// RocksDB holds an exclusive lock on its directory, so a second `DB::open` of the same path in
/// one process fails. The other backends allow several handles on one store, and callers rely on
/// that, so handles are shared instead.
fn open_shared(path: &Path) -> anyhow::Result<Arc<DB>> {
    static OPEN: LazyLock<std::sync::Mutex<HashMap<PathBuf, Weak<DB>>>> =
        LazyLock::new(Default::default);

    let path = path
        .canonicalize()
        .with_context(|| format!("failed to resolve '{}'", path.display()))?;
    let mut open_dbs = OPEN
        .lock()
        .expect("the open-database registry is never held across a panic");
    if let Some(db) = open_dbs.get(&path).and_then(Weak::upgrade) {
        return Ok(db);
    }
    let db = Arc::new(open(&path)?);
    open_dbs.retain(|_, db| db.strong_count() > 0);
    open_dbs.insert(path, Arc::downgrade(&db));
    Ok(db)
}

fn open(path: &Path) -> anyhow::Result<DB> {
    let mut opts = rocksdb::Options::default();
    opts.create_if_missing(true);
    opts.create_missing_column_families(true);
    let parallelism = std::thread::available_parallelism().map_or(2, |n| n.get());
    opts.increase_parallelism(i32::try_from(parallelism).unwrap_or(i32::MAX));
    let mut table = BlockBasedOptions::default();
    table.set_bloom_filter(10.0, false);
    opts.set_block_based_table_factory(&table);

    let cfs = Cf::ALL
        .map(|cf| ColumnFamilyDescriptor::new(cf.name(), cf.options()))
        .into_iter();
    DB::open_cf_descriptors(&opts, path, cfs)
        .with_context(|| format!("failed to open RocksDB at '{}'", path.display()))
}

/// Merge operator for monotonic records: a value is an 8-byte big-endian view followed by an
/// arbitrary payload, and the merge keeps the value with the highest view. On a tie the earlier
/// value wins, so re-writing an equal view is a no-op, as the trait requires of `append_high_qc2`.
fn keep_highest_view(
    _key: &[u8],
    existing: Option<&[u8]>,
    operands: &MergeOperands,
) -> Option<Vec<u8>> {
    existing
        .into_iter()
        .chain(operands.iter())
        .reduce(|best, candidate| {
            if view_tag(candidate) > view_tag(best) {
                candidate
            } else {
                best
            }
        })
        .map(<[u8]>::to_vec)
}

fn view_tag(value: &[u8]) -> u64 {
    value
        .first_chunk::<8>()
        .map_or(0, |bytes| u64::from_be_bytes(*bytes))
}

fn tagged(view: ViewNumber, payload: &[u8]) -> Vec<u8> {
    let mut value = Vec::with_capacity(8 + payload.len());
    value.extend_from_slice(&view.u64().to_be_bytes());
    value.extend_from_slice(payload);
    value
}

fn u64_key(n: u64) -> [u8; 8] {
    n.to_be_bytes()
}

fn decode_u64(bytes: &[u8]) -> anyhow::Result<u64> {
    let bytes = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("expected an 8-byte key, got {} bytes", bytes.len()))?;
    Ok(u64::from_be_bytes(bytes))
}

fn stake_event_key((l1_block, log_index): EventKey) -> [u8; 16] {
    let mut key = [0; 16];
    key[..8].copy_from_slice(&l1_block.to_be_bytes());
    key[8..].copy_from_slice(&log_index.to_be_bytes());
    key
}

fn decode_stake_event_key(key: &[u8]) -> anyhow::Result<EventKey> {
    let (l1_block, log_index) = key
        .split_at_checked(8)
        .context("stake table event key is too short")?;
    Ok((decode_u64(l1_block)?, decode_u64(log_index)?))
}

fn cf(db: &DB, cf: Cf) -> &ColumnFamily {
    db.cf_handle(cf.name())
        .expect("every column family is created when the database is opened")
}

fn synced() -> WriteOptions {
    let mut opts = WriteOptions::default();
    opts.set_sync(true);
    opts
}

fn write(db: &DB, batch: WriteBatch) -> anyhow::Result<()> {
    db.write_opt(batch, &synced())
        .context("failed to write to RocksDB")
}

fn put_raw(db: &DB, family: Cf, key: impl AsRef<[u8]>, value: &[u8]) -> anyhow::Result<()> {
    let mut batch = WriteBatch::default();
    batch.put_cf(cf(db, family), key, value);
    write(db, batch)
}

fn put<T>(db: &DB, family: Cf, key: impl AsRef<[u8]>, value: &T) -> anyhow::Result<()>
where
    T: Serialize + ?Sized,
{
    let bytes = bincode::serialize(value)
        .with_context(|| format!("failed to serialize a {} value", family.name()))?;
    put_raw(db, family, key, &bytes)
}

fn get<T>(db: &DB, family: Cf, key: impl AsRef<[u8]>) -> anyhow::Result<Option<T>>
where
    T: DeserializeOwned,
{
    let Some(bytes) = db
        .get_pinned_cf(cf(db, family), key)
        .with_context(|| format!("failed to read from {}", family.name()))?
    else {
        return Ok(None);
    };
    let value = bincode::deserialize(&bytes)
        .with_context(|| format!("failed to deserialize a {} value", family.name()))?;
    Ok(Some(value))
}

fn get_tagged<T>(db: &DB, key: &[u8]) -> anyhow::Result<Option<T>>
where
    T: DeserializeOwned,
{
    let Some(bytes) = db
        .get_pinned_cf(cf(db, Cf::Meta), key)
        .context("failed to read from meta")?
    else {
        return Ok(None);
    };
    let payload = bytes
        .get(8..)
        .context("tagged value is shorter than its view")?;
    Ok(Some(
        bincode::deserialize(payload).context("failed to deserialize a tagged value")?,
    ))
}

fn get_view(db: &DB, key: &[u8]) -> anyhow::Result<Option<ViewNumber>> {
    let bytes = db
        .get_pinned_cf(cf(db, Cf::Meta), key)
        .context("failed to read from meta")?;
    Ok(bytes.map(|bytes| ViewNumber::new(view_tag(&bytes))))
}

/// Decode every entry of `family` with a key in `from..=to`, in key order.
fn range<T>(db: &DB, family: Cf, from: u64, to: u64) -> anyhow::Result<Vec<(u64, T)>>
where
    T: DeserializeOwned,
{
    let from = u64_key(from);
    let mut entries = Vec::new();
    for item in db.iterator_cf(
        cf(db, family),
        IteratorMode::From(&from, Direction::Forward),
    ) {
        let (key, value) = item.with_context(|| format!("failed to iterate {}", family.name()))?;
        let key = decode_u64(&key)?;
        if key > to {
            break;
        }
        let value = bincode::deserialize(&value)
            .with_context(|| format!("failed to deserialize {} entry {key}", family.name()))?;
        entries.push((key, value));
    }
    Ok(entries)
}

fn last<T>(db: &DB, family: Cf) -> anyhow::Result<Option<(u64, T)>>
where
    T: DeserializeOwned,
{
    let Some(item) = db.iterator_cf(cf(db, family), IteratorMode::End).next() else {
        return Ok(None);
    };
    let (key, value) = item.with_context(|| format!("failed to iterate {}", family.name()))?;
    let value = bincode::deserialize(&value)
        .with_context(|| format!("failed to deserialize a {} value", family.name()))?;
    Ok(Some((decode_u64(&key)?, value)))
}

/// Promote the state certificates of the leaves decided up to `view`, then drop every per-view
/// row at or below it except the leaf at `view`, which is the restart anchor.
///
/// Only views with a decided leaf are promoted: a certificate stored from a proposal that was
/// never decided must not become its epoch's finalized certificate.
fn collect_decided(db: &DB, view: u64) -> anyhow::Result<()> {
    let mut batch = WriteBatch::default();
    let start = u64_key(0);
    for item in db.iterator_cf(
        cf(db, Cf::Leaves),
        IteratorMode::From(&start, Direction::Forward),
    ) {
        let (key, _) = item.context("failed to iterate decided leaves")?;
        if decode_u64(&key)? > view {
            break;
        }
        let Some(cert) =
            get::<LightClientStateUpdateCertificateV2<SeqTypes>>(db, Cf::StateCerts, &key)?
        else {
            continue;
        };
        let bytes = bincode::serialize(&cert).context("failed to serialize a state certificate")?;
        batch.put_cf(
            cf(db, Cf::FinalizedStateCerts),
            u64_key(cert.epoch.u64()),
            bytes,
        );
    }
    for family in Cf::PER_VIEW {
        let end = match family {
            Cf::Leaves => view,
            _ => view.saturating_add(1),
        };
        batch.delete_range_cf(cf(db, family), u64_key(0), u64_key(end));
    }
    write(db, batch)
}

#[async_trait]
impl SequencerPersistence for Persistence {
    async fn load_config(&self) -> anyhow::Result<Option<NetworkConfig>> {
        let Some(bytes) = self
            .db
            .run(|db| {
                db.get_cf(cf(db, Cf::Meta), meta::CONFIG)
                    .context("failed to read the network config")
            })
            .await?
        else {
            tracing::info!("config not found");
            return Ok(None);
        };
        let json = serde_json::from_slice(&bytes).context("config is not valid JSON")?;
        let json = migrate_network_config(json).context("migration of network config failed")?;
        Ok(Some(
            serde_json::from_value(json).context("malformed config")?,
        ))
    }

    async fn save_config(&self, cfg: &NetworkConfig) -> anyhow::Result<()> {
        let bytes = serde_json::to_vec(cfg).context("failed to serialize the network config")?;
        self.db
            .run(move |db| put_raw(db, Cf::Meta, meta::CONFIG, &bytes))
            .await
    }

    async fn load_latest_acted_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        self.db.run(|db| get_view(db, meta::ACTED_VIEW)).await
    }

    async fn load_restart_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        self.db.run(|db| get_view(db, meta::RESTART_VIEW)).await
    }

    async fn persist_decided_leaves(
        &self,
        _view: ViewNumber,
        leaf_chain: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, CertificatePair<SeqTypes>)> + Send,
        _deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        _consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()> {
        let rows = leaf_chain
            .into_iter()
            .map(|(info, cert)| {
                // The payload is already stored with the DA proposal.
                let mut leaf = info.leaf.clone();
                leaf.unfill_block_payload();
                let view = cert.view_number().u64();
                let bytes =
                    bincode::serialize(&(leaf, cert)).context("failed to serialize a leaf")?;
                Ok((view, bytes))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        self.db
            .run(move |db| {
                let mut batch = WriteBatch::default();
                for (view, bytes) in rows {
                    batch.put_cf(cf(db, Cf::Leaves), u64_key(view), bytes);
                }
                write(db, batch)
            })
            .await
    }

    async fn process_decided_events(
        &self,
        view: ViewNumber,
        _deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        _consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<Option<ViewNumber>> {
        let now = Instant::now();
        let decided = view.u64();
        self.db.run(move |db| collect_decided(db, decided)).await?;
        self.metrics
            .internal_process_decided_events_duration
            .add_point(now.elapsed().as_secs_f64());
        Ok(Some(view))
    }

    async fn load_anchor_leaf(&self) -> anyhow::Result<Option<(Leaf2, CertificatePair<SeqTypes>)>> {
        self.db
            .run(|db| Ok(last(db, Cf::Leaves)?.map(|(_, anchor)| anchor)))
            .await
    }

    async fn load_quorum_proposals(
        &self,
    ) -> anyhow::Result<BTreeMap<ViewNumber, Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>>>
    {
        self.db
            .run(|db| {
                let proposals = range(db, Cf::QuorumProposals, 0, u64::MAX)?;
                Ok(proposals
                    .into_iter()
                    .map(|(view, proposal)| (ViewNumber::new(view), proposal))
                    .collect())
            })
            .await
    }

    async fn load_quorum_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>> {
        self.db
            .run(move |db| get(db, Cf::QuorumProposals, u64_key(view.u64())))
            .await?
            .with_context(|| format!("no quorum proposal stored for view {view}"))
    }

    async fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, VidDisperseShare<SeqTypes>>>> {
        self.db
            .run(move |db| get(db, Cf::VidShares, u64_key(view.u64())))
            .await
    }

    async fn load_da_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal2<SeqTypes>>>> {
        self.db
            .run(move |db| get(db, Cf::DaProposals, u64_key(view.u64())))
            .await
    }

    async fn load_upgrade_certificate(
        &self,
    ) -> anyhow::Result<Option<UpgradeCertificate<SeqTypes>>> {
        self.db
            .run(|db| get(db, Cf::Meta, meta::UPGRADE_CERT))
            .await
    }

    async fn load_start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>> {
        self.db
            .run(|db| {
                let mut infos = Vec::new();
                for item in db
                    .iterator_cf(cf(db, Cf::DrbResults), IteratorMode::End)
                    .take(RECENT_STAKE_TABLES_LIMIT as usize)
                {
                    let (key, value) = item.context("failed to iterate DRB results")?;
                    let epoch = decode_u64(&key)?;
                    let drb_result = bincode::deserialize::<DrbResult>(&value)
                        .with_context(|| format!("failed to deserialize DRB result {epoch}"))?;
                    let block_header = get::<Header>(db, Cf::EpochRoots, u64_key(epoch))?;
                    infos.push(InitializerEpochInfo::<SeqTypes> {
                        epoch: EpochNumber::new(epoch),
                        drb_result,
                        block_header,
                    });
                }
                infos.reverse();
                Ok(infos)
            })
            .await
    }

    async fn load_state_cert(
        &self,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        self.db
            .run(|db| Ok(last(db, Cf::FinalizedStateCerts)?.map(|(_, cert)| cert)))
            .await
    }

    async fn get_state_cert_by_epoch(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        self.db
            .run(move |db| get(db, Cf::FinalizedStateCerts, u64_key(epoch)))
            .await
    }

    async fn insert_state_cert(
        &self,
        epoch: u64,
        cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        self.db
            .run(move |db| put(db, Cf::FinalizedStateCerts, u64_key(epoch), &cert))
            .await
    }

    async fn append_vid(
        &self,
        proposal: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let view = proposal.data.view_number().u64();
        let bytes = bincode::serialize(proposal).context("failed to serialize a VID share")?;
        let now = Instant::now();
        let res = self
            .db
            .run(move |db| put_raw(db, Cf::VidShares, u64_key(view), &bytes))
            .await;
        self.metrics
            .internal_append_vid_duration
            .add_point(now.elapsed().as_secs_f64());
        res
    }

    async fn append_da(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal<SeqTypes>>,
        vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        self.append_da2(&convert_proposal(proposal.clone()), vid_commit)
            .await
    }

    async fn record_action(
        &self,
        view: ViewNumber,
        _epoch: Option<EpochNumber>,
        action: HotShotAction,
    ) -> anyhow::Result<()> {
        let restart_view = match action {
            HotShotAction::Vote => Some(view + 1),
            HotShotAction::Propose => None,
            HotShotAction::TimeoutVote
            | HotShotAction::ViewSyncVote
            | HotShotAction::DaPropose
            | HotShotAction::DaVote
            | HotShotAction::DaCert
            | HotShotAction::VidDisperse
            | HotShotAction::UpgradeVote
            | HotShotAction::UpgradePropose => return Ok(()),
        };
        self.db
            .run(move |db| {
                let mut batch = WriteBatch::default();
                batch.merge_cf(cf(db, Cf::Meta), meta::ACTED_VIEW, tagged(view, &[]));
                if let Some(restart_view) = restart_view {
                    batch.merge_cf(
                        cf(db, Cf::Meta),
                        meta::RESTART_VIEW,
                        tagged(restart_view, &[]),
                    );
                }
                write(db, batch)
            })
            .await
    }

    async fn append_quorum_proposal2(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let view = proposal.data.view_number().u64();
        let bytes = bincode::serialize(proposal).context("failed to serialize a proposal")?;
        let now = Instant::now();
        let res = self
            .db
            .run(move |db| put_raw(db, Cf::QuorumProposals, u64_key(view), &bytes))
            .await;
        self.metrics
            .internal_append_quorum2_duration
            .add_point(now.elapsed().as_secs_f64());
        res
    }

    async fn append_cert2(
        &self,
        view: ViewNumber,
        cert2: Certificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        self.db
            .run(move |db| put(db, Cf::Cert2, u64_key(view.u64()), &cert2))
            .await
    }

    async fn load_cert2(&self, view: ViewNumber) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        self.db
            .run(move |db| get(db, Cf::Cert2, u64_key(view.u64())))
            .await
    }

    async fn append_high_qc2(&self, high_qc: QuorumCertificate2<SeqTypes>) -> anyhow::Result<()> {
        let value = tagged(
            high_qc.view_number(),
            &bincode::serialize(&high_qc).context("failed to serialize high_qc2")?,
        );
        self.db
            .run(move |db| {
                let mut batch = WriteBatch::default();
                batch.merge_cf(cf(db, Cf::Meta), meta::HIGH_QC2, value);
                write(db, batch)
            })
            .await
    }

    async fn load_high_qc2(&self) -> anyhow::Result<Option<QuorumCertificate2<SeqTypes>>> {
        self.db.run(|db| get_tagged(db, meta::HIGH_QC2)).await
    }

    async fn store_eqc(
        &self,
        high_qc: QuorumCertificate2<SeqTypes>,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        self.db
            .run(move |db| put(db, Cf::Meta, meta::EQC, &(high_qc, next_epoch_high_qc)))
            .await
    }

    async fn load_eqc(
        &self,
    ) -> Option<(
        QuorumCertificate2<SeqTypes>,
        NextEpochQuorumCertificate2<SeqTypes>,
    )> {
        match self.db.run(|db| get(db, Cf::Meta, meta::EQC)).await {
            Ok(eqc) => eqc,
            Err(err) => {
                tracing::warn!(err = %format_args!("{err:#}"), "failed to load eqc");
                None
            },
        }
    }

    async fn store_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let Some(cert) = decided_upgrade_certificate else {
            return Ok(());
        };
        self.db
            .run(move |db| put(db, Cf::Meta, meta::UPGRADE_CERT, &cert))
            .await
    }

    async fn load_next_epoch_quorum_certificate(
        &self,
    ) -> anyhow::Result<Option<NextEpochQuorumCertificate2<SeqTypes>>> {
        self.db.run(|db| get_tagged(db, meta::NEXT_EPOCH_QC)).await
    }

    async fn append_next_epoch_high_qc2(
        &self,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let value = tagged(
            next_epoch_high_qc.view_number(),
            &bincode::serialize(&next_epoch_high_qc)
                .context("failed to serialize the next epoch high_qc2")?,
        );
        self.db
            .run(move |db| {
                let mut batch = WriteBatch::default();
                batch.merge_cf(cf(db, Cf::Meta), meta::NEXT_EPOCH_QC, value);
                write(db, batch)
            })
            .await
    }

    async fn append_da2(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        _vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        let view = proposal.data.view_number().u64();
        let bytes = bincode::serialize(proposal).context("failed to serialize a DA proposal")?;
        let now = Instant::now();
        let res = self
            .db
            .run(move |db| put_raw(db, Cf::DaProposals, u64_key(view), &bytes))
            .await;
        self.metrics
            .internal_append_da2_duration
            .add_point(now.elapsed().as_secs_f64());
        res
    }

    async fn store_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        self.db
            .run(move |db| put(db, Cf::DrbResults, u64_key(epoch.u64()), &drb_result))
            .await
    }

    async fn store_drb_input(&self, drb_input: DrbInput) -> anyhow::Result<()> {
        let _guard = self.drb_input_lock.lock().await;
        if let Ok(stored) = self.load_drb_input(drb_input.epoch).await {
            if stored.difficulty_level != drb_input.difficulty_level {
                tracing::error!("Overwriting {stored:?} in storage with {drb_input:?}");
            } else if stored.iteration >= drb_input.iteration {
                bail!(
                    "DrbInput in storage {stored:?} is more recent than {drb_input:?}, refusing \
                     to update"
                );
            }
        }
        self.db
            .run(move |db| put(db, Cf::DrbInputs, u64_key(drb_input.epoch), &drb_input))
            .await
    }

    async fn load_drb_input(&self, epoch: u64) -> anyhow::Result<DrbInput> {
        self.db
            .run(move |db| get(db, Cf::DrbInputs, u64_key(epoch)))
            .await?
            .with_context(|| format!("no DrbInput for epoch {epoch} in storage"))
    }

    async fn add_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let view = state_cert.light_client_state.view_number;
        self.db
            .run(move |db| put(db, Cf::StateCerts, u64_key(view), &state_cert))
            .await
    }

    fn enable_metrics(&mut self, metrics: &dyn Metrics) {
        self.metrics = Arc::new(PersistenceMetricsValue::new(metrics));
        self.probe.register(&*metrics.subgroup("disk".into()));
    }
}

#[async_trait]
impl MembershipPersistence for Persistence {
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        self.db
            .run(move |db| get(db, Cf::StakeTables, u64_key(epoch.u64())))
            .await
    }

    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        self.db
            .run(move |db| {
                let mut stakes = Vec::new();
                for item in db
                    .iterator_cf(cf(db, Cf::StakeTables), IteratorMode::End)
                    .take(usize::try_from(limit).unwrap_or(usize::MAX))
                {
                    let (key, value) = item.context("failed to iterate stake tables")?;
                    let epoch = decode_u64(&key)?;
                    let (validators, block_reward, hash): StakeTuple = bincode::deserialize(&value)
                        .with_context(|| format!("failed to deserialize stake table {epoch}"))?;
                    stakes.push((EpochNumber::new(epoch), (validators, block_reward), hash));
                }
                Ok(Some(stakes))
            })
            .await
    }

    async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        self.db
            .run(move |db| get(db, Cf::DrbResults, u64_key(epoch.u64())))
            .await
    }

    async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        self.db
            .run(move |db| get(db, Cf::EpochRoots, u64_key(epoch.u64())))
            .await
    }

    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: Header,
    ) -> anyhow::Result<()> {
        self.db
            .run(move |db| put(db, Cf::EpochRoots, u64_key(epoch.u64()), &block_header))
            .await
    }

    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        let stake: StakeTuple = (stake, block_reward, stake_table_hash);
        self.db
            .run(move |db| put(db, Cf::StakeTables, u64_key(epoch.u64()), &stake))
            .await
    }

    async fn store_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        let events = events
            .into_iter()
            .map(|(key, event)| {
                let json = serde_json::to_vec(&event)
                    .context("failed to serialize a stake table event")?;
                Ok((stake_event_key(key), json))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        let _guard = self.stake_events_lock.lock().await;
        self.db
            .run(move |db| {
                let stored = get::<u64>(db, Cf::Meta, meta::STAKE_EVENTS_L1_BLOCK)?;
                if stored.is_some_and(|stored| stored > l1_finalized) {
                    tracing::debug!(?stored, l1_finalized, "stored l1 block is already higher");
                    return Ok(());
                }
                let mut batch = WriteBatch::default();
                for (key, json) in events {
                    batch.put_cf(cf(db, Cf::StakeEvents), key, json);
                }
                batch.put_cf(
                    cf(db, Cf::Meta),
                    meta::STAKE_EVENTS_L1_BLOCK,
                    bincode::serialize(&l1_finalized)
                        .context("failed to serialize the l1 block")?,
                );
                write(db, batch)
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
        self.db
            .run(move |db| {
                let Some(stored) = get::<u64>(db, Cf::Meta, meta::STAKE_EVENTS_L1_BLOCK)? else {
                    return Ok((None, Vec::new()));
                };
                let query_l1_block = stored.min(to_l1_block);

                let start = stake_event_key((from_l1_block, 0));
                let mut events = Vec::new();
                for item in db.iterator_cf(
                    cf(db, Cf::StakeEvents),
                    IteratorMode::From(&start, Direction::Forward),
                ) {
                    let (key, value) = item.context("failed to iterate stake table events")?;
                    let key = decode_stake_event_key(&key)?;
                    if key.0 > query_l1_block {
                        break;
                    }
                    let event = serde_json::from_slice(&value).with_context(|| {
                        format!("failed to deserialize stake table event {key:?}")
                    })?;
                    events.push((key, event));
                }

                let read = if query_l1_block == to_l1_block {
                    EventsPersistenceRead::Complete
                } else {
                    EventsPersistenceRead::UntilL1Block(query_l1_block)
                };
                Ok((Some(read), events))
            })
            .await
    }

    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        let _guard = self.stake_events_lock.lock().await;
        self.db
            .run(|db| {
                let mut batch = WriteBatch::default();
                for family in [
                    Cf::StakeEvents,
                    Cf::StakeTables,
                    Cf::DrbResults,
                    Cf::EpochRoots,
                    Cf::Validators,
                ] {
                    batch.delete_range_cf(cf(db, family), &[][..], &KEY_UPPER_BOUND[..]);
                }
                batch.delete_cf(cf(db, Cf::Meta), meta::STAKE_EVENTS_L1_BLOCK);
                write(db, batch)
            })
            .await
    }

    async fn store_all_validators(
        &self,
        epoch: EpochNumber,
        all_validators: RegisteredValidatorMap,
    ) -> anyhow::Result<()> {
        let rows = all_validators
            .into_iter()
            .map(|(address, validator)| {
                let mut key = Vec::with_capacity(28);
                key.extend_from_slice(&u64_key(epoch.u64()));
                key.extend_from_slice(address.as_slice());
                let json = serde_json::to_vec(&validator)
                    .with_context(|| format!("failed to serialize validator {address}"))?;
                Ok((key, json))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        self.db
            .run(move |db| {
                let mut batch = WriteBatch::default();
                for (key, json) in rows {
                    batch.put_cf(cf(db, Cf::Validators), key, json);
                }
                write(db, batch)
            })
            .await
    }

    async fn load_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        self.db
            .run(move |db| {
                let prefix = u64_key(epoch.u64());
                let offset = usize::try_from(offset).unwrap_or(usize::MAX);
                let limit = usize::try_from(limit).unwrap_or(usize::MAX);
                let mut validators = Vec::new();
                for (index, item) in db
                    .iterator_cf(
                        cf(db, Cf::Validators),
                        IteratorMode::From(&prefix, Direction::Forward),
                    )
                    .enumerate()
                {
                    let (key, value) = item.context("failed to iterate validators")?;
                    if !key.starts_with(&prefix) || validators.len() == limit {
                        break;
                    }
                    if index < offset {
                        continue;
                    }
                    validators.push(
                        serde_json::from_slice(&value)
                            .context("failed to deserialize a validator")?,
                    );
                }
                Ok(validators)
            })
            .await
    }
}

#[async_trait]
impl DhtPersistentStorage for Persistence {
    async fn save(&self, records: Vec<SerializableRecord>) -> anyhow::Result<()> {
        self.db
            .run(move |db| put(db, Cf::Meta, meta::DHT, &records))
            .await
    }

    async fn load(&self) -> anyhow::Result<Vec<SerializableRecord>> {
        self.db
            .run(|db| get(db, Cf::Meta, meta::DHT))
            .await?
            .context("no DHT records stored")
    }
}

#[cfg(test)]
mod test {
    use hotshot_types::new_protocol::CoordinatorEvent;
    use tempfile::TempDir;

    use super::*;
    use crate::{api::data_source::StorageOptions as _, persistence::tests::TestablePersistence};

    #[async_trait]
    impl TestablePersistence for Persistence {
        type Storage = TempDir;

        async fn tmp_storage() -> TempDir {
            TempDir::new().unwrap()
        }

        fn options(storage: &TempDir) -> impl PersistenceOptions<Persistence = Persistence> {
            Options::new(storage.path().into())
        }
    }

    #[derive(Debug)]
    struct RefusingConsumer;

    #[async_trait]
    impl EventConsumer for RefusingConsumer {
        async fn handle_event(&self, _: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()> {
            bail!("a non-query node must not replay decide events")
        }
    }

    fn keys(db: &DB, family: Cf) -> Vec<u64> {
        db.iterator_cf(cf(db, family), IteratorMode::Start)
            .map(|item| decode_u64(&item.unwrap().0).unwrap())
            .collect()
    }

    #[tokio::test]
    async fn decide_drops_decided_views_without_calling_the_consumer() {
        let dir = TempDir::new().unwrap();
        let persistence = Options::new(dir.path().into()).create().await.unwrap();
        let db = &persistence.db.0;
        for view in 1..=4 {
            for family in [
                Cf::VidShares,
                Cf::DaProposals,
                Cf::QuorumProposals,
                Cf::Cert2,
            ] {
                put_raw(db, family, u64_key(view), b"row").unwrap();
            }
            let mut cert = LightClientStateUpdateCertificateV2::<SeqTypes>::genesis();
            cert.epoch = EpochNumber::new(view);
            cert.light_client_state.view_number = view;
            put(db, Cf::StateCerts, u64_key(view), &cert).unwrap();
        }
        // Views 1 and 3 were decided, view 2 was a fork and view 4 is still undecided.
        for view in [1, 3] {
            put_raw(db, Cf::Leaves, u64_key(view), b"leaf").unwrap();
        }

        let processed = persistence
            .process_decided_events(ViewNumber::new(3), None, &RefusingConsumer)
            .await
            .unwrap();

        assert_eq!(processed, Some(ViewNumber::new(3)));
        assert_eq!(keys(db, Cf::Leaves), Vec::from([3]));
        for family in [
            Cf::VidShares,
            Cf::DaProposals,
            Cf::QuorumProposals,
            Cf::Cert2,
            Cf::StateCerts,
        ] {
            assert_eq!(keys(db, family), Vec::from([4]), "{}", family.name());
        }
        assert_eq!(keys(db, Cf::FinalizedStateCerts), Vec::from([1, 3]));
    }

    #[tokio::test]
    async fn monotonic_records_survive_closing_every_handle() {
        let dir = TempDir::new().unwrap();
        let mut options = Options::new(dir.path().into());
        let persistence = options.create().await.unwrap();
        persistence
            .record_action(ViewNumber::new(9), None, HotShotAction::Vote)
            .await
            .unwrap();
        persistence
            .record_action(ViewNumber::new(4), None, HotShotAction::Vote)
            .await
            .unwrap();
        persistence
            .record_action(ViewNumber::new(12), None, HotShotAction::Propose)
            .await
            .unwrap();
        drop(persistence);

        let persistence = options.create().await.unwrap();
        assert_eq!(
            persistence.load_latest_acted_view().await.unwrap(),
            Some(ViewNumber::new(12))
        );
        assert_eq!(
            persistence.load_restart_view().await.unwrap(),
            Some(ViewNumber::new(10))
        );
    }

    #[tokio::test]
    async fn storage_rocksdb_refuses_the_query_module() {
        let options = Options::new(PathBuf::from("/unused"));
        let err = options
            .enable_query_module(
                crate::api::Options::with_port(0),
                crate::api::options::Query::default(),
            )
            .unwrap_err();
        assert!(format!("{err:#}").contains("cannot serve the query module"));
    }

    #[test]
    fn keep_highest_view_ignores_stale_and_equal_writes() {
        let db_dir = TempDir::new().unwrap();
        let db = open(db_dir.path()).unwrap();
        for (view, payload) in [(5, b"alpha"), (3, b"bravo"), (5, b"delta"), (7, b"hotel")] {
            let mut batch = WriteBatch::default();
            batch.merge_cf(
                cf(&db, Cf::Meta),
                meta::HIGH_QC2,
                tagged(ViewNumber::new(view), payload),
            );
            write(&db, batch).unwrap();
            if view == 5 {
                db.flush_cf(cf(&db, Cf::Meta)).unwrap();
            }
        }
        let stored = db
            .get_cf(cf(&db, Cf::Meta), meta::HIGH_QC2)
            .unwrap()
            .unwrap();
        assert_eq!(stored, tagged(ViewNumber::new(7), b"hotel"));

        let mut batch = WriteBatch::default();
        batch.merge_cf(
            cf(&db, Cf::Meta),
            meta::HIGH_QC2,
            tagged(ViewNumber::new(7), b"dupe"),
        );
        write(&db, batch).unwrap();
        let stored = db
            .get_cf(cf(&db, Cf::Meta), meta::HIGH_QC2)
            .unwrap()
            .unwrap();
        assert_eq!(stored, tagged(ViewNumber::new(7), b"hotel"));
    }
}
