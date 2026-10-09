//! Block payloads and VID shares outside SQL, in two append-only `journal-lane` lanes.
//!
//! `<dir>/payload` holds one `Durable` record per height. The caller keeps the returned `BlobLoc`
//! (in SQL) and unlinks whole segments once their heights are pruned. `<dir>/share` holds one
//! `Enqueue` record per height, synced every second and found through an in-memory index that
//! `open` rebuilds. Shares are unlinked by segment age. Writer I/O errors abort the process, like
//! the consensus journal lanes.

use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Weak},
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Context, ensure};
use hotshot_types::traits::metrics::{Counter, Metrics, NoMetrics};
use journal_lane::{
    format::{Class, Kind, SegmentHeader, Stream},
    lane::{
        self, DataIndex, JournalFs, Lane, LaneConfig, LaneMetrics, LaneMode, Location, Recovered,
        SegmentMeta, Segments, StdFs, segment_path,
    },
};
use parking_lot::Mutex;
use tokio::time::MissedTickBehavior;

const PAYLOAD: Kind = Kind(1);
const SHARE: Kind = Kind(2);
const SEGMENT_BYTES: u64 = 1 << 30;
const MAX_BATCH_BYTES: usize = 64 << 20;
const IN_FLIGHT_BYTES: usize = 128 << 20;
const SHARE_SYNC_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Debug)]
pub struct BlobCfg {
    pub dir: PathBuf,
    pub share_retention: Duration,
}

/// Where a payload record lives. The text form `{seq:x}:{offset:x}:{len:x}:{lsn:x}` goes into SQL;
/// `lsn` lets reads check that the frame at `offset` is the one that was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlobLoc {
    pub seq: u64,
    pub offset: u64,
    pub len: u32,
    pub lsn: u64,
}

impl fmt::Display for BlobLoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            seq,
            offset,
            len,
            lsn,
        } = self;
        write!(f, "{seq:x}:{offset:x}:{len:x}:{lsn:x}")
    }
}

impl FromStr for BlobLoc {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let loc = Self::parse(s).with_context(|| format!("invalid blob locator {s:?}"))?;
        // Rejects forms `from_str_radix` accepts: a leading `+`, uppercase digits, leading zeros.
        ensure!(loc.to_string() == s, "non-canonical blob locator {s:?}");
        Ok(loc)
    }
}

impl BlobLoc {
    fn parse(s: &str) -> Option<Self> {
        let mut fields = s.split(':').map(|f| u64::from_str_radix(f, 16).ok());
        let loc = Self {
            seq: fields.next()??,
            offset: fields.next()??,
            len: u32::try_from(fields.next()??).ok()?,
            lsn: fields.next()??,
        };
        fields.next().is_none().then_some(loc)
    }
}

impl From<BlobLoc> for Location {
    fn from(loc: BlobLoc) -> Self {
        Self {
            seq: loc.seq,
            offset: loc.offset,
            lsn: loc.lsn,
            len: loc.len,
        }
    }
}

impl From<Location> for BlobLoc {
    fn from(loc: Location) -> Self {
        Self {
            seq: loc.seq,
            offset: loc.offset,
            len: loc.len,
            lsn: loc.lsn,
        }
    }
}

/// Locators of payloads written but not yet committed to SQL, per height.
pub type StagedBlobs = Vec<(u64, BlobLoc)>;

pub struct BlobStore<F: JournalFs = StdFs> {
    fs: Arc<F>,
    share_retention: Duration,
    payload: Arc<BlobLane>,
    share: Arc<BlobLane>,
    /// Concurrent `append_payload` calls per height; the last one out removes the index entry.
    appending: Mutex<HashMap<u64, u32>>,
    /// Payload records a reader needed and could not read.
    missing: Mutex<Box<dyn Counter>>,
    /// Payloads appended inside a write transaction because the caller did not stage them.
    staged_in_tx: Mutex<Box<dyn Counter>>,
    // Field order matters: the writer threads exit once the lanes above are dropped, and the
    // directory stays locked until they have.
    _threads: JoinOnDrop,
    _lock: File,
}

impl<F: JournalFs> fmt::Debug for BlobStore<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlobStore").finish_non_exhaustive()
    }
}

impl<F: JournalFs> BlobStore<F> {
    /// Takes the directory lock, recovers both lanes, rebuilds the share index and starts the
    /// writers and the share sync task. A corrupt segment is dropped with every segment after it.
    pub async fn open(cfg: BlobCfg, fs: Arc<F>) -> anyhow::Result<Arc<Self>> {
        Self::open_with(cfg, fs, SEGMENT_BYTES).await
    }

    pub fn install_metrics(&self, metrics: &dyn Metrics) {
        LaneMetrics::install(metrics, "blob", &[&self.payload.lane, &self.share.lane]);
        for lane in [&self.payload, &self.share] {
            lane.lane.set_total_bytes(lane.segments.lock().bytes());
        }
        *self.missing.lock() = metrics.create_counter("blob_missing".into(), None);
        *self.staged_in_tx.lock() = metrics.create_counter("blob_staged_in_tx".into(), None);
    }

    /// `append_payload` for a caller inside a write transaction, which should have staged.
    pub async fn append_payload_in_tx(
        &self,
        height: u64,
        body: Vec<u8>,
    ) -> anyhow::Result<BlobLoc> {
        self.staged_in_tx.lock().add(1);
        self.append_payload(height, body).await
    }

    /// Returns once the record is fsynced, so a committed locator never points at lost bytes.
    pub async fn append_payload(&self, height: u64, body: Vec<u8>) -> anyhow::Result<BlobLoc> {
        let len = u32::try_from(body.len()).context("payload over 4 GiB")?;
        let lane = &self.payload;
        let permit = lane.lane.reserve(len).await;
        let _appending = Appending::enter(&self.appending, &lane.index, height);

        let lsn = lane
            .lane
            .enqueue(height, PAYLOAD, body, Class::Durable, Some(permit));
        lane.lane.wait_durable(lsn).await?;
        let loc = lane
            .locate(height, PAYLOAD)
            .context("payload index entry missing after durable write")?;
        // A concurrent store of this height wrote a newer record; both are valid, use the newest.
        if loc.lsn > lsn {
            lane.lane.wait_durable(loc.lsn).await?;
        }
        Ok(loc.into())
    }

    /// Queues the record; the share sync task makes it durable within a second.
    pub fn append_share(&self, height: u64, body: Vec<u8>) {
        self.share
            .lane
            .enqueue(height, SHARE, body, Class::Enqueue, None);
    }

    /// `None` when the record is gone: unlinked segment, short read or bad checksum.
    pub async fn read_payload(&self, loc: BlobLoc) -> anyhow::Result<Option<Vec<u8>>> {
        let bytes = self.read(&self.payload, loc.into()).await?;
        if bytes.is_none() {
            self.missing.lock().add(1);
        }
        Ok(bytes)
    }

    pub async fn read_share(&self, height: u64) -> anyhow::Result<Option<Vec<u8>>> {
        match self.share.locate(height, SHARE) {
            Some(loc) => self.read(&self.share, loc).await,
            None => Ok(None),
        }
    }

    /// Unlinks sealed payload segments whose heights are all at or below `height`, oldest first.
    /// Returns the bytes freed.
    pub async fn gc_payload_below(&self, height: u64) -> anyhow::Result<u64> {
        let lane = self.payload.clone();
        let fs = self.fs.clone();
        tokio::task::spawn_blocking(move || {
            lane.unlink(&*fs, |segments| {
                Ok(segments.oldest_below(height.saturating_add(1)))
            })
        })
        .await
        .context("payload gc task panicked")?
    }

    /// Unlinks sealed share segments last written at least `share_retention` before `now`, oldest
    /// first. Returns the bytes freed.
    pub async fn gc_shares_older_than(&self, now: SystemTime) -> anyhow::Result<u64> {
        let lane = self.share.clone();
        let fs = self.fs.clone();
        let retention = self.share_retention;
        tokio::task::spawn_blocking(move || {
            let fs = &*fs;
            lane.unlink(fs, |segments| {
                expired(fs, &lane.dir, segments, now, retention)
            })
        })
        .await
        .context("share gc task panicked")?
    }

    pub fn payload_bytes(&self) -> u64 {
        self.payload.segments.lock().bytes()
    }

    pub fn share_bytes(&self) -> u64 {
        self.share.segments.lock().bytes()
    }

    async fn open_with(cfg: BlobCfg, fs: Arc<F>, segment_bytes: u64) -> anyhow::Result<Arc<Self>> {
        let payload_dir = cfg.dir.join("payload");
        let share_dir = cfg.dir.join("share");
        let lock = lock_dir(&*fs, &cfg.dir, &[&payload_dir, &share_dir])?;

        let (payload, share) = tokio::try_join!(
            open_lane(fs.clone(), payload_dir, Stream::Payload, segment_bytes),
            open_lane(fs.clone(), share_dir, Stream::Share, segment_bytes),
        )?;
        let (payload, payload_thread) = payload;
        let (share, share_thread) = share;
        let store = Arc::new(Self {
            fs,
            share_retention: cfg.share_retention,
            payload: Arc::new(payload),
            share: Arc::new(share),
            appending: Mutex::default(),
            missing: Mutex::new(NoMetrics.create_counter(String::new(), None)),
            staged_in_tx: Mutex::new(NoMetrics.create_counter(String::new(), None)),
            _threads: JoinOnDrop(vec![payload_thread, share_thread]),
            _lock: lock,
        });
        spawn_share_sync(Arc::downgrade(&store.share));
        Ok(store)
    }

    async fn read(&self, lane: &BlobLane, loc: Location) -> anyhow::Result<Option<Vec<u8>>> {
        let fs = self.fs.clone();
        let path = segment_path(&lane.dir, loc.seq);
        tokio::task::spawn_blocking(move || lane::read_frame(&*fs, &path, loc))
            .await
            .context("blob read task panicked")?
    }
}

struct BlobLane {
    dir: PathBuf,
    lane: Lane,
    segments: Arc<Mutex<Segments>>,
    /// Payload lane: records between write and the end of `append_payload`. Share lane: the newest
    /// record per height, for as long as its segment exists.
    index: DataIndex,
    gc_lock: Mutex<()>,
}

impl BlobLane {
    fn locate(&self, height: u64, kind: Kind) -> Option<Location> {
        self.index.lock().get(&(height, kind)).copied()
    }

    /// Unlinks the segments `select` picks and returns the bytes freed.
    fn unlink<F: JournalFs>(
        &self,
        fs: &F,
        select: impl FnOnce(&Segments) -> anyhow::Result<Vec<SegmentMeta>>,
    ) -> anyhow::Result<u64> {
        let _gc = self.gc_lock.lock();
        let doomed = select(&self.segments.lock())?;
        let freed = doomed.iter().map(|meta| meta.bytes).sum();
        if let Some(bytes_left) = lane::prune(fs, &self.dir, &self.segments, &doomed)? {
            self.lane.set_total_bytes(bytes_left);
            let oldest = self.segments.lock().list().first().map(|meta| meta.seq);
            if let Some(oldest) = oldest {
                self.index.lock().retain(|_, loc| loc.seq >= oldest);
            }
        }
        Ok(freed)
    }
}

/// Keeps the payload index entry of `height` alive while any `append_payload` for it is running,
/// including one whose future is dropped midway.
struct Appending<'a> {
    counts: &'a Mutex<HashMap<u64, u32>>,
    index: &'a DataIndex,
    height: u64,
}

impl<'a> Appending<'a> {
    fn enter(counts: &'a Mutex<HashMap<u64, u32>>, index: &'a DataIndex, height: u64) -> Self {
        *counts.lock().entry(height).or_default() += 1;
        Self {
            counts,
            index,
            height,
        }
    }
}

impl Drop for Appending<'_> {
    fn drop(&mut self) {
        let mut counts = self.counts.lock();
        let count = counts
            .get_mut(&self.height)
            .expect("entered in `Appending::enter`");
        *count -= 1;
        if *count == 0 {
            counts.remove(&self.height);
            self.index.lock().remove(&(self.height, PAYLOAD));
        }
    }
}

struct JoinOnDrop(Vec<JoinHandle<()>>);

impl Drop for JoinOnDrop {
    fn drop(&mut self) {
        for thread in self.0.drain(..) {
            thread.join().ok();
        }
    }
}

fn is_payload(kind: u8) -> bool {
    kind == PAYLOAD.0
}

fn is_share(kind: u8) -> bool {
    kind == SHARE.0
}

/// The lane directories must exist on the real filesystem for the lock, whatever `F` is.
fn lock_dir<F: JournalFs>(fs: &F, dir: &Path, lane_dirs: &[&Path]) -> anyhow::Result<File> {
    for lane_dir in lane_dirs {
        std::fs::create_dir_all(lane_dir)
            .with_context(|| format!("creating blob directory {}", lane_dir.display()))?;
    }
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("LOCK"))
        .context("opening blob LOCK file")?;
    lock.try_lock()
        .map_err(io::Error::from)
        .context("blob directory is already in use by another process")?;
    // Directory entries `create_dir_all` just added are not durable until fsynced.
    let parent = dir.parent().filter(|p| !p.as_os_str().is_empty());
    for synced in lane_dirs.iter().copied().chain([dir]).chain(parent) {
        fs.sync_dir(synced)
            .with_context(|| format!("fsyncing {}", synced.display()))?;
    }
    Ok(lock)
}

async fn open_lane<F: JournalFs>(
    fs: Arc<F>,
    dir: PathBuf,
    stream: Stream,
    segment_bytes: u64,
) -> anyhow::Result<(BlobLane, JoinHandle<()>)> {
    tokio::task::spawn_blocking(move || open_lane_blocking(&fs, dir, stream, segment_bytes))
        .await
        .context("blob lane open task panicked")?
        .with_context(|| format!("opening {} lane", stream.label()))
}

fn open_lane_blocking<F: JournalFs>(
    fs: &Arc<F>,
    dir: PathBuf,
    stream: Stream,
    segment_bytes: u64,
) -> anyhow::Result<(BlobLane, JoinHandle<()>)> {
    let started = Instant::now();
    let known_kind: fn(u8) -> bool = match stream {
        Stream::Payload => is_payload,
        Stream::Share => is_share,
        Stream::Wal | Stream::Data => unreachable!("not a blob lane: {stream:?}"),
    };
    let recovered = recover_lane(&**fs, &dir, stream, known_kind)?;

    let index = DataIndex::default();
    if stream == Stream::Share {
        // Oldest first, so the newest record of a height overwrites older ones.
        for meta in &recovered.segments {
            let frames = lane::frame_locations(&**fs, &dir, stream, meta.seq, known_kind)?;
            index.lock().extend(
                frames
                    .into_iter()
                    .map(|(header, loc)| ((header.key, header.kind), loc)),
            );
        }
    }

    let segments = Arc::new(Mutex::new(Segments::from(recovered.segments)));
    let cfg = LaneConfig {
        stream,
        mode: LaneMode::Append,
        known_kind,
        segment_bytes,
        max_key_span: u64::MAX,
        max_batch_bytes: MAX_BATCH_BYTES,
        // No snapshot hook, so no snapshot frame is ever written.
        max_snapshot_bytes: 0,
        in_flight_bytes: IN_FLIGHT_BYTES,
    };
    let (lane, thread) = lane::spawn_lane(
        fs.clone(),
        dir.clone(),
        cfg,
        (recovered.next_seq, recovered.next_lsn),
        None,
        segments.clone(),
        Some(index.clone()),
    )?;

    {
        let segments = segments.lock();
        tracing::info!(
            lane = stream.label(),
            segments = segments.list().len(),
            bytes = segments.bytes(),
            max_key = segments.list().last().map_or(0, |meta| meta.max_key),
            ms = started.elapsed().as_millis() as u64,
            "blob store open"
        );
    }
    let blob_lane = BlobLane {
        dir,
        lane,
        segments,
        index,
        gc_lock: Mutex::new(()),
    };
    Ok((blob_lane, thread))
}

/// Recovers `dir`; if a segment header is unreadable or the sequence has a gap, removes that
/// segment and every later one, then recovers again. Their records read as missing.
fn recover_lane<F: JournalFs>(
    fs: &F,
    dir: &Path,
    stream: Stream,
    known_kind: fn(u8) -> bool,
) -> anyhow::Result<Recovered> {
    let err = match lane::recover(fs, dir, stream, LaneMode::Append, known_kind) {
        Ok(recovered) => return Ok(recovered),
        Err(err) => err,
    };
    let dropped = remove_unreadable_tail(fs, dir, stream)?;
    if dropped == 0 {
        return Err(err);
    }
    tracing::warn!(
        lane = stream.label(),
        dropped,
        "blob store: removed unreadable segments: {err:#}"
    );
    lane::recover(fs, dir, stream, LaneMode::Append, known_kind)
}

/// Removes every segment from the first one with a bad header or a sequence gap on. Returns how
/// many were removed.
fn remove_unreadable_tail<F: JournalFs>(
    fs: &F,
    dir: &Path,
    stream: Stream,
) -> anyhow::Result<usize> {
    let mut segments: Vec<(u64, PathBuf)> = fs
        .list(dir)?
        .into_iter()
        .filter_map(|path| lane::parse_seq(&path).map(|seq| (seq, path)))
        .collect();
    segments.sort_by_key(|(seq, _)| *seq);

    let mut readable = 0;
    for (i, (seq, path)) in segments.iter().enumerate() {
        let contiguous = i == 0 || *seq == segments[i - 1].0 + 1;
        if !contiguous || !header_matches(fs, path, *seq, stream) {
            break;
        }
        readable += 1;
    }
    for (_, path) in &segments[readable..] {
        fs.remove(path)?;
    }
    if readable < segments.len() {
        fs.sync_dir(dir)?;
    }
    Ok(segments.len() - readable)
}

fn header_matches<F: JournalFs>(fs: &F, path: &Path, seq: u64, stream: Stream) -> bool {
    fs.read_header(path)
        .ok()
        .and_then(|bytes| SegmentHeader::decode(&bytes).ok())
        .is_some_and(|header| header.seq == seq && header.stream == stream)
}

/// The oldest run of sealed segments last modified at least `retention` before `now`.
fn expired<F: JournalFs>(
    fs: &F,
    dir: &Path,
    segments: &Segments,
    now: SystemTime,
    retention: Duration,
) -> anyhow::Result<Vec<SegmentMeta>> {
    let mut old = Vec::new();
    for meta in segments.sealed() {
        let modified = fs
            .modified(&segment_path(dir, meta.seq))
            .with_context(|| format!("reading mtime of share segment {}", meta.seq))?;
        let age = now.duration_since(modified).unwrap_or_default();
        if age < retention {
            break;
        }
        old.push(*meta);
    }
    Ok(old)
}

fn spawn_share_sync(share: Weak<BlobLane>) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(SHARE_SYNC_INTERVAL);
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tick.tick().await;
            // Ends once the store is dropped, so the lane's writer thread can exit.
            let Some(lane) = share.upgrade() else { break };
            lane.lane.request_sync();
        }
    });
}

#[cfg(test)]
mod tests {
    use hotshot_types::traits::metrics::NoMetrics;
    use journal_lane::{format, mem::MemFs};
    use tempfile::TempDir;

    use super::*;

    /// Every record rolls the segment, so each lands in a segment of its own.
    const ROLL_EACH: u64 = 1;
    const RETENTION: Duration = Duration::from_secs(3600);

    struct Env {
        tmp: TempDir,
        fs: MemFs,
    }

    impl Env {
        fn new() -> Self {
            Self {
                tmp: tempfile::tempdir().unwrap(),
                fs: MemFs::default(),
            }
        }

        async fn open(&self, segment_bytes: u64) -> Arc<BlobStore<MemFs>> {
            self.try_open(segment_bytes).await.unwrap()
        }

        async fn try_open(&self, segment_bytes: u64) -> anyhow::Result<Arc<BlobStore<MemFs>>> {
            let cfg = BlobCfg {
                dir: self.tmp.path().to_path_buf(),
                share_retention: RETENTION,
            };
            BlobStore::open_with(cfg, Arc::new(self.fs.clone()), segment_bytes).await
        }

        fn payload_segment(&self, seq: u64) -> PathBuf {
            segment_path(&self.tmp.path().join("payload"), seq)
        }

        fn share_segment(&self, seq: u64) -> PathBuf {
            segment_path(&self.tmp.path().join("share"), seq)
        }
    }

    fn body(tag: u8) -> Vec<u8> {
        vec![tag; 100]
    }

    /// Queues the `nth` share (counting from 1) and waits until it is fsynced.
    async fn append_share_synced(store: &BlobStore<MemFs>, height: u64, tag: u8, nth: u64) {
        store.append_share(height, body(tag));
        store.share.lane.request_sync();
        store.share.lane.wait_durable(nth).await.unwrap();
    }

    /// Appends one payload per height. Returns their locators.
    ///
    /// The final extra record makes sure the writer has rolled past every listed height, which it
    /// does before it picks up the next batch.
    async fn append_payloads(store: &BlobStore<MemFs>, heights: &[u64]) -> Vec<BlobLoc> {
        let mut locs = Vec::new();
        for &height in heights {
            locs.push(
                store
                    .append_payload(height, body(height as u8))
                    .await
                    .unwrap(),
            );
        }
        let last = heights.iter().max().unwrap() + 1;
        store.append_payload(last, body(0)).await.unwrap();
        locs
    }

    #[test]
    fn blob_loc_text_roundtrips() {
        let loc = BlobLoc {
            seq: 0x1f,
            offset: 0x28,
            len: 0xabc,
            lsn: 7,
        };
        assert_eq!(loc.to_string(), "1f:28:abc:7");
        assert_eq!("1f:28:abc:7".parse::<BlobLoc>().unwrap(), loc);
        let max = BlobLoc {
            seq: u64::MAX,
            offset: u64::MAX,
            len: u32::MAX,
            lsn: u64::MAX,
        };
        assert_eq!(max.to_string().parse::<BlobLoc>().unwrap(), max);
    }

    #[test]
    fn blob_loc_parse_is_strict() {
        for bad in [
            "",
            "1:2:3",
            "1:2:3:4:5",
            "1:2:3:",
            "+1:2:3:4",
            "1:2:3:G",
            "1:2:3:A",
            "01:2:3:4",
            "1:2:100000000:4",
            " 1:2:3:4",
            "1:-2:3:4",
        ] {
            assert!(bad.parse::<BlobLoc>().is_err(), "accepted {bad:?}");
        }
    }

    #[tokio::test]
    async fn payload_roundtrips_and_leaves_no_index_entry() {
        let env = Env::new();
        let store = env.open(SEGMENT_BYTES).await;
        store.install_metrics(&NoMetrics);

        let loc = store.append_payload(5, body(7)).await.unwrap();
        assert_eq!(loc.len, 100);
        assert_eq!(store.read_payload(loc).await.unwrap(), Some(body(7)));
        assert!(store.payload.index.lock().is_empty());
        assert!(store.payload_bytes() > 100);
    }

    #[tokio::test]
    async fn read_payload_with_wrong_lsn_is_none() {
        let env = Env::new();
        let store = env.open(SEGMENT_BYTES).await;
        let loc = store.append_payload(5, body(7)).await.unwrap();
        let wrong = BlobLoc {
            lsn: loc.lsn + 1,
            ..loc
        };
        assert_eq!(store.read_payload(wrong).await.unwrap(), None);
    }

    #[tokio::test]
    async fn durable_payload_survives_crash_and_reopen() {
        let env = Env::new();
        let store = env.open(SEGMENT_BYTES).await;
        let first = store.append_payload(1, body(1)).await.unwrap();
        drop(store);
        env.fs.crash(|_| 0);

        let store = env.open(SEGMENT_BYTES).await;
        assert_eq!(store.read_payload(first).await.unwrap(), Some(body(1)));
        let second = store.append_payload(2, body(2)).await.unwrap();
        assert!(second.seq > first.seq);
        assert_eq!(store.read_payload(first).await.unwrap(), Some(body(1)));
        assert_eq!(store.read_payload(second).await.unwrap(), Some(body(2)));
    }

    #[tokio::test]
    async fn concurrent_stores_of_one_height_both_return_readable_locators() {
        let env = Env::new();
        let store = env.open(SEGMENT_BYTES).await;
        let (a, b) = tokio::join!(
            store.append_payload(3, body(3)),
            store.append_payload(3, body(3))
        );
        for loc in [a.unwrap(), b.unwrap()] {
            assert_eq!(store.read_payload(loc).await.unwrap(), Some(body(3)));
        }
        assert!(store.payload.index.lock().is_empty());
        assert!(store.appending.lock().is_empty());
    }

    #[tokio::test]
    async fn gc_payload_below_unlinks_sealed_segments_up_to_height() {
        let env = Env::new();
        let store = env.open(ROLL_EACH).await;
        let locs = append_payloads(&store, &[1, 2, 3, 4]).await;

        let freed = store.gc_payload_below(2).await.unwrap();

        let segment = SegmentHeader::LEN + format::FRAME_HEADER_LEN + body(1).len();
        assert_eq!(freed, 2 * segment as u64);
        assert_eq!(store.read_payload(locs[0]).await.unwrap(), None);
        assert_eq!(store.read_payload(locs[1]).await.unwrap(), None);
        assert_eq!(store.read_payload(locs[2]).await.unwrap(), Some(body(3)));
        assert_eq!(store.read_payload(locs[3]).await.unwrap(), Some(body(4)));
        assert_eq!(store.gc_payload_below(2).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn gc_payload_below_keeps_the_active_segment() {
        let env = Env::new();
        let store = env.open(SEGMENT_BYTES).await;
        let loc = store.append_payload(1, body(1)).await.unwrap();

        assert_eq!(store.gc_payload_below(u64::MAX).await.unwrap(), 0);
        assert_eq!(store.read_payload(loc).await.unwrap(), Some(body(1)));
    }

    #[tokio::test]
    async fn gc_payload_stops_at_a_segment_holding_a_backfilled_old_height() {
        let env = Env::new();
        let store = env.open(ROLL_EACH).await;
        // Height 1 lands after height 10, so its segment inherits the bound 10.
        let locs = append_payloads(&store, &[10, 1, 11]).await;

        assert_eq!(store.gc_payload_below(5).await.unwrap(), 0);
        assert_eq!(store.read_payload(locs[1]).await.unwrap(), Some(body(1)));

        assert!(store.gc_payload_below(10).await.unwrap() > 0);
        assert_eq!(store.read_payload(locs[0]).await.unwrap(), None);
        assert_eq!(store.read_payload(locs[1]).await.unwrap(), None);
        assert_eq!(store.read_payload(locs[2]).await.unwrap(), Some(body(11)));
    }

    #[tokio::test]
    async fn gc_payload_survives_reopen() {
        let env = Env::new();
        let store = env.open(ROLL_EACH).await;
        let locs = append_payloads(&store, &[1, 2, 3]).await;
        store.gc_payload_below(1).await.unwrap();
        drop(store);
        env.fs.crash(|_| 0);

        let store = env.open(ROLL_EACH).await;
        assert_eq!(store.read_payload(locs[0]).await.unwrap(), None);
        assert_eq!(store.read_payload(locs[1]).await.unwrap(), Some(body(2)));
        assert!(store.gc_payload_below(2).await.unwrap() > 0);
        assert_eq!(store.read_payload(locs[1]).await.unwrap(), None);
        assert_eq!(store.read_payload(locs[2]).await.unwrap(), Some(body(3)));
    }

    #[tokio::test]
    async fn open_fails_while_another_store_holds_the_lock() {
        let env = Env::new();
        let _store = env.open(SEGMENT_BYTES).await;
        let err = env.try_open(SEGMENT_BYTES).await.err().unwrap();
        assert!(format!("{err:#}").contains("already in use"), "{err:#}");
    }

    #[tokio::test]
    async fn share_reads_by_height_and_survives_reopen() {
        let env = Env::new();
        let store = env.open(SEGMENT_BYTES).await;
        append_share_synced(&store, 5, 5, 1).await;
        append_share_synced(&store, 6, 6, 2).await;
        assert_eq!(store.read_share(5).await.unwrap(), Some(body(5)));
        assert_eq!(store.read_share(7).await.unwrap(), None);
        drop(store);
        env.fs.crash(|_| 0);

        let store = env.open(SEGMENT_BYTES).await;
        assert_eq!(store.read_share(5).await.unwrap(), Some(body(5)));
        assert_eq!(store.read_share(6).await.unwrap(), Some(body(6)));
        assert_eq!(store.read_share(7).await.unwrap(), None);
    }

    #[tokio::test]
    async fn share_index_rebuild_keeps_the_newest_record_per_height() {
        let env = Env::new();
        let store = env.open(ROLL_EACH).await;
        append_share_synced(&store, 5, 1, 1).await;
        append_share_synced(&store, 6, 6, 2).await;
        append_share_synced(&store, 5, 2, 3).await;
        drop(store);
        env.fs.crash(|_| 0);

        let store = env.open(ROLL_EACH).await;
        assert_eq!(store.read_share(5).await.unwrap(), Some(body(2)));
        assert_eq!(store.read_share(6).await.unwrap(), Some(body(6)));
        assert_eq!(store.share.index.lock().len(), 2);
    }

    #[tokio::test]
    async fn share_sync_task_makes_enqueued_shares_durable() {
        let env = Env::new();
        let store = env.open(SEGMENT_BYTES).await;
        store.append_share(9, body(9));
        tokio::time::timeout(Duration::from_secs(5), store.share.lane.wait_durable(1))
            .await
            .expect("share sync task never synced")
            .unwrap();
        drop(store);
        env.fs.crash(|_| 0);

        let store = env.open(SEGMENT_BYTES).await;
        assert_eq!(store.read_share(9).await.unwrap(), Some(body(9)));
    }

    #[tokio::test]
    async fn unsynced_share_is_lost_in_a_crash_without_error() {
        let env = Env::new();
        let store = env.open(SEGMENT_BYTES).await;
        append_share_synced(&store, 1, 1, 1).await;
        store.append_share(2, body(2));
        drop(store);
        env.fs.crash(|_| 0);

        let store = env.open(SEGMENT_BYTES).await;
        assert_eq!(store.read_share(1).await.unwrap(), Some(body(1)));
        assert_eq!(store.read_share(2).await.unwrap(), None);
    }

    #[tokio::test]
    async fn torn_share_tail_is_truncated_on_reopen() {
        let env = Env::new();
        let store = env.open(SEGMENT_BYTES).await;
        append_share_synced(&store, 1, 1, 1).await;
        store.append_share(2, body(2));
        drop(store);
        // Keeps all but the last byte of the unsynced record.
        env.fs.crash(|n| n.saturating_sub(2));

        let store = env.open(SEGMENT_BYTES).await;
        assert_eq!(store.read_share(1).await.unwrap(), Some(body(1)));
        assert_eq!(store.read_share(2).await.unwrap(), None);
        append_share_synced(&store, 2, 3, 2).await;
        assert_eq!(store.read_share(2).await.unwrap(), Some(body(3)));
    }

    #[tokio::test]
    async fn torn_payload_tail_is_truncated_on_reopen() {
        let env = Env::new();
        let store = env.open(SEGMENT_BYTES).await;
        let loc = store.append_payload(1, body(1)).await.unwrap();
        drop(store);
        // Garbage after the last durable frame, as a partial write leaves it.
        let path = env.payload_segment(loc.seq);
        let end = env.fs.file_len(&path) as u64;
        let mut file = env.fs.open_write(&path).unwrap();
        journal_lane::lane::JournalFile::write_all_at(&mut file, end, &[0xAB; 11]).unwrap();
        journal_lane::lane::JournalFile::sync_data(&mut file).unwrap();

        let store = env.open(SEGMENT_BYTES).await;
        assert_eq!(store.read_payload(loc).await.unwrap(), Some(body(1)));
        let next = store.append_payload(2, body(2)).await.unwrap();
        assert_eq!(store.read_payload(next).await.unwrap(), Some(body(2)));
    }

    #[tokio::test]
    async fn gc_shares_unlinks_the_old_prefix_of_sealed_segments() {
        let env = Env::new();
        let store = env.open(ROLL_EACH).await;
        for height in 1..=4 {
            append_share_synced(&store, height, height as u8, height).await;
        }
        let now = SystemTime::now();
        let old = now - 2 * RETENTION;
        let young = now - RETENTION / 2;
        env.fs.set_modified(&env.share_segment(1), old);
        env.fs.set_modified(&env.share_segment(2), young);
        env.fs.set_modified(&env.share_segment(3), old);

        assert!(store.gc_shares_older_than(now).await.unwrap() > 0);
        assert_eq!(store.read_share(1).await.unwrap(), None);
        assert_eq!(store.read_share(2).await.unwrap(), Some(body(2)));
        assert_eq!(store.read_share(3).await.unwrap(), Some(body(3)));

        let later = now + 2 * RETENTION;
        assert!(store.gc_shares_older_than(later).await.unwrap() > 0);
        assert_eq!(store.read_share(2).await.unwrap(), None);
        assert_eq!(store.read_share(3).await.unwrap(), None);
        assert_eq!(store.read_share(4).await.unwrap(), None);
        assert_eq!(store.share_bytes(), SegmentHeader::LEN as u64);
        assert_eq!(store.gc_shares_older_than(later).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn gc_shares_ignores_height_and_keeps_young_segments() {
        let env = Env::new();
        let store = env.open(ROLL_EACH).await;
        for height in 1..=3 {
            append_share_synced(&store, height, height as u8, height).await;
        }
        assert_eq!(
            store.gc_shares_older_than(SystemTime::now()).await.unwrap(),
            0
        );
        assert_eq!(store.read_share(1).await.unwrap(), Some(body(1)));
    }

    #[tokio::test]
    async fn corrupt_sealed_payload_segment_drops_it_and_later_ones() {
        let env = Env::new();
        let store = env.open(ROLL_EACH).await;
        let locs = append_payloads(&store, &[1, 2, 3]).await;
        drop(store);
        let mut file = env
            .fs
            .open_write(&env.payload_segment(locs[1].seq))
            .unwrap();
        journal_lane::lane::JournalFile::write_all_at(&mut file, 0, &[0xFF; 8]).unwrap();
        journal_lane::lane::JournalFile::sync_data(&mut file).unwrap();

        let store = env.open(ROLL_EACH).await;
        assert_eq!(store.read_payload(locs[0]).await.unwrap(), Some(body(1)));
        assert_eq!(store.read_payload(locs[1]).await.unwrap(), None);
        assert_eq!(store.read_payload(locs[2]).await.unwrap(), None);
        let fresh = store.append_payload(2, body(2)).await.unwrap();
        assert_eq!(store.read_payload(fresh).await.unwrap(), Some(body(2)));
        assert_eq!(store.read_payload(locs[0]).await.unwrap(), Some(body(1)));
    }

    #[tokio::test]
    async fn payload_segment_sequence_gap_drops_later_segments() {
        let env = Env::new();
        let store = env.open(ROLL_EACH).await;
        let locs = append_payloads(&store, &[1, 2, 3]).await;
        drop(store);
        env.fs.remove(&env.payload_segment(locs[1].seq)).unwrap();

        let store = env.open(ROLL_EACH).await;
        assert_eq!(store.read_payload(locs[0]).await.unwrap(), Some(body(1)));
        assert_eq!(store.read_payload(locs[2]).await.unwrap(), None);
        let fresh = store.append_payload(2, body(2)).await.unwrap();
        assert_eq!(store.read_payload(fresh).await.unwrap(), Some(body(2)));
    }

    #[tokio::test]
    async fn corrupt_sealed_share_segment_keeps_earlier_shares() {
        let env = Env::new();
        let store = env.open(ROLL_EACH).await;
        for height in 1..=3 {
            append_share_synced(&store, height, height as u8, height).await;
        }
        drop(store);
        let mut file = env.fs.open_write(&env.share_segment(2)).unwrap();
        journal_lane::lane::JournalFile::write_all_at(&mut file, 0, &[0xFF; 8]).unwrap();
        journal_lane::lane::JournalFile::sync_data(&mut file).unwrap();

        let store = env.open(ROLL_EACH).await;
        assert_eq!(store.read_share(1).await.unwrap(), Some(body(1)));
        assert_eq!(store.read_share(2).await.unwrap(), None);
        assert_eq!(store.read_share(3).await.unwrap(), None);
    }
}
