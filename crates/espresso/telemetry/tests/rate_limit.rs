//! 429 rate-limit handling: a single shared ERROR across logs+metrics, no
//! crash, no resend, configured filter embedded in the message.

// Tests share the global tracing subscriber + capture buffer, so they run
// sequentially via `TEST_LOCK`. The guard is held across `await` points by
// design: only one test runs at a time, so deadlock is impossible.
#![allow(clippy::await_holding_lock)]

use std::{
    fmt,
    net::TcpListener,
    sync::{
        Arc, Mutex, MutexGuard, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use espresso_telemetry::{TelemetryOptions, init};
use jf_signature::{
    SignatureScheme,
    bls_over_bn254::{BLSOverBN254CurveSignatureScheme, SignKey},
};
use prometheus::{Counter, Opts};
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    Layer, Registry, fmt::format::Writer, layer::SubscriberExt, registry::LookupSpan,
};
use url::Url;

// Tests in this file share the global tracing subscriber. Run them
// sequentially via this mutex.
static TEST_LOCK: Mutex<()> = Mutex::new(());

// Captured ERROR/WARN/INFO log lines. The global subscriber appends here.
// Tests clear it before running and read it after.
static CAPTURED: OnceLock<Arc<Mutex<Vec<String>>>> = OnceLock::new();

fn captured() -> Arc<Mutex<Vec<String>>> {
    CAPTURED
        .get_or_init(|| Arc::new(Mutex::new(Vec::new())))
        .clone()
}

/// A `tracing` layer that formats each event into a single line and appends
/// it to the shared `CAPTURED` buffer. Cross-thread safe; works regardless of
/// whether the event originates inside or outside a tokio runtime.
struct CaptureLayer {
    buf: Arc<Mutex<Vec<String>>>,
}

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut line = String::new();
        let metadata = event.metadata();
        // Prefix with level + target so tests can grep for ERROR.
        line.push_str(&format!("{} {}: ", metadata.level(), metadata.target()));
        let mut visitor = FmtVisitor(&mut line);
        event.record(&mut visitor);
        self.buf.lock().unwrap().push(line);
    }
}

struct FmtVisitor<'a>(&'a mut String);

impl tracing::field::Visit for FmtVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        let _ = std::fmt::write(
            &mut Writer::new(self.0),
            format_args!("{}={:?} ", field.name(), value),
        );
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.push_str(&format!("{}={value} ", field.name()));
    }
}

fn install_global_subscriber() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        let subscriber = Registry::default().with(CaptureLayer { buf: captured() });
        // Set as global default. Best-effort: if another test crate's harness
        // already installed one, the test author is responsible for fixing
        // the conflict, but in this single-file integration test we're the
        // only installer.
        tracing::subscriber::set_global_default(subscriber).expect("install global subscriber");
    });
}

fn make_staking_key() -> SignKey {
    BLSOverBN254CurveSignatureScheme::key_gen(&(), &mut rand::thread_rng())
        .unwrap()
        .0
}

fn reserve_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

#[derive(Default, Clone)]
struct StubState {
    /// Per-path counters used to script status code sequences. Indexed by path.
    metrics_calls: Arc<AtomicUsize>,
    /// When set, return 200 for the Nth-and-later call, 429 for the others
    /// (per `mode`).
    mode: StubMode,
}

#[derive(Default, Clone, Copy)]
enum StubMode {
    #[default]
    Always429,
    /// 200, then 429, then 200, then 200, ...
    OkThen429ThenOk,
}

async fn start_stub(port: u16, mode: StubMode) -> StubState {
    let state = StubState {
        mode,
        ..Default::default()
    };
    let app_state = state.clone();
    let app = Router::new()
        .fallback(
            move |State(s): State<StubState>, req: axum::http::Request<axum::body::Body>| async move {
                let path = req.uri().path().to_owned();
                if path == "/api/v1/write" {
                    let n = s.metrics_calls.fetch_add(1, Ordering::SeqCst);
                    let status = match s.mode {
                        StubMode::Always429 => StatusCode::TOO_MANY_REQUESTS,
                        StubMode::OkThen429ThenOk => match n {
                            0 => StatusCode::OK,
                            1 => StatusCode::TOO_MANY_REQUESTS,
                            _ => StatusCode::OK,
                        },
                    };
                    if status == StatusCode::TOO_MANY_REQUESTS {
                        let mut headers = HeaderMap::new();
                        headers.insert("retry-after", "42".parse().unwrap());
                        return (status, headers, "rate limited").into_response();
                    }
                    return (status, "ok").into_response();
                }
                (StatusCode::OK, "ok").into_response()
            },
        )
        .with_state(app_state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    state
}

fn opts() -> TelemetryOptions {
    TelemetryOptions {
        logs_enable: true,
        metrics_enable: true,
        log_filter: "warn".to_owned(),
        // 1s -> push_task uses 1s after the immediate first tick is skipped.
        metrics_interval_secs: 1,
        ..Default::default()
    }
}

fn build_test_registry() -> Arc<prometheus::Registry> {
    let registry = Arc::new(prometheus::Registry::new());
    let counter = Counter::with_opts(Opts::new("rate_limit_test_total", "test counter")).unwrap();
    registry.register(Box::new(counter.clone())).unwrap();
    counter.inc();
    registry
}

fn lock<'a>() -> MutexGuard<'a, ()> {
    install_global_subscriber();
    let guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    captured().lock().unwrap_or_else(|e| e.into_inner()).clear();
    guard
}

fn count_rate_limit_errors() -> usize {
    captured()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .filter(|line| line.starts_with("ERROR ") && line.contains("telemetry rate limit hit"))
        .count()
}

fn captured_lines() -> Vec<String> {
    captured().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

// TEST:operator-error-once-fails
//
// Stub returns 429 for every metrics push. We tick the push task ~3 times,
// then shut it down. Exactly one ERROR must appear, regardless of how many
// 429s landed.
#[tokio::test(flavor = "multi_thread")]
async fn operator_error_once_fails() {
    let _g = lock();

    let port = reserve_port();
    let stub = start_stub(port, StubMode::Always429).await;
    let endpoint: Url = format!("http://127.0.0.1:{port}").parse().unwrap();

    let registry = build_test_registry();
    let key = make_staking_key();
    let (handle, _warnings) =
        init(&opts(), &key, None, None, &endpoint, Some(registry)).expect("init succeeds");
    let handle = handle.expect("telemetry enabled returns handle");

    // Wait for at least 3 push attempts so we know the dedup actually ran
    // across multiple ticks.
    let started = std::time::Instant::now();
    while stub.metrics_calls.load(Ordering::SeqCst) < 3 {
        if started.elapsed() > Duration::from_secs(8) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    handle.shutdown();

    assert!(
        stub.metrics_calls.load(Ordering::SeqCst) >= 3,
        "stub should have received >=3 push attempts, got {}",
        stub.metrics_calls.load(Ordering::SeqCst)
    );
    assert_eq!(
        count_rate_limit_errors(),
        1,
        "exactly one rate-limit ERROR should fire across {} pushes; got lines: {:?}",
        stub.metrics_calls.load(Ordering::SeqCst),
        captured_lines()
    );
    let line = captured_lines()
        .into_iter()
        .find(|l| l.contains("telemetry rate limit hit"))
        .expect("rate-limit error line");
    assert!(
        line.contains("\"warn\""),
        "ERROR must embed ESPRESSO_NODE_TELEMETRY_LOG value (\"warn\"); got: {line}"
    );
    assert!(
        line.contains("Retry-After: 42s"),
        "ERROR must embed Retry-After=42s from the stub; got: {line}"
    );
}

// TEST:operator-no-crash-ok
//
// Same harness as above; assert the metrics-push thread keeps making
// progress past the 429 (no panic, no early exit) and shuts down cleanly.
#[tokio::test(flavor = "multi_thread")]
async fn operator_no_crash_ok() {
    let _g = lock();

    let port = reserve_port();
    let stub = start_stub(port, StubMode::Always429).await;
    let endpoint: Url = format!("http://127.0.0.1:{port}").parse().unwrap();

    let registry = build_test_registry();
    let key = make_staking_key();
    let (handle, _warnings) =
        init(&opts(), &key, None, None, &endpoint, Some(registry)).expect("init succeeds");
    let handle = handle.expect("telemetry enabled returns handle");

    // Drive several ticks. If the task panicked we'd see metrics_calls
    // plateau early.
    let started = std::time::Instant::now();
    while stub.metrics_calls.load(Ordering::SeqCst) < 3 {
        if started.elapsed() > Duration::from_secs(8) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let before_shutdown = stub.metrics_calls.load(Ordering::SeqCst);
    assert!(
        before_shutdown >= 3,
        "task should keep ticking through 429s, got {before_shutdown}"
    );

    // Clean shutdown: returns without panicking.
    handle.shutdown();
}

// TEST:operator-custom-filter-embedded-ok
//
// The configured `log_filter` is embedded verbatim in the rate-limit ERROR.
#[tokio::test(flavor = "multi_thread")]
async fn operator_custom_filter_embedded_ok() {
    let _g = lock();

    let port = reserve_port();
    let stub = start_stub(port, StubMode::Always429).await;
    let endpoint: Url = format!("http://127.0.0.1:{port}").parse().unwrap();

    let mut opts = opts();
    opts.log_filter = "warn,hotshot=info".to_owned();

    let registry = build_test_registry();
    let key = make_staking_key();
    let (handle, _warnings) =
        init(&opts, &key, None, None, &endpoint, Some(registry)).expect("init succeeds");
    let handle = handle.expect("telemetry enabled returns handle");

    let started = std::time::Instant::now();
    while stub.metrics_calls.load(Ordering::SeqCst) < 1 {
        if started.elapsed() > Duration::from_secs(8) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    handle.shutdown();

    assert!(stub.metrics_calls.load(Ordering::SeqCst) >= 1);
    let line = captured_lines()
        .into_iter()
        .find(|l| l.contains("telemetry rate limit hit"))
        .expect("rate-limit error line");
    assert!(
        line.contains("warn,hotshot=info"),
        "configured filter must be embedded verbatim; got: {line}"
    );
}

// TEST:operator-recovery-no-second-error-ok
//
// Stub returns 200 -> 429 -> 200 -> ... The first 429 must fire ERROR; the
// preceding 200 and the trailing 200s must not unset the dedup latch and
// no second ERROR may fire even if another 429 happened to arrive.
#[tokio::test(flavor = "multi_thread")]
async fn operator_recovery_no_second_error_ok() {
    let _g = lock();

    let port = reserve_port();
    let stub = start_stub(port, StubMode::OkThen429ThenOk).await;
    let endpoint: Url = format!("http://127.0.0.1:{port}").parse().unwrap();

    let registry = build_test_registry();
    let key = make_staking_key();
    let (handle, _warnings) =
        init(&opts(), &key, None, None, &endpoint, Some(registry)).expect("init succeeds");
    let handle = handle.expect("telemetry enabled returns handle");

    // Drive 4 pushes: 200, 429, 200, 200.
    let started = std::time::Instant::now();
    while stub.metrics_calls.load(Ordering::SeqCst) < 4 {
        if started.elapsed() > Duration::from_secs(10) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    handle.shutdown();

    assert!(
        stub.metrics_calls.load(Ordering::SeqCst) >= 4,
        "stub should have received >=4 push attempts, got {}",
        stub.metrics_calls.load(Ordering::SeqCst)
    );
    assert_eq!(
        count_rate_limit_errors(),
        1,
        "single ERROR across recovery sequence; got: {:?}",
        captured_lines()
    );
}
