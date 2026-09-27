//! Append-only journal persistence backend for non-query (light) nodes.
//!
//! Two independent streams (`wal`, `data`), each a std writer thread doing group commit with a
//! durable/enqueue sync policy; see the design doc for the write path, roll and recovery. Reads of
//! consensus state come from an in-memory `State` folded from wal records; `MembershipPersistence`
//! and `DhtPersistentStorage` delegate to a side `fs::Persistence` tree, which keeps fs semantics
//! (no fsync) since DRB/stake/state-cert data is recoverable from L1 or peers.

pub mod format;
pub mod lane;
pub mod state;

use std::{collections::BTreeMap, fs, io, path::PathBuf, sync::Arc, time::Instant};

use anyhow::{Context, ensure};
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
    message::Proposal,
    simple_certificate::{
        CertificatePair, LightClientStateUpdateCertificateV2, NextEpochQuorumCertificate2,
        QuorumCertificate2, UpgradeCertificate,
    },
    traits::metrics::Metrics,
};

use crate::{
    ViewNumber,
    persistence::{
        fs as side_fs,
        journal::{
            format::{Class, Kind, ScanEnd, SegmentHeader, Stream},
            lane::{JournalFs, Lane, LaneConfig, LaneMetrics, SegmentSet, SnapshotHook, StdFs},
            state::{Record, State},
        },
        persistence_metrics::PersistenceMetricsValue,
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

/// Options for the append-only journal storage backend.
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
}

#[async_trait]
impl PersistenceOptions for Options {
    type Persistence = Persistence;

    fn set_view_retention(&mut self, view_retention: u64) {
        self.view_retention = view_retention;
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
    segments: Arc<parking_lot::Mutex<SegmentSet>>,
    side: side_fs::Persistence,
    opts: Options,
    wal_dir: PathBuf,
    data_dir: PathBuf,
    probe: StorageProbe,
    limits: RecordLimits,
    /// Measured before any real `Metrics` is attached; `enable_metrics` records it once a
    /// histogram exists to record it into.
    replay_seconds: f64,
    _lock: std::fs::File,
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
            let wal = tokio::task::spawn_blocking(move || lane::recover(&*fs, &dir, Stream::Wal))
                .await
                .context("wal recovery task panicked")?
                .context("recovering wal stream")?;

            let fs = std_fs.clone();
            let dir = data_dir.clone();
            let data = tokio::task::spawn_blocking(move || lane::recover(&*fs, &dir, Stream::Data))
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
            let record = Record::decode(header.kind, header.view, body)
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

        let segments = Arc::new(parking_lot::Mutex::new(SegmentSet {
            wal: wal_recovered.segments,
            data: data_recovered.segments,
        }));
        let state = Arc::new(parking_lot::Mutex::new(state));
        let hook: Arc<dyn SnapshotHook> = Arc::new(StateSnapshotHook(state.clone()));

        let (wal, wal_thread) = lane::spawn_lane(
            std_fs.clone(),
            wal_dir.clone(),
            LaneConfig {
                stream: Stream::Wal,
                segment_bytes: WAL_SEGMENT_BYTES,
                max_view_span: WAL_MAX_VIEW_SPAN,
                max_batch_bytes: WAL_MAX_BATCH_BYTES,
                max_snapshot_bytes: MAX_SNAPSHOT_BYTES,
                in_flight_bytes: IN_FLIGHT_BYTES,
            },
            (wal_recovered.next_seq, wal_recovered.next_lsn),
            Some(hook),
            segments.clone(),
        )
        .context("spawning wal writer thread")?;

        let (data, data_thread) = lane::spawn_lane(
            std_fs,
            data_dir.clone(),
            LaneConfig {
                stream: Stream::Data,
                segment_bytes: DATA_SEGMENT_BYTES,
                max_view_span: u64::MAX,
                max_batch_bytes: DATA_MAX_BATCH_BYTES,
                // Never read: the data lane has no `SnapshotHook`, so no `Snapshot` frame is ever
                // written on it.
                max_snapshot_bytes: MAX_SNAPSHOT_BYTES,
                in_flight_bytes: IN_FLIGHT_BYTES,
            },
            (data_recovered.next_seq, data_recovered.next_lsn),
            None,
            segments.clone(),
        )
        .context("spawning data writer thread")?;

        Ok(Self {
            inner: Arc::new(Inner {
                state,
                wal,
                data,
                _threads: JoinOnDrop(vec![wal_thread, data_thread]),
                segments,
                side,
                opts,
                wal_dir,
                data_dir,
                probe,
                limits,
                replay_seconds,
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
            let lsn = self.inner.wal.enqueue(view, kind, body, class, None);
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
            .enqueue(view, kind, body, class, Some(permit));
        if class == Class::Durable {
            self.inner.data.wait_durable(lsn).await?;
        }
        Ok(())
    }

    /// Cold linear scan for a historical record: reconstruction/test/operator-tool path only,
    /// with no production caller. Picks segments whose bound (from `SegmentSet`) covers `view`.
    async fn scan_for<T>(
        &self,
        stream: Stream,
        kind: Kind,
        view: ViewNumber,
    ) -> anyhow::Result<Option<T>>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let dir = match stream {
            Stream::Wal => self.inner.wal_dir.clone(),
            Stream::Data => self.inner.data_dir.clone(),
        };
        let view_u64 = view.u64();
        // (seq, sealed): only the last segment is active and may have a torn tail.
        let candidates: Vec<(u64, bool)> = {
            let segments = self.inner.segments.lock();
            let list = segments.list(stream);
            let active = list.last().map(|m| m.seq);
            list.iter()
                .filter(|m| m.max_view >= view_u64)
                .map(|m| (m.seq, Some(m.seq) != active))
                .collect()
        };

        tokio::task::spawn_blocking(move || -> anyhow::Result<Option<T>> {
            for (seq, sealed) in candidates {
                let path = lane::segment_path(&dir, seq);
                let bytes = std::fs::read(&path)
                    .with_context(|| format!("reading segment {}", path.display()))?;
                let found = find_in_segment(&bytes, kind, view_u64, sealed)
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

    /// Unlinks segments `SegmentSet::to_unlink` selects, then fsyncs each touched directory once.
    async fn gc(&self, decided: ViewNumber) -> anyhow::Result<()> {
        let view_retention = self.inner.opts.view_retention;
        let max_bytes = self.inner.opts.max_bytes;
        let segments = self.inner.segments.clone();
        let wal_dir = self.inner.wal_dir.clone();
        let data_dir = self.inner.data_dir.clone();
        let wal_lane = self.inner.wal.clone();
        let data_lane = self.inner.data.clone();
        let decided = decided.u64();

        tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            // Drop from the in-memory set before unlinking: an overlapping GC pass never sees the
            // same segment twice, so a `NotFound` below is always benign.
            let (to_unlink, wal_bytes, data_bytes) = {
                let mut segments = segments.lock();
                let to_unlink = segments.to_unlink(decided, view_retention, max_bytes);
                segments.wal.retain(|m| {
                    !to_unlink
                        .iter()
                        .any(|u| u.stream == Stream::Wal && u.seq == m.seq)
                });
                segments.data.retain(|m| {
                    !to_unlink
                        .iter()
                        .any(|u| u.stream == Stream::Data && u.seq == m.seq)
                });
                let wal_bytes: u64 = segments.wal.iter().map(|m| m.bytes).sum();
                let data_bytes: u64 = segments.data.iter().map(|m| m.bytes).sum();
                (to_unlink, wal_bytes, data_bytes)
            };
            if to_unlink.is_empty() {
                return Ok(());
            }
            let std_fs = StdFs;
            let (mut wal_touched, mut data_touched) = (false, false);
            for meta in &to_unlink {
                let dir = match meta.stream {
                    Stream::Wal => &wal_dir,
                    Stream::Data => &data_dir,
                };
                match std_fs.remove(&lane::segment_path(dir, meta.seq)) {
                    Ok(()) => {},
                    Err(err) if err.kind() == io::ErrorKind::NotFound => {},
                    Err(err) => {
                        return Err(err).with_context(|| format!("unlinking segment {}", meta.seq));
                    },
                }
                match meta.stream {
                    Stream::Wal => wal_touched = true,
                    Stream::Data => data_touched = true,
                }
            }
            if wal_touched {
                std_fs.sync_dir(&wal_dir)?;
                wal_lane.set_total_bytes(wal_bytes);
            }
            if data_touched {
                std_fs.sync_dir(&data_dir)?;
                data_lane.set_total_bytes(data_bytes);
            }
            Ok(())
        })
        .await
        .context("gc: blocking task panicked")??;
        Ok(())
    }
}

/// Body of the last `kind` record at `view` in a whole segment file. A torn tail is expected on
/// the active segment; on a sealed one (fsynced before the roll) it is corruption.
fn find_in_segment(
    bytes: &[u8],
    kind: Kind,
    view: u64,
    sealed: bool,
) -> anyhow::Result<Option<Vec<u8>>> {
    ensure!(
        bytes.len() >= SegmentHeader::LEN,
        "segment shorter than its header"
    );
    let header = SegmentHeader::decode(bytes).context("corrupt segment header")?;
    let mut found = None;
    let (end, ..) = format::scan(&bytes[SegmentHeader::LEN..], header.first_lsn, |h, body| {
        if h.kind == kind && h.view == view {
            found = Some(body.to_vec());
        }
        Ok(())
    })?;
    if let ScanEnd::Torn(offset) = end {
        ensure!(!sealed, "sealed segment corrupt at frame offset {offset}");
    }
    Ok(found)
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
        // No event consumer call: non-query nodes run `NullEventConsumer`, and the query-service
        // ingestion this would feed needs the replaying store that only fs/sql provide.
        let records: Vec<_> = leaf_chain
            .into_iter()
            .map(|(info, cert)| {
                let mut leaf = info.leaf.clone();
                leaf.unfill_block_payload();
                Record::Leaf {
                    leaf,
                    qc: cert.qc().clone(),
                    next_epoch_qc: cert.next_epoch_qc().cloned(),
                }
            })
            .collect();
        for rec in records {
            self.put_wal(rec, Class::Enqueue).await?;
        }
        Ok(())
    }

    async fn process_decided_events(
        &self,
        decided_view: ViewNumber,
        _deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        _consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<Option<ViewNumber>> {
        let now = Instant::now();
        self.gc(decided_view).await?;
        self.metrics
            .internal_process_decided_events_duration
            .add_point(now.elapsed().as_secs_f64());
        Ok(Some(decided_view))
    }

    async fn load_anchor_leaf(&self) -> anyhow::Result<Option<(Leaf2, CertificatePair<SeqTypes>)>> {
        Ok(self.inner.state.lock().anchor_pair())
    }

    async fn load_da_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal2<SeqTypes>>>> {
        self.scan_for(Stream::Data, Kind::Da, view).await
    }

    async fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, VidDisperseShare<SeqTypes>>>> {
        self.scan_for(Stream::Data, Kind::Vid, view).await
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
        self.put_wal(Record::Cert2 { view, cert: cert2 }, Class::Enqueue)
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
        let now = Instant::now();
        let res = self
            .put_data(Record::Da(proposal.clone()), Class::Enqueue)
            .await;
        self.metrics
            .internal_append_da2_duration
            .add_point(now.elapsed().as_secs_f64());
        res
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
        LaneMetrics::install(metrics, &[&self.inner.wal, &self.inner.data]);
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
    use espresso_types::{NodeState, ValidatedState};
    use hotshot::types::{BLSPubKey, SignatureKey};
    use hotshot_example_types::node_types::TEST_VERSIONS;
    use hotshot_query_service::metrics::PrometheusMetrics;
    use hotshot_types::{traits::EncodeBytes, utils::EpochTransitionIndicator};
    use tempfile::TempDir;

    use super::{testing::TEST_MAX_BLOCK_SIZE, *};

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
            prev_max_view: 0,
        };
        let mut seg = header.encode().to_vec();
        format::encode_frame(&mut seg, 1, 5, Kind::Da, b"five");
        format::encode_frame(&mut seg, 2, 6, Kind::Da, b"six");
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
            ignore_existing: false,
            max_block_size: Some(TEST_MAX_BLOCK_SIZE),
        }
    }

    /// A view-1 DA proposal whose payload is `len` bytes.
    async fn da_record(len: usize) -> Record {
        let leaf = Leaf2::genesis(
            &ValidatedState::default(),
            &NodeState::mock(),
            TEST_VERSIONS.test.base,
        )
        .await;
        let payload = leaf.block_payload().unwrap();
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
            ignore_existing: false,
            max_block_size: Some(TEST_MAX_BLOCK_SIZE),
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

        fn options(storage: &Self::Storage) -> impl PersistenceOptions<Persistence = Self> {
            Options {
                path: storage.path().into(),
                view_retention: DEFAULT_VIEW_RETENTION,
                max_bytes: DEFAULT_MAX_BYTES,
                ignore_existing: false,
                max_block_size: Some(TEST_MAX_BLOCK_SIZE),
            }
        }
    }
}
