use hotshot_types::traits::metrics::{Counter, Histogram, Metrics, NoMetrics};

/// Metrics for the persistence layer
#[derive(Clone, Debug)]
pub struct PersistenceMetricsValue {
    /// Time taken by the underlying storage to execute the command that appends a VID
    pub internal_append_vid_duration: Box<dyn Histogram>,
    /// Time taken by the underlying storage to execute the command that appends DA
    pub internal_append_da_duration: Box<dyn Histogram>,
    /// Time taken by the underlying storage to execute the command that appends DA 2
    pub internal_append_da2_duration: Box<dyn Histogram>,
    /// Time taken by the underlying storage to execute the command that appends Quorum Proposal 2
    pub internal_append_quorum2_duration: Box<dyn Histogram>,
    /// Decide events emitted without a block payload (leaf reported for background peer recovery)
    pub decide_missing_payload: Box<dyn Counter>,
    /// Decide events emitted without VID data; healed by the query service's peer fetching
    pub decide_missing_vid: Box<dyn Counter>,
    /// Block payloads filled from in-memory decide data (may double-count across retries)
    pub decide_payload_from_memory: Box<dyn Counter>,
    /// VID shares filled from in-memory decide data (may double-count across retries)
    pub decide_vid_from_memory: Box<dyn Counter>,
    /// Height gaps hit during decide event generation (a missing decided leaf; investigate if
    /// recurring)
    pub decide_height_gaps: Box<dyn Counter>,
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
            decide_missing_payload: metrics
                .create_counter(String::from("decide_missing_payload"), None),
            decide_missing_vid: metrics.create_counter(String::from("decide_missing_vid"), None),
            decide_payload_from_memory: metrics
                .create_counter(String::from("decide_payload_from_memory"), None),
            decide_vid_from_memory: metrics
                .create_counter(String::from("decide_vid_from_memory"), None),
            decide_height_gaps: metrics.create_counter(String::from("decide_height_gaps"), None),
        }
    }
}

impl Default for PersistenceMetricsValue {
    fn default() -> Self {
        Self::new(&*NoMetrics::boxed())
    }
}
