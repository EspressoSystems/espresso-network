use crate::traits::metrics::{Histogram, Metrics, NoMetrics};

/// Storage metrics
#[derive(Clone, Debug)]
pub struct StorageMetricsValue {
    /// Time taken by the storage to append a VID
    pub append_vid_duration: Box<dyn Histogram>,
    /// Time taken by the storage to append DA
    pub append_da_duration: Box<dyn Histogram>,
    /// Time taken by the storage to append Quorum Proposal
    pub append_quorum_duration: Box<dyn Histogram>,
}

impl StorageMetricsValue {
    /// Create a new instance of this [`StorageMetricsValue`] struct, setting all the counters and gauges
    #[must_use]
    pub fn new(metrics: &dyn Metrics) -> Self {
        Self {
            append_vid_duration: metrics.create_histogram(
                String::from("append_vid_duration"),
                Some("seconds".to_string()),
            ),
            append_da_duration: metrics.create_histogram(
                String::from("append_da_duration"),
                Some("seconds".to_string()),
            ),
            append_quorum_duration: metrics.create_histogram(
                String::from("append_quorum_duration"),
                Some("seconds".to_string()),
            ),
        }
    }
}

impl Default for StorageMetricsValue {
    fn default() -> Self {
        Self::new(&*NoMetrics::boxed())
    }
}
