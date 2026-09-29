//! Per-stream writer thread: group commit, segment rolling, recovery and GC bookkeeping.
//!
//! Operates purely on encoded bytes (`Kind`/`Stream`/`Class` from `format.rs`); it knows nothing
//! about `Record`/`State` (`state.rs`). The one place that needs the current `State` -- writing a
//! fresh `Snapshot` at the start of every wal segment -- goes through the `SnapshotHook` trait so
//! this module stays decoupled.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use anyhow::Context;
use hotshot_types::traits::metrics::{Gauge, Histogram, Metrics, NoMetrics};
use parking_lot::Mutex;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, watch};

use crate::persistence::journal::format::{
    self, Class, FrameHeader, Kind, Lsn, ScanEnd, SegmentHeader, Stream,
};

/// Per-stream journal metrics. A lane's writer thread starts recording into this before any real
/// `Metrics` implementation exists (`enable_metrics` runs only after `Persistence::open` returns),
/// so every field starts as the `NoMetrics` no-op and `install` swaps in the real thing later;
/// every call site keeps the same `&LaneMetrics` handle across that swap.
pub struct LaneMetrics {
    batch_records: Mutex<Box<dyn Histogram>>,
    batch_bytes: Mutex<Box<dyn Histogram>>,
    fsync_seconds: Mutex<Box<dyn Histogram>>,
    queue_bytes: Mutex<Box<dyn Gauge>>,
    total_bytes: Mutex<Box<dyn Gauge>>,
}

impl LaneMetrics {
    fn noop() -> Self {
        Self {
            batch_records: Mutex::new(Box::new(NoMetrics)),
            batch_bytes: Mutex::new(Box::new(NoMetrics)),
            fsync_seconds: Mutex::new(Box::new(NoMetrics)),
            queue_bytes: Mutex::new(Box::new(NoMetrics)),
            total_bytes: Mutex::new(Box::new(NoMetrics)),
        }
    }

    /// Installs real histograms/gauges labelled `stream = "wal"|"data"` on every lane. Each family
    /// is registered once: registering a name twice fails with Prometheus.
    pub fn install(metrics: &dyn Metrics, lanes: &[&Lane]) {
        let labels = || vec!["stream".to_string()];
        let batch_records = metrics.histogram_family("journal_batch_records".into(), labels());
        let batch_bytes = metrics.histogram_family("journal_batch_bytes".into(), labels());
        let fsync_seconds = metrics.histogram_family("journal_fsync_seconds".into(), labels());
        let queue_bytes = metrics.gauge_family("journal_queue_bytes".into(), labels());
        let total_bytes = metrics.gauge_family("journal_bytes".into(), labels());
        for lane in lanes {
            let label = vec![lane.stream.label().to_string()];
            let m = &lane.metrics;
            *m.batch_records.lock() = batch_records.create(label.clone());
            *m.batch_bytes.lock() = batch_bytes.create(label.clone());
            *m.fsync_seconds.lock() = fsync_seconds.create(label.clone());
            *m.queue_bytes.lock() = queue_bytes.create(label.clone());
            *m.total_bytes.lock() = total_bytes.create(label);
        }
    }

    fn record_batch(&self, records: usize, bytes: usize) {
        self.batch_records.lock().add_point(records as f64);
        self.batch_bytes.lock().add_point(bytes as f64);
    }

    fn record_fsync(&self, seconds: f64) {
        self.fsync_seconds.lock().add_point(seconds);
    }

    fn set_queue_bytes(&self, bytes: u64) {
        self.queue_bytes.lock().set(bytes as usize);
    }

    /// Total bytes on disk across every live segment of this stream; the caller (writer thread or
    /// `Persistence::gc`) computes the sum from `SegmentSet`.
    fn set_total_bytes(&self, bytes: u64) {
        self.total_bytes.lock().set(bytes as usize);
    }
}

/// Times `file.sync_data()` and records it into `metrics`, regardless of the outcome: an error is
/// about to abort the process anyway, but the sample still belongs in the histogram.
fn timed_sync<Fh: JournalFile>(file: &mut Fh, metrics: &LaneMetrics) -> io::Result<()> {
    let start = Instant::now();
    let result = file.sync_data();
    metrics.record_fsync(start.elapsed().as_secs_f64());
    result
}

/// Abstraction over the filesystem so tests can inject a crash model (`MemFs`).
pub trait JournalFs: Send + Sync + 'static {
    type File: JournalFile;

    /// Creates a brand new segment file; must fail if `path` already exists.
    fn create(&self, path: &Path) -> io::Result<Self::File>;
    /// Opens an existing segment file for truncation during recovery.
    fn open_write(&self, path: &Path) -> io::Result<Self::File>;
    /// Reads an entire segment into memory. Only ever used for small, known-bounded reads (e.g.
    /// `MemFs`'s own header/length helpers); recovery uses `open_read_stream` instead.
    fn open_read(&self, path: &Path) -> io::Result<Vec<u8>>;
    /// A `Read` stream over the whole segment, for scanning it frame by frame without holding the
    /// (possibly gigabyte-sized) file in memory all at once.
    fn open_read_stream(&self, path: &Path) -> io::Result<Box<dyn Read>>;
    /// Reads just the fixed-size segment header, without loading the rest of a (possibly
    /// gigabyte-sized) data segment.
    fn read_header(&self, path: &Path) -> io::Result<Vec<u8>>;
    fn len(&self, path: &Path) -> io::Result<u64>;
    fn list(&self, dir: &Path) -> io::Result<Vec<PathBuf>>;
    fn remove(&self, path: &Path) -> io::Result<()>;
    fn sync_dir(&self, dir: &Path) -> io::Result<()>;
}

pub trait JournalFile: Send + 'static {
    fn write_all_at(&mut self, off: u64, buf: &[u8]) -> io::Result<()>;
    fn sync_data(&mut self) -> io::Result<()>;
    fn set_len(&mut self, len: u64) -> io::Result<()>;
}

/// Runs the snapshot side of a wal roll under the shared state lock. `drain` must be called
/// exactly once, while the lock is held, before this returns: it flushes any wal records already
/// enqueued (and thus already reflected in `State`) into the old segment, so the returned snapshot
/// is consistent with everything on disk up to the roll point.
pub trait SnapshotHook: Send + Sync + 'static {
    fn snapshot_locked(&self, drain: &mut dyn FnMut()) -> Vec<u8>;
}

pub struct StdFs;

impl JournalFs for StdFs {
    type File = std::fs::File;

    fn create(&self, path: &Path) -> io::Result<Self::File> {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
    }

    fn open_write(&self, path: &Path) -> io::Result<Self::File> {
        std::fs::OpenOptions::new().write(true).open(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn open_read_stream(&self, path: &Path) -> io::Result<Box<dyn Read>> {
        Ok(Box::new(std::io::BufReader::new(std::fs::File::open(
            path,
        )?)))
    }

    fn read_header(&self, path: &Path) -> io::Result<Vec<u8>> {
        let mut file = std::fs::File::open(path)?;
        let mut buf = vec![0u8; SegmentHeader::LEN];
        file.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn len(&self, path: &Path) -> io::Result<u64> {
        Ok(std::fs::metadata(path)?.len())
    }

    fn list(&self, dir: &Path) -> io::Result<Vec<PathBuf>> {
        std::fs::read_dir(dir)?
            .map(|e| e.map(|e| e.path()))
            .collect()
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        std::fs::File::open(dir)?.sync_all()
    }
}

impl JournalFile for std::fs::File {
    fn write_all_at(&mut self, off: u64, buf: &[u8]) -> io::Result<()> {
        use std::io::{Seek, SeekFrom, Write};
        self.seek(SeekFrom::Start(off))?;
        self.write_all(buf)
    }

    fn sync_data(&mut self) -> io::Result<()> {
        std::fs::File::sync_data(self)
    }

    fn set_len(&mut self, len: u64) -> io::Result<()> {
        std::fs::File::set_len(self, len)
    }
}

pub(crate) fn segment_path(dir: &Path, seq: u64) -> PathBuf {
    dir.join(format!("{seq:016x}.log"))
}

fn parse_seq(path: &Path) -> Option<u64> {
    if path.extension().and_then(|e| e.to_str()) != Some("log") {
        return None;
    }
    u64::from_str_radix(path.file_stem()?.to_str()?, 16).ok()
}

#[derive(Clone, Copy, Debug)]
pub struct SegmentMeta {
    pub stream: Stream,
    pub seq: u64,
    pub bytes: u64,
    pub max_view: u64,
}

/// Where one frame lives, so it can be read back without scanning its segment.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Location {
    pub seq: u64,
    pub offset: u64,
    pub lsn: Lsn,
    pub len: u32,
}

/// The newest data frame for each `(view, kind)`, kept only on a query node, whose replay reads
/// every decided block's payload and share back. Entries appear once their bytes are written.
pub(crate) type DataIndex = Arc<Mutex<std::collections::BTreeMap<(u64, Kind), Location>>>;

/// In-memory index of every segment a stream currently has on disk, kept live by the writer
/// thread. Lets GC compute what to unlink without a directory scan or a full-segment read.
#[derive(Clone, Debug, Default)]
pub struct SegmentSet {
    pub wal: Vec<SegmentMeta>,
    pub data: Vec<SegmentMeta>,
}

impl SegmentSet {
    fn list_mut(&mut self, stream: Stream) -> &mut Vec<SegmentMeta> {
        match stream {
            Stream::Wal => &mut self.wal,
            Stream::Data => &mut self.data,
        }
    }

    pub fn list(&self, stream: Stream) -> &[SegmentMeta] {
        match stream {
            Stream::Wal => &self.wal,
            Stream::Data => &self.data,
        }
    }

    fn push_active(&mut self, meta: SegmentMeta) {
        self.list_mut(meta.stream).push(meta);
    }

    fn update_active(&mut self, stream: Stream, bytes: u64, max_view: u64) {
        if let Some(last) = self.list_mut(stream).last_mut() {
            last.bytes = bytes;
            last.max_view = max_view;
        }
    }

    /// Segments GC may unlink. `wal`: age-based only, never the active segment or the one behind
    /// it (a crash mid-roll falls back to that segment's snapshot). `data`: age-based, then
    /// oldest-first while the running total exceeds `max_bytes`; never the active segment.
    ///
    /// Both only ever take a contiguous prefix of the oldest segments and stop at the first one
    /// that doesn't qualify: `max_view` is monotonic (see `open_new_segment`), so a segment that
    /// isn't old enough means nothing sealed after it is either, and unlinking one segment while
    /// keeping an older one would leave a gap GC can never fill in later.
    ///
    /// `data` segments holding any view after `replayed` stay, even over `max_bytes`: a query
    /// node's query service still has to ingest them.
    pub fn to_unlink(
        &self,
        decided: u64,
        view_retention: u64,
        max_bytes: u64,
        replayed: Option<u64>,
    ) -> Vec<SegmentMeta> {
        let bound = decided.saturating_sub(view_retention);
        let mut out = Vec::new();

        if self.wal.len() > 2 {
            out.extend(
                self.wal[..self.wal.len() - 2]
                    .iter()
                    .take_while(|m| m.max_view < bound)
                    .copied(),
            );
        }

        if !self.data.is_empty() {
            let mut total: u64 = self.data.iter().map(|m| m.bytes).sum();
            for meta in &self.data[..self.data.len() - 1] {
                if replayed.is_some_and(|replayed| meta.max_view > replayed) {
                    break;
                }
                if meta.max_view >= bound && total <= max_bytes {
                    break;
                }
                out.push(*meta);
                total = total.saturating_sub(meta.bytes);
            }
        }

        out
    }
}

/// Bytes remaining on disk per stream after a `prune` pass, if that stream's live set changed
/// (`None` means nothing to report, matching `Persistence::gc`'s prior skip-if-untouched metric).
#[derive(Debug)]
pub struct PruneStats {
    pub wal_bytes: Option<u64>,
    pub data_bytes: Option<u64>,
}

/// Unlinks the segments `SegmentSet::to_unlink` selects, oldest first within each stream. A segment
/// is dropped from `segments` only once its file is actually gone (`NotFound` counts as gone), so a
/// failed unlink leaves it both on disk and tracked; the pass stops there and returns the error
/// instead of unlinking younger segments first, which would leave a sequence gap recovery can never
/// close. The caller must serialize passes (e.g. a gc lock): `to_unlink` is computed once up front,
/// so an overlapping pass could otherwise select the same segment twice.
pub fn prune<F: JournalFs>(
    fs: &F,
    segments: &Mutex<SegmentSet>,
    wal_dir: &Path,
    data_dir: &Path,
    decided: u64,
    view_retention: u64,
    max_bytes: u64,
    replayed: Option<u64>,
) -> anyhow::Result<PruneStats> {
    let to_unlink = segments
        .lock()
        .to_unlink(decided, view_retention, max_bytes, replayed);
    let mut stats = PruneStats {
        wal_bytes: None,
        data_bytes: None,
    };

    for (stream, dir) in [(Stream::Wal, wal_dir), (Stream::Data, data_dir)] {
        let mut touched = false;
        for meta in to_unlink.iter().filter(|m| m.stream == stream) {
            match fs.remove(&segment_path(dir, meta.seq)) {
                Ok(()) => {},
                Err(err) if err.kind() == io::ErrorKind::NotFound => {},
                Err(err) => {
                    if touched {
                        fs.sync_dir(dir)?;
                    }
                    return Err(err).with_context(|| format!("unlinking segment {}", meta.seq));
                },
            }
            segments
                .lock()
                .list_mut(stream)
                .retain(|m| m.seq != meta.seq);
            touched = true;
        }
        if touched {
            fs.sync_dir(dir)?;
            let bytes = segments.lock().list(stream).iter().map(|m| m.bytes).sum();
            match stream {
                Stream::Wal => stats.wal_bytes = Some(bytes),
                Stream::Data => stats.data_bytes = Some(bytes),
            }
        }
    }
    Ok(stats)
}

pub struct LaneConfig {
    pub stream: Stream,
    pub segment_bytes: u64,
    pub max_view_span: u64,
    pub max_batch_bytes: usize,
    /// Bound on a wal `Snapshot` frame; irrelevant for a data lane, which never writes one.
    pub max_snapshot_bytes: u32,
    pub in_flight_bytes: usize,
}

/// One record queued for the writer thread. Allocated and sent atomically (see `Lane::enqueue`)
/// so channel order always matches `lsn` order, which `format::scan` requires on replay.
struct Write {
    lsn: Lsn,
    view: u64,
    kind: Kind,
    body: Vec<u8>,
    class: Class,
    _permit: Option<OwnedSemaphorePermit>,
}

#[derive(Clone)]
pub struct Lane {
    stream: Stream,
    tx: mpsc::UnboundedSender<Write>,
    /// Guards lsn allocation *and* the channel send together, so a data-stream caller racing
    /// another can't allocate lsn N, then lose the race to send before lsn N+1. Shared with the
    /// writer thread so a wal roll's snapshot frame reserves its lsn from the same counter,
    /// instead of guessing `active.last_lsn + 1` and racing the next `enqueue`.
    alloc: Arc<Mutex<Lsn>>,
    durable: watch::Receiver<Lsn>,
    in_flight: Arc<Semaphore>,
    in_flight_budget: u32,
    /// Bytes enqueued (sent to the writer thread) but not yet picked up into a batch; mirrors
    /// `journal_queue_bytes`. Shared with the writer thread, which subtracts a batch's bytes once
    /// it drains them off the channel.
    queue_bytes: Arc<AtomicU64>,
    metrics: Arc<LaneMetrics>,
}

impl Lane {
    /// Caller holds the shared state lock for wal records (so lsn order matches
    /// `State::apply` order); data records don't need it.
    pub fn enqueue(
        &self,
        view: u64,
        kind: Kind,
        body: Vec<u8>,
        class: Class,
        permit: Option<OwnedSemaphorePermit>,
    ) -> Lsn {
        let mut next = self.alloc.lock();
        let lsn = *next;
        *next += 1;
        let queued = self
            .queue_bytes
            .fetch_add(body.len() as u64, Ordering::Relaxed)
            + body.len() as u64;
        self.metrics.set_queue_bytes(queued);
        // The writer thread may already have exited (process is aborting); a dropped receiver
        // just means this record never lands, which is moot since the process is on its way down.
        let _ = self.tx.send(Write {
            lsn,
            view,
            kind,
            body,
            class,
            _permit: permit,
        });
        lsn
    }

    /// Data-stream backpressure: blocks until `bytes` of the in-flight budget are free. Capped at
    /// the whole budget so a record over it cannot block forever.
    pub async fn reserve(&self, bytes: u32) -> OwnedSemaphorePermit {
        self.in_flight
            .clone()
            .acquire_many_owned(bytes.min(self.in_flight_budget))
            .await
            .expect("in-flight semaphore is never closed")
    }

    pub async fn wait_durable(&self, lsn: Lsn) -> anyhow::Result<()> {
        self.durable
            .clone()
            .wait_for(|durable| *durable >= lsn)
            .await
            .context("journal writer thread exited before acking a durable write")?;
        Ok(())
    }

    /// Reports `journal_bytes`: the caller (`Persistence::gc`) sums `SegmentSet` after unlinking.
    pub fn set_total_bytes(&self, bytes: u64) {
        self.metrics.set_total_bytes(bytes);
    }
}

struct ActiveSegment<Fh> {
    file: Fh,
    seq: u64,
    offset: u64,
    /// Last lsn actually written to this segment (including any not-yet-fsynced bytes).
    last_lsn: Lsn,
    /// View baseline for this segment, used to bound `max_view_span`; unset (`None`) until the
    /// first real record lands, since the wal's own `Snapshot` record has no consensus view.
    first_view: Option<u64>,
    max_view: u64,
    /// Offset right after the segment's own `Snapshot` frame (or right after the header, for a
    /// data segment, which never writes one). `over_bytes` in `Writer::run` measures from here,
    /// not from `offset` directly, so a large snapshot can't make the very next batch look like
    /// it already overflows `segment_bytes` and roll again immediately.
    snapshot_end: u64,
}

fn open_new_segment<F: JournalFs>(
    fs: &F,
    dir: &Path,
    stream: Stream,
    seq: u64,
    first_lsn: Lsn,
    prev_max_view: u64,
    metrics: &LaneMetrics,
) -> anyhow::Result<ActiveSegment<F::File>> {
    let path = segment_path(dir, seq);
    let mut file = fs
        .create(&path)
        .with_context(|| format!("creating segment {}", path.display()))?;
    let header = SegmentHeader {
        stream,
        seq,
        first_lsn,
        prev_max_view,
    }
    .encode();
    file.write_all_at(0, &header)?;
    timed_sync(&mut file, metrics)?;
    fs.sync_dir(dir)?;
    Ok(ActiveSegment {
        file,
        seq,
        offset: SegmentHeader::LEN as u64,
        last_lsn: first_lsn.saturating_sub(1),
        first_view: None,
        // Seeded from the previous segment's bound, not 0: `SegmentMeta::max_view` is a running
        // max across the whole stream, so GC (`SegmentSet::to_unlink`) never sees a segment appear
        // younger than the one sealed right before it (e.g. a snapshot-only segment with no real
        // records of its own).
        max_view: prev_max_view,
        snapshot_end: SegmentHeader::LEN as u64,
    })
}

/// Segments recovered for one stream, ready for the caller (`journal.rs`) to fold into `State`.
pub struct Recovered {
    pub segments: Vec<SegmentMeta>,
    pub next_seq: u64,
    pub next_lsn: Lsn,
    /// wal only: the replay source's clean records, in lsn order, starting with `Snapshot`.
    pub wal_records: Vec<(FrameHeader, Vec<u8>)>,
}

/// One segment's header plus everything a full scan of it produces.
struct SegmentRead {
    header: SegmentHeader,
    /// Only populated when `collect_records` is set (wal streams; recovery never needs a data
    /// stream's decoded records, only where its tail ends).
    records: Vec<(FrameHeader, Vec<u8>)>,
    locations: Vec<(FrameHeader, Location)>,
    end: ScanEnd,
    next_lsn: Lsn,
    max_view: u64,
}

/// Fills `buf` from `r` as far as it goes before EOF, returning how many bytes landed (which may
/// be less than `buf.len()` at a genuine end of stream, distinguishing a clean end from a torn
/// one exactly like `format::decode_frame`'s short-read check does on an in-memory slice).
fn read_up_to(r: &mut dyn Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut got = 0;
    while got < buf.len() {
        match r.read(&mut buf[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }
    Ok(got)
}

/// Scans one segment frame by frame over a stream instead of loading it whole, so a (possibly
/// gigabyte-sized) data segment's recovery scan holds at most one frame in memory at a time.
/// `format::decode_frame` still does the actual crc/lsn/kind validation, on a per-frame buffer
/// reused (cleared and resized) across iterations.
fn read_segment<F: JournalFs>(
    fs: &F,
    path: &Path,
    seq: u64,
    stream: Stream,
    collect_records: bool,
) -> anyhow::Result<SegmentRead> {
    let file_len = fs.len(path)?;
    let mut reader = fs.open_read_stream(path)?;

    let mut header_buf = [0u8; SegmentHeader::LEN];
    read_up_to(&mut *reader, &mut header_buf)
        .with_context(|| format!("reading header of segment {}", path.display()))?;
    let header = SegmentHeader::decode(&header_buf)
        .with_context(|| format!("corrupt segment header {}", path.display()))?;
    anyhow::ensure!(header.seq == seq, "segment header seq mismatch in {path:?}");
    anyhow::ensure!(
        header.stream as u8 == stream as u8,
        "stream mismatch in {path:?}"
    );

    let mut records = Vec::new();
    let mut locations = Vec::new();
    let mut frame_buf: Vec<u8> = Vec::new();
    let mut expect_lsn = header.first_lsn;
    let mut max_view = 0u64;
    let mut consumed = 0u64;

    let end = loop {
        frame_buf.clear();
        frame_buf.resize(format::FRAME_HEADER_LEN, 0);
        let read = read_up_to(&mut *reader, &mut frame_buf)
            .with_context(|| format!("reading segment {}", path.display()))?;
        if read == 0 {
            break ScanEnd::Clean(consumed);
        }
        if read < frame_buf.len() {
            break ScanEnd::Torn(consumed);
        }

        let len = u32::from_le_bytes(frame_buf[4..8].try_into().unwrap());
        // A torn length can claim up to 4 GiB; never allocate past what the file holds.
        let remaining = file_len
            .saturating_sub((SegmentHeader::LEN + format::FRAME_HEADER_LEN) as u64 + consumed);
        if u64::from(len) > remaining {
            break ScanEnd::Torn(consumed);
        }
        let header_len = frame_buf.len();
        frame_buf.resize(header_len + len as usize, 0);
        let body_read = read_up_to(&mut *reader, &mut frame_buf[header_len..])
            .with_context(|| format!("reading segment {}", path.display()))?;
        if body_read < len as usize {
            break ScanEnd::Torn(consumed);
        }

        match format::decode_frame(&frame_buf, expect_lsn) {
            Ok((frame_header, body, total)) => {
                if collect_records {
                    records.push((frame_header, body.to_vec()));
                }
                locations.push((
                    frame_header,
                    Location {
                        seq,
                        offset: SegmentHeader::LEN as u64 + consumed,
                        lsn: frame_header.lsn,
                        len: frame_header.len,
                    },
                ));
                max_view = max_view.max(frame_header.view);
                expect_lsn = frame_header.lsn + 1;
                consumed += total as u64;
            },
            Err(_) => break ScanEnd::Torn(consumed),
        }
    };

    Ok(SegmentRead {
        header,
        records,
        locations,
        end,
        next_lsn: expect_lsn,
        max_view,
    })
}

/// Every frame in segment `seq` with where it lives, stopping at a torn tail. Streams the segment
/// like recovery does, so at most one frame is in memory at a time.
pub(crate) fn frame_locations<F: JournalFs>(
    fs: &F,
    dir: &Path,
    stream: Stream,
    seq: u64,
) -> anyhow::Result<Vec<(FrameHeader, Location)>> {
    Ok(read_segment(fs, &segment_path(dir, seq), seq, stream, false)?.locations)
}

/// True only for a segment that was created but never durably written to: shorter than a header,
/// or a header-sized region of all zeros (what a filesystem that zero-fills new blocks leaves
/// behind after a crash right after `create`). This is the *only* case safe to silently drop and
/// retry with the segment behind it; any other unreadable/corrupt header is real corruption and
/// must be a hard error rather than a silent data loss.
fn is_unwritten_segment<F: JournalFs>(fs: &F, path: &Path) -> anyhow::Result<bool> {
    if fs.len(path)? < SegmentHeader::LEN as u64 {
        return Ok(true);
    }
    Ok(fs.read_header(path)?.iter().all(|&b| b == 0))
}

/// Recovers one stream's segments: validates headers, truncates a torn newest segment, and (wal
/// only) returns the records to replay `State` from.
///
/// The newest segment must contain at least one acked record, i.e. (wal only) start with a
/// `Snapshot`; if it was never written to (see `is_unwritten_segment`) or, for wal, is a valid
/// header with zero parseable records (a torn snapshot write), nothing in it was ever acked, so it
/// is a candidate to unlink and retry with the segment behind it. Nothing is actually unlinked
/// until a keepable segment is found: an error validating the fallback must not leave a segment
/// deleted with no replacement.
pub fn recover<F: JournalFs>(fs: &F, dir: &Path, stream: Stream) -> anyhow::Result<Recovered> {
    let mut paths: Vec<(u64, PathBuf)> = fs
        .list(dir)?
        .into_iter()
        .filter_map(|p| parse_seq(&p).map(|seq| (seq, p)))
        .collect();
    paths.sort_by_key(|(seq, _)| *seq);

    // A single check over the initial list suffices: `paths.pop()` below only ever shrinks it
    // from the end, which can't introduce a gap into what remains.
    for pair in paths.windows(2) {
        anyhow::ensure!(
            pair[1].0 == pair[0].0 + 1,
            "gap in {stream:?} segment sequence: {} then {}",
            pair[0].0,
            pair[1].0
        );
    }

    // Deferred until a keepable newest segment is confirmed (see the function doc).
    let mut to_remove: Vec<PathBuf> = Vec::new();
    // Set when a segment is queued for removal but its header was still readable: the segment
    // that becomes newest next iteration must end exactly where the dropped one's header began.
    let mut expect_next_lsn: Option<Lsn> = None;

    let (newest_seq, newest_path, read) = loop {
        let Some((seq, path)) = paths.last().cloned() else {
            for p in &to_remove {
                fs.remove(p)?;
            }
            if !to_remove.is_empty() {
                fs.sync_dir(dir)?;
            }
            return Ok(Recovered {
                segments: vec![],
                next_seq: 1,
                next_lsn: 1,
                wal_records: vec![],
            });
        };

        if is_unwritten_segment(fs, &path)? {
            tracing::warn!(
                ?stream,
                seq,
                "journal: dropping newest segment: never written"
            );
            to_remove.push(path);
            paths.pop();
            expect_next_lsn = None;
            continue;
        }

        let read = read_segment(fs, &path, seq, stream, stream == Stream::Wal)?;

        if let Some(expected) = expect_next_lsn {
            anyhow::ensure!(
                read.next_lsn == expected,
                "lsn gap in {stream:?}: segment {seq} ends at lsn {}, dropped segment expected \
                 {expected}",
                read.next_lsn
            );
        }

        if stream == Stream::Wal && read.records.is_empty() {
            tracing::warn!(seq, "journal: dropping newest wal segment: no snapshot");
            expect_next_lsn = Some(read.header.first_lsn);
            to_remove.push(path);
            paths.pop();
            continue;
        }

        if stream == Stream::Wal {
            anyhow::ensure!(
                read.records[0].0.kind == Kind::Snapshot,
                "wal segment {seq} has records but the first is {:?}, not Snapshot",
                read.records[0].0.kind
            );
        }

        break (seq, path, read);
    };

    for p in &to_remove {
        fs.remove(p)?;
    }
    if !to_remove.is_empty() {
        fs.sync_dir(dir)?;
    }

    // A valid newest segment: truncate any torn tail and always sync, so a crash between
    // truncating and the writer's first append can't leave an unsynced length change behind.
    let clean_end = match read.end {
        ScanEnd::Clean(n) | ScanEnd::Torn(n) => n,
    };
    let total = SegmentHeader::LEN as u64 + clean_end;
    let mut file = fs.open_write(&newest_path)?;
    if total < fs.len(&newest_path)? {
        file.set_len(total)?;
    }
    file.sync_data()?;

    let mut segments = Vec::with_capacity(paths.len());
    for (seq, path) in &paths[..paths.len() - 1] {
        let header_bytes = fs.read_header(path)?;
        let header = SegmentHeader::decode(&header_bytes)
            .with_context(|| format!("corrupt segment header {}", path.display()))?;
        anyhow::ensure!(
            header.seq == *seq,
            "segment header seq mismatch in {path:?}"
        );
        segments.push(SegmentMeta {
            stream,
            seq: *seq,
            bytes: fs.len(path)?,
            // Backfilled below from the following segment's header.
            max_view: 0,
        });
    }
    segments.push(SegmentMeta {
        stream,
        seq: newest_seq,
        bytes: total,
        // `read.max_view` alone is only this segment's own records; carry the running bound
        // forward from its header, same as the writer does for a freshly rolled segment.
        max_view: read.max_view.max(read.header.prev_max_view),
    });

    backfill_prev_max_view(fs, stream, &paths, &mut segments)?;

    Ok(Recovered {
        segments,
        next_seq: newest_seq + 1,
        next_lsn: read.next_lsn,
        wal_records: if stream == Stream::Wal {
            read.records
        } else {
            vec![]
        },
    })
}

fn backfill_prev_max_view<F: JournalFs>(
    fs: &F,
    stream: Stream,
    paths: &[(u64, PathBuf)],
    segments: &mut [SegmentMeta],
) -> anyhow::Result<()> {
    for i in 0..segments.len().saturating_sub(1) {
        if segments[i].max_view != 0 {
            continue;
        }
        let next_path = &paths[i + 1].1;
        let header_bytes = fs.read_header(next_path)?;
        let header = SegmentHeader::decode(&header_bytes)
            .with_context(|| format!("corrupt segment header {next_path:?}"))?;
        anyhow::ensure!(
            header.stream as u8 == stream as u8,
            "stream mismatch in {next_path:?}"
        );
        segments[i].max_view = header.prev_max_view;
    }
    Ok(())
}

/// Aborts the process if the writer thread's body panics, instead of leaving a dead thread behind
/// (`Lane::wait_durable` would then just hang: no one is left to ack anything).
fn abort_on_panic(f: impl FnOnce() + std::panic::UnwindSafe) {
    if std::panic::catch_unwind(f).is_err() {
        tracing::error!("journal: writer thread panicked");
        std::process::abort();
    }
}

/// `State` (in particular `proposals`) is not size-bounded, so a snapshot can legitimately grow
/// large; warn well before it gets anywhere near `max_snapshot_bytes` so an operator sees it
/// coming, then abort if it actually crosses that bound.
const SNAPSHOT_WARN_BYTES: u64 = 16 << 20;

fn check_snapshot_size(stream: Stream, body: &[u8], max_snapshot_bytes: u32) {
    if body.len() as u64 > SNAPSHOT_WARN_BYTES {
        tracing::warn!(
            ?stream,
            len = body.len(),
            "journal: wal snapshot is unusually large"
        );
    }
    if body.len() > max_snapshot_bytes as usize {
        tracing::error!(
            ?stream,
            len = body.len(),
            max = max_snapshot_bytes,
            "journal: snapshot exceeds max_snapshot_bytes"
        );
        std::process::abort();
    }
}

/// Spawns the writer thread and returns a handle to enqueue records on it plus the thread's join
/// handle, which the caller must join after dropping the `Lane` (see `journal::Inner`'s `Drop`).
/// `hook` is `Some` only for the wal lane: it provides the `Snapshot` payload opening every new
/// wal segment. `index`, when given, learns where every record this lane writes lives.
pub(crate) fn spawn_lane<F: JournalFs>(
    fs: Arc<F>,
    dir: PathBuf,
    cfg: LaneConfig,
    start: (u64, Lsn),
    hook: Option<Arc<dyn SnapshotHook>>,
    segments: Arc<Mutex<SegmentSet>>,
    index: Option<DataIndex>,
) -> anyhow::Result<(Lane, std::thread::JoinHandle<()>)> {
    let (next_seq, next_lsn) = start;
    let metrics = Arc::new(LaneMetrics::noop());
    let prev_max_view = segments
        .lock()
        .list(cfg.stream)
        .last()
        .map_or(0, |m| m.max_view);
    let mut active = open_new_segment(
        &*fs,
        &dir,
        cfg.stream,
        next_seq,
        next_lsn,
        prev_max_view,
        &metrics,
    )?;

    // The seed snapshot's lsn is consumed synchronously here, before `Lane` (and its lsn counter)
    // is handed to any caller, so there's no race with the writer thread's own send of it.
    let seed = hook.as_ref().map(|h| h.snapshot_locked(&mut || {}));
    let counter_start = if seed.is_some() {
        next_lsn + 1
    } else {
        next_lsn
    };

    let (tx, rx) = mpsc::unbounded_channel::<Write>();
    let (durable_tx, durable_rx) = watch::channel(active.last_lsn);
    let in_flight = Arc::new(Semaphore::new(cfg.in_flight_bytes));
    let in_flight_budget =
        u32::try_from(cfg.in_flight_bytes).context("in_flight_bytes must fit in u32")?;
    let alloc = Arc::new(Mutex::new(counter_start));
    let queue_bytes = Arc::new(AtomicU64::new(0));

    let seeded = seed.is_some();
    if let Some(body) = seed {
        check_snapshot_size(cfg.stream, &body, cfg.max_snapshot_bytes);
        let mut frame = Vec::new();
        format::encode_frame(&mut frame, next_lsn, 0, Kind::Snapshot, &body);
        active
            .file
            .write_all_at(active.offset, &frame)
            .context("writing initial wal snapshot")?;
        active.offset += frame.len() as u64;
        active.last_lsn = next_lsn;
        active.snapshot_end = active.offset;
        timed_sync(&mut active.file, &metrics).context("fsyncing initial wal snapshot")?;
    }

    segments.lock().push_active(SegmentMeta {
        stream: cfg.stream,
        seq: next_seq,
        bytes: active.offset,
        max_view: active.max_view,
    });
    // Published only after `segments` above reflects this segment: `wait_durable` callers must
    // never observe a durable lsn `scan_for`/gc can't yet see in `SegmentSet`.
    if seeded {
        durable_tx.send(active.last_lsn).ok();
    }

    let frame_buf = Vec::with_capacity(cfg.max_batch_bytes);
    let writer = Writer {
        fs,
        dir,
        cfg,
        rx,
        active,
        durable_tx,
        hook,
        segments,
        alloc: alloc.clone(),
        queue_bytes: queue_bytes.clone(),
        metrics: metrics.clone(),
        frame_buf,
        index,
    };
    let stream = writer.cfg.stream;
    let handle = std::thread::Builder::new()
        .name(format!("journal-{:?}", writer.cfg.stream).to_lowercase())
        .spawn(move || abort_on_panic(std::panic::AssertUnwindSafe(|| writer.run())))
        .context("spawning journal writer thread")?;

    Ok((
        Lane {
            stream,
            tx,
            alloc,
            durable: durable_rx,
            in_flight,
            in_flight_budget,
            queue_bytes,
            metrics,
        },
        handle,
    ))
}

/// Owns everything the writer thread touches. A struct instead of function arguments keeps `run`
/// and `roll` to one receiver argument each.
struct Writer<F: JournalFs> {
    fs: Arc<F>,
    dir: PathBuf,
    cfg: LaneConfig,
    rx: mpsc::UnboundedReceiver<Write>,
    active: ActiveSegment<F::File>,
    durable_tx: watch::Sender<Lsn>,
    hook: Option<Arc<dyn SnapshotHook>>,
    segments: Arc<Mutex<SegmentSet>>,
    /// Shared with `Lane::enqueue`; see the field doc on `Lane::alloc`.
    alloc: Arc<Mutex<Lsn>>,
    /// Shared with `Lane::enqueue`; see the field doc on `Lane::queue_bytes`.
    queue_bytes: Arc<AtomicU64>,
    metrics: Arc<LaneMetrics>,
    frame_buf: Vec<u8>,
    index: Option<DataIndex>,
}

/// Applies one record's bookkeeping to `active`: `max_view`, `first_view` (view 0 marks
/// `Snapshot`/`Upgrade`, never a span baseline, or `over_views` would latch true forever) and
/// `last_lsn`. Shared by `Writer::run`'s batch loop and `roll`'s drain, so a record landing during
/// a roll is tracked exactly like one landing in the normal path.
fn apply_write_to_active<Fh>(active: &mut ActiveSegment<Fh>, w: &Write) {
    active.max_view = active.max_view.max(w.view);
    if w.view != 0 {
        active.first_view.get_or_insert(w.view);
    }
    active.last_lsn = w.lsn;
}

impl<F: JournalFs> Writer<F> {
    fn run(mut self) {
        loop {
            let Some(first) = self.rx.blocking_recv() else {
                return; // Lane (and the Persistence holding it) was dropped.
            };
            let mut batch = vec![first];
            let mut batch_bytes = batch[0].body.len();
            while batch_bytes < self.cfg.max_batch_bytes {
                match self.rx.try_recv() {
                    Ok(w) => {
                        batch_bytes += w.body.len();
                        batch.push(w);
                    },
                    Err(_) => break,
                }
            }
            self.metrics.record_batch(batch.len(), batch_bytes);
            self.queue_bytes
                .fetch_sub(batch_bytes as u64, Ordering::Relaxed);
            self.metrics
                .set_queue_bytes(self.queue_bytes.load(Ordering::Relaxed));

            let mut durable_in_batch = false;
            let mut written = Vec::new();
            for w in &batch {
                if self.index.is_some() {
                    written.push((
                        (w.view, w.kind),
                        Location {
                            seq: self.active.seq,
                            offset: self.active.offset + self.frame_buf.len() as u64,
                            lsn: w.lsn,
                            len: w.body.len() as u32,
                        },
                    ));
                }
                format::encode_frame(&mut self.frame_buf, w.lsn, w.view, w.kind, &w.body);
                durable_in_batch |= w.class == Class::Durable;
                apply_write_to_active(&mut self.active, w);
            }

            if let Err(err) = self
                .active
                .file
                .write_all_at(self.active.offset, &self.frame_buf)
            {
                tracing::error!(stream = ?self.cfg.stream, %err, "journal: write failed");
                std::process::abort();
            }
            if let Some(index) = &self.index {
                index.lock().extend(written);
            }
            self.active.offset += self.frame_buf.len() as u64;
            // A batch can overshoot `max_batch_bytes`; shrink only well past it, so saturated
            // batches reuse the buffer and one oversized record does not keep its copy resident.
            self.frame_buf.clear();
            if self.frame_buf.capacity() > 2 * self.cfg.max_batch_bytes {
                self.frame_buf.shrink_to(self.cfg.max_batch_bytes);
            }
            // Permits are held by `batch` until here, bounding in-flight data bytes until the write
            // (not the fsync) lands; the semaphore is about memory, not durability.
            drop(batch);

            // Update before publishing durability: `wait_durable` callers must see this batch's
            // views in `SegmentSet` (e.g. via `scan_for`) as soon as it returns.
            let total_bytes = {
                let mut segments = self.segments.lock();
                segments.update_active(self.cfg.stream, self.active.offset, self.active.max_view);
                segments.list(self.cfg.stream).iter().map(|m| m.bytes).sum()
            };
            self.metrics.set_total_bytes(total_bytes);

            if durable_in_batch {
                if let Err(err) = timed_sync(&mut self.active.file, &self.metrics) {
                    tracing::error!(stream = ?self.cfg.stream, %err, "journal: fsync failed");
                    std::process::abort();
                }
                self.durable_tx.send(self.active.last_lsn).ok();
            }

            let over_bytes = self.active.offset.saturating_sub(self.active.snapshot_end)
                >= self.cfg.segment_bytes;
            let over_views = self.cfg.stream == Stream::Wal
                && self
                    .active
                    .max_view
                    .saturating_sub(self.active.first_view.unwrap_or(self.active.max_view))
                    >= self.cfg.max_view_span;
            if over_bytes || over_views {
                self.roll();
            }
        }
    }

    /// Drains any records already sent to the channel into the *old* segment under the state lock
    /// (via `hook`), so replay can require every record's lsn to be exactly one more than the
    /// last, with no channel-full deadlock and no lost acked record on a crash at any point in
    /// this function (every write below is abort-on-error, matching the writer's policy).
    fn roll(&mut self) {
        let stream = self.cfg.stream;
        let max_snapshot_bytes = self.cfg.max_snapshot_bytes;

        let mut drain_buf = Vec::new();
        let mut drained_records = 0usize;
        let mut drained_bytes = 0usize;
        let mut new_first_lsn = self.active.last_lsn + 1;
        let rx = &mut self.rx;
        let active = &mut self.active;
        let alloc = &self.alloc;
        let snapshot = self.hook.as_deref().map(|h| {
            h.snapshot_locked(&mut || {
                drain_buf.clear();
                drained_records = 0;
                drained_bytes = 0;
                while let Ok(w) = rx.try_recv() {
                    format::encode_frame(&mut drain_buf, w.lsn, w.view, w.kind, &w.body);
                    drained_records += 1;
                    drained_bytes += w.body.len();
                    apply_write_to_active(active, &w);
                }
                if !drain_buf.is_empty() {
                    if let Err(err) = active.file.write_all_at(active.offset, &drain_buf) {
                        tracing::error!(?stream, %err, "journal: roll drain write failed");
                        std::process::abort();
                    }
                    active.offset += drain_buf.len() as u64;
                }
                // Reserve the snapshot's lsn from the same counter `Lane::enqueue` uses: the state
                // lock held by `snapshot_locked` blocks any wal put from allocating concurrently.
                let mut next = alloc.lock();
                new_first_lsn = *next;
                *next += 1;
            })
        });
        if drained_records > 0 {
            self.metrics.record_batch(drained_records, drained_bytes);
            self.queue_bytes
                .fetch_sub(drained_bytes as u64, Ordering::Relaxed);
            self.metrics
                .set_queue_bytes(self.queue_bytes.load(Ordering::Relaxed));
        }

        if let Err(err) = timed_sync(&mut active.file, &self.metrics) {
            tracing::error!(?stream, %err, "journal: roll fsync of old segment failed");
            std::process::abort();
        }
        self.segments
            .lock()
            .update_active(stream, active.offset, active.max_view);
        self.durable_tx.send(active.last_lsn).ok();

        let new_seq = active.seq + 1;
        let mut new_active = match open_new_segment(
            &*self.fs,
            &self.dir,
            stream,
            new_seq,
            new_first_lsn,
            active.max_view,
            &self.metrics,
        ) {
            Ok(seg) => seg,
            Err(err) => {
                tracing::error!(?stream, %err, "journal: creating rolled segment failed");
                std::process::abort();
            },
        };

        // Register the new segment before any `durable_tx` send that could ack a write into it.
        self.segments.lock().push_active(SegmentMeta {
            stream,
            seq: new_seq,
            bytes: new_active.offset,
            max_view: new_active.max_view,
        });

        if let Some(body) = snapshot {
            check_snapshot_size(stream, &body, max_snapshot_bytes);
            let mut frame = Vec::new();
            format::encode_frame(&mut frame, new_first_lsn, 0, Kind::Snapshot, &body);
            if new_active
                .file
                .write_all_at(new_active.offset, &frame)
                .is_err()
                || timed_sync(&mut new_active.file, &self.metrics).is_err()
            {
                tracing::error!(?stream, "journal: writing rolled snapshot failed");
                std::process::abort();
            }
            new_active.offset += frame.len() as u64;
            new_active.last_lsn = new_first_lsn;
            new_active.snapshot_end = new_active.offset;
            self.segments
                .lock()
                .update_active(stream, new_active.offset, new_active.max_view);
            self.durable_tx.send(new_active.last_lsn).ok();
        }

        let total_bytes = self
            .segments
            .lock()
            .list(stream)
            .iter()
            .map(|m| m.bytes)
            .sum();
        self.metrics.set_total_bytes(total_bytes);

        self.active = new_active;
    }
}

#[cfg(test)]
pub mod mem {
    use std::{
        collections::{HashMap, HashSet},
        sync::{Arc, Mutex},
    };

    use super::{JournalFile, JournalFs};

    #[derive(Default)]
    struct FileState {
        synced: Vec<u8>,
        unsynced: Vec<u8>,
    }

    impl FileState {
        fn len(&self) -> usize {
            self.synced.len().max(self.unsynced.len())
        }

        fn write_at(&mut self, off: usize, buf: &[u8]) {
            let end = off + buf.len();
            if self.unsynced.len() < end {
                self.unsynced.resize(end, 0);
            }
            self.unsynced[off..end].copy_from_slice(buf);
        }
    }

    /// In-memory `JournalFs` with a crash model: `crash()` keeps every fsynced byte range and
    /// randomly truncates the unsynced tail of each file, simulating a power loss mid-write.
    #[derive(Clone, Default)]
    pub struct MemFs {
        files: Arc<Mutex<HashMap<std::path::PathBuf, FileState>>>,
        pub fail_next_write: Arc<std::sync::atomic::AtomicBool>,
        /// Paths on which `remove` fails once (removed from the set on the failing call).
        pub fail_remove: Arc<Mutex<HashSet<std::path::PathBuf>>>,
    }

    pub struct MemFile {
        path: std::path::PathBuf,
        files: Arc<Mutex<HashMap<std::path::PathBuf, FileState>>>,
        fail_next_write: Arc<std::sync::atomic::AtomicBool>,
    }

    impl MemFs {
        pub fn crash(&self, mut rng: impl FnMut(usize) -> usize) {
            let mut files = self.files.lock().unwrap();
            for state in files.values_mut() {
                let extra = rng(state.unsynced.len().saturating_sub(state.synced.len()) + 1);
                let keep = state.synced.len()
                    + extra.min(state.unsynced.len().saturating_sub(state.synced.len()));
                state.unsynced.truncate(keep.max(state.synced.len()));
                state.synced = state.unsynced.clone();
            }
        }

        pub fn file_len(&self, path: &std::path::Path) -> usize {
            self.files
                .lock()
                .unwrap()
                .get(path)
                .map_or(0, FileState::len)
        }
    }

    impl JournalFs for MemFs {
        type File = MemFile;

        fn create(&self, path: &std::path::Path) -> std::io::Result<Self::File> {
            let mut files = self.files.lock().unwrap();
            if files.contains_key(path) {
                return Err(std::io::ErrorKind::AlreadyExists.into());
            }
            files.insert(path.to_path_buf(), FileState::default());
            Ok(MemFile {
                path: path.to_path_buf(),
                files: self.files.clone(),
                fail_next_write: self.fail_next_write.clone(),
            })
        }

        fn open_write(&self, path: &std::path::Path) -> std::io::Result<Self::File> {
            if !self.files.lock().unwrap().contains_key(path) {
                return Err(std::io::ErrorKind::NotFound.into());
            }
            Ok(MemFile {
                path: path.to_path_buf(),
                files: self.files.clone(),
                fail_next_write: self.fail_next_write.clone(),
            })
        }

        fn open_read(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>> {
            self.files
                .lock()
                .unwrap()
                .get(path)
                .map(|s| s.synced.clone())
                .ok_or_else(|| std::io::ErrorKind::NotFound.into())
        }

        fn open_read_stream(
            &self,
            path: &std::path::Path,
        ) -> std::io::Result<Box<dyn std::io::Read>> {
            Ok(Box::new(std::io::Cursor::new(self.open_read(path)?)))
        }

        fn read_header(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>> {
            let bytes = self.open_read(path)?;
            if bytes.len() < super::SegmentHeader::LEN {
                return Err(std::io::ErrorKind::UnexpectedEof.into());
            }
            Ok(bytes[..super::SegmentHeader::LEN].to_vec())
        }

        fn len(&self, path: &std::path::Path) -> std::io::Result<u64> {
            self.open_read(path).map(|b| b.len() as u64)
        }

        fn list(&self, dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
            Ok(self
                .files
                .lock()
                .unwrap()
                .keys()
                .filter(|p| p.parent() == Some(dir))
                .cloned()
                .collect())
        }

        fn remove(&self, path: &std::path::Path) -> std::io::Result<()> {
            if self.fail_remove.lock().unwrap().remove(path) {
                return Err(std::io::ErrorKind::Other.into());
            }
            self.files.lock().unwrap().remove(path);
            Ok(())
        }

        fn sync_dir(&self, _dir: &std::path::Path) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl JournalFile for MemFile {
        fn write_all_at(&mut self, off: u64, buf: &[u8]) -> std::io::Result<()> {
            if self
                .fail_next_write
                .swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                return Err(std::io::ErrorKind::Other.into());
            }
            let mut files = self.files.lock().unwrap();
            let state = files.get_mut(&self.path).expect("file exists");
            state.write_at(off as usize, buf);
            Ok(())
        }

        fn sync_data(&mut self) -> std::io::Result<()> {
            let mut files = self.files.lock().unwrap();
            let state = files.get_mut(&self.path).expect("file exists");
            state.synced = state.unsynced.clone();
            Ok(())
        }

        fn set_len(&mut self, len: u64) -> std::io::Result<()> {
            let mut files = self.files.lock().unwrap();
            let state = files.get_mut(&self.path).expect("file exists");
            state.unsynced.truncate(len as usize);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{mem::MemFs, *};

    fn meta(stream: Stream, seq: u64, bytes: u64, max_view: u64) -> SegmentMeta {
        SegmentMeta {
            stream,
            seq,
            bytes,
            max_view,
        }
    }

    #[test]
    fn to_unlink_keeps_two_newest_wal_segments() {
        let set = SegmentSet {
            wal: vec![
                meta(Stream::Wal, 1, 100, 5),
                meta(Stream::Wal, 2, 100, 10),
                meta(Stream::Wal, 3, 100, 20),
                meta(Stream::Wal, 4, 100, 30),
            ],
            data: vec![],
        };
        // decided=100, retention=50 -> bound=50: segments 1,2 are old enough, but only those
        // outside the two newest (3, 4) are candidates.
        let unlink = set.to_unlink(100, 50, u64::MAX, None);
        assert_eq!(unlink.iter().map(|m| m.seq).collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn to_unlink_never_touches_active_or_recent_wal() {
        let set = SegmentSet {
            wal: vec![meta(Stream::Wal, 1, 100, 0), meta(Stream::Wal, 2, 100, 0)],
            data: vec![],
        };
        assert!(set.to_unlink(1_000_000, 0, u64::MAX, None).is_empty());
    }

    // A zero-view segment (e.g. rolled right after a snapshot, before any real record landed)
    // must never make `to_unlink` skip over an older, not-yet-old-enough segment: `max_view` is
    // supposed to be monotonic (`open_new_segment` seeds it from the previous segment's bound),
    // but this asserts `to_unlink` itself never produces a gap even if that invariant is somehow
    // violated.
    #[test]
    fn to_unlink_wal_never_skips_a_segment_to_reach_a_younger_looking_one() {
        let set = SegmentSet {
            wal: vec![
                meta(Stream::Wal, 1, 100, 60), // not old enough: bound is 50
                meta(Stream::Wal, 2, 100, 0),  // looks old, but sits after segment 1
                meta(Stream::Wal, 3, 100, 80),
                meta(Stream::Wal, 4, 100, 90), // active
            ],
            data: vec![],
        };
        assert!(set.to_unlink(100, 50, u64::MAX, None).is_empty());
    }

    #[test]
    fn to_unlink_data_drops_oldest_first_over_byte_cap() {
        let set = SegmentSet {
            wal: vec![],
            data: vec![
                meta(Stream::Data, 1, 40, 100),
                meta(Stream::Data, 2, 40, 100),
                meta(Stream::Data, 3, 40, 100),
                meta(Stream::Data, 4, 40, 100), // active, never dropped
            ],
        };
        // All young (max_view 100 >= bound 0), total 160 > cap 90: drop oldest until <= cap.
        let unlink = set.to_unlink(0, 0, 90, None);
        assert_eq!(unlink.iter().map(|m| m.seq).collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn to_unlink_data_age_based_ignores_byte_cap() {
        let set = SegmentSet {
            wal: vec![],
            data: vec![
                meta(Stream::Data, 1, 10, 0),
                meta(Stream::Data, 2, 10, 1000),
                meta(Stream::Data, 3, 10, 1000), // active
            ],
        };
        let unlink = set.to_unlink(1000, 10, u64::MAX, None);
        assert_eq!(unlink.iter().map(|m| m.seq).collect::<Vec<_>>(), vec![1]);
    }

    fn cfg(stream: Stream) -> LaneConfig {
        LaneConfig {
            stream,
            segment_bytes: 10_000,
            max_view_span: 10_000,
            max_batch_bytes: 4096,
            max_snapshot_bytes: 1024,
            in_flight_bytes: 1 << 20,
        }
    }

    #[tokio::test]
    async fn recover_empty_dir() {
        let fs = Arc::new(MemFs::default());
        let dir = std::path::PathBuf::from("/wal");
        let recovered = recover(&*fs, &dir, Stream::Wal).unwrap();
        assert!(recovered.segments.is_empty());
        assert_eq!(recovered.next_seq, 1);
        assert_eq!(recovered.next_lsn, 1);
    }

    #[tokio::test]
    async fn durable_ack_survives_crash() {
        let fs = Arc::new(MemFs::default());
        let dir = std::path::PathBuf::from("/data");
        let segments = Arc::new(Mutex::new(SegmentSet::default()));
        let (lane, _handle) = spawn_lane(
            fs.clone(),
            dir.clone(),
            cfg(Stream::Data),
            (1, 1),
            None,
            segments,
            None,
        )
        .unwrap();

        let lsn = lane.enqueue(1, Kind::Vid, b"hello".to_vec(), Class::Durable, None);
        lane.wait_durable(lsn).await.unwrap();

        // Simulate a crash: nothing unsynced should exist, since the ack implies fsync happened.
        fs.crash(|_| 0);
        let recovered = recover(&*fs, &dir, Stream::Data).unwrap();
        assert_eq!(recovered.next_lsn, 2);
    }

    // Regression test: `SegmentSet` must reflect an acked write's view before `durable_tx` publishes
    // it, or a `scan_for` racing right after `wait_durable` returns can pick stale segment bounds and
    // miss the record. Two independent threads (writer thread vs. this task's tokio runtime) racing
    // on the watch channel, so many iterations to give a wrong order a chance to show up.
    #[tokio::test]
    async fn durable_ack_makes_segment_set_reflect_the_acked_view_immediately() {
        let fs = Arc::new(MemFs::default());
        let dir = std::path::PathBuf::from("/data-order");
        let segments = Arc::new(Mutex::new(SegmentSet::default()));
        let (lane, _handle) = spawn_lane(
            fs,
            dir,
            cfg(Stream::Data),
            (1, 1),
            None,
            segments.clone(),
            None,
        )
        .unwrap();

        for view in 1..=2000u64 {
            let lsn = lane.enqueue(view, Kind::Vid, vec![0u8; 4], Class::Durable, None);
            lane.wait_durable(lsn).await.unwrap();
            let max_view = segments
                .lock()
                .list(Stream::Data)
                .last()
                .map(|m| m.max_view)
                .unwrap_or(0);
            assert!(
                max_view >= view,
                "SegmentSet max_view {max_view} lagged the just-acked view {view}"
            );
        }
    }

    #[tokio::test]
    async fn reserve_over_budget_takes_whole_budget() {
        let fs = Arc::new(MemFs::default());
        let segments = Arc::new(Mutex::new(SegmentSet::default()));
        let (lane, _handle) = spawn_lane(
            fs,
            PathBuf::from("/data-reserve"),
            cfg(Stream::Data),
            (1, 1),
            None,
            segments,
            None,
        )
        .unwrap();

        let permit = tokio::time::timeout(Duration::from_secs(5), lane.reserve(2 << 20))
            .await
            .expect("reserve over the 1 MiB budget must not block");
        assert_eq!(permit.num_permits(), 1 << 20);
    }

    #[tokio::test]
    async fn empty_dir_then_write_then_recover_roundtrips() {
        let fs = Arc::new(MemFs::default());
        let dir = std::path::PathBuf::from("/data2");
        let segments = Arc::new(Mutex::new(SegmentSet::default()));
        let (lane, _handle) = spawn_lane(
            fs.clone(),
            dir.clone(),
            cfg(Stream::Data),
            (1, 1),
            None,
            segments,
            None,
        )
        .unwrap();
        for i in 0..5u64 {
            let lsn = lane.enqueue(i, Kind::Vid, vec![i as u8; 4], Class::Durable, None);
            lane.wait_durable(lsn).await.unwrap();
        }
        let recovered = recover(&*fs, &dir, Stream::Data).unwrap();
        assert_eq!(recovered.next_lsn, 6);
        assert_eq!(recovered.segments.len(), 1);
    }

    /// Snapshots a plain counter under its own lock, mimicking how `journal.rs`'s `put_wal` holds
    /// the state lock across both `State::apply` and `Lane::enqueue`.
    struct CounterHook(Arc<Mutex<u64>>);

    impl SnapshotHook for CounterHook {
        fn snapshot_locked(&self, drain: &mut dyn FnMut()) -> Vec<u8> {
            let n = self.0.lock();
            drain();
            n.to_le_bytes().to_vec()
        }
    }

    // Regression test for the roll bug: a wal roll's snapshot frame must reserve its lsn from the
    // same counter `Lane::enqueue` uses. A tiny `segment_bytes` forces many rolls across the loop
    // below; with the bug, the roll's snapshot silently reused the next `enqueue`'s lsn, `scan`
    // then rejected that duplicate as a torn tail, and `next_lsn` ended up far short of the last
    // acked record.
    #[tokio::test]
    async fn wal_roll_reserves_snapshot_lsn_so_recovery_reaches_the_last_acked_record() {
        let fs = Arc::new(MemFs::default());
        let dir = std::path::PathBuf::from("/wal3");
        let segments = Arc::new(Mutex::new(SegmentSet::default()));
        let state = Arc::new(Mutex::new(0u64));
        let hook: Arc<dyn SnapshotHook> = Arc::new(CounterHook(state.clone()));

        let mut wal_cfg = cfg(Stream::Wal);
        wal_cfg.segment_bytes = 200;

        let (lane, _handle) = spawn_lane(
            fs.clone(),
            dir.clone(),
            wal_cfg,
            (1, 1),
            Some(hook),
            segments,
            None,
        )
        .unwrap();

        for view in 1..=40u64 {
            // Mirrors `put_wal`: mutate "state" and enqueue while holding the same lock the hook
            // locks, so alloc's counter and the hook's drain always agree on what's been sent.
            let lsn = {
                let mut n = state.lock();
                *n += 1;
                lane.enqueue(
                    view,
                    Kind::Action,
                    view.to_le_bytes().to_vec(),
                    Class::Durable,
                    None,
                )
            };
            lane.wait_durable(lsn).await.unwrap();
        }

        fs.crash(|_| 0);
        let recovered = recover(&*fs, &dir, Stream::Wal).unwrap();
        assert_eq!(
            recovered.wal_records.first().map(|(h, _)| h.kind),
            Some(Kind::Snapshot)
        );
        // `scan` already enforces lsn-consecutiveness; the count check is a sanity check on top.
        assert_eq!(
            recovered.wal_records.len() as u64,
            recovered.next_lsn - recovered.wal_records[0].0.lsn
        );
        // Snapshot counter plus the records after it must account for every acked write.
        let (_, snap) = &recovered.wal_records[0];
        let snap_count = u64::from_le_bytes(snap[..8].try_into().unwrap());
        assert_eq!(snap_count + recovered.wal_records.len() as u64 - 1, 40);
    }

    // Regression test: a failed unlink must not drop its segment from `SegmentSet` before the file
    // is actually gone, or a later pass moves on to younger segments and leaves a gap on disk that
    // `recover` refuses to start over.
    #[tokio::test]
    async fn prune_stops_at_a_failed_unlink_instead_of_skipping_ahead() {
        let fs = MemFs::default();
        let wal_dir = std::path::PathBuf::from("/gc-wal");
        let data_dir = std::path::PathBuf::from("/gc-data");

        // 6 data segments, all old enough to qualify except the last (active, never unlinked).
        let mut segments = SegmentSet::default();
        for seq in 1..=6u64 {
            write_segment(
                &fs,
                &data_dir,
                SegmentHeader {
                    stream: Stream::Data,
                    seq,
                    first_lsn: seq,
                    prev_max_view: 100,
                },
                &[],
            );
            segments
                .data
                .push(meta(Stream::Data, seq, SegmentHeader::LEN as u64, 100));
        }
        let segments = Mutex::new(segments);

        fs.fail_remove
            .lock()
            .unwrap()
            .insert(segment_path(&data_dir, 2));

        let err = prune(&fs, &segments, &wal_dir, &data_dir, 1000, 0, u64::MAX, None).unwrap_err();
        assert!(format!("{err:#}").contains("segment 2"), "{err:#}");

        assert_eq!(fs.list(&data_dir).unwrap().len(), 5, "only seq 1 unlinked");
        assert!(fs.open_read(&segment_path(&data_dir, 1)).is_err());
        for seq in 2..=6 {
            assert!(
                fs.open_read(&segment_path(&data_dir, seq)).is_ok(),
                "seq {seq} must survive a failure unlinking an older segment"
            );
        }
        assert_eq!(
            segments
                .lock()
                .data
                .iter()
                .map(|m| m.seq)
                .collect::<Vec<_>>(),
            vec![2, 3, 4, 5, 6],
            "seq 2 must stay tracked: its unlink failed"
        );

        // Second pass, fault cleared (single-shot): must retry seq 2 first, in order.
        let stats = prune(&fs, &segments, &wal_dir, &data_dir, 1000, 0, u64::MAX, None).unwrap();
        assert_eq!(stats.data_bytes, Some(SegmentHeader::LEN as u64));
        assert_eq!(
            segments
                .lock()
                .data
                .iter()
                .map(|m| m.seq)
                .collect::<Vec<_>>(),
            vec![6]
        );
        assert_eq!(fs.list(&data_dir).unwrap().len(), 1);

        recover(&fs, &data_dir, Stream::Data).expect("no sequence gap left behind");
    }

    fn write_segment(fs: &MemFs, dir: &Path, header: SegmentHeader, frames: &[u8]) {
        let mut file = fs.create(&segment_path(dir, header.seq)).unwrap();
        file.write_all_at(0, &header.encode()).unwrap();
        file.write_all_at(SegmentHeader::LEN as u64, frames)
            .unwrap();
        file.sync_data().unwrap();
    }

    // EDGE:lane-snapshot-torn: a crash between creating a rolled segment's header and writing its
    // snapshot leaves the newest segment with a valid header but no records. Recovery must drop
    // it and fall back to the previous segment's snapshot, losing nothing acked.
    #[tokio::test]
    async fn recover_falls_back_when_newest_wal_segment_has_no_snapshot() {
        let fs = MemFs::default();
        let dir = std::path::PathBuf::from("/wal4");

        let mut seg1 = Vec::new();
        format::encode_frame(&mut seg1, 1, 0, Kind::Snapshot, b"snap");
        format::encode_frame(&mut seg1, 2, 7, Kind::Action, b"a");
        write_segment(
            &fs,
            &dir,
            SegmentHeader {
                stream: Stream::Wal,
                seq: 1,
                first_lsn: 1,
                prev_max_view: 0,
            },
            &seg1,
        );
        // seg2: header only, as if the process crashed before the rolled snapshot was written.
        write_segment(
            &fs,
            &dir,
            SegmentHeader {
                stream: Stream::Wal,
                seq: 2,
                first_lsn: 3,
                prev_max_view: 7,
            },
            &[],
        );

        let recovered = recover(&fs, &dir, Stream::Wal).unwrap();
        assert_eq!(
            recovered
                .wal_records
                .iter()
                .map(|(h, _)| h.lsn)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(recovered.next_lsn, 3);
        assert_eq!(recovered.next_seq, 2);
        assert!(
            fs.list(&dir).unwrap().len() == 1,
            "the torn seg2 was unlinked"
        );
    }

    // EDGE:lane-snapshot-torn, crash after the snapshot sync: the rolled segment's snapshot landed
    // and synced, but the next real record was torn. That segment is valid (starts with
    // `Snapshot`) and must be kept and truncated, not dropped.
    #[tokio::test]
    async fn recover_truncates_torn_tail_after_a_valid_snapshot() {
        let fs = MemFs::default();
        let dir = std::path::PathBuf::from("/wal5");

        let mut seg = Vec::new();
        format::encode_frame(&mut seg, 1, 0, Kind::Snapshot, b"snap");
        let clean_end = seg.len();
        format::encode_frame(&mut seg, 2, 7, Kind::Action, b"torn-record");
        seg.truncate(seg.len() - 3); // torn tail: the second frame's crc no longer matches.

        write_segment(
            &fs,
            &dir,
            SegmentHeader {
                stream: Stream::Wal,
                seq: 1,
                first_lsn: 1,
                prev_max_view: 0,
            },
            &seg,
        );

        let recovered = recover(&fs, &dir, Stream::Wal).unwrap();
        assert_eq!(
            recovered.wal_records.first().map(|(h, _)| h.kind),
            Some(Kind::Snapshot)
        );
        assert_eq!(recovered.wal_records.len(), 1);
        assert_eq!(recovered.next_lsn, 2);
        assert_eq!(
            fs.file_len(&segment_path(&dir, 1)),
            SegmentHeader::LEN + clean_end,
            "the torn second frame must be truncated off, not just ignored"
        );
    }

    // Regression test: a torn frame header whose length exceeds the file must end the scan
    // before the body buffer is sized from it.
    #[tokio::test]
    async fn recover_truncates_torn_frame_length_past_file_end() {
        let fs = MemFs::default();
        let header = |seq| SegmentHeader {
            stream: Stream::Wal,
            seq,
            first_lsn: 1,
            prev_max_view: 0,
        };

        let mut seg = Vec::new();
        format::encode_frame(&mut seg, 1, 0, Kind::Snapshot, b"snap");
        let clean_end = seg.len();
        format::encode_frame(&mut seg, 2, 7, Kind::Action, b"x");

        // A final frame whose length exactly fills the file is valid.
        let exact = std::path::PathBuf::from("/wal6-exact");
        write_segment(&fs, &exact, header(1), &seg);
        let recovered = recover(&fs, &exact, Stream::Wal).unwrap();
        assert_eq!(recovered.wal_records.len(), 2);
        assert_eq!(recovered.next_lsn, 3);

        // One byte past the file end is torn.
        let remaining = (seg.len() - clean_end - format::FRAME_HEADER_LEN) as u32;
        seg[clean_end + 4..clean_end + 8].copy_from_slice(&(remaining + 1).to_le_bytes());
        let dir = std::path::PathBuf::from("/wal6");
        write_segment(&fs, &dir, header(1), &seg);

        let recovered = recover(&fs, &dir, Stream::Wal).unwrap();
        assert_eq!(recovered.wal_records.len(), 1);
        assert_eq!(recovered.next_lsn, 2);
        assert_eq!(
            fs.file_len(&segment_path(&dir, 1)),
            SegmentHeader::LEN + clean_end
        );
    }
}
