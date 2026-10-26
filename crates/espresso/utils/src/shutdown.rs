//! Graceful shutdown on OS termination signals.

use tokio::signal::unix::{SignalKind, signal};

/// Block until the process receives `SIGINT` or `SIGTERM`, log the signal at
/// `WARN`, and return its name.
///
/// Intended for use in a `tokio::select!` alongside a daemon's main future so
/// the process exits cleanly instead of being hard-killed.
pub async fn wait_for_shutdown_signal() -> &'static str {
    let mut interrupt = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    let signal = tokio::select! {
        _ = interrupt.recv() => "SIGINT",
        _ = terminate.recv() => "SIGTERM",
    };
    tracing::warn!(signal, "received shutdown signal; shutting down gracefully");
    signal
}
