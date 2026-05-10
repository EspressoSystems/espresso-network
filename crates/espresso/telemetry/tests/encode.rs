use espresso_telemetry::{
    build_write_request,
    remote_write::{TimeSeries, WriteRequest},
};
use prometheus::{
    Counter, Gauge, Histogram, HistogramOpts, Opts, Registry,
    proto::{Metric, MetricFamily, MetricType},
};
use prost::Message;

const NAME_LABEL: &str = "__name__";

fn name_of(series: &TimeSeries) -> Option<&str> {
    series
        .labels
        .iter()
        .find(|l| l.name == NAME_LABEL)
        .map(|l| l.value.as_str())
}

fn label_value<'a>(series: &'a TimeSeries, name: &str) -> Option<&'a str> {
    series
        .labels
        .iter()
        .find(|l| l.name == name)
        .map(|l| l.value.as_str())
}

fn round_trip(req: &WriteRequest) -> WriteRequest {
    let bytes = req.encode_to_vec();
    WriteRequest::decode(&*bytes).expect("decode WriteRequest")
}

// TEST:metrics-counter-encode-ok
#[test]
fn metrics_counter_encode_ok() {
    let registry = Registry::new();
    let counter = Counter::with_opts(Opts::new("blocks_synced_total", "blocks synced")).unwrap();
    registry.register(Box::new(counter.clone())).unwrap();
    counter.inc_by(3.0);

    let families = registry.gather();
    let req = build_write_request(&families).expect("encode succeeds");
    let decoded = round_trip(&req);

    let counter_series: Vec<_> = decoded
        .timeseries
        .iter()
        .filter(|s| name_of(s) == Some("blocks_synced_total"))
        .collect();
    assert_eq!(
        counter_series.len(),
        1,
        "exactly one counter series expected"
    );
    let s = counter_series[0];
    assert_eq!(s.samples.len(), 1, "exactly one sample");
    assert_eq!(s.samples[0].value, 3.0, "counter value should be 3.0");
}

// TEST:metrics-gauge-encode-ok
#[test]
fn metrics_gauge_encode_ok() {
    let registry = Registry::new();
    let gauge = Gauge::with_opts(Opts::new("peer_count", "connected peers")).unwrap();
    registry.register(Box::new(gauge.clone())).unwrap();
    gauge.set(8.0);

    let families = registry.gather();
    let req = build_write_request(&families).expect("encode succeeds");
    let decoded = round_trip(&req);

    let gauge_series: Vec<_> = decoded
        .timeseries
        .iter()
        .filter(|s| name_of(s) == Some("peer_count"))
        .collect();
    assert_eq!(gauge_series.len(), 1, "exactly one gauge series expected");
    assert_eq!(gauge_series[0].samples[0].value, 8.0);
}

// TEST:metrics-histogram-buckets-ok
#[test]
fn metrics_histogram_buckets_ok() {
    let registry = Registry::new();
    let buckets = vec![1.0, 5.0, 10.0];
    let hist = Histogram::with_opts(
        HistogramOpts::new("block_seconds", "block production time").buckets(buckets.clone()),
    )
    .unwrap();
    registry.register(Box::new(hist.clone())).unwrap();
    hist.observe(0.5);
    hist.observe(3.0);
    hist.observe(7.0);
    hist.observe(20.0);

    let families = registry.gather();
    let req = build_write_request(&families).expect("encode succeeds");
    let decoded = round_trip(&req);

    let block_series: Vec<_> = decoded
        .timeseries
        .iter()
        .filter(|s| {
            name_of(s)
                .map(|n| n.starts_with("block_seconds"))
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(
        block_series.len(),
        buckets.len() + 3,
        "histogram should expand to N+3 series, got: {:?}",
        block_series
            .iter()
            .map(|s| name_of(s).unwrap_or(""))
            .collect::<Vec<_>>()
    );

    let bucket_value = |le: &str| -> Option<f64> {
        decoded
            .timeseries
            .iter()
            .find(|s| {
                name_of(s) == Some("block_seconds_bucket") && label_value(s, "le") == Some(le)
            })
            .map(|s| s.samples[0].value)
    };
    assert_eq!(bucket_value("1"), Some(1.0));
    assert_eq!(bucket_value("5"), Some(2.0));
    assert_eq!(bucket_value("10"), Some(3.0));
    assert_eq!(bucket_value("+Inf"), Some(4.0));

    let count = decoded
        .timeseries
        .iter()
        .find(|s| name_of(s) == Some("block_seconds_count"))
        .expect("_count series");
    assert_eq!(count.samples[0].value, 4.0);

    let sum = decoded
        .timeseries
        .iter()
        .find(|s| name_of(s) == Some("block_seconds_sum"))
        .expect("_sum series");
    assert_eq!(sum.samples[0].value, 0.5 + 3.0 + 7.0 + 20.0);
}

// TEST:metrics-labels-sorted-ok
// Prometheus remote-write 1.0 requires labels sorted by name within each
// TimeSeries. Vector accepts unsorted in practice, but stricter receivers
// may not.
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

    let families = registry.gather();
    let req = build_write_request(&families).expect("encode succeeds");

    for series in &req.timeseries {
        let names: Vec<&str> = series.labels.iter().map(|l| l.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "labels not sorted in series {names:?}");
        // __name__ should be first lexicographically.
        assert_eq!(names.first().copied(), Some(NAME_LABEL));
    }
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
