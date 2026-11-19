//! A [`Store`] on redb.
//!
//! The store is a directory holding the redb file `index.redb` and the directory `extern/`.
//! Inline values live in redb. Extern values are appended to segment files in `extern/`, and
//! their rows hold where. A segment file is deleted once no row refers to it and it is no
//! longer the one being appended to. There is no compaction, so a single live value keeps its
//! whole segment on disk.
//!
//! An `apply` first syncs the segments it appended to, then commits the redb transaction. A
//! committed row therefore never refers to extern data that is not on disk. The redb table
//! `_segments` records, for each segment, how many rows refer to it and how many bytes of it
//! are committed. On open, files without an entry and bytes past the committed end, both left
//! by a crash, are removed.
//!
//! Every live segment stays open for as long as the store, because snapshots read segments
//! that later applies delete through handles they hold. The process needs an open-file limit
//! above the number of live segments, about four per GiB of live extern data by default.
//!
//! The backend is Unix-only: it relies on positioned file I/O, on syncing directories, and on
//! deleted segment files staying readable through handles that snapshots still hold.

mod apply;
mod row;
mod segments;

use std::{
    collections::BTreeMap,
    fs::File,
    io,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use bytes::Bytes;
use consensus_store::{Batch, Error, KeyRange, Snapshot, Store, Table, Usage};
use parking_lot::Mutex;
use redb::{Database, ReadTransaction, ReadableDatabase, TableDefinition, TableError};

use crate::{row::Row, segments::Segments};

/// The default size after which the writer starts a new segment.
pub const DEFAULT_SEGMENT_SIZE: u64 = 256 << 20;

#[derive(Clone, Debug)]
pub struct Config {
    /// The writer starts a new segment when an append would grow the current one past this.
    pub segment_size: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            segment_size: DEFAULT_SEGMENT_SIZE,
        }
    }
}

pub struct RedbStore {
    db: Database,
    segments: Segments,
    writer: Mutex<Writer>,

    /// Set once a commit has failed with an unknown outcome.
    poisoned: AtomicBool,
    config: Config,

    /// Makes the next commit report failure although it succeeded.
    #[cfg(test)]
    fail_next_commit: AtomicBool,
}

impl RedbStore {
    /// Opens the store in `dir`, creating it if needed, and removes what a crash left behind.
    pub fn open(dir: impl AsRef<Path>, config: Config) -> Result<Self, Error> {
        let dir = std::path::absolute(dir.as_ref())?;
        create_dir_durably(&dir)?;
        let segments = Segments::open(dir.join("extern"))?;
        // redb does not sync the directory of a new database file.
        let db = Database::create(dir.join("index.redb")).map_err(backend)?;
        segments::sync_dir(&dir)?;

        let recorded = recorded_segments(&db)?;
        segments.recover(&recorded)?;

        let active = recorded.last_key_value().map(|(&id, &(_, end))| Active {
            id,
            file: segments.get(id).expect("recovered segment is open"),
            end,
        });
        let writer = Writer {
            next_segment: active.as_ref().map_or(0, |a| a.id + 1),
            active,
        };
        Ok(Self {
            db,
            segments,
            writer: Mutex::new(writer),
            poisoned: AtomicBool::new(false),
            config,
            #[cfg(test)]
            fail_next_commit: AtomicBool::new(false),
        })
    }

    fn check_poisoned(&self) -> Result<(), Error> {
        if self.poisoned.load(Ordering::Acquire) {
            Err(Error::Poisoned("an earlier commit failed".into()))
        } else {
            Ok(())
        }
    }
}

impl Store for RedbStore {
    type Snapshot = RedbSnapshot;

    fn apply(&self, batch: &Batch) -> Result<(), Error> {
        let mut writer = self.writer.lock();
        self.check_poisoned()?;
        if batch.is_empty() {
            return Ok(());
        }
        match apply::apply(self, &writer, batch) {
            Ok(committed) => {
                *writer = committed;
                Ok(())
            },
            Err(apply::Failure::Rejected(e)) => Err(e),
            Err(apply::Failure::Uncertain(e)) => {
                // The segment state in memory may no longer match the disk. Appending on top
                // of it could overwrite committed extern data, and reads may show a commit
                // that reopening rolls back, so refuse both until reopened.
                self.poisoned.store(true, Ordering::Release);
                Err(Error::Poisoned(Box::new(e)))
            },
        }
    }

    fn snapshot(&self) -> Result<RedbSnapshot, Error> {
        self.check_poisoned()?;
        // Holding the lock across both steps pairs the transaction with exactly the segment
        // files its rows can refer to: `apply` opens a segment before committing rows that
        // refer to it, and closes one only after committing that no row does.
        let files = self.segments.lock();
        let txn = self.db.begin_read().map_err(backend)?;
        Ok(RedbSnapshot {
            txn,
            files: files.clone(),
        })
    }
}

pub struct RedbSnapshot {
    txn: ReadTransaction,
    files: Arc<BTreeMap<u64, Arc<File>>>,
}

impl Snapshot for RedbSnapshot {
    fn get(&self, table: &Table, key: &[u8]) -> Result<Option<Bytes>, Error> {
        let Some(t) = self.open(table)? else {
            return Ok(None);
        };
        match t.get(key).map_err(backend)? {
            Some(raw) => Ok(Some(self.read(table, key, raw.value())?)),
            None => Ok(None),
        }
    }

    fn range(
        &self,
        table: &Table,
        keys: &KeyRange,
    ) -> Result<impl Iterator<Item = Result<(Bytes, Bytes), Error>> + Send, Error> {
        let entries = match (self.open(table)?, keys.bounds()) {
            (Some(t), Some(bounds)) => Some(t.range::<&[u8]>(bounds).map_err(backend)?),
            _ => None,
        };
        let table = table.clone();
        Ok(entries.into_iter().flatten().map(move |entry| {
            let (k, v) = entry.map_err(backend)?;
            let key = k.value();
            Ok((
                Bytes::copy_from_slice(key),
                self.read(&table, key, v.value())?,
            ))
        }))
    }

    fn value_len(&self, table: &Table, key: &[u8]) -> Result<Option<u64>, Error> {
        let Some(t) = self.open(table)? else {
            return Ok(None);
        };
        match t.get(key).map_err(backend)? {
            Some(raw) => Ok(Some(Row::decode(table, key, raw.value())?.value_len())),
            None => Ok(None),
        }
    }

    fn usage(&self, table: &Table) -> Result<Usage, Error> {
        let t = match self.txn.open_table(USAGE) {
            Ok(t) => t,
            Err(TableError::TableDoesNotExist(_)) => return Ok(Usage::default()),
            Err(e) => return Err(backend(e)),
        };
        Ok(t.get(table.name())
            .map_err(backend)?
            .map(|v| usage_from(v.value()))
            .unwrap_or_default())
    }
}

impl RedbSnapshot {
    fn open(&self, table: &Table) -> Result<Option<Rows>, Error> {
        match self.txn.open_table(rows(table)) {
            Ok(t) => Ok(Some(t)),
            Err(TableError::TableDoesNotExist(_)) => Ok(None),
            Err(e) => Err(backend(e)),
        }
    }

    fn read(&self, table: &Table, key: &[u8], raw: &[u8]) -> Result<Bytes, Error> {
        Row::decode(table, key, raw)?.read(table, key, |id| self.files.get(&id).cloned())
    }
}

/// The committed state of the segment files.
#[derive(Clone)]
struct Writer {
    /// The segment being appended to.
    active: Option<Active>,
    next_segment: u64,
}

#[derive(Clone)]
struct Active {
    id: u64,
    file: Arc<File>,
    end: u64,
}

/// Table name to [`Usage`]: rows, inline bytes, extern bytes.
const USAGE: TableDefinition<&str, (u64, u64, u64)> = TableDefinition::new("_usage");

/// Segment id to the number of rows that refer to it, and its committed length.
const SEGMENTS: TableDefinition<u64, (u64, u64)> = TableDefinition::new("_segments");

/// The rows of a user table.
type Rows = redb::ReadOnlyTable<&'static [u8], &'static [u8]>;

fn rows(table: &Table) -> TableDefinition<'_, &'static [u8], &'static [u8]> {
    TableDefinition::new(table.name())
}

fn usage_from((rows, inline_bytes, extern_bytes): (u64, u64, u64)) -> Usage {
    Usage {
        rows,
        inline_bytes,
        extern_bytes,
    }
}

fn usage_value(u: &Usage) -> (u64, u64, u64) {
    (u.rows, u.inline_bytes, u.extern_bytes)
}

/// Creates the absolute path `dir` and its missing ancestors, and syncs every directory whose
/// entries changed.
fn create_dir_durably(dir: &Path) -> io::Result<()> {
    let existing = dir
        .ancestors()
        .find(|d| d.exists())
        .unwrap_or(Path::new("/"))
        .to_path_buf();
    std::fs::create_dir_all(dir)?;
    for d in dir.ancestors().take_while(|d| *d != existing) {
        segments::sync_dir(d)?;
    }
    segments::sync_dir(&existing)
}

fn recorded_segments(db: &Database) -> Result<BTreeMap<u64, (u64, u64)>, Error> {
    let txn = db.begin_read().map_err(backend)?;
    let t = match txn.open_table(SEGMENTS) {
        Ok(t) => t,
        Err(TableError::TableDoesNotExist(_)) => return Ok(BTreeMap::new()),
        Err(e) => return Err(backend(e)),
    };
    let mut recorded = BTreeMap::new();
    for entry in t.range::<u64>(..).map_err(backend)? {
        let (id, value) = entry.map_err(backend)?;
        recorded.insert(id.value(), value.value());
    }
    Ok(recorded)
}

fn backend(e: impl Into<redb::Error>) -> Error {
    Error::Backend(Box::new(e.into()))
}

#[cfg(test)]
mod tests;
