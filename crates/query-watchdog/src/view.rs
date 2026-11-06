//! What a node watch publishes for the page and the reporter.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    time::Duration,
};

use chrono::{DateTime, Utc};
use tokio::sync::watch;

use crate::{
    kind::{DataKind, Epochs, Height},
    missing::{KindTally, ListedRows},
    node::WatchedNode,
    outcome::NotServed,
    schedule::{Frontier, NodeProgress, SweepState},
    state::Retry,
};

/// Block heights the tip poller last saw per node, and their maximum.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tips {
    max: Height,
    nodes: BTreeMap<String, Height>,
}

impl Tips {
    /// Records the answered `height` of `label`, also a lower one (a node restored from an older
    /// backup is lagging, not missing data). A failed poll records nothing, so it keeps the old
    /// height.
    pub fn observed(&mut self, label: &str, height: Height) {
        self.nodes.insert(label.to_owned(), height);
        self.max = self.nodes.values().copied().max().unwrap_or_default();
    }

    /// The highest height of any node.
    pub fn max(&self) -> Height {
        self.max
    }

    /// The node's own height; `None` until it has answered.
    pub fn node(&self, label: &str) -> Option<Height> {
        self.nodes.get(label).copied()
    }

    /// Blocks the node is behind the highest node.
    pub fn node_lag_blocks(&self, label: &str) -> u64 {
        self.max.blocks_since(self.node(label).unwrap_or_default())
    }
}

/// Rows per node on the missing-data page.
pub const PAGE_ROWS: usize = 200;

/// Page text of when a row was last probed and is probed next, at a `now`.
pub struct TryTimes {
    last_tried: Duration,
    next_try: Option<Duration>,
}

impl TryTimes {
    /// Time since `retry.last_tried_at()`; `next_try` is `None` once `retry.next_at` is not after
    /// `now`.
    pub fn at(retry: &Retry, now: DateTime<Utc>) -> Self {
        Self {
            last_tried: age_at(retry.last_tried_at(), now),
            next_try: (retry.next_at - now).to_std().ok().filter(|d| !d.is_zero()),
        }
    }

    /// `42s ago`.
    pub fn last_tried_text(&self) -> String {
        format!("{} ago", short_duration(self.last_tried))
    }

    /// `in 18s`, or `due`.
    pub fn next_try_text(&self) -> String {
        match self.next_try {
            Some(wait) => format!("in {}", short_duration(wait)),
            None => "due".to_owned(),
        }
    }
}

/// `now - first_missing_at`, 0 when the clock stepped back.
pub fn age_at(first_missing_at: DateTime<Utc>, now: DateTime<Utc>) -> Duration {
    (now - first_missing_at).to_std().unwrap_or(Duration::ZERO)
}

/// The two largest non-zero-leading units of `duration`, whole units only: `9s`, `1m 38s`,
/// `2h 5m`, `6d 23h`; a zero second unit is dropped (`2h`).
pub fn short_duration(duration: Duration) -> String {
    const UNITS: [(u64, &str); 4] = [(86_400, "d"), (3_600, "h"), (60, "m"), (1, "s")];
    let mut left = duration.as_secs();
    let mut parts = Vec::with_capacity(2);
    for (size, name) in UNITS {
        let count = left / size;
        left %= size;
        if parts.is_empty() && count == 0 {
            continue;
        }
        parts.push((count, name));
        if parts.len() == 2 {
            break;
        }
    }
    parts.retain(|(count, _)| *count > 0);
    if parts.is_empty() {
        return "0s".to_owned();
    }
    parts
        .iter()
        .map(|(count, name)| format!("{count}{name}"))
        .collect::<Vec<_>>()
        .join(" ")
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

/// A kind's missing count and oldest age at a `now`.
#[derive(Clone, Debug, PartialEq)]
pub struct KindSummary {
    pub kind: DataKind,
    pub missing_count: u64,
    pub oldest_age: Option<Duration>,
}

/// One node's state at a `now`.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSummary {
    pub tip_height: Height,
    pub follow_lag_blocks: u64,
    /// `None` until the node answers `v1/block-state/block-height`.
    pub merklized_state_lag_blocks: Option<u64>,
    /// `NodeView.not_found_units`; the node's share of the alert total.
    pub not_found_units: u64,
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
    /// The loop's frontier at publish; `not_found_units` judged settled rows by its `settled_end`.
    pub frontier: Frontier,
    pub progress: NodeProgress,
    /// `None` until the node answers `v1/block-state/block-height`.
    pub merklized_state_height: Option<Height>,
    /// The watch's epochs, for `RowKey::first_block` of listed epoch rows.
    pub epochs: Epochs,
    /// `MissingSet::not_found_units(tally_settled_end())` at publish.
    pub not_found_units: u64,
    /// `MissingSet::tallies()`.
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
    /// Lags against `tip`; ages at `now`.
    pub fn summary(&self, tip: Height, now: DateTime<Utc>) -> NodeSummary {
        NodeSummary {
            tip_height: tip,
            follow_lag_blocks: tip.blocks_since(self.progress.follow_next_height),
            merklized_state_lag_blocks: self
                .merklized_state_height
                .map(|height| tip.blocks_since(height)),
            not_found_units: self.not_found_units,
            kinds: self
                .tallies
                .iter()
                .map(|tally| KindSummary {
                    kind: tally.kind,
                    missing_count: tally.missing_count,
                    oldest_age: tally
                        .oldest
                        .as_ref()
                        .map(|row| age_at(row.state.first_missing_at(), now)),
                })
                .collect(),
        }
    }

    /// `frontier.tip() - 1`; `None` at tip 0.
    pub fn newest_block(&self) -> Option<Height> {
        self.frontier.tip().0.checked_sub(1).map(Height)
    }

    /// Heights the page prints with a time: newest block, follow next height, the running sweep's
    /// next and until heights, `first_block` of each listed row.
    pub fn shown_block_heights(&self) -> BTreeSet<Height> {
        let mut heights: BTreeSet<Height> = self.newest_block().into_iter().collect();
        heights.insert(self.progress.follow_next_height);
        if let SweepState::Running { sweep, .. } = &self.progress.sweep {
            heights.extend([sweep.next_height, sweep.until_height]);
        }
        heights.extend(
            self.listed
                .shown
                .iter()
                .filter_map(|row| row.key.first_block(self.epochs)),
        );
        heights
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

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::{
        kind::{Epoch, EpochConfig, EpochKind, PointKind, RangeKind},
        missing::{MissingRow, RowKey},
        schedule::{RunningSweep, SweepWindow},
        state::{RetryDelay, UnitState},
    };

    fn row(key: RowKey) -> MissingRow {
        let at = DateTime::from_timestamp(0, 0).unwrap();
        MissingRow {
            key,
            state: UnitState::NotFound {
                first_missing_at: at,
                not_found_probes: NonZeroU32::new(1).unwrap(),
                retry: Retry::after(RetryDelay::from_secs(10).unwrap(), at),
            },
        }
    }

    fn view(tip: u64, sweep: SweepState, rows: Vec<MissingRow>, epochs: Epochs) -> NodeView {
        NodeView {
            published_at: DateTime::from_timestamp(0, 0).unwrap(),
            frontier: Frontier::new(Height(tip), SweepWindow::Archive),
            progress: NodeProgress {
                follow_next_height: Height(90),
                sweep,
            },
            merklized_state_height: None,
            epochs,
            not_found_units: 0,
            tallies: Vec::new(),
            listed: ListedRows {
                total: rows.len() as u64,
                shown: rows,
            },
            counters: WatchCounters::default(),
            next_sweep_start: None,
            sweep_eta: None,
        }
    }

    fn rows() -> Vec<MissingRow> {
        vec![
            row(RowKey::Range {
                kind: RangeKind::Leaf,
                heights: Height(10)..Height(20),
            }),
            row(RowKey::Point {
                kind: PointKind::LcLeafProof,
                height: Height(30),
            }),
            row(RowKey::Epoch {
                kind: EpochKind::StakeTable,
                epoch: Epoch(4),
            }),
        ]
    }

    #[test]
    fn shown_block_heights_cover_tip_follow_sweep_and_rows() {
        let at = DateTime::from_timestamp(0, 0).unwrap();
        let sweep = SweepState::Running {
            sweep: RunningSweep {
                started_at: at,
                start_height: Height(0),
                next_height: Height(50),
                until_height: Height(80),
            },
            last_completed_at: None,
        };
        let epochs = Epochs::On(EpochConfig {
            epoch_height: std::num::NonZeroU64::new(10).unwrap(),
            epoch_start_block: Height(0),
        });
        let heights = view(100, sweep, rows(), epochs).shown_block_heights();
        // Stake table epoch 4 has root block (4 - 2) * 10.
        let expected = [99, 90, 50, 80, 10, 30, 20]
            .map(Height)
            .into_iter()
            .collect();
        assert_eq!(heights, expected);

        let off = view(100, SweepState::NeverSwept, rows(), Epochs::Off).shown_block_heights();
        assert_eq!(off, [99, 90, 10, 30].map(Height).into_iter().collect());

        let at_zero = view(0, SweepState::NeverSwept, Vec::new(), Epochs::Off);
        assert_eq!(at_zero.newest_block(), None);
        assert_eq!(at_zero.shown_block_heights(), [Height(90)].into());
    }

    #[test]
    fn try_times_read_due_once_next_at_is_not_after_now() {
        let start = DateTime::from_timestamp(1_000, 0).unwrap();
        let retry = Retry::after(RetryDelay::from_secs(18).unwrap(), start);
        let at = |secs| start + chrono::TimeDelta::seconds(secs);
        let waiting = TryTimes::at(&retry, at(6));
        assert_eq!(waiting.last_tried_text(), "6s ago");
        assert_eq!(waiting.next_try_text(), "in 12s");
        assert_eq!(TryTimes::at(&retry, at(18)).next_try_text(), "due");
        assert_eq!(TryTimes::at(&retry, at(40)).next_try_text(), "due");
    }

    #[test]
    fn short_duration_keeps_two_units() {
        let cases = [
            (0, "0s"),
            (9, "9s"),
            (60, "1m"),
            (98, "1m 38s"),
            (7_500, "2h 5m"),
            (7_200, "2h"),
            (7_205, "2h"),
            (3_600 * 24 * 7 - 100, "6d 23h"),
            (86_400 * 3 + 59, "3d"),
        ];
        for (secs, text) in cases {
            assert_eq!(short_duration(Duration::from_secs(secs)), text, "{secs}");
        }
    }

    #[test]
    fn tips_follow_answered_heights_only() {
        let mut tips = Tips::default();
        assert_eq!(tips.node("a"), None);
        tips.observed("a", Height(10));
        tips.observed("b", Height(7));
        assert_eq!((tips.max(), tips.node_lag_blocks("b")), (Height(10), 3));
        tips.observed("a", Height(4));
        assert_eq!(
            (tips.node("a"), tips.node("b")),
            (Some(Height(4)), Some(Height(7)))
        );
        assert_eq!(tips.max(), Height(7));
        assert_eq!(tips.node_lag_blocks("unknown"), 7);
    }
}
