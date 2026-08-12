//! Process-fatal failure reporting for detached tasks.
//!
//! A failure that leaves the node unable to participate in consensus must
//! terminate the process, but the code that detects it typically runs inside a
//! detached `tokio::spawn` whose outcome nothing observes. Library code calls
//! [`report`]; the binary that owns process shutdown calls [`subscribe`] once
//! and exits when [`Subscriber::failure`] resolves. Without a subscriber
//! (tests and CLIs linking the same libraries) [`report`] only logs, so it can
//! never terminate a test run.

use std::sync::OnceLock;

use tokio::sync::mpsc;

static SENDER: OnceLock<mpsc::Sender<String>> = OnceLock::new();

/// Buffers reports fired before the subscriber awaits [`Subscriber::failure`].
const BUFFER: usize = 16;

/// Receives fatal failure reports. Obtained from [`subscribe`].
pub struct Subscriber(mpsc::Receiver<String>);

/// Register the process-wide subscriber for fatal failures.
///
/// # Panics
///
/// Panics on a second call; only the binary entry point subscribes.
pub fn subscribe() -> Subscriber {
    let (tx, rx) = mpsc::channel(BUFFER);
    SENDER.set(tx).expect("first subscriber");
    Subscriber(rx)
}

/// Report an unrecoverable failure. Logs it and wakes the subscriber, if any.
pub fn report(reason: impl Into<String>) {
    let reason = reason.into();
    tracing::error!(%reason, "fatal failure");
    if let Some(tx) = SENDER.get() {
        // A full buffer already carries enough reports to terminate the process.
        let _ = tx.try_send(reason);
    }
}

impl Subscriber {
    /// Resolve with the first reported fatal failure.
    pub async fn failure(&mut self) -> anyhow::Error {
        let reason = self.0.recv().await.expect("static sender never drops");
        anyhow::anyhow!("fatal failure: {reason}")
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test_log::test(tokio::test)]
    async fn test_report_and_subscribe() {
        // Without a subscriber a report is dropped and the process survives.
        report("before subscribe");

        let mut subscriber = subscribe();
        report("l1 dead");
        let err = subscriber.failure().await;
        assert!(err.to_string().contains("l1 dead"), "{err}");
    }
}
