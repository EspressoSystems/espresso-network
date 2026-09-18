//! One-shot startup probe of the SQLite and file-system storage backends.
//!
//! Consensus writes to disk on every view, taking the database's (or data directory's) single
//! writer lock. A validator on a network filesystem, a volatile filesystem, or a disk with slow
//! fsync sees that cost as consensus timeouts, with nothing pointing at the storage layer. This
//! module answers that once per process start, and never fails startup: every part that can
//! error degrades to an unknown/absent value and logs at DEBUG.
//!
//! The fs backend never calls fsync itself (`fs::Persistence` writes via `std::fs::write`, which
//! does not sync); the fsync number still bounds `sync_data` cost there, and the filesystem
//! classification is backend-neutral either way.

use std::{
    io::{self, Seek, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[cfg(feature = "embedded-db")]
use hotshot_query_service::data_source::storage::sql::Db;
use hotshot_types::traits::metrics::Metrics;
use rand::Rng;
#[cfg(feature = "embedded-db")]
use sqlx::{Decode, Pool, Row, Type};
use tempfile::Builder;
use tracing::Level;

/// Logs `$msg` at `$level`, a runtime value, with the given fields; `tracing::event!` requires
/// its level to be a compile-time constant, so this dispatches to the matching fixed-level macro
/// instead.
macro_rules! log_at {
    ($level:expr, $msg:expr, $($fields:tt)*) => {
        match $level {
            Level::ERROR => tracing::error!(target: "announce", $($fields)*, "{}", $msg),
            Level::WARN => tracing::warn!(target: "announce", $($fields)*, "{}", $msg),
            Level::INFO => tracing::info!(target: "announce", $($fields)*, "{}", $msg),
            Level::DEBUG => tracing::debug!(target: "announce", $($fields)*, "{}", $msg),
            Level::TRACE => tracing::trace!(target: "announce", $($fields)*, "{}", $msg),
        }
    };
}

const MOUNTINFO_PATH: &str = "/proc/self/mountinfo";

const MAX_SAMPLES: usize = 64;
const BUDGET: Duration = Duration::from_secs(1);

/// p50 fsync latency above which the finding is logged as WARN; above [`FSYNC_ERROR`], ERROR.
const FSYNC_WARN: Duration = Duration::from_millis(10);
const FSYNC_ERROR: Duration = Duration::from_millis(50);

/// Filesystem types where POSIX advisory locking is unreliable: remote/distributed filesystems,
/// plus the FUSE drivers that proxy to one. Other `fuse.*` drivers (e.g. `fuse.gocryptfs`,
/// `fuse.bindfs`) are local-backed, so they fall through to [`FsClass::Unknown`] instead.
const NETWORK_FS: &[&str] = &[
    "nfs",
    "nfs4",
    "cifs",
    "smb3",
    "smbfs",
    "9p",
    "ceph",
    "glusterfs",
    "lustre",
    "afs",
    "fuse.sshfs",
    "fuse.s3fs",
    "fuse.rclone",
    "fuse.juicefs",
    "fuse.gcsfuse",
    "fuse.blobfuse",
    "fuse.davfs",
];
const VOLATILE_FS: &[&str] = &["tmpfs", "ramfs"];
const CONTAINER_FS: &[&str] = &["overlay", "aufs"];
const LOCAL_FS: &[&str] = &[
    "ext2", "ext3", "ext4", "xfs", "btrfs", "zfs", "f2fs", "bcachefs", "jfs", "reiserfs", "apfs",
    "hfs", "hfsplus",
];

#[derive(Clone, Debug)]
pub struct StorageProbe {
    pub path: PathBuf,
    pub fs: FsInfo,
    pub fsync: Option<FsyncStats>,
    /// `None` for the fs backend, which has no pragmas to read.
    pub pragmas: Option<SqlitePragmas>,
}

#[derive(Clone, Debug)]
pub struct FsInfo {
    pub fs_type: String,
    pub source: String,
    pub mount_point: PathBuf,
    pub class: FsClass,
}

/// How suitable a filesystem is for a SQLite database that consensus writes to on every view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsClass {
    Local,
    /// POSIX advisory locking is unreliable.
    Network,
    /// Contents do not survive a restart.
    Volatile,
    /// Container layer or union mount; writes land somewhere the operator did not choose.
    Container,
    /// Neither local nor one of the above; may also mean mountinfo couldn't be read.
    Unknown,
}

#[derive(Clone, Debug)]
pub struct FsyncStats {
    /// How many samples the budget allowed; a low count means each fsync was slow.
    pub samples: usize,
    pub p50: Duration,
    pub max: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SqlitePragmas {
    pub journal_mode: String,
    pub synchronous: &'static str,
    pub page_size: u64,
}

/// Probes the filesystem backing `dir`, and, when `pragmas` is `Some` (the SQLite backend),
/// includes them in the log line before returning. Runs inline; this happens once per process
/// before consensus starts, bounded at roughly [`BUDGET`], so blocking is not observable.
pub async fn probe(dir: &Path, pragmas: Option<SqlitePragmas>) -> StorageProbe {
    let page_size = pragmas.as_ref().map(|p| p.page_size);
    let fs = resolve_fs_info(dir);
    let fsync = match fsync_probe(dir, page_size) {
        Ok(stats) => Some(stats),
        Err(err) => {
            tracing::debug!(?err, "storage probe: fsync probe failed");
            None
        },
    };

    let probe = StorageProbe {
        path: dir.to_path_buf(),
        fs,
        fsync,
        pragmas,
    };
    probe.log();
    probe
}

impl StorageProbe {
    /// Logs two lines: filesystem classification, then fsync latency and pragmas.
    pub fn log(&self) {
        self.log_fs();
        self.log_fsync_and_pragmas();
    }

    fn log_fs(&self) {
        let fs = self.fs.fs_type.as_str();
        let source = self.fs.source.as_str();
        let mount = self.fs.mount_point.display();
        let class = fs_class_label(self.fs.class);
        let path = self.path.display();
        let level = fs_class_severity(self.fs.class);
        let msg = fs_class_message(self.fs.class);

        log_at!(level, msg, fs, source, %mount, class, %path);
    }

    fn log_fsync_and_pragmas(&self) {
        let journal_mode = self
            .pragmas
            .as_ref()
            .map_or("n/a", |p| p.journal_mode.as_str());
        let synchronous = self.pragmas.as_ref().map_or("n/a", |p| p.synchronous);
        let page_size = self
            .pragmas
            .as_ref()
            .map_or("n/a".to_string(), |p| p.page_size.to_string());
        let page_size = page_size.as_str();

        let Some(fsync) = &self.fsync else {
            tracing::warn!(
                target: "announce",
                journal_mode,
                synchronous,
                page_size,
                "storage probe: Could not write a test file into the storage directory. Check \
                 that it exists and is writable by the user running the node."
            );
            return;
        };

        let p50_us = fsync.p50.as_micros();
        let max_us = fsync.max.as_micros();
        let samples = fsync.samples;
        let level = fsync_severity(fsync.p50);
        let msg = fsync_message(level);

        log_at!(
            level,
            msg,
            p50_us,
            max_us,
            samples,
            journal_mode,
            synchronous,
            page_size
        );
    }

    fn backend_label(&self) -> &'static str {
        if self.pragmas.is_some() {
            "sqlite"
        } else {
            "fs"
        }
    }

    /// Registers the probe's findings on `metrics` as `info` (text) and `fsync_micros` (gauge per
    /// statistic); callers pass a subgroup so the exported names carry that prefix, e.g.
    /// `consensus_disk_info`. Uses microseconds because [`Gauge::set`] takes a `usize`; a
    /// histogram would give seconds, but the probe never re-runs, so its buckets' rate is always
    /// zero.
    pub fn register(&self, metrics: &dyn Metrics) {
        metrics
            .text_family(
                "info".to_string(),
                vec![
                    "backend".to_string(),
                    "fs".to_string(),
                    "source".to_string(),
                    "class".to_string(),
                    "journal_mode".to_string(),
                    "synchronous".to_string(),
                ],
            )
            .create(vec![
                self.backend_label().to_string(),
                self.fs.fs_type.clone(),
                self.fs.source.clone(),
                fs_class_label(self.fs.class).to_string(),
                self.pragmas
                    .as_ref()
                    .map_or("n/a".to_string(), |p| p.journal_mode.clone()),
                self.pragmas
                    .as_ref()
                    .map_or("n/a".to_string(), |p| p.synchronous.to_string()),
            ]);

        let Some(fsync) = &self.fsync else {
            return;
        };

        let fsync_micros =
            metrics.gauge_family("fsync_micros".to_string(), vec!["stat".to_string()]);
        for (stat, value) in [("p50", fsync.p50), ("max", fsync.max)] {
            fsync_micros
                .create(vec![stat.to_string()])
                .set(value.as_micros() as usize);
        }
    }
}

fn fs_class_label(class: FsClass) -> &'static str {
    match class {
        FsClass::Local => "local",
        FsClass::Network => "network",
        FsClass::Volatile => "volatile",
        FsClass::Container => "container",
        FsClass::Unknown => "unknown",
    }
}

fn fs_class_severity(class: FsClass) -> Level {
    match class {
        FsClass::Network => Level::ERROR,
        FsClass::Volatile | FsClass::Container => Level::WARN,
        FsClass::Local | FsClass::Unknown => Level::INFO,
    }
}

fn fs_class_message(class: FsClass) -> &'static str {
    match class {
        FsClass::Network => {
            "storage probe: ESPRESSO_NODE_STORAGE_PATH is on a network filesystem. SQLite file \
             locking does not work reliably over the network and the database can be corrupted. \
             Move it to a disk attached to this machine."
        },
        FsClass::Volatile => {
            "storage probe: ESPRESSO_NODE_STORAGE_PATH is on a RAM-backed filesystem. Everything \
             under it is erased when this machine reboots, and the node will have to resync from \
             genesis. Point it at a real disk, for example /var/lib/espresso."
        },
        FsClass::Container => {
            "storage probe: ESPRESSO_NODE_STORAGE_PATH is inside the container's own writable \
             layer, not a mounted volume. The database is destroyed whenever the container is \
             recreated, which includes every image update. Mount a Docker volume or a host \
             directory and point ESPRESSO_NODE_STORAGE_PATH at it."
        },
        FsClass::Unknown => {
            "storage probe: Could not classify the filesystem backing ESPRESSO_NODE_STORAGE_PATH. \
             If this is a network mount or a RAM disk, move the database to a local persistent \
             disk."
        },
        FsClass::Local => "storage probe: storage filesystem looks suitable",
    }
}

fn fsync_severity(p50: Duration) -> Level {
    if p50 > FSYNC_ERROR {
        Level::ERROR
    } else if p50 > FSYNC_WARN {
        Level::WARN
    } else {
        Level::INFO
    }
}

fn fsync_message(level: Level) -> &'static str {
    if level == Level::INFO {
        "storage probe: fsync latency"
    } else {
        "storage probe: Disk is slow to commit writes. Consensus makes several durable writes per \
         view, so at this latency the node is likely to miss views. Use a local SSD or NVMe; \
         network storage and throttled cloud volumes (exhausted burst credits) are the usual cause."
    }
}

fn unknown_fs_info() -> FsInfo {
    FsInfo {
        fs_type: "unknown".to_string(),
        source: "unknown".to_string(),
        mount_point: PathBuf::new(),
        class: FsClass::Unknown,
    }
}

fn resolve_fs_info(dir: &Path) -> FsInfo {
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());

    let mountinfo = match std::fs::read_to_string(MOUNTINFO_PATH) {
        Ok(content) => content,
        Err(err) => {
            tracing::debug!(?err, "storage probe: failed to read {MOUNTINFO_PATH}");
            return unknown_fs_info();
        },
    };

    parse_mountinfo(&mountinfo, &canonical).unwrap_or_else(unknown_fs_info)
}

/// Longest mount point that is a prefix of `path`. `path` is already canonicalized.
///
/// When two mounts share the same mount point (an overmount stacking a new mount on an existing
/// path), the later line in `/proc/self/mountinfo` is the one currently visible there.
/// `Iterator::max_by_key` returns the *last* maximal element on a tie, so preserving the file's
/// line order here (rather than, say, sorting) is load-bearing for that tie-break.
fn parse_mountinfo(mountinfo: &str, path: &Path) -> Option<FsInfo> {
    mountinfo
        .lines()
        .filter_map(parse_mountinfo_line)
        .filter(|(mount_point, ..)| path.starts_with(mount_point))
        .max_by_key(|(mount_point, ..)| mount_point.as_os_str().len())
        .map(|(mount_point, fs_type, source)| FsInfo {
            class: classify_fs(&fs_type),
            fs_type,
            source,
            mount_point,
        })
}

/// Parses one `/proc/self/mountinfo` line into `(mount_point, fs_type, source)`.
///
/// Format: `<fields...> - <fs_type> <source> <options>`, where fields before the `-` include the
/// mount point as the 5th whitespace-separated field, followed by zero or more optional fields
/// (`shared:N`, `master:N`, `propagate_from:N`, `unbindable`). The mount point's fixed position
/// makes the optional field count irrelevant here. See `proc(5)`.
fn parse_mountinfo_line(line: &str) -> Option<(PathBuf, String, String)> {
    let (fields, fs_fields) = line.split_once(" - ")?;

    let mount_point = fields.split_whitespace().nth(4)?;
    let mut fs_fields = fs_fields.split_whitespace();
    let fs_type = fs_fields.next()?;
    let source = fs_fields.next()?;

    Some((
        PathBuf::from(mount_point.replace("\\040", " ")),
        fs_type.to_string(),
        source.replace("\\040", " "),
    ))
}

/// Unrecognized filesystem types are [`FsClass::Unknown`] rather than assumed local; asserting
/// safety for a filesystem this probe has never seen would defeat the point of probing.
fn classify_fs(fs_type: &str) -> FsClass {
    if NETWORK_FS.contains(&fs_type) {
        FsClass::Network
    } else if VOLATILE_FS.contains(&fs_type) {
        FsClass::Volatile
    } else if CONTAINER_FS.contains(&fs_type) {
        FsClass::Container
    } else if LOCAL_FS.contains(&fs_type) {
        FsClass::Local
    } else {
        FsClass::Unknown
    }
}

/// Writes and syncs a scratch file inside `dir`, sized to `page_size` (SQLite's own page size, or
/// 4096 if unknown), and returns fsync latency stats. The file is written and synced once before
/// sampling to preallocate its page; each sample then seeks back to the start and overwrites in
/// place, mirroring SQLite's steady-state writes rather than growing the file every iteration, so
/// that `sync_data` (`fdatasync` on Linux) need not flush changed metadata such as file size.
fn fsync_probe(dir: &Path, page_size: Option<u64>) -> io::Result<FsyncStats> {
    let page_size = page_size.unwrap_or(4096) as usize;
    let mut buf = vec![0u8; page_size];
    // Random, not zero-filled: a copy-on-write filesystem with compression enabled (ZFS, btrfs)
    // collapses an all-zero write into a hole, measuring nothing about real fsync latency.
    rand::thread_rng().fill(&mut buf[..]);

    let mut file = Builder::new().tempfile_in(dir)?;
    file.write_all(&buf)?;
    file.as_file().sync_data()?;

    let mut samples = Vec::with_capacity(MAX_SAMPLES);
    let start = Instant::now();
    loop {
        let sample_start = Instant::now();
        file.seek(io::SeekFrom::Start(0))?;
        file.write_all(&buf)?;
        file.as_file().sync_data()?;
        samples.push(sample_start.elapsed());

        if samples.len() >= MAX_SAMPLES || start.elapsed() >= BUDGET {
            break;
        }
    }

    samples.sort_unstable();
    Ok(FsyncStats {
        p50: samples[samples.len() / 2],
        max: *samples.last().expect("loop takes at least one sample"),
        samples: samples.len(),
    })
}

#[cfg(feature = "embedded-db")]
async fn read_pragma<T>(pool: &Pool<Db>, pragma: &str) -> Option<T>
where
    T: for<'a> Decode<'a, Db> + Type<Db>,
{
    match sqlx::query(pragma).fetch_one(pool).await {
        Ok(row) => row.try_get(0).ok(),
        Err(err) => {
            tracing::debug!(pragma, ?err, "storage probe: pragma query failed");
            None
        },
    }
}

/// Reads `journal_mode`, `synchronous` and `page_size` from the same pool. If one query fails the
/// pool is unusable and the rest would too, so the first failure short-circuits the others.
#[cfg(feature = "embedded-db")]
pub async fn read_pragmas(pool: &Pool<Db>) -> Option<SqlitePragmas> {
    let journal_mode = read_pragma(pool, "PRAGMA journal_mode").await?;
    let synchronous = synchronous_name(read_pragma(pool, "PRAGMA synchronous").await?);
    let page_size: i64 = read_pragma(pool, "PRAGMA page_size").await?;

    Some(SqlitePragmas {
        journal_mode,
        synchronous,
        page_size: page_size as u64,
    })
}

/// SQLite reports `PRAGMA synchronous` back as its numeric setting, not the name used to set it.
#[cfg(feature = "embedded-db")]
fn synchronous_name(code: i64) -> &'static str {
    match code {
        0 => "off",
        1 => "normal",
        2 => "full",
        3 => "extra",
        _ => "unknown",
    }
}

#[cfg(test)]
mod test {
    #[cfg(feature = "embedded-db")]
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    const MOUNTINFO_FIXTURE: &str = concat!(
        "36 35 98:0 / / rw,relatime shared:1 - ext4 /dev/nvme0n1p2 rw\n",
        "37 35 0:31 / /var/lib/espresso rw,relatime shared:2 - nfs4 nfsserver:/export rw\n",
        "38 35 0:32 / /mnt/my\\040disk rw,relatime shared:3 - ext4 /dev/sdb1 rw\n",
        "39 35 0:33 / /proc rw,nosuid,relatime - proc proc rw\n",
        "40 35 0:34 / /run rw,nosuid shared:1 master:2 propagate_from:3 - tmpfs tmpfs rw\n",
        "41 35 0:35 / /mnt/overmounted rw,relatime - ext4 /dev/sdc1 rw\n",
        "42 35 0:36 / /mnt/overmounted rw,relatime - tmpfs tmpfs rw\n",
    );

    #[test]
    fn classify_fs_representative_per_class() {
        assert_eq!(classify_fs("ext4"), FsClass::Local);
        assert_eq!(classify_fs("nfs4"), FsClass::Network);
        assert_eq!(classify_fs("tmpfs"), FsClass::Volatile);
        assert_eq!(classify_fs("overlay"), FsClass::Container);
        assert_eq!(classify_fs("made_up_fs"), FsClass::Unknown);
    }

    #[test]
    fn parse_mountinfo_picks_longest_matching_mount() {
        let info = parse_mountinfo(
            MOUNTINFO_FIXTURE,
            Path::new("/var/lib/espresso/sqlite/database"),
        )
        .expect("mount found");

        assert_eq!(info.mount_point, Path::new("/var/lib/espresso"));
        assert_eq!(info.fs_type, "nfs4");
        assert_eq!(info.source, "nfsserver:/export");
        assert_eq!(info.class, FsClass::Network);
    }

    #[test]
    fn parse_mountinfo_unescapes_mount_point() {
        let info = parse_mountinfo(MOUNTINFO_FIXTURE, Path::new("/mnt/my disk/database")).unwrap();

        assert_eq!(info.mount_point, Path::new("/mnt/my disk"));
        assert_eq!(info.source, "/dev/sdb1");
    }

    /// Two entries at the same mount point: the later one (the currently visible overmount) must
    /// win. This is the tie-break documented on `parse_mountinfo`.
    #[test]
    fn parse_mountinfo_overmount_prefers_later_entry() {
        let info = parse_mountinfo(MOUNTINFO_FIXTURE, Path::new("/mnt/overmounted/database"))
            .expect("mount found");

        assert_eq!(info.fs_type, "tmpfs");
        assert_eq!(info.class, FsClass::Volatile);
    }

    #[test]
    fn parse_mountinfo_line_variants() {
        let cases = [
            // Falls back to the root mount.
            ("/home/user/data", "/", "ext4", FsClass::Local),
            // No optional fields before the `-`.
            ("/proc/self", "/proc", "proc", FsClass::Unknown),
            // Several optional fields (shared/master/propagate_from) before the `-`.
            ("/run/lock", "/run", "tmpfs", FsClass::Volatile),
        ];
        for (path, mount, fs_type, class) in cases {
            let info = parse_mountinfo(MOUNTINFO_FIXTURE, Path::new(path)).unwrap_or_else(|| {
                panic!("expected a mount for {path}");
            });
            assert_eq!(info.mount_point, Path::new(mount), "{path}");
            assert_eq!(info.fs_type, fs_type, "{path}");
            assert_eq!(info.class, class, "{path}");
        }

        assert!(parse_mountinfo("", Path::new("/anything")).is_none());
    }

    #[cfg(feature = "embedded-db")]
    #[tokio::test]
    async fn read_pragmas_falls_back_to_none_on_query_error() {
        let pool = SqlitePoolOptions::new().connect(":memory:").await.unwrap();
        pool.close().await;

        assert!(read_pragmas(&pool).await.is_none());
    }

    #[cfg(feature = "embedded-db")]
    #[tokio::test]
    async fn read_pragmas_reads_live_values() {
        let pool = SqlitePoolOptions::new().connect(":memory:").await.unwrap();

        let pragmas = read_pragmas(&pool).await.expect("pragmas readable");

        assert_eq!(pragmas.journal_mode, "memory");
        assert_ne!(pragmas.synchronous, "unknown");
        assert!(pragmas.page_size > 0);
    }
}
