//! One [`Store::apply`](consensus_store::Store::apply): a redb write transaction, and the usage
//! bookkeeping that goes with it.

use std::collections::BTreeMap;

use consensus_store::{Batch, Error, KeyRange, Op, Table, Usage};
use redb::{Durability, ReadableTable, ReadableTableMetadata, WriteTransaction};

use crate::{RedbStore, USAGE, backend, rows, usage_from, usage_value};

/// Applies `batch` in one redb transaction.
pub fn apply(store: &RedbStore, batch: &Batch) -> Result<(), Failure> {
    let mut txn = store
        .db
        .begin_write()
        .map_err(|e| Failure::Rejected(backend(e)))?;
    txn.set_durability(Durability::Immediate)
        .map_err(|e| Failure::Rejected(backend(e)))?;
    let result = Apply::new(&txn).run(batch);
    #[cfg(test)]
    let result = match store
        .reject_next_apply
        .swap(false, std::sync::atomic::Ordering::Relaxed)
    {
        true => Err(Error::Backend("injected".into())),
        false => result,
    };
    // Dropping the transaction uncommitted discards it.
    result.map_err(Failure::Rejected)?;
    txn.commit().map_err(|e| Failure::Uncertain(backend(e)))?;
    #[cfg(test)]
    if store
        .fail_next_commit
        .swap(false, std::sync::atomic::Ordering::Relaxed)
    {
        return Err(Failure::Uncertain(Error::Backend("injected".into())));
    }
    Ok(())
}

pub enum Failure {
    /// Nothing was committed.
    Rejected(Error),

    /// The commit failed, and may or may not have reached disk.
    Uncertain(Error),
}

type RowsMut<'a> = redb::Table<'a, &'static [u8], &'static [u8]>;

struct Apply<'a> {
    txn: &'a WriteTransaction,

    /// The user tables this transaction has opened.
    tables: BTreeMap<Table, RowsMut<'a>>,

    /// The usage of each table this transaction changes, as it leaves it.
    usage: BTreeMap<Table, Usage>,
}

impl<'a> Apply<'a> {
    fn new(txn: &'a WriteTransaction) -> Self {
        Self {
            txn,
            tables: BTreeMap::new(),
            usage: BTreeMap::new(),
        }
    }

    /// Applies the operations and writes the usage, short of committing.
    fn run(mut self, batch: &Batch) -> Result<(), Error> {
        for op in batch.ops() {
            self.op(op)?;
        }
        self.delete_empty_tables()?;
        self.write_usage()
    }

    fn op(&mut self, op: &Op) -> Result<(), Error> {
        match op {
            Op::Put { table, key, value } => self.put(table, key, value),
            Op::Delete { table, key } => self.delete(table, key),
            Op::DeleteRange { table, keys } => self.delete_range(table, keys),
            Op::DropTable { table } => self.drop_table(table),
        }
    }

    fn put(&mut self, table: &Table, key: &[u8], value: &[u8]) -> Result<(), Error> {
        let old = self
            .table(table)?
            .insert(key, value)
            .map_err(backend)?
            .map(|v| v.value().len());
        self.account(table, |usage| {
            usage.add(Usage::of_row(key.len(), value.len()));
            if let Some(old) = old {
                usage.sub(Usage::of_row(key.len(), old));
            }
        })
    }

    fn delete(&mut self, table: &Table, key: &[u8]) -> Result<(), Error> {
        let old = self
            .table(table)?
            .remove(key)
            .map_err(backend)?
            .map(|v| v.value().len());
        match old {
            Some(old) => self.account(table, |usage| usage.sub(Usage::of_row(key.len(), old))),
            None => Ok(()),
        }
    }

    fn delete_range(&mut self, table: &Table, keys: &KeyRange) -> Result<(), Error> {
        let Some(bounds) = keys.bounds() else {
            return Ok(());
        };
        let mut removed = Usage::default();
        self.table(table)?
            .retain_in::<&[u8], _>(bounds, |k, v| {
                removed.add(Usage::of_row(k.len(), v.len()));
                false
            })
            .map_err(backend)?;
        self.account(table, |usage| usage.sub(removed))
    }

    fn drop_table(&mut self, table: &Table) -> Result<(), Error> {
        self.tables.remove(table);
        self.txn.delete_table(rows(table)).map_err(backend)?;
        self.usage.insert(table.clone(), Usage::default());
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

    /// Changes the usage of `table` with `change`, loading it first.
    fn account(&mut self, table: &Table, change: impl FnOnce(&mut Usage)) -> Result<(), Error> {
        if !self.usage.contains_key(table) {
            let t = self.txn.open_table(USAGE).map_err(backend)?;
            let usage = t
                .get(table.name())
                .map_err(backend)?
                .map(|v| usage_from(v.value()))
                .unwrap_or_default();
            self.usage.insert(table.clone(), usage);
        }
        change(self.usage.get_mut(table).expect("loaded"));
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
