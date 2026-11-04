use std::{
    collections::BTreeMap,
    convert::Infallible,
    error::Error,
    fmt,
    ops::Range,
    sync::{Arc, Mutex},
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
    gaps::{Known, Presence, Unrefined, find_gaps, parse_missing_height},
    metrics::WatchdogMetrics,
    store::{
        ALERT_MIN_FAILED_PROBES, AlertCandidate, KindSummary, NodeProgress, NodeStore, Store,
        SweepProgress, SweepState,
    },
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
    /// Minimum age of missing data for the alert (ERROR on target `query_watchdog::alert`, once a
    /// minute per node and kind). 60s leaves room for retries at about 5, 15 and 35 s first.
    #[arg(
        long,
        env = "ESPRESSO_QUERY_WATCHDOG_ALERT_MISSING_AFTER",
        value_parser = humantime::parse_duration,
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
    /// Labelled `host:port`; node URLs are internal and carry no credentials. Errors on a base
    /// path without a trailing slash, which `Url::join` would truncate.
    pub fn new(base_url: Url) -> anyhow::Result<Self> {
        let host = base_url
            .host_str()
            .ok_or_else(|| anyhow!("{} node URL has no host", base_url.scheme()))?;
        let port = base_url
            .port_or_known_default()
            .ok_or_else(|| anyhow!("node URL of {host} has no port"))?;
        ensure!(
            base_url.username().is_empty() && base_url.password().is_none(),
            "node URL of {host} must not carry credentials"
        );
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

    /// `route` below the base URL.
    pub(crate) fn route_url(&self, route: &str) -> Url {
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

/// Estimated finish time of the running sweep per node label, read by the page; a node with no ETA
/// has no entry.
pub type SweepEtas = Arc<Mutex<BTreeMap<String, DateTime<Utc>>>>;

/// Parts shared by all node watches; every field is a cheap clone.
#[derive(Clone)]
pub struct Watchdog {
    http: reqwest::Client,
    options: WatchOptions,
    metrics: Arc<WatchdogMetrics>,
    store: Store,
    sweep_etas: SweepEtas,
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
        sweep_etas: SweepEtas,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            http: reqwest::Client::builder().timeout(HTTP_TIMEOUT).build()?,
            options,
            metrics,
            store,
            sweep_etas,
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
        let mut watch = NodeWatch {
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
            eta_origin: None,
            node,
        };
        watch.reset_eta_origin();
        let summary = watch.summary().await?;
        watch.log_started(loaded.is_some(), &summary);
        self.metrics.export_summary(&watch.node.label, &summary)?;
        watch.publish_progress()?;
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
        self.metrics.set_tip_height(tip)?;
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

    /// GET `route` after its HEAD returned 404: the height named by the 404 message, `NoHeight` when
    /// the 404 body does not name one, `Present` when the GET succeeds, an error for any other
    /// status.
    async fn lookup_missing_height(
        &self,
        node: &WatchedNode,
        route: &str,
    ) -> Result<HeightLookup, FetchError> {
        let response = self
            .http
            .get(node.route_url(route))
            .send()
            .await
            .map_err(FetchError::Transport)?;
        let status = response.status();
        if status.is_success() {
            return Ok(HeightLookup::Present);
        }
        if status != StatusCode::NOT_FOUND {
            return Err(FetchError::Status(status));
        }
        let body = response.bytes().await.map_err(FetchError::Transport)?;
        let height = serde_json::from_slice::<ErrorBody>(&body)
            .ok()
            .and_then(|body| parse_missing_height(&body.custom.message));
        Ok(height.map_or(HeightLookup::NoHeight, HeightLookup::Named))
    }
}

/// What the GET of a range that failed with 404 says.
enum HeightLookup {
    /// The first missing height.
    Named(u64),
    /// A 404 whose body does not name a height.
    NoHeight,
    /// The range is served after all.
    Present,
}

/// The 404 body of the availability routes.
#[derive(Deserialize)]
struct ErrorBody {
    #[serde(rename = "Custom")]
    custom: ErrorMessage,
}

/// The message of an `ErrorBody`.
#[derive(Deserialize)]
struct ErrorMessage {
    message: String,
}

/// Publishes the max block height of all watched nodes, polled every `TIP_POLL_INTERVAL`.
pub struct TipPoller {
    watchdog: Watchdog,
    nodes: Vec<WatchedNode>,
    tip: watch::Sender<u64>,
}

impl TipPoller {
    /// Polls forever, returning only an error; a round where no node answers keeps the previous
    /// tip.
    pub async fn run(self) -> anyhow::Result<Infallible> {
        loop {
            tokio::time::sleep(TIP_POLL_INTERVAL).await;
            if let Some(tip) = self.watchdog.fetch_tip(&self.nodes).await {
                self.watchdog.metrics.set_tip_height(tip)?;
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
    eta_origin: Option<EtaOrigin>,
}

/// Where the ETA rate starts: when this process began or resumed the running sweep.
#[derive(Clone, Copy, Debug)]
struct EtaOrigin {
    at: Instant,
    next_height: u64,
}

/// Due missing ranges fetched per batch of retries.
const RETRIES_PER_BATCH: u32 = 20;
/// Time one iteration spends on retries; sweep and follow keep the rest.
const RETRY_BUDGET: Duration = Duration::from_secs(5);
/// Time between tip polls and between polls of a node's merklized state height.
const TIP_POLL_INTERVAL: Duration = Duration::from_secs(10);
/// Blocks below the tip within which missing data is not alerted and follow does not probe yet:
/// nodes lag the tip a little, and proofs need the blocks after them.
const ALERT_TIP_MARGIN: u64 = 10;
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
            self.publish_progress()?;
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

    /// Checks the next follow chunk, which ends at least `ALERT_TIP_MARGIN` below the tip, for every
    /// kind and saves progress; returns requests sent.
    async fn follow_step(&mut self) -> anyhow::Result<usize> {
        let follow_until = self.tip_height.saturating_sub(ALERT_TIP_MARGIN);
        let Some(chunk) = next_chunk(self.progress.follow_next_height, follow_until) else {
            return Ok(0);
        };
        let sent = self.check_chunk(&chunk).await?;
        self.progress.follow_next_height = chunk.end;
        self.store.save_progress(&self.progress).await?;
        Ok(sent)
    }

    /// Re-checks due missing ranges in batches until none is due or `RETRY_BUDGET` is spent;
    /// returns the number of ranges re-checked.
    async fn retry_step(&mut self) -> anyhow::Result<usize> {
        let deadline = Instant::now() + RETRY_BUDGET;
        let mut checked = 0;
        while Instant::now() < deadline {
            let due = self.store.due_retries(utc_now(), RETRIES_PER_BATCH).await?;
            if due.is_empty() {
                break;
            }
            for probe in &due {
                self.check_probe(probe).await?;
            }
            checked += due.len();
        }
        Ok(checked)
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
        self.reset_eta_origin();
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
        self.reset_eta_origin();
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

    /// Sends the probe, then `record_found` on a 200 or `record_failure` otherwise.
    async fn check_probe(&mut self, probe: &Probe) -> anyhow::Result<()> {
        match self.send_probe(probe).await {
            Ok(()) => self.store.record_found(probe, utc_now()).await,
            Err(error) => {
                self.record_error(FetchTarget::Data(probe.kind), &error);
                self.record_failure(probe, &error).await
            },
        }
    }

    /// Waits `request_interval`, counts the probe and sends HEAD to `probe.route()`.
    async fn send_probe(&self, probe: &Probe) -> Result<(), FetchError> {
        tokio::time::sleep(self.watchdog.options.request_interval).await;
        self.watchdog
            .metrics
            .count_probe(&self.node.label, probe.kind);
        self.watchdog.head(&self.node, &probe.route()).await
    }

    /// Stores a failed probe: a 404 of a range kind is refined to its exact gaps (the tail that
    /// cannot be refined stays chunk level); another failure of a range kind backs off the row for
    /// exactly that range, otherwise adds rows only for heights no row covers; anything else is
    /// missing as a whole.
    async fn record_failure(&mut self, probe: &Probe, error: &FetchError) -> anyhow::Result<()> {
        let is_not_found = matches!(error, FetchError::Status(StatusCode::NOT_FOUND));
        if !probe.kind.is_range() {
            return self
                .store
                .record_missing(probe, error.status_label(), utc_now())
                .await;
        }
        if !is_not_found {
            return self
                .store
                .record_range_failure(probe, error.status_label(), utc_now())
                .await;
        }
        let known = if self.store.is_exact_gap(probe).await? {
            Known::Gap
        } else {
            Known::Missing
        };
        let mut prober = RangeProber {
            watch: self,
            kind: probe.kind,
        };
        let search = find_gaps(&mut prober, probe.range.clone(), known).await;
        self.store
            .replace_missing(
                probe.kind,
                &probe.range,
                &search.gaps,
                search
                    .unrefined
                    .as_ref()
                    .map(|tail| (&tail.range, tail.status.as_str())),
                utc_now(),
            )
            .await
    }

    /// Shares the sweep finish time with the page and sets the follow and sweep gauges.
    fn publish_progress(&self) -> anyhow::Result<()> {
        let eta = self.sweep_eta();
        let label = &self.node.label;
        let finish = eta
            .and_then(|eta| TimeDelta::from_std(eta).ok())
            .and_then(|delta| utc_now().checked_add_signed(delta));
        {
            let mut etas = self
                .watchdog
                .sweep_etas
                .lock()
                .expect("sweep_etas lock poisoned");
            match finish {
                Some(finish) => etas.insert(label.clone(), finish),
                None => etas.remove(label),
            };
        }
        self.watchdog
            .metrics
            .export_progress(label, &self.progress, eta)
    }

    /// ETA of the running sweep at the pace since `eta_origin`.
    fn sweep_eta(&self) -> Option<Duration> {
        let SweepState::Running(sweep) = &self.progress.sweep else {
            return None;
        };
        let origin = self.eta_origin?;
        eta_at_pace(
            origin.next_height,
            origin.at.elapsed(),
            sweep.next_height,
            sweep.until_height,
        )
    }

    /// Restarts the ETA rate from now at the running sweep's next height; none while waiting.
    fn reset_eta_origin(&mut self) {
        self.eta_origin = match &self.progress.sweep {
            SweepState::Running(sweep) => Some(EtaOrigin {
                at: Instant::now(),
                next_height: sweep.next_height,
            }),
            SweepState::Waiting { .. } => None,
        };
    }

    /// GETs the 404 of `probe` and reads the first missing height from its message; `None` when
    /// the range is served after all.
    async fn missing_height(&mut self, probe: &Probe) -> Result<Option<u64>, Unrefined> {
        tokio::time::sleep(self.watchdog.options.request_interval).await;
        self.watchdog
            .metrics
            .count_probe(&self.node.label, probe.kind);
        let route = probe.route();
        match self
            .watchdog
            .lookup_missing_height(&self.node, &route)
            .await
        {
            Ok(HeightLookup::Named(height)) => Ok(Some(height)),
            Ok(HeightLookup::Present) => Ok(None),
            Ok(HeightLookup::NoHeight) => {
                tracing::debug!(node = %self.node.label, route, "404 body names no height");
                Err(Unrefined {
                    status: "no-height".to_owned(),
                })
            },
            Err(error) => Err(self.refinement_failed(probe.kind, &error)),
        }
    }

    /// Counts a failed refinement request; the range stays chunk level.
    fn refinement_failed(&mut self, kind: DataKind, error: &FetchError) -> Unrefined {
        self.record_error(FetchTarget::Data(kind), error);
        Unrefined {
            status: error.status_label().to_owned(),
        }
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

    /// After `SUMMARY_INTERVAL`: trims to the sweep window (retention only), logs one INFO line,
    /// one alert per qualifying kind (`alert_missing`), a WARN when the node answered with other
    /// failures than 404 (`warn_unhealthy`), exports gauges, resets `errors_since_summary`.
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
            sweep_eta = self
                .sweep_eta()
                .map(|eta| humantime::format_duration(eta).to_string()),
            %summary,
            "summary"
        );
        self.alert_missing(&summary).await?;
        self.warn_unhealthy();
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
            let sweep_changed = clamped.sweep != self.progress.sweep;
            self.progress = clamped;
            if sweep_changed {
                self.reset_eta_origin();
            }
            self.store.save_progress(&self.progress).await?;
        }
        let forgotten = self.store.forget_below(window_start, self.epochs).await?;
        Ok(Some(forgotten))
    }

    /// The oldest qualifying missing range of every kind that should alert now.
    async fn alerts(&self) -> anyhow::Result<Vec<AlertCandidate>> {
        self.store
            .alert_candidates(
                utc_now(),
                self.watchdog.options.alert_missing_after,
                ALERT_MIN_FAILED_PROBES,
                self.tip_height.saturating_sub(ALERT_TIP_MARGIN),
                self.epochs,
            )
            .await
    }

    /// One ERROR on target `query_watchdog::alert` per kind from `alerts`; `url` is the GET of the
    /// range.
    async fn alert_missing(&self, summary: &NodeSummary) -> anyhow::Result<()> {
        for candidate in self.alerts().await? {
            let probe = Probe {
                kind: candidate.kind,
                range: candidate.range.clone(),
            };
            let missing = summary
                .kinds
                .iter()
                .find(|kind| kind.kind == candidate.kind)
                .expect("missing_summary has every kind")
                .missing_count;
            // Node URLs are internal and carry no credentials.
            tracing::error!(
                target: "query_watchdog::alert",
                node = %self.node.label,
                kind = <&str>::from(candidate.kind),
                missing,
                oldest_age_seconds = candidate.age.as_secs(),
                failed_probes = candidate.failed_probes,
                range_start = candidate.range.start,
                range_end = candidate.range.end,
                url = %self.node.route_url(&probe.route()),
                "missing data"
            );
        }
        Ok(())
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
}

/// Asks one node which heights of one range kind it serves, for `find_gaps`.
struct RangeProber<'a> {
    watch: &'a mut NodeWatch,
    kind: DataKind,
}

impl Presence for RangeProber<'_> {
    async fn first_missing(
        &mut self,
        range: Range<u64>,
        known_missing: bool,
    ) -> Result<Option<u64>, Unrefined> {
        let probe = Probe {
            kind: self.kind,
            range,
        };
        if known_missing {
            return self.watch.missing_height(&probe).await;
        }
        match self.watch.send_probe(&probe).await {
            Ok(()) => Ok(None),
            Err(FetchError::Status(StatusCode::NOT_FOUND)) => {
                self.watch.missing_height(&probe).await
            },
            Err(error) => Err(self.watch.refinement_failed(self.kind, &error)),
        }
    }

    async fn is_present(&mut self, height: u64) -> Result<bool, Unrefined> {
        let probe = Probe {
            kind: self.kind,
            range: height..height + 1,
        };
        match self.watch.send_probe(&probe).await {
            Ok(()) => Ok(true),
            Err(FetchError::Status(StatusCode::NOT_FOUND)) => Ok(false),
            Err(error) => Err(self.watch.refinement_failed(self.kind, &error)),
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

/// Time left at the pace since the origin: `remaining * elapsed / done`, in whole seconds; `None`
/// before a height is done.
fn eta_at_pace(
    origin_height: u64,
    elapsed: Duration,
    next_height: u64,
    until_height: u64,
) -> Option<Duration> {
    let done = next_height.saturating_sub(origin_height);
    if done == 0 {
        return None;
    }
    let remaining = until_height.saturating_sub(next_height);
    let secs = elapsed.as_secs_f64() * remaining as f64 / done as f64;
    Duration::try_from_secs_f64(secs)
        .ok()
        .map(|eta| Duration::from_secs(eta.as_secs()))
}

/// Current time at whole-second precision, the precision of persisted times.
pub(crate) fn utc_now() -> DateTime<Utc> {
    Utc::now().trunc_subsecs(0)
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, str::FromStr};

    use axum::{
        Router,
        extract::{Path, State},
        http::{Method, Uri},
        routing::get,
    };
    use prometheus::Registry;
    use sqlx::sqlite::SqliteConnectOptions;
    use tokio::net::TcpListener;

    use super::*;

    /// A node serving every leaf except the `missing` heights, recording what it is asked.
    #[derive(Clone)]
    struct FakeNode {
        missing: Arc<BTreeSet<u64>>,
        requests: Arc<Mutex<Vec<(Method, String)>>>,
    }

    /// Like the availability range route: 404 naming the first missing height, else 200.
    async fn fake_leaf_range(
        State(node): State<FakeNode>,
        method: Method,
        uri: Uri,
        Path((from, until)): Path<(u64, u64)>,
    ) -> (StatusCode, String) {
        node.requests
            .lock()
            .unwrap()
            .push((method, uri.path().to_owned()));
        match node.missing.range(from..until).next() {
            Some(height) => (
                StatusCode::NOT_FOUND,
                format!(r#"{{"Custom":{{"message":"leaf {height} not found","status":404}}}}"#),
            ),
            None => (StatusCode::OK, String::new()),
        }
    }

    async fn serve_fake(missing: impl IntoIterator<Item = u64>) -> (WatchedNode, FakeNode) {
        let fake = FakeNode {
            missing: Arc::new(missing.into_iter().collect()),
            requests: Arc::default(),
        };
        let app = Router::new()
            .route("/v1/availability/leaf/{from}/{until}", get(fake_leaf_range))
            .fallback(|| async { StatusCode::OK })
            .with_state(fake.clone());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });
        (WatchedNode::new(base_url).unwrap(), fake)
    }

    /// A watchdog over an in-memory store with no pacing; its registry shows the metrics.
    async fn test_watchdog(retention_blocks: Option<u64>) -> (Watchdog, Store, Registry) {
        let registry = Registry::new();
        let options = WatchOptions {
            sweep_interval: Duration::from_secs(1),
            request_interval: Duration::ZERO,
            retention_blocks,
            alert_missing_after: Duration::ZERO,
        };
        let store = Store::connect(SqliteConnectOptions::from_str("sqlite::memory:").unwrap())
            .await
            .unwrap();
        let watchdog = Watchdog::new(
            options,
            Arc::new(WatchdogMetrics::new(&registry).unwrap()),
            store.clone(),
            SweepEtas::default(),
        )
        .unwrap();
        (watchdog, store, registry)
    }

    #[tokio::test]
    async fn a_failed_range_probe_is_refined_with_one_head_per_range() {
        let (node, fake) = serve_fake(103..106).await;
        let (watchdog, store, registry) = test_watchdog(None).await;
        let (_tip_sender, tip) = watch::channel(1_000);
        let label = node.label.clone();
        let mut node_watch = watchdog.start_node(node, tip, None).await.unwrap();

        let chunk = Probe {
            kind: DataKind::Leaf,
            range: 100..110,
        };
        node_watch.check_probe(&chunk).await.unwrap();

        let requests = fake.requests.lock().unwrap().clone();
        let paths = |wanted: Method| -> Vec<String> {
            requests
                .iter()
                .filter(|(method, _)| *method == wanted)
                .map(|(_, path)| path.clone())
                .collect()
        };
        let heads = paths(Method::HEAD);
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
        assert_eq!(heads, expected_heads);
        assert_eq!(paths(Method::GET), [chunk_path]);

        let summary = store.node(&label).missing_summary(utc_now()).await.unwrap();
        assert_eq!(summary[0].missing_count, 3);
        assert_eq!(summary[0].oldest.as_ref().unwrap().range, 103..106);

        let probes_total: u64 = registry
            .gather()
            .iter()
            .filter(|family| family.get_name() == "query_watchdog_probes_total")
            .flat_map(|family| family.get_metric())
            .map(|metric| metric.get_counter().get_value() as u64)
            .sum();
        assert_eq!(probes_total, 7, "six HEADs and one GET");
    }

    #[tokio::test]
    async fn the_watch_loop_follows_sweeps_alerts_and_trims() {
        let (node, _fake) = serve_fake(750..760).await;
        let (watchdog, store, _registry) = test_watchdog(Some(300)).await;
        let (_tip_sender, tip) = watch::channel(1_000);
        let label = node.label.clone();
        let mut watch = watchdog.start_node(node, tip, None).await.unwrap();
        let node_store = store.node(&label);
        let progress = || node_store.load_progress();

        // Follow stays the margin behind the tip, so nothing is probed yet.
        assert_eq!(watch.follow_step().await.unwrap(), 0);

        // The sweep covers `[700, 1000)` in chunks, finds the gap exactly, then waits.
        for _ in 0..5 {
            watch.sweep_step().await.unwrap();
        }
        let stored = progress().await.unwrap().unwrap();
        assert!(
            matches!(stored.sweep, SweepState::Waiting { .. }),
            "{stored:?}"
        );
        assert!(stored.last_sweep_completed_at.is_some());
        let summary = watch.summary().await.unwrap();
        let leaf = &summary.kinds[0];
        assert_eq!((leaf.kind, leaf.missing_count), (DataKind::Leaf, 10));
        assert_eq!(leaf.oldest.as_ref().unwrap().range, 750..760);

        // Two more failed 404 probes make the row alert: 3 probes, old enough, below the margin.
        let gap = Probe {
            kind: DataKind::Leaf,
            range: 750..760,
        };
        for _ in 0..2 {
            node_store
                .record_missing(&gap, "404", utc_now())
                .await
                .unwrap();
        }
        let alerts = watch.alerts().await.unwrap();
        let alerting: Vec<_> = alerts.iter().map(|a| (a.kind, a.range.clone())).collect();
        assert_eq!(alerting, [(DataKind::Leaf, 750..760)]);
        watch.tip_height = 755;
        assert!(
            watch.alerts().await.unwrap().is_empty(),
            "inside the tip margin"
        );

        // A later tip moves follow and the window; rows below the window are forgotten.
        watch.tip_height = 1_500;
        assert_eq!(watch.follow_step().await.unwrap(), 5);
        assert_eq!(watch.progress.follow_next_height, 1_100);
        assert_eq!(watch.trim_to_window().await.unwrap(), Some(1));
        assert_eq!(watch.progress.follow_next_height, 1_200);
        assert_eq!(progress().await.unwrap().unwrap(), watch.progress);
        assert_eq!(watch.summary().await.unwrap().kinds[0].missing_count, 0);
    }

    #[test]
    fn urls_with_credentials_are_rejected() {
        let url = |s: &str| Url::parse(s).unwrap();
        assert!(WatchedNode::new(url("http://host:1/")).is_ok());
        assert!(WatchedNode::new(url("http://user:token@host:1/")).is_err());
        assert!(WatchedNode::new(url("http://user@host:1/")).is_err());
    }

    #[test]
    fn eta_at_pace_table() {
        let secs = |s| Some(Duration::from_secs(s));
        let cases = [
            ("nothing done", (0, 100, 0, 1000), None),
            ("quarter done in 100 s", (0, 100, 250, 1000), secs(300)),
            ("origin offset", (500, 100, 750, 1000), secs(100)),
            ("finished", (0, 100, 1000, 1000), secs(0)),
        ];
        for (name, (origin, elapsed, next, until), expected) in cases {
            let eta = eta_at_pace(origin, Duration::from_secs(elapsed), next, until);
            assert_eq!(eta, expected, "{name}");
        }
    }

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
