//! What a node watch publishes for the page and the reporter.

use std::{collections::BTreeMap, fmt, time::Duration};

use chrono::{DateTime, Utc};
use tokio::sync::watch;

use crate::{
    kind::{DataKind, Height},
    missing::{AlertRule, KindTally, ListedRows},
    node::WatchedNode,
    outcome::NotServed,
    schedule::{Frontier, NodeProgress},
};

/// Rows per node on the missing-data page.
pub const PAGE_ROWS: usize = 200;

/// `now - first_missing_at`, 0 when the clock stepped back.
pub fn age_at(first_missing_at: DateTime<Utc>, now: DateTime<Utc>) -> Duration {
    (now - first_missing_at).to_std().unwrap_or(Duration::ZERO)
}

/// Requests without 2xx per `NotServed`; `Debug` prints a label map, as HEAD's `errors` field.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct ErrorCounts(BTreeMap<NotServed, u64>);

impl ErrorCounts {
    /// One more `not_served`.
    pub fn count(&mut self, not_served: NotServed) {
        *self.0.entry(not_served).or_default() += 1;
    }

    /// The entries other than `NotFound`; non-empty means `node unhealthy`.
    pub fn failures(&self) -> ErrorCounts {
        Self(
            self.0
                .iter()
                .filter(|(not_served, _)| **not_served != NotServed::NotFound)
                .map(|(not_served, count)| (*not_served, *count))
                .collect(),
        )
    }

    /// No entry.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for ErrorCounts {
    /// `{"404": 1, "500": 2}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.0
                    .iter()
                    .map(|(not_served, count)| (not_served.label(), count)),
            )
            .finish()
    }
}

/// Counters of one node watch, cumulative; the reporter logs differences.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WatchCounters {
    pub errors: ErrorCounts,
    pub forgotten_rows: u64,
}

impl WatchCounters {
    /// `self - earlier`, per key.
    pub fn since(&self, earlier: &WatchCounters) -> WatchCounters {
        let errors = self
            .errors
            .0
            .iter()
            .filter_map(|(not_served, count)| {
                let before = earlier.errors.0.get(not_served).copied().unwrap_or(0);
                let difference = count.saturating_sub(before);
                (difference > 0).then_some((*not_served, difference))
            })
            .collect();
        WatchCounters {
            errors: ErrorCounts(errors),
            forgotten_rows: self.forgotten_rows.saturating_sub(earlier.forgotten_rows),
        }
    }
}

/// A kind's missing and confirmed counts and oldest age at a `now`.
#[derive(Clone, Debug, PartialEq)]
pub struct KindSummary {
    pub kind: DataKind,
    pub missing_count: u64,
    pub confirmed_missing: u64,
    pub oldest_age: Option<Duration>,
}

/// One node's state at a `now`.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSummary {
    pub tip_height: Height,
    pub follow_lag_blocks: u64,
    /// `None` until the node answers `v1/block-state/block-height`.
    pub merklized_state_lag_blocks: Option<u64>,
    pub kinds: Vec<KindSummary>,
}

impl NodeSummary {
    /// Sum of `confirmed_missing` over kinds (heights and epochs): the `confirmed_missing{node}`
    /// gauge.
    pub fn confirmed_missing(&self) -> u64 {
        self.kinds.iter().map(|kind| kind.confirmed_missing).sum()
    }
}

/// One line: `tip=.. lag=.. state_lag=.. leaf missing=0 confirmed=0 oldest_age=-, block ...`.
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
            write!(
                f,
                "{label} missing={} confirmed={} oldest_age=",
                kind.missing_count, kind.confirmed_missing
            )?;
            match kind.oldest_age {
                Some(age) => write!(f, "{}", humantime::format_duration(age))?,
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

/// What a node watch last published; time-free apart from `published_at`, so readers take ages at
/// their own `now`.
#[derive(Clone, Debug)]
pub struct NodeView {
    pub published_at: DateTime<Utc>,
    /// The loop's frontier at publish; `tallies` judged settled rows by its `settled_end`.
    pub frontier: Frontier,
    pub progress: NodeProgress,
    /// `None` until the node answers `v1/block-state/block-height`.
    pub merklized_state_height: Option<Height>,
    /// The options' rule; `tallies` split confirmed units at `alert_rule.counted_until(published_at)`.
    pub alert_rule: AlertRule,
    /// `MissingSet::tallies(frontier.settled_end(), alert_rule.counted_until(published_at))`.
    pub tallies: Vec<KindTally>,
    /// `MissingSet::listed(PAGE_ROWS)`.
    pub listed: ListedRows,
    /// Cumulative since the watch started.
    pub counters: WatchCounters,
    /// `SweepState::next_start`; `None` under `--sweep-once`.
    pub next_sweep_start: Option<DateTime<Utc>>,
    /// `None` while waiting or before a height of the running sweep is done.
    pub sweep_eta: Option<SweepEta>,
}

impl NodeView {
    /// Lags against `tip`; ages and confirmed counts at `now`.
    pub fn summary(&self, tip: Height, now: DateTime<Utc>) -> NodeSummary {
        let counted_until = self.alert_rule.counted_until(now);
        NodeSummary {
            tip_height: tip,
            follow_lag_blocks: tip.blocks_since(self.progress.follow_next_height),
            merklized_state_lag_blocks: self
                .merklized_state_height
                .map(|height| tip.blocks_since(height)),
            kinds: self
                .tallies
                .iter()
                .map(|tally| KindSummary {
                    kind: tally.kind,
                    missing_count: tally.missing_count,
                    confirmed_missing: tally.confirmed.count(counted_until),
                    oldest_age: tally
                        .oldest
                        .as_ref()
                        .map(|row| age_at(row.state.first_missing_at(), now)),
                })
                .collect(),
        }
    }
}

/// Publishing side of one node's view; `None` until the watch starts.
pub type ViewSender = watch::Sender<Option<NodeView>>;
/// Reading side of one node's view.
pub type ViewReceiver = watch::Receiver<Option<NodeView>>;

/// A channel holding no view until the node watch starts.
pub fn channel() -> (ViewSender, ViewReceiver) {
    watch::channel(None)
}

/// One watched node and the receiver of its view.
#[derive(Clone)]
pub struct NodeViewSlot {
    pub node: WatchedNode,
    pub view: ViewReceiver,
}
