//! An in-memory [`Store`] for tests.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, PoisonError},
};

use bytes::Bytes;

use crate::{Batch, Error, KeyRange, Op, Snapshot, Store, Table, Usage};

/// A [`Store`] that keeps everything in memory and loses it on drop.
///
/// `apply` copies every table it changes, so it suits tests, not large data.
#[derive(Default)]
pub struct MemStore {
    state: Mutex<Arc<State>>,
}

#[derive(Clone)]
pub struct MemSnapshot(Arc<State>);

#[derive(Clone, Default)]
struct State {
    tables: BTreeMap<Table, Arc<BTreeMap<Bytes, Bytes>>>,
}

impl MemStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Store for MemStore {
    type Snapshot = MemSnapshot;

    fn apply(&self, batch: &Batch) -> Result<(), Error> {
        let mut current = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mut next = State::clone(&current);
        for op in batch.ops() {
            next.apply(op);
        }
        *current = Arc::new(next);
        Ok(())
    }

    fn snapshot(&self) -> Result<MemSnapshot, Error> {
        let current = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(MemSnapshot(current.clone()))
    }
}

impl State {
    fn apply(&mut self, op: &Op) {
        match op {
            Op::Put { table, key, value } => {
                self.table_mut(table).insert(key.clone(), value.clone());
            },
            Op::Delete { table, key } => {
                self.table_mut(table).remove(key);
            },
            Op::DeleteRange { table, keys } => {
                self.table_mut(table).retain(|k, _| !keys.contains(k));
            },
            Op::DropTable { table } => {
                self.tables.remove(table);
            },
        }
    }

    fn get(&self, table: &Table, key: &[u8]) -> Option<Bytes> {
        self.tables.get(table)?.get(key).cloned()
    }

    fn table_mut(&mut self, table: &Table) -> &mut BTreeMap<Bytes, Bytes> {
        Arc::make_mut(self.tables.entry(table.clone()).or_default())
    }
}

impl Snapshot for MemSnapshot {
    fn get(&self, table: &Table, key: &[u8]) -> Result<Option<Bytes>, Error> {
        Ok(self.0.get(table, key))
    }

    fn range(
        &self,
        table: &Table,
        keys: &KeyRange,
    ) -> Result<impl Iterator<Item = Result<(Bytes, Bytes), Error>> + Send, Error> {
        let entries = self
            .0
            .tables
            .get(table)
            .zip(keys.bounds())
            .into_iter()
            .flat_map(|(t, bounds)| t.range::<[u8], _>(bounds))
            .map(|(k, v)| Ok((k.clone(), v.clone())));
        Ok(entries)
    }

    fn usage(&self, table: &Table) -> Result<Usage, Error> {
        let mut usage = Usage::default();
        for (k, v) in self.0.tables.get(table).into_iter().flat_map(|t| t.iter()) {
            usage.add(Usage::of_row(k.len(), v.len()));
        }
        Ok(usage)
    }
}

#[cfg(test)]
mod tests {
    use super::MemStore;

    #[test]
    fn conformance() {
        crate::conformance::run(MemStore::new);
    }
}
