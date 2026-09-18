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

use hotshot_types::traits::metrics::Metrics;
use rand::Rng;
use tempfile::Builder;

#[cfg(target_os = "linux")]
const MOUNTINFO_PATH: &str = "/proc/self/mountinfo";

const MAX_SAMPLES: usize = 64;
const BUDGET: Duration = Duration::from_secs(1);

/// p50 fsync latency above which the finding is logged as WARN; above [`FSYNC_ERROR`], ERROR.
const FSYNC_WARN: Duration = Duration::from_millis(10);
const FSYNC_ERROR: Duration = Duration::from_millis(50);

/// Filesystem types where POSIX advisory locking is unreliable: remote/distributed filesystems,
/// the FUSE drivers that proxy to one, and the host-shared mounts Docker Desktop, Colima, Lima
/// and VirtualBox use to bind a directory from outside the VM/container. Other `fuse.*` drivers
/// (e.g. `fuse.gocryptfs`, `fuse.bindfs`) are local-backed, so they fall through to
/// [`FsClass::Unknown`] instead.
#[cfg(target_os = "linux")]
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
    "fuse.glusterfs",
    "fuse.cephfs",
    "fuse.ceph",
    "virtiofs",
    "vboxsf",
    "fuse.grpcfuse",
    "fuse.osxfs",
];
#[cfg(target_os = "linux")]
const VOLATILE_FS: &[&str] = &["tmpfs", "ramfs"];
#[cfg(target_os = "linux")]
const CONTAINER_FS: &[&str] = &["overlay", "aufs"];
#[cfg(target_os = "linux")]
const LOCAL_FS: &[&str] = &[
    "ext2", "ext3", "ext4", "xfs", "btrfs", "zfs", "f2fs", "bcachefs", "jfs", "reiserfs",
];

#[derive(Clone, Debug)]
pub struct StorageProbe {
    /// `None` when the filesystem wasn't classified: on non-Linux platforms, where
    /// `/proc/self/mountinfo` doesn't exist.
    pub fs: Option<FsInfo>,
    pub fsync: Option<FsyncStats>,
    /// Set alongside `fsync: None`, to carry the `io::Error` into the WARN log line.
    fsync_error: Option<String>,
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

/// `p50`/`max` time the whole write loop (`seek` + `write_all` + `sync_data`), so they measure
/// durable-write latency, not `sync_data` alone.
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
///
/// Logs the filesystem classification as soon as it's known, before the fsync probe runs: a mount
/// whose `fdatasync` hangs would otherwise produce no output pointing at the storage layer.
pub async fn probe(dir: &Path, pragmas: Option<SqlitePragmas>) -> StorageProbe {
    let page_size = pragmas.as_ref().map(|p| p.page_size);
    let fs = resolve_fs_info(dir);
    log_fs_info(dir, fs.as_ref());

    let (fsync, fsync_error) = match fsync_probe(dir, page_size) {
        Ok(stats) => (Some(stats), None),
        Err(err) => (None, Some(err.to_string())),
    };

    let probe = StorageProbe {
        fs,
        fsync,
        fsync_error,
        pragmas,
    };
    probe.log_fsync_and_pragmas();
    probe
}

/// Logs the classification unconditionally at INFO, then, only if it's a problem, one additional
/// line carrying just the remediation message. A healthy local filesystem gets no second line.
fn log_fs_info(path: &Path, fs: Option<&FsInfo>) {
    let Some(fs) = fs else {
        tracing::debug!(
            path = %path.display(),
            "storage probe: filesystem was not probed on this platform"
        );
        return;
    };

    tracing::info!(
        target: "announce",
        fs = fs.fs_type.as_str(),
        source = fs.source.as_str(),
        mount = %fs.mount_point.display(),
        class = fs_class_label(fs.class),
        path = %path.display(),
        "storage probe: filesystem"
    );

    match fs.class {
        FsClass::Local => {},
        FsClass::Network => tracing::error!(
            "storage probe: ESPRESSO_NODE_STORAGE_PATH is on a network or host-shared filesystem. \
             SQLite file locking does not work reliably on these and the database can be \
             corrupted. Move it to a disk attached to this machine."
        ),
        FsClass::Volatile => tracing::warn!(
            "storage probe: ESPRESSO_NODE_STORAGE_PATH is on a RAM-backed filesystem. Everything \
             under it is erased when this machine reboots, and the node will have to resync from \
             genesis. Point it at a real disk, for example /var/lib/espresso."
        ),
        FsClass::Container => tracing::warn!(
            "storage probe: ESPRESSO_NODE_STORAGE_PATH is inside the container's own writable \
             layer, not a mounted volume. The database is destroyed whenever the container is \
             recreated, which includes every image update. Mount a Docker volume or a host \
             directory and point ESPRESSO_NODE_STORAGE_PATH at it."
        ),
        FsClass::Unknown => tracing::warn!(
            "storage probe: Could not classify the filesystem backing ESPRESSO_NODE_STORAGE_PATH. \
             If this is a network mount or a RAM disk, move the database to a local persistent \
             disk."
        ),
    }
}

impl StorageProbe {
    /// Logs the fsync/pragma facts unconditionally at INFO, then, only if it's a problem, one
    /// additional line: the remediation message, or, if the fsync probe couldn't even run, the
    /// `io::Error` that stopped it.
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
        let p50_us = self
            .fsync
            .as_ref()
            .map_or("n/a".to_string(), |f| f.p50.as_micros().to_string());
        let max_us = self
            .fsync
            .as_ref()
            .map_or("n/a".to_string(), |f| f.max.as_micros().to_string());
        let samples = self
            .fsync
            .as_ref()
            .map_or("n/a".to_string(), |f| f.samples.to_string());

        tracing::info!(
            target: "announce",
            journal_mode,
            synchronous,
            page_size = page_size.as_str(),
            p50_us = p50_us.as_str(),
            max_us = max_us.as_str(),
            samples = samples.as_str(),
            "storage probe: fsync and pragmas"
        );

        let Some(fsync) = &self.fsync else {
            let error = self.fsync_error.as_deref().unwrap_or("unknown error");
            tracing::warn!(
                error,
                "storage probe: Could not write a test file into the storage directory. Check \
                 that it exists and is writable by the user running the node."
            );
            return;
        };

        if fsync.p50 > FSYNC_ERROR {
            tracing::error!(
                "storage probe: Disk is slow to commit writes. Consensus makes several durable \
                 writes per view, so at this latency the node is likely to miss views. Use a \
                 local SSD or NVMe; network storage and throttled cloud volumes (exhausted burst \
                 credits) are the usual cause."
            );
        } else if fsync.p50 > FSYNC_WARN {
            tracing::warn!(
                "storage probe: Disk is slow to commit writes. Consensus makes several durable \
                 writes per view, so at this latency the node is likely to miss views. Use a \
                 local SSD or NVMe; network storage and throttled cloud volumes (exhausted burst \
                 credits) are the usual cause."
            );
        }
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
        let (fs_type, fs_class) = match &self.fs {
            Some(fs) => (fs.fs_type.clone(), fs_class_label(fs.class).to_string()),
            None => ("unknown".to_string(), "not_probed".to_string()),
        };

        // The mount source stays out of the label set: for a network mount it is an internal
        // hostname, and nothing aggregates by device. It is in the log line instead.
        metrics
            .text_family(
                "info".to_string(),
                vec![
                    "backend".to_string(),
                    "fs".to_string(),
                    "class".to_string(),
                    "journal_mode".to_string(),
                    "synchronous".to_string(),
                ],
            )
            .create(vec![
                self.backend_label().to_string(),
                fs_type,
                fs_class,
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

#[cfg(target_os = "linux")]
fn unknown_fs_info() -> FsInfo {
    FsInfo {
        fs_type: "unknown".to_string(),
        source: "unknown".to_string(),
        mount_point: PathBuf::new(),
        class: FsClass::Unknown,
    }
}

#[cfg(target_os = "linux")]
fn resolve_fs_info(dir: &Path) -> Option<FsInfo> {
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());

    let mountinfo = match std::fs::read_to_string(MOUNTINFO_PATH) {
        Ok(content) => content,
        Err(err) => {
            // On a Linux node an unreadable mountinfo is a real anomaly, not an expected gap.
            tracing::warn!(?err, "storage probe: failed to read {MOUNTINFO_PATH}");
            return Some(unknown_fs_info());
        },
    };

    Some(parse_mountinfo(&mountinfo, &canonical).unwrap_or_else(unknown_fs_info))
}

/// `/proc/self/mountinfo` is Linux-only; other platforms are not classified.
#[cfg(not(target_os = "linux"))]
fn resolve_fs_info(_dir: &Path) -> Option<FsInfo> {
    None
}

/// Longest mount point that is a prefix of `path`. `path` is already canonicalized.
///
/// When two mounts share the same mount point (an overmount stacking a new mount on an existing
/// path), the later line in `/proc/self/mountinfo` is the one currently visible there.
/// `Iterator::max_by_key` returns the *last* maximal element on a tie, so preserving the file's
/// line order here (rather than, say, sorting) is load-bearing for that tie-break.
#[cfg(target_os = "linux")]
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
#[cfg(target_os = "linux")]
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
#[cfg(target_os = "linux")]
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
    let page_size = page_size.unwrap_or(4096).clamp(512, 65536) as usize;
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

#[cfg(test)]
mod test {
    use super::*;

    #[cfg(target_os = "linux")]
    const MOUNTINFO_FIXTURE: &str = concat!(
        "36 35 98:0 / / rw,relatime shared:1 - ext4 /dev/nvme0n1p2 rw\n",
        "37 35 0:31 / /var/lib/espresso rw,relatime shared:2 - nfs4 nfsserver:/export rw\n",
        "38 35 0:32 / /mnt/my\\040disk rw,relatime shared:3 - ext4 /dev/sdb1 rw\n",
        "39 35 0:33 / /proc rw,nosuid,relatime - proc proc rw\n",
        "40 35 0:34 / /run rw,nosuid shared:1 master:2 propagate_from:3 - tmpfs tmpfs rw\n",
        "41 35 0:35 / /mnt/overmounted rw,relatime - ext4 /dev/sdc1 rw\n",
        "42 35 0:36 / /mnt/overmounted rw,relatime - tmpfs tmpfs rw\n",
    );

    #[cfg(target_os = "linux")]
    #[test]
    fn classify_fs_representative_per_class() {
        assert_eq!(classify_fs("ext4"), FsClass::Local);
        assert_eq!(classify_fs("nfs4"), FsClass::Network);
        assert_eq!(classify_fs("tmpfs"), FsClass::Volatile);
        assert_eq!(classify_fs("overlay"), FsClass::Container);
        assert_eq!(classify_fs("made_up_fs"), FsClass::Unknown);
    }

    #[cfg(target_os = "linux")]
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

    #[cfg(target_os = "linux")]
    #[test]
    fn parse_mountinfo_unescapes_mount_point() {
        let info = parse_mountinfo(MOUNTINFO_FIXTURE, Path::new("/mnt/my disk/database")).unwrap();

        assert_eq!(info.mount_point, Path::new("/mnt/my disk"));
        assert_eq!(info.source, "/dev/sdb1");
    }

    /// Two entries at the same mount point: the later one (the currently visible overmount) must
    /// win. This is the tie-break documented on `parse_mountinfo`.
    #[cfg(target_os = "linux")]
    #[test]
    fn parse_mountinfo_overmount_prefers_later_entry() {
        let info = parse_mountinfo(MOUNTINFO_FIXTURE, Path::new("/mnt/overmounted/database"))
            .expect("mount found");

        assert_eq!(info.fs_type, "tmpfs");
        assert_eq!(info.class, FsClass::Volatile);
    }

    #[cfg(target_os = "linux")]
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
}
