//! An L1 event stream based on a standard JSON-RPC server.

use super::{
    BlockInput, ResettableStream, options::L1ClientOptions, switching_transport::SwitchingTransport,
};
use crate::types::common::{Address, Timestamp};
use crate::{Result, types::common::L1BlockId};
use alloy::{
    eips::BlockId,
    network::Ethereum,
    providers::{Provider, ProviderBuilder, RootProvider, WsConnect},
    rpc::{client::RpcClient, types::Header},
};
use futures::stream::{self, BoxStream, Stream, StreamExt};
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tide_disco::Url;
use tokio::time::sleep;

/// Builder for creating an RpcStream.
struct RpcStreamBuilder {
    /// Provider for making RPC calls
    provider: Arc<RootProvider<Ethereum>>,
    /// Transport for switching between HTTP providers
    transport: SwitchingTransport,
    options: L1ClientOptions,
}

/// An L1 event stream based on a standard JSON-RPC server.
pub struct RpcStream {
    /// block stream
    stream: BoxStream<'static, BlockInput>,
    /// Builder for recreating the stream
    builder: Arc<RpcStreamBuilder>,
}

impl std::fmt::Debug for RpcStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcStream")
            .field("stream", &"<stream>")
            .finish()
    }
}

impl RpcStreamBuilder {
    /// Create a new builder from L1 client options.
    fn new(options: L1ClientOptions) -> Result<Self> {
        let http_urls = options.http_providers.clone();
        let transport = SwitchingTransport::new(options.clone(), http_urls)?;
        let rpc_client = RpcClient::new(transport.clone(), false);
        let provider = Arc::new(RootProvider::new(rpc_client));

        Ok(Self {
            provider,
            transport,
            options,
        })
    }

    /// Build the RpcStream
    async fn build(self) -> RpcStream {
        let builder = Arc::new(self);
        let stream = builder.clone().stream_with_reconnect(None).await;
        RpcStream { stream, builder }
    }

    /// Create the stream wrapper with reconnection
    async fn stream_with_reconnect(
        self: Arc<Self>,
        last_block: Option<u64>,
    ) -> BoxStream<'static, BlockInput> {
        let stream = self.establish_stream(last_block).await;

        stream::unfold(
            (self.clone(), stream, last_block),
            |(builder, mut stream, mut last_block)| async move {
                loop {
                    match stream.next().await {
                        Some(item) => {
                            last_block = Some(item.block.number);
                            return Some((item, (builder, stream, last_block)));
                        }
                        None => {
                            sleep(builder.options.l1_retry_delay).await;
                            tracing::warn!("L1 block stream ended, reconnecting...");
                            stream = builder.establish_stream(last_block).await;
                            tracing::info!("Successfully reconnected to L1 block stream");
                        }
                    }
                }
            },
        )
        .boxed()
    }

    async fn create_ws_stream(
        &self,
        url: &Url,
        last_block: Option<u64>,
    ) -> Result<BoxStream<'static, BlockInput>> {
        let ws = ProviderBuilder::new()
            .connect_ws(WsConnect::new(url.clone()))
            .await
            .map_err(|err| {
                tracing::warn!("Failed to connect WebSockets provider: {err:#}");
                crate::Error::internal().context(format!("Failed to connect: {err}"))
            })?;

        let block_stream = ws.subscribe_blocks().await.map_err(|err| {
            tracing::warn!("Failed to subscribe to blocks: {err:#}");
            crate::Error::internal().context(format!("Failed to subscribe using ws: {err}"))
        })?;
        tracing::info!(%url, "Successfully connected to WebSocket provider and subscribed to blocks");

        let retry_delay = self.options.l1_retry_delay;
        let provider = self.provider.clone();

        let provider_for_fetch = provider.clone();
        let block_stream = block_stream.into_stream();

        Ok(stream::unfold(
            (block_stream, last_block, ws),
            move |(mut stream, mut last_block_number, ws)| {
                let provider = provider.clone();

                async move {
                    let head = stream.next().await?;
                    let blocks =
                        process_block_header(&provider, &mut last_block_number, head, retry_delay)
                            .await;
                    Some((stream::iter(blocks), (stream, last_block_number, ws)))
                }
            },
        )
        .flatten()
        .then(move |head| {
            let provider = provider_for_fetch.clone();
            async move {
                let finalized = fetch_finalized(&provider, retry_delay).await;
                block_to_input(head, finalized)
            }
        })
        .boxed())
    }

    async fn create_http_stream(
        &self,
        last_block: Option<u64>,
    ) -> Result<BoxStream<'static, BlockInput>> {
        let poller = self
            .provider
            .watch_blocks()
            .await
            .map_err(|err| {
                crate::Error::internal().context(format!("Failed to watch blocks: {err}"))
            })?
            .with_poll_interval(self.options.l1_polling_interval)
            .into_stream();

        let provider = self.provider.clone();
        let retry_delay = self.options.l1_retry_delay;
        let switch_notify = self.transport.switch_notify.clone();

        let provider_for_fetch = provider.clone();
        let poller_stream = poller.map(stream::iter).flatten();

        Ok(stream::unfold(
            (poller_stream, last_block),
            move |(mut stream, mut last_block_number)| {
                let provider = provider_for_fetch.clone();

                async move {
                    let hash = stream.next().await?;

                    let block = match provider.get_block(BlockId::hash(hash)).await {
                        Ok(Some(block)) => block,
                        Ok(None) => {
                            tracing::warn!(%hash, "HTTP stream: Block not available");
                            return Some((stream::iter(Vec::new()), (stream, last_block_number)));
                        }
                        Err(err) => {
                            tracing::warn!(%hash, "HTTP stream: Failed to fetch block: {err:#}");
                            return Some((stream::iter(Vec::new()), (stream, last_block_number)));
                        }
                    };

                    let blocks = process_block_header(
                        &provider,
                        &mut last_block_number,
                        block.header,
                        retry_delay,
                    )
                    .await;
                    Some((stream::iter(blocks), (stream, last_block_number)))
                }
            },
        )
        .flatten()
        .take_until(async move {
            switch_notify.notified().await;
            tracing::warn!("HTTP stream shutting down due to provider switch");
        })
        .then(move |head| {
            let provider = provider.clone();
            async move {
                let finalized = fetch_finalized(&provider, retry_delay).await;
                block_to_input(head, finalized)
            }
        })
        .boxed())
    }

    /// Establish a new block stream connection
    async fn establish_stream(&self, last_block: Option<u64>) -> BoxStream<'static, BlockInput> {
        // Try to establish connection with retries
        for i in 0.. {
            let res = match &self.options.l1_ws_provider {
                Some(urls) => {
                    let provider_index = i % urls.len();
                    let url = &urls[provider_index];
                    self.create_ws_stream(url, last_block).await
                }
                None => self.create_http_stream(last_block).await,
            };

            match res {
                Ok(stream) => {
                    tracing::info!(attempt = i, "Successfully established L1 block stream");
                    return stream;
                }
                Err(err) => {
                    tracing::warn!(
                        attempt = i,
                        "Failed to establish stream: {err}, retrying..."
                    );
                    sleep(self.options.l1_retry_delay).await;
                }
            }
        }

        unreachable!("Infinite loop")
    }
}

impl RpcStream {
    /// Construct a new [`RpcStream`].
    ///
    /// The stream will establish a connection to the given providers over HTTP and WebSockets,
    /// respectively. It is recommended to provide both, so that WebSockets can be used to listen
    /// for new L1 heads, and then HTTP can be used to download corresponding block data. This is
    /// the cheapest and most efficient way to use JSON-RPC.
    ///
    /// If multiple providers are given for either protocol or both, the system will rotate between
    /// them if and when one provider fails.
    pub async fn new(options: L1ClientOptions) -> Result<Self> {
        let builder = RpcStreamBuilder::new(options)?;
        Ok(builder.build().await)
    }

    /// Get the Espresso stake table genesis block.
    pub async fn genesis(&self, _stake_table: Address) -> Result<(L1BlockId, Timestamp)> {
        todo!()
    }
}

impl Stream for RpcStream {
    type Item = BlockInput;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.stream.as_mut().poll_next(cx)
    }
}

impl ResettableStream for RpcStream {
    async fn reset(&mut self, block: u64) {
        tracing::info!("Resetting RpcStream to block {block}");

        // Recreate the stream to discard any buffered blocks from the flatten().
        // The missing fetch logic returns a vector which gets flattened into individual blocks.
        // Without recreating the stream, next() would continue with buffered blocks
        // instead of triggering the missing fetch from the reset point.
        self.stream = self
            .builder
            .clone()
            .stream_with_reconnect(Some(block))
            .await;

        tracing::warn!(
            "Reset RpcStream to block {block} (will resume from block {})",
            block + 1
        );
    }
}

/// Fetch finalized block with retry logic.
async fn fetch_finalized(
    provider: &Arc<RootProvider<Ethereum>>,
    retry_delay: Duration,
) -> L1BlockId {
    loop {
        match provider.get_block(BlockId::finalized()).await {
            Ok(Some(block)) => {
                return L1BlockId {
                    number: block.header.number,
                    hash: block.header.hash,
                    parent: block.header.parent_hash,
                };
            }
            Ok(None) => {
                tracing::warn!("finalized block is None, will retry");
            }
            Err(err) => {
                tracing::warn!("Failed to fetch finalized block: {err}, will retry");
            }
        }
        sleep(retry_delay).await;
    }
}

/// Process a new block header, handling missing blocks
/// and updating last_block_number.
/// Returns a vector of headers in order, including any missing blocks that were fetched.
async fn process_block_header(
    provider: &Arc<RootProvider<Ethereum>>,
    last_block_number: &mut Option<u64>,
    new_header: Header,
    retry_delay: Duration,
) -> Vec<Header> {
    let new_block_number = new_header.number;
    let prev_block_number = *last_block_number;

    let mut new_blocks = Vec::new();

    if let Some(prev) = prev_block_number {
        if new_block_number <= prev {
            tracing::info!(
                "Skipping block {new_block_number} as it's not newer than previous block {prev}"
            );
            return Vec::new();
        }

        if new_block_number != prev + 1 {
            let missing_blocks =
                fetch_missing_blocks(provider, prev, new_block_number, retry_delay).await;

            tracing::info!(
                fetched_count = missing_blocks.len(),
                "Successfully fetched missing blocks"
            );
            new_blocks.extend(missing_blocks);
        }
    }

    new_blocks.push(new_header);
    *last_block_number = Some(new_block_number);
    new_blocks
}

/// Fetch missing blocks with retry logic.
async fn fetch_missing_blocks(
    provider: &Arc<RootProvider<Ethereum>>,
    prev_block: u64,
    new_block: u64,
    retry_delay: Duration,
) -> Vec<Header> {
    let mut headers = Vec::new();

    if new_block > prev_block + 1 {
        tracing::warn!(
            "Fetching missing blocks from {} to {}",
            prev_block + 1,
            new_block - 1
        );

        for block_num in (prev_block + 1)..new_block {
            loop {
                match provider.get_block(BlockId::number(block_num)).await {
                    Ok(Some(block)) => {
                        headers.push(block.header);
                        break;
                    }
                    Ok(None) => {
                        tracing::warn!("Missing block {block_num} not found, retrying...");
                        sleep(retry_delay).await;
                    }
                    Err(err) => {
                        tracing::warn!(
                            "Failed to fetch missing block {block_num}: {err}, retrying..."
                        );
                        sleep(retry_delay).await;
                    }
                }
            }
        }
    }

    headers
}

/// Convert header and finalized block to BlockInput.
fn block_to_input(head: Header, finalized: L1BlockId) -> BlockInput {
    let block = L1BlockId {
        number: head.number,
        hash: head.hash,
        parent: head.parent_hash,
    };

    BlockInput {
        block,
        finalized,
        timestamp: head.timestamp,
        events: vec![],
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use alloy::node_bindings::Anvil;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_rpc_stream_with_anvil() {
        let anvil = Anvil::new().block_time(1).spawn();
        let url = anvil.endpoint().parse::<Url>().unwrap();

        let options = L1ClientOptions {
            http_providers: vec![url],
            ..Default::default()
        };

        let mut stream = RpcStream::new(options).await.unwrap();

        let mut last_block_number = 0;

        for i in 1..=10 {
            println!("Waiting for block {i}");
            let block_input = stream.next().await.expect("Stream ended unexpectedly");

            assert!(block_input.block.number == last_block_number + 1);
            last_block_number = block_input.block.number;
        }
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_rpc_stream_websocket() {
        let anvil = Anvil::new().block_time(1).spawn();
        let http_url = anvil.endpoint().parse::<Url>().unwrap();
        let ws_url = anvil.ws_endpoint().parse::<Url>().unwrap();

        let options = L1ClientOptions {
            http_providers: vec![http_url],
            l1_ws_provider: Some(vec![ws_url]),
            ..Default::default()
        };

        let mut stream = RpcStream::new(options).await.unwrap();

        println!("Stream created successfully");

        let mut last_block_number = 0;

        for i in 1..=10 {
            println!("Waiting for block {i}");
            let block_input = stream.next().await.expect("Stream ended unexpectedly");

            assert!(block_input.block.number == last_block_number + 1);
            last_block_number = block_input.block.number;
            println!("Received block {i}");
        }
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_l1_stream_reconnect() {
        // Start two Anvil instances
        let anvil1 = Anvil::new().block_time(1).spawn();
        let http_url1 = anvil1.endpoint().parse::<Url>().unwrap();

        let anvil2 = Anvil::new().block_time(1).spawn();
        let http_url2 = anvil2.endpoint().parse::<Url>().unwrap();

        let options = L1ClientOptions {
            http_providers: vec![http_url1.clone(), http_url2.clone()],
            l1_retry_delay: Duration::from_millis(100),
            ..Default::default()
        };

        let mut stream = RpcStream::new(options).await.unwrap();

        for _i in 1..=3 {
            let block_input = stream.next().await.unwrap();
            assert!(block_input.block.number > 0);
        }

        drop(anvil1);

        for _i in 1..=5 {
            let block_input = stream.next().await.unwrap();

            assert!(block_input.block.number > 0);
        }
    }

    /// Helper function to test reset functionality for RpcStream
    async fn test_reset_logic(stream: &mut RpcStream) {
        let mut last_block_number = 0;
        for i in 1..=10 {
            println!("Waiting for block {i}");
            let block_input = stream.next().await.expect("Stream ended unexpectedly");
            assert_eq!(block_input.block.number, last_block_number + 1);
            last_block_number = block_input.block.number;
        }

        println!("Reached block 10, resetting to block 5");
        stream.reset(5).await;

        let block_input = stream.next().await.expect("Stream ended unexpectedly");
        println!("After reset, received block: {}", block_input.block.number);
        assert_eq!(
            block_input.block.number, 6,
            "Expected block 6 after reset to block 5"
        );

        for i in 7..=15 {
            let block_input = stream.next().await.expect("Stream ended unexpectedly");
            assert_eq!(block_input.block.number, i, "Expected block {i}");
        }

        println!("Reset to genesis");
        stream.reset(0).await;
        let block = stream.next().await.expect("Stream ended unexpectedly");
        assert_eq!(
            block.block.number, 1,
            "Expected block 1 after reset to genesis"
        );

        println!("Reset to block 1");
        stream.reset(1).await;
        let block = stream.next().await.expect("Stream ended unexpectedly");
        assert_eq!(
            block.block.number, 2,
            "Expected block 2 after reset to block 1"
        );

        println!("Reset to current block");
        for _ in 3..=10 {
            stream.next().await.expect("Stream ended unexpectedly");
        }
        stream.reset(10).await;
        let block = stream.next().await.expect("Stream ended unexpectedly");
        assert_eq!(
            block.block.number, 11,
            "Expected block 11 after reset to current"
        );

        stream.reset(5).await;
        let block = stream.next().await.expect("Stream ended unexpectedly");
        assert_eq!(block.block.number, 6);

        stream.reset(3).await;
        let block = stream.next().await.expect("Stream ended unexpectedly");
        assert_eq!(block.block.number, 4);

        stream.reset(15).await;
        let block = stream.next().await.expect("Stream ended unexpectedly");
        assert_eq!(block.block.number, 16);

        for _ in 17..=30 {
            stream.next().await.expect("Stream ended unexpectedly");
        }
        stream.reset(10).await;

        for expected in 11..=20 {
            let block = stream.next().await.expect("Stream ended unexpectedly");
            assert_eq!(block.block.number, expected);
        }

        println!("Reset to future block");
        stream.reset(50).await;
        let block = stream.next().await.expect("Stream ended unexpectedly");
        assert_eq!(
            block.block.number, 51,
            "Expected block 51 after reset to block 50"
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_rpc_stream_reset_http() {
        let anvil = Anvil::new()
            .block_time(2)
            .args(["--slots-in-an-epoch", "0"])
            .spawn();
        let url = anvil.endpoint().parse::<Url>().unwrap();

        let options = L1ClientOptions {
            http_providers: vec![url],
            ..Default::default()
        };

        let mut stream = RpcStream::new(options).await.unwrap();
        println!("Testing HTTP stream reset");

        test_reset_logic(&mut stream).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_rpc_stream_reset_websocket() {
        let anvil = Anvil::new()
            .block_time(2)
            .args(["--slots-in-an-epoch", "0"])
            .spawn();
        let http_url = anvil.endpoint().parse::<Url>().unwrap();
        let ws_url = anvil.ws_endpoint().parse::<Url>().unwrap();

        let options = L1ClientOptions {
            http_providers: vec![http_url],
            l1_ws_provider: Some(vec![ws_url]),
            ..Default::default()
        };

        let mut stream = RpcStream::new(options).await.unwrap();
        println!("Testing WebSocket stream reset");

        test_reset_logic(&mut stream).await;
    }
}
