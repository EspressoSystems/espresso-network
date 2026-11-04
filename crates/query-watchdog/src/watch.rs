use std::{
    collections::BTreeMap, convert::Infallible, error::Error, fmt, ops::Range, sync::Arc,
    time::Duration,
};

use anyhow::{Context, anyhow, bail, ensure};
use chrono::{DateTime, SubsecRound, TimeDelta, Utc};
use clap::Args;
use reqwest::StatusCode;
use serde::{Deserialize, de::DeserializeOwned};
use tokio::{sync::watch, time::Instant};
use url::Url;

use crate::{
    chunk::{CHUNK_LEN, DataKind, EpochConfig, Probe, next_chunk, probes},
    metrics::{ProbeResult, WatchdogMetrics},
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
    /// Missing data older than this is logged as a WARN once a minute per node and kind.
    #[arg(
        long,
        env = "ESPRESSO_QUERY_WATCHDOG_WARN_MISSING_AFTER",
        value_parser = humantime::parse_duration,
        default_value = "10m"
    )]
    pub warn_missing_after: Duration,
}

impl WatchOptions {
    /// `Archive` when `retention_blocks` is absent.
    pub fn sweep_window(&self) -> SweepWindow {
        self.retention_blocks
            .map_or(SweepWindow::Archive, |blocks| SweepWindow::Retention {
                blocks,
            })
    }
}

/// Heights a sweep covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SweepWindow {
    /// `[0, tip_height)`.
    Archive,
    /// `[tip_height - blocks, tip_height)`, saturating; the start rounds up to a multiple of
    /// `CHUNK_LEN` so chunk boundaries repeat across sweeps.
    Retention { blocks: u64 },
}

impl SweepWindow {
    /// First height of the window for `tip_height`.
    pub fn start_height(self, tip_height: u64) -> u64 {
        match self {
            Self::Archive => 0,
            Self::Retention { blocks } => tip_height
                .saturating_sub(blocks)
                .next_multiple_of(CHUNK_LEN),
        }
    }
}

/// A query node's base URL and label.
#[derive(Clone, Debug)]
pub struct WatchedNode {
    pub label: String,
    pub base_url: Url,
}

impl WatchedNode {
    /// Labelled `host:port` (never the authority, which can carry credentials); errors on a base
    /// path without a trailing slash, which `Url::join` would truncate.
    pub fn new(base_url: Url) -> anyhow::Result<Self> {
        let host = base_url
            .host_str()
            .ok_or_else(|| anyhow!("{} node URL has no host", base_url.scheme()))?;
        let port = base_url
            .port_or_known_default()
            .ok_or_else(|| anyhow!("node URL of {host} has no port"))?;
        ensure!(
            base_url.path().ends_with('/'),
            "node URL path must end with '/': {}",
            base_url.path()
        );
        Ok(Self {
            label: format!("{host}:{port}"),
            base_url,
        })
    }

    /// The GET URL of `probe`'s route.
    pub fn probe_url(&self, probe: &Probe) -> Url {
        self.route_url(&probe.route())
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
    fn from_status(status: StatusCode) -> Result<(), Self> {
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
            Self::Transport(error) => {
                write!(f, "transport error: {error}")?;
                let mut cause = error.source();
                while let Some(source) = cause {
                    write!(f, ": {source}")?;
                    cause = source.source();
                }
                Ok(())
            },
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
    config: EpochConfig,
}

/// Total time of a GET that is decoded (tip, state height, config).
const JSON_TIMEOUT: Duration = Duration::from_secs(5);

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

    /// Loads or creates the node's progress, clamped to the sweep window start on resume, and logs
    /// `watching` or `resumed`.
    pub async fn start_node(
        &self,
        node: WatchedNode,
        tip: watch::Receiver<u64>,
        epochs: Option<EpochConfig>,
    ) -> anyhow::Result<NodeWatch> {
        let tip_height = *tip.borrow();
        let store = self.store.node(&node.label);
        let loaded = store.load_progress().await?;
        let window_start = self.options.sweep_window().start_height(tip_height);
        let progress = match &loaded {
            Some(progress) => progress.clone().clamped_to(window_start),
            None => {
                let progress = NodeProgress {
                    follow_next_height: tip_height,
                    sweep: SweepState::Waiting {
                        start_at: utc_now(),
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
            tip,
            store,
            progress,
            epochs,
            tip_height,
            merklized_state_height: None,
            next_state_poll_at: now,
            last_summary_at: now,
            errors_since_summary: BTreeMap::new(),
            node,
        };
        let summary = watch.summary().await?;
        watch.log_started(loaded.is_some(), &summary);
        self.metrics.export_summary(&watch.node.label, &summary)?;
        watch.export_progress()?;
        Ok(watch)
    }

    /// Fetches the first tip, which errors when no node answers (Q5), and returns the poller with
    /// the receiver every `NodeWatch` reads.
    pub async fn start_tip_poller(
        &self,
        nodes: Vec<WatchedNode>,
    ) -> anyhow::Result<(TipPoller, watch::Receiver<u64>)> {
        let tip = self.fetch_tip(&nodes).await.ok_or_else(|| {
            let labels: Vec<_> = nodes.iter().map(|node| &node.label).collect();
            anyhow!("no watched node answered v1/node/block-height: {labels:?}")
        })?;
        let (sender, receiver) = watch::channel(tip);
        let poller = TipPoller {
            watchdog: self.clone(),
            nodes,
            tip: sender,
        };
        Ok((poller, receiver))
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

    /// Max `v1/node/block-height` across `nodes`, queried in turn; counts each failure under the
    /// node's label; `None` when none answers.
    async fn fetch_tip(&self, nodes: &[WatchedNode]) -> Option<u64> {
        let mut tip = None;
        for node in nodes {
            match self.get_json::<u64>(node, "v1/node/block-height").await {
                Ok(height) => tip = tip.max(Some(height)),
                Err(error) => {
                    tracing::debug!(node = %node.label, %error, "tip request failed");
                    self.metrics
                        .count_fetch_error(&node.label, FetchTarget::BlockHeight, &error);
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
            .timeout(JSON_TIMEOUT)
            .send()
            .await
            .map_err(FetchError::Transport)?;
        FetchError::from_status(response.status())?;
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
        FetchError::from_status(response.status())
    }
}

/// Publishes the max block height of all watched nodes, polled every `TIP_POLL_INTERVAL`.
pub struct TipPoller {
    watchdog: Watchdog,
    nodes: Vec<WatchedNode>,
    tip: watch::Sender<u64>,
}

impl TipPoller {
    /// Polls forever, never returning; a round where no node answers keeps the previous tip.
    pub async fn run(self) -> Infallible {
        loop {
            tokio::time::sleep(TIP_POLL_INTERVAL).await;
            if let Some(tip) = self.watchdog.fetch_tip(&self.nodes).await {
                self.tip.send_replace(tip);
            }
        }
    }
}

/// Checker of one watched node: follow, retries and sweep, one request at a time.
pub struct NodeWatch {
    watchdog: Watchdog,
    node: WatchedNode,
    tip: watch::Receiver<u64>,
    store: NodeStore,
    progress: NodeProgress,
    epochs: Option<EpochConfig>,
    tip_height: u64,
    merklized_state_height: Option<u64>,
    next_state_poll_at: Instant,
    last_summary_at: Instant,
    errors_since_summary: BTreeMap<String, u64>,
}

/// Missing chunks re-checked per iteration; small so sweep and follow keep their share.
const RETRIES_PER_STEP: u32 = 4;
/// Time between tip polls and between polls of a node's merklized state height.
const TIP_POLL_INTERVAL: Duration = Duration::from_secs(10);
/// Time between summary lines of one node.
const SUMMARY_INTERVAL: Duration = Duration::from_secs(60);

impl NodeWatch {
    /// Runs the loop until a store error.
    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            self.tip_height = *self.tip.borrow_and_update();
            self.poll_state_height(Instant::now()).await;
            let mut sent = self.follow_step().await?;
            sent += self.retry_step().await?;
            sent += self.sweep_step().await?;
            self.summarize_if_due(Instant::now()).await?;
            self.export_progress()?;
            if sent == 0 {
                tokio::select! {
                    () = tokio::time::sleep_until(self.next_state_poll_at) => {},
                    changed = self.tip.changed() => changed?,
                }
            }
        }
    }

    /// Logs the `watching` or `resumed` line of `start_node`.
    fn log_started(&self, resumed: bool, summary: &NodeSummary) {
        let node = &self.node.label;
        if !resumed {
            tracing::info!(%node, tip_height = self.tip_height, %summary, "watching");
            return;
        }
        let (sweep, next_height, until_height, start_at) = match &self.progress.sweep {
            SweepState::Running(sweep) => (
                "running",
                Some(sweep.next_height),
                Some(sweep.until_height),
                None,
            ),
            SweepState::Waiting { start_at } => {
                ("waiting", None, None, Some(start_at.to_rfc3339()))
            },
        };
        tracing::info!(
            %node,
            follow_next_height = self.progress.follow_next_height,
            sweep,
            next_height,
            until_height,
            start_at,
            %summary,
            "resumed"
        );
    }

    /// Refreshes the node's `v1/block-state/block-height` when due.
    async fn poll_state_height(&mut self, now: Instant) {
        if now < self.next_state_poll_at {
            return;
        }
        self.next_state_poll_at = now + TIP_POLL_INTERVAL;
        tokio::time::sleep(self.watchdog.options.request_interval).await;
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
        let due = self.store.due_retries(utc_now(), RETRIES_PER_STEP).await?;
        for probe in &due {
            self.check_probe(probe).await?;
        }
        Ok(due.len())
    }

    /// Starts a due sweep or checks its next chunk for every kind; logs start and end; saves progress.
    async fn sweep_step(&mut self) -> anyhow::Result<usize> {
        if let SweepState::Waiting { start_at } = self.progress.sweep {
            if utc_now() < start_at {
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

    /// Begins a sweep over the window below the tip.
    async fn start_sweep(&mut self) -> anyhow::Result<()> {
        let start_height = self
            .watchdog
            .options
            .sweep_window()
            .start_height(self.tip_height);
        self.progress.sweep = SweepState::Running(SweepProgress {
            until_height: self.tip_height,
            next_height: start_height,
            started_at: utc_now(),
        });
        self.store.save_progress(&self.progress).await?;
        tracing::info!(
            node = %self.node.label,
            start_height,
            until_height = self.tip_height,
            "sweep started"
        );
        Ok(())
    }

    /// Schedules the next sweep, then logs the end line and sets the gauge.
    async fn finish_sweep(&mut self, sweep: &SweepProgress) -> anyhow::Result<()> {
        let completed_at = utc_now();
        let interval = TimeDelta::from_std(self.watchdog.options.sweep_interval)?;
        let duration = (completed_at - sweep.started_at)
            .to_std()
            .with_context(|| {
                format!(
                    "{}: sweep completed at {completed_at} before it started at {}",
                    self.node.label, sweep.started_at
                )
            })?;
        self.progress.last_sweep_completed_at = Some(completed_at);
        self.progress.sweep = SweepState::Waiting {
            start_at: sweep.started_at + interval,
        };
        self.store.save_progress(&self.progress).await?;
        self.watchdog
            .metrics
            .mark_sweep_completed(&self.node.label, completed_at);
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
            Ok(()) => {
                self.count_probe(probe, ProbeResult::Found);
                self.store.record_found(probe).await
            },
            Err(error) => {
                self.count_probe(probe, ProbeResult::Missing);
                self.record_error(FetchTarget::Data(probe.kind), &error);
                self.store.record_missing(probe, utc_now()).await
            },
        }
    }

    /// Counts one probe of this node.
    fn count_probe(&self, probe: &Probe, result: ProbeResult) {
        self.watchdog
            .metrics
            .count_probe(&self.node.label, probe.kind, result);
    }

    /// Sets the tip, follow and sweep progress gauges.
    fn export_progress(&self) -> anyhow::Result<()> {
        self.watchdog
            .metrics
            .export_progress(&self.node.label, self.tip_height, &self.progress)
    }

    /// Counts a failed request of the watched node in the metrics and the summary window.
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
            kinds: self.store.missing_summary(utc_now()).await?,
        })
    }

    /// After `SUMMARY_INTERVAL`: trims to the sweep window (retention only), logs one INFO line, one WARN
    /// per kind missing longer than `warn_missing_after`, exports gauges, resets
    /// `errors_since_summary`.
    async fn summarize_if_due(&mut self, now: Instant) -> anyhow::Result<()> {
        if now.duration_since(self.last_summary_at) < SUMMARY_INTERVAL {
            return Ok(());
        }
        self.last_summary_at = now;
        let forgotten = self.trim_to_window().await?;
        let summary = self.summary().await?;
        tracing::info!(
            node = %self.node.label,
            errors = ?self.errors_since_summary,
            forgotten,
            %summary,
            "summary"
        );
        self.warn_missing(&summary);
        self.watchdog
            .metrics
            .export_summary(&self.node.label, &summary)?;
        self.errors_since_summary.clear();
        Ok(())
    }

    /// With a retention window, clamps progress to the window start and forgets missing rows below
    /// it; returns the forgotten count. `None` in archive mode.
    async fn trim_to_window(&mut self) -> anyhow::Result<Option<u64>> {
        let window = self.watchdog.options.sweep_window();
        if window == SweepWindow::Archive {
            return Ok(None);
        }
        let window_start = window.start_height(self.tip_height);
        let clamped = self.progress.clone().clamped_to(window_start);
        if clamped != self.progress {
            self.progress = clamped;
            self.store.save_progress(&self.progress).await?;
        }
        let forgotten = self.store.forget_below(window_start, self.epochs).await?;
        Ok(Some(forgotten))
    }

    /// One WARN per kind whose oldest missing range is at least `warn_missing_after` old; `url` is
    /// the GET of that range.
    fn warn_missing(&self, summary: &NodeSummary) {
        for kind in &summary.kinds {
            let Some(oldest) = &kind.oldest else {
                continue;
            };
            if oldest.age < self.watchdog.options.warn_missing_after {
                continue;
            }
            let probe = Probe {
                kind: kind.kind,
                range: oldest.range.clone(),
            };
            // Node URLs are internal and carry no credentials.
            tracing::warn!(
                node = %self.node.label,
                kind = <&str>::from(kind.kind),
                missing = kind.missing_count,
                oldest_age_seconds = oldest.age.as_secs(),
                range_start = oldest.range.start,
                range_end = oldest.range.end,
                url = %self.node.probe_url(&probe),
                "missing data"
            );
        }
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

/// Current time at whole-second precision, the precision of persisted times.
fn utc_now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_window_start_height() {
        assert_eq!(SweepWindow::Archive.start_height(5000), 0);
        assert_eq!(
            SweepWindow::Retention { blocks: 2000 }.start_height(5000),
            3000
        );
        assert_eq!(
            SweepWindow::Retention { blocks: 2000 }.start_height(1500),
            0
        );
        assert_eq!(
            SweepWindow::Retention { blocks: 2000 }.start_height(5030),
            3100
        );
    }
}
