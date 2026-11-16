//! One [`Store::apply`](consensus_store::Store::apply): a redb write transaction, and the
//! segment appends and reference counts that go with it.

use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
    io,
    os::unix::fs::FileExt,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
};

use bytes::Bytes;
use consensus_store::{Batch, EXTERN_THRESHOLD, Error, KeyRange, Op, Table, UpdateFn, Usage};
use redb::{Durability, ReadableTable, ReadableTableMetadata, WriteTransaction};

use crate::{
    Active, RedbStore, SEGMENTS, USAGE, Writer, backend,
    row::{ExternRef, Row, encode_extern, encode_inline},
    rows, usage_from, usage_value,
};

/// Applies `batch` on top of the committed state `writer`, and returns the new committed state.
///
/// A panic in an update closure aborts the transaction, deletes the segments it created, and
/// then continues unwinding.
pub fn apply(store: &RedbStore, writer: &Writer, batch: &Batch) -> Result<Writer, Failure> {
    let mut txn = store
        .db
        .begin_write()
        .map_err(|e| Failure::Rejected(backend(e)))?;
    txn.set_durability(Durability::Immediate)
        .map_err(|e| Failure::Rejected(backend(e)))?;
    let mut apply = Apply::new(store, &txn, writer.clone());
    let result = apply.run(batch);
    let Done {
        writer,
        created,
        panic,
    } = apply.done();
    if let Some(panic) = panic {
        let _ = txn.abort();
        discard(store, &created);
        resume_unwind(panic);
    }
    let freed = match result {
        Ok(freed) => freed,
        Err(e) => {
            discard(store, &created);
            return Err(Failure::Rejected(e));
        },
    };
    // A failed commit may still have reached disk, so the segments it created stay. If it
    // did not, the next open deletes them.
    txn.commit().map_err(|e| Failure::Uncertain(backend(e)))?;
    #[cfg(test)]
    if store
        .fail_next_commit
        .swap(false, std::sync::atomic::Ordering::Relaxed)
    {
        return Err(Failure::Uncertain(Error::Backend("injected".into())));
    }
    for id in freed {
        store.segments.remove(id);
    }
    Ok(writer)
}

pub enum Failure {
    /// Nothing was committed, and the segment files created along the way are deleted.
    Rejected(Error),

    /// The commit failed, and may or may not have reached disk.
    Uncertain(Error),
}

type RowsMut<'a> = redb::Table<'a, &'static [u8], &'static [u8]>;

struct Apply<'a> {
    store: &'a RedbStore,
    txn: &'a WriteTransaction,

    /// The user tables this transaction has opened.
    tables: BTreeMap<Table, RowsMut<'a>>,

    /// The segment state as this transaction leaves it.
    writer: Writer,

    /// Segments created by this transaction.
    created: Vec<u64>,

    /// Segments appended to, with their new length.
    appended: BTreeMap<u64, u64>,

    /// Segments whose `_segments` entry may change.
    touched: BTreeSet<u64>,

    /// Change in the number of rows that refer to each segment.
    refs: BTreeMap<u64, i64>,

    /// The usage of each table this transaction changes, as it leaves it.
    usage: BTreeMap<Table, Usage>,

    /// The payload of a panic in an update closure.
    panic: Option<Box<dyn Any + Send>>,
}

/// What a finished [`Apply`] leaves, once its tables are closed.
struct Done {
    writer: Writer,
    created: Vec<u64>,
    panic: Option<Box<dyn Any + Send>>,
}

/// What a row contributes to the bookkeeping.
#[derive(Clone, Copy)]
struct RowInfo {
    usage: Usage,
    segment: Option<u64>,
}

impl RowInfo {
    fn of(table: &Table, key: &[u8], raw: &[u8]) -> Result<Self, Error> {
        let row = Row::decode(table, key, raw)?;
        Ok(Self {
            usage: row.usage(key.len()),
            segment: row.extern_ref().map(|r| r.segment),
        })
    }

    /// The contribution of a row being removed, or `None` if the row is corrupt.
    ///
    /// A corrupt row can still be removed. If it referred to a segment, that segment keeps a
    /// reference and outlives its last real row: space is lost, data is not.
    fn of_removed(table: &Table, key: &[u8], raw: &[u8]) -> Option<Self> {
        Self::of(table, key, raw).ok()
    }
}

#[derive(Clone, Copy)]
enum Change {
    Added,
    Removed,
}

impl<'a> Apply<'a> {
    fn new(store: &'a RedbStore, txn: &'a WriteTransaction, writer: Writer) -> Self {
        Self {
            store,
            txn,
            tables: BTreeMap::new(),
            writer,
            created: Vec::new(),
            appended: BTreeMap::new(),
            touched: BTreeSet::new(),
            refs: BTreeMap::new(),
            usage: BTreeMap::new(),
            panic: None,
        }
    }

    /// Applies the operations and writes the bookkeeping, short of committing. Returns the
    /// segments to delete once committed.
    fn run(&mut self, batch: &Batch) -> Result<Vec<u64>, Error> {
        for op in batch.ops() {
            self.op(op)?;
        }
        self.delete_empty_tables()?;
        self.sync_segments()?;
        let freed = self.write_segments()?;
        self.write_usage()?;
        Ok(freed)
    }

    fn done(self) -> Done {
        Done {
            writer: self.writer,
            created: self.created,
            panic: self.panic,
        }
    }

    fn op(&mut self, op: &Op) -> Result<(), Error> {
        match op {
            Op::Put { table, key, value } => self.put(table, key, value),
            Op::Delete { table, key } => self.delete(table, key),
            Op::DeleteRange { table, keys } => self.delete_range(table, keys),
            Op::Update { table, key, f } => self.update(table, key, f),
            Op::Copy {
                from,
                from_key,
                to,
                to_key,
            } => self.copy(from, from_key, to, to_key),
            Op::DropTable { table } => self.drop_table(table),
        }
    }

    fn put(&mut self, table: &Table, key: &[u8], value: &[u8]) -> Result<(), Error> {
        if value.len() > EXTERN_THRESHOLD {
            let r = self.append(value)?;
            self.insert_raw(table, key, &encode_extern(r))
        } else {
            self.insert_raw(table, key, &encode_inline(value))
        }
    }

    fn delete(&mut self, table: &Table, key: &[u8]) -> Result<(), Error> {
        let old = self
            .table(table)?
            .remove(key)
            .map_err(backend)?
            .map(|v| RowInfo::of_removed(table, key, v.value()));
        if let Some(old) = old {
            self.account(table, old, Change::Removed)?;
        }
        Ok(())
    }

    fn delete_range(&mut self, table: &Table, keys: &KeyRange) -> Result<(), Error> {
        let Some(bounds) = keys.bounds() else {
            return Ok(());
        };
        let mut removed = Vec::new();
        self.table(table)?
            .retain_in::<&[u8], _>(bounds, |k, v| {
                removed.push(RowInfo::of_removed(table, k, v));
                false
            })
            .map_err(backend)?;
        for info in removed {
            self.account(table, info, Change::Removed)?;
        }
        Ok(())
    }

    fn update(&mut self, table: &Table, key: &[u8], f: &UpdateFn) -> Result<(), Error> {
        let store = self.store;
        let current = match self.table(table)?.get(key).map_err(backend)? {
            Some(raw) => Some(
                Row::decode(table, key, raw.value())?
                    .read(table, key, |id| store.segments.get(id))?,
            ),
            None => None,
        };
        let new = match catch_unwind(AssertUnwindSafe(|| f(current.as_deref()))) {
            Ok(new) => new,
            Err(panic) => {
                self.panic = Some(panic);
                return Err(Error::Backend("update closure panicked".into()));
            },
        };
        match new {
            Some(value) => self.put(table, key, &value),
            None => Ok(()),
        }
    }

    fn copy(
        &mut self,
        from: &Table,
        from_key: &[u8],
        to: &Table,
        to_key: &[u8],
    ) -> Result<(), Error> {
        // An inline value or an extern reference, so the copy is small.
        let raw = self
            .table(from)?
            .get(from_key)
            .map_err(backend)?
            .map(|v| Bytes::copy_from_slice(v.value()));
        match raw {
            Some(raw) => self.insert_raw(to, to_key, &raw),
            None => Ok(()),
        }
    }

    fn drop_table(&mut self, table: &Table) -> Result<(), Error> {
        self.load_usage(table)?;
        // Only rows with extern values hold references to release. Usage says whether there
        // are any; if it is wrong, segments are kept too long, never freed too early.
        if self.usage[table].extern_bytes > 0 {
            let mut segments = Vec::new();
            for entry in self.table(table)?.iter().map_err(backend)? {
                let (k, v) = entry.map_err(backend)?;
                if let Some(info) = RowInfo::of_removed(table, k.value(), v.value()) {
                    segments.extend(info.segment);
                }
            }
            for id in segments {
                self.release(id);
            }
        }
        self.tables.remove(table);
        self.txn.delete_table(rows(table)).map_err(backend)?;
        self.usage.insert(table.clone(), Usage::default());
        Ok(())
    }

    /// Sets `key` to the encoded row `raw`, replacing any row it had.
    fn insert_raw(&mut self, table: &Table, key: &[u8], raw: &[u8]) -> Result<(), Error> {
        let new = RowInfo::of(table, key, raw)?;
        let old = self
            .table(table)?
            .insert(key, raw)
            .map_err(backend)?
            .map(|v| RowInfo::of_removed(table, key, v.value()));
        self.account(table, Some(new), Change::Added)?;
        if let Some(old) = old {
            self.account(table, old, Change::Removed)?;
        }
        Ok(())
    }

    /// The rows of `table`, opened once per transaction.
    ///
    /// Opening creates the table; [`Self::delete_empty_tables`] deletes it again if it ends up
    /// empty.
    fn table(&mut self, table: &Table) -> Result<&mut RowsMut<'a>, Error> {
        if !self.tables.contains_key(table) {
            let t = self.txn.open_table(rows(table)).map_err(backend)?;
            self.tables.insert(table.clone(), t);
        }
        Ok(self.tables.get_mut(table).expect("opened"))
    }

    /// Counts a row that `change` adds or removes. `None` is a corrupt row being removed,
    /// which is not counted.
    fn account(
        &mut self,
        table: &Table,
        info: Option<RowInfo>,
        change: Change,
    ) -> Result<(), Error> {
        let Some(info) = info else {
            return Ok(());
        };
        self.load_usage(table)?;
        let usage = self.usage.get_mut(table).expect("loaded");
        match change {
            Change::Added => {
                usage.add(info.usage);
                if let Some(id) = info.segment {
                    *self.refs.entry(id).or_default() += 1;
                    self.touched.insert(id);
                }
            },
            Change::Removed => {
                usage.sub(info.usage);
                if let Some(id) = info.segment {
                    self.release(id);
                }
            },
        }
        Ok(())
    }

    fn release(&mut self, segment: u64) {
        *self.refs.entry(segment).or_default() -= 1;
        self.touched.insert(segment);
    }

    fn load_usage(&mut self, table: &Table) -> Result<(), Error> {
        if self.usage.contains_key(table) {
            return Ok(());
        }
        let t = self.txn.open_table(USAGE).map_err(backend)?;
        let usage = t
            .get(table.name())
            .map_err(backend)?
            .map(|v| usage_from(v.value()))
            .unwrap_or_default();
        self.usage.insert(table.clone(), usage);
        Ok(())
    }

    /// Appends `value` to the active segment, starting a new one if it would grow too large.
    fn append(&mut self, value: &[u8]) -> Result<ExternRef, Error> {
        let len = value.len() as u64;
        let full = |a: &Active| a.end > 0 && a.end + len > self.store.config.segment_size;
        if self.writer.active.as_ref().is_none_or(full) {
            self.start_segment()?;
        }
        let active = self.writer.active.as_mut().expect("started");
        active.file.write_all_at(value, active.end)?;
        let r = ExternRef {
            segment: active.id,
            offset: active.end,
            len,
            crc: crc32fast::hash(value),
        };
        active.end += len;
        self.appended.insert(active.id, active.end);
        self.touched.insert(active.id);
        Ok(r)
    }

    fn start_segment(&mut self) -> Result<(), Error> {
        if let Some(previous) = &self.writer.active {
            self.touched.insert(previous.id);
        }
        let id = self.writer.next_segment;
        let file = self.store.segments.create(id)?;
        self.created.push(id);
        self.writer.next_segment += 1;
        self.writer.active = Some(Active { id, file, end: 0 });
        Ok(())
    }

    /// Deletes the opened tables that end up empty, including ones opened only to read.
    fn delete_empty_tables(&mut self) -> Result<(), Error> {
        for (table, t) in std::mem::take(&mut self.tables) {
            let empty = t.is_empty().map_err(backend)?;
            drop(t);
            if empty {
                self.txn.delete_table(rows(&table)).map_err(backend)?;
                self.usage.insert(table, Usage::default());
            }
        }
        Ok(())
    }

    /// Makes the appended extern data durable, so that the commit can refer to it.
    fn sync_segments(&self) -> Result<(), Error> {
        for &id in self.appended.keys() {
            let file = self.store.segments.get(id).ok_or_else(|| {
                io::Error::other(format!("segment {id:016x} closed while being written"))
            })?;
            file.sync_data()?;
        }
        if !self.created.is_empty() {
            self.store.segments.sync_dir()?;
        }
        Ok(())
    }

    /// Writes the reference counts and lengths of the touched segments. Returns those no row
    /// refers to any more, other than the active one.
    fn write_segments(&self) -> Result<Vec<u64>, Error> {
        let active = self.writer.active.as_ref().map(|a| a.id);
        let mut t = self.txn.open_table(SEGMENTS).map_err(backend)?;
        let mut freed = Vec::new();
        for &id in &self.touched {
            let delta = self.refs.get(&id).copied().unwrap_or(0);
            let (refs, end) = match t.get(id).map_err(backend)?.map(|v| v.value()) {
                Some((refs, end)) => (checked_refs(id, refs, delta)?, end),
                // A segment without an entry is new, or was named by a corrupt row being
                // removed; it has no references to release.
                None => (delta.max(0) as u64, 0),
            };
            let end = self.appended.get(&id).copied().unwrap_or(end);
            if refs == 0 && Some(id) != active {
                t.remove(id).map_err(backend)?;
                freed.push(id);
            } else {
                t.insert(id, (refs, end)).map_err(backend)?;
            }
        }
        Ok(freed)
    }

    fn write_usage(&self) -> Result<(), Error> {
        let mut t = self.txn.open_table(USAGE).map_err(backend)?;
        for (table, usage) in &self.usage {
            if usage.is_empty() {
                t.remove(table.name()).map_err(backend)?;
            } else {
                t.insert(table.name(), usage_value(usage))
                    .map_err(backend)?;
            }
        }
        Ok(())
    }
}

/// The reference count of segment `id` after adding `delta` to `refs`.
///
/// Releasing more references than recorded means the bookkeeping is wrong. Freeing the segment
/// then could lose data other rows refer to, so the batch is rejected.
fn checked_refs(id: u64, refs: u64, delta: i64) -> Result<u64, Error> {
    u64::try_from(i128::from(refs) + i128::from(delta)).map_err(|_| {
        Error::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("segment {id:016x}: more references released than recorded"),
        ))
    })
}

/// Deletes segment files that a failed transaction created.
fn discard(store: &RedbStore, created: &[u64]) {
    for &id in created {
        store.segments.remove(id);
    }
}
