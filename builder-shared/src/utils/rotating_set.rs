use std::{
    collections::HashSet,
    hash::Hash,
    mem,
    time::{Duration, Instant},
};

/// A set that allows for time-based garbage collection,
/// implemented as three sets that are periodically shifted right.
/// Garbage collection is triggered by calling [`Self::rotate`].
#[derive(Clone, Debug)]
pub struct RotatingSet<T>
where
    T: PartialEq + Eq + Hash + Clone,
{
    fresh: HashSet<T>,
    stale: HashSet<T>,
    expiring: HashSet<T>,
    pub last_rotation: Instant,
    pub period: Duration,
}

impl<T> RotatingSet<T>
where
    T: PartialEq + Eq + Hash + Clone,
{
    /// Construct a new `RotatingSet`
    pub fn new(period: Duration) -> Self {
        Self {
            fresh: HashSet::new(),
            stale: HashSet::new(),
            expiring: HashSet::new(),
            last_rotation: Instant::now(),
            period,
        }
    }

    /// Returns `true` if the key is contained in the set
    pub fn contains(&self, key: &T) -> bool {
        self.fresh.contains(key) || self.stale.contains(key) || self.expiring.contains(key)
    }

    /// Insert a `key` into the set. Doesn't trigger garbage collection
    pub fn insert(&mut self, value: T) {
        self.fresh.insert(value);
    }

    /// Force garbage collection, even if the time elapsed since
    ///  the last garbage collection is less than `self.period`
    pub fn force_rotate(&mut self) {
        let now_stale = mem::take(&mut self.fresh);
        let now_expiring = mem::replace(&mut self.stale, now_stale);
        self.expiring = now_expiring;
        self.last_rotation = Instant::now();
    }

    /// Trigger garbage collection.
    pub fn rotate(&mut self) -> bool {
        if self.last_rotation.elapsed() > self.period {
            self.force_rotate();
            true
        } else {
            false
        }
    }
}

impl<T> Extend<T> for RotatingSet<T>
where
    T: PartialEq + Eq + Hash + Clone,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.fresh.extend(iter)
    }
}

#[cfg(test)]
mod tests {
    use std::thread::sleep;

    use tracing_test::traced_test;

    use super::*;

    #[test]
    #[traced_test]
    fn test_insert_and_contains() {
        let mut set = RotatingSet::new(Duration::from_secs(1));
        set.insert(1);
        assert!(set.contains(&1));
        assert!(!set.contains(&2));
    }

    #[test]
    #[traced_test]
    fn test_rotate_and_contains() {
        let mut set = RotatingSet::new(Duration::from_micros(10));
        set.insert(1);

        // Immediately after insertion, item should be in the set
        assert!(set.contains(&1));

        // Wait for longer than the rotation period
        sleep(Duration::from_micros(15));

        // Rotate and check if the item is still in the set
        assert!(set.rotate());
        assert!(set.contains(&1));

        // Shouldn't rotate this time
        assert!(!set.rotate());

        // Wait and rotate again to check if it moves to `expiring`
        sleep(Duration::from_micros(15));
        assert!(set.rotate());
        assert!(set.contains(&1));

        // Wait and rotate again to check if it gets removed
        sleep(Duration::from_micros(15));
        assert!(set.rotate());
        assert!(!set.contains(&1));
    }

    #[test]
    #[traced_test]
    fn test_force_rotate() {
        let mut set = RotatingSet::new(Duration::MAX);
        set.insert(1);
        set.force_rotate();
        assert!(set.contains(&1));
        set.force_rotate();
        assert!(set.contains(&1));
        set.force_rotate();
        assert!(!set.contains(&1));
    }
}
