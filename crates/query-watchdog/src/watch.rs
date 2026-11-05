use std::{collections::BTreeMap, convert::Infallible, ops::Range, sync::Arc, time::Duration};

use anyhow::{anyhow, bail};
use chrono::{DateTime, TimeDelta, Utc};
use clap::Args;
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::{sync::watch, time::Instant};

use crate::{
    gaps::{Presence, find_gaps},
    kind::{EpochConfig, Height, Probe, RangeKind, next_chunk, probes},
    metrics::WatchdogMetrics,
    missing::{AgedRow, AlertRule, MissingUpdate},
    node::{FetchError, FetchTarget, HeightLookup, WatchedNode},
    outcome::{NotServed, Observation, RangeOutcome, RequestHealth, Unconfirmed, UnitOutcome},
    schedule::{
        EtaOrigin, Frontier, NodeProgress, RunningSweep, SweepGate, SweepSchedule, SweepState,
        SweepWindow,
    },
    store::{NodeStore, PersistedMissingSet, Store},
    system::{Clock, System},
    view::{NodeSummary, NodeView, PAGE_ROWS, SweepEta, ViewSender},
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
    /// Minimum age of missing data for the alert (ERROR on target `query_watchdog::alert`, once a
    /// minute per node and kind); at most 1 year. 60s leaves room for retries at about 5, 15 and
    /// 35 s first.
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
    config: EpochConfig,
}

/// Due missing rows fetched per batch of retries.
const RETRIES_PER_BATCH: usize = 20;
/// Time one iteration spends on retries; sweep and follow keep the rest.
const RETRY_BUDGET: Duration = Duration::from_secs(5);
/// Time between tip polls and between polls of a node's merklized state height.
const TIP_POLL_INTERVAL: Duration = Duration::from_secs(10);
/// Time between summary lines of one node.
const SUMMARY_INTERVAL: Duration = Duration::from_secs(60);

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

    /// Loads or creates progress (clamped to the window start on resume) and the missing set,
    /// logs `watching` or `resumed`, exports gauges and publishes the first view.
    pub async fn start_node(
        &self,
        node: WatchedNode,
        tip: watch::Receiver<Height>,
        epoch_config: Option<EpochConfig>,
        view: ViewSender,
    ) -> anyhow::Result<NodeWatch<C>> {
        let frontier = Frontier::new(*tip.borrow(), self.options.sweep_window());
        let store = self.store.node(&node.label);
        let (progress, start) = match store.load_progress().await? {
            Some(progress) => (
                progress.clamped_to(frontier.window_start()),
                NodeStart::Resumed,
            ),
            None => {
                let progress = NodeProgress::first_run(&frontier);
                store.save_progress(&progress).await?;
                (progress, NodeStart::Fresh)
            },
        };
        let missing = PersistedMissingSet::load(store.clone(), epoch_config).await?;
        if let Some(completed_at) = progress.sweep.last_completed() {
            self.metrics.mark_sweep_completed(&node.label, completed_at);
        }
        let now = self.system.clock.instant();
        let mut watch = NodeWatch {
            watchdog: self.clone(),
            node,
            tip,
            frontier,
            missing,
            store,
            progress,
            gate: SweepGate::Open,
            merklized_state_height: None,
            next_state_poll_at: now,
            last_summary_at: now,
            errors_since_summary: BTreeMap::new(),
            eta_origin: None,
            view,
        };
        watch.reset_eta_origin();
        let summary = watch.summary(self.system.clock.now());
        watch.log_started(start, &summary);
        self.metrics.export_summary(&watch.node.label, &summary)?;
        watch.publish()?;
        Ok(watch)
    }

    /// First tip, errors when no node answers; returns the poller and the receiver every
    /// `NodeWatch` reads.
    pub async fn start_tip_poller(
        &self,
        nodes: Vec<WatchedNode>,
    ) -> anyhow::Result<(TipPoller<C>, watch::Receiver<Height>)> {
        let tip = self.fetch_tip(&nodes).await.ok_or_else(|| {
            let labels: Vec<_> = nodes.iter().map(|node| &node.label).collect();
            anyhow!("no watched node answered v1/node/block-height: {labels:?}")
        })?;
        self.metrics.set_tip_height(tip)?;
        let (sender, receiver) = watch::channel(tip);
        let poller = TipPoller {
            watchdog: self.clone(),
            nodes,
            tip: sender,
        };
        Ok((poller, receiver))
    }

    /// Deletes the stored state (`--reset`) and always logs a WARN.
    pub async fn reset_state(&self) -> anyhow::Result<()> {
        let deleted = self.store.reset().await?;
        tracing::warn!(deleted, "--reset deleted the stored state");
        Ok(())
    }

    /// `v1/config/hotshot` from the first node that answers; `None` when `epoch_height` is 0.
    pub async fn fetch_epoch_config(
        &self,
        nodes: &[WatchedNode],
    ) -> anyhow::Result<Option<EpochConfig>> {
        let mut failures = Vec::new();
        for node in nodes {
            match self
                .system
                .http
                .get_json::<NetworkConfig>(node, "v1/config/hotshot")
                .await
            {
                Ok(NetworkConfig { config }) => {
                    return Ok((config.epoch_height != 0).then_some(config));
                },
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

    /// Max `v1/node/block-height` across `nodes`; `None` when none answers.
    async fn fetch_tip(&self, nodes: &[WatchedNode]) -> Option<Height> {
        let mut tip = None;
        for node in nodes {
            match self
                .system
                .http
                .get_json::<u64>(node, "v1/node/block-height")
                .await
            {
                Ok(height) => tip = tip.max(Some(Height(height))),
                Err(error) => {
                    tracing::debug!(node = %node.label, %error, "tip request failed");
                    self.metrics
                        .count_fetch_error(&node.label, FetchTarget::BlockHeight, &error);
                },
            }
        }
        tip
    }
}

/// Publishes the max block height of all watched nodes every `TIP_POLL_INTERVAL`.
pub struct TipPoller<C: Clock> {
    watchdog: Watchdog<C>,
    nodes: Vec<WatchedNode>,
    tip: watch::Sender<Height>,
}

impl<C: Clock> TipPoller<C> {
    /// Polls forever, returning only an error; a round where no node answers keeps the tip.
    pub async fn run(self) -> anyhow::Result<Infallible> {
        loop {
            self.watchdog.system.clock.sleep(TIP_POLL_INTERVAL).await;
            if let Some(tip) = self.watchdog.fetch_tip(&self.nodes).await {
                self.watchdog.metrics.set_tip_height(tip)?;
                self.tip.send_replace(tip);
            }
        }
    }
}

/// First run or resume of a node watch, for the start line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeStart {
    Fresh,
    Resumed,
}

/// Checker of one watched node: follow, retries and sweep, one request at a time.
pub struct NodeWatch<C: Clock> {
    watchdog: Watchdog<C>,
    node: WatchedNode,
    tip: watch::Receiver<Height>,
    /// Set once per loop iteration from `tip`.
    frontier: Frontier,
    missing: PersistedMissingSet,
    store: NodeStore,
    progress: NodeProgress,
    gate: SweepGate,
    merklized_state_height: Option<Height>,
    next_state_poll_at: Instant,
    last_summary_at: Instant,
    errors_since_summary: BTreeMap<String, u64>,
    eta_origin: Option<EtaOrigin>,
    view: ViewSender,
}

impl<C: Clock> NodeWatch<C> {
    fn clock(&self) -> &C {
        &self.watchdog.system.clock
    }

    /// Runs the loop until a store error.
    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            let tip = *self.tip.borrow_and_update();
            self.frontier = Frontier::new(tip, self.watchdog.options.sweep_window());
            self.poll_state_height().await;
            let mut sent = self.follow_step().await?;
            sent += self.retry_step().await?;
            sent += self.sweep_step().await?;
            self.summarize_if_due().await?;
            self.publish()?;
            if sent == 0 {
                let wait = self
                    .next_state_poll_at
                    .saturating_duration_since(self.clock().instant());
                tokio::select! {
                    () = self.watchdog.system.clock.sleep(wait) => {},
                    changed = self.tip.changed() => changed?,
                }
            }
        }
    }

    /// The `watching` or `resumed` line.
    fn log_started(&self, start: NodeStart, summary: &NodeSummary) {
        let node = &self.node.label;
        let tip_height = self.frontier.tip().0;
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
            Ok(height) => self.merklized_state_height = Some(Height(height)),
            Err(error) => self.record_error(FetchTarget::MerklizedStateHeight, &error),
        }
    }

    /// Checks the next follow chunk below `frontier.settled_end`; returns requests sent.
    async fn follow_step(&mut self) -> anyhow::Result<usize> {
        let Some(chunk) = next_chunk(
            self.progress.follow_next_height,
            self.frontier.settled_end(),
        ) else {
            return Ok(0);
        };
        let sent = self.check_chunk(&chunk).await?;
        self.commit_progress(NodeProgress {
            follow_next_height: chunk.end,
            ..self.progress
        })
        .await?;
        Ok(sent)
    }

    /// Re-checks `MissingSet::due` probes in batches until none is due or `RETRY_BUDGET` is
    /// spent.
    async fn retry_step(&mut self) -> anyhow::Result<usize> {
        let deadline = self.clock().instant() + RETRY_BUDGET;
        let mut checked = 0;
        while self.clock().instant() < deadline {
            let due = self
                .missing
                .set()
                .due(self.clock().now(), RETRIES_PER_BATCH);
            if due.is_empty() {
                break;
            }
            for probe in &due {
                if self.clock().instant() >= deadline {
                    break;
                }
                self.check_probe(probe).await?;
                checked += 1;
            }
        }
        Ok(checked)
    }

    /// Reschedules, starts, advances or finishes the sweep when `gate.admits`; returns requests
    /// sent.
    async fn sweep_step(&mut self) -> anyhow::Result<usize> {
        let now = self.clock().instant();
        if !self.gate.admits(now) {
            return Ok(0);
        }
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
        let sent = self.check_chunk(&chunk).await?;
        self.commit_progress(NodeProgress {
            sweep: SweepState::Running {
                sweep: sweep.advanced(chunk.end),
                last_completed_at,
            },
            ..self.progress
        })
        .await?;
        Ok(sent)
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

    /// Finishes `sweep`, sets the gauge, logs `sweep completed` (WARN on a backward
    /// clock step).
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
        self.watchdog
            .metrics
            .mark_sweep_completed(&self.node.label, completed_at);
        let summary = self.summary(completed_at);
        tracing::info!(
            node = %self.node.label,
            duration = %humantime::format_duration(duration),
            %summary,
            "sweep completed"
        );
        Ok(())
    }

    /// `check_probe` for every probe of the chunk, then `gate.after_chunk` with the combined
    /// health.
    async fn check_chunk(&mut self, chunk: &Range<Height>) -> anyhow::Result<usize> {
        let to_check = probes(chunk, self.missing.set().epoch_config());
        let mut health = RequestHealth::Answered;
        for probe in &to_check {
            health = health.and(self.check_probe(probe).await?);
        }
        let now = self.clock().instant();
        self.gate = self.gate.after_chunk(health, now);
        Ok(to_check.len())
    }

    /// `observe` then `missing.apply`; returns the observation's health.
    async fn check_probe(&mut self, probe: &Probe) -> anyhow::Result<RequestHealth> {
        let observation = self.observe(probe).await;
        let health = observation.health();
        let now = self.clock().now();
        self.missing
            .apply(MissingUpdate::Observed(observation), now)
            .await?;
        Ok(health)
    }

    /// Sends the probe and, for a range 404, runs `find_gaps` with `missing.set().known(..)`;
    /// counts errors.
    async fn observe(&mut self, probe: &Probe) -> Observation {
        let result = self.send_probe(probe).await;
        if let Err(error) = &result {
            self.record_error(FetchTarget::Data(probe.kind()), error);
        }
        match probe {
            Probe::Range { kind, heights } => {
                let outcome = match result.map_err(|error| error.not_served()) {
                    Ok(()) => RangeOutcome::Present,
                    Err(NotServed::Failed(failure)) => RangeOutcome::Failed(failure),
                    Err(NotServed::NotFound) => {
                        let known = self.missing.set().known(*kind, heights);
                        let mut prober = RangeProber {
                            watch: self,
                            kind: *kind,
                        };
                        RangeOutcome::Refined(find_gaps(&mut prober, heights.clone(), known).await)
                    },
                };
                Observation::Range {
                    kind: *kind,
                    heights: heights.clone(),
                    outcome,
                }
            },
            Probe::Point { kind, height } => Observation::Point {
                kind: *kind,
                height: *height,
                outcome: unit_outcome(result),
            },
            Probe::Epoch { kind, epoch } => Observation::Epoch {
                kind: *kind,
                epoch: *epoch,
                outcome: unit_outcome(result),
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
        self.watchdog
            .system
            .http
            .head(&self.node, &probe.route(), probe.waits_server_side())
            .await
    }

    /// GETs the 404 of the range probe and reads the first missing height; `None` when the range
    /// is served after all.
    async fn missing_height(
        &mut self,
        kind: RangeKind,
        heights: &Range<Height>,
    ) -> Result<Option<Height>, Unconfirmed> {
        self.clock()
            .sleep(self.watchdog.options.request_interval)
            .await;
        self.watchdog
            .metrics
            .count_probe(&self.node.label, kind.into());
        let route = Probe::Range {
            kind,
            heights: heights.clone(),
        }
        .route();
        match self
            .watchdog
            .system
            .http
            .lookup_missing_height(&self.node, &route)
            .await
        {
            Ok(HeightLookup::Named(height)) => Ok(Some(height)),
            Ok(HeightLookup::Present) => Ok(None),
            Ok(HeightLookup::NoHeight) => {
                tracing::debug!(node = %self.node.label, route, "404 body names no height");
                Err(Unconfirmed::NoHeight)
            },
            Err(error) => Err(self.refinement_failed(kind, &error)),
        }
    }

    /// Counts a failed refinement request; the range stays chunk level.
    fn refinement_failed(&mut self, kind: RangeKind, error: &FetchError) -> Unconfirmed {
        self.record_error(FetchTarget::Data(kind.into()), error);
        error.not_served().into()
    }

    /// Counts a failed request in metrics and the summary window.
    fn record_error(&mut self, target: FetchTarget, error: &FetchError) {
        tracing::debug!(node = %self.node.label, target = target.label(), %error, "request failed");
        self.watchdog
            .metrics
            .count_fetch_error(&self.node.label, target, error);
        *self
            .errors_since_summary
            .entry(error.status_label().to_owned())
            .or_default() += 1;
    }

    /// Saves `progress` and makes it current; the only progress write.
    async fn commit_progress(&mut self, progress: NodeProgress) -> anyhow::Result<()> {
        self.store.save_progress(&progress).await?;
        self.progress = progress;
        Ok(())
    }

    /// Sends the view with `send_replace` (never fails without receivers) and sets the progress
    /// gauges.
    fn publish(&self) -> anyhow::Result<()> {
        let now = self.clock().now();
        let remaining = self.sweep_eta();
        let sweep_eta = remaining.and_then(|remaining| {
            let finish = now.checked_add_signed(TimeDelta::from_std(remaining).ok()?)?;
            Some(SweepEta { remaining, finish })
        });
        self.view.send_replace(Some(NodeView {
            summary: self.summary(now),
            progress: self.progress,
            next_sweep_start: self
                .progress
                .sweep
                .next_start(self.watchdog.options.sweep_schedule()),
            sweep_eta,
            listed: self.missing.set().listed(now, PAGE_ROWS),
        }));
        self.watchdog
            .metrics
            .export_progress(&self.node.label, &self.progress, remaining)
    }

    /// Snapshot from the frontier, progress, state height and `MissingSet::summary(now)`.
    fn summary(&self, now: DateTime<Utc>) -> NodeSummary {
        let tip = self.frontier.tip();
        NodeSummary {
            tip_height: tip,
            follow_lag_blocks: tip.blocks_since(self.progress.follow_next_height),
            merklized_state_lag_blocks: self
                .merklized_state_height
                .map(|height| tip.blocks_since(height)),
            kinds: self.missing.set().summary(now),
        }
    }

    /// Once per `SUMMARY_INTERVAL`: trim, summary line, alerts, unhealthy WARN, gauges.
    async fn summarize_if_due(&mut self) -> anyhow::Result<()> {
        let now = self.clock().instant();
        if now.saturating_duration_since(self.last_summary_at) < SUMMARY_INTERVAL {
            return Ok(());
        }
        self.last_summary_at = now;
        let forgotten = self.trim_to_window().await?;
        let now = self.clock().now();
        let summary = self.summary(now);
        tracing::info!(
            node = %self.node.label,
            errors = ?self.errors_since_summary,
            forgotten,
            sweep_eta = self
                .sweep_eta()
                .map(|eta| humantime::format_duration(eta).to_string()),
            %summary,
            "summary"
        );
        self.alert_missing(&summary, now);
        self.warn_unhealthy();
        self.watchdog
            .metrics
            .export_summary(&self.node.label, &summary)?;
        self.errors_since_summary.clear();
        Ok(())
    }

    /// Retention only: clamps progress and applies `ForgetBelow(frontier.window_start)`; returns
    /// rows removed.
    async fn trim_to_window(&mut self) -> anyhow::Result<Option<usize>> {
        if self.watchdog.options.sweep_window() == SweepWindow::Archive {
            return Ok(None);
        }
        let window_start = self.frontier.window_start();
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
        Ok(Some(diff.rows_removed))
    }

    /// `MissingSet::alert_candidates` with the options' rule and `frontier.settled_end`.
    fn alerts(&self, now: DateTime<Utc>) -> Vec<AgedRow> {
        self.missing.set().alert_candidates(
            self.watchdog.options.alert_rule(),
            self.frontier.settled_end(),
            now,
        )
    }

    /// One ERROR on target `query_watchdog::alert` per candidate; fields as at HEAD.
    fn alert_missing(&self, summary: &NodeSummary, now: DateTime<Utc>) {
        for candidate in self.alerts(now) {
            let key = &candidate.row.key;
            let kind = key.kind();
            let missing = summary
                .kinds
                .iter()
                .find(|summary| summary.kind == kind)
                .expect("the summary has every kind")
                .missing_count;
            let range_start = key.first_unit();
            // Node URLs are internal and carry no credentials.
            tracing::error!(
                target: "query_watchdog::alert",
                node = %self.node.label,
                kind = <&str>::from(kind),
                missing,
                oldest_age_seconds = candidate.age.as_secs(),
                failed_probes = candidate.row.state.failed_probes,
                range_start,
                range_end = range_start + key.unit_count(),
                url = %self.node.route_url(&key.first_probe().route()),
                "missing data"
            );
        }
    }

    /// One WARN when requests of the last minute failed other than with 404; not an alert.
    fn warn_unhealthy(&self) {
        let failures: BTreeMap<_, _> = self
            .errors_since_summary
            .iter()
            .filter(|(status, _)| status.as_str() != StatusCode::NOT_FOUND.as_str())
            .collect();
        if !failures.is_empty() {
            tracing::warn!(node = %self.node.label, errors = ?failures, "node unhealthy");
        }
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

/// What a point or epoch HEAD established.
fn unit_outcome(result: Result<(), FetchError>) -> UnitOutcome {
    match result {
        Ok(()) => UnitOutcome::Present,
        Err(error) => UnitOutcome::Missing(error.not_served()),
    }
}

/// Asks one node which heights of one range kind it serves, for `find_gaps`.
struct RangeProber<'a, C: Clock> {
    watch: &'a mut NodeWatch<C>,
    kind: RangeKind,
}

impl<C: Clock> Presence for RangeProber<'_, C> {
    async fn first_missing(
        &mut self,
        heights: Range<Height>,
        known_missing: bool,
    ) -> Result<Option<Height>, Unconfirmed> {
        if known_missing {
            return self.watch.missing_height(self.kind, &heights).await;
        }
        let probe = Probe::Range {
            kind: self.kind,
            heights: heights.clone(),
        };
        match self.watch.send_probe(&probe).await {
            Ok(()) => Ok(None),
            Err(error) if error.not_served() == NotServed::NotFound => {
                self.watch.missing_height(self.kind, &heights).await
            },
            Err(error) => Err(self.watch.refinement_failed(self.kind, &error)),
        }
    }

    async fn is_present(&mut self, height: Height) -> Result<bool, Unconfirmed> {
        let probe = Probe::Range {
            kind: self.kind,
            heights: height..height.next(),
        };
        match self.watch.send_probe(&probe).await {
            Ok(()) => Ok(true),
            Err(error) if error.not_served() == NotServed::NotFound => Ok(false),
            Err(error) => Err(self.watch.refinement_failed(self.kind, &error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        fmt::Debug,
        str::FromStr,
        sync::{
            Mutex,
            atomic::{AtomicBool, Ordering},
        },
    };

    use axum::{
        Router,
        extract::{Path, State},
        http::{Method, Uri},
        routing::get,
    };
    use pretty_assertions::assert_eq;
    use prometheus::Registry;
    use sqlx::sqlite::SqliteConnectOptions;
    use tokio::net::TcpListener;
    use tracing::{Event, Subscriber, field::Field};
    use tracing_subscriber::{
        Layer,
        layer::{Context, SubscriberExt},
    };
    use url::Url;

    use super::*;
    use crate::{
        kind::{DataKind, PointKind},
        missing::RowKey,
        node::NodeHttp,
        outcome::MissingReason,
        schedule::{CompletedSweep, RunningSweep},
        system::ManualClock,
        view::{self, ViewReceiver},
    };

    /// A node serving every leaf except the `missing` heights, recording what it is asked.
    #[derive(Clone)]
    struct FakeNode {
        missing: Arc<Mutex<BTreeSet<u64>>>,
        /// Every request answers 500 while set.
        failing: Arc<AtomicBool>,
        /// The leaf route answers 500 while set; other routes still answer.
        failing_leaf: Arc<AtomicBool>,
        /// GET requests answer 500 while set; HEAD still answers.
        failing_gets: Arc<AtomicBool>,
        requests: Arc<Mutex<Vec<(Method, String)>>>,
    }

    /// Like the availability range route: 404 naming the first missing height, else 200.
    async fn fake_leaf_range(
        State(node): State<FakeNode>,
        method: Method,
        uri: Uri,
        Path((from, until)): Path<(u64, u64)>,
    ) -> (StatusCode, String) {
        let is_get = method == Method::GET;
        node.requests
            .lock()
            .unwrap()
            .push((method, uri.path().to_owned()));
        if node.failing.load(Ordering::SeqCst)
            || node.failing_leaf.load(Ordering::SeqCst)
            || (is_get && node.failing_gets.load(Ordering::SeqCst))
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, String::new());
        }
        let first_missing = node
            .missing
            .lock()
            .unwrap()
            .range(from..until)
            .next()
            .copied();
        match first_missing {
            Some(height) => (
                StatusCode::NOT_FOUND,
                format!(r#"{{"Custom":{{"message":"leaf {height} not found","status":404}}}}"#),
            ),
            None => (StatusCode::OK, String::new()),
        }
    }

    /// Every other route: present, unless the node is failing.
    async fn fake_other(State(node): State<FakeNode>) -> StatusCode {
        if node.failing.load(Ordering::SeqCst) {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::OK
        }
    }

    async fn serve_fake(missing: impl IntoIterator<Item = u64>) -> (WatchedNode, FakeNode) {
        let fake = FakeNode {
            missing: Arc::new(Mutex::new(missing.into_iter().collect())),
            failing: Arc::default(),
            failing_leaf: Arc::default(),
            failing_gets: Arc::default(),
            requests: Arc::default(),
        };
        let app = Router::new()
            .route("/v1/availability/leaf/{from}/{until}", get(fake_leaf_range))
            .fallback(fake_other)
            .with_state(fake.clone());
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
        /// Keeps the tip channel open.
        _tip_sender: watch::Sender<Height>,
        tip: watch::Receiver<Height>,
    }

    impl Harness {
        /// A watchdog over an in-memory store, a manual clock and no pacing.
        async fn new(retention_blocks: Option<u64>) -> Self {
            Self::with_options(WatchOptions {
                sweep_interval: Duration::from_secs(1),
                sweep_once: false,
                request_interval: Duration::ZERO,
                retention_blocks,
                alert_missing_after: Duration::ZERO,
            })
            .await
        }

        async fn with_options(options: WatchOptions) -> Self {
            let registry = Registry::new();
            let store = Store::connect(SqliteConnectOptions::from_str("sqlite::memory:").unwrap())
                .await
                .unwrap();
            let clock = ManualClock::new(DateTime::from_timestamp(1_000_000, 0).unwrap());
            let system = System {
                clock: clock.clone(),
                http: NodeHttp::new().unwrap(),
            };
            let watchdog = Watchdog::new(
                system,
                options,
                Arc::new(WatchdogMetrics::new(&registry).unwrap()),
                store.clone(),
            );
            let (tip_sender, tip) = watch::channel(Height(1_000));
            Self {
                watchdog,
                store,
                registry,
                clock,
                _tip_sender: tip_sender,
                tip,
            }
        }

        async fn start(&self, node: WatchedNode) -> (NodeWatch<ManualClock>, ViewReceiver) {
            self.start_with(node, None).await
        }

        async fn start_with(
            &self,
            node: WatchedNode,
            epoch_config: Option<EpochConfig>,
        ) -> (NodeWatch<ManualClock>, ViewReceiver) {
            let (sender, receiver) = view::channel();
            let watch = self
                .watchdog
                .start_node(node, self.tip.clone(), epoch_config, sender)
                .await
                .unwrap();
            (watch, receiver)
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

    fn leaf_missing<C: Clock>(watch: &NodeWatch<C>) -> u64 {
        watch.summary(watch.clock().now()).kinds[0].missing_count
    }

    fn alerting_ranges<C: Clock>(watch: &NodeWatch<C>) -> Vec<Range<Height>> {
        let alerts = watch.alerts(watch.clock().now());
        alerts
            .into_iter()
            .map(|aged| match aged.row.key {
                RowKey::Range { heights, .. } => heights,
                other => panic!("{other:?}"),
            })
            .collect()
    }

    fn is_running<C: Clock>(watch: &NodeWatch<C>) -> bool {
        matches!(watch.progress.sweep, SweepState::Running { .. })
    }

    fn is_paused<C: Clock>(watch: &NodeWatch<C>) -> bool {
        matches!(watch.gate, SweepGate::Paused { .. })
    }

    #[tokio::test]
    async fn a_failed_range_probe_is_refined_with_one_head_per_range() {
        let (node, fake) = serve_fake(103..106).await;
        let harness = Harness::new(None).await;
        let (mut watch, _view) = harness.start(node).await;

        watch.check_probe(&leaf(100..110)).await.unwrap();

        let requests = fake.requests.lock().unwrap().clone();
        let paths = |wanted: Method| -> Vec<String> {
            requests
                .iter()
                .filter(|(method, _)| *method == wanted)
                .map(|(_, path)| path.clone())
                .collect()
        };
        let chunk_path = "/v1/availability/leaf/100/110";
        // The chunk, the gallop at 104, 105 and 107, the binary step at 106, the rest from 106.
        let expected_heads = [
            chunk_path,
            "/v1/availability/leaf/104/105",
            "/v1/availability/leaf/105/106",
            "/v1/availability/leaf/107/108",
            "/v1/availability/leaf/106/107",
            "/v1/availability/leaf/106/110",
        ];
        assert_eq!(paths(Method::HEAD), expected_heads);
        assert_eq!(paths(Method::GET), [chunk_path]);

        let summary = watch.summary(watch.clock().now());
        assert_eq!(summary.kinds[0].missing_count, 3);
        let oldest = summary.kinds[0].oldest.as_ref().unwrap();
        assert_eq!(
            oldest.key,
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
        assert_eq!(probes_total, 7, "six HEADs and one GET");
    }

    #[tokio::test]
    async fn a_gap_is_alerted_then_cleared_as_the_node_fills_it() {
        let (node, fake) = serve_fake(750..760).await;
        let harness = Harness::new(None).await;
        let (mut watch, _view) = harness.start(node).await;
        let gap = leaf(750..760);

        // The chunk probe finds the exact gap; two retries bring it to 3 failed 404 probes.
        watch.check_probe(&leaf(700..800)).await.unwrap();
        assert_eq!(leaf_missing(&watch), 10);
        for _ in 0..2 {
            watch.check_probe(&gap).await.unwrap();
        }
        assert_eq!(alerting_ranges(&watch), [h(750..760)]);

        // A node that answers 500 is unhealthy, not missing data: the 404 row stays as it was.
        fake.failing.store(true, Ordering::SeqCst);
        watch.check_probe(&gap).await.unwrap();
        assert_eq!(alerting_ranges(&watch), [h(750..760)]);
        let row = watch.missing.set().rows().next().unwrap();
        assert_eq!(row.state.reason, MissingReason::NotFound);
        assert_eq!(row.state.failed_probes, 3);
        assert_eq!(watch.errors_since_summary.get("500"), Some(&1));
        fake.failing.store(false, Ordering::SeqCst);

        // Half of the gap is filled: the next retry shrinks the row, which continues the run of
        // 404 probes and keeps alerting.
        fake.missing.lock().unwrap().retain(|height| *height < 755);
        watch.check_probe(&gap).await.unwrap();
        assert_eq!(leaf_missing(&watch), 5);
        let rest = leaf(750..755);
        assert_eq!(alerting_ranges(&watch), [h(750..755)]);
        let row = watch.missing.set().rows().next().unwrap();
        assert_eq!(
            row.state.failed_probes, 4,
            "the 500 did not break the run of 404s"
        );

        // The rest is filled: the retry clears the row and the alert.
        fake.missing.lock().unwrap().clear();
        watch.check_probe(&rest).await.unwrap();
        assert_eq!(leaf_missing(&watch), 0);
        assert!(alerting_ranges(&watch).is_empty());
    }

    /// Alert events collected from the `query_watchdog::alert` target.
    #[derive(Clone, Default)]
    struct AlertEvents(Arc<Mutex<Vec<BTreeMap<String, String>>>>);

    #[derive(Default)]
    struct Fields(BTreeMap<String, String>);

    impl tracing::field::Visit for Fields {
        fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
            self.0.insert(field.name().to_owned(), format!("{value:?}"));
        }

        fn record_str(&mut self, field: &Field, value: &str) {
            self.0.insert(field.name().to_owned(), value.to_owned());
        }
    }

    impl<S: Subscriber> Layer<S> for AlertEvents {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            if event.metadata().target() == "query_watchdog::alert" {
                let mut fields = Fields::default();
                event.record(&mut fields);
                self.0.lock().unwrap().push(fields.0);
            }
        }
    }

    #[tokio::test]
    async fn the_summary_logs_one_alert_per_confirmed_gap() {
        let (node, _fake) = serve_fake(750..760).await;
        let events = AlertEvents::default();
        let _guard =
            tracing::subscriber::set_default(tracing_subscriber::registry().with(events.clone()));
        let harness = Harness::new(None).await;
        let (mut watch, _view) = harness.start(node.clone()).await;
        watch.check_probe(&leaf(700..800)).await.unwrap();
        for _ in 0..2 {
            watch.check_probe(&leaf(750..760)).await.unwrap();
        }

        harness.clock.advance(SUMMARY_INTERVAL);
        watch.summarize_if_due().await.unwrap();

        let events = events.0.lock().unwrap().clone();
        assert_eq!(events.len(), 1, "{events:?}");
        let event = &events[0];
        let expected = [
            ("node", node.label.clone()),
            ("kind", "leaf".to_owned()),
            ("missing", "10".to_owned()),
            ("failed_probes", "3".to_owned()),
            ("range_start", "750".to_owned()),
            ("range_end", "760".to_owned()),
            (
                "url",
                node.route_url("v1/availability/leaf/750/760").to_string(),
            ),
        ];
        for (name, value) in expected {
            assert_eq!(event.get(name), Some(&value), "{name} in {event:?}");
        }
    }

    /// Rows of an aggregate point probe at heights `1..=count`, all due after five seconds.
    async fn add_due_points(watch: &mut NodeWatch<ManualClock>, count: u64) {
        for height in 1..=count {
            let observation = Observation::Point {
                kind: PointKind::LcLeafProof,
                height: Height(height),
                outcome: UnitOutcome::Missing(NotServed::NotFound),
            };
            let now = watch.clock().now();
            watch
                .missing
                .apply(MissingUpdate::Observed(observation), now)
                .await
                .unwrap();
        }
        watch.clock().sleep(Duration::from_secs(5)).await;
    }

    #[tokio::test]
    async fn the_retries_stop_at_their_budget() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::with_options(WatchOptions {
            sweep_interval: Duration::from_secs(1),
            sweep_once: false,
            request_interval: Duration::from_secs(2),
            retention_blocks: None,
            alert_missing_after: Duration::ZERO,
        })
        .await;
        let (mut watch, _view) = harness.start(node).await;
        add_due_points(&mut watch, 10).await;

        // 2 s per request against a budget of 5 s: probes at 0, 2 and 4 s.
        assert_eq!(watch.retry_step().await.unwrap(), 3);
        assert_eq!(
            watch.missing.set().rows().count(),
            7,
            "present points are cleared"
        );
    }

    #[tokio::test]
    async fn the_retries_run_in_batches_until_nothing_is_due() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::new(None).await;
        let (mut watch, _view) = harness.start(node).await;
        let count = RETRIES_PER_BATCH as u64 + 5;
        add_due_points(&mut watch, count).await;

        assert_eq!(watch.retry_step().await.unwrap() as u64, count);
        assert_eq!(watch.missing.set().rows().count(), 0);
    }

    #[tokio::test]
    async fn a_500_on_a_404_row_is_retried_once_and_then_waits() {
        let (node, fake) = serve_fake(750..760).await;
        let harness = Harness::with_options(WatchOptions {
            sweep_interval: Duration::from_secs(1),
            sweep_once: false,
            request_interval: Duration::from_secs(1),
            retention_blocks: None,
            alert_missing_after: Duration::ZERO,
        })
        .await;
        let (mut watch, _view) = harness.start(node).await;
        watch.check_probe(&leaf(700..800)).await.unwrap();
        for _ in 0..2 {
            watch.check_probe(&leaf(750..760)).await.unwrap();
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
        assert_eq!(row.state.reason, MissingReason::NotFound);
        assert_eq!(row.state.failed_probes, 3);
        assert!(row.state.next_retry_at > watch.clock().now());
        assert_eq!(alerting_ranges(&watch), [h(750..760)]);
    }

    #[tokio::test]
    async fn a_one_time_sweep_ends_in_done_and_stays_quiet() {
        let (node, _fake) = serve_fake([]).await;
        let harness = Harness::with_options(WatchOptions {
            sweep_interval: Duration::from_secs(3_600),
            sweep_once: true,
            request_interval: Duration::ZERO,
            retention_blocks: Some(200),
            alert_missing_after: Duration::ZERO,
        })
        .await;
        let (mut watch, _view) = harness.start(node.clone()).await;

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
    async fn the_sweep_pauses_while_follow_or_sweep_probes_fail_and_resumes_after() {
        let (node, fake) = serve_fake([905]).await;
        let harness = Harness::new(Some(200)).await;
        let (mut watch, _view) = harness.start(node).await;
        let chunk = h(900..910);
        let probe = leaf(900..910);

        // A retry probe that fails neither sets nor clears the pause.
        fake.failing.store(true, Ordering::SeqCst);
        assert_eq!(
            watch.check_probe(&probe).await.unwrap(),
            RequestHealth::Failed
        );
        assert!(!is_paused(&watch));
        assert!(watch.sweep_step().await.unwrap() > 0, "the sweep goes on");
        watch.progress.sweep = SweepState::NeverSwept;

        // A failing chunk probe pauses the sweep: no sweep starts.
        watch.check_chunk(&chunk).await.unwrap();
        assert!(is_paused(&watch));
        assert_eq!(watch.sweep_step().await.unwrap(), 0);
        assert_eq!(watch.progress.sweep, SweepState::NeverSwept);

        // A retry probe that answers does not resume it; a chunk probe that answers does.
        fake.failing.store(false, Ordering::SeqCst);
        assert_eq!(
            watch.check_probe(&probe).await.unwrap(),
            RequestHealth::Answered
        );
        assert!(is_paused(&watch));
        watch.check_chunk(&chunk).await.unwrap();
        assert!(!is_paused(&watch));
        assert!(watch.sweep_step().await.unwrap() > 0);
        assert!(is_running(&watch));
    }

    #[tokio::test]
    async fn one_failed_probe_in_a_chunk_pauses_the_sweep_until_a_chunk_is_fully_answered() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::new(Some(200)).await;
        let (mut watch, _view) = harness.start(node).await;
        let chunk = h(900..910);

        // The leaf probe, first of five, fails; the four after it answer.
        fake.failing_leaf.store(true, Ordering::SeqCst);
        watch.check_chunk(&chunk).await.unwrap();
        assert!(is_paused(&watch), "any failed probe pauses");
        assert_eq!(watch.sweep_step().await.unwrap(), 0);

        fake.failing_leaf.store(false, Ordering::SeqCst);
        watch.check_chunk(&chunk).await.unwrap();
        assert!(!is_paused(&watch));
    }

    #[tokio::test]
    async fn a_paused_sweep_rechecks_the_node_once_a_minute_and_resumes() {
        let (node, fake) = serve_fake([]).await;
        let harness = Harness::new(Some(400)).await;
        let (mut watch, _view) = harness.start(node).await;

        // A flat tip gives follow nothing to probe, so only the recheck can clear the pause.
        fake.failing.store(true, Ordering::SeqCst);
        watch.check_chunk(&h(900..910)).await.unwrap();
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
    async fn a_failed_refinement_pauses_the_sweep() {
        let (node, fake) = serve_fake([905]).await;
        let harness = Harness::new(Some(200)).await;
        let (mut watch, _view) = harness.start(node).await;

        // The chunk HEAD says 404, but the GET naming the height fails with 500.
        fake.failing_gets.store(true, Ordering::SeqCst);
        assert_eq!(
            watch.check_probe(&leaf(900..910)).await.unwrap(),
            RequestHealth::Failed
        );
        assert_eq!(watch.errors_since_summary.get("500"), Some(&1));
        // A chunk of follow or sweep with such a probe pauses the sweep.
        watch.check_chunk(&h(900..910)).await.unwrap();
        assert!(is_paused(&watch));
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
                request_interval: Duration::ZERO,
                retention_blocks: Some(200),
                alert_missing_after: Duration::ZERO,
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
    async fn the_watch_loop_follows_sweeps_alerts_and_trims() {
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
        let summary = watch.summary(watch.clock().now());
        let leaf_summary = &summary.kinds[0];
        assert_eq!(
            (leaf_summary.kind, leaf_summary.missing_count),
            (DataKind::Leaf, 10)
        );

        // Two more failed 404 probes make the row alert: 3 probes, old enough, below the margin.
        for _ in 0..2 {
            watch.check_probe(&leaf(750..760)).await.unwrap();
        }
        assert_eq!(alerting_ranges(&watch), [h(750..760)]);
        watch.frontier = Frontier::new(Height(755), window);
        assert!(alerting_ranges(&watch).is_empty(), "inside the tip margin");

        // A later tip moves follow and the window; rows below the window are forgotten.
        watch.frontier = Frontier::new(Height(1_500), window);
        assert_eq!(watch.follow_step().await.unwrap(), 5);
        assert_eq!(watch.progress.follow_next_height, Height(1_090));
        assert_eq!(watch.trim_to_window().await.unwrap(), Some(1));
        assert_eq!(watch.progress.follow_next_height, Height(1_200));
        assert_eq!(
            watch.store.load_progress().await.unwrap().unwrap(),
            watch.progress
        );
        assert_eq!(leaf_missing(&watch), 0);
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
    async fn the_watch_publishes_its_view() {
        let (node, _fake) = serve_fake(750..760).await;
        let harness = Harness::new(None).await;
        let (mut watch, view) = harness.start(node).await;
        {
            let first = view.borrow();
            let first = first
                .as_ref()
                .expect("the first view is published at start");
            assert_eq!(first.summary.tip_height, Height(1_000));
            assert_eq!(first.progress, watch.progress);
        }
        watch.check_probe(&leaf(700..800)).await.unwrap();
        watch.publish().unwrap();
        let view = view.borrow();
        let view = view.as_ref().unwrap();
        assert_eq!(view.listed.total, 1);
        assert_eq!(view.summary.kinds[0].missing_count, 10);
    }
}
