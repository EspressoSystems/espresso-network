use clap::{Parser, ValueEnum};
pub use hotshot::helpers::FmtSubscriber;
use hotshot::helpers::{initialize_logging, initialize_logging_with};
use log_panics::BacktraceMode;
use tracing_subscriber::Layer;

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

    /// Like `init`, but also attaches an additional tracing `Layer` (e.g. an
    /// OpenTelemetry bridge). Pass `None` for the default behavior.
    ///
    /// The layer must implement `Layer<FmtSubscriber>`; a polymorphic layer
    /// like `OpenTelemetryTracingBridge` works directly without erasure.
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
