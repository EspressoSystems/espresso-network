//! An RPC client with multiple remote (HTTP) providers that can switch between them.

use super::options::L1ClientOptions;
use crate::Result;
use alloy::{
    rpc::json_rpc::{RequestPacket, ResponsePacket},
    transports::{RpcError, TransportErrorKind, http::Http},
};
use futures::Future;
use parking_lot::RwLock;
use std::{pin::Pin, sync::Arc, time::Instant};
use tide_disco::Url;
use tokio::sync::Notify;
use tower_service::Service;

/// An RPC client with multiple remote (HTTP) providers.
///
/// This client utilizes one RPC provider at a time, but if it detects that the provider is in a
/// failing state, it will automatically switch to the next provider in its list.
#[derive(Clone, Debug)]
pub struct SwitchingTransport {
    /// The transport currently being used by the client
    current_transport: Arc<RwLock<SingleTransport>>,
    opt: Arc<L1ClientOptions>,
    pub(super) switch_notify: Arc<Notify>,
}

/// The state of the current provider being used by a [`SwitchingTransport`].
/// This is cloneable and returns a reference to the same underlying data.
#[derive(Debug, Clone)]
pub(crate) struct SingleTransport {
    generation: usize,
    client: Http<alloy::transports::http::Client>,
    status: Arc<RwLock<SingleTransportStatus>>,
    /// Time at which to revert back to the primary provider after a failover.
    revert_at: Option<Instant>,
}

/// The status of a single transport
#[derive(Debug, Default)]
pub(crate) struct SingleTransportStatus {
    last_failure: Option<Instant>,
    consecutive_failures: usize,
    rate_limited_until: Option<Instant>,
    /// Whether or not this current transport is being shut down (switching to the next transport)
    shutting_down: bool,
}

impl SwitchingTransport {
    /// Create a new `SwitchingTransport` with the given options and URLs
    pub fn new(opt: L1ClientOptions) -> Result<Self> {
        // Return early if there were no URLs provided
        let Some(first_url) = opt.http_providers.first().cloned() else {
            return Err(crate::Error::internal().context("No valid URLs provided"));
        };

        // Create a new `SingleTransport` for the first URL
        let first_transport = Arc::new(RwLock::new(SingleTransport::new(&first_url, 0, None)));

        Ok(Self {
            current_transport: first_transport,
            opt: Arc::new(opt),
            switch_notify: Arc::new(Notify::new()),
        })
    }

    fn urls(&self) -> &[Url] {
        &self.opt.http_providers
    }

    fn switch_to(&self, next_gen: usize, current_transport: SingleTransport) -> SingleTransport {
        let next_index = next_gen % self.urls().len();
        let url = self.urls()[next_index].clone();
        tracing::info!(%url, next_gen, "switch L1 transport");

        let revert_at = if next_gen.is_multiple_of(self.urls().len()) {
            // If we are reverting to the primary transport, clear our scheduled revert time.
            None
        } else if current_transport
            .generation
            .is_multiple_of(self.urls().len())
        {
            // If we are failing over from the primary transport, schedule a time to automatically
            // revert back.
            Some(Instant::now() + self.opt.l1_failover_revert)
        } else {
            // Otherwise keep the currently scheduled revert time.
            current_transport.revert_at
        };

        // Create a new transport from the next URL and index
        let new_transport = SingleTransport::new(&url, next_gen, revert_at);

        // Switch to the next URL
        *self.current_transport.write() = new_transport.clone();

        // Notify the transport that it has been switched
        self.switch_notify.notify_waiters();

        new_transport
    }
}

impl SingleTransport {
    /// Create a new `SingleTransport` with the given URL
    fn new(url: &Url, generation: usize, revert_at: Option<Instant>) -> Self {
        Self {
            generation,
            client: Http::new(url.clone()),
            status: Default::default(),
            revert_at,
        }
    }
}

impl SingleTransportStatus {
    /// Log a successful call to the inner transport
    fn log_success(&mut self) {
        self.consecutive_failures = 0;
    }

    /// Log a failure to call the inner transport. Returns whether or not the transport should be switched to the next URL
    fn log_failure(&mut self, opt: &L1ClientOptions) -> bool {
        // Increment the consecutive failures
        self.consecutive_failures += 1;

        // Check if we should switch to the next URL
        let should_switch = self.should_switch(opt);

        // Update the last failure time
        self.last_failure = Some(Instant::now());

        // Return whether or not we should switch
        should_switch
    }

    /// Whether or not the transport should be switched to the next URL
    fn should_switch(&mut self, opt: &L1ClientOptions) -> bool {
        // If someone else already beat us to switching, return false
        if self.shutting_down {
            return false;
        }

        // If we've reached the max number of consecutive failures, switch to the next URL
        if self.consecutive_failures >= opt.l1_consecutive_failure_tolerance {
            self.shutting_down = true;
            return true;
        }

        // If we've failed recently, switch to the next URL
        let now = Instant::now();
        if let Some(prev) = self.last_failure
            && now.saturating_duration_since(prev) < opt.l1_frequent_failure_tolerance
        {
            self.shutting_down = true;
            return true;
        }

        false
    }

    /// Whether or not the transport should be switched back to the primary URL.
    fn should_revert(&mut self, revert_at: Option<Instant>) -> bool {
        if self.shutting_down {
            // We have already switched away from this transport in another thread.
            return false;
        }
        let Some(revert_at) = revert_at else {
            return false;
        };
        if Instant::now() >= revert_at {
            self.shutting_down = true;
            return true;
        }

        false
    }
}

impl Service<RequestPacket> for SwitchingTransport {
    type Error = RpcError<TransportErrorKind>;
    type Response = ResponsePacket;
    type Future = Pin<
        Box<
            dyn Future<Output = std::result::Result<ResponsePacket, RpcError<TransportErrorKind>>>
                + Send,
        >,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), Self::Error>> {
        // Just poll the (current) inner client
        self.current_transport.read().clone().client.poll_ready(cx)
    }

    fn call(&mut self, req: RequestPacket) -> Self::Future {
        // Clone ourselves
        let self_clone = self.clone();

        // Pin and box, which turns this into a future
        Box::pin(async move {
            // Clone the current transport
            let mut current_transport = self_clone.current_transport.read().clone();

            // Revert back to the primary transport if it's time.
            let should_revert = current_transport
                .status
                .write()
                .should_revert(current_transport.revert_at);
            if should_revert {
                // Switch to the next generation which maps to index 0.
                let n = self_clone.urls().len();
                // Rounding down to a multiple of n gives us the last generation of the primary transport.
                let prev_primary_gen = (current_transport.generation / n) * n;
                // Adding n jumps to the next generation.
                let next_gen = prev_primary_gen + n;
                current_transport = self_clone.switch_to(next_gen, current_transport);
            }

            // If we've been rate limited, back off until the limit (hopefully) expires.
            let rate_limit_until = current_transport.status.read().rate_limited_until;
            if let Some(t) = rate_limit_until {
                if t > Instant::now() {
                    // Return an error with a non-standard code to indicate client-side rate limit.
                    return Err(RpcError::Transport(TransportErrorKind::Custom(
                        "Rate limit exceeded".into(),
                    )));
                } else {
                    // Reset the rate limit if we are passed it so we don't check every time
                    current_transport.status.write().rate_limited_until = None;
                }
            }

            // Call the inner client, match on the result
            match current_transport.client.call(req).await {
                Ok(res) => {
                    // If it's okay, log the success to the status
                    current_transport.status.write().log_success();
                    Ok(res)
                }
                Err(err) => {
                    // Treat rate limited errors specially; these should not cause failover, but instead
                    // should only cause us to temporarily back off on making requests to the RPC
                    // server.
                    if let RpcError::ErrorResp(e) = &err {
                        // 429 == Too Many Requests
                        if e.code == 429 {
                            current_transport.status.write().rate_limited_until =
                                Some(Instant::now() + self_clone.opt.rate_limit_delay());
                            return Err(err);
                        }
                    }

                    // Log the error and indicate a failure
                    tracing::warn!(?err, "L1 client error");

                    // If the transport should switch, do so. We don't need to worry about
                    // race conditions here, since it will only return true once.
                    if current_transport
                        .status
                        .write()
                        .log_failure(&self_clone.opt)
                    {
                        tracing::info!("Switching to next L1 provider due to failures");
                        self_clone.switch_to(current_transport.generation + 1, current_transport);
                    }

                    Err(err)
                }
            }
        })
    }
}
