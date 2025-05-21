// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

#![cfg(feature = "file-system-data-source")]

use std::{collections::VecDeque, fmt::Debug};

use atomic_store::{
    append_log, load_store::BincodeLoadStore, AppendLog, AtomicStoreLoader, PersistenceError,
};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{debug, warn};

/// A caching append log for ledger objects.
#[derive(Debug)]
pub(crate) struct LedgerLog<T: Serialize + DeserializeOwned> {
    cache_start: usize,
    cache_size: usize,
    cache: VecDeque<Option<T>>,
    store: AppendLog<BincodeLoadStore<Option<T>>>,
    // Keep track of the number of appended objects which have not yet been committed. We need this
    // to detect when we are inserting at the end of the log or in the middle, as the two casese are
    // handled differently and `self.store.iter().len()` does not update until a new version is
    // committed.
    pending_inserts: usize,
    // Track the number of missing objects, for health reporting.
    missing: usize,
}

impl<T: Serialize + DeserializeOwned + Clone> LedgerLog<T> {
    pub(crate) fn create(
        loader: &mut AtomicStoreLoader,
        file_pattern: &str,
        cache_size: usize,
    ) -> Result<Self, PersistenceError> {
        Ok(Self {
            cache_start: 0,
            cache_size,
            cache: VecDeque::with_capacity(cache_size),
            store: AppendLog::create(
                loader,
                Default::default(),
                file_pattern,
                10u64 << 20, // 10 MB
            )?,
            pending_inserts: 0,
            missing: 0,
        })
    }

    pub(crate) fn open(
        loader: &mut AtomicStoreLoader,
        file_pattern: &str,
        cache_size: usize,
    ) -> Result<Self, PersistenceError> {
        let store = AppendLog::load(
            loader,
            Default::default(),
            file_pattern,
            1u64 << 20, // 1 MB
        )?;
        let len = store.iter().len();
        tracing::info!("loading LedgerLog {}, len={}", file_pattern, len);

        let cache_start = len.saturating_sub(cache_size);
        let mut missing = 0;
        let mut cache = store
            .iter()
            .skip(cache_start)
            .map(|r| {
                if let Err(e) = &r {
                    warn!("failed to load object. Error: {}", e);
                }
                // We treat missing objects and failed-to-load objects the same:
                // if we failed to load a object, it is now missing!
                let obj = r.ok().flatten();
                if obj.is_none() {
                    missing += 1;
                }
                obj
            })
            .collect::<VecDeque<_>>();
        cache.reserve_exact(cache_size - cache.len());

        Ok(Self {
            cache_start,
            cache_size,
            cache,
            store,
            pending_inserts: 0,
            missing,
        })
    }

    pub(crate) fn iter(&self) -> Iter<T> {
        Iter {
            index: 0,
            cache_start: self.cache_start,
            cache: &self.cache,
            store: self.store.iter(),
        }
    }

    pub(crate) fn store_resource(&mut self, resource: Option<T>) -> Result<(), PersistenceError> {
        let missing = resource.is_none();
        self.store.store_resource(&resource)?;
        self.pending_inserts += 1;
        if missing {
            self.missing += 1;
        }
        if self.cache.len() >= self.cache_size {
            self.cache.pop_front();
            self.cache_start += 1;
        }
        self.cache.push_back(resource);
        Ok(())
    }

    /// Insert an object at position `index`.
    ///
    /// Returns whether the object was newly inserted; that is, returns `false` if and only if there
    /// was already an object present at this index.
    pub(crate) fn insert(&mut self, index: usize, object: T) -> Result<bool, PersistenceError>
    where
        T: Debug,
    {
        // If there are missing objects between what we currently have and `object`, pad with
        // placeholders.
        let len = self.iter().len();
        let target_len = std::cmp::max(index, len);
        for i in len..target_len {
            debug!("storing placeholders for position {i}/{target_len}");
            if let Err(err) = self.store_resource(None) {
                warn!("Failed to store placeholder: {}", err);
                return Err(err);
            }
        }
        assert!(target_len >= index);
        if target_len == index {
            // This is the next object in the chain, append it to the log.
            if let Err(err) = self.store_resource(Some(object)) {
                warn!("Failed to store object at index {}: {}", index, err);
                return Err(err);
            }
            Ok(true)
        } else if matches!(self.iter().nth(index), Some(Some(_))) {
            // This is a duplicate, we don't have to insert anything.
            Ok(false)
        } else {
            // This is an object earlier in the chain that we are now receiving asynchronously.
            // Update the placeholder with the actual contents of the object.
            // TODO update persistent storage once AppendLog supports updates.
            // See: https://github.com/EspressoSystems/hotshot-query-service/issues/16
            warn!(
                index,
                len, target_len, "skipping out-of-order object; random inserts not yet supported"
            );

            // TODO Update the object in cache if necessary. Note that we could do this now, even
            // without support for storing the object persistently. But this makes the cache out of
            // sync with persistent storage, and it means we have an object available that will
            // become unavailable once it drops out of cache, which is not really what we want.
            // See: https://github.com/EspressoSystems/hotshot-query-service/issues/16
            // if index >= self.cache_start {
            //     self.cache[index - self.cache_start] = Some(object);
            // }
            Ok(true)
        }
    }

    pub(crate) async fn commit_version(&mut self) -> Result<(), PersistenceError> {
        tracing::debug!("committing new version of LedgerLog");
        self.store.commit_version()?;
        self.pending_inserts = 0;
        Ok(())
    }

    pub(crate) fn skip_version(&mut self) -> Result<(), PersistenceError> {
        self.store.skip_version()
    }

    pub(crate) fn revert_version(&mut self) -> Result<(), PersistenceError> {
        self.store.revert_version()?;

        // Remove objects which were inserted in cache but not committed to storage.
        for _ in 0..self.pending_inserts {
            self.cache.pop_back();
        }

        self.pending_inserts = 0;
        Ok(())
    }

    pub(crate) fn missing(&self, to_height: usize) -> usize {
        // The number of missing objects is the number missing from the sequence we currently have,
        // plus any extra objects at the end if this sequence is shorter than `to_height`.
        self.missing + to_height.saturating_sub(self.iter().len())
    }
}

pub struct Iter<'a, T: Serialize + DeserializeOwned> {
    index: usize,
    cache_start: usize,
    cache: &'a VecDeque<Option<T>>,
    store: append_log::Iter<'a, BincodeLoadStore<Option<T>>>,
}

impl<T: Serialize + DeserializeOwned + Clone> Iterator for Iter<'_, T> {
    type Item = Option<T>;

    fn next(&mut self) -> Option<Self::Item> {
        // False positive: clippy suggests `self.next()` instead of `self.nth(0)`, but that would be
        // recursive.
        #[allow(clippy::iter_nth_zero)]
        self.nth(0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Include objects in cache that haven't necessarily been committed to storage yet. This is
        // consistent with `nth`, which will yield such objects.
        let len = (self.cache_start + self.cache.len()).saturating_sub(self.index);
        (len, Some(len))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.index += n;
        let res = if self.index >= self.cache_start {
            // Get the object from cache if we can.
            self.cache.get(self.index - self.cache_start).cloned()
        } else {
            // Otherwise load from storage.
            self.store.nth(n).map(|res| {
                if let Err(e) = &res {
                    warn!("failed to load object at position {}: error {}", n, e);
                }
                // Both a failed load and a successful load of `None` are treated the same: as
                // missing data, so we yield `None`. The latter case can happen if there was a
                // previous failed load and we marked this entry as explicitly missing.
                res.ok().flatten()
            })
        };

        self.index += 1;
        res
    }

    fn count(self) -> usize {
        self.size_hint().0
    }
}

impl<T: Serialize + DeserializeOwned + Clone> ExactSizeIterator for Iter<'_, T> {}

#[cfg(test)]
mod test {
    use atomic_store::AtomicStore;
    use tempfile::TempDir;

    use super::*;
    use crate::testing::setup_test;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ledger_log_creation() {
        setup_test();

        let dir = TempDir::with_prefix("test_ledger_log").unwrap();

        // Create and populuate a log.
        {
            let mut loader = AtomicStoreLoader::create(dir.path(), "test_ledger_log").unwrap();
            let mut log = LedgerLog::<u64>::create(&mut loader, "ledger", 3).unwrap();
            let mut store = AtomicStore::open(loader).unwrap();
            for i in 0..5 {
                log.store_resource(Some(i)).unwrap();
                log.commit_version().await.unwrap();
                store.commit_version().unwrap();
            }
        }

        // Load the log from storage and check that we get the correct contents.
        {
            let mut loader = AtomicStoreLoader::load(dir.path(), "test_ledger_log").unwrap();
            let log = LedgerLog::<u64>::open(&mut loader, "ledger", 3).unwrap();
            AtomicStore::open(loader).unwrap();
            assert_eq!(
                log.iter().collect::<Vec<_>>(),
                (0..5).map(Some).collect::<Vec<_>>()
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ledger_log_insert() {
        setup_test();

        let dir = TempDir::with_prefix("test_ledger_log").unwrap();
        let mut loader = AtomicStoreLoader::create(dir.path(), "test_ledger_log").unwrap();
        let mut log = LedgerLog::<u64>::create(&mut loader, "ledger", 3).unwrap();
        let mut store = AtomicStore::open(loader).unwrap();
        assert_eq!(log.iter().collect::<Vec<_>>(), Vec::<Option<u64>>::new());

        // Insert at end.
        log.insert(0, 1).unwrap();
        log.commit_version().await.unwrap();
        store.commit_version().unwrap();
        assert_eq!(log.iter().collect::<Vec<_>>(), vec![Some(1)]);

        // Insert past end.
        log.insert(4, 2).unwrap();
        log.commit_version().await.unwrap();
        store.commit_version().unwrap();
        assert_eq!(
            log.iter().collect::<Vec<_>>(),
            vec![Some(1), None, None, None, Some(2)]
        );

        // Insert in middle (in cache).
        log.insert(2, 3).unwrap();
        log.commit_version().await.unwrap();
        store.commit_version().unwrap();
        // TODO re-enable this check once AppendLog supports random access updates.
        // See https://github.com/EspressoSystems/hotshot-query-service/issues/16
        // assert_eq!(
        //     log.iter().collect::<Vec<_>>(),
        //     vec![Some(1), None, Some(3), None, Some(2)]
        // );

        // Insert in middle (out of cache).
        log.insert(1, 4).unwrap();
        log.commit_version().await.unwrap();
        store.commit_version().unwrap();
        // TODO check results once AppendLog supports random access updates.
        // See https://github.com/EspressoSystems/hotshot-query-service/issues/16
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ledger_log_iter() {
        setup_test();

        let dir = TempDir::with_prefix("test_ledger_log").unwrap();
        let mut loader = AtomicStoreLoader::create(dir.path(), "test_ledger_log").unwrap();
        let mut log = LedgerLog::<u64>::create(&mut loader, "ledger", 3).unwrap();
        let mut store = AtomicStore::open(loader).unwrap();
        for i in 0..5 {
            log.store_resource(Some(i)).unwrap();
            log.commit_version().await.unwrap();
            store.commit_version().unwrap();
        }

        assert_eq!(log.iter().len(), 5);
        for i in 0..5 {
            let mut iter = log.iter();
            assert_eq!(iter.nth(i as usize).unwrap(), Some(i), "{:?}", log);

            // `nth` should not only have returned the `n`th element, but also advanced the iterator.
            assert_eq!(
                iter.collect::<Vec<_>>(),
                (i + 1..5).map(Some).collect::<Vec<_>>()
            );
        }
        assert_eq!(log.iter().nth(5), None);
    }
}
