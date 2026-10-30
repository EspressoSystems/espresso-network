use std::sync::Arc;

use crate::{LaneId, x25519::PublicKey};

/// Type that records metrics.
pub trait Metrics: Send + Sync {
    /// Set a peer gauge to the given value.
    fn set(&self, key: &PublicKey, label: &str, val: usize);

    /// Add to a peer counter the given value.
    fn add(&self, key: &PublicKey, label: &str, val: usize);

    /// Remove all peer metrics.
    fn del(&self, key: &PublicKey);
}

/// A no-op [`Metrics`] implementation.
pub(crate) struct NoMetrics;

impl Metrics for NoMetrics {
    fn set(&self, _: &PublicKey, _: &str, _: usize) {}
    fn add(&self, _: &PublicKey, _: &str, _: usize) {}
    fn del(&self, _: &PublicKey) {}
}

/// Records a lane's metrics under `lane{id}_` labels, so they do not
/// overwrite the main connection's metrics of the same party.
pub(crate) struct LaneMetrics {
    inner: Arc<dyn Metrics>,
    prefix: String,
}

impl LaneMetrics {
    pub(crate) fn new(inner: Arc<dyn Metrics>, id: LaneId) -> Self {
        Self {
            inner,
            prefix: format!("lane{id}_"),
        }
    }
}

impl Metrics for LaneMetrics {
    fn set(&self, key: &PublicKey, label: &str, val: usize) {
        self.inner.set(key, &format!("{}{label}", self.prefix), val)
    }

    fn add(&self, key: &PublicKey, label: &str, val: usize) {
        self.inner.add(key, &format!("{}{label}", self.prefix), val)
    }

    // Party metrics are removed per party, which covers every lane.
    fn del(&self, _: &PublicKey) {}
}
