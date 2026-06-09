use espresso_telemetry::{build_write_request, remote_write::WriteRequest};
use prometheus::{
    Counter, Gauge, Histogram, HistogramOpts, Opts, Registry,
    proto::{Metric, MetricFamily, MetricType},
};
use prost::Message;

const NAME_LABEL: &str = "__name__";

// Render a decoded WriteRequest as Prometheus text-exposition lines, one per
// series, preserving emission and label order. Timestamps are dropped so the
// output is deterministic and directly comparable to an expected literal.
fn render(req: &WriteRequest) -> String {
    req.timeseries
        .iter()
        .map(|series| {
            let name = series
                .labels
                .iter()
                .find(|l| l.name == NAME_LABEL)
                .map(|l| l.value.as_str())
                .unwrap_or_default();
            let labels: Vec<String> = series
                .labels
                .iter()
                .filter(|l| l.name != NAME_LABEL)
                .map(|l| format!("{}={:?}", l.name, l.value))
                .collect();
            let value = series.samples[0].value;
            if labels.is_empty() {
                format!("{name} {value}")
            } else {
                format!("{name}{{{}}} {value}", labels.join(","))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn round_trip(req: &WriteRequest) -> WriteRequest {
    let bytes = req.encode_to_vec();
    WriteRequest::decode(&*bytes).expect("decode WriteRequest")
}

fn encode(families: &[MetricFamily]) -> String {
    let req = build_write_request(families).expect("encode succeeds");
    render(&round_trip(&req))
}

// TEST:metrics-counter-encode-ok
#[test]
fn metrics_counter_encode_ok() {
    let registry = Registry::new();
    let counter = Counter::with_opts(Opts::new("blocks_synced_total", "blocks synced")).unwrap();
    registry.register(Box::new(counter.clone())).unwrap();
    counter.inc_by(3.0);

    assert_eq!(encode(&registry.gather()), "blocks_synced_total 3");
}

// TEST:metrics-gauge-encode-ok
#[test]
fn metrics_gauge_encode_ok() {
    let registry = Registry::new();
    let gauge = Gauge::with_opts(Opts::new("peer_count", "connected peers")).unwrap();
    registry.register(Box::new(gauge.clone())).unwrap();
    gauge.set(8.0);

    assert_eq!(encode(&registry.gather()), "peer_count 8");
}

// TEST:metrics-histogram-buckets-ok
//
// A histogram expands to one cumulative series per finite bucket, a trailing
// +Inf bucket carrying sample_count, then _sum and _count.
#[test]
fn metrics_histogram_buckets_ok() {
    let registry = Registry::new();
    let hist = Histogram::with_opts(
        HistogramOpts::new("block_seconds", "block production time").buckets(vec![1.0, 5.0, 10.0]),
    )
    .unwrap();
    registry.register(Box::new(hist.clone())).unwrap();
    for v in [0.5, 3.0, 7.0, 20.0] {
        hist.observe(v);
    }

    assert_eq!(
        encode(&registry.gather()),
        r#"block_seconds_bucket{le="1"} 1
block_seconds_bucket{le="5"} 2
block_seconds_bucket{le="10"} 3
block_seconds_bucket{le="+Inf"} 4
block_seconds_sum 30.5
block_seconds_count 4"#
    );
}

// TEST:metrics-histogram-explicit-plus-inf-bucket-ok
//
// Regression: prometheus accepts `f64::INFINITY` as a user-supplied bucket and
// returns it verbatim from `get_bucket()`. Without dedup, the loop emits a
// `le="+Inf"` series, then the trailing unconditional emit produces another,
// colliding at the receiver. `build_write_request` skips infinite upper bounds
// in the loop so only the trailing emit remains: exactly one +Inf series.
#[test]
fn metrics_histogram_explicit_plus_inf_bucket_ok() {
    let registry = Registry::new();
    let hist = Histogram::with_opts(
        HistogramOpts::new("plus_inf_seconds", "with explicit +Inf bucket").buckets(vec![
            1.0,
            5.0,
            f64::INFINITY,
        ]),
    )
    .unwrap();
    registry.register(Box::new(hist.clone())).unwrap();
    for v in [0.5, 3.0, 99.0] {
        hist.observe(v);
    }

    assert_eq!(
        encode(&registry.gather()),
        r#"plus_inf_seconds_bucket{le="1"} 1
plus_inf_seconds_bucket{le="5"} 2
plus_inf_seconds_bucket{le="+Inf"} 3
plus_inf_seconds_sum 102.5
plus_inf_seconds_count 3"#
    );
}

// TEST:metrics-labels-sorted-ok
//
// Prometheus remote-write 1.0 requires labels sorted by name within each
// series. The rendered braces show the encoded order, so sorted const labels
// (app, region, zone) prove the property directly.
#[test]
fn metrics_labels_sorted_ok() {
    let registry = Registry::new();
    let opts = Opts::new("zebra_total", "with const labels").const_labels(
        [
            ("zone".to_owned(), "us-east-1".to_owned()),
            ("app".to_owned(), "espresso".to_owned()),
            ("region".to_owned(), "use".to_owned()),
        ]
        .into_iter()
        .collect(),
    );
    let counter = Counter::with_opts(opts).unwrap();
    registry.register(Box::new(counter.clone())).unwrap();
    counter.inc();

    assert_eq!(
        encode(&registry.gather()),
        r#"zebra_total{app="espresso",region="use",zone="us-east-1"} 1"#
    );
}

// TEST:metrics-unsupported-type-fails
#[test]
fn metrics_unsupported_type_fails() {
    let mut family = MetricFamily::default();
    family.set_name("widget_summary".to_owned());
    family.set_help("a summary".to_owned());
    family.set_field_type(MetricType::SUMMARY);
    family.set_metric(vec![Metric::default()]);

    let err = build_write_request(&[family]).expect_err("summary should fail");
    let msg = format!("{err:#}").to_lowercase();
    assert!(
        msg.contains("summary") || msg.contains("unsupported"),
        "expected summary/unsupported in error, got: {msg}"
    );
}
