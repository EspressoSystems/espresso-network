//! Scrubs credential-bearing URLs out of OTLP log records before they leave the process.
//!
//! Applies to every log record, including URLs baked into the rendered `Debug`/`Display` of
//! someone else's error type (`reqwest`, `alloy`, ...).

use std::time::Duration;

use espresso_utils::redact::scrub;
use opentelemetry::{
    InstrumentationScope,
    logs::{AnyValue, LogRecord as _, Logger as _, LoggerProvider as _, Severity},
};
use opentelemetry_sdk::{
    Resource,
    error::OTelSdkResult,
    logs::{LogBatch, LogExporter, SdkLogRecord, SdkLogger, SdkLoggerProvider},
};

#[derive(Debug)]
pub(crate) struct RedactingLogExporter<T> {
    inner: T,
    // `SdkLogRecord` cannot be constructed directly, nor its attributes replaced once set, so a
    // scrubbed record must be rebuilt field by field from a blank one.
    blank_record_source: SdkLogger,
}

impl<T> RedactingLogExporter<T> {
    pub(crate) fn new(inner: T) -> Self {
        let blank_record_source = SdkLoggerProvider::builder()
            .build()
            .logger("redact-exporter");
        Self {
            inner,
            blank_record_source,
        }
    }
}

impl<T: LogExporter> LogExporter for RedactingLogExporter<T> {
    async fn export(&self, batch: LogBatch<'_>) -> OTelSdkResult {
        let scrubbed: Vec<(SdkLogRecord, InstrumentationScope)> = batch
            .iter()
            .map(|(record, scope)| {
                (
                    scrub_record(&self.blank_record_source, record),
                    scope.clone(),
                )
            })
            .collect();
        let refs: Vec<(&SdkLogRecord, &InstrumentationScope)> = scrubbed
            .iter()
            .map(|(record, scope)| (record, scope))
            .collect();
        self.inner.export(LogBatch::new(&refs)).await
    }

    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.inner.shutdown_with_timeout(timeout)
    }

    fn event_enabled(&self, level: Severity, target: &str, name: Option<&str>) -> bool {
        self.inner.event_enabled(level, target, name)
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.inner.set_resource(resource);
    }
}

fn scrub_record(blank_record_source: &SdkLogger, record: &SdkLogRecord) -> SdkLogRecord {
    let mut out = blank_record_source.create_log_record();
    if let Some(name) = record.event_name() {
        out.set_event_name(name);
    }
    if let Some(target) = record.target() {
        out.set_target(target.clone());
    }
    if let Some(timestamp) = record.timestamp() {
        out.set_timestamp(timestamp);
    }
    if let Some(timestamp) = record.observed_timestamp() {
        out.set_observed_timestamp(timestamp);
    }
    if let Some(text) = record.severity_text() {
        out.set_severity_text(text);
    }
    if let Some(number) = record.severity_number() {
        out.set_severity_number(number);
    }
    if let Some(ctx) = record.trace_context() {
        out.set_trace_context(ctx.trace_id, ctx.span_id, ctx.trace_flags);
    }
    if let Some(body) = record.body() {
        out.set_body(scrub_any_value(body));
    }
    out.add_attributes(
        record
            .attributes_iter()
            .map(|(key, value)| (key.clone(), scrub_any_value(value))),
    );
    out
}

/// Recurses through `ListAny`/`Map` so nested string leaves are reached, not just top-level ones.
fn scrub_any_value(value: &AnyValue) -> AnyValue {
    match value {
        AnyValue::String(s) => AnyValue::String(scrub_str(s.as_str()).into()),
        AnyValue::ListAny(items) => {
            AnyValue::ListAny(Box::new(items.iter().map(scrub_any_value).collect()))
        },
        AnyValue::Map(entries) => AnyValue::Map(Box::new(
            entries
                .iter()
                .map(|(k, v)| (k.clone(), scrub_any_value(v)))
                .collect(),
        )),
        other => other.clone(),
    }
}

fn scrub_str(s: &str) -> String {
    if s.contains("://") || s.contains("Url {") {
        scrub(s)
    } else {
        s.to_owned()
    }
}

#[cfg(test)]
mod test {
    use opentelemetry::Key;

    use super::*;

    fn logger() -> SdkLogger {
        SdkLoggerProvider::builder().build().logger("test")
    }

    #[test]
    fn scrubs_body() {
        let logger = logger();
        let mut record = logger.create_log_record();
        record.set_body(AnyValue::String("https://host/v1/FAKEKEY".into()));

        let scrubbed = scrub_record(&logger, &record);

        assert_eq!(
            scrubbed.body(),
            Some(&AnyValue::String("https://host/***".into()))
        );
    }

    #[test]
    fn scrubs_nested_attribute_leaf() {
        let logger = logger();
        let mut record = logger.create_log_record();
        let leaf = AnyValue::String("https://host/v1/FAKEKEY".into());
        let map = AnyValue::Map(Box::new([(Key::new("url"), leaf)].into_iter().collect()));
        record.add_attribute("nested", AnyValue::ListAny(Box::new(vec![map])));

        let scrubbed = scrub_record(&logger, &record);

        let (_, value) = scrubbed
            .attributes_iter()
            .next()
            .expect("attribute present");
        let AnyValue::ListAny(items) = value else {
            panic!("expected ListAny, got {value:?}");
        };
        let AnyValue::Map(entries) = &items[0] else {
            panic!("expected Map, got {:?}", items[0]);
        };
        assert_eq!(
            entries.get(&Key::new("url")),
            Some(&AnyValue::String("https://host/***".into()))
        );
    }

    #[test]
    fn passes_through_record_without_url_unchanged() {
        let logger = logger();
        let mut record = logger.create_log_record();
        record.set_body(AnyValue::String("no url here".into()));
        record.add_attribute("k", "v");

        assert_eq!(scrub_record(&logger, &record), record);
    }
}
