//! In-process OTel logs export plus a periodic Prometheus remote-write push for
//! metrics. Both pipelines share one six-month BLS-BN254 JWT minted from the
//! staking key. Tracing events go through an OTLP/HTTP batch exporter; an
//! externally supplied `prometheus::Registry` is scraped on a tokio interval and
//! POSTed to `/api/v1/write` as snappy-compressed protobuf.
//!
//! Failure modes:
//! - Proxy down: `BatchLogProcessor` queue fills (~2k records) then drops;
//!   metrics push `warn!`s and retries next tick. Neither uses disk nor blocks
//!   consensus.
//! - JWT misconfig: [`init`] returns `Err`; the caller continues without telemetry.
//! - Token TTL expiry mid-process is not handled; the six-month TTL outlasts
//!   typical restart cadence.

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, atomic::AtomicBool},
    time::Duration,
};

use anyhow::Context;
use clap::Parser;
use derivative::Derivative;
use jf_signature::bls_over_bn254::SignKey;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{Compression, LogExporter, Protocol, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::{Resource, logs::SdkLoggerProvider};
use prometheus::Registry;
use tokio::sync::oneshot;
use tracing::Subscriber;
use tracing_subscriber::{EnvFilter, Layer, registry::LookupSpan};
use url::Url;

use crate::{UnauthenticatedToken, push_task, remote_write::Label};

const SERVICE_NAME: &str = "espresso-node";
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

/// Join `thread`, detaching it after [`SHUTDOWN_TIMEOUT`] so a wedged exporter
/// can't hang shutdown on the main thread.
fn join_bounded(thread: std::thread::JoinHandle<()>, what: &str) {
    let (done_tx, done_rx) = std::sync::mpsc::sync_channel::<()>(0);
    if let Err(e) = std::thread::Builder::new()
        .name("espresso-telemetry-join".into())
        .spawn(move || {
            let _ = thread.join();
            let _ = done_tx.send(());
        })
    {
        tracing::warn!(error = %e, "telemetry: cannot spawn {what} join watcher");
        return;
    }
    if done_rx.recv_timeout(SHUTDOWN_TIMEOUT).is_err() {
        tracing::warn!(
            timeout_secs = SHUTDOWN_TIMEOUT.as_secs(),
            "telemetry: {what} shutdown timed out; detaching thread",
        );
    }
}

/// Global handoff for the prometheus `Registry` populated by HotShot.
///
/// The API setup builds the `Registry` after [`init`] and can't reach the
/// telemetry wiring, so it deposits a clone here for the run path to read back.
/// Single writer, single reader, ordered by node startup. Tests pass the
/// registry directly into [`init`] instead.
static REGISTRY: OnceLock<Arc<Registry>> = OnceLock::new();

/// Deposit the `Registry`. Idempotent: subsequent calls are no-ops, since
/// `OnceLock::set` returns `Err`.
pub fn set_registry(registry: Arc<Registry>) {
    let _ = REGISTRY.set(registry);
}

/// Read the registry deposited by [`set_registry`]. `None` if no API setup has
/// run yet (tests, CLI tools without the HTTP module).
pub fn registry() -> Option<Arc<Registry>> {
    REGISTRY.get().cloned()
}

/// Operator-facing telemetry configuration.
#[derive(Parser, Clone, Derivative)]
#[derivative(Debug)]
pub struct TelemetryOptions {
    /// Enable the OTel logs pipeline.
    #[clap(
        long,
        env = "ESPRESSO_NODE_TELEMETRY_LOGS_ENABLE",
        default_value = "false"
    )]
    pub logs_enable: bool,

    /// Enable the Prometheus remote-write metrics pipeline.
    #[clap(
        long,
        env = "ESPRESSO_NODE_TELEMETRY_METRICS_ENABLE",
        default_value = "false"
    )]
    pub metrics_enable: bool,

    /// OTLP/HTTP base URL override. When unset, the caller selects the default
    /// endpoint (the node picks it by chain ID).
    #[clap(long, env = "ESPRESSO_NODE_TELEMETRY_ENDPOINT")]
    pub endpoint: Option<Url>,

    /// `EnvFilter` for the OTel log layer only; the local stderr layer is
    /// unaffected. Default `warn`; per-target syntax works (e.g.
    /// `warn,hotshot=info`).
    #[clap(long, env = "ESPRESSO_NODE_TELEMETRY_LOG", default_value = "warn")]
    pub log_filter: String,

    /// Seconds between Prometheus remote-write pushes.
    #[clap(
        long,
        env = "ESPRESSO_NODE_TELEMETRY_METRICS_INTERVAL",
        default_value = "60"
    )]
    pub metrics_interval_secs: u64,
}

impl Default for TelemetryOptions {
    fn default() -> Self {
        Self {
            logs_enable: false,
            metrics_enable: false,
            endpoint: None,
            log_filter: "warn".to_owned(),
            metrics_interval_secs: 60,
        }
    }
}

/// Handle to the metrics push thread. The thread owns a dedicated
/// single-threaded runtime so a slow proxy can't starve a consensus worker.
struct MetricsPushHandle {
    shutdown: oneshot::Sender<()>,
    thread: std::thread::JoinHandle<()>,
}

/// Owns the OTel logger provider and, optionally, the metrics push task, so
/// both flush on graceful shutdown. Stashes the JWT and endpoint so the push
/// task can be spawned later via [`TelemetryHandle::attach_metrics_push`] once
/// the API setup has built the `Registry`.
pub struct TelemetryHandle {
    /// `None` when only metrics are enabled (logs pipeline disabled).
    logger_provider: Option<SdkLoggerProvider>,
    log_filter: String,
    jwt: String,
    endpoint: String,
    metrics_interval: Duration,
    metrics_push: Option<MetricsPushHandle>,
    /// When false, `attach_metrics_push` is a no-op.
    metrics_enabled: bool,
    /// Labels stamped onto every pushed TimeSeries (e.g. `service`, `instance`),
    /// mirroring the OTel resource attributes so the aggregator partitions logs
    /// and metrics consistently.
    metrics_external_labels: Vec<Label>,
    /// Latch flipped on the first HTTP 429 from the metrics push, so the
    /// operator-facing ERROR is logged once per process across push ticks. The
    /// logs pipeline retries 429s via opentelemetry-otlp's built-in
    /// `experimental-http-retry` and does not surface them here.
    rate_limit_warned: Arc<AtomicBool>,
    /// `log_filter` embedded verbatim in the rate-limit ERROR so operators see
    /// their active filter.
    telemetry_log_filter: Arc<String>,
}

impl std::fmt::Debug for TelemetryHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryHandle")
            .field("log_filter", &self.log_filter)
            .field("metrics_push_active", &self.metrics_push.is_some())
            .finish()
    }
}

impl TelemetryHandle {
    /// Layer bridging tracing events into the OTLP exporter, filtered by
    /// `log_filter`. `None` when the logs pipeline is disabled.
    pub fn tracing_layer<S>(&self) -> Option<impl Layer<S> + Send + Sync + 'static>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let provider = self.logger_provider.as_ref()?;
        let bridge = OpenTelemetryTracingBridge::new(provider);
        Some(bridge.with_filter(EnvFilter::new(self.log_filter.clone())))
    }

    /// Wrapper over [`attach_metrics_push_buffered`] that emits setup warnings
    /// directly, for callers that already have a subscriber installed.
    pub fn attach_metrics_push(&mut self, registry: Arc<Registry>) {
        let mut deferred = Vec::new();
        self.attach_metrics_push_buffered(registry, &mut deferred);
        for w in deferred {
            tracing::warn!("{w}");
        }
    }

    /// Spawn the periodic metrics push on its own thread. Idempotent: no-op if
    /// already attached or if metrics are disabled. Setup warnings buffer into
    /// `deferred` so [`init`] can replay them after a subscriber is installed.
    fn attach_metrics_push_buffered(
        &mut self,
        registry: Arc<Registry>,
        deferred: &mut Vec<String>,
    ) {
        if !self.metrics_enabled {
            return;
        }
        if self.metrics_push.is_some() {
            return;
        }
        let push_endpoint: Url = match self.endpoint.parse() {
            Ok(u) => u,
            Err(e) => {
                deferred.push(format!(
                    "telemetry: cannot parse endpoint {endpoint:?} as URL ({e}); skipping metrics \
                     push",
                    endpoint = self.endpoint,
                ));
                return;
            },
        };
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let jwt = self.jwt.clone();
        let interval = self.metrics_interval;
        let rate_limit_warned = self.rate_limit_warned.clone();
        let telemetry_log_filter = self.telemetry_log_filter.clone();
        let external_labels = self.metrics_external_labels.clone();
        let thread = match std::thread::Builder::new()
            .name("espresso-telemetry-metrics".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "telemetry: cannot build dedicated runtime; metrics push disabled"
                        );
                        return;
                    },
                };
                rt.block_on(push_task::run(
                    registry,
                    push_endpoint,
                    jwt,
                    interval,
                    external_labels,
                    rate_limit_warned,
                    telemetry_log_filter,
                    shutdown_rx,
                ));
            }) {
            Ok(t) => t,
            Err(e) => {
                deferred.push(format!("telemetry: cannot spawn metrics push thread: {e}"));
                return;
            },
        };
        self.metrics_push = Some(MetricsPushHandle {
            shutdown: shutdown_tx,
            thread,
        });
    }

    /// Flush the push thread, then shut down the OTel logger provider.
    /// Best-effort; failures are logged, never bubbled. Both joins run through
    /// [`join_bounded`] (the provider shutdown can deadlock on a current-thread
    /// runtime).
    pub fn shutdown(self) {
        if let Some(MetricsPushHandle { shutdown, thread }) = self.metrics_push {
            let _ = shutdown.send(());
            join_bounded(thread, "metrics push");
        }
        if let Some(provider) = self.logger_provider {
            match std::thread::Builder::new()
                .name("espresso-telemetry-shutdown".into())
                .spawn(move || {
                    if let Err(e) = provider.shutdown() {
                        tracing::warn!(error = %e, "telemetry: logger provider shutdown error");
                    }
                }) {
                Ok(thread) => join_bounded(thread, "logger provider"),
                Err(e) => tracing::warn!(error = %e, "telemetry: cannot spawn shutdown thread"),
            }
        }
    }

    /// True once the metrics push task is spawned.
    #[doc(hidden)]
    pub fn metrics_push_active(&self) -> bool {
        self.metrics_push.is_some()
    }

    /// True when the metrics pipeline is enabled.
    #[doc(hidden)]
    pub fn metrics_enabled(&self) -> bool {
        self.metrics_enabled
    }
}

/// Initialize the OTel logger pipeline and, when `registry` is `Some`, spawn the
/// metrics push immediately. Production passes `None` (the `Registry` isn't built
/// yet) and later calls [`TelemetryHandle::attach_metrics_push`]; tests pass it
/// directly.
///
/// `Ok((None, _))` when telemetry is disabled; `Err` on misconfig (bad endpoint,
/// JWT mint failure). The returned warnings are produced before any subscriber is
/// installed; callers MUST replay them via `tracing::warn!` afterward or they are
/// lost.
pub fn init(
    opts: &TelemetryOptions,
    staking_key: &SignKey,
    node_name: Option<&str>,
    company_name: Option<&str>,
    endpoint: &Url,
    registry: Option<Arc<Registry>>,
) -> anyhow::Result<(Option<TelemetryHandle>, Vec<String>)> {
    let logs_on = opts.logs_enable;
    let metrics_on = opts.metrics_enable;

    if !logs_on && !metrics_on {
        return Ok((None, Vec::new()));
    }

    let mut deferred: Vec<String> = Vec::new();

    if logs_on && let Err(e) = EnvFilter::try_new(&opts.log_filter) {
        deferred.push(format!(
            "telemetry: invalid log_filter {filter:?} ({e}); falling back to lossy parse, some \
             directives may be ignored",
            filter = opts.log_filter,
        ));
    }

    let jwt = UnauthenticatedToken::generate_with(staking_key, node_name, company_name)
        .context("mint telemetry JWT")?
        .encode();

    // Both signals share this base URL. Reject non-http(s) so the metrics push
    // can't silently no-op later.
    let scheme = endpoint.scheme();
    if scheme != "http" && scheme != "https" {
        anyhow::bail!(
            "telemetry endpoint must use http or https scheme, got {scheme:?}: {endpoint}"
        );
    }
    let endpoint = endpoint.as_str().to_owned();

    let telemetry_log_filter: Arc<String> = Arc::new(opts.log_filter.clone());
    let rate_limit_warned: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let logger_provider = if logs_on {
        Some(build_logger_provider(jwt.clone(), &endpoint, node_name)?)
    } else {
        None
    };

    let metrics_interval = Duration::from_secs(opts.metrics_interval_secs.max(1));
    let mut metrics_external_labels = vec![Label {
        name: "service".to_owned(),
        value: SERVICE_NAME.to_owned(),
    }];
    if let Some(name) = node_name {
        metrics_external_labels.push(Label {
            name: "instance".to_owned(),
            value: name.to_owned(),
        });
    }
    let mut handle = TelemetryHandle {
        logger_provider,
        log_filter: opts.log_filter.clone(),
        jwt,
        endpoint,
        metrics_interval,
        metrics_push: None,
        metrics_enabled: metrics_on,
        metrics_external_labels,
        rate_limit_warned,
        telemetry_log_filter,
    };

    if let Some(registry) = registry {
        handle.attach_metrics_push_buffered(registry, &mut deferred);
    }

    Ok((Some(handle), deferred))
}

fn build_logger_provider(
    jwt: String,
    endpoint: &str,
    node_name: Option<&str>,
) -> anyhow::Result<SdkLoggerProvider> {
    let logs_endpoint = format!("{}/v1/logs", endpoint.trim_end_matches('/'));

    let mut headers = HashMap::new();
    headers.insert("authorization".to_string(), format!("Bearer {jwt}"));

    // Logs are highly compressible; gzip keeps flush payloads small.
    let exporter = LogExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_compression(Compression::Gzip)
        .with_endpoint(logs_endpoint)
        .with_headers(headers)
        .build()
        .context("build OTLP log exporter")?;

    let mut resource = Resource::builder().with_service_name(SERVICE_NAME);
    if let Some(name) = node_name {
        // service.instance.id distinguishes individual operators in the aggregator.
        resource = resource.with_attribute(KeyValue::new("service.instance.id", name.to_owned()));
    }

    Ok(SdkLoggerProvider::builder()
        .with_resource(resource.build())
        .with_batch_exporter(exporter)
        .build())
}
