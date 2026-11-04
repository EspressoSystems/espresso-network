//! What a node watch publishes for the page and the summary line.

use std::{fmt, time::Duration};

use chrono::{DateTime, Utc};
use tokio::sync::watch;

use crate::{
    kind::Height,
    missing::{KindSummary, ListedRows},
    schedule::NodeProgress,
};

/// Rows per node on the missing-data page.
pub const PAGE_ROWS: usize = 200;

/// One node's state, logged each minute and exported as gauges.
pub struct NodeSummary {
    pub tip_height: Height,
    pub follow_lag_blocks: u64,
    /// `None` until the node answers `v1/block-state/block-height`.
    pub merklized_state_lag_blocks: Option<u64>,
    pub kinds: Vec<KindSummary>,
}

/// One line: `tip=.. lag=.. state_lag=.. leaf missing=0 oldest_age=-, block ...`.
impl fmt::Display for NodeSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tip={} lag={} state_lag=",
            self.tip_height.0, self.follow_lag_blocks
        )?;
        match self.merklized_state_lag_blocks {
            Some(lag) => write!(f, "{lag}")?,
            None => f.write_str("-")?,
        }
        for (i, kind) in self.kinds.iter().enumerate() {
            let label: &str = kind.kind.into();
            f.write_str(if i == 0 { " " } else { ", " })?;
            write!(f, "{label} missing={} oldest_age=", kind.missing_count)?;
            match &kind.oldest {
                Some(oldest) => write!(f, "{}", humantime::format_duration(oldest.age))?,
                None => f.write_str("-")?,
            }
        }
        Ok(())
    }
}

/// Time left of the running sweep and when it ends.
#[derive(Clone, Copy, Debug)]
pub struct SweepEta {
    pub remaining: Duration,
    pub finish: DateTime<Utc>,
}

/// What a node watch last published; the page renders it. Ages in `summary` and `listed` are as of
/// the publish.
pub struct NodeView {
    pub summary: NodeSummary,
    pub progress: NodeProgress,
    /// `SweepState::next_start` for the schedule; `None` under `--sweep-once`.
    pub next_sweep_start: Option<DateTime<Utc>>,
    /// `None` while waiting or before a height of the running sweep is done.
    pub sweep_eta: Option<SweepEta>,
    /// `MissingSet::listed(now, PAGE_ROWS)`.
    pub listed: ListedRows,
}

/// Publishing side of one node's view; `None` until the watch starts.
pub type ViewSender = watch::Sender<Option<NodeView>>;
/// Reading side of one node's view.
pub type ViewReceiver = watch::Receiver<Option<NodeView>>;

/// A channel holding no view until the node watch starts.
pub fn channel() -> (ViewSender, ViewReceiver) {
    watch::channel(None)
}
