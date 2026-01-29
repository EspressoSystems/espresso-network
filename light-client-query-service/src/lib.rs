use clap::ValueEnum;
use log_panics::BacktraceMode;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

/// Controls how logs are displayed and how backtraces are logged on panic.
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
pub enum LogFormat {
    #[default]
    Full,
    Compact,
    Json,
}

pub fn init_logging(fmt: Option<LogFormat>) {
    // Parse the `RUST_LOG_SPAN_EVENTS` environment variable
    let span_event_filter = match std::env::var("RUST_LOG_SPAN_EVENTS") {
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
    };

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(span_event_filter);

    // Conditionally initialize in `json` mode
    if let LogFormat::Json = fmt.unwrap_or_default() {
        let _ = subscriber.json().try_init();
        log_panics::Config::new()
            .backtrace_mode(BacktraceMode::Resolved)
            .install_panic_hook();
    } else {
        let _ = subscriber.try_init();
    }
}
