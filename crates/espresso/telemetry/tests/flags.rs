//! Unit tests for per-signal telemetry enable flags.

use std::{net::TcpListener, sync::Arc};

use espresso_telemetry::{TelemetryOptions, init};
use jf_signature::{
    SignatureScheme,
    bls_over_bn254::{BLSOverBN254CurveSignatureScheme, SignKey},
};

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

// With both flags false, `init` must return `Ok(None)`.
#[test]
fn all_flags_off_returns_none() {
    let key = make_staking_key();
    let opts = TelemetryOptions {
        logs_enable: false,
        metrics_enable: false,
        ..Default::default()
    };
    let endpoint = local_endpoint(reserve_port());
    let (handle, warnings) =
        init(&opts, &key, None, None, &endpoint, None).expect("disabled init never errors");
    assert!(handle.is_none());
    assert!(warnings.is_empty());
}

// Both flags on: `tracing_layer() == Some` and `metrics_enabled == true`.
#[tokio::test(flavor = "multi_thread")]
async fn both_flags_enabled() {
    let key = make_staking_key();
    let endpoint = local_endpoint(reserve_port());
    let opts = TelemetryOptions {
        logs_enable: true,
        metrics_enable: true,
        ..Default::default()
    };
    let (handle, _) = init(&opts, &key, None, None, &endpoint, None).expect("init ok");
    let handle = handle.expect("both flags returns handle");

    assert!(
        handle
            .tracing_layer::<tracing_subscriber::Registry>()
            .is_some()
    );
    assert!(handle.metrics_enabled());
    handle.shutdown();
}

// `logs_enable=true` alone: tracing_layer Some, attach_metrics_push is a no-op
// because metrics are disabled.
#[tokio::test(flavor = "multi_thread")]
async fn logs_enable_only() {
    let key = make_staking_key();
    let endpoint = local_endpoint(reserve_port());
    let opts = TelemetryOptions {
        logs_enable: true,
        metrics_enable: false,
        ..Default::default()
    };
    let (handle, _) = init(&opts, &key, None, None, &endpoint, None).expect("init ok");
    let mut handle = handle.expect("logs_enable returns handle");

    assert!(
        handle
            .tracing_layer::<tracing_subscriber::Registry>()
            .is_some()
    );
    assert!(!handle.metrics_enabled());

    handle.attach_metrics_push(Arc::new(prometheus::Registry::new()));
    assert!(!handle.metrics_push_active());

    handle.shutdown();
}

// `metrics_enable=true` alone: tracing_layer None, attach_metrics_push with a
// Registry activates the push task.
#[tokio::test(flavor = "multi_thread")]
async fn metrics_enable_only() {
    let key = make_staking_key();
    let endpoint = local_endpoint(reserve_port());
    let opts = TelemetryOptions {
        logs_enable: false,
        metrics_enable: true,
        ..Default::default()
    };
    let (handle, _) = init(&opts, &key, None, None, &endpoint, None).expect("init ok");
    let mut handle = handle.expect("metrics_enable returns handle");

    assert!(
        handle
            .tracing_layer::<tracing_subscriber::Registry>()
            .is_none()
    );
    assert!(handle.metrics_enabled());
    assert!(!handle.metrics_push_active());

    handle.attach_metrics_push(Arc::new(prometheus::Registry::new()));
    assert!(handle.metrics_push_active());

    handle.shutdown();
}
