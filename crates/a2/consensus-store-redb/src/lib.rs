//! A [`Store`] on redb.
//!
//! The store is a directory holding the redb file `index.redb`. Every value lives in redb, and the
//! redb table `_usage` records how many rows and bytes each user table holds.

mod apply;

use std::{
    fs::File,
    io,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use bytes::Bytes;
use consensus_store::{Batch, Error, KeyRange, Snapshot, Store, Table, Usage};
use parking_lot::Mutex;
use redb::{Database, ReadTransaction, ReadableDatabase, TableDefinition, TableError};

pub struct RedbStore {
    db: Database,

    /// Serializes the applies.
    writer: Mutex<()>,

    /// Set once a commit has failed with an unknown outcome.
    poisoned: AtomicBool,

    /// Makes the next commit report failure although it succeeded.
    #[cfg(test)]
    fail_next_commit: AtomicBool,

    /// Makes the next apply fail after its operations, before it commits.
    #[cfg(test)]
    reject_next_apply: AtomicBool,
}

impl RedbStore {
    /// Opens the store in `dir`, creating it if needed.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, Error> {
        let dir = std::path::absolute(dir.as_ref())?;
        create_dir_durably(&dir)?;
        // redb does not sync the directory of a new database file.
        let db = Database::create(dir.join("index.redb")).map_err(backend)?;
        sync_dir(&dir)?;
        Ok(Self {
            db,
            writer: Mutex::new(()),
            poisoned: AtomicBool::new(false),
            #[cfg(test)]
            fail_next_commit: AtomicBool::new(false),
            #[cfg(test)]
            reject_next_apply: AtomicBool::new(false),
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
        let _writer = self.writer.lock();
        self.check_poisoned()?;
        if batch.is_empty() {
            return Ok(());
        }
        match apply::apply(self, batch) {
            Ok(()) => Ok(()),
            Err(apply::Failure::Rejected(e)) => Err(e),
            Err(apply::Failure::Uncertain(e)) => {
                // Reads may show a commit that reopening rolls back, so refuse them, and further
                // writes, until reopened.
                self.poisoned.store(true, Ordering::Release);
                Err(Error::Poisoned(Box::new(e)))
            },
        }
    }

    fn snapshot(&self) -> Result<RedbSnapshot, Error> {
        self.check_poisoned()?;
        let txn = self.db.begin_read().map_err(backend)?;
        Ok(RedbSnapshot { txn })
    }
}

pub struct RedbSnapshot {
    txn: ReadTransaction,
}

impl Snapshot for RedbSnapshot {
    fn get(&self, table: &Table, key: &[u8]) -> Result<Option<Bytes>, Error> {
        let Some(t) = self.open(table)? else {
            return Ok(None);
        };
        Ok(t.get(key)
            .map_err(backend)?
            .map(|value| Bytes::copy_from_slice(value.value())))
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
        Ok(entries.into_iter().flatten().map(|entry| {
            let (k, v) = entry.map_err(backend)?;
            Ok((
                Bytes::copy_from_slice(k.value()),
                Bytes::copy_from_slice(v.value()),
            ))
        }))
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
}

/// Table name to [`Usage`]: rows and bytes.
const USAGE: TableDefinition<&str, (u64, u64)> = TableDefinition::new("_usage");

/// The rows of a user table.
type Rows = redb::ReadOnlyTable<&'static [u8], &'static [u8]>;

fn rows(table: &Table) -> TableDefinition<'_, &'static [u8], &'static [u8]> {
    TableDefinition::new(table.name())
}

fn usage_from((rows, bytes): (u64, u64)) -> Usage {
    Usage { rows, bytes }
}

fn usage_value(u: &Usage) -> (u64, u64) {
    (u.rows, u.bytes)
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
        sync_dir(d)?;
    }
    sync_dir(&existing)
}

/// Makes the entries of the directory `dir` durable.
fn sync_dir(dir: &Path) -> io::Result<()> {
    File::open(dir)?.sync_all()
}

fn backend(e: impl Into<redb::Error>) -> Error {
    Error::Backend(Box::new(e.into()))
}

#[cfg(test)]
mod tests;
