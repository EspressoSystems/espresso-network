use std::{collections::BTreeMap, fmt, ops::Range, sync::Arc, time::Duration};

use anyhow::{anyhow, bail};
use chrono::{TimeDelta, Utc};
use clap::Args;
use futures::future::join_all;
use reqwest::StatusCode;
use serde::{Deserialize, de::DeserializeOwned};
use tokio::time::Instant;
use url::Url;

use crate::{
    chunk::{DataKind, EpochConfig, Probe, next_chunk, probes},
    metrics::WatchdogMetrics,
    store::{KindSummary, NodeProgress, NodeStore, Store, SweepProgress, SweepState},
};

/// Pacing shared by all node watches.
#[derive(Clone, Debug, Args)]
pub struct WatchOptions {
    /// Time from one sweep start to the next.
    #[arg(
        long,
        env = "ESPRESSO_QUERY_WATCHDOG_SWEEP_INTERVAL",
        value_parser = humantime::parse_duration,
        default_value = "24h"
    )]
    pub sweep_interval: Duration,
    /// Minimum gap between two requests to one node.
    #[arg(
        long,
        env = "ESPRESSO_QUERY_WATCHDOG_REQUEST_INTERVAL",
        value_parser = humantime::parse_duration,
        default_value = "200ms"
    )]
    pub request_interval: Duration,
}

/// A query node's base URL and label.
#[derive(Clone, Debug)]
pub struct WatchedNode {
    pub label: String,
    pub base_url: Url,
}

impl WatchedNode {
    /// Labelled `url.authority()`.
    pub fn new(base_url: Url) -> Self {
        Self {
            label: base_url.authority().to_owned(),
            base_url,
        }
    }

    /// `route` below the base URL.
    fn route_url(&self, route: &str) -> Url {
        self.base_url
            .join(route)
            .expect("a base URL can be extended by a relative route")
    }
}

/// Why a request returned no 200.
#[derive(Debug)]
pub enum FetchError {
    Status(StatusCode),
    Transport(reqwest::Error),
}

impl FetchError {
    /// The `status` label: `StatusCode::as_str()` (`"404"`, `"500"`, ...) or `"transport"`.
    pub fn status_label(&self) -> &str {
        match self {
            Self::Status(status) => status.as_str(),
            Self::Transport(_) => "transport",
        }
    }

    /// `Ok` for a 2xx status.
    fn check(status: StatusCode) -> Result<(), Self> {
        status
            .is_success()
            .then_some(())
            .ok_or(Self::Status(status))
    }
}

impl fmt::Display for FetchError {
    /// Status code, or the transport error with its cause.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Status(status) => write!(f, "status {status}"),
            Self::Transport(error) => write!(f, "transport error: {error:#}"),
        }
    }
}

/// What a failed request asked for.
#[derive(Clone, Copy, Debug)]
pub enum FetchTarget {
    Data(DataKind),
    BlockHeight,
    MerklizedStateHeight,
    Config,
}

impl FetchTarget {
    /// The `kind` label of `fetch_errors_total`: the `DataKind` label, `"block_height"`,
    /// `"merklized_state_height"` or `"config"`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Data(kind) => kind.into(),
            Self::BlockHeight => "block_height",
            Self::MerklizedStateHeight => "merklized_state_height",
            Self::Config => "config",
        }
    }
}

/// Parts shared by all node watches; every field is a cheap clone.
#[derive(Clone)]
pub struct Watchdog {
    http: reqwest::Client,
    options: WatchOptions,
    metrics: Arc<WatchdogMetrics>,
    store: Store,
}

/// Total time of one request, including server-side range waits (`state.rs:85`).
const HTTP_TIMEOUT: Duration = Duration::from_secs(60);

/// The part of `v1/config/hotshot` the watchdog reads.
#[derive(Deserialize)]
struct NetworkConfig {
    config: HotShotConfig,
}

/// Epoch fields of `NetworkConfig::config`.
#[derive(Deserialize)]
struct HotShotConfig {
    epoch_height: u64,
    epoch_start_block: u64,
}

impl Watchdog {
    /// Builds the HTTP client with `HTTP_TIMEOUT`.
    pub fn new(
        options: WatchOptions,
        metrics: Arc<WatchdogMetrics>,
        store: Store,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder().timeout(HTTP_TIMEOUT).build()?,
            options,
            metrics,
            store,
        })
    }

    /// Fetches the tip, loads or creates the node's progress, logs `watching` or `resumed`.
    pub async fn start_node(
        &self,
        node: WatchedNode,
        tip_sources: Vec<WatchedNode>,
        epochs: Option<EpochConfig>,
    ) -> anyhow::Result<NodeWatch> {
        let tip_height = self
            .fetch_tip(&tip_sources)
            .await
            .ok_or_else(|| anyhow!("no tip source answered for {}", node.label))?;
        let store = self.store.node(&node.label);
        let loaded = store.load_progress().await?;
        let progress = match &loaded {
            Some(progress) => progress.clone(),
            None => {
                let progress = NodeProgress {
                    follow_next_height: tip_height,
                    sweep: SweepState::Waiting {
                        start_at: Utc::now(),
                    },
                    last_sweep_completed_at: None,
                };
                store.save_progress(&progress).await?;
                progress
            },
        };
        if let Some(completed_at) = progress.last_sweep_completed_at {
            self.metrics.mark_sweep_completed(&node.label, completed_at);
        }
        let now = Instant::now();
        let watch = NodeWatch {
            watchdog: self.clone(),
            tip_sources,
            store,
            progress,
            epochs,
            tip_height,
            merklized_state_height: None,
            next_tip_poll_at: now,
            last_summary_at: now,
            errors_since_summary: BTreeMap::new(),
            node,
        };
        watch.log_started(loaded.is_some()).await?;
        Ok(watch)
    }

    /// `v1/config/hotshot` from the first node that answers; `None` when `epoch_height` is 0;
    /// errors when no node answers (Q3).
    pub async fn fetch_epoch_config(
        &self,
        nodes: &[WatchedNode],
    ) -> anyhow::Result<Option<EpochConfig>> {
        let mut failures = Vec::new();
        for node in nodes {
            match self
                .get_json::<NetworkConfig>(node, "v1/config/hotshot")
                .await
            {
                Ok(NetworkConfig { config }) => {
                    return Ok((config.epoch_height != 0).then_some(EpochConfig {
                        epoch_height: config.epoch_height,
                        epoch_start_block: config.epoch_start_block,
                    }));
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

    /// Max `v1/node/block-height` across `tip_sources`; counts each failure under the source's
    /// label; `None` when none answers.
    async fn fetch_tip(&self, tip_sources: &[WatchedNode]) -> Option<u64> {
        let heights = join_all(
            tip_sources
                .iter()
                .map(|source| self.get_json::<u64>(source, "v1/node/block-height")),
        )
        .await;
        let mut tip = None;
        for (source, height) in tip_sources.iter().zip(heights) {
            match height {
                Ok(height) => tip = tip.max(Some(height)),
                Err(error) => {
                    tracing::debug!(node = %source.label, ?error, "tip request failed");
                    self.metrics
                        .count_fetch_error(&source.label, FetchTarget::BlockHeight, &error);
                },
            }
        }
        tip
    }

    /// GET `route` and decodes the JSON body of a 2xx response.
    async fn get_json<T: DeserializeOwned>(
        &self,
        node: &WatchedNode,
        route: &str,
    ) -> Result<T, FetchError> {
        let response = self
            .http
            .get(node.route_url(route))
            .send()
            .await
            .map_err(FetchError::Transport)?;
        FetchError::check(response.status())?;
        response.json().await.map_err(FetchError::Transport)
    }

    /// HEAD `route`; the status is the answer.
    async fn head(&self, node: &WatchedNode, route: &str) -> Result<(), FetchError> {
        let response = self
            .http
            .head(node.route_url(route))
            .send()
            .await
            .map_err(FetchError::Transport)?;
        FetchError::check(response.status())
    }
}

/// Checker of one watched node: follow, retries and sweep, one request at a time.
pub struct NodeWatch {
    watchdog: Watchdog,
    node: WatchedNode,
    tip_sources: Vec<WatchedNode>,
    store: NodeStore,
    progress: NodeProgress,
    epochs: Option<EpochConfig>,
    tip_height: u64,
    merklized_state_height: Option<u64>,
    next_tip_poll_at: Instant,
    last_summary_at: Instant,
    errors_since_summary: BTreeMap<String, u64>,
}

/// Missing chunks re-checked per iteration; small so sweep and follow keep their share.
const RETRIES_PER_STEP: u32 = 4;
/// Time between tip polls.
const TIP_POLL_INTERVAL: Duration = Duration::from_secs(10);
/// Time between summary lines of one node.
const SUMMARY_INTERVAL: Duration = Duration::from_secs(60);

impl NodeWatch {
    /// Runs the loop until a store error.
    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            self.poll_tip(Instant::now()).await;
            let mut sent = self.follow_step().await?;
            sent += self.retry_step().await?;
            sent += self.sweep_step().await?;
            self.summarize_if_due(Instant::now()).await?;
            if sent == 0 {
                tokio::time::sleep_until(self.next_tip_poll_at).await;
            }
        }
    }

    /// Logs the `watching` or `resumed` line of `start_node`.
    async fn log_started(&self, resumed: bool) -> anyhow::Result<()> {
        let node = &self.node.label;
        if resumed {
            let summary = self.summary().await?;
            tracing::info!(
                %node,
                follow_next_height = self.progress.follow_next_height,
                sweep = ?self.progress.sweep,
                %summary,
                "resumed"
            );
        } else {
            let tip_sources: Vec<_> = self.tip_sources.iter().map(|s| &s.label).collect();
            tracing::info!(%node, tip_height = self.tip_height, ?tip_sources, "watching");
        }
        Ok(())
    }

    /// Refreshes `tip_height` (`Watchdog::fetch_tip`) and the node's `v1/block-state/block-height`
    /// when due.
    async fn poll_tip(&mut self, now: Instant) {
        if now < self.next_tip_poll_at {
            return;
        }
        self.next_tip_poll_at = now + TIP_POLL_INTERVAL;
        if let Some(tip) = self.watchdog.fetch_tip(&self.tip_sources).await {
            self.tip_height = tip;
        }
        match self
            .watchdog
            .get_json::<u64>(&self.node, "v1/block-state/block-height")
            .await
        {
            Ok(height) => self.merklized_state_height = Some(height),
            Err(error) => self.record_error(FetchTarget::MerklizedStateHeight, &error),
        }
    }

    /// Checks the next follow chunk for every kind and saves progress; returns requests sent.
    async fn follow_step(&mut self) -> anyhow::Result<usize> {
        let Some(chunk) = next_chunk(self.progress.follow_next_height, self.tip_height) else {
            return Ok(0);
        };
        let sent = self.check_chunk(&chunk).await?;
        self.progress.follow_next_height = chunk.end;
        self.store.save_progress(&self.progress).await?;
        Ok(sent)
    }

    /// Re-checks due missing chunks; returns requests sent.
    async fn retry_step(&mut self) -> anyhow::Result<usize> {
        let due = self.store.due_retries(Utc::now(), RETRIES_PER_STEP).await?;
        for probe in &due {
            self.check_probe(probe).await?;
        }
        Ok(due.len())
    }

    /// Starts a due sweep or checks its next chunk for every kind; logs start and end; saves progress.
    async fn sweep_step(&mut self) -> anyhow::Result<usize> {
        if let SweepState::Waiting { start_at } = self.progress.sweep {
            if Utc::now() < start_at {
                return Ok(0);
            }
            self.start_sweep().await?;
        }
        let SweepState::Running(sweep) = self.progress.sweep.clone() else {
            return Ok(0);
        };
        let Some(chunk) = next_chunk(sweep.next_height, sweep.until_height) else {
            self.finish_sweep(&sweep).await?;
            return Ok(0);
        };
        let sent = self.check_chunk(&chunk).await?;
        self.progress.sweep = SweepState::Running(SweepProgress {
            next_height: chunk.end,
            ..sweep
        });
        self.store.save_progress(&self.progress).await?;
        Ok(sent)
    }

    /// Begins a sweep over `[0, tip_height)`.
    async fn start_sweep(&mut self) -> anyhow::Result<()> {
        self.progress.sweep = SweepState::Running(SweepProgress {
            until_height: self.tip_height,
            next_height: 0,
            started_at: Utc::now(),
        });
        self.store.save_progress(&self.progress).await?;
        tracing::info!(node = %self.node.label, until_height = self.tip_height, "sweep started");
        Ok(())
    }

    /// Schedules the next sweep, then logs the end line and sets the gauge.
    async fn finish_sweep(&mut self, sweep: &SweepProgress) -> anyhow::Result<()> {
        let completed_at = Utc::now();
        let interval = TimeDelta::from_std(self.watchdog.options.sweep_interval)?;
        self.progress.last_sweep_completed_at = Some(completed_at);
        self.progress.sweep = SweepState::Waiting {
            start_at: sweep.started_at + interval,
        };
        self.store.save_progress(&self.progress).await?;
        self.watchdog
            .metrics
            .mark_sweep_completed(&self.node.label, completed_at);
        let duration = Duration::from_secs(
            (completed_at - sweep.started_at)
                .num_seconds()
                .max(0)
                .unsigned_abs(),
        );
        let summary = self.summary().await?;
        tracing::info!(
            node = %self.node.label,
            duration = %humantime::format_duration(duration),
            %summary,
            "sweep completed"
        );
        Ok(())
    }

    /// Runs `check_probe` for every entry of `probes(chunk, self.epochs)`; returns requests sent.
    async fn check_chunk(&mut self, chunk: &Range<u64>) -> anyhow::Result<usize> {
        let to_check = probes(chunk, self.epochs);
        for probe in &to_check {
            self.check_probe(probe).await?;
        }
        Ok(to_check.len())
    }

    /// Waits `request_interval`, sends HEAD to `probe.route()`, counts and DEBUG-logs a failure,
    /// then `record_found` on a 200 or `record_missing` otherwise.
    async fn check_probe(&mut self, probe: &Probe) -> anyhow::Result<()> {
        tokio::time::sleep(self.watchdog.options.request_interval).await;
        match self.watchdog.head(&self.node, &probe.route()).await {
            Ok(()) => self.store.record_found(probe).await,
            Err(error) => {
                self.record_error(FetchTarget::Data(probe.kind), &error);
                self.store.record_missing(probe, Utc::now()).await
            },
        }
    }

    /// Counts a failed request of the watched node in the metrics and the summary window.
    fn record_error(&mut self, target: FetchTarget, error: &FetchError) {
        tracing::debug!(node = %self.node.label, target = target.label(), ?error, "request failed");
        self.watchdog
            .metrics
            .count_fetch_error(&self.node.label, target, error);
        *self
            .errors_since_summary
            .entry(error.status_label().to_owned())
            .or_default() += 1;
    }

    /// Snapshot from `tip_height`, `progress`, `merklized_state_height` and `missing_summary`.
    async fn summary(&self) -> anyhow::Result<NodeSummary> {
        Ok(NodeSummary {
            tip_height: self.tip_height,
            follow_lag_blocks: self
                .tip_height
                .saturating_sub(self.progress.follow_next_height),
            merklized_state_lag_blocks: self
                .merklized_state_height
                .map(|height| self.tip_height.saturating_sub(height)),
            kinds: self.store.missing_summary(Utc::now()).await?,
        })
    }

    /// After `SUMMARY_INTERVAL`: logs one INFO line, exports gauges, resets `errors_since_summary`.
    async fn summarize_if_due(&mut self, now: Instant) -> anyhow::Result<()> {
        if now.duration_since(self.last_summary_at) < SUMMARY_INTERVAL {
            return Ok(());
        }
        self.last_summary_at = now;
        let summary = self.summary().await?;
        tracing::info!(
            node = %self.node.label,
            errors = ?self.errors_since_summary,
            %summary,
            "summary"
        );
        self.watchdog
            .metrics
            .export_summary(&self.node.label, &summary);
        self.errors_since_summary.clear();
        Ok(())
    }
}

/// One node's state, logged each minute and exported as gauges.
pub struct NodeSummary {
    pub tip_height: u64,
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
            self.tip_height, self.follow_lag_blocks
        )?;
        match self.merklized_state_lag_blocks {
            Some(lag) => write!(f, "{lag}")?,
            None => f.write_str("-")?,
        }
        for (i, kind) in self.kinds.iter().enumerate() {
            let unit = if kind.kind.is_per_epoch() {
                "missing_epochs"
            } else {
                "missing"
            };
            let label: &str = kind.kind.into();
            f.write_str(if i == 0 { " " } else { ", " })?;
            write!(f, "{label} {unit}={} oldest_age=", kind.missing_count)?;
            match kind.oldest_missing_age {
                Some(age) => write!(
                    f,
                    "{}",
                    humantime::format_duration(Duration::from_secs(age.as_secs()))
                )?,
                None => f.write_str("-")?,
            }
        }
        Ok(())
    }
}
