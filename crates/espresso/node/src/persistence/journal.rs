//! Append-only journal persistence backend for consensus data.
//!
//! Two independent streams (`wal`, `data`), each a std writer thread doing group commit with a
//! durable/enqueue sync policy; see the design doc for the write path, roll and recovery. Reads of
//! consensus state come from an in-memory `State` folded from wal records; `MembershipPersistence`
//! and `DhtPersistentStorage` delegate to a side `fs::Persistence` tree, which keeps fs semantics
//! (no fsync) since DRB/stake/state-cert data is recoverable from L1 or peers.
//!
//! A query node pairs the journal with a separate query-service database. Consensus writes stay
//! here, and decided blocks are replayed into the query service from a cursor, off the voting path.

pub mod state;

use std::{
    collections::BTreeMap, fs, io, os::unix::fs::FileExt as _, path::PathBuf, sync::Arc,
    time::Instant,
};

use anyhow::{Context, ensure};
use async_trait::async_trait;
use clap::Parser;
use espresso_types::{
    AuthenticatedValidatorMap, Header, Leaf2, NetworkConfig, Payload, PubKey,
    RegisteredValidatorMap, SeqTypes, StakeTableHash,
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
    message::Proposal,
    new_protocol::CoordinatorEvent,
    simple_certificate::{
        CertificatePair, LightClientStateUpdateCertificateV2, NextEpochQuorumCertificate2,
        QuorumCertificate2, UpgradeCertificate,
    },
    traits::{
        block_contents::{BlockHeader as _, BlockPayload as _},
        metrics::Metrics,
    },
};
use journal_lane::{
    format::{self, Class, ScanEnd, SegmentHeader, Stream},
    lane::{
        self, DataIndex, JournalFs, Lane, LaneConfig, LaneMetrics, LaneMode, SegmentMeta, Segments,
        SnapshotHook, StdFs,
    },
};

use crate::{
    ViewNumber,
    persistence::{
        fs as side_fs,
        journal::state::{Kind, Record, Replay, ReplayLeaf, State},
        persistence_metrics::PersistenceMetricsValue,
        sql::{self, DecidedLeaf, decide_events_from_chain, within_gap_fill_horizon},
        storage_probe::{self, StorageProbe},
    },
};

const WAL_SEGMENT_BYTES: u64 = 256 << 20;
const WAL_MAX_VIEW_SPAN: u64 = 8192;
const WAL_MAX_BATCH_BYTES: usize = 4 << 20;
const DATA_SEGMENT_BYTES: u64 = 1 << 30;
const DATA_MAX_BATCH_BYTES: usize = 64 << 20;
/// Added to both record bounds: covers everything in a record that does not grow with block
/// size (QCs, header fields).
const RECORD_HEADROOM_BYTES: u64 = 64 << 20;
/// Lower bound on the data record limit. VID proofs grow with namespaces x shard weight, not
/// payload bytes: ~160 KB per namespace at 1/3 stake. Covers ~6.5k namespaces at 1/3 stake, ~2.2k
/// at full stake; `put_data` rejects larger shares.
const DATA_MIN_RECORD_BYTES: u64 = 1 << 30;
/// Bound on a wal `Snapshot` frame specifically: `State` (particularly `proposals`) is not pruned,
/// so it needs more headroom than a single consensus record.
const MAX_SNAPSHOT_BYTES: u32 = 1 << 30;
/// A record over this budget takes all of it, so peak memory is about one record plus its
/// `frame_buf` copy.
const IN_FLIGHT_BYTES: usize = 128 << 20;

/// Per-record size bounds, enforced in `put_wal`/`put_data` before a record reaches a lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecordLimits {
    wal: u32,
    data: u32,
}

impl RecordLimits {
    /// `max_block_size` is the largest over the genesis base version and upgrades.
    ///
    /// wal: only the header's ns table grows with block size, and it stays under
    /// `max_block_size` (each namespace takes at least 8 payload bytes for an 8-byte entry).
    /// data: a VID share holds `weight / ceil(total_weight / 3)` of the payload, so up to 3x for a
    /// node with all the stake; a DA proposal holds 1x. Never below `DATA_MIN_RECORD_BYTES`.
    /// Both stop at `u32::MAX`, the frame length field's range.
    fn new(max_block_size: u64) -> Self {
        let bound = |factor: u64| {
            max_block_size
                .saturating_mul(factor)
                .saturating_add(RECORD_HEADROOM_BYTES)
        };
        let frame = |bytes: u64| bytes.min(u32::MAX.into()) as u32;
        Self {
            wal: frame(bound(1)),
            data: frame(bound(3).max(DATA_MIN_RECORD_BYTES)),
        }
    }
}

/// Default `--max-bytes`: 100 GiB. Clap can't parse a `"100gb"` default into a plain `u64`
/// without a custom value parser, so the CLI default is the equivalent byte count.
const DEFAULT_MAX_BYTES: u64 = 100 * (1 << 30);
/// Default `--view-retention`: about 1 week at a 2s view time.
const DEFAULT_VIEW_RETENTION: u64 = 302_000;
/// Most views replayed into the query service in one decide event. Each batch reads its blocks'
/// shares into memory and copies their payloads, so this bounds replay memory at a few blocks.
const MAX_REPLAY_VIEWS: usize = 8;
/// Default `--pending-payload-bytes`: 8 GiB.
const DEFAULT_PENDING_PAYLOAD_BYTES: u64 = 8 * (1 << 30);

/// Options for the append-only journal, the consensus storage of every node.
#[derive(Parser, Clone, Debug)]
pub struct Options {
    /// Storage path for persistent data.
    #[clap(long, env = "ESPRESSO_NODE_STORAGE_PATH")]
    pub path: PathBuf,

    /// Views to retain before an undecided (forked or offline) view's data is garbage collected.
    #[clap(
        long,
        env = "ESPRESSO_NODE_JOURNAL_VIEW_RETENTION",
        default_value_t = DEFAULT_VIEW_RETENTION
    )]
    pub view_retention: u64,

    /// Soft cap on total `data` stream bytes on disk; oldest segments are dropped first once
    /// exceeded (age-based retention runs regardless of this cap).
    #[clap(
        long,
        env = "ESPRESSO_NODE_JOURNAL_MAX_BYTES",
        default_value_t = DEFAULT_MAX_BYTES
    )]
    pub max_bytes: u64,

    /// Cap on the block payloads a query node holds in memory for its query service, from when
    /// it obtains them until their decide is replayed. Oldest dropped first once exceeded; the
    /// query service fetches those blocks from a peer.
    #[clap(
        long,
        env = "ESPRESSO_NODE_JOURNAL_PENDING_PAYLOAD_BYTES",
        default_value_t = DEFAULT_PENDING_PAYLOAD_BYTES
    )]
    pub pending_payload_bytes: u64,

    /// Start even if an existing `storage-sql`/`storage-fs` layout is found at `path`. Without
    /// this, `storage-journal` refuses to start over another backend's data to avoid silently
    /// abandoning it.
    #[clap(
        long = "journal-ignore-existing",
        env = "ESPRESSO_NODE_JOURNAL_IGNORE_EXISTING"
    )]
    pub ignore_existing: bool,

    /// Largest `max_block_size` over the genesis base version and upgrades, set from genesis.
    #[clap(skip)]
    pub max_block_size: Option<u64>,

    #[clap(skip)]
    pub(crate) query_storage: Option<QueryStorage>,
}

/// The database a query node's query service reads from, filled by replaying decided leaves.
#[derive(Clone, Debug)]
pub(crate) enum QueryStorage {
    Sql(Box<sql::Options>),
    Fs(side_fs::Options),
}

impl Options {
    /// The options of a node started without the `storage-journal` module: its environment
    /// variables alone.
    pub fn from_env() -> anyhow::Result<Self> {
        Self::try_parse_from(std::iter::empty::<String>()).context(
            "consensus storage needs ESPRESSO_NODE_STORAGE_PATH, or the storage-journal module \
             with --path",
        )
    }

    /// Back the query service with `query_storage`.
    ///
    /// Decided leaves and their VID shares are then kept until they have been replayed as decide
    /// events to the query service, which may lag consensus; their payloads wait in memory.
    /// Consensus never waits on the query service database.
    pub(crate) fn with_query_storage(mut self, query_storage: QueryStorage) -> Options {
        self.query_storage = Some(query_storage);
        self
    }
}

#[async_trait]
impl PersistenceOptions for Options {
    type Persistence = Persistence;

    fn set_view_retention(&mut self, view_retention: u64) {
        self.view_retention = view_retention;
    }

    /// Without query storage the journal keeps no payloads for replay and emits no decide events.
    fn set_consensus_only(&mut self) {
        self.query_storage = None;
    }

    async fn create(&mut self) -> anyhow::Result<Self::Persistence> {
        Persistence::open(self.clone()).await
    }

    async fn reset(self) -> anyhow::Result<()> {
        for dir in [self.path.join("journal"), self.path.join("side")] {
            match fs::remove_dir_all(&dir) {
                Ok(()) => {},
                Err(err) if err.kind() == io::ErrorKind::NotFound => {},
                Err(err) => return Err(err).with_context(|| format!("removing {}", dir.display())),
            }
        }
        Ok(())
    }
}

/// Append-only journal persistence.
#[derive(Clone)]
pub struct Persistence {
    inner: Arc<Inner>,
    /// Per-method timing histograms; replaced wholesale by `enable_metrics`, same as fs/sql. Not
    /// inside `Inner`: it starts as a `NoMetrics`-backed default and only this clone (the one
    /// `enable_metrics` is called on) observes the swap, matching fs/sql's existing behaviour.
    metrics: Arc<PersistenceMetricsValue>,
}

struct Inner {
    state: Arc<parking_lot::Mutex<State>>,
    // Field order matters: dropping the lanes closes the channels, then `threads` joins the
    // writers, then `_lock` is released.
    wal: Lane,
    data: Lane,
    _threads: JoinOnDrop,
    wal_segments: Arc<parking_lot::Mutex<Segments>>,
    data_segments: Arc<parking_lot::Mutex<Segments>>,
    // Serializes `gc` passes: `gc` computes the segments to unlink once up front, so an
    // overlapping pass could otherwise select and unlink the same segment twice.
    gc_lock: parking_lot::Mutex<()>,
    side: side_fs::Persistence,
    opts: Options,
    wal_dir: PathBuf,
    data_dir: PathBuf,
    probe: StorageProbe,
    limits: RecordLimits,
    /// Measured before any real `Metrics` is attached; `enable_metrics` records it once a
    /// histogram exists to record it into.
    replay_seconds: f64,
    /// `Some` exactly on a query node, which replays decided blocks into its query service.
    replay_index: Option<DataIndex>,
    /// Payloads not yet delivered to the query service. Memory only, see [`PendingPayloads`].
    pending_payloads: parking_lot::Mutex<PendingPayloads>,
    _lock: std::fs::File,
}

/// A payload this node obtained for a view, held for the query service.
struct PendingPayload {
    header: Header,
    payload: Arc<Payload>,
}

/// Obtained payloads a query node has not yet replayed into its query service, by view. Held in
/// memory, never written: a decide carries the payload of a block obtained before it, so only a
/// restart loses one, and the query service then fetches that block from a peer. The cap bounds
/// the backlog while the query service lags; the oldest views go first.
struct PendingPayloads {
    by_view: BTreeMap<ViewNumber, PendingPayload>,
    bytes: u64,
    max_bytes: u64,
}

impl PendingPayloads {
    fn new(max_bytes: u64) -> Self {
        Self {
            by_view: BTreeMap::new(),
            bytes: 0,
            max_bytes,
        }
    }

    /// Hold `payload` for `view`, replacing an earlier one. Returns the oldest views dropped to
    /// stay under the cap; a lone payload over the cap is kept.
    fn insert(
        &mut self,
        view: ViewNumber,
        header: Header,
        payload: Arc<Payload>,
    ) -> Vec<ViewNumber> {
        let pending = PendingPayload { header, payload };
        self.bytes += pending.bytes();
        if let Some(replaced) = self.by_view.insert(view, pending) {
            self.bytes -= replaced.bytes();
        }
        let mut dropped = Vec::new();
        while self.bytes > self.max_bytes && self.by_view.len() > 1 {
            let (view, pending) = self.by_view.pop_first().expect("more than one entry");
            self.bytes -= pending.bytes();
            dropped.push(view);
        }
        dropped
    }

    fn holds(&self, view: ViewNumber, header: &Header) -> bool {
        self.payload_for(view, header).is_some()
    }

    /// The payload held for `view`, if it is for the block with `header`.
    fn payload_for(&self, view: ViewNumber, header: &Header) -> Option<Arc<Payload>> {
        self.by_view
            .get(&view)
            .filter(|pending| pending.header == *header)
            .map(|pending| Arc::clone(&pending.payload))
    }

    fn remove(&mut self, view: ViewNumber) {
        if let Some(pending) = self.by_view.remove(&view) {
            self.bytes -= pending.bytes();
        }
    }

    /// Take every payload at or below `view`.
    fn take_up_to(&mut self, view: ViewNumber) -> Vec<(ViewNumber, PendingPayload)> {
        let newer = self.by_view.split_off(&(view + 1));
        let taken = std::mem::replace(&mut self.by_view, newer);
        self.bytes -= taken.values().map(PendingPayload::bytes).sum::<u64>();
        taken.into_iter().collect()
    }
}

impl PendingPayload {
    fn bytes(&self) -> u64 {
        self.payload.byte_len().as_usize() as u64
    }
}

struct JoinOnDrop(Vec<std::thread::JoinHandle<()>>);

impl Drop for JoinOnDrop {
    fn drop(&mut self) {
        for t in self.0.drain(..) {
            t.join().ok();
        }
    }
}

struct StateSnapshotHook(Arc<parking_lot::Mutex<State>>);

impl SnapshotHook for StateSnapshotHook {
    fn snapshot_locked(&self, drain: &mut dyn FnMut()) -> Vec<u8> {
        let state = self.0.lock();
        drain();
        // Same encoding `Record::Snapshot(State).encode()` produces, without cloning `State` just
        // to hand it to a `Record` we immediately discard.
        bincode::serialize(&*state).expect("encoding a Snapshot record never fails")
    }
}

/// True if `dir` contains at least one `*.log` segment. Missing/unreadable `dir` counts as empty:
/// this only exists to tell "never written to" apart from "has a prior journal run".
fn dir_has_log_files(dir: &std::path::Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .any(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("log"))
}

/// Fsyncs `journal_dir`, `path` and `path`'s parent (if any): the directory entries
/// `fs::create_dir_all` adds for them are not durable until fsynced. `path` and its parent already
/// exist by the time this runs (creating `journal_dir` requires it), so all three are safe to open.
fn sync_new_dirs(journal_dir: &std::path::Path, path: &std::path::Path) -> io::Result<()> {
    StdFs.sync_dir(journal_dir)?;
    StdFs.sync_dir(path)?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        StdFs.sync_dir(parent)?;
    }
    Ok(())
}

impl Persistence {
    pub async fn open(opts: Options) -> anyhow::Result<Self> {
        let path = opts.path.clone();
        let journal_dir = path.join("journal");
        let wal_dir = journal_dir.join("wal");
        let data_dir = journal_dir.join("data");
        let side_dir = path.join("side");

        // Must run before any directory is created (including by `storage_probe::probe` below):
        // `wal_dir`/`data_dir` existing is not evidence of a prior journal run, only of this check
        // itself having run before.
        let old_layout_present =
            path.join("sqlite").join("database").is_file() || path.join("decided_leaves2").is_dir();
        let journal_empty = !dir_has_log_files(&wal_dir) && !dir_has_log_files(&data_dir);
        ensure!(
            opts.ignore_existing || !old_layout_present || !journal_empty,
            "found an existing sql/fs storage layout at {}; refusing to start storage-journal \
             without --journal-ignore-existing",
            path.display()
        );

        fs::create_dir_all(&journal_dir).context("creating journal directory")?;
        // Recovery and the writer threads never touch the filesystem except through `JournalFs`
        // (so `MemFs` tests stay hermetic); the one directory-creation exception lives here.
        fs::create_dir_all(&wal_dir).context("creating wal directory")?;
        fs::create_dir_all(&data_dir).context("creating data directory")?;
        // `wal_dir`/`data_dir` get fsynced once their first segment is created (`open_new_segment`);
        // this covers the parent directory entries `create_dir_all` just added, which are not.
        sync_new_dirs(&journal_dir, &path).context("fsyncing newly created journal directories")?;

        let lock_path = journal_dir.join("LOCK");
        let lock_file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .context("opening journal LOCK file")?;
        lock_file
            .try_lock()
            .map_err(std::io::Error::from)
            .context("journal directory is already in use by another process")?;

        let probe = storage_probe::probe(&wal_dir, None)
            .await
            .context("probing wal directory")?;

        let limits = RecordLimits::new(
            opts.max_block_size
                .context("journal options have no max_block_size")?,
        );

        let std_fs = Arc::new(StdFs);
        let (wal_recovered, data_recovered) = {
            let fs = std_fs.clone();
            let dir = wal_dir.clone();
            let wal = tokio::task::spawn_blocking(move || {
                lane::recover(&*fs, &dir, Stream::Wal, LaneMode::Snapshot, Kind::is_known)
            })
            .await
            .context("wal recovery task panicked")?
            .context("recovering wal stream")?;

            let fs = std_fs.clone();
            let dir = data_dir.clone();
            let data = tokio::task::spawn_blocking(move || {
                lane::recover(&*fs, &dir, Stream::Data, LaneMode::Append, Kind::is_known)
            })
            .await
            .context("data recovery task panicked")?
            .context("recovering data stream")?;
            (wal, data)
        };

        let mut side_opts = side_fs::Options::new(side_dir);
        let side = side_opts.create().await.context("opening side fs store")?;

        let replay_started = Instant::now();
        let mut state = State::default();
        for (header, body) in &wal_recovered.wal_records {
            let kind = Kind::from_u8(header.kind.0).context("unknown wal record kind")?;
            let record = Record::decode(kind, header.key, body)
                .context("decoding wal record during replay")?;
            match record {
                Record::Snapshot(snapshot) => state = *snapshot,
                record => {
                    if let Some(finalized) = state.apply(&record) {
                        side.insert_state_cert(finalized.epoch, finalized.cert)
                            .await
                            .context("re-upserting state cert recovered from wal replay")?;
                    }
                },
            }
        }
        let replay_seconds = replay_started.elapsed().as_secs_f64();
        tracing::info!(
            wal_segment = ?wal_recovered.segments.last().map(|s| s.seq),
            records = wal_recovered.wal_records.len(),
            anchor_view = ?state.anchor.as_ref().map(|(leaf, ..)| leaf.view_number()),
            replay_seconds,
            "journal: replayed wal"
        );

        if opts.query_storage.is_some() {
            state.replay.get_or_insert_with(Replay::default);
        } else {
            state.replay = None;
        }
        let replay_index = match &state.replay {
            Some(replay) => Some(
                index_unreplayed(
                    std_fs.clone(),
                    data_dir.clone(),
                    &data_recovered.segments,
                    replay,
                )
                .await?,
            ),
            None => None,
        };
        let pending_payloads = PendingPayloads::new(opts.pending_payload_bytes);

        let wal_segments = Arc::new(parking_lot::Mutex::new(Segments::from(
            wal_recovered.segments,
        )));
        let data_segments = Arc::new(parking_lot::Mutex::new(Segments::from(
            data_recovered.segments,
        )));
        let state = Arc::new(parking_lot::Mutex::new(state));
        let hook: Arc<dyn SnapshotHook> = Arc::new(StateSnapshotHook(state.clone()));

        let (wal, wal_thread) = lane::spawn_lane(
            std_fs.clone(),
            wal_dir.clone(),
            LaneConfig {
                stream: Stream::Wal,
                mode: LaneMode::Snapshot,
                segment_bytes: WAL_SEGMENT_BYTES,
                max_key_span: WAL_MAX_VIEW_SPAN,
                max_batch_bytes: WAL_MAX_BATCH_BYTES,
                max_snapshot_bytes: MAX_SNAPSHOT_BYTES,
                in_flight_bytes: IN_FLIGHT_BYTES,
            },
            (wal_recovered.next_seq, wal_recovered.next_lsn),
            Some(hook),
            wal_segments.clone(),
            None,
        )
        .context("spawning wal writer thread")?;

        let (data, data_thread) = lane::spawn_lane(
            std_fs,
            data_dir.clone(),
            LaneConfig {
                stream: Stream::Data,
                mode: LaneMode::Append,
                segment_bytes: DATA_SEGMENT_BYTES,
                max_key_span: u64::MAX,
                max_batch_bytes: DATA_MAX_BATCH_BYTES,
                // Never read: the data lane has no `SnapshotHook`, so no `Snapshot` frame is ever
                // written on it.
                max_snapshot_bytes: MAX_SNAPSHOT_BYTES,
                in_flight_bytes: IN_FLIGHT_BYTES,
            },
            (data_recovered.next_seq, data_recovered.next_lsn),
            None,
            data_segments.clone(),
            replay_index.clone(),
        )
        .context("spawning data writer thread")?;

        Ok(Self {
            inner: Arc::new(Inner {
                state,
                wal,
                data,
                _threads: JoinOnDrop(vec![wal_thread, data_thread]),
                wal_segments,
                data_segments,
                gc_lock: parking_lot::Mutex::new(()),
                side,
                opts,
                wal_dir,
                data_dir,
                probe,
                limits,
                replay_seconds,
                replay_index,
                pending_payloads: parking_lot::Mutex::new(pending_payloads),
                _lock: lock_file,
            }),
            metrics: Arc::new(PersistenceMetricsValue::default()),
        })
    }

    /// Applies `rec` to `State` under the state lock, then enqueues it on the wal lane. LSN
    /// allocation happens inside `Lane::enqueue`, still under the same lock, so channel order,
    /// lsn order and `State::apply` order always agree (see the design's write path).
    async fn put_wal(&self, rec: Record, class: Class) -> anyhow::Result<()> {
        let kind = rec.kind();
        let view = rec.view();
        let body = rec.encode()?;
        let max = self.inner.limits.wal;
        ensure!(
            body.len() <= max as usize,
            "wal record of kind {kind:?} is {} bytes, over the {max}-byte limit",
            body.len()
        );
        let (lsn, finalized) = {
            let mut state = self.inner.state.lock();
            let finalized = state.apply(&rec);
            let lsn = self.inner.wal.enqueue(view, kind.into(), body, class, None);
            (lsn, finalized)
        };
        if let Some(finalized) = finalized {
            // Fail fast, same as replay: the cert was already dropped from `pending_state_certs`
            // by `state.apply` above, so silently continuing would lose it for good.
            self.inner
                .side
                .insert_state_cert(finalized.epoch, finalized.cert)
                .await?;
        }
        if class == Class::Durable {
            self.inner.wal.wait_durable(lsn).await?;
        }
        Ok(())
    }

    /// Data records have no `State` effect, so there is no lock to hold: only the in-flight byte
    /// budget bounds them.
    async fn put_data(&self, rec: Record, class: Class) -> anyhow::Result<()> {
        let kind = rec.kind();
        let view = rec.view();
        let body = rec.encode()?;
        let max = self.inner.limits.data;
        ensure!(
            body.len() <= max as usize,
            "data record of kind {kind:?} is {} bytes, over the {max}-byte limit",
            body.len()
        );
        let permit = self.inner.data.reserve(body.len() as u32).await;
        let lsn = self
            .inner
            .data
            .enqueue(view, kind.into(), body, class, Some(permit));
        if class == Class::Durable {
            self.inner.data.wait_durable(lsn).await?;
        }
        Ok(())
    }

    /// Cold linear scan for a historical record: reconstruction/test/operator-tool path only,
    /// with no production caller. Picks segments whose bound (from `Segments`) covers `view`.
    async fn scan_for<T>(
        &self,
        stream: Stream,
        kind: Kind,
        view: ViewNumber,
    ) -> anyhow::Result<Option<T>>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let (dir, segments) = match stream {
            Stream::Wal => (self.inner.wal_dir.clone(), &self.inner.wal_segments),
            Stream::Data => (self.inner.data_dir.clone(), &self.inner.data_segments),
            Stream::Payload | Stream::Share => unreachable!("journal has no {stream:?} lane"),
        };
        let key = view.u64();
        // (seq, sealed): only the last segment is active and may have a torn tail.
        let candidates: Vec<(u64, bool)> = {
            let segments = segments.lock();
            let list = segments.list();
            let active = list.last().map(|m| m.seq);
            list.iter()
                .filter(|m| m.max_key >= key)
                .map(|m| (m.seq, Some(m.seq) != active))
                .collect()
        };

        tokio::task::spawn_blocking(move || -> anyhow::Result<Option<T>> {
            for (seq, sealed) in candidates {
                let path = lane::segment_path(&dir, seq);
                let bytes = std::fs::read(&path)
                    .with_context(|| format!("reading segment {}", path.display()))?;
                let found = find_in_segment(&bytes, kind, key, sealed)
                    .with_context(|| format!("scanning segment {}", path.display()))?;
                if let Some(body) = found {
                    return Ok(Some(bincode::deserialize::<T>(&body)?));
                }
            }
            Ok(None)
        })
        .await
        .context("scan_for: blocking task panicked")?
    }

    /// Unlinks the segments `wal_unlinkable` and `data_unlinkable` select. Passes are serialized by
    /// `gc_lock`: see `lane::prune` for why an unlink failure stops the pass instead of skipping
    /// ahead. On a query node, `replayed` keeps every data segment the query service still has to
    /// ingest.
    async fn gc(&self, decided: ViewNumber, replayed: Option<u64>) -> anyhow::Result<()> {
        let inner = self.inner.clone();
        let bound = decided.u64().saturating_sub(inner.opts.view_retention);

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let _guard = inner.gc_lock.lock();
            let pruned = prune_both(&inner, bound, replayed);
            // Also after a failed pass: the segments it did unlink are already gone from
            // `data_segments`.
            if let Some(index) = &inner.replay_index {
                let oldest = inner
                    .data_segments
                    .lock()
                    .list()
                    .first()
                    .map(|meta| meta.seq);
                index
                    .lock()
                    .retain(|_, location| oldest.is_some_and(|oldest| location.seq >= oldest));
            }
            let (wal_bytes, data_bytes) = pruned?;
            if let Some(bytes) = wal_bytes {
                inner.wal.set_total_bytes(bytes);
            }
            if let Some(bytes) = data_bytes {
                inner.data.set_total_bytes(bytes);
            }
            Ok(())
        })
        .await
        .context("gc: blocking task panicked")??;
        Ok(())
    }

    /// The newest `kind` data record at `view`. A query node reads it through its index, any
    /// other node scans the segments that could hold it.
    async fn read_data<T>(&self, kind: Kind, view: ViewNumber) -> anyhow::Result<Option<T>>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let Some(index) = &self.inner.replay_index else {
            return self.scan_for(Stream::Data, kind, view).await;
        };
        let Some(location) = index.lock().get(&(view.u64(), kind.into())).copied() else {
            return Ok(None);
        };
        let path = lane::segment_path(&self.inner.data_dir, location.seq);
        tokio::task::spawn_blocking(move || -> anyhow::Result<Option<T>> {
            let file = match std::fs::File::open(&path) {
                Ok(file) => file,
                // GC unlinked the segment after the lookup.
                Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
                Err(err) => {
                    return Err(err).with_context(|| format!("opening segment {}", path.display()));
                },
            };
            let mut frame = vec![0; format::FRAME_HEADER_LEN + location.len as usize];
            file.read_exact_at(&mut frame, location.offset)
                .with_context(|| format!("reading segment {}", path.display()))?;
            let (_, body, _) = format::decode_frame(&frame, location.lsn).map_err(|err| {
                anyhow::anyhow!(
                    "corrupt frame in segment {} at offset {}: {err:?}",
                    path.display(),
                    location.offset
                )
            })?;
            Ok(Some(bincode::deserialize(body)?))
        })
        .await
        .context("read_data: blocking task panicked")?
    }

    /// Replay decided leaves to `consumer` in batches until caught up.
    ///
    /// The cursor moves only after the consumer accepted a whole batch, so a consumer failure
    /// leaves the batch to be retried by the next call.
    async fn replay_decided(
        &self,
        deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()> {
        loop {
            let batch = self
                .inner
                .state
                .lock()
                .replay
                .as_ref()
                .and_then(next_replay_batch);
            let Some(ReplayBatch {
                leaves,
                cert2,
                to_view,
                to_height,
            }) = batch
            else {
                break;
            };
            let mut chain = Vec::with_capacity(leaves.len());
            let mut attached = Vec::new();
            for (
                view,
                ReplayLeaf {
                    mut leaf,
                    cert,
                    state_cert,
                },
            ) in leaves.into_iter().rev()
            {
                if self.fill_payload(view, &mut leaf).await? {
                    attached.push(view);
                }
                let vid_share = self
                    .read_data::<Proposal<SeqTypes, VidDisperseShare<SeqTypes>>>(Kind::Vid, view)
                    .await?
                    .map(|proposal| proposal.data);
                chain.push(DecidedLeaf {
                    info: LeafInfo {
                        leaf,
                        vid_share,
                        state_cert,
                        state: Default::default(),
                        delta: Default::default(),
                    },
                    cert,
                });
            }
            for event in decide_events_from_chain(chain, cert2, deciding_qc.clone()) {
                consumer.handle_event(&event).await?;
            }
            self.put_wal(
                Record::Processed {
                    view: to_view,
                    height: to_height,
                },
                Class::Durable,
            )
            .await?;
            // Only once the batch is processed: a failed batch is retried and needs its payloads.
            let mut pending = self.inner.pending_payloads.lock();
            for view in attached {
                pending.remove(view);
            }
        }
        Ok(())
    }

    /// Fill `leaf` with its payload, from a pending payload of the same block, else from a DA
    /// proposal. Returns whether a pending payload was used.
    async fn fill_payload(&self, view: ViewNumber, leaf: &mut Leaf2) -> anyhow::Result<bool> {
        let pending = self
            .inner
            .pending_payloads
            .lock()
            .payload_for(view, leaf.block_header());
        if let Some(payload) = pending {
            leaf.fill_block_payload_unchecked((*payload).clone());
            return Ok(true);
        }
        match self
            .read_data::<Proposal<SeqTypes, DaProposal2<SeqTypes>>>(Kind::Da, view)
            .await?
        {
            Some(proposal) => leaf.fill_block_payload_unchecked(Payload::from_bytes(
                &proposal.data.encoded_transactions,
                &proposal.data.metadata,
            )),
            // The genesis view has no DA proposal, and its payload is always empty.
            None if view == ViewNumber::genesis() => {
                leaf.fill_block_payload_unchecked(Payload::empty().0)
            },
            None => {
                tracing::warn!(%view, "payload not available at replay, the query service must fetch this block")
            },
        }
        Ok(false)
    }

    /// Send every pending payload at or below `cursor` that no replayed leaf took: forks,
    /// timed-out views, and payloads that arrived after their view was replayed. The consumer
    /// checks every separate payload against the decided chain.
    async fn send_unattached_payloads(
        &self,
        cursor: ViewNumber,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()> {
        let payloads = self.inner.pending_payloads.lock().take_up_to(cursor);
        for (view, PendingPayload { header, payload }) in payloads {
            let event = CoordinatorEvent::BlockPayload {
                view,
                header,
                payload,
            };
            consumer.handle_event(&event).await?;
        }
        Ok(())
    }

    /// On a query node, hold `payload` for the query service until `view` is replayed.
    fn hold_payload(&self, view: ViewNumber, header: &Header, payload: Arc<Payload>) {
        if self.inner.replay_index.is_none() {
            return;
        }
        let dropped = self
            .inner
            .pending_payloads
            .lock()
            .insert(view, header.clone(), payload);
        if !dropped.is_empty() {
            tracing::warn!(
                ?dropped,
                "pending payloads over the memory cap dropped, the query service must fetch these \
                 blocks"
            );
        }
    }

    /// Hold the payload a decided leaf arrived with, unless the `BlockPayload` event for the
    /// same block already did. That event precedes the decide, so this only matters when the
    /// event channel overflowed.
    fn hold_decided_payload(&self, leaf: &Leaf2, payload: Payload) {
        let (view, header) = (leaf.view_number(), leaf.block_header());
        if self.inner.pending_payloads.lock().holds(view, header) {
            return;
        }
        self.hold_payload(view, header, Arc::new(payload));
    }

    /// The view the query service is replayed up to, on a query node.
    fn replay_cursor(&self) -> Option<ViewNumber> {
        self.inner
            .state
            .lock()
            .replay
            .as_ref()
            .and_then(|replay| replay.cursor)
            .map(|(view, _)| view)
    }
}

/// Unlinks the wal and data segments GC selects. Wal first: a failure there leaves data untouched.
/// Returns the bytes left per stream, `None` for a stream nothing was unlinked from.
fn prune_both(
    inner: &Inner,
    bound: u64,
    replayed: Option<u64>,
) -> anyhow::Result<(Option<u64>, Option<u64>)> {
    let wal = wal_unlinkable(&inner.wal_segments.lock(), bound);
    let wal_bytes = lane::prune(&StdFs, &inner.wal_dir, &inner.wal_segments, &wal)?;
    let data = data_unlinkable(
        &inner.data_segments.lock(),
        bound,
        inner.opts.max_bytes,
        replayed,
    );
    let data_bytes = lane::prune(&StdFs, &inner.data_dir, &inner.data_segments, &data)?;
    Ok((wal_bytes, data_bytes))
}

/// Age-based only. The two newest segments stay: a crash mid-roll falls back to the older one's
/// snapshot.
fn wal_unlinkable(segments: &Segments, bound: u64) -> Vec<SegmentMeta> {
    let mut out = segments.oldest_below(bound);
    out.truncate(segments.list().len().saturating_sub(2));
    out
}

/// Age-based, then oldest-first while the stream exceeds `max_bytes`. Segments holding a key after
/// `replayed` stay either way: the query service still has to ingest them.
fn data_unlinkable(
    segments: &Segments,
    bound: u64,
    max_bytes: u64,
    replayed: Option<u64>,
) -> Vec<SegmentMeta> {
    let mut by_age = segments.oldest_below(bound);
    let replayable = by_age
        .iter()
        .take_while(|meta| replayed.is_none_or(|replayed| meta.max_key <= replayed))
        .count();
    by_age.truncate(replayable);
    let by_bytes = segments.oldest_over_bytes(max_bytes, replayed);
    std::cmp::max_by_key(by_age, by_bytes, Vec::len)
}

/// Body of the last `kind` record at `key` in a whole segment file. A torn tail is expected on
/// the active segment; on a sealed one (fsynced before the roll) it is corruption.
fn find_in_segment(
    bytes: &[u8],
    kind: Kind,
    key: u64,
    sealed: bool,
) -> anyhow::Result<Option<Vec<u8>>> {
    ensure!(
        bytes.len() >= SegmentHeader::LEN,
        "segment shorter than its header"
    );
    let header = SegmentHeader::decode(bytes).context("corrupt segment header")?;
    let kind = format::Kind::from(kind);
    let mut found = None;
    let (end, ..) = format::scan(
        &bytes[SegmentHeader::LEN..],
        header.first_lsn,
        Kind::is_known,
        |h, body| {
            if h.kind == kind && h.key == key {
                found = Some(body.to_vec());
            }
            Ok(())
        },
    )?;
    if let ScanEnd::Torn(offset) = end {
        ensure!(!sealed, "sealed segment corrupt at frame offset {offset}");
    }
    Ok(found)
}

/// Index the data frames a query node may still replay: every record past the replay cursor,
/// decided or not yet. `max_key` only grows along a stream, so segments ending at or before the
/// cursor hold nothing replay needs.
async fn index_unreplayed(
    fs: Arc<StdFs>,
    dir: PathBuf,
    segments: &[SegmentMeta],
    replay: &Replay,
) -> anyhow::Result<DataIndex> {
    let index = DataIndex::default();
    let cursor = replay.cursor.map_or(0, |(view, _)| view.u64());
    let seqs = segments
        .iter()
        .filter(|meta| replay.cursor.is_none() || meta.max_key > cursor)
        .map(|meta| meta.seq)
        .collect::<Vec<_>>();
    let frames = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<_>> {
        let mut frames = Vec::new();
        for seq in seqs {
            frames.extend(lane::frame_locations(
                &*fs,
                &dir,
                Stream::Data,
                seq,
                Kind::is_known,
            )?);
        }
        Ok(frames)
    })
    .await
    .context("data index task panicked")??;
    index.lock().extend(
        frames
            .into_iter()
            .map(|(header, location)| ((header.key, header.kind), location)),
    );
    Ok(index)
}

/// A run of consecutive decided leaves, oldest first, ready to become decide events.
struct ReplayBatch {
    leaves: Vec<(ViewNumber, ReplayLeaf)>,
    cert2: Option<Certificate2<SeqTypes>>,
    to_view: ViewNumber,
    to_height: u64,
}

/// The decided leaves after the replay cursor, stopping at the first height gap.
///
/// A gap the newest decides can still fill holds the cursor instead: consensus may yet decide the
/// missing leaf, and replaying past it would leave the query service without that block.
fn next_replay_batch(replay: &Replay) -> Option<ReplayBatch> {
    let watermark = replay.leaves.keys().next_back()?.u64();
    let mut parent = replay.cursor.map(|(_, height)| height);
    let mut leaves = Vec::new();
    for (view, entry) in &replay.leaves {
        let height = entry.leaf.block_header().block_number();
        if let Some(parent) = parent
            && height != parent + 1
        {
            if !leaves.is_empty() {
                break;
            }
            if height > parent + 1 && within_gap_fill_horizon(view.u64(), watermark) {
                tracing::info!(
                    height,
                    parent,
                    %view,
                    "holding the replay cursor for a gap-fill"
                );
                return None;
            }
        }
        parent = Some(height);
        leaves.push((*view, entry.clone()));
        if leaves.len() == MAX_REPLAY_VIEWS {
            break;
        }
    }
    let (to_view, to_height) = leaves
        .last()
        .map(|(view, entry)| (*view, entry.leaf.block_header().block_number()))?;
    Some(ReplayBatch {
        cert2: replay.cert2.get(&to_view).cloned(),
        to_height,
        to_view,
        leaves,
    })
}

#[async_trait]
impl SequencerPersistence for Persistence {
    async fn load_config(&self) -> anyhow::Result<Option<NetworkConfig>> {
        self.inner.side.load_config().await
    }

    async fn save_config(&self, cfg: &NetworkConfig) -> anyhow::Result<()> {
        self.inner.side.save_config(cfg).await
    }

    async fn load_latest_acted_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        Ok(self.inner.state.lock().voted)
    }

    async fn load_restart_view(&self) -> anyhow::Result<Option<ViewNumber>> {
        Ok(self.inner.state.lock().restart)
    }

    async fn persist_decided_leaves(
        &self,
        _decided_view: ViewNumber,
        leaf_chain: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, CertificatePair<SeqTypes>)> + Send,
        _deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        _consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()> {
        // No event consumer call: a query node replays these from `process_decided_events`, off
        // the voting path, and any other node runs `NullEventConsumer`.
        let leaf_chain: Vec<_> = leaf_chain
            .into_iter()
            .map(|(info, cert)| (info.leaf.clone(), cert))
            .collect();
        for (mut leaf, cert) in leaf_chain {
            // The wal record carries no payload: a query node holds it in memory until the
            // replay, any other node has no use for it.
            if let Some(payload) = leaf.unfill_block_payload() {
                self.hold_decided_payload(&leaf, payload);
            }
            let record = Record::Leaf {
                leaf,
                qc: cert.qc().clone(),
                next_epoch_qc: cert.next_epoch_qc().cloned(),
            };
            self.put_wal(record, Class::Enqueue).await?;
        }
        Ok(())
    }

    async fn process_decided_events(
        &self,
        decided_view: ViewNumber,
        deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<Option<ViewNumber>> {
        let now = Instant::now();
        let processed = if self.inner.replay_index.is_some() {
            // GC runs on what did get replayed even when a batch failed, so a query service
            // outage does not also stop the wal from being pruned.
            let replayed = self.replay_decided(deciding_qc, consumer).await;
            let cursor = self.replay_cursor();
            self.gc(decided_view, Some(cursor.map_or(0, |view| view.u64())))
                .await?;
            replayed?;
            if let Some(cursor) = cursor {
                self.send_unattached_payloads(cursor, consumer).await?;
            }
            cursor
        } else {
            self.gc(decided_view, None).await?;
            Some(decided_view)
        };
        self.metrics
            .internal_process_decided_events_duration
            .add_point(now.elapsed().as_secs_f64());
        Ok(processed)
    }

    async fn load_anchor_leaf(&self) -> anyhow::Result<Option<(Leaf2, CertificatePair<SeqTypes>)>> {
        Ok(self.inner.state.lock().anchor_pair())
    }

    async fn load_da_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal2<SeqTypes>>>> {
        self.read_data(Kind::Da, view).await
    }

    async fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, VidDisperseShare<SeqTypes>>>> {
        self.read_data(Kind::Vid, view).await
    }

    async fn append_vid(
        &self,
        proposal: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let now = Instant::now();
        let res = self
            .put_data(Record::Vid(proposal.clone()), Class::Durable)
            .await;
        self.metrics
            .internal_append_vid_duration
            .add_point(now.elapsed().as_secs_f64());
        res
    }

    async fn append_da(
        &self,
        _proposal: &Proposal<SeqTypes, DaProposal<SeqTypes>>,
        _vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        anyhow::bail!(
            "storage-journal does not support the legacy DA proposal format; it requires \
             genesis.base_version >= 0.6"
        )
    }

    async fn record_action(
        &self,
        view: ViewNumber,
        _epoch: Option<EpochNumber>,
        action: HotShotAction,
    ) -> anyhow::Result<()> {
        if !matches!(action, HotShotAction::Propose | HotShotAction::Vote) {
            return Ok(());
        }
        self.put_wal(Record::Action { view, kind: action }, Class::Durable)
            .await
    }

    async fn append_quorum_proposal2(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let now = Instant::now();
        let res = self
            .put_wal(Record::Proposal(proposal.clone()), Class::Durable)
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
        self.put_wal(Record::Cert2 { view, cert: cert2 }, Class::Durable)
            .await
    }

    async fn load_cert2(&self, view: ViewNumber) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        self.scan_for(Stream::Wal, Kind::Cert2, view).await
    }

    async fn load_quorum_proposals(
        &self,
    ) -> anyhow::Result<BTreeMap<ViewNumber, Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>>>
    {
        Ok(self.inner.state.lock().proposals.clone())
    }

    async fn load_quorum_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>> {
        self.inner
            .state
            .lock()
            .proposals
            .get(&view)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("proposal {view:?} not available"))
    }

    async fn load_upgrade_certificate(
        &self,
    ) -> anyhow::Result<Option<UpgradeCertificate<SeqTypes>>> {
        Ok(self.inner.state.lock().upgrade.clone())
    }

    async fn store_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        self.put_wal(Record::Upgrade(decided_upgrade_certificate), Class::Durable)
            .await
    }

    async fn load_next_epoch_quorum_certificate(
        &self,
    ) -> anyhow::Result<Option<NextEpochQuorumCertificate2<SeqTypes>>> {
        Ok(self.inner.state.lock().next_epoch_qc.clone())
    }

    async fn append_next_epoch_high_qc2(
        &self,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        self.put_wal(Record::NextEpochQc(next_epoch_high_qc), Class::Durable)
            .await
    }

    async fn store_eqc(
        &self,
        high_qc: QuorumCertificate2<SeqTypes>,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        self.put_wal(Record::Eqc(high_qc, next_epoch_high_qc), Class::Durable)
            .await
    }

    async fn load_eqc(
        &self,
    ) -> Option<(
        QuorumCertificate2<SeqTypes>,
        NextEpochQuorumCertificate2<SeqTypes>,
    )> {
        self.inner.state.lock().eqc.clone()
    }

    async fn append_high_qc2(&self, high_qc: QuorumCertificate2<SeqTypes>) -> anyhow::Result<()> {
        self.put_wal(Record::HighQc2(high_qc), Class::Durable).await
    }

    async fn load_high_qc2(&self) -> anyhow::Result<Option<QuorumCertificate2<SeqTypes>>> {
        Ok(self.inner.state.lock().high_qc2.clone())
    }

    async fn append_da2(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        _vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        // Only a query node replays payloads from here. No other node keeps them, same as sql/fs.
        if self.inner.replay_index.is_none() {
            return Ok(());
        }
        let now = Instant::now();
        // Durable: the query service is replayed from this record, so it must survive a power
        // loss.
        let res = self
            .put_data(Record::Da(proposal.clone()), Class::Durable)
            .await;
        self.metrics
            .internal_append_da2_duration
            .add_point(now.elapsed().as_secs_f64());
        res
    }

    async fn append_pending_payload(
        &self,
        view: ViewNumber,
        header: &Header,
        payload: &Arc<Payload>,
    ) -> anyhow::Result<()> {
        self.hold_payload(view, header, Arc::clone(payload));
        Ok(())
    }

    async fn store_drb_input(&self, drb_input: DrbInput) -> anyhow::Result<()> {
        self.inner.side.store_drb_input(drb_input).await
    }

    async fn load_drb_input(&self, epoch: u64) -> anyhow::Result<DrbInput> {
        self.inner.side.load_drb_input(epoch).await
    }

    async fn store_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        self.inner.side.store_drb_result(epoch, drb_result).await
    }

    async fn add_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        self.put_wal(Record::StateCert(state_cert), Class::Enqueue)
            .await
    }

    async fn load_start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>> {
        self.inner.side.load_start_epoch_info().await
    }

    async fn load_state_cert(
        &self,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        self.inner.side.load_state_cert().await
    }

    async fn get_state_cert_by_epoch(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        self.inner.side.get_state_cert_by_epoch(epoch).await
    }

    async fn insert_state_cert(
        &self,
        epoch: u64,
        cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        self.inner.side.insert_state_cert(epoch, cert).await
    }

    fn enable_metrics(&mut self, metrics: &dyn Metrics) {
        self.metrics = Arc::new(PersistenceMetricsValue::new(metrics));
        metrics
            .create_histogram(
                "journal_replay_seconds".to_string(),
                Some("seconds".to_string()),
            )
            .add_point(self.inner.replay_seconds);
        self.inner.probe.register(&*metrics.subgroup("disk".into()));
        LaneMetrics::install(
            metrics,
            "journal",
            "stream",
            &[&self.inner.wal, &self.inner.data],
        );
    }
}

#[async_trait]
impl MembershipPersistence for Persistence {
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        self.inner.side.load_stake(epoch).await
    }

    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        self.inner.side.load_latest_stake(limit).await
    }

    async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        self.inner.side.load_drb_result(epoch).await
    }

    async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        self.inner.side.load_epoch_root(epoch).await
    }

    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: Header,
    ) -> anyhow::Result<()> {
        self.inner.side.store_epoch_root(epoch, block_header).await
    }

    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        self.inner
            .side
            .store_stake(epoch, stake, block_reward, stake_table_hash)
            .await
    }

    async fn store_events(
        &self,
        to_l1_block: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        self.inner.side.store_events(to_l1_block, events).await
    }

    async fn load_events(
        &self,
        from_l1_block: u64,
        to_l1_block: u64,
    ) -> anyhow::Result<(
        Option<EventsPersistenceRead>,
        Vec<(EventKey, StakeTableEvent)>,
    )> {
        self.inner
            .side
            .load_events(from_l1_block, to_l1_block)
            .await
    }

    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        self.inner.side.delete_stake_tables().await
    }

    async fn store_all_validators(
        &self,
        epoch: EpochNumber,
        all_validators: RegisteredValidatorMap,
    ) -> anyhow::Result<()> {
        self.inner
            .side
            .store_all_validators(epoch, all_validators)
            .await
    }

    async fn load_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        self.inner
            .side
            .load_all_validators(epoch, offset, limit)
            .await
    }
}

#[async_trait]
impl DhtPersistentStorage for Persistence {
    async fn save(&self, records: Vec<SerializableRecord>) -> anyhow::Result<()> {
        self.inner.side.save(records).await
    }

    async fn load(&self) -> anyhow::Result<Vec<SerializableRecord>> {
        self.inner.side.load().await
    }
}

#[cfg(test)]
mod tests {
    use espresso_types::{NodeState, ValidatedState, traits::NullEventConsumer};
    use hotshot::types::{BLSPubKey, SignatureKey};
    use hotshot_example_types::node_types::TEST_VERSIONS;
    use hotshot_query_service::metrics::PrometheusMetrics;
    use hotshot_types::{traits::EncodeBytes, utils::EpochTransitionIndicator};
    use tempfile::TempDir;

    use super::{testing::TEST_MAX_BLOCK_SIZE, *};
    use crate::persistence::tests::{TestablePersistence, consecutive_height_chain, decide_range};

    fn segments(stream: Stream, spec: &[(u64, u64, u64)]) -> Segments {
        spec.iter()
            .map(|&(seq, bytes, max_key)| SegmentMeta {
                stream,
                seq,
                bytes,
                max_key,
            })
            .collect::<Vec<_>>()
            .into()
    }

    fn seqs(list: Vec<SegmentMeta>) -> Vec<u64> {
        list.iter().map(|m| m.seq).collect()
    }

    #[test]
    fn wal_unlinkable_keeps_two_newest_segments() {
        let segs = segments(
            Stream::Wal,
            &[(1, 100, 5), (2, 100, 10), (3, 100, 20), (4, 100, 30)],
        );
        // bound 50: every segment is old enough, but only those outside the two newest qualify.
        assert_eq!(seqs(wal_unlinkable(&segs, 50)), vec![1, 2]);
    }

    #[test]
    fn wal_unlinkable_never_touches_active_or_recent_segments() {
        let segs = segments(Stream::Wal, &[(1, 100, 0), (2, 100, 0)]);
        assert!(wal_unlinkable(&segs, 1_000_000).is_empty());
        let segs = segments(Stream::Wal, &[(1, 100, 0), (2, 100, 0), (3, 100, 0)]);
        assert_eq!(seqs(wal_unlinkable(&segs, 1_000_000)), vec![1]);
    }

    #[test]
    fn wal_unlinkable_never_skips_a_segment_to_reach_a_younger_looking_one() {
        let segs = segments(
            Stream::Wal,
            &[(1, 100, 60), (2, 100, 0), (3, 100, 80), (4, 100, 90)],
        );
        assert!(wal_unlinkable(&segs, 50).is_empty());
    }

    #[test]
    fn data_unlinkable_drops_oldest_first_over_byte_cap() {
        let segs = segments(
            Stream::Data,
            &[(1, 40, 100), (2, 40, 100), (3, 40, 100), (4, 40, 100)],
        );
        // All young, total 160 > cap 90: drop oldest until under the cap; the active one stays.
        assert_eq!(seqs(data_unlinkable(&segs, 0, 90, None)), vec![1, 2]);
    }

    #[test]
    fn data_unlinkable_age_based_ignores_byte_cap() {
        let segs = segments(Stream::Data, &[(1, 10, 0), (2, 10, 1000), (3, 10, 1000)]);
        assert_eq!(seqs(data_unlinkable(&segs, 990, u64::MAX, None)), vec![1]);
    }

    #[test]
    fn data_unlinkable_keeps_unreplayed_segments_even_over_cap() {
        let segs = segments(
            Stream::Data,
            &[(1, 40, 5), (2, 40, 10), (3, 40, 20), (4, 40, 30)],
        );
        assert_eq!(seqs(data_unlinkable(&segs, 0, 0, Some(10))), vec![1, 2]);
        assert_eq!(seqs(data_unlinkable(&segs, 1000, 0, Some(10))), vec![1, 2]);
        assert_eq!(seqs(data_unlinkable(&segs, 1000, 0, None)), vec![1, 2, 3]);
    }

    /// A payload the decide carries (built, reconstructed or fetched before it) reaches the
    /// query service at replay without a `BlockPayload` event, and neither it nor an event's
    /// payload is written to the journal.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn payloads_are_held_in_memory_not_written() {
        let tmp = TempDir::new().unwrap();
        let storage = Persistence::connect(&tmp).await;
        let consumer = PayloadCollector::default();
        let mut chain = consecutive_height_chain(3).await;

        let carried = Payload::from_bytes(&[1; 100], chain[1].0.block_header().metadata());
        chain[1].0.fill_block_payload_unchecked(carried.clone());
        decide_range(&storage, &chain, 0..2, &consumer).await;

        let obtained = Payload::from_bytes(&[2; 50], chain[2].0.block_header().metadata());
        let event = CoordinatorEvent::BlockPayload {
            view: ViewNumber::new(2),
            header: chain[2].0.block_header().clone(),
            payload: Arc::new(obtained.clone()),
        };
        assert_eq!(
            storage.persist_event(&event, &NullEventConsumer).await,
            None
        );
        decide_range(&storage, &chain, 2..3, &consumer).await;

        assert_eq!(
            consumer.take(),
            vec![(1, Some(carried)), (2, Some(obtained))]
        );
        let index = storage.inner.replay_index.as_ref().unwrap().lock();
        assert!(
            index
                .keys()
                .all(|(_, kind)| *kind != Kind::PendingPayload.into())
        );
        assert!(storage.inner.pending_payloads.lock().by_view.is_empty());
    }

    /// The pending payload cap drops the oldest views first and counts a replaced payload once.
    #[tokio::test]
    async fn pending_payloads_drop_oldest_over_cap() {
        let header = genesis_leaf().await.block_header().clone();
        let payload = |len: usize| Arc::new(Payload::from_bytes(&vec![0; len], header.metadata()));
        let view = ViewNumber::new;
        let mut pending = PendingPayloads::new(250);

        assert!(
            pending
                .insert(view(1), header.clone(), payload(100))
                .is_empty()
        );
        assert!(
            pending
                .insert(view(2), header.clone(), payload(100))
                .is_empty()
        );
        assert_eq!(
            pending.insert(view(3), header.clone(), payload(100)),
            vec![view(1)]
        );
        assert!(!pending.holds(view(1), &header));
        assert!(pending.holds(view(2), &header));
        assert_eq!(pending.bytes, 200);

        assert!(
            pending
                .insert(view(3), header.clone(), payload(50))
                .is_empty()
        );
        assert_eq!(pending.bytes, 150);

        // A lone payload over the cap is kept.
        assert_eq!(
            pending.insert(view(4), header.clone(), payload(300)),
            vec![view(2), view(3)]
        );
        assert!(pending.holds(view(4), &header));

        assert_eq!(pending.take_up_to(view(4)).len(), 1);
        assert_eq!(pending.bytes, 0);
    }

    /// A decided leaf's view and the payload it was delivered with.
    type DeliveredPayload = (u64, Option<Payload>);

    /// The payload of each decided leaf but genesis, in delivery order.
    #[derive(Clone, Debug, Default)]
    struct PayloadCollector(Arc<parking_lot::Mutex<Vec<DeliveredPayload>>>);

    impl PayloadCollector {
        fn take(&self) -> Vec<DeliveredPayload> {
            std::mem::take(&mut *self.0.lock())
        }
    }

    #[async_trait]
    impl EventConsumer for PayloadCollector {
        async fn handle_event(&self, event: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()> {
            if let CoordinatorEvent::NewDecide { leaf_infos, .. } = event {
                self.0.lock().extend(
                    leaf_infos
                        .iter()
                        .rev()
                        .filter(|info| info.leaf.view_number() != ViewNumber::genesis())
                        .map(|info| (info.leaf.view_number().u64(), info.leaf.block_payload())),
                );
            }
            Ok(())
        }
    }

    #[test]
    fn record_limits_keep_minimums_for_small_blocks() {
        assert_eq!(
            RecordLimits::new(0),
            RecordLimits {
                wal: RECORD_HEADROOM_BYTES as u32,
                data: DATA_MIN_RECORD_BYTES as u32,
            }
        );
    }

    #[test]
    fn record_limits_scale_with_block_size() {
        let limits = RecordLimits::new(TEST_MAX_BLOCK_SIZE);
        assert_eq!(
            u64::from(limits.wal),
            RECORD_HEADROOM_BYTES + TEST_MAX_BLOCK_SIZE
        );
        assert_eq!(u64::from(limits.data), DATA_MIN_RECORD_BYTES);

        let big = 400_000_000;
        assert_eq!(
            u64::from(RecordLimits::new(big).data),
            RECORD_HEADROOM_BYTES + 3 * big
        );
    }

    #[test]
    fn record_limits_stop_at_frame_ceiling() {
        let limits = RecordLimits::new(u64::MAX);
        assert_eq!(limits.wal, u32::MAX);
        assert_eq!(limits.data, u32::MAX);
    }

    // Regression test: VID shares and DA payloads grow with block size and stake (78 MB shares at
    // 40% stake with 1 MB txs), past the wal record bound.
    #[tokio::test]
    async fn stores_data_record_larger_than_wal_record_limit() {
        let tmp = TempDir::new().unwrap();
        let storage = Persistence::open(options(&tmp)).await.unwrap();
        let len = RecordLimits::new(TEST_MAX_BLOCK_SIZE).wal as usize + (16 << 20);
        // `append_da2` enqueues without waiting for the write; wait so the cold scan sees it.
        storage
            .put_data(da_record(len).await, Class::Durable)
            .await
            .unwrap();
        let stored = storage.load_da_proposal(ViewNumber::new(1)).await.unwrap();
        assert_eq!(stored.map(|p| p.data.encoded_transactions.len()), Some(len));
    }

    #[tokio::test]
    async fn put_data_rejects_record_over_data_limit() {
        let tmp = TempDir::new().unwrap();
        let mut storage = Persistence::open(options(&tmp)).await.unwrap();
        let len = da_record(1024).await.encode().unwrap().len();
        let inner = Arc::get_mut(&mut storage.inner).expect("no other handle to inner yet");
        inner.limits.data = len as u32 - 1;

        let err = storage
            .put_data(da_record(1024).await, Class::Durable)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("over the"), "{err:#}");

        let inner = Arc::get_mut(&mut storage.inner).expect("no other handle to inner yet");
        inner.limits.data = len as u32;
        storage
            .put_data(da_record(1024).await, Class::Durable)
            .await
            .unwrap();
    }

    #[test]
    fn find_in_segment_fails_on_corrupt_sealed_segment() {
        let header = SegmentHeader {
            stream: Stream::Data,
            seq: 1,
            first_lsn: 1,
            prev_max_key: 0,
        };
        let mut seg = header.encode().to_vec();
        format::encode_frame(&mut seg, 1, 5, Kind::Da.into(), b"five");
        format::encode_frame(&mut seg, 2, 6, Kind::Da.into(), b"six");
        let last = seg.len() - 1;
        seg[last] ^= 0xff;

        assert_eq!(
            find_in_segment(&seg, Kind::Da, 5, false).unwrap(),
            Some(b"five".to_vec())
        );
        assert_eq!(find_in_segment(&seg, Kind::Da, 6, false).unwrap(), None);
        find_in_segment(&seg, Kind::Da, 5, true).unwrap_err();
    }

    #[tokio::test]
    async fn open_fails_without_max_block_size() {
        let tmp = TempDir::new().unwrap();
        let err = Persistence::open(Options {
            max_block_size: None,
            ..options(&tmp)
        })
        .await
        .err()
        .expect("open must fail without max_block_size");
        assert!(err.to_string().contains("max_block_size"), "{err:#}");
    }

    fn options(tmp: &TempDir) -> Options {
        Options {
            path: tmp.path().to_path_buf(),
            view_retention: DEFAULT_VIEW_RETENTION,
            max_bytes: DEFAULT_MAX_BYTES,
            pending_payload_bytes: DEFAULT_PENDING_PAYLOAD_BYTES,
            ignore_existing: false,
            max_block_size: Some(TEST_MAX_BLOCK_SIZE),
            query_storage: None,
        }
    }

    async fn genesis_leaf() -> Leaf2 {
        Leaf2::genesis(
            &ValidatedState::default(),
            &NodeState::mock(),
            TEST_VERSIONS.test.base,
        )
        .await
    }

    /// A view-1 DA proposal whose payload is `len` bytes.
    async fn da_record(len: usize) -> Record {
        let payload = genesis_leaf().await.block_payload().unwrap();
        let (_, privkey) = BLSPubKey::generated_from_seed_indexed([0; 32], 1);
        Record::Da(Proposal {
            data: DaProposal2::<SeqTypes> {
                encoded_transactions: vec![7u8; len].into(),
                metadata: payload.ns_table().clone(),
                view_number: ViewNumber::new(1),
                epoch: None,
                epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
            },
            signature: BLSPubKey::sign(&privkey, &payload.encode()).unwrap(),
            _pd: Default::default(),
        })
    }

    // Regression test: both lanes share one metric family per name; registering a family per lane
    // panicked with Prometheus `AlreadyReg` at node start.
    #[tokio::test]
    async fn enables_prometheus_metrics() {
        let tmp = TempDir::new().unwrap();
        let mut storage = Persistence::open(options(&tmp)).await.unwrap();
        let metrics = PrometheusMetrics::default();
        storage.enable_metrics(&metrics);
        let exported = metrics.export().unwrap();
        assert!(exported.contains("journal_batch_records"), "{exported}");
    }

    // Regression test: the existing-layout guard must actually see the sql layout, not the
    // journal/wal, journal/data dirs this same call creates a moment later.
    #[tokio::test]
    async fn refuses_to_start_over_existing_sql_layout_without_ignore_existing() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("sqlite")).unwrap();
        std::fs::write(tmp.path().join("sqlite").join("database"), b"").unwrap();

        let err = Persistence::open(Options {
            path: tmp.path().to_path_buf(),
            view_retention: DEFAULT_VIEW_RETENTION,
            max_bytes: DEFAULT_MAX_BYTES,
            pending_payload_bytes: DEFAULT_PENDING_PAYLOAD_BYTES,
            ignore_existing: false,
            max_block_size: Some(TEST_MAX_BLOCK_SIZE),
            query_storage: None,
        })
        .await
        .err()
        .expect("open must refuse an existing sql layout");
        assert!(format!("{err:#}").contains("--journal-ignore-existing"));
    }
}

#[cfg(test)]
mod testing {
    use tempfile::TempDir;

    use super::*;
    use crate::persistence::tests::TestablePersistence;

    /// Mainnet's `max_block_size`.
    pub(super) const TEST_MAX_BLOCK_SIZE: u64 = 10_000_000;

    #[async_trait]
    impl TestablePersistence for Persistence {
        type Storage = TempDir;

        async fn tmp_storage() -> Self::Storage {
            TempDir::new().unwrap()
        }

        /// A query node's journal, so the shared tests cover replay. `mod tests` covers the
        /// consensus-only journal.
        fn options(storage: &Self::Storage) -> impl PersistenceOptions<Persistence = Self> {
            Options {
                path: storage.path().into(),
                view_retention: DEFAULT_VIEW_RETENTION,
                max_bytes: DEFAULT_MAX_BYTES,
                pending_payload_bytes: DEFAULT_PENDING_PAYLOAD_BYTES,
                ignore_existing: false,
                max_block_size: Some(TEST_MAX_BLOCK_SIZE),
                query_storage: Some(QueryStorage::Fs(side_fs::Options::new(
                    storage.path().join("query"),
                ))),
            }
        }
    }
}
