use std::sync::Arc;

use crate::x25519::PublicKey;

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

/// Records a query link's metrics under `query_` labels, so they do not
/// overwrite the consensus link's metrics of the same party.
pub(crate) struct QueryMetrics(pub(crate) Arc<dyn Metrics>);

impl Metrics for QueryMetrics {
    fn set(&self, key: &PublicKey, label: &str, val: usize) {
        self.0.set(key, &format!("query_{label}"), val)
    }

    fn add(&self, key: &PublicKey, label: &str, val: usize) {
        self.0.add(key, &format!("query_{label}"), val)
    }

    // Party metrics are removed per party, which covers both links.
    fn del(&self, _: &PublicKey) {}
}
