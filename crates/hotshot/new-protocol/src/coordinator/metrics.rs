use std::{sync::Arc, time::Instant};

use hotshot_types::traits::metrics::{Counter, Histogram, Metrics as HotshotMetrics};

pub struct Metrics {
    pub(crate) timeouts: Box<dyn Counter>,
    pub(crate) apply_consensus: Arc<dyn Histogram>,
    pub(crate) next_consensus_input: Arc<dyn Histogram>,
    pub(crate) process_consensus_output: Arc<dyn Histogram>,
    pub(crate) on_network_message: Arc<dyn Histogram>,
    pub(crate) on_state_manager_output: Arc<dyn Histogram>,
    pub(crate) on_proposal_and_vid_share: Arc<dyn Histogram>,
    pub(crate) on_client_request: Arc<dyn Histogram>,
}

impl Metrics {
    pub fn new(m: &dyn HotshotMetrics) -> Self {
        Self {
            timeouts: m.create_counter("coordinator_timeouts".into(), None),
            apply_consensus: m
                .create_histogram("coordinator_apply_consensus".into(), Some("s".into()))
                .into(),
            next_consensus_input: m
                .create_histogram("coordinator_next_consensus_input".into(), Some("s".into()))
                .into(),
            process_consensus_output: m
                .create_histogram(
                    "coordinator_process_consensus_output".into(),
                    Some("s".into()),
                )
                .into(),
            on_network_message: m
                .create_histogram("coordinator_on_network_message".into(), Some("s".into()))
                .into(),
            on_state_manager_output: m
                .create_histogram(
                    "coordinator_on_state_manager_output".into(),
                    Some("s".into()),
                )
                .into(),
            on_proposal_and_vid_share: m
                .create_histogram(
                    "coordinator_on_proposal_and_vid_share".into(),
                    Some("s".into()),
                )
                .into(),
            on_client_request: m
                .create_histogram("coordinator_on_client_request".into(), Some("s".into()))
                .into(),
        }
    }
}

pub struct Measurement {
    hist: Arc<dyn Histogram>,
    start: Instant,
}

impl Measurement {
    pub fn start(h: Arc<dyn Histogram>) -> Self {
        Self {
            hist: h,
            start: Instant::now(),
        }
    }
}

impl Drop for Measurement {
    fn drop(&mut self) {
        self.hist.add_point(self.start.elapsed().as_secs_f64());
    }
}

pub fn finish_measurement(_: Option<Measurement>) {}
