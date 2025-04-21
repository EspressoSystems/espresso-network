use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

/// Initializes logging
pub fn initialize_logging() {
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

    // Conditionally initialize in `json` mode
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(span_event_filter)
            .json()
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_span_events(span_event_filter)
            .try_init();
    };
}
