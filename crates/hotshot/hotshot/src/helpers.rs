use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    fmt::{format::FmtSpan, writer::BoxMakeWriter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

/// A type-erased fmt layer attached over a `Registry`. The OTel bridge layer
/// wraps a subscriber of type `Layered<ErasedFmtLayer, Registry>`.
pub type ErasedFmtLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

/// Composed subscriber type after the fmt layer is applied. Callers building
/// an OTel layer for `initialize_logging_with` should target this subscriber.
pub type FmtSubscriber = tracing_subscriber::layer::Layered<ErasedFmtLayer, Registry>;

/// Initializes logging on stdout.
pub fn initialize_logging() {
    Registry::default().with(fmt_layer(Stream::Stdout)).init();
}

/// Initializes logging on stderr.
///
/// For binaries whose stdout carries data a caller may pipe or capture, rather than diagnostics.
pub fn initialize_logging_on_stderr() {
    Registry::default().with(fmt_layer(Stream::Stderr)).init();
}

/// Initializes logging with an optional extra `Layer` (e.g. an OTel bridge).
///
/// The extra layer must implement `Layer<FmtSubscriber>`. A polymorphic layer
/// like `OpenTelemetryTracingBridge` satisfies this naturally.
pub fn initialize_logging_with<L>(extra: Option<L>)
where
    L: Layer<FmtSubscriber> + Send + Sync + 'static,
{
    Registry::default()
        .with(fmt_layer(Stream::Stdout))
        .with(extra)
        .init();
}

/// Which standard stream the fmt layer writes to.
#[derive(Clone, Copy, Debug)]
pub enum Stream {
    Stdout,
    Stderr,
}

fn fmt_layer(stream: Stream) -> ErasedFmtLayer {
    let span_event_filter = parse_span_filter();
    let writer = match stream {
        Stream::Stdout => BoxMakeWriter::new(std::io::stdout),
        Stream::Stderr => BoxMakeWriter::new(std::io::stderr),
    };
    let json_mode = std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string());
    if json_mode {
        tracing_subscriber::fmt::layer()
            .json()
            .with_writer(writer)
            .with_span_events(span_event_filter)
            .with_filter(EnvFilter::from_default_env())
            .boxed()
    } else {
        tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .with_span_events(span_event_filter)
            .with_filter(EnvFilter::from_default_env())
            .boxed()
    }
}

fn parse_span_filter() -> FmtSpan {
    match std::env::var("RUST_LOG_SPAN_EVENTS") {
        Ok(val) => val
            .split(',')
            .map(|s| match s.trim() {
                "new" => FmtSpan::NEW,
                "enter" => FmtSpan::ENTER,
                "exit" => FmtSpan::EXIT,
                "close" => FmtSpan::CLOSE,
                "active" => FmtSpan::ACTIVE,
                "full" => FmtSpan::FULL,
                _ => FmtSpan::NONE,
            })
            .fold(FmtSpan::NONE, |acc, x| acc | x),
        Err(_) => FmtSpan::NONE,
    }
}
