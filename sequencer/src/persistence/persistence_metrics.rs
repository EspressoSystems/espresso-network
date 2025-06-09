use hotshot_types::traits::metrics::{Histogram, Metrics, NoMetrics};

/// Metrics for the persistence layer
#[derive(Clone, Debug)]
pub struct PersistenceMetricsValue {
    /// Time it takes to append a vid
    pub append_vid_duration: Box<dyn Histogram>,
    /// Time it takes to append a vid2
    pub append_vid2_duration: Box<dyn Histogram>,
}

impl PersistenceMetricsValue {
    /// Create a new instance of this [`PersistenceMetricsValue`] struct, setting all the counters and gauges
    #[must_use]
    pub fn new(metrics: &dyn Metrics) -> Self {
        Self {
            append_vid_duration: metrics.create_histogram(
                String::from("append_vid_duration"),
                Some("seconds".to_string()),
            ),
            append_vid2_duration: metrics.create_histogram(
                String::from("append_vid2_duration"),
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
