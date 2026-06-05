//! Unit tests for per-signal telemetry enable flags.
//!
//! Tests serialize on `TEST_LOCK` (same pattern as `rate_limit.rs`) because
//! `init` reads process-global env vars.

use std::{
    net::TcpListener,
    sync::{Arc, Mutex, MutexGuard},
};

use espresso_telemetry::{TelemetryOptions, init};
use jf_signature::{
    SignatureScheme,
    bls_over_bn254::{BLSOverBN254CurveSignatureScheme, SignKey},
};

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock<'a>() -> MutexGuard<'a, ()> {
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
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

fn local_endpoint(port: u16) -> url::Url {
    format!("http://127.0.0.1:{port}").parse().unwrap()
}

// TEST:flags-all-off-returns-none
//
// With both flags false, `init` must return `Ok(None)`.
#[test]
fn all_flags_off_returns_none() {
    let _g = lock();
    let key = make_staking_key();
    let opts = TelemetryOptions {
        logs_enable: false,
        metrics_enable: false,
        endpoint: Some(local_endpoint(reserve_port())),
        ..Default::default()
    };
    let (handle, warnings) =
        init(&opts, &key, None, None, None).expect("disabled init never errors");
    assert!(handle.is_none(), "all flags off must return None");
    assert!(warnings.is_empty(), "no warnings when disabled");
}

// TEST:flags-both-enabled
//
// Both flags on: `tracing_layer() == Some` and `metrics_enabled == true`.
#[tokio::test(flavor = "multi_thread")]
async fn both_flags_enabled() {
    let _g = lock();
    let key = make_staking_key();
    let port = reserve_port();
    let opts = TelemetryOptions {
        logs_enable: true,
        metrics_enable: true,
        endpoint: Some(local_endpoint(port)),
        ..Default::default()
    };
    let (handle, _) = init(&opts, &key, None, None, None).expect("init ok");
    let handle = handle.expect("both flags returns handle");

    // Tracing layer must be present (logs pipeline on).
    assert!(
        handle
            .tracing_layer::<tracing_subscriber::Registry>()
            .is_some(),
        "both flags: tracing_layer must be Some"
    );
    // Metrics pipeline enabled.
    assert!(
        handle.metrics_enabled(),
        "both flags: metrics_enabled must be true"
    );
    handle.shutdown();
}

// TEST:flags-logs-only
//
// `logs_enable=true` alone: tracing_layer Some, metrics_push stays None after
// attach_metrics_push (no-op because metrics_enabled=false).
#[tokio::test(flavor = "multi_thread")]
async fn logs_enable_only() {
    let _g = lock();
    let key = make_staking_key();
    let port = reserve_port();
    let opts = TelemetryOptions {
        logs_enable: true,
        metrics_enable: false,
        endpoint: Some(local_endpoint(port)),
        ..Default::default()
    };
    let (handle, _) = init(&opts, &key, None, None, None).expect("init ok");
    let mut handle = handle.expect("logs_enable returns handle");

    assert!(
        handle
            .tracing_layer::<tracing_subscriber::Registry>()
            .is_some(),
        "logs_enable: tracing_layer must be Some"
    );
    assert!(
        !handle.metrics_enabled(),
        "logs_enable only: metrics_enabled must be false"
    );

    // attach_metrics_push is a no-op when metrics disabled.
    let registry = Arc::new(prometheus::Registry::new());
    handle.attach_metrics_push(registry);
    assert!(
        !handle.metrics_push_active(),
        "attach_metrics_push must be no-op when metrics_enabled=false"
    );

    handle.shutdown();
}

// TEST:flags-metrics-only
//
// `metrics_enable=true` alone: tracing_layer None, attach_metrics_push with a
// Registry activates the push task.
#[tokio::test(flavor = "multi_thread")]
async fn metrics_enable_only() {
    let _g = lock();
    let key = make_staking_key();
    let port = reserve_port();
    let opts = TelemetryOptions {
        logs_enable: false,
        metrics_enable: true,
        endpoint: Some(local_endpoint(port)),
        ..Default::default()
    };
    let (handle, _) = init(&opts, &key, None, None, None).expect("init ok");
    let mut handle = handle.expect("metrics_enable returns handle");

    assert!(
        handle
            .tracing_layer::<tracing_subscriber::Registry>()
            .is_none(),
        "metrics_enable only: tracing_layer must be None"
    );
    assert!(
        handle.metrics_enabled(),
        "metrics_enable: metrics_enabled must be true"
    );
    assert!(
        !handle.metrics_push_active(),
        "metrics push not yet attached before attach_metrics_push"
    );

    // After attaching a registry the push task activates.
    let registry = Arc::new(prometheus::Registry::new());
    handle.attach_metrics_push(registry);
    assert!(
        handle.metrics_push_active(),
        "metrics_push must be active after attach_metrics_push"
    );

    handle.shutdown();
}
