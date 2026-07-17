use std::{
    collections::{VecDeque, vec_deque},
    mem,
};

#[derive(Debug, Clone)]
pub struct Outbox<T>(VecDeque<T>);

impl<T> Default for Outbox<T> {
    fn default() -> Self {
        Self(VecDeque::new())
    }
}

impl<T> Outbox<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn push_back<U: Into<T>>(&mut self, item: U) {
        self.0.push_back(item.into())
    }

    pub fn pop_front(&mut self) -> Option<T> {
        self.0.pop_front()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.0.iter()
    }

    pub fn take(&mut self) -> Self {
        Self(mem::take(&mut self.0))
    }
}

impl<T> IntoIterator for Outbox<T> {
    type IntoIter = vec_deque::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Outbox<T> {
    type IntoIter = vec_deque::Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T> Extend<T> for Outbox<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter)
    }
}

impl<T: PartialEq> Outbox<T> {
    pub fn contains(&self, item: &T) -> bool {
        self.0.contains(item)
    }
}
