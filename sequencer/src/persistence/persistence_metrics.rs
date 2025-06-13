use hotshot_types::traits::metrics::{Histogram, Metrics, NoMetrics};

/// Metrics for the persistence layer
#[derive(Clone, Debug)]
pub struct PersistenceMetricsValue {
    /// Time taken by the underlying storage to execute the command that appends a VID
    pub internal_append_vid_duration: Box<dyn Histogram>,
    /// Time taken by the underlying storage to execute the command that appends VID 2
    pub internal_append_vid2_duration: Box<dyn Histogram>,
    /// Time taken by the underlying storage to execute the command that appends DA
    pub internal_append_da_duration: Box<dyn Histogram>,
    /// Time taken by the underlying storage to execute the command that appends DA 2
    pub internal_append_da2_duration: Box<dyn Histogram>,
    /// Time taken by the underlying storage to execute the command that appends Quorum Proposal 2
    pub internal_append_quorum2_duration: Box<dyn Histogram>,
}

impl PersistenceMetricsValue {
    /// Create a new instance of this [`PersistenceMetricsValue`] struct, setting all the counters and gauges
    #[must_use]
    pub fn new(metrics: &dyn Metrics) -> Self {
        Self {
            internal_append_vid_duration: metrics.create_histogram(
                String::from("internal_append_vid_duration"),
                Some("seconds".to_string()),
            ),
            internal_append_vid2_duration: metrics.create_histogram(
                String::from("internal_append_vid2_duration"),
                Some("seconds".to_string()),
            ),
            internal_append_da_duration: metrics.create_histogram(
                String::from("internal_append_da_duration"),
                Some("seconds".to_string()),
            ),
            internal_append_da2_duration: metrics.create_histogram(
                String::from("internal_append_da2_duration"),
                Some("seconds".to_string()),
            ),
            internal_append_quorum2_duration: metrics.create_histogram(
                String::from("internal_append_quorum2_duration"),
                Some("seconds".to_string()),
            ),
        }
    }
}

impl Default for PersistenceMetricsValue {
    fn default() -> Self {
        Self::new(&*NoMetrics::boxed())
    }
}
