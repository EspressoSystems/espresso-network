use std::{convert::Infallible, ops::Range, sync::Arc, time::Duration};

use anyhow::bail;
use chrono::TimeDelta;
use clap::Args;
use serde::Deserialize;
use tokio::{sync::watch, time::Instant};

use crate::{
    gaps::{Presence, find_gaps},
    kind::{Epochs, Height, HotshotEpochs, Probe, RangeKind, next_chunk, probes},
    metrics::WatchdogMetrics,
    missing::{AlertRule, MissingUpdate},
    node::{FetchError, FetchTarget, WatchedNode},
    outcome::{NotServed, Observation, ProbeResult, RangeFindings, RequestFailure, RequestHealth},
    report::{GAUGE_INTERVAL, ReportedNode, TICKS_PER_SUMMARY},
    retry::{ChunkQueue, RETRY_BUDGET, RetryQueue},
    schedule::{
        EtaOrigin, Frontier, NodeProgress, RunningSweep, SweepGate, SweepSchedule, SweepState,
        SweepWindow,
    },
    store::{NodeStore, PersistedMissingSet, Store},
    system::{Clock, System},
    view::{NodeView, NodeViewSlot, PAGE_ROWS, SweepEta, Tips, ViewSender, WatchCounters},
};

/// Pacing shared by all node watches.
#[derive(Clone, Debug, Args)]
pub struct WatchOptions {
    /// Time from one sweep start to the next; at most 1 year.
    #[arg(
        long,
        env = "ESPRESSO_QUERY_WATCHDOG_SWEEP_INTERVAL",
        value_parser = parse_year_bounded_duration,
        default_value = "1week"
    )]
    pub sweep_interval: Duration,
    /// Sweep once, then only follow the tip; `--sweep-interval` is ignored. A node that has not
    /// swept yet sweeps at once, and a restart does not sweep again. Removing the flag schedules
    /// the next sweep one interval after the last started.
    #[arg(long, env = "ESPRESSO_QUERY_WATCHDOG_SWEEP_ONCE")]
    pub sweep_once: bool,
    /// Wait before each request of one node watch.
    #[arg(
        long,
        env = "ESPRESSO_QUERY_WATCHDOG_REQUEST_INTERVAL",
        value_parser = humantime::parse_duration,
        default_value = "200ms"
    )]
    pub request_interval: Duration,
    /// Heights below the tip each sweep checks; absent means the whole chain.
    #[arg(
        long,
        env = "ESPRESSO_QUERY_WATCHDOG_RETENTION_BLOCKS",
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    pub retention_blocks: Option<u64>,
    /// Minimum age before a confirmed unit counts in `query_watchdog_confirmed_missing`; at most 1
    /// year. 60s leaves room for retries at about 5 and 15 s first.
    #[arg(
        long,
        env = "ESPRESSO_QUERY_WATCHDOG_ALERT_MISSING_AFTER",
        value_parser = parse_year_bounded_duration,
        default_value = "60s"
    )]
    pub alert_missing_after: Duration,
}

impl WatchOptions {
    /// `Archive` when `retention_blocks` is absent.
    pub fn sweep_window(&self) -> SweepWindow {
        self.retention_blocks
            .map_or(SweepWindow::Archive, |blocks| SweepWindow::Retention {
                blocks,
            })
    }

    /// `sweep_interval` as a chrono delta; at most a year, checked when the option is parsed.
    pub fn sweep_interval_delta(&self) -> TimeDelta {
        TimeDelta::from_std(self.sweep_interval).expect("sweep_interval is checked at parse time")
    }

    /// `Once` with `--sweep-once`, else `Every(sweep_interval)`.
    pub fn sweep_schedule(&self) -> SweepSchedule {
        if self.sweep_once {
            SweepSchedule::Once
        } else {
            SweepSchedule::Every(self.sweep_interval_delta())
        }
    }

    /// `min_age` from `--alert-missing-after`.
    pub fn alert_rule(&self) -> AlertRule {
        AlertRule {
            min_age: self.alert_missing_after,
        }
    }
}

/// The longest `--sweep-interval` and `--alert-missing-after`: one year (humantime's `year`).
/// Bounded, so adding either to a time cannot overflow.
const MAX_OPTION_DURATION: Duration = Duration::from_secs(31_557_600);

/// Parses a humantime duration of at most `MAX_OPTION_DURATION`.
fn parse_year_bounded_duration(text: &str) -> Result<Duration, String> {
    let duration = humantime::parse_duration(text).map_err(|error| error.to_string())?;
    if duration > MAX_OPTION_DURATION {
        return Err(format!("{text} is longer than 1 year"));
    }
    Ok(duration)
}

/// The part of `v1/config/hotshot` the watchdog reads.
#[derive(Deserialize)]
struct NetworkConfig {
    config: HotshotEpochs,
}

/// Time between tip polls and between polls of a node's merklized state height.
const TIP_POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Parts shared by all node watches; every field is a cheap clone.
#[derive(Clone)]
pub struct Watchdog<C: Clock> {
    system: System<C>,
    options: WatchOptions,
    metrics: Arc<WatchdogMetrics>,
    store: Store,
}

impl<C: Clock> Watchdog<C> {
    /// Holds the parts.
    pub fn new(
        system: System<C>,
        options: WatchOptions,
        metrics: Arc<WatchdogMetrics>,
        store: Store,
    ) -> Self {
        Self {
            system,
            options,
            metrics,
            store,
        }
    }

    /// Loads the missing set and the stored progress and publishes the first view; the reporter
    /// owns the gauges. When the node has answered, the watch begins at once (progress created or
    /// clamped, `watching` or `resumed` logged); otherwise `run` begins it at the first answer.
    pub async fn start_node(
        &self,
        node: WatchedNode,
        tips: watch::Receiver<Tips>,
        epochs: Epochs,
        view: ViewSender,
    ) -> anyhow::Result<NodeWatch<C>> {
        let frontier = Frontier::new(Height(0), self.options.sweep_window());
        let store = self.store.node(&node.label);
        let stored = store.load_progress().await?;
        let missing = PersistedMissingSet::load(store.clone(), epochs).await?;
        let mut watch = NodeWatch {
            watchdog: self.clone(),
            node,
            tips,
            frontier,
            missing,
            store,
            progress: NodeProgress::first_run(&frontier),
            startup: Startup::WaitingForHeight { stored },
            gate: SweepGate::Open,
            merklized_state_height: None,
            next_state_poll_at: self.system.clock.instant(),
            counters: WatchCounters::default(),
            trimmed_to: Height(0),
            recovery: ChunkQueue::default(),
            eta_origin: None,
            view,
        };
        watch.begin_if_height_known().await?;
        watch.publish();
        Ok(watch)
    }

    /// First heights, errors when no node answers; returns the poller and the receiver every
    /// `NodeWatch` and the reporter read.
    pub async fn start_tip_poller(
        &self,
        nodes: Vec<WatchedNode>,
    ) -> anyhow::Result<(TipPoller<C>, watch::Receiver<Tips>)> {
        let mut tips = Tips::default();
        if self.fetch_tips(&nodes, &mut tips).await == 0 {
            let labels: Vec<_> = nodes.iter().map(|node| &node.label).collect();
            bail!("no watched node answered v1/node/block-height: {labels:?}");
        }
        self.metrics.set_tip_height(tips.max())?;
        let (sender, receiver) = watch::channel(tips.clone());
        let poller = TipPoller {
            watchdog: self.clone(),
            nodes,
            tips,
            sender,
        };
        Ok((poller, receiver))
    }

    /// The reporter over `nodes`, reading the live node heights.
    pub fn start_reporter(
        &self,
        tips: watch::Receiver<Tips>,
        nodes: Vec<NodeViewSlot>,
    ) -> Reporter<C> {
        Reporter {
            watchdog: self.clone(),
            tips,
            nodes: nodes.into_iter().map(ReportedNode::new).collect(),
        }
    }

    /// Deletes the stored state (`--reset`) and always logs a WARN.
    pub async fn reset_state(&self) -> anyhow::Result<()> {
        let deleted = self.store.reset().await?;
        tracing::warn!(deleted, "--reset deleted the stored state");
        Ok(())
    }

    /// `v1/config/hotshot` from the first node that answers, through `From<HotshotEpochs> for
    /// Epochs`.
    pub async fn fetch_epochs(&self, nodes: &[WatchedNode]) -> anyhow::Result<Epochs> {
        let mut failures = Vec::new();
        for node in nodes {
            match self
                .system
                .http
                .get_json::<NetworkConfig>(node, "v1/config/hotshot")
                .await
            {
                Ok(NetworkConfig { config }) => return Ok(config.into()),
                Err(error) => {
                    self.metrics
                        .count_fetch_error(&node.label, FetchTarget::Config, &error);
                    failures.push(format!("{}: {error}", node.label));
                },
            }
        }
        bail!(
            "no watched node serves v1/config/hotshot: {}",
            failures.join("; ")
        )
    }

    /// Records the `v1/node/block-height` of every node in `tips`; returns how many answered.
    async fn fetch_tips(&self, nodes: &[WatchedNode], tips: &mut Tips) -> usize {
        let mut answered = 0;
        for node in nodes {
            match self
                .system
                .http
                .get_json::<u64>(node, "v1/node/block-height")
                .await
            {
                Ok(height) => {
                    tips.observed(&node.label, Height(height));
                    answered += 1;
                },
                Err(error) => {
                    tracing::debug!(node = %node.label, %error, "tip request failed");
                    self.metrics
                        .count_fetch_error(&node.label, FetchTarget::BlockHeight, &error);
                },
            }
        }
        answered
    }
}

/// Publishes the block height of every watched node and their maximum every `TIP_POLL_INTERVAL`.
pub struct TipPoller<C: Clock> {
    watchdog: Watchdog<C>,
    nodes: Vec<WatchedNode>,
    tips: Tips,
    sender: watch::Sender<Tips>,
}

impl<C: Clock> TipPoller<C> {
    /// Polls forever, returning only an error; a node that does not answer keeps its height.
    pub async fn run(mut self) -> anyhow::Result<Infallible> {
        loop {
            self.watchdog.system.clock.sleep(TIP_POLL_INTERVAL).await;
            self.watchdog.fetch_tips(&self.nodes, &mut self.tips).await;
            self.watchdog.metrics.set_tip_height(self.tips.max())?;
            self.sender.send_replace(self.tips.clone());
        }
    }
}

/// Writes gauges every `GAUGE_INTERVAL` and logs every `TICKS_PER_SUMMARY` ticks for every node,
/// from published views only.
pub struct Reporter<C: Clock> {
    watchdog: Watchdog<C>,
    tips: watch::Receiver<Tips>,
    nodes: Vec<ReportedNode>,
}

impl<C: Clock> Reporter<C> {
    /// `tick(0)`, then forever: sleep `GAUGE_INTERVAL`, `tick(tick_index + 1)`. Returns only an
    /// error.
    pub async fn run(mut self) -> anyhow::Result<Infallible> {
        let mut tick_index = 0;
        loop {
            self.tick(tick_index)?;
            self.watchdog.system.clock.sleep(GAUGE_INTERVAL).await;
            tick_index += 1;
        }
    }

    /// `ReportedNode::report` and `export_node` for every node at `clock.now()`; also
    /// `ReportedNode::log` when `tick_index` is a positive multiple of `TICKS_PER_SUMMARY`.
    fn tick(&mut self, tick_index: u64) -> anyhow::Result<()> {
        let now = self.watchdog.system.clock.now();
        let tips = self.tips.borrow().clone();
        let log = tick_index > 0 && tick_index.is_multiple_of(TICKS_PER_SUMMARY);
        for node in &mut self.nodes {
            let report = node.report(&tips, now)?;
            self.watchdog.metrics.export_node(&report)?;
            if log {
                node.log(&report);
            }
        }
        Ok(())
    }
}

/// First run or resume of a node watch, for the start line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeStart {
    Fresh,
    Resumed,
}

/// Requests one chunk sent and their combined health.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkChecked {
    pub sent: usize,
    pub health: RequestHealth,
}

/// Whether the watch has built its progress from the node's own height.
#[derive(Clone, Copy, Debug)]
enum Startup {
    /// The node has not answered; `stored` is the progress of an earlier run.
    WaitingForHeight {
        stored: Option<NodeProgress>,
    },
    Begun,
}

/// Checker of one watched node: follow, retries and sweep, one request at a time.
pub struct NodeWatch<C: Clock> {
    watchdog: Watchdog<C>,
    node: WatchedNode,
    tips: watch::Receiver<Tips>,
    /// Set once per loop iteration from the node's own height in `tips`.
    frontier: Frontier,
    missing: PersistedMissingSet,
    store: NodeStore,
    progress: NodeProgress,
    startup: Startup,
    gate: SweepGate,
    merklized_state_height: Option<Height>,
    next_state_poll_at: Instant,
    counters: WatchCounters,
    /// Window start of the last `ForgetBelow`; `Height(0)` at start.
    trimmed_to: Height,
    /// Confirmed rows still to re-check after the representative was found present; empty at
    /// start.
    recovery: ChunkQueue,
    eta_origin: Option<EtaOrigin>,
    view: ViewSender,
}

impl<C: Clock> NodeWatch<C> {
    fn clock(&self) -> &C {
        &self.watchdog.system.clock
    }

    /// Runs the loop until a store error: frontier, `trim_step`, state poll, follow, retry and
    /// sweep steps; idles when nothing was sent.
    pub async fn run(mut self) -> anyhow::Result<()> {
        self.await_own_height().await?;
        loop {
            let own = self.tips.borrow_and_update().node(&self.node.label);
            let own = own.expect("the node answered before the loop starts");
            self.frontier = Frontier::new(own, self.watchdog.options.sweep_window());
            self.trim_step().await?;
            self.poll_state_height().await;
            let mut sent = self.follow_step().await?;
            sent += self.retry_step().await?;
            sent += self.sweep_step().await?;
            if sent == 0 {
                let wait = self
                    .next_state_poll_at
                    .saturating_duration_since(self.clock().instant());
                tokio::select! {
                    () = self.watchdog.system.clock.sleep(wait) => {},
                    changed = self.tips.changed() => changed?,
                }
            }
        }
    }

    /// Waits for the node's first block height, then `begin_if_height_known`.
    async fn await_own_height(&mut self) -> anyhow::Result<()> {
        loop {
            self.begin_if_height_known().await?;
            if matches!(self.startup, Startup::Begun) {
                return Ok(());
            }
            self.tips.changed().await?;
        }
    }

    /// Once the node's own height is known: builds the frontier, creates the progress or clamps
    /// the stored one to the window, drops persisted rows above `settled_end`, logs `watching` or
    /// `resumed`.
    async fn begin_if_height_known(&mut self) -> anyhow::Result<()> {
        let Startup::WaitingForHeight { stored } = self.startup else {
            return Ok(());
        };
        let Some(own) = self.tips.borrow().node(&self.node.label) else {
            return Ok(());
        };
        self.startup = Startup::Begun;
        self.frontier = Frontier::new(own, self.watchdog.options.sweep_window());
        let start = match stored {
            Some(progress) => {
                self.progress = progress.clamped_to(self.frontier.window_start());
                NodeStart::Resumed
            },
            None => {
                self.progress = NodeProgress::first_run(&self.frontier);
                self.store.save_progress(&self.progress).await?;
                NodeStart::Fresh
            },
        };
        self.reset_eta_origin();
        self.drop_rows_above_height().await?;
        self.log_started(start);
        self.publish();
        Ok(())
    }

    /// Drops persisted rows the node cannot serve yet, so they never count with old ages.
    async fn drop_rows_above_height(&mut self) -> anyhow::Result<()> {
        let update = MissingUpdate::ForgetAbove(self.frontier.settled_end());
        let now = self.clock().now();
        let diff = self.missing.apply(update, now).await?;
        if diff.rows_removed > 0 {
            tracing::info!(
                node = %self.node.label,
                rows = diff.rows_removed,
                "dropped rows above the node's own height"
            );
        }
        Ok(())
    }

    /// The `watching` or `resumed` line.
    fn log_started(&self, start: NodeStart) {
        let node = &self.node.label;
        let tip_height = self.frontier.tip().0;
        let summary = self
            .snapshot()
            .summary(self.frontier.tip(), self.clock().now());
        if start == NodeStart::Fresh {
            tracing::info!(%node, tip_height, %summary, "watching");
            return;
        }
        let next_start = self
            .progress
            .sweep
            .next_start(self.watchdog.options.sweep_schedule());
        let (sweep, next_height, until_height, start_at) = match &self.progress.sweep {
            SweepState::Running { sweep, .. } => (
                "running",
                Some(sweep.next_height.0),
                Some(sweep.until_height.0),
                None,
            ),
            SweepState::NeverSwept | SweepState::Waiting { .. } => {
                ("waiting", None, None, next_start.map(|t| t.to_rfc3339()))
            },
            SweepState::Done { .. } => ("done", None, None, None),
        };
        tracing::info!(
            %node,
            follow_next_height = self.progress.follow_next_height.0,
            sweep,
            next_height,
            until_height,
            start_at,
            %summary,
            "resumed"
        );
    }

    /// Retention: when `frontier.window_start` is above `trimmed_to`, clamps progress, applies
    /// `ForgetBelow` and adds `rows_removed` to `counters.forgotten_rows`. Archive: `window_start`
    /// stays 0, never runs.
    async fn trim_step(&mut self) -> anyhow::Result<()> {
        let window_start = self.frontier.window_start();
        if window_start <= self.trimmed_to {
            return Ok(());
        }
        let clamped = self.progress.clamped_to(window_start);
        if clamped != self.progress {
            let sweep_changed = clamped.sweep != self.progress.sweep;
            self.commit_progress(clamped).await?;
            if sweep_changed {
                self.reset_eta_origin();
            }
        }
        let now = self.clock().now();
        let diff = self
            .missing
            .apply(MissingUpdate::ForgetBelow(window_start), now)
            .await?;
        self.counters.forgotten_rows += diff.rows_removed as u64;
        self.recovery
            .forget_below(window_start, self.missing.set().epochs());
        self.trimmed_to = window_start;
        self.publish();
        Ok(())
    }

    /// Refreshes the merklized state height when due.
    async fn poll_state_height(&mut self) {
        let now = self.clock().instant();
        if now < self.next_state_poll_at {
            return;
        }
        self.next_state_poll_at = now + TIP_POLL_INTERVAL;
        self.clock()
            .sleep(self.watchdog.options.request_interval)
            .await;
        match self
            .watchdog
            .system
            .http
            .get_json::<u64>(&self.node, "v1/block-state/block-height")
            .await
        {
            Ok(height) => {
                self.merklized_state_height = Some(Height(height));
                self.publish();
            },
            Err(error) => self.record_error(FetchTarget::MerklizedStateHeight, &error),
        }
    }

    /// Checks the next follow chunk, then `gate.after_follow_chunk`; returns requests sent.
    async fn follow_step(&mut self) -> anyhow::Result<usize> {
        let Some(chunk) = next_chunk(
            self.progress.follow_next_height,
            self.frontier.settled_end(),
        ) else {
            return Ok(0);
        };
        let checked = self.check_chunk(&chunk).await?;
        self.gate = self
            .gate
            .after_follow_chunk(checked.health, self.clock().instant());
        self.commit_progress(NodeProgress {
            follow_next_height: chunk.end,
            ..self.progress
        })
        .await?;
        Ok(checked.sent)
    }

    /// `RetryQueue::new` at `now`; while `RETRY_BUDGET` from the step start has time left: due
    /// not-found rows, `recovery`, the representative when `recovery` was empty at the step start (a result that
    /// `found_present` sets `recovery = ChunkQueue::confirmed(set)`), `recovery` again, then due
    /// failed rows, which never count and so never delay red or green; returns requests sent.
    async fn retry_step(&mut self) -> anyhow::Result<usize> {
        let deadline = self.clock().instant() + RETRY_BUDGET;
        let mut queue = RetryQueue::new(self.missing.set(), self.clock().now());
        let representative = queue
            .representative
            .take()
            .filter(|_| self.recovery.is_empty());
        let mut sent = self.drain(&mut queue.not_found, deadline).await?;
        sent += self.drain_recovery(deadline).await?;
        sent += self.probe_representative(representative, deadline).await?;
        sent += self.drain_recovery(deadline).await?;
        sent += self.drain(&mut queue.failed, deadline).await?;
        Ok(sent)
    }

    /// Probes `queue` until it is empty or `deadline`; returns requests sent.
    async fn drain(&mut self, queue: &mut ChunkQueue, deadline: Instant) -> anyhow::Result<usize> {
        let mut sent = 0;
        while self.clock().instant() < deadline
            && let Some(probe) = queue.next_probe()
        {
            self.check_probe(&probe).await?;
            sent += 1;
        }
        Ok(sent)
    }

    /// `drain` of `recovery`.
    async fn drain_recovery(&mut self, deadline: Instant) -> anyhow::Result<usize> {
        let mut recovery = std::mem::take(&mut self.recovery);
        let sent = self.drain(&mut recovery, deadline).await;
        self.recovery = recovery;
        sent
    }

    /// Probes the representative before `deadline`; a present result queues every confirmed row
    /// in `recovery`. Returns requests sent.
    async fn probe_representative(
        &mut self,
        representative: Option<Probe>,
        deadline: Instant,
    ) -> anyhow::Result<usize> {
        let Some(probe) = representative.filter(|_| self.clock().instant() < deadline) else {
            return Ok(0);
        };
        let result = self.check_probe(&probe).await?;
        if result.found_present() {
            self.recovery = ChunkQueue::confirmed(self.missing.set());
        }
        Ok(1)
    }

    /// Reschedules, starts or finishes the sweep without the gate; sends the next chunk only when
    /// `gate.admits`, then `gate.after_sweep_chunk`; returns requests sent.
    async fn sweep_step(&mut self) -> anyhow::Result<usize> {
        let schedule = self.watchdog.options.sweep_schedule();
        let rescheduled = self.progress.sweep.rescheduled(schedule);
        if rescheduled != self.progress.sweep {
            self.commit_progress(NodeProgress {
                sweep: rescheduled,
                ..self.progress
            })
            .await?;
        }
        if matches!(
            self.progress.sweep,
            SweepState::NeverSwept | SweepState::Waiting { .. }
        ) {
            if !self.progress.sweep.is_due(schedule, self.clock().now()) {
                return Ok(0);
            }
            self.start_sweep().await?;
        }
        let SweepState::Running {
            sweep,
            last_completed_at,
        } = self.progress.sweep
        else {
            return Ok(0);
        };
        let Some(chunk) = sweep.next_chunk() else {
            self.finish_sweep(sweep).await?;
            return Ok(0);
        };
        if chunk.end > self.frontier.settled_end() {
            return Ok(0);
        }
        if !self.gate.admits(self.clock().instant()) {
            return Ok(0);
        }
        let checked = self.check_chunk(&chunk).await?;
        self.gate = self
            .gate
            .after_sweep_chunk(checked.health, self.clock().instant());
        self.commit_progress(NodeProgress {
            sweep: SweepState::Running {
                sweep: sweep.advanced(chunk.end),
                last_completed_at,
            },
            ..self.progress
        })
        .await?;
        Ok(checked.sent)
    }

    /// Starts a sweep over `frontier.sweep_range()` and logs `sweep started`.
    async fn start_sweep(&mut self) -> anyhow::Result<()> {
        let range = self.frontier.sweep_range();
        let sweep = self
            .progress
            .sweep
            .started(range.clone(), self.clock().now());
        self.commit_progress(NodeProgress {
            sweep,
            ..self.progress
        })
        .await?;
        self.reset_eta_origin();
        tracing::info!(
            node = %self.node.label,
            start_height = range.start.0,
            until_height = range.end.0,
            "sweep started"
        );
        Ok(())
    }

    /// Finishes `sweep` and logs `sweep completed` (WARN on a backward clock step).
    async fn finish_sweep(&mut self, sweep: RunningSweep) -> anyhow::Result<()> {
        let completed_at = self.clock().now();
        let duration = (completed_at - sweep.started_at)
            .to_std()
            .unwrap_or_else(|_| {
                tracing::warn!(
                    node = %self.node.label,
                    started_at = %sweep.started_at,
                    %completed_at,
                    "sweep completed before it started, the clock stepped back"
                );
                Duration::ZERO
            });
        let schedule = self.watchdog.options.sweep_schedule();
        self.commit_progress(NodeProgress {
            sweep: sweep.finished(schedule, completed_at),
            ..self.progress
        })
        .await?;
        self.reset_eta_origin();
        let summary = self.snapshot().summary(self.frontier.tip(), completed_at);
        tracing::info!(
            node = %self.node.label,
            duration = %humantime::format_duration(duration),
            %summary,
            "sweep completed"
        );
        Ok(())
    }

    /// `check_probe` for every probe of the chunk.
    async fn check_chunk(&mut self, chunk: &Range<Height>) -> anyhow::Result<ChunkChecked> {
        let to_check = probes(chunk, self.missing.set().epochs());
        let mut health = RequestHealth::Answered;
        for probe in &to_check {
            health = health.and(self.check_probe(probe).await?.gate_health());
        }
        Ok(ChunkChecked {
            sent: to_check.len(),
            health,
        })
    }

    /// `observe`, `missing.apply`, `publish`; returns the applied result.
    async fn check_probe(&mut self, probe: &Probe) -> anyhow::Result<ProbeResult> {
        let result = self.observe(probe).await;
        let now = self.clock().now();
        self.missing
            .apply(MissingUpdate::Observed(result.clone()), now)
            .await?;
        self.publish();
        Ok(result)
    }

    /// Sends the probe and, for a range 404, runs `find_gaps` with `missing.set().known(..)`;
    /// counts errors.
    async fn observe(&mut self, probe: &Probe) -> ProbeResult {
        let answer = self.send_probe(probe).await;
        let observation = match &answer {
            Ok(()) => Observation::Present,
            Err(error) => {
                self.record_error(FetchTarget::Data(probe.kind()), error);
                error.not_served().into()
            },
        };
        match probe {
            Probe::Range { kind, heights } => {
                let findings = match observation {
                    Observation::NotFound => {
                        let known = self.missing.set().known(*kind, heights);
                        let mut prober = RangeProber {
                            watch: self,
                            kind: *kind,
                        };
                        find_gaps(&mut prober, heights.clone(), known).await
                    },
                    Observation::Present | Observation::Failed(_) => {
                        RangeFindings::uniform(heights.clone(), observation)
                    },
                };
                ProbeResult::Range {
                    kind: *kind,
                    findings,
                }
            },
            Probe::Point { kind, height } => ProbeResult::Point {
                kind: *kind,
                height: *height,
                observation,
            },
            Probe::Epoch { kind, epoch } => ProbeResult::Epoch {
                kind: *kind,
                epoch: *epoch,
                observation,
            },
        }
    }

    /// Waits `request_interval`, counts the probe, sends HEAD.
    async fn send_probe(&mut self, probe: &Probe) -> Result<(), FetchError> {
        self.clock()
            .sleep(self.watchdog.options.request_interval)
            .await;
        self.watchdog
            .metrics
            .count_probe(&self.node.label, probe.kind());
        self.watchdog.system.http.head(&self.node, probe).await
    }

    /// Counts a failed request in metrics and `counters.errors`.
    fn record_error(&mut self, target: FetchTarget, error: &FetchError) {
        tracing::debug!(node = %self.node.label, target = target.label(), %error, "request failed");
        self.watchdog
            .metrics
            .count_fetch_error(&self.node.label, target, error);
        self.counters.errors.count(error.not_served());
    }

    /// Saves `progress`, makes it current, publishes; the only progress write.
    async fn commit_progress(&mut self, progress: NodeProgress) -> anyhow::Result<()> {
        self.store.save_progress(&progress).await?;
        self.progress = progress;
        self.publish();
        Ok(())
    }

    /// The view of the current state at `clock.now()`.
    fn snapshot(&self) -> NodeView {
        let now = self.clock().now();
        let rule = self.watchdog.options.alert_rule();
        let sweep_eta = self.sweep_eta().and_then(|remaining| {
            let finish = now.checked_add_signed(TimeDelta::from_std(remaining).ok()?)?;
            Some(SweepEta { remaining, finish })
        });
        let set = self.missing.set();
        NodeView {
            published_at: now,
            frontier: self.frontier,
            progress: self.progress,
            merklized_state_height: self.merklized_state_height,
            alert_rule: rule,
            tallies: set.tallies(self.frontier.settled_end(), rule.counted_until(now)),
            listed: set.listed(PAGE_ROWS),
            counters: self.counters.clone(),
            next_sweep_start: self
                .progress
                .sweep
                .next_start(self.watchdog.options.sweep_schedule()),
            sweep_eta,
        }
    }

    /// `send_replace(Some(snapshot()))`; never fails without receivers.
    fn publish(&self) {
        self.view.send_replace(Some(self.snapshot()));
    }

    /// ETA of the running sweep at the pace since `eta_origin`.
    fn sweep_eta(&self) -> Option<Duration> {
        let SweepState::Running { sweep, .. } = &self.progress.sweep else {
            return None;
        };
        self.eta_origin?.remaining(self.clock().instant(), sweep)
    }

    /// Restarts the ETA rate from now at the running sweep's next height; none otherwise.
    fn reset_eta_origin(&mut self) {
        self.eta_origin = match &self.progress.sweep {
            SweepState::Running { sweep, .. } => Some(EtaOrigin {
                at: self.clock().instant(),
                next_height: sweep.next_height,
            }),
            _ => None,
        };
    }
}

/// Asks one node which heights of one range kind it serves, for `find_gaps`.
struct RangeProber<'a, C: Clock> {
    watch: &'a mut NodeWatch<C>,
    kind: RangeKind,
}

impl<C: Clock> Presence for RangeProber<'_, C> {
    async fn all_present(&mut self, heights: Range<Height>) -> Result<bool, RequestFailure> {
        let probe = Probe::Range {
            kind: self.kind,
            heights,
        };
        match self.watch.send_probe(&probe).await {
            Ok(()) => Ok(true),
            Err(error) => match error.not_served() {
                NotServed::NotFound => Ok(false),
                NotServed::Failed(failure) => {
                    self.watch
                        .record_error(FetchTarget::Data(self.kind.into()), &error);
                    Err(failure)
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        str::FromStr,
        sync::{
            Mutex,
            atomic::{AtomicBool, AtomicU64, Ordering},
        },
    };

    use axum::{
        Router,
        extract::State,
        http::{Method, StatusCode, Uri},
    };
    use chrono::{DateTime, Utc};
    use pretty_assertions::assert_eq;
    use prometheus::Registry;
    use sqlx::sqlite::SqliteConnectOptions;
    use tokio::net::TcpListener;
    use tracing::Level;
    use url::Url;

    use super::*;
    use crate::{
        capture::Capture,
        kind::{DataKind, PointKind},
        missing::RowKey,
        node::{NodeHttp, ProbeTimeouts},
        schedule::{CompletedSweep, RunningSweep},
        state::{RetryDelay, UnitState},
        system::ManualClock,
        view::{self, ViewReceiver},
    };

    const CONFIG_OFF: &str = r#"{"config":{"epoch_height":0,"epoch_start_block":0}}"#;

    /// A node serving every route except what the flags and sets take away, recording requests.
    #[derive(Clone)]
    struct FakeNode {
        /// Leaf heights the leaf range route lacks.
        missing: Arc<Mutex<BTreeSet<u64>>>,
        /// Epochs the stake table route lacks.
        missing_epochs: Arc<Mutex<BTreeSet<u64>>>,
        /// Every route answers 500 while set.
        failing: Arc<AtomicBool>,
        /// The leaf range route answers 500 while set.
        failing_leaf: Arc<AtomicBool>,
        /// Leaf range requests that answer before the rest answer 500; `None` is unlimited.
        leaf_budget: Arc<Mutex<Option<usize>>>,
        /// The light client leaf route answers 500 while set.
        failing_points: Arc<AtomicBool>,
        /// The stake table route answers 500 while set.
        failing_stake_table: Arc<AtomicBool>,
        /// Every data route answers 404 while set.
        all_404: Arc<AtomicBool>,
        /// Every route records the request and never answers while set.
        hang: Arc<AtomicBool>,
        tip: Arc<AtomicU64>,
        config: Arc<Mutex<String>>,
        requests: Arc<Mutex<Vec<(Method, String)>>>,
    }

    impl FakeNode {
        /// HEAD paths requested so far.
        fn heads(&self) -> Vec<String> {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .filter(|(method, _)| *method == Method::HEAD)
                .map(|(_, path)| path.clone())
                .collect()
        }

        /// Requests of any method to `path`.
        fn requests_to(&self, path: &str) -> usize {
            self.requests
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, p)| p == path)
                .count()
        }
    }

    async fn fake_route(
        State(node): State<FakeNode>,
        method: Method,
        uri: Uri,
    ) -> (StatusCode, String) {
        let path = uri.path().to_owned();
        node.requests.lock().unwrap().push((method, path.clone()));
        if node.hang.load(Ordering::SeqCst) {
            std::future::pending::<()>().await;
        }
        let flag = |flag: &AtomicBool| flag.load(Ordering::SeqCst);
        let ok = |body: &str| (StatusCode::OK, body.to_owned());
        let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        if flag(&node.failing) {
            return (StatusCode::INTERNAL_SERVER_ERROR, String::new());
        }
        match segments.as_slice() {
            ["v1", "config", "hotshot"] => ok(&node.config.lock().unwrap()),
            ["v1", "node", "block-height"] => ok(&node.tip.load(Ordering::SeqCst).to_string()),
            ["v1", "block-state", "block-height"] => ok("1000"),
            ["v1", "light-client", "leaf", _] if flag(&node.failing_points) => {
                (StatusCode::INTERNAL_SERVER_ERROR, String::new())
            },
            ["v1", "node", "stake-table", _] if flag(&node.failing_stake_table) => {
                (StatusCode::INTERNAL_SERVER_ERROR, String::new())
            },
            _ if flag(&node.all_404) => (StatusCode::NOT_FOUND, String::new()),
            ["v1", "availability", "leaf", from, until] => {
                if flag(&node.failing_leaf) {
                    return (StatusCode::INTERNAL_SERVER_ERROR, String::new());
                }
                if let Some(left) = node.leaf_budget.lock().unwrap().as_mut() {
                    if *left == 0 {
                        return (StatusCode::INTERNAL_SERVER_ERROR, String::new());
                    }
                    *left -= 1;
                }
                let (from, until): (u64, u64) = (from.parse().unwrap(), until.parse().unwrap());
                let gap = node
                    .missing
                    .lock()
                    .unwrap()
                    .range(from..until)
                    .next()
                    .is_some();
                if gap {
                    (StatusCode::NOT_FOUND, String::new())
                } else {
                    ok("")
                }
            },
            ["v1", "node", "stake-table", epoch] => {
                let epoch: u64 = epoch.parse().unwrap();
                if node.missing_epochs.lock().unwrap().contains(&epoch) {
                    (StatusCode::NOT_FOUND, String::new())
                } else {
                    ok("")
                }
            },
            _ => ok(""),
        }
    }

    async fn serve_fake(missing: impl IntoIterator<Item = u64>) -> (WatchedNode, FakeNode) {
        let fake = FakeNode {
            missing: Arc::new(Mutex::new(missing.into_iter().collect())),
            missing_epochs: Arc::default(),
            failing: Arc::default(),
            failing_leaf: Arc::default(),
            failing_points: Arc::default(),
            failing_stake_table: Arc::default(),
            leaf_budget: Arc::default(),
            all_404: Arc::default(),
            hang: Arc::default(),
            tip: Arc::new(AtomicU64::new(1_000)),
            config: Arc::new(Mutex::new(CONFIG_OFF.to_owned())),
            requests: Arc::default(),
        };
        let app = Router::new().fallback(fake_route).with_state(fake.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });
        (WatchedNode::new(base_url).unwrap(), fake)
    }

    /// Everything a test needs around one watchdog.
    struct Harness {
        watchdog: Watchdog<ManualClock>,
        store: Store,
        registry: Registry,
        clock: ManualClock,
        tips_sender: watch::Sender<Tips>,
        tips: watch::Receiver<Tips>,
    }

    fn options(retention_blocks: Option<u64>) -> WatchOptions {
        WatchOptions {
            sweep_interval: Duration::from_secs(1),
            sweep_once: false,
            request_interval: Duration::ZERO,
            retention_blocks,
            alert_missing_after: Duration::ZERO,
        }
    }

    impl Harness {
        /// A watchdog over an in-memory store, a manual clock and no pacing.
        async fn new(retention_blocks: Option<u64>) -> Self {
            Self::with_options(options(retention_blocks)).await
        }

        async fn with_options(options: WatchOptions) -> Self {
            Self::with_timeouts(options, ProbeTimeouts::DEFAULT).await
        }

        async fn with_timeouts(options: WatchOptions, timeouts: ProbeTimeouts) -> Self {
            let registry = Registry::new();
            let store = Store::connect(SqliteConnectOptions::from_str("sqlite::memory:").unwrap())
                .await
                .unwrap();
            let clock = ManualClock::new(DateTime::from_timestamp(1_000_000, 0).unwrap());
            let system = System {
                clock: clock.clone(),
                http: NodeHttp::new(timeouts).unwrap(),
            };
            let watchdog = Watchdog::new(
                system,
                options,
                Arc::new(WatchdogMetrics::new(&registry).unwrap()),
                store.clone(),
            );
            let (tips_sender, tips) = watch::channel(Tips::default());
            Self {
                watchdog,
                store,
                registry,
                clock,
                tips_sender,
                tips,
            }
        }

        async fn start(&self, node: WatchedNode) -> (NodeWatch<ManualClock>, ViewReceiver) {
            self.start_with(node, Epochs::Off).await
        }

        async fn start_with(
            &self,
            node: WatchedNode,
            epochs: Epochs,
        ) -> (NodeWatch<ManualClock>, ViewReceiver) {
            let (sender, receiver) = view::channel();
            self.tips_sender.send_if_modified(|tips| {
                let unknown = tips.node(&node.label).is_none();
                if unknown {
                    tips.observed(&node.label, Height(1_000));
                }
                unknown
            });
            let watch = self
                .watchdog
                .start_node(node, self.tips.clone(), epochs, sender)
                .await
                .unwrap();
            (watch, receiver)
        }

        /// A reporter over one started node.
        fn reporter(&self, node: &WatchedNode, view: &ViewReceiver) -> Reporter<ManualClock> {
            let slot = NodeViewSlot {
                node: node.clone(),
                view: view.clone(),
            };
            self.watchdog.start_reporter(self.tips.clone(), vec![slot])
        }

        /// Raises the height of `node` as the tip poller would.
        fn set_height(&self, node: &WatchedNode, height: u64) {
            self.tips_sender
                .send_modify(|tips| tips.observed(&node.label, Height(height)));
        }

        /// The value of a gauge of `node` (and `kind`, when given).
        fn gauge(&self, name: &str, node: &WatchedNode, kind: Option<&str>) -> Option<i64> {
            let wanted = |label: &prometheus::proto::LabelPair| match label.get_name() {
                "node" => label.get_value() == node.label,
                "kind" => kind == Some(label.get_value()),
                _ => true,
            };
            self.registry
                .gather()
                .iter()
                .filter(|family| family.get_name() == format!("query_watchdog_{name}"))
                .flat_map(|family| family.get_metric())
                .find(|metric| metric.get_label().iter().all(wanted))
                .map(|metric| metric.get_gauge().get_value() as i64)
        }

        /// Writes `result` into the stored image of `node` at `at`, before it starts.
        async fn preload(&self, node: &WatchedNode, results: Vec<(ProbeResult, DateTime<Utc>)>) {
            let store = self.store.node(&node.label);
            let mut persisted = PersistedMissingSet::load(store, Epochs::Off).await.unwrap();
            for (result, at) in results {
                persisted
                    .apply(MissingUpdate::Observed(result), at)
                    .await
                    .unwrap();
            }
        }
    }

    fn h(range: Range<u64>) -> Range<Height> {
        Height(range.start)..Height(range.end)
    }

    fn leaf(range: Range<u64>) -> Probe {
        Probe::Range {
            kind: RangeKind::Leaf,
            heights: h(range),
        }
    }

    fn leaf_result(range: Range<u64>, observation: Observation) -> ProbeResult {
        ProbeResult::Range {
            kind: RangeKind::Leaf,
            findings: RangeFindings::uniform(h(range), observation),
        }
    }

    fn point_result(height: u64, observation: Observation) -> ProbeResult {
        ProbeResult::Point {
            kind: PointKind::LcLeafProof,
            height: Height(height),
            observation,
        }
    }

    fn leaf_missing<C: Clock>(watch: &NodeWatch<C>) -> u64 {
        watch.snapshot().tallies[0].missing_count
    }

    /// Confirmed missing units as the reporter would count them now.
    fn confirmed<C: Clock>(watch: &NodeWatch<C>) -> u64 {
        let now = watch.clock().now();
        watch
            .snapshot()
            .summary(watch.frontier.tip(), now)
            .confirmed_missing()
    }

    fn is_running<C: Clock>(watch: &NodeWatch<C>) -> bool {
        matches!(watch.progress.sweep, SweepState::Running { .. })
    }

    fn is_paused<C: Clock>(watch: &NodeWatch<C>) -> bool {
        matches!(watch.gate, SweepGate::Paused { .. })
    }

    #[tokio::test]
    async fn a_failed_range_probe_is_refined_by_bisection() {
        let (node, fake) = serve_fake(103..106).await;
        let harness = Harness::new(None).await;
        let (mut watch, _view) = harness.start(node).await;

        watch.check_probe(&leaf(100..110)).await.unwrap();

        // The chunk, the prefix search for 103, the gallop at 104, 105 and 107, the binary step
        // at 106, the rest from 106.
        let expected = [
            "/v1/availability/leaf/100/110",
            "/v1/availability/leaf/100/105",
            "/v1/availability/leaf/100/102",
            "/v1/availability/leaf/100/103",
            "/v1/availability/leaf/100/104",
            "/v1/availability/leaf/104/105",
            "/v1/availability/leaf/105/106",
            "/v1/availability/leaf/107/108",
            "/v1/availability/leaf/106/107",
            "/v1/availability/leaf/106/110",
        ];
        assert_eq!(fake.heads(), expected);
        assert_eq!(
            fake.requests.lock().unwrap().len(),
            expected.len(),
            "HEADs only"
        );

        let tally = watch.snapshot().tallies.remove(0);
        assert_eq!(tally.missing_count, 3);
        assert_eq!(
            tally.oldest.unwrap().key,
            RowKey::Range {
                kind: RangeKind::Leaf,
                heights: h(103..106)
            }
        );
        let probes_total: u64 = harness
            .registry
            .gather()
            .iter()
            .filter(|family| family.get_name() == "query_watchdog_probes_total")
            .flat_map(|family| family.get_metric())
            .map(|metric| metric.get_counter().get_value() as u64)
            .sum();
        assert_eq!(probes_total, 10);
    }

    #[tokio::test]
    async fn a_gap_is_confirmed_then_cleared_as_the_node_fills_it() {
        let (node, fake) = serve_fake(750..760).await;
        let capture = Capture::default();
        let _guard = capture.install();
        let harness = Harness::new(None).await;
        let (mut watch, view) = harness.start(node.clone()).await;
        let mut reporter = harness.reporter(&node, &view);

        // The chunk probe finds the exact gap; two retries bring it to 3 404s.
        watch.check_probe(&leaf(700..800)).await.unwrap();
        assert_eq!(leaf_missing(&watch), 10);
        reporter.tick(1).unwrap();
        assert_eq!(harness.gauge("confirmed_missing", &node, None), Some(0));
        for _ in 0..2 {
            harness.clock.advance(Duration::from_secs(10));
            assert_eq!(watch.retry_step().await.unwrap(), 1);
        }
        reporter.tick(2).unwrap();
        assert_eq!(harness.gauge("confirmed_missing", &node, None), Some(10));

        // A node that answers 500 is unhealthy, not missing data: the 404 row stays confirmed.
        fake.failing.store(true, Ordering::SeqCst);
        harness.clock.advance(Duration::from_secs(61));
        watch.retry_step().await.unwrap();
        assert_eq!(confirmed(&watch), 10);
        assert_eq!(watch.counters.errors.failures().is_empty(), false);
        fake.failing.store(false, Ordering::SeqCst);

        // The node fills the gap: the representative is found present, the row is gone.
        fake.missing.lock().unwrap().clear();
        harness.clock.advance(Duration::from_secs(61));
        watch.retry_step().await.unwrap();
        assert_eq!(leaf_missing(&watch), 0);
        reporter.tick(3).unwrap();
        assert_eq!(harness.gauge("confirmed_missing", &node, None), Some(0));
        assert!(capture.with_message("missing data").is_empty());
    }

    /// `count` confirmed-to-be rows of the point kind at heights `1..=count`.
    async fn add_confirming_points(watch: &mut NodeWatch<ManualClock>, count: u64) {
        for height in 1..=count {
            let now = watch.clock().now();
            let result = point_result(height, Observation::NotFound);
            watch
                .missing
                .apply(MissingUpdate::Observed(result), now)
                .await
                .unwrap();
        }
        watch.clock().sleep(Duration::from_secs(5)).await;
    }

    #[tokio::test]
    async fn the_retries_stop_at_their_budget() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::with_options(WatchOptions {
            request_interval: Duration::from_secs(2),
            ..options(None)
        })
        .await;
        let (mut watch, _view) = harness.start(node).await;
        add_confirming_points(&mut watch, 10).await;

        // 2 s per request against a budget of 5 s: probes at 0, 2 and 4 s.
        assert_eq!(watch.retry_step().await.unwrap(), 3);
        assert_eq!(watch.missing.set().rows().count(), 7, "present points go");
    }

    #[tokio::test]
    async fn a_500_on_a_404_row_is_retried_once_and_then_waits() {
        let (node, fake) = serve_fake(750..760).await;
        let harness = Harness::with_options(WatchOptions {
            request_interval: Duration::from_secs(1),
            ..options(None)
        })
        .await;
        let (mut watch, _view) = harness.start(node).await;
        watch.check_probe(&leaf(700..800)).await.unwrap();
        for _ in 0..2 {
            harness.clock.advance(Duration::from_secs(10));
            watch.retry_step().await.unwrap();
        }
        harness.clock.advance(Duration::from_secs(120));

        fake.failing.store(true, Ordering::SeqCst);
        assert_eq!(watch.retry_step().await.unwrap(), 1);
        assert_eq!(
            watch.retry_step().await.unwrap(),
            0,
            "the row waits for its next retry"
        );
        let row = watch.missing.set().rows().next().unwrap();
        assert!(row.state.is_confirmed());
        assert_eq!(row.state.probe_count().get(), 3);
        assert_eq!(row.state.retry().delay, RetryDelay::from_secs(40).unwrap());
        assert!(row.state.retry().next_at > watch.clock().now());
        assert!(watch.recovery.is_empty());
    }

    /// Preloads `count` confirmed `lc_leaf_proof` rows at heights `1..=count` into the image.
    async fn preload_confirmed_points(harness: &Harness, node: &WatchedNode, count: u64) {
        let start = harness.clock.now();
        let mut results = Vec::new();
        for _ in 0..5 {
            let marks: Vec<_> = (1..=count)
                .map(|height| point_result(height, Observation::NotFound))
                .collect();
            results.extend(marks.into_iter().map(|result| (result, start)));
        }
        harness.preload(node, results).await;
    }

    #[tokio::test]
    async fn only_the_representative_is_retried_until_it_is_found_present() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::with_options(WatchOptions {
            request_interval: Duration::from_secs(1),
            ..options(None)
        })
        .await;
        preload_confirmed_points(&harness, &node, 40).await;
        let (mut watch, view) = harness.start(node).await;
        fake.all_404.store(true, Ordering::SeqCst);

        harness.clock.advance(Duration::from_secs(61));
        assert_eq!(watch.retry_step().await.unwrap(), 1, "only the oldest row");
        assert_eq!(fake.heads(), ["/v1/light-client/leaf/1"]);
        assert_eq!(
            watch.retry_step().await.unwrap(),
            0,
            "its next_at is a minute away"
        );
        harness.clock.advance(Duration::from_secs(61));
        assert_eq!(watch.retry_step().await.unwrap(), 1);

        // The node serves again: the representative, then the others once each at the budget.
        fake.all_404.store(false, Ordering::SeqCst);
        fake.requests.lock().unwrap().clear();
        harness.clock.advance(Duration::from_secs(61));
        let mut sent = vec![watch.retry_step().await.unwrap()];
        while watch.missing.set().rows().count() > 0 {
            sent.push(watch.retry_step().await.unwrap());
            assert!(sent.len() < 20, "{sent:?}");
        }
        assert_eq!(sent, [5; 8], "40 rows at 5 per step");
        let heads = fake.heads();
        let distinct: BTreeSet<_> = heads.iter().collect();
        assert_eq!(heads.len(), 40);
        assert_eq!(distinct.len(), 40, "no row twice");
        let view = view.borrow();
        let tallies = &view.as_ref().unwrap().tallies;
        assert!(tallies.iter().all(|t| t.missing_count == 0));
    }

    #[tokio::test]
    async fn a_new_gap_is_confirmed_whatever_the_number_of_confirmed_rows() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::with_options(WatchOptions {
            request_interval: Duration::from_secs(1),
            ..options(None)
        })
        .await;
        // 1000 confirmed leaf rows of 10 heights, none touching.
        let mut findings = RangeFindings::new(h(0..20_000));
        for i in 0..1_000u64 {
            findings.mark(h(i * 20..i * 20 + 10), Observation::NotFound);
        }
        let start = harness.clock.now();
        let rows = ProbeResult::Range {
            kind: RangeKind::Leaf,
            findings,
        };
        harness
            .preload(&node, (0..5).map(|_| (rows.clone(), start)).collect())
            .await;
        let (mut watch, _view) = harness.start(node).await;
        fake.all_404.store(true, Ordering::SeqCst);
        harness.clock.advance(Duration::from_secs(61));

        let now = watch.clock().now();
        watch
            .missing
            .apply(
                MissingUpdate::Observed(point_result(5, Observation::NotFound)),
                now,
            )
            .await
            .unwrap();
        let fresh_since = watch.clock().now();
        let fresh_state = |watch: &NodeWatch<ManualClock>| {
            watch
                .missing
                .set()
                .rows()
                .find(|row| row.key.kind() == DataKind::LcLeafProof)
                .unwrap()
                .state
        };
        while {
            watch.retry_step().await.unwrap();
            !fresh_state(&watch).is_confirmed()
        } {
            harness.clock.advance(Duration::from_secs(1));
            assert!(
                watch.clock().now() - fresh_since < TimeDelta::seconds(30),
                "not confirmed in time"
            );
        }
        assert!(
            watch.clock().now() - fresh_since <= TimeDelta::seconds(17),
            "confirmed at {}",
            watch.clock().now() - fresh_since
        );
        let heads = fake.heads();
        let leaf_heads: Vec<_> = heads
            .iter()
            .filter(|path| path.starts_with("/v1/availability/leaf"))
            .collect();
        assert_eq!(
            leaf_heads,
            [
                "/v1/availability/leaf/0/10",
                "/v1/availability/leaf/0/1",
                "/v1/availability/leaf/9/10"
            ],
            "the representative and its two end checks"
        );
        assert_eq!(
            heads
                .iter()
                .filter(|p| p.starts_with("/v1/light-client"))
                .count(),
            2
        );
        assert_eq!(heads.len(), 5, "no other request");
    }

    #[tokio::test]
    async fn a_one_time_sweep_ends_in_done_and_stays_quiet() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::with_options(WatchOptions {
            sweep_interval: Duration::from_secs(3_600),
            sweep_once: true,
            ..options(Some(200))
        })
        .await;
        let (mut watch, _view) = harness.start(node).await;

        for _ in 0..5 {
            watch.sweep_step().await.unwrap();
        }
        let stored = watch.store.load_progress().await.unwrap().unwrap();
        assert!(
            matches!(stored.sweep, SweepState::Done { .. }),
            "{stored:?}"
        );
        assert!(stored.sweep.last_completed().is_some());
        assert_eq!(watch.sweep_step().await.unwrap(), 0);
        assert!(matches!(watch.progress.sweep, SweepState::Done { .. }));

        // Without the flag the next sweep is scheduled after the last one.
        watch.watchdog.options.sweep_once = false;
        assert_eq!(watch.sweep_step().await.unwrap(), 0);
        let SweepState::Waiting { last } = watch.progress.sweep else {
            panic!("{:?}", watch.progress.sweep);
        };
        assert_eq!(
            watch
                .progress
                .sweep
                .next_start(watch.watchdog.options.sweep_schedule()),
            Some(last.started_at + TimeDelta::seconds(3_600))
        );
    }

    #[tokio::test]
    async fn a_sweep_that_ends_before_it_started_is_not_an_error() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::new(Some(200)).await;
        let (mut watch, _view) = harness.start(node).await;
        watch.sweep_step().await.unwrap();
        let SweepState::Running {
            sweep,
            last_completed_at,
        } = watch.progress.sweep
        else {
            panic!("running");
        };
        let sweep = RunningSweep {
            started_at: watch.clock().now() + TimeDelta::hours(1),
            ..sweep
        };
        watch.progress.sweep = SweepState::Running {
            sweep,
            last_completed_at,
        };
        watch.finish_sweep(sweep).await.unwrap();
        let SweepState::Waiting { last } = watch.progress.sweep else {
            panic!("{:?}", watch.progress.sweep);
        };
        assert!(last.completed_at < last.started_at);
    }

    #[tokio::test]
    async fn the_sweep_pauses_while_a_chunk_fails_and_resumes_after() {
        let (node, fake) = serve_fake([905]).await;
        let harness = Harness::new(Some(200)).await;
        let (mut watch, _view) = harness.start(node).await;

        // A retry probe that fails neither sets nor clears the pause.
        fake.failing.store(true, Ordering::SeqCst);
        let failed = watch.check_probe(&leaf(900..910)).await.unwrap();
        assert_eq!(failed.health(), RequestHealth::Failed);
        assert!(!is_paused(&watch));

        // A failing sweep chunk pauses; the sweep started without the gate and goes no further.
        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(is_paused(&watch));
        assert_eq!(watch.sweep_step().await.unwrap(), 0);
        assert!(is_running(&watch));

        // A retry probe that answers does not resume it; the recheck chunk after a minute does.
        fake.failing.store(false, Ordering::SeqCst);
        let answered = watch.check_probe(&leaf(900..910)).await.unwrap();
        assert_eq!(answered.health(), RequestHealth::Answered);
        assert!(is_paused(&watch));
        harness.clock.advance(Duration::from_secs(60));
        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(!is_paused(&watch));
    }

    #[tokio::test]
    async fn a_failed_follow_chunk_pauses_the_sweep() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::new(Some(200)).await;
        let window = harness.watchdog.options.sweep_window();
        let (mut watch, _view) = harness.start(node).await;

        fake.failing.store(true, Ordering::SeqCst);
        watch.frontier = Frontier::new(Height(1_500), window);
        assert_eq!(watch.follow_step().await.unwrap(), 4);
        assert!(is_paused(&watch));
        assert_eq!(watch.sweep_step().await.unwrap(), 0);
        assert!(is_running(&watch), "the sweep starts without the gate");

        // A follow chunk that answers opens the gate.
        fake.failing.store(false, Ordering::SeqCst);
        watch.follow_step().await.unwrap();
        assert!(!is_paused(&watch));
        assert!(watch.sweep_step().await.unwrap() > 0);
    }

    #[tokio::test]
    async fn one_failed_probe_in_a_chunk_pauses_the_sweep_until_a_chunk_is_fully_answered() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::new(Some(200)).await;
        let (mut watch, _view) = harness.start(node).await;

        // The leaf probe, first of four, fails; the three after it answer.
        fake.failing_leaf.store(true, Ordering::SeqCst);
        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(is_paused(&watch), "any failed probe pauses");
        assert_eq!(watch.sweep_step().await.unwrap(), 0);

        fake.failing_leaf.store(false, Ordering::SeqCst);
        harness.clock.advance(Duration::from_secs(60));
        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(!is_paused(&watch));
    }

    #[tokio::test]
    async fn a_paused_sweep_rechecks_the_node_once_a_minute_and_resumes() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::new(Some(400)).await;
        let (mut watch, _view) = harness.start(node).await;

        fake.failing.store(true, Ordering::SeqCst);
        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(is_paused(&watch));
        assert_eq!(watch.sweep_step().await.unwrap(), 0, "within the minute");

        // After the minute one chunk goes out; the node still fails, so the pause stays.
        harness.clock.advance(Duration::from_secs(60));
        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(is_paused(&watch));
        assert_eq!(
            watch.sweep_step().await.unwrap(),
            0,
            "the next recheck is a minute away"
        );

        // The node recovers; the next recheck chunk clears the pause and the sweep goes on.
        fake.failing.store(false, Ordering::SeqCst);
        harness.clock.advance(Duration::from_secs(60));
        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(!is_paused(&watch));
        assert!(watch.sweep_step().await.unwrap() > 0);
    }

    #[tokio::test]
    async fn a_recheck_is_kept_while_the_sweep_waits() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::with_options(WatchOptions {
            sweep_interval: Duration::from_secs(3_600),
            ..options(Some(200))
        })
        .await;
        let (mut watch, _view) = harness.start(node).await;
        let started_at = watch.clock().now() - TimeDelta::minutes(1);
        watch.progress.sweep = SweepState::Waiting {
            last: CompletedSweep {
                started_at,
                completed_at: started_at,
            },
        };
        let paused = SweepGate::Paused {
            recheck_at: watch.clock().instant() - Duration::from_secs(1),
        };
        watch.gate = paused;

        assert_eq!(watch.sweep_step().await.unwrap(), 0);
        assert_eq!(watch.gate, paused, "the recheck is not spent");

        harness.clock.advance(Duration::from_secs(3_600));
        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(!is_paused(&watch));
    }

    #[tokio::test]
    async fn a_failed_refinement_fails_the_probe() {
        let (node, fake) = serve_fake([905]).await;
        let harness = Harness::new(Some(200)).await;
        let (mut watch, _view) = harness.start(node).await;

        // The chunk HEAD says 404, the first bisection HEAD fails with 500.
        *fake.leaf_budget.lock().unwrap() = Some(1);
        let result = watch.check_probe(&leaf(900..910)).await.unwrap();
        assert_eq!(result.health(), RequestHealth::Failed);
        assert_eq!(watch.counters.errors.failures().is_empty(), false);
        let row = watch.missing.set().rows().next().unwrap();
        assert!(matches!(row.state, UnitState::Failed { .. }));
        assert_eq!(row.key.units(), 900..910);
    }

    #[tokio::test]
    async fn a_fresh_node_follows_from_where_its_sweep_ends() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::new(Some(300)).await;
        let (watch, _view) = harness.start(node).await;
        assert_eq!(watch.frontier.sweep_range().end, Height(990));
        assert_eq!(watch.progress.follow_next_height, Height(990));
    }

    #[tokio::test]
    async fn sweep_once_starts_the_one_sweep_at_once_whatever_the_schedule() {
        let (node, _fake) = serve_fake([]).await;
        for (sweep_once, expect_running) in [(false, false), (true, true)] {
            let harness = Harness::with_options(WatchOptions {
                sweep_interval: Duration::from_secs(3_600),
                sweep_once,
                ..options(Some(200))
            })
            .await;
            let (mut watch, _view) = harness.start(node.clone()).await;
            // Waiting with the latest sweep started a minute ago: not due for an hour.
            let started_at = watch.clock().now() - TimeDelta::minutes(1);
            watch.progress.sweep = SweepState::Waiting {
                last: CompletedSweep {
                    started_at,
                    completed_at: started_at,
                },
            };
            watch.sweep_step().await.unwrap();
            assert_eq!(
                is_running(&watch),
                expect_running,
                "sweep_once {sweep_once}"
            );
        }
    }

    #[test]
    fn durations_above_a_year_are_rejected_at_parse_time() {
        assert!(parse_year_bounded_duration("60s").is_ok());
        assert!(parse_year_bounded_duration("1year").is_ok());
        assert!(parse_year_bounded_duration("13months").is_err());
        assert!(parse_year_bounded_duration("400000000years").is_err());
    }

    #[test]
    fn the_sweep_interval_defaults_to_a_week() {
        #[derive(clap::Parser)]
        struct Cli {
            #[command(flatten)]
            watch: WatchOptions,
        }
        let cli = <Cli as clap::Parser>::parse_from(["test"]);
        assert_eq!(
            cli.watch.sweep_interval,
            Duration::from_secs(7 * 24 * 3_600)
        );
        assert_eq!(
            humantime::format_duration(cli.watch.sweep_interval).to_string(),
            "7days"
        );
    }

    #[tokio::test]
    async fn reset_state_deletes_everything_and_the_node_starts_fresh() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::new(None).await;
        let node_store = harness.store.node(&node.label);
        harness.start(node.clone()).await;
        assert!(node_store.load_progress().await.unwrap().is_some());

        harness.watchdog.reset_state().await.unwrap();
        assert!(node_store.load_progress().await.unwrap().is_none());
        harness.start(node).await;
        assert!(node_store.load_progress().await.unwrap().is_some());
    }

    #[tokio::test]
    async fn the_watch_loop_follows_sweeps_confirms_and_trims() {
        let (node, _fake) = serve_fake(750..760).await;
        let harness = Harness::new(Some(300)).await;
        let window = harness.watchdog.options.sweep_window();
        let (mut watch, _view) = harness.start(node).await;

        // Follow stays the margin behind the tip, so nothing is probed yet.
        assert_eq!(watch.follow_step().await.unwrap(), 0);

        // The sweep covers `[700, 990)` in chunks, finds the gap exactly, then waits.
        for _ in 0..5 {
            watch.sweep_step().await.unwrap();
        }
        let stored = watch.store.load_progress().await.unwrap().unwrap();
        assert!(
            matches!(stored.sweep, SweepState::Waiting { .. }),
            "{stored:?}"
        );
        assert_eq!(leaf_missing(&watch), 10);
        assert_eq!(watch.snapshot().tallies[0].kind, DataKind::Leaf);

        // Two more 404 probes make the row confirmed: old enough, below the margin.
        for _ in 0..2 {
            watch.check_probe(&leaf(750..760)).await.unwrap();
        }
        assert_eq!(confirmed(&watch), 10);
        watch.frontier = Frontier::new(Height(755), window);
        assert_eq!(confirmed(&watch), 0, "inside the tip margin");

        // A later tip moves follow and the window; rows below the window are forgotten.
        watch.frontier = Frontier::new(Height(1_500), window);
        assert_eq!(watch.follow_step().await.unwrap(), 4);
        assert_eq!(watch.progress.follow_next_height, Height(1_090));
        watch.trim_step().await.unwrap();
        assert_eq!(watch.counters.forgotten_rows, 1);
        assert_eq!(watch.progress.follow_next_height, Height(1_200));
        assert_eq!(
            watch.store.load_progress().await.unwrap().unwrap(),
            watch.progress
        );
        assert_eq!(leaf_missing(&watch), 0);
        // The trim runs once per window start.
        watch.trim_step().await.unwrap();
        assert_eq!(watch.counters.forgotten_rows, 1);
    }

    #[tokio::test]
    async fn a_restarted_watch_loads_the_image_of_the_first() {
        let (node, _fake) = serve_fake(750..760).await;
        let harness = Harness::new(None).await;
        let (mut first, _view) = harness.start(node.clone()).await;
        first.check_probe(&leaf(700..800)).await.unwrap();
        first.sweep_step().await.unwrap();

        let (second, _view) = harness.start(node).await;
        let first_rows: Vec<_> = first.missing.set().rows().collect();
        let second_rows: Vec<_> = second.missing.set().rows().collect();
        assert!(!first_rows.is_empty());
        assert_eq!(second_rows, first_rows);
        assert_eq!(second.progress, first.progress);
    }

    #[tokio::test]
    async fn the_watch_publishes_after_every_probe_and_progress_commit() {
        let (node, _fake) = serve_fake(750..760).await;
        let harness = Harness::new(None).await;
        let (mut watch, view) = harness.start(node).await;
        {
            let first = view.borrow();
            let first = first
                .as_ref()
                .expect("the first view is published at start");
            assert_eq!(first.frontier.tip(), Height(1_000));
            assert_eq!(first.progress, watch.progress);
        }
        harness.clock.advance(Duration::from_secs(7));
        watch.check_probe(&leaf(700..800)).await.unwrap();
        {
            let published = view.borrow();
            let published = published.as_ref().unwrap();
            assert_eq!(published.listed.total, 1);
            assert_eq!(published.tallies[0].missing_count, 10);
            assert_eq!(published.published_at, watch.clock().now());
        }
        let progress = NodeProgress {
            follow_next_height: Height(123),
            ..watch.progress
        };
        watch.commit_progress(progress).await.unwrap();
        assert_eq!(view.borrow().as_ref().unwrap().progress, progress);
    }

    #[tokio::test]
    async fn the_loop_follows_sweeps_retries_and_publishes() {
        let (node, _fake) = serve_fake(750..753).await;
        let harness = Harness::with_options(WatchOptions {
            sweep_once: true,
            ..options(Some(300))
        })
        .await;
        let (watch, view) = harness.start(node).await;
        let task = tokio::spawn(watch.run());
        let done = async {
            loop {
                {
                    let view = view.borrow();
                    let view = view.as_ref().unwrap();
                    let swept = matches!(view.progress.sweep, SweepState::Done { .. });
                    if swept
                        && view.tallies[0].missing_count == 3
                        && view.progress.follow_next_height == Height(990)
                    {
                        return;
                    }
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        };
        tokio::time::timeout(Duration::from_secs(5), done)
            .await
            .expect("the loop sweeps and publishes");
        assert!(!task.is_finished(), "{:?}", task.is_finished());
        task.abort();
    }

    #[tokio::test]
    async fn the_tip_poller_publishes_the_max_and_keeps_it_when_no_node_answers() {
        let (first, fake_first) = serve_fake([]).await;
        let (second, fake_second) = serve_fake([]).await;
        fake_first.tip.store(5, Ordering::SeqCst);
        fake_second.tip.store(7, Ordering::SeqCst);
        let (first_label, second_label) = (first.label.clone(), second.label.clone());
        let harness = Harness::new(None).await;
        let (poller, mut tip) = harness
            .watchdog
            .start_tip_poller(vec![first, second])
            .await
            .unwrap();
        assert_eq!(tip.borrow().max(), Height(7));
        assert_eq!(tip.borrow().node(&first_label), Some(Height(5)));
        assert_eq!(tip.borrow().node_lag_blocks(&first_label), 2);

        fake_first.tip.store(9, Ordering::SeqCst);
        fake_second.tip.store(8, Ordering::SeqCst);
        let task = tokio::spawn(poller.run());
        tokio::time::timeout(Duration::from_secs(5), tip.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(tip.borrow_and_update().max(), Height(9));
        assert_eq!(tip.borrow().node(&second_label), Some(Height(8)));

        let before = fake_first.requests_to("/v1/node/block-height");
        fake_first.failing.store(true, Ordering::SeqCst);
        fake_second.failing.store(true, Ordering::SeqCst);
        tokio::time::timeout(Duration::from_secs(5), async {
            while fake_first.requests_to("/v1/node/block-height") < before + 3
                || fake_second.requests_to("/v1/node/block-height") < before + 3
            {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(tip.borrow().max(), Height(9));
        task.abort();
    }

    #[tokio::test]
    async fn an_epoch_object_missing_on_the_node_is_counted_in_the_gauge() {
        let (node, fake) = serve_fake([]).await;
        *fake.config.lock().unwrap() =
            r#"{"config":{"epoch_height":30,"epoch_start_block":61}}"#.to_owned();
        fake.missing_epochs.lock().unwrap().insert(5);
        fake.tip.store(300, Ordering::SeqCst);
        let harness = Harness::new(None).await;
        harness.set_height(&node, 300);
        let epochs = harness
            .watchdog
            .fetch_epochs(std::slice::from_ref(&node))
            .await
            .unwrap();
        assert!(matches!(epochs, Epochs::On(_)));
        let (mut watch, view) = harness.start_with(node.clone(), epochs).await;
        let mut reporter = harness.reporter(&node, &view);

        for _ in 0..6 {
            watch.sweep_step().await.unwrap();
        }
        assert!(matches!(watch.progress.sweep, SweepState::Waiting { .. }));
        for _ in 0..2 {
            harness.clock.advance(Duration::from_secs(61));
            watch.retry_step().await.unwrap();
        }
        reporter.tick(1).unwrap();
        assert_eq!(harness.gauge("confirmed_missing", &node, None), Some(1));
        let view = view.borrow();
        let view = view.as_ref().unwrap();
        let summary = view.summary(Height(300), watch.clock().now());
        let stake = summary
            .kinds
            .iter()
            .find(|kind| kind.kind == DataKind::StakeTable)
            .unwrap();
        assert_eq!(stake.confirmed_missing, 1);
        let route = view.listed.shown[0].key.first_probe().route();
        assert!(route.ends_with("v1/node/stake-table/5"), "{route}");
        let page = crate::http::render_page(&[(&node, Some(view))]);
        assert!(page.contains("v1/node/stake-table/5"), "{page}");
    }

    #[tokio::test]
    async fn a_node_answering_404_everywhere_is_counted_as_missing() {
        let (node, fake) = serve_fake([]).await;
        fake.all_404.store(true, Ordering::SeqCst);
        let harness = Harness::new(None).await;
        let window = harness.watchdog.options.sweep_window();
        let (mut watch, view) = harness.start(node.clone()).await;
        let mut reporter = harness.reporter(&node, &view);

        watch.frontier = Frontier::new(Height(1_100), window);
        watch.follow_step().await.unwrap();
        for delay in [5, 10] {
            harness.clock.advance(Duration::from_secs(delay));
            watch.retry_step().await.unwrap();
        }
        harness.set_height(&node, 1_100);
        reporter.tick(1).unwrap();
        assert!(harness.gauge("confirmed_missing", &node, None).unwrap() > 0);
    }

    /// The `summary` events of `capture` as (clock time, `confirmed_missing`).
    fn summaries(capture: &Capture) -> Vec<(DateTime<Utc>, String)> {
        capture
            .with_message("summary")
            .into_iter()
            .map(|event| (event.at.unwrap(), event.fields["confirmed_missing"].clone()))
            .collect()
    }

    #[tokio::test]
    async fn the_reporter_writes_gauges_every_tick_and_logs_every_fourth() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::with_options(WatchOptions {
            alert_missing_after: Duration::from_secs(60),
            ..options(None)
        })
        .await;
        let start = harness.clock.now();
        let result = leaf_result(500..510, Observation::NotFound);
        let first = start - TimeDelta::seconds(50);
        harness
            .preload(
                &node,
                (0..3)
                    .map(|i| (result.clone(), first + TimeDelta::seconds(i)))
                    .collect(),
            )
            .await;
        let capture = Capture::with_clock(harness.clock.clone());
        let _guard = capture.install();
        let (_watch, view) = harness.start(node.clone()).await;
        let mut reporter = harness.reporter(&node, &view);

        reporter.tick(0).unwrap();
        for gauge in [
            "confirmed_missing",
            "follow_lag_blocks",
            "follow_next_height",
            "sweep_running",
            "sweep_next_height",
            "sweep_until_height",
        ] {
            assert!(harness.gauge(gauge, &node, None).is_some(), "{gauge}");
        }
        assert_eq!(harness.gauge("missing", &node, Some("leaf")), Some(10));
        assert_eq!(harness.gauge("sweep_eta_seconds", &node, None), None);
        assert_eq!(harness.gauge("confirmed_missing", &node, None), Some(0));
        assert!(capture.with_message("summary").is_empty());

        for tick in 1..=8 {
            harness.clock.advance(GAUGE_INTERVAL);
            reporter.tick(tick).unwrap();
            let logged = capture.with_message("summary").len();
            assert_eq!(logged, tick as usize / 4, "after tick {tick}");
            if tick == 1 {
                assert_eq!(harness.gauge("confirmed_missing", &node, None), Some(10));
            }
        }
        let logged = summaries(&capture);
        assert_eq!(
            logged.iter().map(|(_, n)| n.as_str()).collect::<Vec<_>>(),
            ["10", "10"]
        );
        assert_eq!(logged[1].0 - logged[0].0, TimeDelta::seconds(60));
    }

    #[tokio::test]
    async fn the_report_reads_the_live_tip_and_a_stepped_back_clock() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::new(None).await;
        let now = harness.clock.now();
        harness
            .preload(
                &node,
                (0..3)
                    .map(|_| {
                        (
                            leaf_result(500..510, Observation::NotFound),
                            now + TimeDelta::seconds(100),
                        )
                    })
                    .collect(),
            )
            .await;
        let (_watch, view) = harness.start(node.clone()).await;
        let slot = NodeViewSlot {
            node: node.clone(),
            view: view.clone(),
        };
        let reported = ReportedNode::new(slot);
        let tips = {
            let mut tips = Tips::default();
            tips.observed(&node.label, Height(1_500));
            tips
        };
        let report = reported.report(&tips, now).unwrap();
        assert_eq!(report.summary.follow_lag_blocks, 510);
        let leaf = &report.summary.kinds[0];
        assert_eq!(
            leaf.oldest_age,
            Some(Duration::ZERO),
            "first missing after now"
        );
        assert_eq!(leaf.confirmed_missing, 0);
        assert_eq!(leaf.missing_count, 10);

        // The reporter takes the live tip from its channel.
        harness.set_height(&node, 1_500);
        let mut reporter = harness.reporter(&node, &view);
        reporter.tick(0).unwrap();
        assert_eq!(harness.gauge("follow_lag_blocks", &node, None), Some(510));
    }

    #[tokio::test]
    async fn a_node_without_a_view_is_an_error() {
        let (node, _fake) = serve_fake([]).await;
        let (_sender, receiver) = view::channel();
        let reported = ReportedNode::new(NodeViewSlot {
            node,
            view: receiver,
        });
        let now = DateTime::from_timestamp(1_000_000, 0).unwrap();
        assert!(reported.report(&Tips::default(), now).is_err());
    }

    #[tokio::test]
    async fn the_reporter_keeps_ticking_while_the_node_hangs() {
        let (node, fake) = serve_fake([]).await;
        fake.hang.store(true, Ordering::SeqCst);
        let timeouts = ProbeTimeouts {
            range_head: Duration::from_secs(30),
            single_head: Duration::from_secs(30),
            json: Duration::from_secs(30),
        };
        let harness = Harness::with_timeouts(
            WatchOptions {
                alert_missing_after: Duration::from_secs(120),
                ..options(None)
            },
            timeouts,
        )
        .await;
        let first = harness.clock.now() - TimeDelta::seconds(30);
        harness
            .preload(
                &node,
                (0..3)
                    .map(|i| {
                        (
                            leaf_result(500..510, Observation::NotFound),
                            first + TimeDelta::seconds(i),
                        )
                    })
                    .collect(),
            )
            .await;
        let capture = Capture::with_clock(harness.clock.clone());
        let _guard = capture.install();
        let (watch, view) = harness.start(node.clone()).await;
        let reporter = harness.reporter(&node, &view);
        let watch_task = tokio::spawn(watch.run());
        let reporter_task = tokio::spawn(reporter.run());

        tokio::time::timeout(Duration::from_secs(5), async {
            while capture.with_message("summary").len() < 3 {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .expect("three summaries while the node hangs");
        watch_task.abort();
        reporter_task.abort();

        let logged = summaries(&capture);
        let counts: Vec<_> = logged.iter().take(3).map(|(_, n)| n.as_str()).collect();
        assert_eq!(counts, ["0", "10", "10"]);
        assert_eq!(logged[1].0 - logged[0].0, TimeDelta::seconds(60));
        assert_eq!(logged[2].0 - logged[1].0, TimeDelta::seconds(60));
        assert_eq!(
            fake.requests.lock().unwrap().len(),
            1,
            "one request, still open"
        );
    }

    /// A report with the given cumulative counters.
    fn report_with(
        node: &WatchedNode,
        errors: &[(NotServed, u64)],
        forgotten_rows: u64,
    ) -> crate::report::NodeReport {
        let mut counters = WatchCounters {
            forgotten_rows,
            ..WatchCounters::default()
        };
        for (not_served, times) in errors {
            for _ in 0..*times {
                counters.errors.count(*not_served);
            }
        }
        crate::report::NodeReport {
            node: node.clone(),
            node_lag_blocks: 0,
            summary: crate::view::NodeSummary {
                tip_height: Height(1),
                follow_lag_blocks: 0,
                merklized_state_lag_blocks: None,
                kinds: Vec::new(),
            },
            counters,
            progress: NodeProgress {
                follow_next_height: Height(0),
                sweep: SweepState::NeverSwept,
            },
            sweep_eta: None,
        }
    }

    #[tokio::test]
    async fn the_summary_logs_counter_differences_and_warns_once() {
        let (node, _fake) = serve_fake([]).await;
        let capture = Capture::default();
        let _guard = capture.install();
        let (_sender, receiver) = view::channel();
        let mut reported = ReportedNode::new(NodeViewSlot {
            node: node.clone(),
            view: receiver,
        });
        let transport = NotServed::Failed(crate::outcome::RequestFailure::Transport);
        let server = |code| {
            NotServed::Failed(crate::outcome::RequestFailure::Status(
                crate::outcome::FailedStatus::new(StatusCode::from_u16(code).unwrap()).unwrap(),
            ))
        };
        let errors = [(NotServed::NotFound, 2), (server(500), 1)];
        reported.log(&report_with(&node, &errors, 4));
        let errors = [(NotServed::NotFound, 3), (server(500), 1)];
        reported.log(&report_with(&node, &errors, 4));

        let summaries = capture.with_message("summary");
        assert_eq!(summaries[0].target, "query_watchdog::report");
        assert_eq!(summaries[0].fields["errors"], r#"{"404": 2, "500": 1}"#);
        assert_eq!(summaries[0].fields["forgotten"], "4");
        assert_eq!(summaries[1].fields["errors"], r#"{"404": 1}"#);
        assert_eq!(summaries[1].fields["forgotten"], "0");
        let warnings = capture.with_message("node unhealthy");
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].level, Level::WARN);
        assert_eq!(warnings[0].fields["errors"], r#"{"500": 1}"#);

        let mut all = crate::view::ErrorCounts::default();
        for not_served in [
            NotServed::NotFound,
            server(503),
            server(500),
            server(500),
            transport,
        ] {
            all.count(not_served);
        }
        assert_eq!(
            format!("{all:?}"),
            r#"{"404": 1, "500": 2, "503": 1, "transport": 1}"#
        );
    }

    #[tokio::test]
    async fn the_trim_drops_recovery_keys_below_the_window() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::new(Some(200)).await;
        let start = harness.clock.now();
        let points = (0..5).flat_map(|_| {
            [700, 705, 850, 855].map(|height| (point_result(height, Observation::NotFound), start))
        });
        let range = (0..5).map(|_| (leaf_result(790..810, Observation::NotFound), start));
        harness.preload(&node, points.chain(range).collect()).await;
        let (mut watch, _view) = harness.start(node).await;
        watch.recovery = ChunkQueue::confirmed(watch.missing.set());

        // Window start 800: the queued keys below it are dropped, the range is clipped.
        watch.trim_step().await.unwrap();
        fake.requests.lock().unwrap().clear();
        watch.retry_step().await.unwrap();
        let heads = fake.heads();
        assert!(!heads.is_empty());
        for path in heads {
            let height: u64 = path.rsplit('/').next().unwrap().parse().unwrap();
            assert!(height >= 800, "{path}");
        }
    }

    /// Confirmed `lc_leaf_proof` rows at heights 1 to 3 with their recovery queue queued and the
    /// row of height 2 removed, as a sweep chunk would before its turn.
    async fn recovery_with_a_stale_key(
        fake: &FakeNode,
        node: WatchedNode,
        harness: &Harness,
    ) -> NodeWatch<ManualClock> {
        let _ = fake;
        preload_confirmed_points(harness, &node, 3).await;
        let (mut watch, _view) = harness.start(node).await;
        watch.recovery = ChunkQueue::confirmed(watch.missing.set());
        let now = watch.clock().now();
        watch
            .missing
            .apply(
                MissingUpdate::Observed(point_result(2, Observation::Present)),
                now,
            )
            .await
            .unwrap();
        watch
    }

    #[tokio::test]
    async fn a_stale_recovery_key_that_answers_200_changes_nothing() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::new(None).await;
        let mut watch = recovery_with_a_stale_key(&fake, node, &harness).await;
        watch.retry_step().await.unwrap();
        assert_eq!(watch.missing.set().rows().count(), 0);
        assert!(watch.recovery.is_empty());
    }

    #[tokio::test]
    async fn a_stale_recovery_key_that_answers_404_starts_a_fresh_row() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::new(None).await;
        let mut watch = recovery_with_a_stale_key(&fake, node, &harness).await;
        fake.all_404.store(true, Ordering::SeqCst);
        let now = watch.clock().now();
        watch.retry_step().await.unwrap();
        let rows: Vec<_> = watch.missing.set().rows().collect();
        assert_eq!(rows.len(), 3);
        let fresh = &rows[1];
        assert_eq!(fresh.key.first_unit(), 2);
        assert_eq!(fresh.state.probe_count().get(), 1);
        assert_eq!(fresh.state.first_missing_at(), now);
        assert_eq!(fresh.state.retry().delay, RetryDelay::FIRST);
    }

    #[tokio::test]
    async fn a_failed_refinement_pauses_the_sweep_gate() {
        let (node, fake) = serve_fake([805]).await;
        let harness = Harness::new(Some(200)).await;
        let (mut watch, _view) = harness.start(node).await;

        // The sweep chunk 800..900: the leaf HEAD says 404, the first bisection HEAD fails.
        *fake.leaf_budget.lock().unwrap() = Some(1);
        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(is_paused(&watch));
        assert_eq!(watch.sweep_step().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn two_gaps_are_summed_in_the_gauge() {
        let (node, _fake) = serve_fake((750..760).chain(770..775)).await;
        let harness = Harness::new(None).await;
        let (mut watch, view) = harness.start(node.clone()).await;
        let mut reporter = harness.reporter(&node, &view);

        watch.check_probe(&leaf(700..800)).await.unwrap();
        for _ in 0..2 {
            harness.clock.advance(Duration::from_secs(10));
            watch.retry_step().await.unwrap();
        }
        reporter.tick(1).unwrap();
        assert_eq!(harness.gauge("confirmed_missing", &node, None), Some(15));
    }

    fn server_error() -> crate::outcome::RequestFailure {
        crate::outcome::RequestFailure::Status(
            crate::outcome::FailedStatus::new(StatusCode::INTERNAL_SERVER_ERROR).unwrap(),
        )
    }

    /// 2000 failed `lc_leaf_proof` rows at heights 1000 to 2999, due after a minute.
    async fn preload_failed_points(harness: &Harness, node: &WatchedNode) {
        let start = harness.clock.now();
        let results = (1_000..3_000)
            .map(|height| {
                let failed = Observation::Failed(server_error());
                (point_result(height, failed), start)
            })
            .collect();
        harness.preload(node, results).await;
    }

    #[tokio::test]
    async fn failed_rows_do_not_delay_a_new_gap_from_being_confirmed() {
        let (node, fake) = serve_fake([5]).await;
        fake.failing_points.store(true, Ordering::SeqCst);
        let harness = Harness::with_options(WatchOptions {
            request_interval: Duration::from_millis(100),
            ..options(None)
        })
        .await;
        harness.set_height(&node, 5_000);
        preload_failed_points(&harness, &node).await;
        let (mut watch, _view) = harness.start(node).await;
        harness.clock.advance(Duration::from_secs(61));

        let now = watch.clock().now();
        let gap = leaf_result(5..6, Observation::NotFound);
        watch
            .missing
            .apply(MissingUpdate::Observed(gap), now)
            .await
            .unwrap();
        let confirmed_gap = |watch: &NodeWatch<ManualClock>| {
            let state = watch
                .missing
                .set()
                .rows()
                .find(|row| row.key.kind() == DataKind::Leaf)
                .unwrap()
                .state;
            let retry = state.retry();
            let delay = TimeDelta::from_std(retry.delay.duration()).unwrap();
            // The row was written at its retry time minus its delay.
            state.is_confirmed().then(|| retry.next_at - delay)
        };
        let confirmed_at = loop {
            watch.retry_step().await.unwrap();
            if let Some(at) = confirmed_gap(&watch) {
                break at;
            }
            assert!(watch.clock().now() - now < TimeDelta::seconds(60));
        };
        let waited = confirmed_at - now;
        assert!(waited <= TimeDelta::seconds(17), "{waited}");
        assert!(
            watch.missing.set().rows().count() >= 2_000,
            "failed rows stay"
        );
    }

    #[tokio::test]
    async fn failed_rows_do_not_stop_recovery_from_draining() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::with_options(WatchOptions {
            request_interval: Duration::from_secs(1),
            ..options(None)
        })
        .await;
        harness.set_height(&node, 5_000);
        preload_failed_points(&harness, &node).await;
        preload_confirmed_points(&harness, &node, 40).await;
        let (mut watch, _view) = harness.start(node).await;
        harness.clock.advance(Duration::from_secs(61));

        for _ in 0..8 {
            assert_eq!(watch.retry_step().await.unwrap(), 5);
        }
        assert!(watch.recovery.is_empty());
        let rows: Vec<_> = watch.missing.set().rows().collect();
        assert_eq!(rows.len(), 2_000, "only the confirmed rows were probed");
        assert!(rows.iter().all(|row| !row.state.is_confirmed()));
    }

    #[tokio::test]
    async fn an_epoch_failure_does_not_pause_the_sweep() {
        let (node, fake) = serve_fake([]).await;
        *fake.config.lock().unwrap() =
            r#"{"config":{"epoch_height":30,"epoch_start_block":61}}"#.to_owned();
        fake.failing_stake_table.store(true, Ordering::SeqCst);
        let harness = Harness::new(None).await;
        harness.set_height(&node, 300);
        let epochs = harness
            .watchdog
            .fetch_epochs(std::slice::from_ref(&node))
            .await
            .unwrap();
        let (mut watch, _view) = harness.start_with(node, epochs).await;

        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(fake.requests_to("/v1/node/stake-table/5") > 0);
        assert!(!watch.counters.errors.failures().is_empty());
        assert!(!is_paused(&watch));
    }

    #[tokio::test]
    async fn a_lagging_node_probes_nothing_above_its_own_height_and_never_counts() {
        let (node, fake) = serve_fake(600..1_000).await;
        let harness = Harness::with_options(WatchOptions {
            sweep_once: true,
            ..options(None)
        })
        .await;
        harness.set_height(&node, 500);
        harness.set_height(
            &WatchedNode::new(Url::parse("http://other:1/").unwrap()).unwrap(),
            1_000,
        );
        let (watch, view) = harness.start(node.clone()).await;
        let mut reporter = harness.reporter(&node, &view);
        let task = tokio::spawn(watch.run());
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let done = matches!(
                    view.borrow().as_ref().unwrap().progress.sweep,
                    SweepState::Done { .. }
                );
                if done {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the sweep ends");
        task.abort();

        for path in fake.heads() {
            if let Some(end) = path.strip_prefix("/v1/availability/leaf/") {
                let end: u64 = end.rsplit('/').next().unwrap().parse().unwrap();
                assert!(end <= 490, "{path}");
            }
        }
        reporter.tick(1).unwrap();
        assert_eq!(harness.gauge("confirmed_missing", &node, None), Some(0));
        assert_eq!(harness.gauge("node_lag_blocks", &node, None), Some(500));
    }

    #[tokio::test]
    async fn a_node_that_answers_late_starts_from_its_own_height_without_skipping() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::with_options(WatchOptions {
            sweep_once: true,
            ..options(None)
        })
        .await;
        harness
            .tips_sender
            .send_modify(|tips| tips.observed("other:1", Height(1_000)));
        let (sender, view) = view::channel();
        let watch = harness
            .watchdog
            .start_node(node.clone(), harness.tips.clone(), Epochs::Off, sender)
            .await
            .unwrap();
        let node_store = harness.store.node(&node.label);
        assert!(node_store.load_progress().await.unwrap().is_none());
        let task = tokio::spawn(watch.run());

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(node_store.load_progress().await.unwrap().is_none());
        assert!(matches!(
            view.borrow().as_ref().unwrap().progress.sweep,
            SweepState::NeverSwept
        ));

        harness.set_height(&node, 500);
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let done = matches!(
                    view.borrow().as_ref().unwrap().progress.sweep,
                    SweepState::Done { .. }
                );
                if done {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("the sweep ends");
        task.abort();

        assert_eq!(
            view.borrow().as_ref().unwrap().progress.follow_next_height,
            Height(490)
        );
        let heads = fake.heads();
        assert!(
            heads.contains(&"/v1/availability/leaf/0/100".to_owned()),
            "{heads:?}"
        );
        assert!(
            heads.contains(&"/v1/availability/leaf/400/490".to_owned()),
            "{heads:?}"
        );
    }

    #[tokio::test]
    async fn persisted_rows_above_the_nodes_height_are_dropped_and_never_count() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::new(None).await;
        let start = harness.clock.now() - TimeDelta::seconds(600);
        let rows = (0..5).flat_map(|_| {
            [
                (point_result(900, Observation::NotFound), start),
                (leaf_result(880..900, Observation::NotFound), start),
                (leaf_result(400..410, Observation::NotFound), start),
            ]
        });
        harness.preload(&node, rows.collect()).await;
        harness.set_height(&node, 500);
        let (watch, _view) = harness.start(node).await;

        let kept: Vec<_> = watch
            .missing
            .set()
            .rows()
            .map(|row| row.key.units())
            .collect();
        assert_eq!(kept, vec![400..410]);
        assert_eq!(
            confirmed(&watch),
            10,
            "only the row below the node's height counts"
        );
    }

    #[tokio::test]
    async fn a_sweep_chunk_above_the_nodes_height_waits() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::new(None).await;
        let started_at = harness.clock.now();
        let sweep = RunningSweep {
            started_at,
            start_height: Height(800),
            next_height: Height(800),
            until_height: Height(990),
        };
        let progress = NodeProgress {
            follow_next_height: Height(490),
            sweep: SweepState::Running {
                sweep,
                last_completed_at: None,
            },
        };
        harness
            .store
            .node(&node.label)
            .save_progress(&progress)
            .await
            .unwrap();
        harness.set_height(&node, 500);
        let (mut watch, _view) = harness.start(node).await;

        assert_eq!(watch.sweep_step().await.unwrap(), 0);
        assert!(fake.heads().is_empty());
        assert_eq!(watch.progress, progress);
    }

    #[tokio::test]
    async fn a_drained_recovery_queue_is_not_followed_by_a_stale_representative() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::new(None).await;
        preload_confirmed_points(&harness, &node, 3).await;
        let (mut watch, _view) = harness.start(node).await;
        harness.clock.advance(Duration::from_secs(61));
        watch.recovery = ChunkQueue::confirmed(watch.missing.set());

        assert_eq!(watch.retry_step().await.unwrap(), 3);
        assert_eq!(fake.heads().len(), 3, "no second pass over the first row");
        assert_eq!(watch.missing.set().rows().count(), 0);
    }
}
