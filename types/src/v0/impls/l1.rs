use std::{
    cmp::{min, Ordering},
    num::NonZeroUsize,
    pin::Pin,
    result::Result as StdResult,
    sync::Arc,
    time::Instant,
};

use alloy::{
    eips::BlockId,
    hex,
    primitives::{Address, B256, U256},
    providers::{Provider, ProviderBuilder, WsConnect},
    rpc::{
        client::RpcClient,
        json_rpc::{RequestPacket, ResponsePacket},
        types::Block,
    },
    transports::{http::Http, RpcError, TransportErrorKind},
};
use anyhow::Context;
use async_trait::async_trait;
use clap::Parser;
use committable::{Commitment, Committable, RawCommitmentBuilder};
use futures::{
    future::{Future, TryFuture, TryFutureExt},
    stream::{self, StreamExt},
};
use hotshot_contract_adapter::sol_types::FeeContract;
use hotshot_types::traits::metrics::Metrics;
use lru::LruCache;
use parking_lot::RwLock;
use tokio::{
    spawn,
    sync::{Mutex, MutexGuard, Notify},
    time::{sleep, Duration},
};
use tower_service::Service;
use tracing::Instrument;
use url::Url;

use super::{
    v0_1::{L1BlockInfoWithParent, SingleTransport, SingleTransportStatus, SwitchingTransport},
    L1BlockInfo, L1ClientMetrics, L1State, L1UpdateTask,
};
use crate::{FeeInfo, L1Client, L1ClientOptions, L1Event, L1Snapshot};

impl PartialOrd for L1BlockInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for L1BlockInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        self.number.cmp(&other.number)
    }
}

impl From<&Block> for L1BlockInfo {
    fn from(block: &Block) -> Self {
        Self {
            number: block.header.number,
            timestamp: U256::from(block.header.timestamp),
            hash: block.header.hash,
        }
    }
}

impl From<&Block> for L1BlockInfoWithParent {
    fn from(block: &Block) -> Self {
        Self {
            info: block.into(),
            parent_hash: block.header.parent_hash,
        }
    }
}

impl Committable for L1BlockInfo {
    fn commit(&self) -> Commitment<Self> {
        let timestamp: [u8; 32] = self.timestamp.to_le_bytes();

        RawCommitmentBuilder::new(&Self::tag())
            .u64_field("number", self.number)
            // `RawCommitmentBuilder` doesn't have a `u256_field` method, so we simulate it:
            .constant_str("timestamp")
            .fixed_size_bytes(&timestamp)
            .constant_str("hash")
            .fixed_size_bytes(&self.hash.0)
            .finalize()
    }

    fn tag() -> String {
        "L1BLOCK".into()
    }
}

impl L1BlockInfo {
    pub fn number(&self) -> u64 {
        self.number
    }

    pub fn timestamp(&self) -> U256 {
        self.timestamp
    }

    pub fn hash(&self) -> B256 {
        self.hash
    }
}

impl Drop for L1UpdateTask {
    fn drop(&mut self) {
        if let Some(task) = self.0.get_mut().take() {
            task.abort();
        }
    }
}

impl Default for L1ClientOptions {
    fn default() -> Self {
        Self::parse_from(std::iter::empty::<String>())
    }
}

impl L1ClientOptions {
    /// Use the given metrics collector to publish metrics related to the L1 client.
    pub fn with_metrics(mut self, metrics: &(impl Metrics + ?Sized)) -> Self {
        self.metrics = Arc::new(metrics.subgroup("l1".into()));
        self
    }

    /// Instantiate an `L1Client` for a given list of provider `Url`s.
    pub fn connect(self, urls: Vec<Url>) -> anyhow::Result<L1Client> {
        // create custom transport
        let t = SwitchingTransport::new(self, urls)
            .with_context(|| "failed to create switching transport")?;
        // Create a new L1 client with the transport
        Ok(L1Client::with_transport(t))
    }

    fn rate_limit_delay(&self) -> Duration {
        self.l1_rate_limit_delay.unwrap_or(self.l1_retry_delay)
    }
}

impl L1ClientMetrics {
    fn new(metrics: &(impl Metrics + ?Sized), num_urls: usize) -> Self {
        // Create a counter family for the failures per URL
        let failures = metrics.counter_family("failed_requests".into(), vec!["provider".into()]);

        // Create a counter for each URL
        let mut failure_metrics = Vec::with_capacity(num_urls);
        for url_index in 0..num_urls {
            failure_metrics.push(failures.create(vec![url_index.to_string()]));
        }

        Self {
            head: metrics.create_gauge("head".into(), None).into(),
            finalized: metrics.create_gauge("finalized".into(), None).into(),
            reconnects: metrics
                .create_counter("stream_reconnects".into(), None)
                .into(),
            failovers: metrics.create_counter("failovers".into(), None).into(),
            failures: Arc::new(failure_metrics),
        }
    }
}

impl SwitchingTransport {
    /// Create a new `SwitchingTransport` with the given options and URLs
    fn new(opt: L1ClientOptions, urls: Vec<Url>) -> anyhow::Result<Self> {
        // Return early if there were no URLs provided
        let Some(first_url) = urls.first().cloned() else {
            return Err(anyhow::anyhow!("No valid URLs provided"));
        };

        // Create the metrics
        let metrics = L1ClientMetrics::new(&**opt.metrics, urls.len());

        // Create a new `SingleTransport` for the first URL
        let first_transport = Arc::new(RwLock::new(SingleTransport::new(&first_url, 0, None)));

        Ok(Self {
            urls: Arc::new(urls),
            current_transport: first_transport,
            opt: Arc::new(opt),
            metrics,
            switch_notify: Arc::new(Notify::new()),
        })
    }

    /// Returns when the transport has been switched
    async fn wait_switch(&self) {
        self.switch_notify.notified().await;
    }

    fn options(&self) -> &L1ClientOptions {
        &self.opt
    }

    fn metrics(&self) -> &L1ClientMetrics {
        &self.metrics
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
        if let Some(prev) = self.last_failure {
            if now.saturating_duration_since(prev) < opt.l1_frequent_failure_tolerance {
                self.shutting_down = true;
                return true;
            }
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

/// `SwitchingTransport` is an alternative [`Client`](https://docs.rs/alloy/0.12.5/alloy/transports/http/struct.Client.html)
/// which by implementing `tower_service::Service`, traits like [`Transport`](https://docs.rs/alloy/0.12.5/alloy/transports/trait.Transport.html)
/// are auto-derived, thus can be used as an alt [`RpcClient`](https://docs.rs/alloy/0.12.5/alloy/rpc/client/struct.RpcClient.html#method.new)
/// that can be further hooked with `Provider` via `Provider::on_client()`.
#[async_trait]
impl Service<RequestPacket> for SwitchingTransport {
    type Error = RpcError<TransportErrorKind>;
    type Response = ResponsePacket;
    type Future =
        Pin<Box<dyn Future<Output = Result<ResponsePacket, RpcError<TransportErrorKind>>> + Send>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<StdResult<(), Self::Error>> {
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
                let n = self_clone.urls.len();
                // Rounding down to a multiple of n gives us the last generation of the primary transport.
                let prev_primary_gen = (current_transport.generation / n) * n;
                // Adding n jumps to the next generation.
                let next_gen = prev_primary_gen + n;
                current_transport = self_clone.switch_to(next_gen, current_transport);
            }

            // If we've been rate limited, back off until the limit (hopefully) expires.
            if let Some(t) = current_transport.status.read().rate_limited_until {
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
                },
                Err(err) => {
                    // Increment the failure metric
                    if let Some(f) = self_clone
                        .metrics
                        .failures
                        .get(current_transport.generation % self_clone.urls.len())
                    {
                        f.add(1);
                    }

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
                        // Increment the failovers metric
                        self_clone.metrics.failovers.add(1);
                        self_clone.switch_to(current_transport.generation + 1, current_transport);
                    }

                    Err(err)
                },
            }
        })
    }
}

impl SwitchingTransport {
    fn switch_to(&self, next_gen: usize, current_transport: SingleTransport) -> SingleTransport {
        let next_index = next_gen % self.urls.len();
        let url = self.urls[next_index].clone();
        tracing::info!(%url, next_gen, "switch L1 transport");

        let revert_at = if next_gen % self.urls.len() == 0 {
            // If we are reverting to the primary transport, clear our scheduled revert time.
            None
        } else if current_transport.generation % self.urls.len() == 0 {
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

impl L1Client {
    fn with_transport(transport: SwitchingTransport) -> Self {
        // Create a new provider with that RPC client using the custom transport
        let rpc_client = RpcClient::new(transport.clone(), false);
        let provider = ProviderBuilder::new().on_client(rpc_client);

        let opt = transport.options().clone();

        let (sender, mut receiver) = async_broadcast::broadcast(opt.l1_events_channel_capacity);
        receiver.set_await_active(false);
        receiver.set_overflow(true);

        Self {
            provider,
            transport,
            state: Arc::new(Mutex::new(L1State::new(opt.l1_blocks_cache_size))),
            sender,
            receiver: receiver.deactivate(),
            update_task: Default::default(),
        }
    }

    /// Construct a new L1 client with the default options.
    pub fn new(url: Vec<Url>) -> anyhow::Result<Self> {
        L1ClientOptions::default().connect(url)
    }

    /// test only
    pub fn anvil(anvil: &alloy::node_bindings::AnvilInstance) -> anyhow::Result<Self> {
        L1ClientOptions {
            l1_ws_provider: Some(vec![anvil.ws_endpoint().parse()?]),
            ..Default::default()
        }
        .connect(vec![anvil.endpoint().parse()?])
    }

    /// Start the background tasks which keep the L1 client up to date.
    pub async fn spawn_tasks(&self) {
        let mut update_task = self.update_task.0.lock().await;
        if update_task.is_none() {
            *update_task = Some(spawn(self.update_loop()));
        }
    }

    /// Shut down background tasks associated with this L1 client.
    ///
    /// The L1 client will still be usable, but will stop updating until [`start`](Self::start) is
    /// called again.
    pub async fn shut_down_tasks(&self) {
        if let Some(update_task) = self.update_task.0.lock().await.take() {
            update_task.abort();
        }
    }

    fn update_loop(&self) -> impl Future<Output = ()> {
        let opt = self.options().clone();
        let rpc = self.provider.clone();
        let ws_urls = opt.l1_ws_provider.clone();
        let retry_delay = opt.l1_retry_delay;
        let subscription_timeout = opt.subscription_timeout;
        let state = self.state.clone();
        let sender = self.sender.clone();
        let metrics = self.metrics().clone();
        let polling_interval = opt.l1_polling_interval;
        let transport = self.transport.clone();

        let span = tracing::warn_span!("L1 client update");

        async move {

            for i in 0.. {
                let ws;

                // Fetch current L1 head block for the first value of the stream to avoid having
                // to wait for new L1 blocks until the update loop starts processing blocks.
                let l1_head = loop {
                    match rpc.get_block(BlockId::latest()).await {
                        Ok(Some(block)) => break block.header,
                        Ok(None) => {
                            tracing::info!("Failed to fetch L1 head block, will retry");
                        },
                        Err(err) => {
                            tracing::info!("Failed to fetch L1 head block, will retry: err {err}");
                        }
                    }
                    sleep(retry_delay).await;
                };

                // Subscribe to new blocks.
                let mut block_stream = {
                    let res = match &ws_urls {
                        Some(urls) => {
                            // Use a new WebSockets host each time we retry in case there is a
                            // problem with one of the hosts specifically.
                            let provider = i % urls.len();
                            let url = &urls[provider];
                            ws = match ProviderBuilder::new().on_ws(WsConnect::new(url.clone())).await {
                                Ok(ws) => ws,
                                Err(err) => {
                                    tracing::warn!(provider, "Failed to connect WebSockets provider: {err:#}");
                                    sleep(retry_delay).await;
                                    continue;
                                }
                            };
                            ws.subscribe_blocks().await.map(|stream| {stream::once(async { l1_head.clone() }).chain(stream.into_stream()).boxed()})
                        },
                        None => {
                           rpc
                            .watch_blocks()
                            .await
                            .map(|poller_builder| {
                                // Configure it and get the stream
                                let stream = poller_builder.with_poll_interval(polling_interval).into_stream();

                                let rpc = rpc.clone();

                                // For HTTP, we simulate a subscription by polling. The polling
                                // stream provided by ethers only yields block hashes, so for each
                                // one, we have to go fetch the block itself.
                                stream::once(async { l1_head.clone() })
                                 .chain(
                                    stream.map(stream::iter).flatten().filter_map(move |hash| {
                                        let rpc = rpc.clone();
                                        async move {
                                            match rpc.get_block(BlockId::hash(hash)).await {
                                                Ok(Some(block)) => Some(block.header),
                                                // If we can't fetch the block for some reason, we can
                                                // just skip it.
                                                Ok(None) => {
                                                    tracing::warn!(%hash, "HTTP stream yielded a block hash that was not available");
                                                    None
                                                }
                                                Err(err) => {
                                                    tracing::warn!(%hash, "Error fetching block from HTTP stream: {err:#}");
                                                    None
                                                }
                                            }
                                        }
                                    }))
                                // Take until the transport is switched, so we will call `watch_blocks` instantly on it
                            }.take_until(transport.wait_switch())
                            .boxed())
                        }
                    };
                    match res {
                        Ok(stream) => stream,
                        Err(err) => {
                            tracing::error!("Error subscribing to L1 blocks: {err:#}");
                            sleep(retry_delay).await;
                            continue;
                        }
                    }
                };

                tracing::info!("Established L1 block stream");
                loop {
                    // Wait for a block, timing out if we don't get one soon enough
                    let block_timeout = tokio::time::timeout(subscription_timeout, block_stream.next()).await;
                    match block_timeout {
                        // We got a block
                        Ok(Some(head)) => {
                            let head = head.number;
                            tracing::debug!(head, "Received L1 block");

                            // A new block has been produced. This happens fairly rarely, so it is now ok to
                            // poll to see if a new block has been finalized.
                            let finalized = loop {
                                match fetch_finalized_block_from_rpc(&rpc).await {
                                    Ok(finalized) => break finalized,
                                    Err(err) => {
                                        tracing::warn!("Error getting finalized block: {err:#}");
                                        sleep(retry_delay).await;
                                    }
                                }
                            };

                            // Update the state snapshot;
                            let mut state = state.lock().await;
                            if head > state.snapshot.head {
                                tracing::debug!(head, old_head = state.snapshot.head, "L1 head updated");
                                metrics.head.set(head as usize);
                                state.snapshot.head = head;
                                // Emit an event about the new L1 head. Ignore send errors; it just means no
                                // one is listening to events right now.
                                sender
                                    .broadcast_direct(L1Event::NewHead { head })
                                    .await
                                    .ok();
                            }
                            if let Some(finalized) = finalized {
                                if Some(finalized.info) > state.snapshot.finalized {
                                    tracing::info!(
                                        ?finalized,
                                        old_finalized = ?state.snapshot.finalized,
                                        "L1 finalized updated",
                                    );
                                    metrics.finalized.set(finalized.info.number as usize);
                                    state.snapshot.finalized = Some(finalized.info);
                                    state.put_finalized(finalized);
                                    sender
                                        .broadcast_direct(L1Event::NewFinalized { finalized })
                                        .await
                                        .ok();
                                }
                            }
                            tracing::debug!("Updated L1 snapshot to {:?}", state.snapshot);
                        }
                        // The stream ended
                        Ok(None) => {
                            tracing::error!("L1 block stream ended unexpectedly, trying to re-establish block stream");
                            break;
                        }
                        // We timed out waiting for a block
                        Err(_) => {
                            tracing::error!("No block received for {} seconds, trying to re-establish block stream", subscription_timeout.as_secs());
                            break;
                        }
                    }
                }

                metrics.reconnects.add(1);
            }
        }.instrument(span)
    }

    /// Get a snapshot from the l1.
    pub async fn snapshot(&self) -> L1Snapshot {
        self.state.lock().await.snapshot
    }

    /// Wait until the highest L1 block number reaches at least `number`.
    ///
    /// This function does not return any information about the block, since the block is not
    /// necessarily finalized when it returns. It is only used to guarantee that some block at
    /// height `number` exists, possibly in the unsafe part of the L1 chain.
    pub async fn wait_for_block(&self, number: u64) {
        loop {
            // Subscribe to events before checking the current state, to ensure we don't miss a
            // relevant event.
            let mut events = self.receiver.activate_cloned();

            // Check if the block we are waiting for already exists.
            {
                let state = self.state.lock().await;
                if state.snapshot.head >= number {
                    return;
                }
                tracing::info!(number, head = state.snapshot.head, "Waiting for l1 block");
            }

            // Wait for the block.
            while let Some(event) = events.next().await {
                let L1Event::NewHead { head } = event else {
                    continue;
                };
                if head >= number {
                    tracing::info!(number, head, "Got L1 block");
                    return;
                }
                tracing::debug!(number, head, "Waiting for L1 block");
            }

            // This should not happen: the event stream ended. All we can do is try again.
            tracing::warn!(number, "L1 event stream ended unexpectedly; retry");
            self.retry_delay().await;
        }
    }

    /// Get information about the given block.
    ///
    /// If the desired block number is not finalized yet, this function will block until it becomes
    /// finalized.
    pub async fn wait_for_finalized_block(&self, number: u64) -> L1BlockInfo {
        loop {
            // Subscribe to events before checking the current state, to ensure we don't miss a relevant
            // event.
            let mut events = self.receiver.activate_cloned();

            // Check if the block we are waiting for already exists.
            {
                let state = self.state.lock().await;
                if let Some(finalized) = state.snapshot.finalized {
                    if finalized.number >= number {
                        return self.fetch_finalized_block_by_number(state, number).await.1;
                    }
                    tracing::info!(
                        number,
                        finalized = ?state.snapshot.finalized,
                        "waiting for l1 finalized block",
                    );
                };
            }

            // Wait for the block.
            while let Some(event) = events.next().await {
                let L1Event::NewFinalized { finalized } = event else {
                    continue;
                };
                let mut state = self.state.lock().await;
                state.put_finalized(finalized);
                if finalized.info.number >= number {
                    tracing::info!(number, ?finalized, "got finalized L1 block");
                    return self.fetch_finalized_block_by_number(state, number).await.1;
                }
                tracing::debug!(number, ?finalized, "waiting for finalized L1 block");
            }

            // This should not happen: the event stream ended. All we can do is try again.
            tracing::warn!(number, "L1 event stream ended unexpectedly; retry",);
            self.retry_delay().await;
        }
    }

    /// Get information about the first finalized block with timestamp greater than or equal
    /// `timestamp`.
    pub async fn wait_for_finalized_block_with_timestamp(&self, timestamp: U256) -> L1BlockInfo {
        // Wait until the finalized block has timestamp >= `timestamp`.
        let (mut state, mut block) = 'outer: loop {
            // Subscribe to events before checking the current state, to ensure we don't miss a
            // relevant event.
            let mut events = self.receiver.activate_cloned();

            // Check if the block we are waiting for already exists.
            {
                let state = self.state.lock().await;
                if let Some(finalized) = state.snapshot.finalized {
                    if finalized.timestamp >= timestamp {
                        break 'outer (state, finalized);
                    }
                }
                tracing::info!(
                    %timestamp,
                    finalized = ?state.snapshot.finalized,
                    "waiting for L1 finalized block",
                );
            }

            // Wait for the block.
            while let Some(event) = events.next().await {
                let L1Event::NewFinalized { finalized } = event else {
                    continue;
                };
                if finalized.info.timestamp >= timestamp {
                    tracing::info!(%timestamp, ?finalized, "got finalized block");
                    break 'outer (self.state.lock().await, finalized.info);
                }
                tracing::debug!(%timestamp, ?finalized, "waiting for L1 finalized block");
            }

            // This should not happen: the event stream ended. All we can do is try again.
            tracing::warn!(%timestamp, "L1 event stream ended unexpectedly; retry",);
            self.retry_delay().await;
        };

        // It is possible there is some earlier block that also has the proper timestamp. Binary
        // search until we find the true earliest block with timestamp >= `timestamp`.
        //
        // Invariants:
        // * `upper_bound <= lower_bound`
        // * `upper_bound = block.number`
        // * Block number `lower_bound - 1` has timestamp < `timestamp` (strictly)
        // * `block` has timestamp >= `timestamp`
        let mut upper_bound = block.number;
        let mut lower_bound = 0;
        while lower_bound < upper_bound {
            let midpoint = (upper_bound + lower_bound) / 2;
            tracing::debug!(
                lower_bound,
                midpoint,
                upper_bound,
                %timestamp,
                ?block,
                "searching for earliest block with sufficient timestamp"
            );

            let (state_lock, midpoint_block) =
                self.fetch_finalized_block_by_number(state, midpoint).await;
            state = state_lock;

            tracing::debug!(?midpoint_block, %timestamp, "pivot on midpoint block");
            if midpoint_block.timestamp < timestamp {
                lower_bound = midpoint + 1;
            } else {
                upper_bound = midpoint;
                block = midpoint_block;
            }
        }

        block
    }

    async fn fetch_finalized_block_by_number<'a>(
        &'a self,
        mut state: MutexGuard<'a, L1State>,
        number: u64,
    ) -> (MutexGuard<'a, L1State>, L1BlockInfo) {
        let latest_finalized = state
            .snapshot
            .finalized
            .expect("get_finalized_block called before any blocks are finalized");
        assert!(
            number <= latest_finalized.number,
            "requesting a finalized block {number} that isn't finalized; snapshot: {:?}",
            state.snapshot,
        );

        if let Some(safety_margin) = self.options().l1_finalized_safety_margin {
            if number < latest_finalized.number.saturating_sub(safety_margin) {
                // If the requested block height is so old that we can assume all L1 providers have
                // finalized it, we don't need to worry about failing over to a lagging L1 provider
                // which has yet to finalize the block, so we don't need to bother with the
                // expensive hash chaining logic below. Just look up the block by number and assume
                // the response is finalized.
                tracing::debug!(
                    number,
                    ?latest_finalized,
                    "skipping hash check for old finalized block"
                );
                let (state, block) = self
                    .load_and_cache_finalized_block(state, number.into())
                    .await;
                return (state, block.info);
            }
        }

        // To get this block and be sure we are getting the correct finalized block, we first need
        // to find an equal or later block so we can find the expected hash of this block. If we
        // were to just look up the block by number, there could be problems if we failed over to a
        // different (lagging) L1 provider, which has yet to finalize this block and reports a
        // different block with the same number.
        let mut successor_number = number;
        let mut successor = loop {
            if let Some(block) = state.finalized.get(&successor_number) {
                break *block;
            }
            successor_number += 1;
            if successor_number > latest_finalized.number {
                // We don't have any cached finalized block after the requested one; fetch the
                // current finalized block from the network.
                // Don't hold state lock while fetching from network.
                drop(state);
                let block = loop {
                    match fetch_finalized_block_from_rpc(&self.provider).await {
                        Ok(Some(block)) => {
                            break block;
                        },
                        Ok(None) => {
                            tracing::warn!("no finalized block even though finalized snapshot is Some; this can be caused by an L1 client failover");
                            self.retry_delay().await;
                        },
                        Err(err) => {
                            tracing::warn!("Error getting finalized block: {err:#}");
                            self.retry_delay().await;
                        },
                    }
                };
                state = self.state.lock().await;
                state.put_finalized(block);
                break block;
            }
        };

        // Work backwards from the known finalized successor, fetching blocks by parent hash so we
        // know we are getting the correct block.
        while successor.info.number > number {
            tracing::debug!(
                number,
                ?successor,
                "checking hash chaining for finalized block"
            );
            (state, successor) = self
                .load_and_cache_finalized_block(state, successor.parent_hash.into())
                .await;
        }

        (state, successor.info)
    }

    async fn load_and_cache_finalized_block<'a>(
        &'a self,
        mut state: MutexGuard<'a, L1State>,
        id: BlockId,
    ) -> (MutexGuard<'a, L1State>, L1BlockInfoWithParent) {
        // Don't hold state lock while fetching from network.
        drop(state);
        let block = loop {
            let block = match self.provider.get_block(id).await {
                Ok(Some(block)) => block,
                Ok(None) => {
                    tracing::warn!(
                        %id,
                        "provider error: finalized L1 block should always be available"
                    );
                    self.retry_delay().await;
                    continue;
                },
                Err(err) => {
                    tracing::warn!(%id, "failed to get finalized L1 block: {err:#}");
                    self.retry_delay().await;
                    continue;
                },
            };
            break (&block).into();
        };
        state = self.state.lock().await;
        state.put_finalized(block);
        (state, block)
    }

    /// Get fee info for each `Deposit` occurring between `prev`
    /// and `new`. Returns `Vec<FeeInfo>`
    pub async fn get_finalized_deposits(
        &self,
        fee_contract_address: Address,
        prev_finalized: Option<u64>,
        new_finalized: u64,
    ) -> Vec<FeeInfo> {
        // No new blocks have been finalized, therefore there are no
        // new deposits.
        if prev_finalized >= Some(new_finalized) {
            return vec![];
        }

        let opt = self.options();

        // `prev` should have already been processed unless we
        // haven't processed *any* blocks yet.
        let prev = prev_finalized.map(|prev| prev + 1).unwrap_or(0);

        // Divide the range `prev_finalized..=new_finalized` into chunks of size
        // `events_max_block_range`.
        let mut start = prev;
        let end = new_finalized;
        let chunk_size = opt.l1_events_max_block_range;
        let chunks = std::iter::from_fn(move || {
            let chunk_end = min(start + chunk_size - 1, end);
            if chunk_end < start {
                return None;
            }

            let chunk = (start, chunk_end);
            start = chunk_end + 1;
            Some(chunk)
        });

        // Fetch events for each chunk.
        let events = stream::iter(chunks).then(|(from, to)| {
            let retry_delay = opt.l1_retry_delay;
            let fee_contract = FeeContract::new(fee_contract_address, self.provider.clone());
            async move {
                tracing::debug!(from, to, "fetch events in range");

                // query for deposit events, loop until successful.
                loop {
                    match fee_contract
                        .Deposit_filter()
                        .address(*fee_contract.address())
                        .from_block(from)
                        .to_block(to)
                        .query()
                        .await
                    {
                        Ok(events) => break stream::iter(events),
                        Err(err) => {
                            tracing::warn!(from, to, %err, "Fee L1Event Error");
                            sleep(retry_delay).await;
                        },
                    }
                }
            }
        });
        events
            .flatten()
            .map(|(deposit, _)| FeeInfo::from(deposit))
            .collect()
            .await
    }

    /// Check if the given address is a proxy contract.
    pub async fn is_proxy_contract(&self, proxy_address: Address) -> anyhow::Result<bool> {
        // confirm that the proxy_address is a proxy
        // using the implementation slot, 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc, which is the keccak-256 hash of "eip1967.proxy.implementation" subtracted by 1
        let hex_bytes =
            hex::decode("360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc")
                .expect("Failed to decode hex string");
        let implementation_slot = B256::from_slice(&hex_bytes);
        let storage = self
            .provider
            .get_storage_at(proxy_address, implementation_slot.into())
            .await?;

        let implementation_address = Address::from_slice(&storage.to_be_bytes::<32>()[12..]);

        // when the implementation address is not equal to zero, it's a proxy
        Ok(implementation_address != Address::ZERO)
    }

    pub async fn retry_on_all_providers<Fut>(
        &self,
        op: impl Fn() -> Fut,
    ) -> Result<Fut::Ok, Fut::Error>
    where
        Fut: TryFuture,
    {
        let transport = &self.transport;
        let start = transport.current_transport.read().generation % transport.urls.len();
        let end = start + transport.urls.len();
        loop {
            match op().into_future().await {
                Ok(res) => return Ok(res),
                Err(err) => {
                    if transport.current_transport.read().generation >= end {
                        return Err(err);
                    } else {
                        self.retry_delay().await;
                    }
                },
            }
        }
    }

    pub(crate) fn options(&self) -> &L1ClientOptions {
        self.transport.options()
    }

    fn metrics(&self) -> &L1ClientMetrics {
        self.transport.metrics()
    }

    async fn retry_delay(&self) {
        sleep(self.options().l1_retry_delay).await;
    }
}

impl L1State {
    fn new(cache_size: NonZeroUsize) -> Self {
        Self {
            snapshot: Default::default(),
            finalized: LruCache::new(cache_size),
            last_finalized: None,
        }
    }

    fn put_finalized(&mut self, block: L1BlockInfoWithParent) {
        assert!(
            self.snapshot.finalized.is_some()
                && block.info.number <= self.snapshot.finalized.unwrap().number,
            "inserting a finalized block {block:?} that isn't finalized; snapshot: {:?}",
            self.snapshot,
        );

        if Some(block.info.number()) > self.last_finalized {
            self.last_finalized = Some(block.info.number());
        }

        if let Some((old_number, old_block)) = self.finalized.push(block.info.number, block) {
            if old_number == block.info.number && block != old_block {
                tracing::error!(
                    ?old_block,
                    ?block,
                    "got different info for the same finalized height; something has gone very wrong with the L1",
                );
            }
        }
    }
}

async fn fetch_finalized_block_from_rpc(
    rpc: &impl Provider,
) -> anyhow::Result<Option<L1BlockInfoWithParent>> {
    let Some(block) = rpc.get_block(BlockId::finalized()).await? else {
        // This can happen in rare cases where the L1 chain is very young and has not finalized a
        // block yet. This is more common in testing and demo environments. In any case, we proceed
        // with a null L1 block rather than wait for the L1 to finalize a block, which can take a
        // long time.
        tracing::warn!("no finalized block yet");
        return Ok(None);
    };

    Ok(Some((&block).into()))
}

#[cfg(test)]
mod test {
    use std::{ops::Add, time::Duration};

    use alloy::{
        eips::BlockNumberOrTag,
        node_bindings::{Anvil, AnvilInstance},
        primitives::utils::parse_ether,
        providers::layers::AnvilProvider,
    };
    use espresso_contract_deployer::{deploy_fee_contract_proxy, Contracts};
    use portpicker::pick_unused_port;
    use sequencer_utils::test_utils::setup_test;
    use time::OffsetDateTime;

    use super::*;

    async fn new_l1_client_opt(
        anvil: &Arc<AnvilInstance>,
        f: impl FnOnce(&mut L1ClientOptions),
    ) -> L1Client {
        let mut opt = L1ClientOptions {
            l1_events_max_block_range: 1,
            l1_polling_interval: Duration::from_secs(1),
            subscription_timeout: Duration::from_secs(5),
            ..Default::default()
        };
        f(&mut opt);

        let l1_client = opt
            .connect(vec![anvil.endpoint_url()])
            .expect("Failed to create L1 client");

        l1_client.spawn_tasks().await;
        l1_client
    }

    async fn new_l1_client(anvil: &Arc<AnvilInstance>, include_ws: bool) -> L1Client {
        new_l1_client_opt(anvil, |opt| {
            if include_ws {
                opt.l1_ws_provider = Some(vec![anvil.ws_endpoint_url()]);
            }
        })
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_finalized_deposits() -> anyhow::Result<()> {
        setup_test();
        let num_deposits = 5;

        let anvil = Anvil::new().spawn();
        let wallet = anvil.wallet().unwrap();
        let deployer = wallet.default_signer().address();
        let inner_provider = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(anvil.endpoint_url());
        // a provider that holds both anvil (to avoid accidental drop) and wallet-enabled L1 provider
        let provider = AnvilProvider::new(inner_provider, Arc::new(anvil));
        // cache store for deployed contracts
        let mut contracts = Contracts::new();

        // init and kick off the L1Client which wraps around standard L1 provider with more app-specific state management
        let l1_client = new_l1_client(provider.anvil(), false).await;

        // Initialize a contract with some deposits
        let fee_proxy_addr = deploy_fee_contract_proxy(&provider, &mut contracts, deployer).await?;
        let fee_proxy = FeeContract::new(fee_proxy_addr, &provider);
        let num_tx_for_deploy = provider.get_block_number().await?;

        // make some deposits.
        for n in 1..=num_deposits {
            // Varied amounts are less boring.
            let amount = n as f32 / 10.0;
            let receipt = fee_proxy
                .deposit(deployer)
                .value(parse_ether(&amount.to_string())?)
                .send()
                .await?
                .get_receipt()
                .await?;
            assert!(receipt.inner.is_success());
        }

        let cur_height = provider.get_block_number().await?;
        assert_eq!(num_deposits + num_tx_for_deploy, cur_height);

        // Set prev deposits to `None` so `Filter` will start at block
        // 0. The test would also succeed if we pass `0` (b/c first
        // block did not deposit).
        let pending = l1_client
            .get_finalized_deposits(fee_proxy_addr, None, cur_height)
            .await;

        assert_eq!(num_deposits as usize, pending.len(), "{pending:?}");
        assert_eq!(deployer, pending[0].account().0);
        assert_eq!(
            U256::from(1500000000000000000u64),
            pending
                .iter()
                .fold(U256::from(0), |total, info| total.add(info.amount().0))
        );

        // check a few more cases
        let pending = l1_client
            .get_finalized_deposits(fee_proxy_addr, Some(0), cur_height)
            .await;
        assert_eq!(num_deposits as usize, pending.len());

        let pending = l1_client
            .get_finalized_deposits(fee_proxy_addr, Some(0), 0)
            .await;
        assert_eq!(0, pending.len());

        let pending = l1_client
            .get_finalized_deposits(fee_proxy_addr, Some(0), 1)
            .await;
        assert_eq!(0, pending.len());

        let pending = l1_client
            .get_finalized_deposits(fee_proxy_addr, Some(num_tx_for_deploy), num_tx_for_deploy)
            .await;
        assert_eq!(0, pending.len());

        let pending = l1_client
            .get_finalized_deposits(
                fee_proxy_addr,
                Some(num_tx_for_deploy),
                num_tx_for_deploy + 1,
            )
            .await;
        assert_eq!(1, pending.len());

        // what happens if `new_finalized` is `0`?
        let pending = l1_client
            .get_finalized_deposits(fee_proxy_addr, Some(num_tx_for_deploy), 0)
            .await;
        assert_eq!(0, pending.len());

        Ok(())
    }

    async fn test_wait_for_finalized_block_helper(ws: bool) {
        setup_test();

        let anvil = Arc::new(Anvil::new().block_time_f64(0.1).spawn());
        let l1_client = new_l1_client(&anvil, ws).await;
        let provider = &l1_client.provider;

        // Wait for a block 10 blocks in the future.
        let block_height = provider.get_block_number().await.unwrap();
        let block = l1_client.wait_for_finalized_block(block_height + 10).await;
        assert_eq!(block.number, block_height + 10);

        // Compare against underlying provider.
        let true_block = provider
            .get_block(BlockId::Number(BlockNumberOrTag::Number(block_height + 10)))
            .full()
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            block.timestamp.to::<u64>(),
            true_block.header.inner.timestamp
        );
        assert_eq!(block.hash, true_block.header.hash);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_finalized_block_ws() {
        test_wait_for_finalized_block_helper(true).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_finalized_block_http() {
        test_wait_for_finalized_block_helper(false).await
    }

    async fn test_wait_for_old_finalized_block_helper(ws: bool) {
        setup_test();

        let anvil = Arc::new(Anvil::new().block_time_f64(0.2).spawn());
        let l1_client = new_l1_client_opt(&anvil, |opt| {
            if ws {
                opt.l1_ws_provider = Some(vec![anvil.ws_endpoint_url()]);
            }
            opt.l1_finalized_safety_margin = Some(1);
        })
        .await;
        let provider = &l1_client.provider;

        // Wait for anvil to finalize a few blocks.
        l1_client.wait_for_finalized_block(2).await;

        // Get an old finalized block.
        let block = l1_client.wait_for_finalized_block(0).await;

        // Compare against underlying provider.
        let true_block = provider.get_block(0.into()).await.unwrap().unwrap();
        assert_eq!(block.hash, true_block.header.hash);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_old_finalized_block_ws() {
        test_wait_for_old_finalized_block_helper(true).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_old_finalized_block_http() {
        test_wait_for_old_finalized_block_helper(false).await
    }

    async fn test_wait_for_finalized_block_by_timestamp_helper(ws: bool) {
        setup_test();

        let anvil = Arc::new(Anvil::new().block_time_f64(0.2).spawn());
        let l1_client = new_l1_client(&anvil, ws).await;
        let provider = &l1_client.provider;

        // Wait for a block 5 blocks in the future.
        let timestamp = U256::from(OffsetDateTime::now_utc().unix_timestamp() as u64 + 5);
        let block = l1_client
            .wait_for_finalized_block_with_timestamp(timestamp)
            .await;
        assert!(
                block.timestamp >= timestamp,
                "wait_for_finalized_block_with_timestamp({timestamp}) returned too early a block: {block:?}",
            );
        let parent = provider
            .get_block(BlockId::Number(BlockNumberOrTag::Number(block.number - 1)))
            .full()
            .await
            .unwrap()
            .unwrap();
        assert!(
                parent.header.inner.timestamp < timestamp.to::<u64>(),
                "wait_for_finalized_block_with_timestamp({timestamp}) did not return the earliest possible block: returned {block:?}, but earlier block {parent:?} has an acceptable timestamp too",
            );

        // Compare against underlying provider.
        let true_block = provider
            .get_block(BlockId::Number(BlockNumberOrTag::Number(block.number)))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            block.timestamp.to::<u64>(),
            true_block.header.inner.timestamp
        );
        assert_eq!(block.hash, true_block.header.hash);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_finalized_block_by_timestamp_ws() {
        test_wait_for_finalized_block_by_timestamp_helper(true).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_finalized_block_by_timestamp_http() {
        test_wait_for_finalized_block_by_timestamp_helper(false).await
    }

    async fn test_wait_for_old_finalized_block_by_timestamp_helper(ws: bool) {
        setup_test();

        let anvil = Arc::new(Anvil::new().block_time_f64(0.2).spawn());
        let l1_client = new_l1_client(&anvil, ws).await;

        // Get the timestamp of the first block.
        let true_block = l1_client.wait_for_finalized_block(0).await;
        let timestamp = true_block.timestamp;

        // Wait for some more blocks to be produced.
        l1_client.wait_for_finalized_block(10).await;

        // Get the old block by timestamp.
        let block = l1_client
            .wait_for_finalized_block_with_timestamp(U256::from(timestamp))
            .await;
        assert_eq!(block, true_block);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_old_finalized_block_by_timestamp_ws() {
        test_wait_for_old_finalized_block_by_timestamp_helper(true).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_old_finalized_block_by_timestamp_http() {
        test_wait_for_old_finalized_block_by_timestamp_helper(false).await
    }

    async fn test_wait_for_block_helper(ws: bool) {
        setup_test();

        let anvil = Arc::new(Anvil::new().block_time_f64(0.1).spawn());
        let l1_client = new_l1_client(&anvil, ws).await;
        let provider = &l1_client.provider;

        // Wait for a block 10 blocks in the future.
        let block_height = provider.get_block_number().await.unwrap();
        l1_client.wait_for_block(block_height + 10).await;

        let new_block_height = provider.get_block_number().await.unwrap();
        assert!(
                new_block_height >= block_height + 10,
                "wait_for_block returned too early; initial height = {block_height}, new height = {new_block_height}",
            );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_block_ws() {
        test_wait_for_block_helper(true).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_block_http() {
        test_wait_for_block_helper(false).await
    }

    async fn test_reconnect_update_task_helper(ws: bool) {
        setup_test();

        let port = pick_unused_port().unwrap();
        let anvil = Arc::new(Anvil::new().block_time(1).port(port).spawn());
        let client = new_l1_client(&anvil, ws).await;

        let initial_state = client.snapshot().await;
        tracing::info!(?initial_state, "initial state");

        // Check the state is updating.
        let mut retry = 0;
        let updated_state = loop {
            assert!(retry < 10, "state did not update in time");

            let updated_state = client.snapshot().await;
            if updated_state.head > initial_state.head {
                break updated_state;
            }
            tracing::info!(retry, "waiting for state update");
            sleep(Duration::from_secs(1)).await;
            retry += 1;
        };
        tracing::info!(?updated_state, "state updated");

        // Disconnect the WebSocket and reconnect it. Technically this spawns a whole new Anvil
        // chain, but for the purposes of this test it should look to the client like an L1 server
        // closing a WebSocket connection.
        drop(anvil);

        // Let the connection stay down for a little while: Ethers internally tries to reconnect,
        // and starting up to fast again might hit that and cause a false positive. The problem is,
        // Ethers doesn't try very hard, and if we wait a bit, we will test the worst possible case
        // where the internal retry logic gives up and just kills the whole provider.
        tracing::info!("sleep 5");
        sleep(Duration::from_secs(5)).await;

        // Once a connection is reestablished, the state will eventually start to update again.
        tracing::info!("restarting L1");
        let _anvil = Anvil::new().block_time(1).port(port).spawn();

        let mut retry = 0;
        let final_state = loop {
            assert!(retry < 5, "state did not update in time");

            let final_state = client.snapshot().await;
            if final_state.head > updated_state.head {
                break final_state;
            }
            tracing::info!(retry, "waiting for state update");
            sleep(Duration::from_secs(1)).await;
            retry += 1;
        };
        tracing::info!(?final_state, "state updated");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_reconnect_update_task_ws() {
        test_reconnect_update_task_helper(true).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_reconnect_update_task_http() {
        test_reconnect_update_task_helper(false).await
    }

    // #[tokio::test]
    // async fn test_fetch_stake_table() -> anyhow::Result<()> {
    //     setup_test();

    //     let anvil = Anvil::new().spawn();
    //     let wallet = anvil.wallet().unwrap();
    //     let inner_provider = ProviderBuilder::new()
    //         .wallet(wallet)
    //         .on_http(anvil.endpoint_url());
    //     let provider = AnvilProvider::new(inner_provider, Arc::new(anvil));

    //     let l1_client = new_l1_client(provider.anvil(), false).await;
    //     let mut contracts = Contracts::new();

    //     let stake_table_addr =
    //         deploy_permissioned_stake_table(&provider, &mut contracts, vec![]).await?;
    //     let stake_table_contract = PermissionedStakeTable::new(stake_table_addr, &provider);

    //     let mut rng = rand::thread_rng();
    //     let node = NodeInfoSol::rand(&mut rng);

    //     let new_nodes: Vec<NodeInfoSol> = vec![node];
    //     stake_table_contract
    //         .update(vec![], new_nodes)
    //         .send()
    //         .await?
    //         .watch()
    //         .await?;

    //     let block = l1_client.get_block(BlockId::latest()).await?.unwrap();
    //     let nodes = l1_client
    //         .get_stake_table(stake_table_addr, block.header.inner.number)
    //         .await?;

    //     let result = nodes.stake_table.0[0].clone();
    //     assert_eq!(result.stake_table_entry.stake_amount.to::<u64>(), 1);
    //     Ok(())
    // }

    /// A helper function to get the index of the current provider in the failover list.
    fn get_failover_index(provider: &L1Client) -> usize {
        let transport = &provider.transport;
        provider.transport.current_transport.read().generation % transport.urls.len()
    }

    async fn test_failover_update_task_helper(ws: bool) {
        setup_test();

        let anvil = Anvil::new().block_time(1).spawn();

        // Create an L1 client with fake providers, and check that the state is still updated after
        // it correctly fails over to the real providers.
        let client = L1ClientOptions {
            l1_polling_interval: Duration::from_secs(1),
            // Use a very long subscription timeout, so that we only succeed by triggering a
            // failover.
            subscription_timeout: Duration::from_secs(1000),
            l1_ws_provider: if ws {
                Some(vec![
                    "ws://notarealurl:1234".parse().unwrap(),
                    anvil.ws_endpoint_url(),
                ])
            } else {
                None
            },
            ..Default::default()
        }
        .connect(vec![
            "http://notarealurl:1234".parse().unwrap(),
            anvil.endpoint_url(),
        ])
        .expect("Failed to create L1 client");

        client.spawn_tasks().await;

        let initial_state = client.snapshot().await;
        tracing::info!(?initial_state, "initial state");

        // Check the state is updating.
        let mut retry = 0;
        let updated_state = loop {
            assert!(retry < 10, "state did not update in time");

            let updated_state = client.snapshot().await;
            if updated_state.head > initial_state.head {
                break updated_state;
            }
            tracing::info!(retry, "waiting for state update");
            sleep(Duration::from_secs(1)).await;
            retry += 1;
        };
        tracing::info!(?updated_state, "state updated");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_failover_update_task_ws() {
        test_failover_update_task_helper(true).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_failover_update_task_http() {
        test_failover_update_task_helper(false).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_failover_consecutive_failures() {
        setup_test();

        let anvil = Anvil::new().block_time(1).spawn();

        let l1_options = L1ClientOptions {
            l1_polling_interval: Duration::from_secs(1),
            l1_frequent_failure_tolerance: Duration::from_millis(0),
            l1_consecutive_failure_tolerance: 3,
            ..Default::default()
        };

        let provider = l1_options
            .connect(vec![
                "http://notarealurl:1234".parse().unwrap(),
                anvil.endpoint_url(),
            ])
            .expect("Failed to create L1 client");

        // Make just enough failed requests not to trigger a failover.
        for _ in 0..2 {
            provider.get_block_number().await.unwrap_err();
            assert!(get_failover_index(&provider) == 0);
        }

        // The final request triggers failover.
        provider.get_block_number().await.unwrap_err();
        assert!(get_failover_index(&provider) == 1);

        // Now requests succeed.
        provider.get_block_number().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_failover_frequent_failures() {
        setup_test();

        let anvil = Anvil::new().block_time(1).spawn();
        let provider = L1ClientOptions {
            l1_polling_interval: Duration::from_secs(1),
            l1_frequent_failure_tolerance: Duration::from_millis(100),
            ..Default::default()
        }
        .connect(vec![
            "http://notarealurl:1234".parse().unwrap(),
            anvil.endpoint_url(),
        ])
        .expect("Failed to create L1 client");

        // Two failed requests that are not within the tolerance window do not trigger a failover.
        provider.get_block_number().await.unwrap_err();
        sleep(Duration::from_secs(1)).await;
        provider.get_block_number().await.unwrap_err();

        // Check that we didn't fail over.
        assert!(get_failover_index(&provider) == 0);

        // Reset the window.
        sleep(Duration::from_secs(1)).await;

        // Two failed requests in a row trigger failover.
        provider.get_block_number().await.unwrap_err();
        provider.get_block_number().await.unwrap_err();
        provider.get_block_number().await.unwrap();
        assert!(get_failover_index(&provider) == 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_failover_revert() {
        setup_test();

        let anvil = Anvil::new().block_time(1).spawn();
        let provider = L1ClientOptions {
            l1_polling_interval: Duration::from_secs(1),
            l1_consecutive_failure_tolerance: 1,
            l1_failover_revert: Duration::from_secs(2),
            ..Default::default()
        }
        .connect(vec![
            "http://notarealurl:1234".parse().unwrap(),
            anvil.endpoint_url(),
        ])
        .expect("Failed to create L1 client");

        // The first request fails and triggers a failover.
        provider.get_block_number().await.unwrap_err();
        assert_eq!(get_failover_index(&provider), 1);

        // The next request succeeds from the other provider.
        provider.get_block_number().await.unwrap();

        // Eventually we revert back to the primary and requests fail again.
        sleep(Duration::from_millis(2100)).await;
        provider.get_block_number().await.unwrap_err();
    }

    // Checks that the L1 client initialized the state on startup even
    // if the L1 is not currently mining blocks. It's useful for testing that we
    // don't require an L1 that is continuously mining blocks.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_loop_initializes_l1_state() {
        setup_test();
        let anvil = Arc::new(Anvil::new().port(9988u16).spawn());
        let l1_client = new_l1_client(&anvil, true).await;

        for _try in 0..10 {
            let mut state = l1_client.state.lock().await;
            let has_snapshot = state.snapshot.finalized.is_some();
            let has_cache = state.finalized.get(&0).is_some();
            drop(state);
            if has_snapshot && has_cache {
                return;
            }
            sleep(Duration::from_millis(200)).await;
        }
        panic!("L1 state of L1Client not initialized");
    }
}
