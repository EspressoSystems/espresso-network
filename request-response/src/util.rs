use std::{collections::VecDeque, hash::Hash, sync::Arc};

use dashmap::DashMap;
use parking_lot::Mutex;

/// A [`VecDeque`] with a maximum size
pub struct BoundedVecDeque<T> {
    /// The inner [`VecDeque`]
    inner: VecDeque<T>,
    /// The maximum size of the [`VecDeque`]
    max_size: usize,
}

impl<T> BoundedVecDeque<T> {
    /// Create a new bounded [`VecDeque`] with the given maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: VecDeque::new(),
            max_size,
        }
    }

    /// Push an item into the bounded [`VecDeque`], removing the oldest item if the
    /// maximum size is reached
    pub fn push(&mut self, item: T) {
        if self.inner.len() >= self.max_size {
            self.inner.pop_front();
        }
        self.inner.push_back(item);
    }
}

#[derive(Clone)]
pub struct NamedSemaphore<T: Clone + Eq + Hash> {
    /// The underlying map of keys to their semaphore
    inner: Arc<DashMap<T, Arc<()>>>,

    /// The maximum number of permits for each key
    max_permits_per_key: usize,

    /// The maximum number of permits that can be held across all keys
    max_total_permits: Option<usize>,

    /// The total number of permits that are currently being held
    total_num_permits_held: Arc<Mutex<usize>>,
}

#[derive(Debug, thiserror::Error)]
pub enum NamedSemaphoreError {
    /// The global permit limit has been reached
    #[error("global permit limit reached")]
    GlobalLimitReached,

    /// The per-key permit limit has been reached
    #[error("per-key permit limit reached")]
    PerKeyLimitReached,
}

impl<T: Clone + Eq + Hash> NamedSemaphore<T> {
    /// Create a new named semaphore, specifying the maximum number of permits for each key.
    pub fn new(max_permits_per_key: usize, max_total_permits: Option<usize>) -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            max_permits_per_key,
            max_total_permits,
            total_num_permits_held: Arc::new(Mutex::new(0)),
        }
    }

    /// Try to acquire a permit for the given key.
    pub fn try_acquire(&self, key: T) -> Result<NamedSemaphorePermit<T>, NamedSemaphoreError> {
        // Get the permit tracker for the key
        let permit_tracker = self
            .inner
            .entry(key.clone())
            .or_insert_with(|| Arc::new(()));

        // Lock the number of permits held
        let mut total_num_permits_guard = self.total_num_permits_held.lock();

        // If the total number of permits is greater than the maximum number of permits, return None
        if let Some(max_total_permits) = self.max_total_permits {
            if *total_num_permits_guard >= max_total_permits {
                return Err(NamedSemaphoreError::GlobalLimitReached);
            }
        }

        // If the number of permits is greater than or equal to the maximum number of permits, return an error
        if Arc::strong_count(&permit_tracker).saturating_sub(1) >= self.max_permits_per_key {
            return Err(NamedSemaphoreError::PerKeyLimitReached);
        }

        // Increment the total number of permits
        *total_num_permits_guard += 1;

        // Return the new permit
        Ok(NamedSemaphorePermit {
            key,
            parent: self.clone(),
            permit: permit_tracker.clone(),
        })
    }

    /// Get the total number of permits that are currently being held across all keys
    pub fn total_num_permits_held(&self) -> usize {
        *self.total_num_permits_held.lock()
    }
}

pub struct NamedSemaphorePermit<T: Clone + Eq + Hash> {
    /// The key that we are holding a permit for
    key: T,

    /// The parent semaphore that we are borrowing a permit from
    parent: NamedSemaphore<T>,

    /// The permit that we are holding
    permit: Arc<()>,
}

impl<T: Clone + Eq + Hash> Drop for NamedSemaphorePermit<T> {
    fn drop(&mut self) {
        // Decrement the total number of permits
        *self.parent.total_num_permits_held.lock() -= 1;

        // Remove the semaphore but only if there are no more strong references to the parent
        if Arc::strong_count(&self.permit) == 2 {
            self.parent.inner.remove(&self.key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_vec_deque() {
        let mut deque = BoundedVecDeque::new(3);
        deque.push(1);
        deque.push(2);
        deque.push(3);
        deque.push(4);
        deque.push(5);
        assert_eq!(deque.inner.len(), 3);
        assert_eq!(deque.inner, vec![3, 4, 5]);
    }

    #[test]
    fn test_named_semaphore() {
        // Create a new semaphore with a maximum of 1 permit
        let semaphore = NamedSemaphore::new(1, None);

        // Try to acquire a permit for the key "test"
        let permit = semaphore.try_acquire("test");

        // Assert that the permit is Some
        assert!(permit.is_ok());

        // Try to acquire a permit for the key "test" again
        let permit2 = semaphore.try_acquire("test");

        // Assert that the permit is None
        assert!(permit2.is_err());

        // Drop the first permit
        drop(permit);

        // Try to acquire a permit for the key "test" again
        let permit3 = semaphore.try_acquire("test");

        // Assert that the permit is Some
        assert!(permit3.is_ok());

        // Drop permit3
        drop(permit3);

        // Make sure the semaphore is empty
        assert!(semaphore.inner.is_empty());
    }

    #[test]
    fn test_named_semaphore_with_max_total_permits() {
        // Create a new semaphore with a maximum of 1 permit
        let semaphore = NamedSemaphore::new(1, Some(2));

        // Try to acquire a permit for the key "test"
        let permit = semaphore.try_acquire("test");

        // Assert that the permit is Some
        assert!(permit.is_ok());

        // Try to acquire a permit for the key "test2"
        let permit2 = semaphore.try_acquire("test2");

        // Assert that the permit is Some
        assert!(permit2.is_ok());

        // Try to acquire a permit for the key "test3"
        let permit3 = semaphore.try_acquire("test3");

        // Assert that the permit is None
        assert!(permit3.is_err());

        // Drop the first permit
        drop(permit);

        // Try to acquire a permit for the key "test3" again
        let permit4 = semaphore.try_acquire("test3");

        // Assert that the permit is Some
        assert!(permit4.is_ok());

        // Make sure the total number of permits held is 2
        assert_eq!(semaphore.total_num_permits_held(), 2);

        // Drop all permits
        drop(permit2);
        drop(permit3);
        drop(permit4);

        // Make sure the semaphore is empty
        assert!(semaphore.inner.is_empty());

        // Make sure the total number of permits is 0
        assert_eq!(semaphore.total_num_permits_held(), 0);
    }
}
