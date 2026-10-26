use clap::{Parser, ValueEnum};
use log_panics::BacktraceMode;
use tracing_subscriber::{
    EnvFilter, Layer, Registry,
    fmt::{format::FmtSpan, writer::BoxMakeWriter},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

/// Controls how backtraces are logged on panic.
///
/// The values here match the possible values of `RUST_LOG_FORMAT`, and their corresponding behavior
/// on backtrace logging is:
/// * `full`: print a prettified dump of the stack trace and span trace to stdout, optimized for
///   human readability rather than machine parsing
/// * `compact`: output the default panic message, with backtraces controlled by `RUST_BACKTRACE`
/// * `json`: output the panic message and stack trace as a tracing event. This in turn works with
///   the behavior of the tracing subscriber with `RUST_LOG_FORMAT=json` to output the event in a
///   machine-parseable, JSON format.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum BacktraceLoggingMode {
    #[default]
    Full,
    Compact,
    Json,
}

/// Logging configuration.
#[derive(Clone, Debug, Default, Parser)]
pub struct Config {
    #[clap(long, env = "RUST_LOG_FORMAT")]
    backtrace_mode: Option<BacktraceLoggingMode>,
}

impl Config {
    /// Get the logging configuration from the environment.
    pub fn from_env() -> Self {
        Self::parse_from(std::iter::empty::<String>())
    }

    /// Initialize logging and panic handlers based on this configuration.
    pub fn init(&self) {
        initialize_logging();
        self.install_panic_hook();
    }

    /// Like `init`, but logs to stderr so that stdout carries only the program's own output.
    pub fn init_on_stderr(&self) {
        initialize_logging_on_stderr();
        self.install_panic_hook();
    }

    /// Like `init`, but also attaches an extra tracing `Layer` (e.g. an OTel bridge).
    pub fn init_with_otel<L>(&self, otel_layer: Option<L>)
    where
        L: Layer<FmtSubscriber> + Send + Sync + 'static,
    {
        initialize_logging_with(otel_layer);
        self.install_panic_hook();
    }

    fn install_panic_hook(&self) {
        if let BacktraceLoggingMode::Json = self.backtrace_mode.unwrap_or_default() {
            log_panics::Config::new()
                .backtrace_mode(BacktraceMode::Resolved)
                .install_panic_hook();
        }
    }
}

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
