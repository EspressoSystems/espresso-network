use std::{collections::BTreeMap, iter::once};

/// Small helper to ensure some container type is not empty.
#[derive(Debug)]
pub(crate) struct NonEmpty<T: Container>(T::Element, T);

pub(crate) trait Container {
    type Element;
}

impl<A> Container for Vec<A> {
    type Element = A;
}

impl<K: Ord, V> Container for BTreeMap<K, V> {
    type Element = (K, V);
}

impl<A> NonEmpty<Vec<A>> {
    pub(crate) fn new<I>(fst: A, rest: I) -> Self
    where
        I: IntoIterator<Item = A>,
    {
        Self(fst, rest.into_iter().collect())
    }

    pub(crate) fn assert_non_empty_vec<I>(it: I) -> Self
    where
        I: IntoIterator<Item = A>,
    {
        let mut it = it.into_iter();
        let fst = it.next().expect("non empty vector");
        Self(fst, it.collect())
    }

    pub(crate) fn last(&self) -> &A {
        self.1.as_slice().last().unwrap_or(&self.0)
    }

    pub(crate) fn get(&self, i: usize) -> Option<&A> {
        match i {
            0 => Some(&self.0),
            _ => self.1.get(i - 1),
        }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &A> {
        once(&self.0).chain(self.1.as_slice().iter())
    }
}

impl<K: Ord, V> NonEmpty<BTreeMap<K, V>> {
    pub(crate) fn assert_non_empty_map<I>(it: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
    {
        let mut m = BTreeMap::from_iter(it);
        let fst = m.pop_first().expect("non empty map");
        Self(fst, m)
    }

    pub(crate) fn first(&self) -> (&K, &V) {
        (&self.0.0, &self.0.1)
    }

    pub(crate) fn last(&self) -> (&K, &V) {
        self.1.last_key_value().unwrap_or((&self.0.0, &self.0.1))
    }

    pub(crate) fn get(&self, k: &K) -> Option<&V> {
        (&self.0.0 == k)
            .then_some(&self.0.1)
            .or_else(|| self.1.get(k))
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        once((&self.0.0, &self.0.1)).chain(self.1.iter())
    }
}
