mod lifecycle;
mod push_task;
pub mod remote_write;
mod retry;
pub mod token;

use std::time::{SystemTime, UNIX_EPOCH};

pub use lifecycle::{TelemetryHandle, TelemetryOptions, init, registry, set_registry};
use prometheus::proto::{LabelPair, MetricFamily, MetricType};
use prost::Message;
use remote_write::{Label, Sample, TimeSeries, WriteRequest};
pub use token::{
    Token, TokenParseError, TokenVerifyError, UnauthenticatedToken, load_bls_signing_key,
    parse_bls_signing_key,
};

const NAME_LABEL: &str = "__name__";

pub fn build_write_request(families: &[MetricFamily]) -> anyhow::Result<WriteRequest> {
    let timestamp_ms = now_unix_millis()?;
    let mut series: Vec<TimeSeries> = Vec::new();

    for family in families {
        let name = family.get_name();
        match family.get_field_type() {
            MetricType::COUNTER | MetricType::GAUGE => {
                for metric in family.get_metric() {
                    let value = if family.get_field_type() == MetricType::COUNTER {
                        metric.get_counter().get_value()
                    } else {
                        metric.get_gauge().get_value()
                    };
                    series.push(make_series(name, metric.get_label(), value, timestamp_ms));
                }
            },
            MetricType::HISTOGRAM => {
                let bucket_name = format!("{name}_bucket");
                let sum_name = format!("{name}_sum");
                let count_name = format!("{name}_count");
                for metric in family.get_metric() {
                    let h = metric.get_histogram();
                    for bucket in h.get_bucket() {
                        series.push(make_bucket_series(
                            &bucket_name,
                            metric.get_label(),
                            &format_float(bucket.get_upper_bound()),
                            bucket.get_cumulative_count() as f64,
                            timestamp_ms,
                        ));
                    }
                    series.push(make_bucket_series(
                        &bucket_name,
                        metric.get_label(),
                        "+Inf",
                        h.get_sample_count() as f64,
                        timestamp_ms,
                    ));
                    series.push(make_series(
                        &sum_name,
                        metric.get_label(),
                        h.get_sample_sum(),
                        timestamp_ms,
                    ));
                    series.push(make_series(
                        &count_name,
                        metric.get_label(),
                        h.get_sample_count() as f64,
                        timestamp_ms,
                    ));
                }
            },
            other => {
                anyhow::bail!("unsupported metric type {other:?} for family {name}");
            },
        }
    }

    Ok(WriteRequest {
        timeseries: series,
        ..Default::default()
    })
}

pub fn encode_to_snappy(req: &WriteRequest) -> anyhow::Result<Vec<u8>> {
    let buf = req.encode_to_vec();
    snap::raw::Encoder::new()
        .compress_vec(&buf)
        .map_err(|e| anyhow::anyhow!("snappy compress: {e}"))
}

fn make_series(name: &str, label_pairs: &[LabelPair], value: f64, timestamp_ms: i64) -> TimeSeries {
    let labels = label_pairs_to_labels(label_pairs);
    named_series(name, labels, value, timestamp_ms)
}

fn make_bucket_series(
    name: &str,
    label_pairs: &[LabelPair],
    le: &str,
    value: f64,
    timestamp_ms: i64,
) -> TimeSeries {
    let mut labels = label_pairs_to_labels(label_pairs);
    labels.push(Label {
        name: "le".into(),
        value: le.into(),
    });
    named_series(name, labels, value, timestamp_ms)
}

fn named_series(name: &str, mut labels: Vec<Label>, value: f64, timestamp_ms: i64) -> TimeSeries {
    labels.push(Label {
        name: NAME_LABEL.into(),
        value: name.into(),
    });
    // Prometheus remote-write 1.0 requires labels sorted by name within each
    // TimeSeries. Receivers are typically lenient but the spec is explicit.
    labels.sort_by(|a, b| a.name.cmp(&b.name));
    TimeSeries {
        labels,
        samples: vec![Sample {
            value,
            timestamp: timestamp_ms,
        }],
        ..Default::default()
    }
}

fn label_pairs_to_labels(pairs: &[LabelPair]) -> Vec<Label> {
    pairs
        .iter()
        .map(|p| Label {
            name: p.get_name().to_owned(),
            value: p.get_value().to_owned(),
        })
        .collect()
}

fn format_float(v: f64) -> String {
    if v.is_infinite() {
        if v.is_sign_positive() {
            "+Inf".to_string()
        } else {
            "-Inf".to_string()
        }
    } else {
        format!("{v}")
    }
}

fn now_unix_millis() -> anyhow::Result<i64> {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("system clock is before UNIX_EPOCH: {e}"))?;
    Ok(d.as_millis() as i64)
}
