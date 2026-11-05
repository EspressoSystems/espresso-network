//! Heights derived from the tip, the sweep schedule and the pacing state of a node watch.

use std::{ops::Range, time::Duration};

use chrono::{DateTime, TimeDelta, Utc};
use tokio::time::Instant;

use crate::{
    kind::{CHUNK_LEN, Height, next_chunk},
    outcome::RequestHealth,
};

/// Blocks below the tip within which follow does not probe and missing data does not alert.
const ALERT_TIP_MARGIN: u64 = 10;
/// Time between chunks a paused sweep sends to test the node.
pub const SWEEP_RECHECK_INTERVAL: Duration = Duration::from_secs(60);

/// Heights a sweep covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SweepWindow {
    /// `[0, tip_height)`.
    Archive,
    /// `[tip_height - blocks, tip_height)`, saturating; the start rounds up to a multiple of
    /// `CHUNK_LEN` so chunk boundaries repeat across sweeps.
    Retention { blocks: u64 },
}

/// Heights derived from one tip; computed once per tip read, and every tip comparison reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frontier {
    tip: Height,
    /// `tip - ALERT_TIP_MARGIN`: end of follow, end of a sweep started now, alert bound.
    settled_end: Height,
    /// First height of the window; rounded up to `CHUNK_LEN` under retention.
    window_start: Height,
}

impl Frontier {
    /// The frontier of `tip` under `window`.
    pub fn new(tip: Height, window: SweepWindow) -> Self {
        let window_start = match window {
            SweepWindow::Archive => Height(0),
            SweepWindow::Retention { blocks } => {
                Height(tip.saturating_sub(blocks).0.next_multiple_of(CHUNK_LEN))
            },
        };
        Self {
            tip,
            settled_end: tip.saturating_sub(ALERT_TIP_MARGIN),
            window_start,
        }
    }

    pub fn tip(&self) -> Height {
        self.tip
    }

    /// `tip - ALERT_TIP_MARGIN`.
    pub fn settled_end(&self) -> Height {
        self.settled_end
    }

    /// First height of the window.
    pub fn window_start(&self) -> Height {
        self.window_start
    }

    /// `[window_start, settled_end)`: the range of a sweep started at this tip. Empty when the
    /// window start rounds past the settled end.
    pub fn sweep_range(&self) -> Range<Height> {
        self.window_start..self.settled_end
    }
}

/// `--sweep-interval` or `--sweep-once`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SweepSchedule {
    Every(TimeDelta),
    Once,
}

/// A finished sweep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletedSweep {
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
}

/// A sweep over `[next_height, until_height)` that began at `start_height` at `started_at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunningSweep {
    pub started_at: DateTime<Utc>,
    pub start_height: Height,
    pub next_height: Height,
    pub until_height: Height,
}

impl RunningSweep {
    /// Next chunk of the sweep; `None` when it is finished.
    pub fn next_chunk(&self) -> Option<Range<Height>> {
        next_chunk(self.next_height, self.until_height)
    }

    /// Percent done from `start_height`, capped at 100; an empty sweep is 100.
    pub fn percent_done(&self) -> f64 {
        let total = self.until_height.blocks_since(self.start_height);
        if total == 0 {
            return 100.0;
        }
        let done = self.next_height.blocks_since(self.start_height).min(total);
        done as f64 * 100.0 / total as f64
    }

    /// `next_height` moved to `chunk_end`.
    pub fn advanced(self, chunk_end: Height) -> Self {
        Self {
            next_height: chunk_end,
            ..self
        }
    }

    /// Completed at `now`: `Waiting` under `Every`, `Done` under `Once`.
    pub fn finished(self, schedule: SweepSchedule, now: DateTime<Utc>) -> SweepState {
        let last = CompletedSweep {
            started_at: self.started_at,
            completed_at: now,
        };
        match schedule {
            SweepSchedule::Every(_) => SweepState::Waiting { last },
            SweepSchedule::Once => SweepState::Done { last },
        }
    }
}

/// Sweep schedule of one node; each variant carries its times.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SweepState {
    /// No sweep has run: the first is due at once.
    NeverSwept,
    /// The next sweep is due one interval after `last.started_at`.
    Waiting { last: CompletedSweep },
    Running {
        sweep: RunningSweep,
        /// Completion time of the sweep before this one.
        last_completed_at: Option<DateTime<Utc>>,
    },
    /// The one sweep of `--sweep-once` finished.
    Done { last: CompletedSweep },
}

impl SweepState {
    /// `Done` becomes `Waiting` under `Every`; anything else is unchanged.
    pub fn rescheduled(self, schedule: SweepSchedule) -> Self {
        match (self, schedule) {
            (Self::Done { last }, SweepSchedule::Every(_)) => Self::Waiting { last },
            (state, _) => state,
        }
    }

    /// Whether a `NeverSwept` or `Waiting` sweep starts at `now`: always under `Once`, else at or
    /// after `last.started_at + interval`.
    pub fn is_due(&self, schedule: SweepSchedule, now: DateTime<Utc>) -> bool {
        match (self, schedule) {
            (Self::NeverSwept, _) | (Self::Waiting { .. }, SweepSchedule::Once) => true,
            (Self::Waiting { last }, SweepSchedule::Every(interval)) => {
                now >= last.started_at + interval
            },
            (Self::Running { .. } | Self::Done { .. }, _) => false,
        }
    }

    /// `Running` over `range` from `now`, keeping the last completed sweep.
    pub fn started(self, range: Range<Height>, now: DateTime<Utc>) -> Self {
        Self::Running {
            last_completed_at: self.last_completed(),
            sweep: RunningSweep {
                started_at: now,
                start_height: range.start,
                next_height: range.start,
                until_height: range.end,
            },
        }
    }

    /// `last.started_at + interval` for `Waiting` under `Every`; `None` otherwise (page and
    /// `resumed` line).
    pub fn next_start(&self, schedule: SweepSchedule) -> Option<DateTime<Utc>> {
        match (self, schedule) {
            (Self::Waiting { last }, SweepSchedule::Every(interval)) => {
                Some(last.started_at + interval)
            },
            _ => None,
        }
    }

    /// Completion time of the latest finished sweep.
    pub fn last_completed(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::NeverSwept => None,
            Self::Waiting { last } | Self::Done { last } => Some(last.completed_at),
            Self::Running {
                last_completed_at, ..
            } => *last_completed_at,
        }
    }
}

/// Persisted progress of one node watch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeProgress {
    pub follow_next_height: Height,
    pub sweep: SweepState,
}

impl NodeProgress {
    /// First run: follow from `frontier.settled_end`, where a sweep started now ends;
    /// `NeverSwept`.
    pub fn first_run(frontier: &Frontier) -> Self {
        Self {
            follow_next_height: frontier.settled_end(),
            sweep: SweepState::NeverSwept,
        }
    }

    /// Raises the follow height and a running sweep's next height to `window_start`.
    pub fn clamped_to(self, window_start: Height) -> Self {
        let sweep = match self.sweep {
            SweepState::Running {
                sweep,
                last_completed_at,
            } => SweepState::Running {
                sweep: RunningSweep {
                    next_height: sweep.next_height.max(window_start),
                    ..sweep
                },
                last_completed_at,
            },
            other => other,
        };
        Self {
            follow_next_height: self.follow_next_height.max(window_start),
            sweep,
        }
    }
}

/// Whether the sweep may send its next chunk. Only follow and sweep chunks change it; retries
/// never do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SweepGate {
    Open,
    Paused { recheck_at: Instant },
}

impl SweepGate {
    /// After a follow chunk: `Answered` opens; `Failed` pauses an open gate until `now +
    /// SWEEP_RECHECK_INTERVAL` and keeps a paused gate's `recheck_at`.
    pub fn after_follow_chunk(self, health: RequestHealth, now: Instant) -> Self {
        match (self, health) {
            (_, RequestHealth::Answered) => Self::Open,
            (Self::Open, RequestHealth::Failed) => Self::pausing(now),
            (paused @ Self::Paused { .. }, RequestHealth::Failed) => paused,
        }
    }

    /// After a sweep chunk: `Answered` opens; `Failed` pauses until `now +
    /// SWEEP_RECHECK_INTERVAL`, so a recheck is spent only by a chunk that was sent.
    pub fn after_sweep_chunk(self, health: RequestHealth, now: Instant) -> Self {
        match health {
            RequestHealth::Answered => Self::Open,
            RequestHealth::Failed => Self::pausing(now),
        }
    }

    /// `Open`, or `Paused` at or after `recheck_at`. Pure.
    pub fn admits(&self, now: Instant) -> bool {
        match self {
            Self::Open => true,
            Self::Paused { recheck_at } => now >= *recheck_at,
        }
    }

    /// Paused until one recheck interval after `now`.
    fn pausing(now: Instant) -> Self {
        Self::Paused {
            recheck_at: now + SWEEP_RECHECK_INTERVAL,
        }
    }
}

/// Where the ETA rate starts: when this process began or resumed the running sweep.
#[derive(Clone, Copy, Debug)]
pub struct EtaOrigin {
    pub at: Instant,
    pub next_height: Height,
}

impl EtaOrigin {
    /// Time left of `sweep` at the pace since the origin, whole seconds; `None` before a height is
    /// done. The rule of HEAD's `eta_at_pace`.
    pub fn remaining(self, now: Instant, sweep: &RunningSweep) -> Option<Duration> {
        let done = sweep.next_height.blocks_since(self.next_height);
        if done == 0 {
            return None;
        }
        let left = sweep.until_height.blocks_since(sweep.next_height);
        let elapsed = now.saturating_duration_since(self.at);
        let secs = elapsed.as_secs_f64() * left as f64 / done as f64;
        Duration::try_from_secs_f64(secs)
            .ok()
            .map(|eta| Duration::from_secs(eta.as_secs()))
    }
}

#[cfg(test)]
mod tests {
    use RequestHealth::{Answered, Failed};
    use pretty_assertions::assert_eq;

    use super::*;

    const T0: i64 = 1_000;
    const WEEK: TimeDelta = TimeDelta::weeks(1);
    const EVERY: SweepSchedule = SweepSchedule::Every(WEEK);

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    fn range(start: u64, end: u64) -> Range<Height> {
        Height(start)..Height(end)
    }

    fn running(start: u64, next: u64, until: u64) -> RunningSweep {
        RunningSweep {
            started_at: at(T0),
            start_height: Height(start),
            next_height: Height(next),
            until_height: Height(until),
        }
    }

    fn completed(started: i64, done: i64) -> CompletedSweep {
        CompletedSweep {
            started_at: at(started),
            completed_at: at(done),
        }
    }

    #[test]
    fn frontier_table() {
        let retention = |blocks| SweepWindow::Retention { blocks };
        let cases = [
            ("archive", 5000, SweepWindow::Archive, 0, 4990),
            ("retention", 5000, retention(2000), 3000, 4990),
            ("tip below the retention", 1500, retention(2000), 0, 1490),
            ("rounds up", 5030, retention(2000), 3100, 5020),
            ("tip below the margin", 4, SweepWindow::Archive, 0, 0),
        ];
        for (name, tip, window, start, end) in cases {
            let frontier = Frontier::new(Height(tip), window);
            assert_eq!(frontier.tip(), Height(tip), "{name}");
            assert_eq!(frontier.sweep_range(), range(start, end), "{name}");
        }
    }

    #[test]
    fn an_empty_window_is_done_at_once() {
        // The window start rounds up past the settled end.
        let frontier = Frontier::new(Height(1_050), SweepWindow::Retention { blocks: 10 });
        assert!(frontier.window_start() > frontier.settled_end());
        let sweep = SweepState::NeverSwept.started(frontier.sweep_range(), at(T0));
        let SweepState::Running { sweep, .. } = sweep else {
            panic!("running");
        };
        assert_eq!(sweep.percent_done(), 100.0);
        assert_eq!(sweep.next_chunk(), None);
    }

    #[test]
    fn first_run_follows_from_where_the_sweep_ends() {
        let frontier = Frontier::new(Height(1_000), SweepWindow::Archive);
        let progress = NodeProgress::first_run(&frontier);
        assert_eq!(progress.follow_next_height, Height(990));
        assert_eq!(progress.sweep, SweepState::NeverSwept);
    }

    #[test]
    fn progress_clamps_to_window_start() {
        let sweep = SweepState::Running {
            sweep: running(0, 80, 500),
            last_completed_at: None,
        };
        let progress = NodeProgress {
            follow_next_height: Height(50),
            sweep,
        };
        let clamped = progress.clamped_to(Height(100));
        assert_eq!(clamped.follow_next_height, Height(100));
        let SweepState::Running { sweep, .. } = clamped.sweep else {
            panic!("sweep stays running");
        };
        assert_eq!(sweep.next_height, Height(100));
        assert_eq!(progress.clamped_to(Height(0)), progress);
    }

    #[test]
    fn percent_done_table() {
        let cases = [
            (running(0, 250, 1000), 25.0),
            (running(700, 745, 790), 50.0),
            (running(0, 1, 3), 100.0 / 3.0),
            (running(1050, 1050, 1050), 100.0),
            // `next` beyond `until` caps at 100%.
            (running(0, 1200, 1000), 100.0),
        ];
        for (sweep, percent) in cases {
            assert!((sweep.percent_done() - percent).abs() < 1e-9, "{sweep:?}");
        }
    }

    #[test]
    fn sweep_transitions() {
        let last = completed(T0, T0 + 50);
        let waiting = SweepState::Waiting { last };
        let done = SweepState::Done { last };

        assert_eq!(done.rescheduled(EVERY), waiting);
        assert_eq!(done.rescheduled(SweepSchedule::Once), done);
        assert_eq!(waiting.rescheduled(EVERY), waiting);

        let due_at = at(T0) + WEEK;
        assert!(SweepState::NeverSwept.is_due(EVERY, at(0)));
        assert!(!waiting.is_due(EVERY, due_at - TimeDelta::seconds(1)));
        assert!(waiting.is_due(EVERY, due_at));
        assert!(waiting.is_due(SweepSchedule::Once, at(T0 + 1)));
        assert!(!done.is_due(SweepSchedule::Once, at(T0 + 1)));

        assert_eq!(waiting.next_start(EVERY), Some(due_at));
        assert_eq!(waiting.next_start(SweepSchedule::Once), None);
        assert_eq!(SweepState::NeverSwept.next_start(EVERY), None);

        let started = waiting.started(range(100, 500), at(T0 + 9_999));
        let SweepState::Running {
            sweep,
            last_completed_at,
        } = started
        else {
            panic!("running");
        };
        assert_eq!(last_completed_at, Some(at(T0 + 50)));
        assert_eq!(sweep.started_at, at(T0 + 9_999));
        assert_eq!(sweep.next_chunk(), Some(range(100, 200)));
        let sweep = sweep.advanced(Height(200));
        assert_eq!(sweep.next_height, Height(200));
        assert_eq!(sweep.start_height, Height(100));

        let finished = sweep.finished(EVERY, at(T0 + 10_000));
        let last = completed(T0 + 9_999, T0 + 10_000);
        assert_eq!(finished, SweepState::Waiting { last });
        assert_eq!(finished.last_completed(), Some(at(T0 + 10_000)));
        assert_eq!(
            sweep.finished(SweepSchedule::Once, at(T0 + 10_000)),
            SweepState::Done { last }
        );
        assert_eq!(SweepState::NeverSwept.last_completed(), None);
    }

    #[test]
    fn gate_cells() {
        let start = Instant::now();
        let at_secs = |secs| start + Duration::from_secs(secs);
        let paused = SweepGate::Paused {
            recheck_at: at_secs(70),
        };
        let paused_from = |secs: u64| SweepGate::Paused {
            recheck_at: at_secs(secs + 60),
        };
        // (gate, health, now, after follow chunk, after sweep chunk)
        let cases = [
            (
                SweepGate::Open,
                Answered,
                10,
                SweepGate::Open,
                SweepGate::Open,
            ),
            (
                SweepGate::Open,
                Failed,
                10,
                paused_from(10),
                paused_from(10),
            ),
            (paused, Answered, 40, SweepGate::Open, SweepGate::Open),
            (paused, Failed, 40, paused, paused_from(40)),
            (paused, Failed, 80, paused, paused_from(80)),
        ];
        for (gate, health, now, follow, sweep) in cases {
            assert_eq!(gate.after_follow_chunk(health, at_secs(now)), follow);
            assert_eq!(gate.after_sweep_chunk(health, at_secs(now)), sweep);
        }
        for (secs, admits) in [(69, false), (70, true), (71, true)] {
            assert_eq!(paused.admits(at_secs(secs)), admits);
        }
        assert!(SweepGate::Open.admits(start));
    }

    #[test]
    fn eta_at_pace_table() {
        let start = Instant::now();
        let secs = |s| Some(Duration::from_secs(s));
        let cases = [
            ("nothing done", (0, 100, 0, 1000), None),
            ("quarter done in 100 s", (0, 100, 250, 1000), secs(300)),
            ("origin offset", (500, 100, 750, 1000), secs(100)),
            ("finished", (0, 100, 1000, 1000), secs(0)),
        ];
        for (name, (origin, elapsed, next, until), expected) in cases {
            let origin = EtaOrigin {
                at: start,
                next_height: Height(origin),
            };
            let sweep = running(0, next, until);
            let now = start + Duration::from_secs(elapsed);
            assert_eq!(origin.remaining(now, &sweep), expected, "{name}");
        }
    }
}
