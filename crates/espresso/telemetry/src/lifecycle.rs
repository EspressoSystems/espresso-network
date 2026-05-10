//! In-process OTel logs export with a BLS-signed JWT, plus a periodic Prometheus
//! remote-write push for metrics.
//!
//! Tracing events flow through an OpenTelemetry log appender + OTLP/HTTP batch
//! exporter; an externally-supplied `prometheus::Registry` is scraped on a
//! tokio interval and POSTed to `/api/v1/write` as snappy-compressed protobuf.
//! Both pipelines share the same six-month BLS-BN254 JWT minted from the
//! staking key.
//!
//! Failure modes:
//! - Proxy/aggregator down: `BatchLogProcessor` queue fills (~2k records) then
//!   drops. Metrics push logs at `warn!` and retries on the next tick.
//!   Neither path uses disk; neither blocks consensus.
//! - JWT misconfig at startup: [`init`] returns `Err`; the caller logs and
//!   continues without telemetry.
//! - Token TTL expires mid-process: not handled here. Operators restart often
//!   enough for a six-month TTL to be fine.
//!
//! Registry threading: callers that build the `prometheus::Registry` after
//! [`init`] (e.g. the API setup runs deep inside the run path) use
//! [`set_registry`] / [`registry`] to hand it off, then
//! [`TelemetryHandle::attach_metrics_push`] to spawn the push task.

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
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

use crate::{UnauthenticatedToken, push_task, retry::RetryingLogExporter};

const DEFAULT_OTLP_ENDPOINT: &str = "https://telemetry.main.net.espresso.network";
const SERVICE_NAME: &str = "espresso-node";

/// Global handoff for the prometheus `Registry` populated by HotShot.
///
/// Callers that build the `Registry` after [`init`] (e.g. the API setup runs
/// deep inside the run path and the data source's `Registry` is not exposed
/// through the closure that wires consensus and telemetry) deposit a clone
/// into this `OnceLock` and the run path reads it back when calling [`init`].
/// Single writer (API setup), single reader (telemetry init), strictly ordered
/// by node startup.
///
/// Tests do not use this static; they pass the registry directly into [`init`].
static REGISTRY: OnceLock<Arc<Registry>> = OnceLock::new();

/// Deposit the `Registry`. Idempotent: subsequent calls are no-ops, since
/// `OnceLock::set` returns `Err`.
pub fn set_registry(registry: Arc<Registry>) {
    let _ = REGISTRY.set(registry);
}

/// Read the registry deposited by [`set_registry`]. Returns `None` if no API
/// setup has run yet — common in tests and CLI tools that don't spin up the
/// HTTP module.
pub fn registry() -> Option<Arc<Registry>> {
    REGISTRY.get().cloned()
}

/// Operator-facing telemetry configuration.
#[derive(Parser, Clone, Derivative)]
#[derivative(Debug)]
pub struct TelemetryOptions {
    /// Master toggle. Telemetry is opt-in.
    #[clap(long, env = "ESPRESSO_NODE_TELEMETRY_ENABLE", default_value = "false")]
    pub enable: bool,

    /// OTLP/HTTP base URL. Defaults to the production aggregator if unset.
    #[clap(long, env = "ESPRESSO_NODE_TELEMETRY_ENDPOINT")]
    pub endpoint: Option<Url>,

    /// `EnvFilter` applied to the OTel log layer only. Local stderr layer is
    /// unaffected. Default `warn` keeps initial network bandwidth modest;
    /// operators can opt into `info` per-target via standard `EnvFilter`
    /// syntax (e.g. `warn,hotshot=info`).
    #[clap(long, env = "ESPRESSO_TELEMETRY_LOG", default_value = "warn")]
    pub log_filter: String,

    /// Seconds between Prometheus remote-write pushes. Operators rarely tune
    /// this; the aggregator handles arbitrary cadences.
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
            enable: false,
            endpoint: None,
            log_filter: "warn".to_owned(),
            metrics_interval_secs: 60,
        }
    }
}

/// Handle to the metrics push thread. Owns a dedicated single-threaded tokio
/// runtime so the push loop is isolated from the node's main runtime — a slow
/// or hung proxy can't starve a consensus worker. Drops the shutdown sender +
/// joins the thread on `TelemetryHandle::shutdown`.
struct MetricsPushHandle {
    shutdown: oneshot::Sender<()>,
    thread: std::thread::JoinHandle<()>,
}

/// Owns the OTel logger provider + (optionally) the metrics push task, so both
/// can be flushed on graceful shutdown.
///
/// The handle stashes the JWT and resolved endpoint so the metrics push task
/// can be spawned later via [`TelemetryHandle::attach_metrics_push`], once the
/// API setup has constructed the `Registry`. Logs and metrics share the JWT
/// minted at `init` time.
pub struct TelemetryHandle {
    logger_provider: SdkLoggerProvider,
    log_filter: String,
    jwt: String,
    endpoint: String,
    metrics_interval: Duration,
    metrics_push: Option<MetricsPushHandle>,
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
    /// Build a `tracing_subscriber::Layer` that bridges tracing events into the
    /// OTLP exporter. Filtered by the configured `log_filter`. Generic over the
    /// target subscriber so callers compose this with whatever stack they wire
    /// up (e.g. the `FmtSubscriber` produced by the consumer's logging init).
    pub fn tracing_layer<S>(&self) -> impl Layer<S> + Send + Sync + 'static
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let bridge = OpenTelemetryTracingBridge::new(&self.logger_provider);
        bridge.with_filter(EnvFilter::new(self.log_filter.clone()))
    }

    /// Spawn the periodic metrics push on its own thread with a dedicated
    /// single-threaded tokio runtime. Idempotent: no-op if already attached.
    /// Does not require the caller to be inside a tokio runtime.
    pub fn attach_metrics_push(&mut self, registry: Arc<Registry>) {
        if self.metrics_push.is_some() {
            return;
        }
        let push_endpoint: Url = match self.endpoint.parse() {
            Ok(u) => u,
            Err(e) => {
                tracing::warn!(
                    endpoint = %self.endpoint,
                    error = %e,
                    "telemetry: cannot parse endpoint as URL; skipping metrics push"
                );
                return;
            },
        };
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let jwt = self.jwt.clone();
        let interval = self.metrics_interval;
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
                    shutdown_rx,
                ));
            }) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(error = %e, "telemetry: cannot spawn metrics push thread");
                return;
            },
        };
        self.metrics_push = Some(MetricsPushHandle {
            shutdown: shutdown_tx,
            thread,
        });
    }

    /// Signal the push thread, await its final flush, then shut down the OTel
    /// logger provider. Best-effort; failures are logged but never bubbled.
    ///
    /// The metrics-push thread runs its own dedicated runtime, so joining it
    /// from any tokio flavor (or no runtime at all) is safe — the joined
    /// thread doesn't depend on the caller's runtime to make progress. The
    /// `push_task::run` final flush is bounded by the inner reqwest 10s
    /// timeout.
    ///
    /// `SdkLoggerProvider::shutdown` is documented as deadlock-prone when
    /// called from a tokio current-thread runtime, so it's offloaded to a
    /// fresh OS thread.
    pub fn shutdown(self) {
        if let Some(MetricsPushHandle { shutdown, thread }) = self.metrics_push {
            let _ = shutdown.send(());
            if thread.join().is_err() {
                tracing::warn!("telemetry: metrics push thread panicked");
            }
        }
        let provider = self.logger_provider;
        let join = std::thread::Builder::new()
            .name("espresso-telemetry-shutdown".into())
            .spawn(move || {
                if let Err(e) = provider.shutdown() {
                    tracing::warn!(error = %e, "telemetry: logger provider shutdown error");
                }
            });
        match join {
            Ok(j) => {
                if j.join().is_err() {
                    tracing::warn!("telemetry: logger provider shutdown thread panicked");
                }
            },
            Err(e) => tracing::warn!(error = %e, "telemetry: cannot spawn shutdown thread"),
        }
    }
}

/// Initialize the OTel logger pipeline and (when `registry` is `Some`) spawn
/// the periodic Prometheus remote-write push task immediately.
///
/// In production, the `Registry` isn't available at logs-init time (the API
/// setup runs later). The call site passes `None` here and later calls
/// [`TelemetryHandle::attach_metrics_push`] once the registry is built. Tests
/// pass the registry directly.
///
/// Returns `Ok(None)` when telemetry is disabled. Returns `Err` for misconfig
/// (bad endpoint, JWT mint failure). The call site is expected to log and
/// continue without telemetry.
pub fn init(
    opts: &TelemetryOptions,
    staking_key: &SignKey,
    node_name: Option<&str>,
    company_name: Option<&str>,
    registry: Option<Arc<Registry>>,
) -> anyhow::Result<Option<TelemetryHandle>> {
    if !opts.enable {
        return Ok(None);
    }

    let jwt = UnauthenticatedToken::generate_with(staking_key, node_name, company_name)
        .context("mint telemetry JWT")?
        .encode();

    let endpoint = opts
        .endpoint
        .as_ref()
        .map(|u| u.as_str().to_owned())
        .unwrap_or_else(|| DEFAULT_OTLP_ENDPOINT.to_owned());

    // Both signals share this base URL. Reject non-http(s) once here so the
    // metrics push can't silently no-op later; the logs path used to do this
    // check, the metrics path used to skip it.
    let parsed =
        Url::parse(&endpoint).with_context(|| format!("invalid telemetry endpoint: {endpoint}"))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        anyhow::bail!(
            "telemetry endpoint must use http or https scheme, got {scheme:?}: {endpoint}"
        );
    }

    let provider = build_logger_provider(jwt.clone(), &endpoint, node_name)?;

    let metrics_interval = Duration::from_secs(opts.metrics_interval_secs.max(1));
    let mut handle = TelemetryHandle {
        logger_provider: provider,
        log_filter: opts.log_filter.clone(),
        jwt,
        endpoint,
        metrics_interval,
        metrics_push: None,
    };

    if let Some(registry) = registry {
        handle.attach_metrics_push(registry);
    }

    Ok(Some(handle))
}

fn build_logger_provider(
    jwt: String,
    endpoint: &str,
    node_name: Option<&str>,
) -> anyhow::Result<SdkLoggerProvider> {
    let logs_endpoint = format!("{}/v1/logs", endpoint.trim_end_matches('/'));

    let mut headers = HashMap::new();
    headers.insert("authorization".to_string(), format!("Bearer {jwt}"));

    // Gzip the OTLP body. Logs are highly compressible (repeated field names,
    // span attributes, message templates); without this the BatchLogProcessor's
    // 2k-record queue translates to multi-MB payloads on a flush.
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
        // service.instance.id distinguishes individual operators in the
        // aggregator. The JWT also carries node_name, but resource attributes
        // ride on every log record so triage doesn't need a join.
        resource = resource.with_attribute(KeyValue::new("service.instance.id", name.to_owned()));
    }

    Ok(SdkLoggerProvider::builder()
        .with_resource(resource.build())
        .with_batch_exporter(RetryingLogExporter::new(exporter))
        .build())
}
