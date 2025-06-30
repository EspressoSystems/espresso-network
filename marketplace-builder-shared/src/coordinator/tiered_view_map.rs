//! This module contains an optimized implementation of a
//! composite key two-tier map, where the first key is a view
//! number

use std::{
    collections::{BTreeMap, btree_map::Entry},
    hash::Hash,
    ops::RangeBounds,
};

use hotshot_types::{
    data::VidCommitment,
    traits::node_implementation::{ConsensusTime, NodeType},
    utils::BuilderCommitment,
};
use nonempty_collections::{NEMap, nem};

use crate::block::{BlockId, BuilderStateId};

/// A map from [`ViewCompositeKey`] to arbitrary value, implemented as a tiered map
/// with the first tier being [`BTreeMap`] keyed by view number of [`ViewCompositeKey`]
/// and the second [`NEMap`] keyed by subkey of [`ViewCompositeKey`].
///
/// Usage of [`BTreeMap`] means that the map has an convenient property of always being
/// sorted by view number, which makes common operations such as pruning old views more efficient.
///
/// Second tier being non-empty by construction [`NEMap`] ensures that we can't accidentally
/// create phantom entries with empty maps in the first tier.
#[derive(Debug)]
pub struct TieredViewMap<K, V>(BTreeMap<K::View, NEMap<K::Subkey, V>>)
where
    K: ViewCompositeKey;

/// A two-component key, of which one component is [`ConsensusTime`]
///
/// See [`TieredViewMap`] documentation for more information
pub trait ViewCompositeKey {
    type Subkey: Hash + Eq;
    type View: ConsensusTime;
    fn view(&self) -> &Self::View;
    fn subkey(&self) -> &Self::Subkey;
    fn into_subkey(self) -> Self::Subkey;
}

impl<Types: NodeType> ViewCompositeKey for BlockId<Types> {
    type Subkey = BuilderCommitment;
    type View = Types::View;

    fn view(&self) -> &<Types as NodeType>::View {
        &self.view
    }

    fn subkey(&self) -> &Self::Subkey {
        &self.hash
    }

    fn into_subkey(self) -> Self::Subkey {
        self.hash
    }
}

impl<Types: NodeType> ViewCompositeKey for BuilderStateId<Types> {
    type Subkey = VidCommitment;
    type View = Types::View;

    fn view(&self) -> &<Types as NodeType>::View {
        &self.parent_view
    }

    fn subkey(&self) -> &Self::Subkey {
        &self.parent_commitment
    }

    fn into_subkey(self) -> Self::Subkey {
        self.parent_commitment
    }
}

impl<K, V> TieredViewMap<K, V>
where
    K: ViewCompositeKey,
{
    /// Create a new empty map
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Returns an iterator visiting all values in this map
    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.0
            .values()
            .flat_map(|bucket| bucket.values().into_iter())
    }

    /// Returns a nested iterator visiting all values for view numbers in given range
    pub fn range<R>(&self, range: R) -> impl Iterator<Item = impl Iterator<Item = &V>>
    where
        R: RangeBounds<K::View>,
    {
        self.0
            .range(range)
            .map(|(_, bucket)| bucket.values().into_iter())
    }

    /// Returns an iterator visiting all values for given view number
    pub fn bucket(&self, view_number: &K::View) -> impl Iterator<Item = &V> {
        self.0
            .get(view_number)
            .into_iter()
            .flat_map(|bucket| bucket.values().into_iter())
    }

    /// Returns the number of entries in this map
    pub fn len(&self) -> usize {
        self.0.values().map(|bucket| bucket.len().get()).sum()
    }

    /// Returns whether this map is empty
    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }

    /// Get reference to a value by key
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.get(key.view())?.get(key.subkey())
    }

    /// Get mutable reference to a value by key
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.0.get_mut(key.view())?.get_mut(key.subkey())
    }

    /// Get highest view value (no guarantees as to which one exactly
    /// if there's multiple stored)
    ///
    /// Returns `None` if the map is empty
    pub fn highest_view_builder(&self) -> Option<&V> {
        Some(&self.0.last_key_value()?.1.head_val)
    }

    /// Insert a new value
    pub fn insert(&mut self, key: K, value: V) {
        match self.0.entry(*key.view()) {
            Entry::Vacant(entry) => {
                entry.insert(nem![key.into_subkey() => value]);
            },
            Entry::Occupied(mut entry) => {
                entry.get_mut().insert(key.into_subkey(), value);
            },
        }
    }

    /// Returns highest view number for which we have a value
    pub fn highest_view(&self) -> Option<K::View> {
        Some(*self.0.last_key_value()?.0)
    }

    /// Returns lowest view number for which we have a value
    pub fn lowest_view(&self) -> Option<K::View> {
        Some(*self.0.first_key_value()?.0)
    }

    /// Removes every view lower than the `cutoff_view` (exclusive) from self and returns all removed views.
    pub fn prune(&mut self, cutoff_view: K::View) -> Self {
        let high = self.0.split_off(&cutoff_view);
        let low = std::mem::replace(&mut self.0, high);
        Self(low)
    }
}

impl<K, V> Default for TieredViewMap<K, V>
where
    K: ViewCompositeKey,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{cmp::Ordering, ops::Bound, sync::Arc};

    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{data::ViewNumber, traits::node_implementation::ConsensusTime};
    use rand::{Rng, distributions::Standard, thread_rng};
    use tracing_test::traced_test;

    use super::*;
    use crate::{state::BuilderState, testing::mock};

    type View = ViewNumber;
    type BuilderStateMap =
        super::TieredViewMap<BuilderStateId<TestTypes>, Arc<BuilderState<TestTypes>>>;

    #[test]
    #[traced_test]
    fn test_new_map() {
        let new_map = BuilderStateMap::new();
        assert!(new_map.is_empty());
        assert_eq!(new_map.len(), 0);

        let default_map = BuilderStateMap::default();
        assert!(default_map.is_empty());
        assert_eq!(default_map.len(), 0);
    }

    #[test]
    #[traced_test]
    fn test_insert_and_get() {
        let mut map = BuilderStateMap::new();

        let builder_state = mock::builder_state(1);
        let state_id = builder_state.id();

        map.insert(state_id.clone(), Arc::clone(&builder_state));

        assert!(!map.is_empty());
        assert_eq!(map.len(), 1);
        assert!(Arc::ptr_eq(map.get(&state_id).unwrap(), &builder_state));
    }

    #[test]
    #[traced_test]
    fn test_range_iteration() {
        let mut map = BuilderStateMap::new();

        for i in 0..5 {
            let builder_state = mock::builder_state(i);
            map.insert(builder_state.id(), builder_state);
        }

        let start = View::new(1);
        let end = View::new(3);

        let collected: Vec<_> = map
            .range((Bound::Included(start), Bound::Excluded(end)))
            .flatten()
            .collect();

        assert_eq!(collected.len(), 2);
        assert_eq!(*collected[0].parent_block_references.view_number, 1);
        assert_eq!(*collected[1].parent_block_references.view_number, 2);
    }

    #[test]
    #[traced_test]
    fn test_pruning() {
        let view_count = 11;
        let states_per_view = 13;
        let cutoff = 7;

        let mut map = BuilderStateMap::new();

        for view in 0..view_count {
            for _ in 0..states_per_view {
                let state = mock::builder_state(view);
                map.insert(state.id(), state);
            }
        }

        let pruned_map = map.prune(View::new(cutoff));
        assert_eq!(pruned_map.len() as u64, cutoff * states_per_view);
        assert_eq!(map.len() as u64, (view_count - cutoff) * states_per_view);

        assert!(pruned_map.bucket(&View::new(cutoff - 1)).next().is_some());
        assert!(map.bucket(&View::new(cutoff)).next().is_some());

        assert!(pruned_map.bucket(&View::new(cutoff)).next().is_none());
        assert!(map.bucket(&View::new(cutoff - 1)).next().is_none());
    }

    #[test]
    #[traced_test]
    fn test_highest_and_lowest_view() {
        let mut map = BuilderStateMap::new();
        assert_eq!(map.highest_view(), None);
        assert_eq!(map.lowest_view(), None);

        for i in 3..13 {
            let state = mock::builder_state(i);
            map.insert(state.id(), state);
        }

        assert_eq!(*map.highest_view().unwrap(), 12);
        assert_eq!(*map.lowest_view().unwrap(), 3);
    }

    #[test]
    #[traced_test]
    fn test_highest_view_builder() {
        let mut map = BuilderStateMap::new();
        assert!(map.highest_view_builder().is_none());

        let mut states: Vec<_> = thread_rng()
            .sample_iter(Standard)
            .take(100)
            .map(mock::builder_state)
            .collect();

        for state in states.iter() {
            map.insert(state.id(), Arc::clone(state));
        }

        states.sort_by(|a, b| {
            a.parent_block_references
                .view_number
                .cmp(&b.parent_block_references.view_number)
        });

        assert_eq!(
            map.highest_view_builder()
                .unwrap()
                .parent_block_references
                .view_number,
            states.last().unwrap().parent_block_references.view_number
        );
    }

    #[test]
    #[traced_test]
    fn test_iterator() {
        let mut map = BuilderStateMap::new();

        let mut states: Vec<_> = thread_rng()
            .sample_iter(Standard)
            .take(100)
            .map(mock::builder_state)
            .collect();
        assert_eq!(states.len(), 100);

        for state in states.iter() {
            map.insert(state.id(), Arc::clone(state));
        }

        states.sort_by(compare_builders);

        let mut map_states = map.values().cloned().collect::<Vec<_>>();
        map_states.sort_by(compare_builders);

        for (original_state, map_state) in states.into_iter().zip(map_states) {
            assert!(Arc::ptr_eq(&original_state, &map_state));
        }
    }

    fn compare_builders(
        a: &Arc<BuilderState<TestTypes>>,
        b: &Arc<BuilderState<TestTypes>>,
    ) -> Ordering {
        a.parent_block_references
            .view_number
            .cmp(&b.parent_block_references.view_number)
            .then(
                a.parent_block_references
                    .vid_commitment
                    .cmp(&b.parent_block_references.vid_commitment),
            )
    }
}
