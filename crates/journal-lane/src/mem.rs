use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use crate::{
    format::SegmentHeader,
    lane::{JournalFile, JournalFs},
};

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

    fn open_read_stream(&self, path: &std::path::Path) -> std::io::Result<Box<dyn std::io::Read>> {
        Ok(Box::new(std::io::Cursor::new(self.open_read(path)?)))
    }

    fn read_header(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>> {
        let bytes = self.open_read(path)?;
        if bytes.len() < SegmentHeader::LEN {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        Ok(bytes[..SegmentHeader::LEN].to_vec())
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
