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
