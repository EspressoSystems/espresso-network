//! The segment files that hold extern values.

use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use consensus_store::Error;
use parking_lot::{Mutex, MutexGuard};

/// The open file of every live segment, by id.
///
/// Snapshots share the map; a change copies it.
pub type Files = Arc<BTreeMap<u64, Arc<File>>>;

/// The segment directory and the open file of every segment in it.
pub struct Segments {
    dir: PathBuf,
    files: Mutex<Files>,
}

impl Segments {
    pub fn open(dir: PathBuf) -> io::Result<Self> {
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            files: Mutex::new(Arc::default()),
        })
    }

    /// Opens the recorded segments, cut to their committed length, and deletes the files of
    /// all others.
    ///
    /// `recorded` maps a segment id to its reference count and committed length.
    pub fn recover(&self, recorded: &BTreeMap<u64, (u64, u64)>) -> Result<(), Error> {
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            // Only this store writes files here; anything else is not ours to delete.
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let recorded_id = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(parse_id)
                .filter(|id| recorded.contains_key(id));
            if recorded_id.is_none() {
                std::fs::remove_file(&path)?;
            }
        }
        let mut guard = self.files.lock();
        let files = Arc::make_mut(&mut guard);
        for (&id, &(_, end)) in recorded {
            let path = self.path(id);
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|e| match e.kind() {
                    io::ErrorKind::NotFound => invalid(id, "recorded segment file is missing"),
                    _ => Error::Io(e),
                })?;
            let len = file.metadata()?.len();
            if len < end {
                return Err(invalid(
                    id,
                    &format!("{len} bytes, shorter than its committed length {end}"),
                ));
            }
            if len > end {
                file.set_len(end)?;
                file.sync_all()?;
            }
            files.insert(id, Arc::new(file));
        }
        drop(guard);
        sync_dir(&self.dir)?;
        Ok(())
    }

    /// Creates segment `id`, empty, replacing any file of that id.
    pub fn create(&self, id: u64) -> io::Result<Arc<File>> {
        let file = Arc::new(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(self.path(id))?,
        );
        Arc::make_mut(&mut self.files.lock()).insert(id, file.clone());
        Ok(file)
    }

    pub fn get(&self, id: u64) -> Option<Arc<File>> {
        self.files.lock().get(&id).cloned()
    }

    /// Closes segment `id` and deletes its file.
    ///
    /// Snapshots that still hold the file can read it until they are dropped. A failed delete
    /// leaves a file without an entry in `_segments`, which the next open deletes.
    pub fn remove(&self, id: u64) {
        Arc::make_mut(&mut self.files.lock()).remove(&id);
        let _ = std::fs::remove_file(self.path(id));
    }

    pub fn lock(&self) -> MutexGuard<'_, Files> {
        self.files.lock()
    }

    pub fn sync_dir(&self) -> io::Result<()> {
        sync_dir(&self.dir)
    }

    fn path(&self, id: u64) -> PathBuf {
        self.dir.join(format!("{id:016x}"))
    }
}

pub fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

/// The id of a segment file name, which [`Segments::path`] writes as 16 lowercase hex digits.
fn parse_id(name: &str) -> Option<u64> {
    let canonical = name.len() == 16
        && name
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    canonical
        .then(|| u64::from_str_radix(name, 16).ok())
        .flatten()
}

fn invalid(id: u64, reason: &str) -> Error {
    Error::Io(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("segment {id:016x}: {reason}"),
    ))
}
