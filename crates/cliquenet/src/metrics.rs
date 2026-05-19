use crate::x25519::PublicKey;

pub trait Metrics: Send + Sync {
    fn set(&self, key: &PublicKey, label: &str, val: usize);
    fn add(&self, key: &PublicKey, label: &str, val: usize);
    fn del(&self, key: &PublicKey);
}

pub(crate) struct NoMetrics;

impl Metrics for NoMetrics {
    fn set(&self, _: &PublicKey, _: &str, _: usize) {}
    fn add(&self, _: &PublicKey, _: &str, _: usize) {}
    fn del(&self, _: &PublicKey) {}
}
