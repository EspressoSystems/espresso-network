use std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{Router, extract::State, http::HeaderMap, response::IntoResponse};
use espresso_telemetry::{
    TelemetryHandle, TelemetryOptions, UnauthenticatedToken, init,
    remote_write::{TimeSeries, WriteRequest},
};
use jf_signature::{
    SignatureScheme,
    bls_over_bn254::{BLSOverBN254CurveSignatureScheme, SignKey},
};
use prometheus::{Counter, Opts};
use prost::Message;
use tracing_subscriber::{
    EnvFilter, Layer, Registry as TracingRegistry, fmt::MakeWriter, layer::SubscriberExt,
    registry::LookupSpan,
};
use url::Url;

// Mirror the production subscriber layout (fmt over Registry) so the OTel layer
// composes the same way.
type FmtSubscriber = tracing_subscriber::layer::Layered<
    Box<dyn Layer<TracingRegistry> + Send + Sync + 'static>,
    TracingRegistry,
>;

fn build_subscriber<W>(
    writer: W,
    otel: Option<impl Layer<FmtSubscriber> + Send + Sync + 'static>,
) -> impl tracing::Subscriber + for<'a> LookupSpan<'a> + Send + Sync + 'static
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let fmt: Box<dyn Layer<TracingRegistry> + Send + Sync + 'static> =
        tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .with_ansi(false)
            .with_filter(EnvFilter::new("info"))
            .boxed();
    TracingRegistry::default().with(fmt).with(otel)
}

type CapturedRequest = (String, HeaderMap, Vec<u8>);

#[derive(Clone, Default)]
struct Captured {
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
}

impl Captured {
    fn snapshot(&self) -> Vec<CapturedRequest> {
        self.requests.lock().unwrap().clone()
    }
}

fn reserve_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

async fn start_mock_otlp(port: u16) -> Captured {
    start_mock_otlp_with_failures(port, 0).await
}

/// Like [`start_mock_otlp`], but returns 503 for the first `fail_n` POSTs to
/// `/v1/logs`. All other paths and subsequent log requests return 200.
async fn start_mock_otlp_with_failures(port: u16, fail_n: usize) -> Captured {
    let captured = Captured::default();
    let state = captured.clone();
    let logs_failures_remaining = Arc::new(Mutex::new(fail_n));
    let app = Router::new()
        .fallback({
            let logs_failures_remaining = logs_failures_remaining.clone();
            move |State(s): State<Captured>, req: axum::http::Request<axum::body::Body>| {
                let logs_failures_remaining = logs_failures_remaining.clone();
                async move {
                    let (parts, body) = req.into_parts();
                    let path = parts.uri.path().to_owned();
                    let bytes = axum::body::to_bytes(body, 10 * 1024 * 1024)
                        .await
                        .unwrap_or_default();
                    s.requests
                        .lock()
                        .unwrap()
                        .push((path.clone(), parts.headers, bytes.to_vec()));
                    if path == "/v1/logs" {
                        let mut remaining = logs_failures_remaining.lock().unwrap();
                        if *remaining > 0 {
                            *remaining -= 1;
                            return (axum::http::StatusCode::SERVICE_UNAVAILABLE, "fail")
                                .into_response();
                        }
                    }
                    "ok".into_response()
                }
            }
        })
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    captured
}

fn make_staking_key() -> SignKey {
    BLSOverBN254CurveSignatureScheme::key_gen(&(), &mut rand::thread_rng())
        .unwrap()
        .0
}

fn telemetry_opts(metrics_interval_secs: u64) -> TelemetryOptions {
    TelemetryOptions {
        logs_enable: true,
        metrics_enable: true,
        log_filter: "info".to_owned(),
        metrics_interval_secs,
        ..Default::default()
    }
}

async fn wait_for(captured: &Captured, min: usize, timeout: Duration) -> Vec<CapturedRequest> {
    let start = std::time::Instant::now();
    loop {
        let snap = captured.snapshot();
        if snap.len() >= min {
            return snap;
        }
        if start.elapsed() > timeout {
            return snap;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn wait_for_path(
    captured: &Captured,
    path: &str,
    min: usize,
    timeout: Duration,
) -> Vec<CapturedRequest> {
    let start = std::time::Instant::now();
    loop {
        let snap: Vec<_> = captured
            .snapshot()
            .into_iter()
            .filter(|(p, ..)| p == path)
            .collect();
        if snap.len() >= min {
            return snap;
        }
        if start.elapsed() > timeout {
            return snap;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn decode_remote_write(body: &[u8]) -> WriteRequest {
    let decompressed = snap::raw::Decoder::new()
        .decompress_vec(body)
        .expect("snappy decompress");
    WriteRequest::decode(&*decompressed).expect("decode WriteRequest")
}

fn series_name(series: &TimeSeries) -> Option<&str> {
    series
        .labels
        .iter()
        .find(|l| l.name == "__name__")
        .map(|l| l.value.as_str())
}

#[tokio::test(flavor = "multi_thread")]
async fn telemetry_jwt_mint_ok() {
    let port = reserve_port();
    let captured = start_mock_otlp(port).await;
    let endpoint = format!("http://127.0.0.1:{port}").parse::<Url>().unwrap();

    let key = make_staking_key();
    let opts = telemetry_opts(60);
    let (handle, _warnings) =
        init(&opts, &key, Some("node-42"), Some("acme"), &endpoint, None).expect("init succeeds");
    let handle: TelemetryHandle = handle.expect("telemetry enabled returns handle");

    // Scoped subscriber so the global default (shared across tests) isn't touched.
    let subscriber = build_subscriber(std::io::sink, handle.tracing_layer());
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("hello from test");
    });

    // Force flush.
    handle.shutdown();

    // Poll briefly for delivery.
    let snap = wait_for(&captured, 1, Duration::from_secs(5)).await;
    assert!(!snap.is_empty());

    let (path, headers, body) = &snap[0];
    assert_eq!(path, "/v1/logs");
    assert!(!body.is_empty());

    assert_eq!(
        headers
            .get("content-encoding")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default(),
        "gzip",
    );
    let mut decoder = flate2::read::GzDecoder::new(body.as_slice());
    let mut decompressed = Vec::new();
    std::io::Read::read_to_end(&mut decoder, &mut decompressed).expect("gunzip OTLP body");
    let body_str = String::from_utf8_lossy(&decompressed);
    assert!(
        body_str.contains("service.instance.id") && body_str.contains("node-42"),
        "expected service.instance.id=node-42 in resource attributes; got: {body_str:?}"
    );

    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .expect("authorization header");
    let jwt = auth
        .strip_prefix("Bearer ")
        .expect("auth header must use Bearer scheme");
    let token = UnauthenticatedToken::parse(jwt).expect("parseable JWT");
    let authed = token.verify(60).expect("token verifies");
    assert_eq!(authed.node_name(), Some("node-42"));
    assert_eq!(authed.company_name(), Some("acme"));
}

#[tokio::test(flavor = "multi_thread")]
async fn telemetry_log_retry_survives_transient_5xx() {
    let port = reserve_port();
    // First two POSTs to /v1/logs fail with 503; opentelemetry-otlp's built-in
    // retry (experimental-http-retry) should retry so a final 200 lands.
    let captured = start_mock_otlp_with_failures(port, 2).await;
    let endpoint: Url = format!("http://127.0.0.1:{port}").parse().unwrap();

    let key = make_staking_key();
    let opts = telemetry_opts(60);
    let (handle, _warnings) =
        init(&opts, &key, None, None, &endpoint, None).expect("init succeeds");
    let handle = handle.expect("telemetry enabled returns handle");

    let subscriber = build_subscriber(std::io::sink, handle.tracing_layer());
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("retry-survives marker");
    });
    handle.shutdown();

    // Three log POSTs total (two 503s + one success delivering the same batch).
    let log_requests = wait_for_path(&captured, "/v1/logs", 3, Duration::from_secs(5)).await;
    assert!(
        log_requests.len() >= 3,
        "expected at least 3 /v1/logs POSTs (2 retries + 1 success), got {}",
        log_requests.len()
    );
}

#[test]
fn telemetry_disabled_noop_ok() {
    let key = make_staking_key();
    let opts = TelemetryOptions {
        logs_enable: false,
        metrics_enable: false,
        log_filter: "info".to_owned(),
        metrics_interval_secs: 60,
        ..Default::default()
    };
    let endpoint: Url = "http://does-not-exist.invalid".parse().unwrap();
    // Even when a registry is supplied, disabled init must return None and
    // not spawn any push task.
    let registry = Arc::new(prometheus::Registry::new());
    let (h, warnings) = init(&opts, &key, None, None, &endpoint, Some(registry))
        .expect("disabled init never errors");
    assert!(h.is_none());
    assert!(warnings.is_empty(), "got: {warnings:?}");
}

#[test]
fn telemetry_bad_endpoint_fails() {
    // Non-http(s) scheme: rejected by the explicit scheme check in
    // `build_logger_provider`. OTLP/HTTP only ever speaks http/https.
    let key = make_staking_key();
    let bad: Url = "ftp://example.com".parse().unwrap();
    let opts = telemetry_opts(60);
    let err = init(&opts, &key, None, None, &bad, None).expect_err("bad endpoint must fail");
    let msg = format!("{err:#}").to_lowercase();
    assert!(
        msg.contains("endpoint") || msg.contains("scheme"),
        "expected endpoint-related error, got: {msg}"
    );
}

// Sanity-check that wiring the OTel layer alongside the existing stderr fmt
// layer doesn't suppress stderr output.
#[tokio::test(flavor = "multi_thread")]
async fn telemetry_stderr_untouched_ok() {
    use std::sync::Mutex as StdMutex;

    #[derive(Clone, Default)]
    struct BufWriter(Arc<StdMutex<Vec<u8>>>);

    impl std::io::Write for BufWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl tracing_subscriber::fmt::MakeWriter<'_> for BufWriter {
        type Writer = BufWriter;
        fn make_writer(&self) -> Self::Writer {
            self.clone()
        }
    }

    let port = reserve_port();
    let _captured = start_mock_otlp(port).await;
    let endpoint: Url = format!("http://127.0.0.1:{port}").parse().unwrap();
    let key = make_staking_key();
    let opts = telemetry_opts(60);
    let (handle, _warnings) = init(&opts, &key, None, None, &endpoint, None).unwrap();
    let handle = handle.unwrap();

    let buf = BufWriter::default();
    let subscriber = build_subscriber(buf.clone(), handle.tracing_layer());
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("local fmt layer marker");
    });
    handle.shutdown();

    let captured = String::from_utf8_lossy(&buf.0.lock().unwrap()).into_owned();
    assert!(
        captured.contains("local fmt layer marker"),
        "stderr fmt layer must still emit; got: {captured:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn telemetry_invalid_log_filter_warns() {
    let port = reserve_port();
    let _captured = start_mock_otlp(port).await;
    let endpoint: Url = format!("http://127.0.0.1:{port}").parse().unwrap();
    let key = make_staking_key();
    let bogus = "hotshot=NOT_A_LEVEL";
    let opts = TelemetryOptions {
        logs_enable: true,
        log_filter: bogus.to_owned(),
        metrics_interval_secs: 60,
        ..Default::default()
    };

    let (handle, warnings) = init(&opts, &key, None, None, &endpoint, None).expect("init succeeds");
    let handle = handle.expect("telemetry enabled returns handle");
    handle.shutdown();

    let has_filter_warning = warnings
        .iter()
        .any(|w| w.contains("log_filter") && w.contains(bogus));
    assert!(
        has_filter_warning,
        "expected a deferred warning mentioning `log_filter` and the invalid value, got: \
         {warnings:?}",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn metrics_remote_write_push_ok() {
    let port = reserve_port();
    let captured = start_mock_otlp(port).await;
    let endpoint: Url = format!("http://127.0.0.1:{port}").parse().unwrap();

    let registry = Arc::new(prometheus::Registry::new());
    let counter = Counter::with_opts(Opts::new(
        "espresso_node_test_counter_total",
        "test counter",
    ))
    .unwrap();
    registry.register(Box::new(counter.clone())).unwrap();
    counter.inc_by(7.0);

    let key = make_staking_key();
    // 1s -> push_task uses 1s after skipping the immediate first tick.
    let opts = telemetry_opts(1);
    let (handle, _warnings) =
        init(&opts, &key, None, None, &endpoint, Some(registry.clone())).expect("init succeeds");
    let handle = handle.expect("telemetry enabled returns handle");

    let snap = wait_for_path(&captured, "/api/v1/write", 1, Duration::from_secs(8)).await;
    handle.shutdown();
    assert!(!snap.is_empty());

    let mut found_metric = false;
    let mut saw_jwt = false;
    for (path, headers, body) in &snap {
        assert_eq!(path, "/api/v1/write");
        assert_eq!(
            headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default(),
            "application/x-protobuf",
        );
        assert_eq!(
            headers
                .get("content-encoding")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default(),
            "snappy",
        );
        let auth = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if !auth.is_empty() {
            saw_jwt = true;
            let jwt = auth
                .strip_prefix("Bearer ")
                .expect("metrics auth header must use Bearer scheme");
            let _ = UnauthenticatedToken::parse(jwt).expect("metrics auth header parses as JWT");
        }
        let req = decode_remote_write(body);
        for ts in &req.timeseries {
            if series_name(ts) == Some("espresso_node_test_counter_total")
                && ts.samples.iter().any(|s| s.value >= 7.0)
            {
                found_metric = true;
            }
        }
    }
    assert!(saw_jwt);
    assert!(found_metric);
}

#[tokio::test(flavor = "multi_thread")]
async fn metrics_shared_jwt_ok() {
    let port = reserve_port();
    let captured = start_mock_otlp(port).await;
    let endpoint: Url = format!("http://127.0.0.1:{port}").parse().unwrap();

    let registry = Arc::new(prometheus::Registry::new());
    let counter = Counter::with_opts(Opts::new(
        "espresso_node_shared_jwt_total",
        "shared-jwt counter",
    ))
    .unwrap();
    registry.register(Box::new(counter.clone())).unwrap();
    counter.inc();

    let key = make_staking_key();
    let opts = telemetry_opts(1);
    let (handle, _warnings) =
        init(&opts, &key, None, None, &endpoint, Some(registry.clone())).expect("init succeeds");
    let handle = handle.expect("telemetry enabled returns handle");

    // Emit a log so the OTel batch processor has something to flush.
    let subscriber = build_subscriber(std::io::sink, handle.tracing_layer());
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("shared-jwt test marker");
    });

    // Wait for both pipelines.
    let _ = wait_for_path(&captured, "/v1/logs", 1, Duration::from_secs(5)).await;
    let _ = wait_for_path(&captured, "/api/v1/write", 1, Duration::from_secs(8)).await;
    handle.shutdown();

    let logs = captured
        .snapshot()
        .into_iter()
        .filter(|(p, ..)| p == "/v1/logs")
        .collect::<Vec<_>>();
    let metrics = captured
        .snapshot()
        .into_iter()
        .filter(|(p, ..)| p == "/api/v1/write")
        .collect::<Vec<_>>();
    assert!(!logs.is_empty());
    assert!(!metrics.is_empty());

    let logs_auth = logs[0]
        .1
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .expect("logs auth header")
        .to_owned();
    let metrics_auth = metrics[0]
        .1
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .expect("metrics auth header")
        .to_owned();
    assert_eq!(logs_auth, metrics_auth);
}

#[tokio::test(flavor = "multi_thread")]
async fn metrics_shutdown_flush_ok() {
    let port = reserve_port();
    let captured = start_mock_otlp(port).await;
    let endpoint: Url = format!("http://127.0.0.1:{port}").parse().unwrap();

    let registry = Arc::new(prometheus::Registry::new());
    let counter = Counter::with_opts(Opts::new(
        "espresso_node_shutdown_flush_total",
        "shutdown-flush counter",
    ))
    .unwrap();
    registry.register(Box::new(counter.clone())).unwrap();
    counter.inc();

    let key = make_staking_key();
    // only fire shutdown flush
    let opts = telemetry_opts(600);
    let (handle, _warnings) =
        init(&opts, &key, None, None, &endpoint, Some(registry.clone())).expect("init succeeds");
    let handle = handle.expect("telemetry enabled returns handle");

    // Give the task a moment to install the interval (which it skips).
    tokio::time::sleep(Duration::from_millis(200)).await;
    handle.shutdown();

    // After shutdown, the receiver must have at least one /api/v1/write.
    let snap = wait_for_path(&captured, "/api/v1/write", 1, Duration::from_secs(5)).await;
    assert!(!snap.is_empty());
}
