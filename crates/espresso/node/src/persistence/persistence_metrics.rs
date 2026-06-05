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
    /// Decide events emitted without a block payload (grace period expired and recovery
    /// failed); the query service is left with a leaf-only block for this height
    pub decide_missing_payload: Box<dyn Counter>,
    /// Decide events emitted without VID data (grace period expired)
    pub decide_missing_vid: Box<dyn Counter>,
    /// Block payloads filled into decide events from the in-memory decide data, without
    /// touching consensus storage (may count a view more than once across retry passes)
    pub decide_payload_from_memory: Box<dyn Counter>,
    /// VID shares filled into decide events from the in-memory decide data, without
    /// touching consensus storage (may count a view more than once across retry passes)
    pub decide_vid_from_memory: Box<dyn Counter>,
    /// Block payloads successfully recovered from peers by the decide processor
    pub payloads_recovered: Box<dyn Counter>,
    /// Failed peer-recovery attempts for block payloads
    pub payload_recovery_failures: Box<dyn Counter>,
    /// Times decide event generation stopped at a non-consecutive leaf (a height gap in
    /// consensus storage; if it persists, the decide pipeline is stalled)
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
            payloads_recovered: metrics.create_counter(String::from("payloads_recovered"), None),
            payload_recovery_failures: metrics
                .create_counter(String::from("payload_recovery_failures"), None),
            decide_height_gaps: metrics.create_counter(String::from("decide_height_gaps"), None),
        }
    }
}

impl Default for PersistenceMetricsValue {
    fn default() -> Self {
        Self::new(&*NoMetrics::boxed())
    }
}
