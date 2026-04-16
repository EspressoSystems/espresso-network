use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use hotshot_types::{data::Leaf2, traits::node_implementation::NodeType};

use crate::message::Certificate2;

/// A decided leaf paired with the Certificate2 that decided it.
#[derive(Clone, Debug)]
pub struct DecidedLeafEntry<T: NodeType> {
    pub leaf: Leaf2<T>,
    pub cert2: Certificate2<T>,
}

/// Stores only epoch-relevant decided leaves: epoch root blocks and
/// epoch transition blocks (which carry DRB results).
///
/// Each entry includes the [`Certificate2`] that decided the leaf,
/// enabling peers to verify authenticity during catchup.
#[derive(Clone)]
pub struct EpochLeafStore<T: NodeType> {
    inner: Arc<RwLock<BTreeMap<u64, DecidedLeafEntry<T>>>>,
}

impl<T: NodeType> EpochLeafStore<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    /// Insert an epoch-relevant decided leaf with its deciding certificate.
    pub fn insert(&self, leaf: Leaf2<T>, cert2: Certificate2<T>) {
        let height = leaf.height();
        let mut store = self.inner.write().expect("leaf store lock poisoned");
        store.insert(height, DecidedLeafEntry { leaf, cert2 });
    }

    /// Look up a decided leaf entry by block height.
    pub fn get(&self, height: u64) -> Option<DecidedLeafEntry<T>> {
        let store = self.inner.read().expect("leaf store lock poisoned");
        store.get(&height).cloned()
    }

    /// Remove entries with block heights strictly below `min_height`.
    pub fn gc(&self, min_height: u64) {
        let mut store = self.inner.write().expect("leaf store lock poisoned");
        *store = store.split_off(&min_height);
    }
}
