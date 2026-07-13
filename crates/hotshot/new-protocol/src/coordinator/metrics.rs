use std::{sync::Arc, time::Instant};

use hotshot_types::{consensus::ConsensusMetricsValue, traits::metrics::Histogram};

pub struct Metrics {
    pub(crate) consensus: ConsensusMetricsValue,
}

impl Metrics {
    pub fn new(consensus: ConsensusMetricsValue) -> Self {
        Self { consensus }
    }
}

pub struct Measurement {
    hist: Arc<dyn Histogram>,
    start: Instant,
    record: bool,
}

impl Measurement {
    pub fn start(h: Arc<dyn Histogram>) -> Self {
        Self {
            hist: h,
            start: Instant::now(),
            record: true,
        }
    }
}

impl Drop for Measurement {
    fn drop(&mut self) {
        if self.record {
            self.hist.add_point(self.start.elapsed().as_secs_f64());
        }
    }
}

pub fn finish_measurement(_: Option<Measurement>) {}

pub fn ignore_measurement(m: Option<Measurement>) {
    if let Some(mut m) = m {
        m.record = false;
    }
}
