//! An L1 event stream based on a standard JSON-RPC server.

use super::{
    BlockInput, L1Event, ResettableStream, options::L1ClientOptions,
    switching_transport::SwitchingTransport,
};
use crate::input::l1::provider::get_events;
use crate::types::common::Address;
use crate::{Error, Result, types::common::L1BlockId};
use alloy::{
    eips::BlockId,
    network::Ethereum,
    providers::{Provider, ProviderBuilder, RootProvider, WsConnect},
    rpc::types::{Filter, Header},
};
use futures::{
    future,
    stream::{self, BoxStream, Stream, StreamExt},
};
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
    /// Create a new builder from L1 client options and contract addresses.
    fn new(options: L1ClientOptions) -> Result<Self> {
        let (provider, transport) = options.provider()?;
        Ok(Self {
            provider: Arc::new(provider),
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
                Error::internal().context(format!("Failed to connect: {err}"))
            })?;

        let block_stream = ws.subscribe_blocks().await.map_err(|err| {
            tracing::warn!("Failed to subscribe to blocks: {err:#}");
            Error::internal().context(format!("Failed to subscribe using ws: {err}"))
        })?;
        tracing::info!(%url, "Successfully connected to WebSocket provider and subscribed to blocks");

        let retry_delay = self.options.l1_retry_delay;
        let provider = self.provider.clone();
        let stake_table_address = self.options.stake_table_address;
        let reward_contract_address = self.options.reward_contract_address;

        let provider_for_fetch = provider.clone();
        let block_stream = block_stream.into_stream();

        Ok(stream::unfold(
            (block_stream, last_block, ws),
            move |(mut stream, mut last_block_number, ws)| {
                let provider = provider.clone();

                async move {
                    let head = stream.next().await?;
                    let blocks =
                        process_block_header(provider, &mut last_block_number, head, retry_delay);
                    Some((blocks, (stream, last_block_number, ws)))
                }
            },
        )
        .flatten()
        .then(move |head| {
            let provider = provider_for_fetch.clone();
            async move {
                create_block_input(
                    head,
                    &provider,
                    retry_delay,
                    stake_table_address,
                    reward_contract_address,
                )
                .await
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
            .map_err(|err| Error::internal().context(format!("Failed to watch blocks: {err}")))?
            .with_poll_interval(self.options.l1_polling_interval)
            .into_stream();

        let provider = self.provider.clone();
        let retry_delay = self.options.l1_retry_delay;
        let switch_notify = self.transport.switch_notify.clone();
        let stake_table_address = self.options.stake_table_address;
        let reward_contract_address = self.options.reward_contract_address;

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
                            return Some((stream::empty().boxed(), (stream, last_block_number)));
                        }
                        Err(err) => {
                            tracing::warn!(%hash, "HTTP stream: Failed to fetch block: {err:#}");
                            return Some((stream::empty().boxed(), (stream, last_block_number)));
                        }
                    };

                    let blocks = process_block_header(
                        provider,
                        &mut last_block_number,
                        block.header,
                        retry_delay,
                    );
                    Some((blocks, (stream, last_block_number)))
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
                create_block_input(
                    head,
                    &provider,
                    retry_delay,
                    stake_table_address,
                    reward_contract_address,
                )
                .await
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
    head: &Header,
    retry_delay: Duration,
) -> L1BlockId {
    loop {
        match provider.get_block(BlockId::finalized()).await {
            Ok(Some(block)) if block.number() >= head.number => {
                // The finalized block should always trail the head. This might not be the case if
                // we are catching up, in which case `head` might actually be an old block. In this
                // case just use the block before head as the "latest" finalized block; since it is
                // older than the true finalized block, it must be finalized itself.
                tracing::info!(
                    ?head,
                    ?block,
                    "head is older than finalized block, returning head's parent"
                );
                match provider.get_block(head.parent_hash.into()).await {
                    Ok(Some(block)) => {
                        return L1BlockId {
                            number: block.header.number,
                            hash: block.header.hash,
                            parent: block.header.parent_hash,
                        };
                    }
                    Ok(None) => {
                        tracing::warn!("head's parent is None, will retry");
                    }
                    Err(err) => {
                        tracing::warn!("failed to fetch head's parent: {err:#}");
                    }
                }
            }
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
fn process_block_header(
    provider: Arc<RootProvider<Ethereum>>,
    last_block_number: &mut Option<u64>,
    new_header: Header,
    retry_delay: Duration,
) -> BoxStream<'static, Header> {
    let new_block_number = new_header.number;
    let prev_block_number = *last_block_number;

    let mut new_blocks = stream::empty().boxed();
    if let Some(prev) = prev_block_number {
        if new_block_number <= prev {
            tracing::info!(
                "Skipping block {new_block_number} as it's not newer than previous block {prev}"
            );
            return new_blocks;
        }

        if new_block_number != prev + 1 {
            new_blocks =
                fetch_missing_blocks(provider, prev, new_block_number, retry_delay).boxed();
        }
    }

    new_blocks = new_blocks
        .chain(stream::once(future::ready(new_header)))
        .boxed();
    *last_block_number = Some(new_block_number);
    new_blocks
}

/// Fetch missing blocks with retry logic.
fn fetch_missing_blocks(
    provider: Arc<RootProvider<Ethereum>>,
    prev_block: u64,
    new_block: u64,
    retry_delay: Duration,
) -> impl Stream<Item = Header> {
    tracing::warn!(
        "Fetching missing blocks from {} to {}",
        prev_block + 1,
        new_block - 1
    );
    stream::iter(prev_block + 1..new_block).then(move |block_num| {
        let provider = provider.clone();
        async move {
            loop {
                match provider.get_block(BlockId::number(block_num)).await {
                    Ok(Some(block)) => {
                        break block.header;
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
    })
}

/// Try to fetch and decode events for a given header.
/// Returns Ok(events, header) on success, Err on failure.
async fn try_get_events_from_header(
    provider: &RootProvider<Ethereum>,
    header: &Header,
    stake_table_address: Address,
    reward_contract_address: Address,
) -> Result<(Vec<L1Event>, Header)> {
    // Check bloom filter with header
    let has_logs = header.logs_bloom.contains_raw_log(stake_table_address, &[])
        || header
            .logs_bloom
            .contains_raw_log(reward_contract_address, &[]);

    if !has_logs {
        return Ok((Vec::new(), header.clone()));
    }

    // try fetching the events with the block hash
    let events = get_events(
        provider,
        Filter::new().at_block_hash(header.hash),
        stake_table_address,
        reward_contract_address,
    )
    .await?
    .into_iter()
    .map(|(_, _, event)| event)
    .collect();
    Ok((events, header.clone()))
}

/// Fetch events from L1 for a specific block with retry logic.
/// Returns the events and the header which may differ from input header due to reorg.
async fn fetch_block_events(
    provider: &RootProvider<Ethereum>,
    header: &Header,
    stake_table_address: Address,
    reward_contract_address: Address,
    retry_delay: Duration,
) -> (Vec<L1Event>, Header) {
    const MAX_HASH_RETRIES: u32 = 3;

    // Try fetching the events with the block hash
    for attempt in 1..=MAX_HASH_RETRIES {
        match try_get_events_from_header(
            provider,
            header,
            stake_table_address,
            reward_contract_address,
        )
        .await
        {
            Ok((events, header)) => return (events, header),
            Err(err) => {
                if attempt == MAX_HASH_RETRIES {
                    tracing::warn!(
                        block = header.number,
                        hash = ?header.hash,
                        "Failed to fetch logs by hash after {MAX_HASH_RETRIES} attempts: {err}, falling back to fetch by block number"
                    );
                } else {
                    sleep(retry_delay).await;
                }
            }
        }
    }

    // Fall back to fetching by block number
    let block_number = header.number;
    loop {
        // Refetch the header on each retry, even if get_logs() call failed,
        // in case the RPC is still returning the old block
        let new_header = match provider.get_block(BlockId::number(block_number)).await {
            Ok(Some(block)) => block.header,
            Ok(None) => {
                tracing::warn!("Block {block_number} not found, retrying...");
                sleep(retry_delay).await;
                continue;
            }
            Err(err) => {
                tracing::warn!("Failed to fetch block {block_number}: {err}, retrying...");
                sleep(retry_delay).await;
                continue;
            }
        };

        match try_get_events_from_header(
            provider,
            &new_header,
            stake_table_address,
            reward_contract_address,
        )
        .await
        {
            Ok((events, header)) => return (events, header),
            Err(err) => {
                tracing::warn!(
                    block = block_number,
                    hash = ?new_header.hash,
                    "Failed to fetch logs: {err}, retrying..."
                );
                sleep(retry_delay).await;
            }
        }
    }
}

/// Fetch all necessary data and construct a BlockInput.
async fn create_block_input(
    head: Header,
    provider: &Arc<RootProvider<Ethereum>>,
    retry_delay: Duration,
    stake_table_address: Address,
    reward_contract_address: Address,
) -> BlockInput {
    // Fetch events for the block. The returned header may differ from the input header
    // if a reorg occurred, we get the new header for the same block number
    // with potentially different hash
    let (events, header) = fetch_block_events(
        provider,
        &head,
        stake_table_address,
        reward_contract_address,
        retry_delay,
    )
    .await;

    let finalized = fetch_finalized(provider, &header, retry_delay).await;

    BlockInput {
        block: L1BlockId {
            number: header.number,
            hash: header.hash,
            parent: header.parent_hash,
        },
        finalized,
        timestamp: header.timestamp,
        events,
    }
}
#[cfg(test)]
mod tests {
    use crate::input::l1;
    use crate::input::l1::testing::ContractDeployment;

    use crate::input::l1::testing::MemoryStorage;
    use crate::input::l1::testing::NoCatchup;
    use crate::input::l1::{Snapshot, State};
    use crate::types::common::Ratio;
    use async_lock::RwLock;
    use tokio::time::sleep;

    use super::*;
    use alloy::providers::ext::AnvilApi;
    use alloy::sol_types::SolEvent;
    use alloy::{node_bindings::Anvil, providers::ProviderBuilder};
    use committable::Committable;
    use espresso_types::v0::StakeTableState;
    use futures::StreamExt;
    use hotshot_contract_adapter::sol_types::StakeTableV2::{
        CommissionUpdated, ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated, Undelegated,
        ValidatorExit, ValidatorRegistered, ValidatorRegisteredV2,
    };
    use staking_cli::demo::DelegationConfig;
    use std::time::Duration;

    #[tokio::test]
    async fn test_rpc_stream_with_anvil() {
        let anvil = Anvil::new().block_time(1).spawn();
        let url = anvil.endpoint().parse::<Url>().unwrap();

        let options = L1ClientOptions {
            http_providers: vec![url],
            stake_table_address: Address::ZERO,
            reward_contract_address: Address::ZERO,
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
            stake_table_address: Address::ZERO,
            reward_contract_address: Address::ZERO,
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
            stake_table_address: Address::ZERO,
            reward_contract_address: Address::ZERO,
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
            stake_table_address: Address::ZERO,
            reward_contract_address: Address::ZERO,
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
            stake_table_address: Address::ZERO,
            reward_contract_address: Address::ZERO,
            ..Default::default()
        };

        let mut stream = RpcStream::new(options).await.unwrap();
        println!("Testing WebSocket stream reset");

        test_reset_logic(&mut stream).await;
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_deploy_contracts_helper() {
        let anvil = Anvil::new().args(["--slots-in-an-epoch", "0"]).spawn();
        let rpc_url: Url = anvil.endpoint().parse().unwrap();

        let deployment = ContractDeployment::deploy(rpc_url.clone())
            .await
            .expect("Failed to deploy contracts");

        let stake_table = deployment.stake_table_addr;
        let token = deployment.token_addr;
        let reward_claim = deployment.reward_claim_addr;

        assert_ne!(
            stake_table,
            Address::ZERO,
            "Stake table address should not be zero"
        );
        assert_ne!(token, Address::ZERO, "Token address should not be zero");
        assert_ne!(
            reward_claim,
            Address::ZERO,
            "Reward claim address should not be zero"
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_events_basic() {
        let anvil = Anvil::new().args(["--slots-in-an-epoch", "0"]).spawn();
        let rpc_url: Url = anvil.endpoint().parse().unwrap();

        let deployment = ContractDeployment::deploy(rpc_url).await.unwrap();

        let validators = ContractDeployment::create_test_validators(2);

        let options = L1ClientOptions {
            http_providers: vec![deployment.rpc_url.clone()],
            stake_table_address: deployment.stake_table_addr,
            reward_contract_address: deployment.reward_claim_addr,
            ..Default::default()
        };

        let mut stream = RpcStream::new(options).await.unwrap();

        let _receipts = deployment
            .register_validators(validators, DelegationConfig::VariableAmounts)
            .await
            .expect("Failed to register validators");

        let mut found_stake_event = false;
        'outer: for _ in 0..20 {
            let block_input = stream.next().await.expect("Stream ended unexpectedly");

            for event in &block_input.events {
                match event {
                    L1Event::StakeTable(_) => {
                        found_stake_event = true;
                        break 'outer;
                    }
                    L1Event::Reward(_) => {}
                }
            }
        }

        assert!(
            found_stake_event,
            "Should have found at least one stake event from validator registration"
        );
    }

    /// Test that verifies events correctness
    /// - Spawns a background task that performs random stake table operations (register, delegate, etc.)
    /// - Processes 150 blocks through the RPC stream, applying events to a StakeTableState
    /// - Stops the background task and fetches all events using a range query directly from the endpoint
    /// - Applies the queried events to a fresh StakeTableState
    /// - Compares the state commit of both states they must match
    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_events() {
        let anvil = Anvil::new()
            .block_time(1)
            .args(["--slots-in-an-epoch", "0"])
            .spawn();
        let rpc_url: Url = anvil.endpoint().parse().unwrap();

        let deployment = ContractDeployment::deploy(rpc_url.clone()).await.unwrap();

        let options = L1ClientOptions {
            http_providers: vec![rpc_url.clone()],
            stake_table_address: deployment.stake_table_addr,
            reward_contract_address: deployment.reward_claim_addr,
            ..Default::default()
        };

        // Create the rpc stream BEFORE starting the background task.
        // because we are not handling the snapshots here so  the stream starts processing blocks
        // and then events begin appearing in those blocks.
        let mut stream = RpcStream::new(options.clone()).await.unwrap();

        let background_task = deployment.spawn_task();

        let mut stake_table_state_from_stream = StakeTableState::new();
        let start_block = 1u64;
        let mut end_block = start_block;

        for _i in 1..=150 {
            let block_input = stream.next().await.expect("Stream ended unexpectedly");
            end_block = block_input.block.number;

            println!("Block {}", block_input.block.number);

            for event in &block_input.events {
                match event {
                    L1Event::StakeTable(stake_event) => {
                        println!("Stream event: {stake_event:?}");
                        if let Ok(event) = (**stake_event).clone().try_into() {
                            match stake_table_state_from_stream.apply_event(event) {
                                Ok(Ok(())) => {}
                                Ok(Err(e)) => {
                                    println!("Expected error: {e:?}");
                                }
                                Err(err) => {
                                    panic!("Critical stake table error: {err:?}");
                                }
                            }
                        }
                    }
                    L1Event::Reward(_) => {}
                }
            }
        }

        drop(background_task);

        println!(
            "Fetching events from blocks {start_block} to {end_block} directly from rpc provider"
        );
        let provider = ProviderBuilder::new().connect_http(rpc_url.clone());
        let stake_table_state_from_provider =
            fetch_stake_table_state_from_provider(&provider, &deployment, start_block, end_block)
                .await;

        let stream_commit = stake_table_state_from_stream.commit();
        let provider_commit = stake_table_state_from_provider.commit();

        assert_eq!(stream_commit, provider_commit, "commit mismatch",);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_stake_table_events_with_reset_from_genesis() {
        let anvil = Anvil::new()
            .block_time(1)
            .args(["--slots-in-an-epoch", "0"])
            .spawn();
        let rpc_url: Url = anvil.endpoint().parse().unwrap();

        let deployment = ContractDeployment::deploy(rpc_url.clone()).await.unwrap();
        let background_task = deployment.spawn_task();

        println!("Waiting for stake table events");
        sleep(Duration::from_secs(150)).await;
        drop(background_task);

        let provider = ProviderBuilder::new().connect_http(rpc_url.clone());
        provider.anvil_mine(Some(500), None).await.unwrap();

        let options = L1ClientOptions {
            http_providers: vec![rpc_url.clone()],
            stake_table_address: deployment.stake_table_addr,
            reward_contract_address: deployment.reward_claim_addr,
            ..Default::default()
        };
        let mut stream = RpcStream::new(options.clone()).await.unwrap();

        // Fetch genesis using load_genesis
        let genesis =
            crate::input::l1::provider::load_genesis(&provider, deployment.stake_table_addr)
                .await
                .unwrap();

        println!(
            "Fetched genesis block: number={}, hash={:?}",
            genesis.number(),
            genesis.hash()
        );

        stream.reset(genesis.number()).await;
        println!("Reset stream to genesis, now processing all blocks");

        // Process all blocks from genesis through the stream
        let mut stake_table_state_from_stream = StakeTableState::new();
        let start_block = genesis.number() + 1;
        let mut end_block = start_block;

        for _i in 1..=650 {
            let block_input = stream.next().await.expect("Stream ended unexpectedly");
            end_block = block_input.block.number;

            for event in &block_input.events {
                match event {
                    L1Event::StakeTable(event) => {
                        if let Ok(stake_table_event) = (**event).clone().try_into() {
                            let result =
                                stake_table_state_from_stream.apply_event(stake_table_event);
                            match result {
                                Ok(Ok(())) => {}
                                Ok(Err(e)) => {
                                    println!("Expected error: {e:?}");
                                }
                                Err(err) => {
                                    panic!("Critical stake table error: {err:?}");
                                }
                            }
                        } else {
                            tracing::info!(?event, "skipping irrelevant contract event");
                        }
                    }
                    L1Event::Reward(_) => {}
                }
            }
        }

        println!(
            "Fetching events from blocks {start_block} to {end_block} directly from rpc provider"
        );
        let stake_table_state_from_provider =
            fetch_stake_table_state_from_provider(&provider, &deployment, start_block, end_block)
                .await;

        // Compare the commits
        let stream_commit = stake_table_state_from_stream.commit();
        let provider_commit = stake_table_state_from_provider.commit();

        assert_eq!(
            stream_commit, provider_commit,
            "Commit mismatch after reset from genesis"
        );
    }

    async fn fetch_stake_table_state_from_provider(
        provider: &impl Provider,
        deployment: &ContractDeployment,
        start_block: u64,
        end_block: u64,
    ) -> StakeTableState {
        let filter = Filter::new()
            .events([
                ValidatorRegistered::SIGNATURE,
                ValidatorRegisteredV2::SIGNATURE,
                ValidatorExit::SIGNATURE,
                Delegated::SIGNATURE,
                Undelegated::SIGNATURE,
                ConsensusKeysUpdated::SIGNATURE,
                ConsensusKeysUpdatedV2::SIGNATURE,
                CommissionUpdated::SIGNATURE,
            ])
            .from_block(start_block)
            .to_block(end_block);
        let events = get_events(
            &provider,
            filter,
            deployment.stake_table_addr,
            deployment.reward_claim_addr,
        )
        .await
        .unwrap();

        // Apply events from range query to a fresh state
        let mut state = StakeTableState::new();

        for (_, _, event) in events {
            let L1Event::StakeTable(st_event) = event else {
                panic!("got unexpected non-stake table event");
            };
            let event = st_event.as_ref().clone().try_into().unwrap();
            let result = state.apply_event(event);
            match result {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    println!("Expected error: {err:?}");
                }
                Err(err) => {
                    panic!("Critical err: {err:?}");
                }
            }
        }

        state
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_state_snapshot_and_events_from_provider() {
        let anvil = Anvil::new()
            .block_time(1)
            .args(["--slots-in-an-epoch", "0"])
            .spawn();
        let rpc_url: Url = anvil.endpoint().parse().unwrap();

        let deployment = ContractDeployment::deploy(rpc_url.clone()).await.unwrap();

        let options = L1ClientOptions {
            http_providers: vec![rpc_url.clone()],
            stake_table_address: deployment.stake_table_addr,
            reward_contract_address: deployment.reward_claim_addr,
            ..Default::default()
        };

        let stream = RpcStream::new(options.clone()).await.unwrap();

        let provider = ProviderBuilder::new().connect_http(rpc_url.clone());
        let genesis_block = l1::provider::load_genesis(&provider, deployment.stake_table_addr)
            .await
            .unwrap();
        let genesis_snapshot = Snapshot::empty(genesis_block);

        let storage = MemoryStorage::default();
        let state = Arc::new(RwLock::new(
            State::new(storage.clone(), genesis_snapshot.clone(), &NoCatchup)
                .await
                .unwrap(),
        ));

        let background_task = deployment.spawn_task_with_interval(Duration::from_millis(100));

        // Subscribe to the stream in a background task
        let state_clone = state.clone();
        let subscription_task =
            tokio::spawn(async move { State::subscribe(state_clone, stream).await });

        let target_blocks = 400;
        let genesis_block_number = genesis_block.number();

        loop {
            sleep(Duration::from_secs(1)).await;
            let current_state = state.read().await;
            let current_block = current_state.latest_l1_block().number;
            let blocks_processed = current_block - genesis_block_number;

            println!("Processed {blocks_processed} / {target_blocks} blocks");

            if blocks_processed >= target_blocks {
                println!("Reached target block: {current_block}");
                break;
            }
        }

        // Stop background operations
        drop(background_task);
        subscription_task.abort();
        let _ = subscription_task.await;

        let subscription_latest_block = state.read().await.latest_l1_block();
        println!(
            "Subscription processed up to block: {}",
            subscription_latest_block.number
        );

        let state_from_subscription = state.read().await;
        let subscription_node_set = state_from_subscription
            .full_node_set(subscription_latest_block.hash)
            .unwrap()
            .0
            .clone();

        let start_block = genesis_block_number + 1;
        let end_block = subscription_latest_block.number;

        let stake_table_from_l1_query =
            fetch_stake_table_state_from_provider(&provider, &deployment, start_block, end_block)
                .await;

        // Compare the states
        // Both states should have the same set of validators
        assert_eq!(
            subscription_node_set.len(),
            stake_table_from_l1_query.validators().len(),
            "Number of validators mismatch between subscription and L1 query"
        );

        // Verify each node exists in both states and compare their values
        for node in subscription_node_set.values() {
            let l1_validator = stake_table_from_l1_query.validators().get(&node.address);

            let l1_validator = l1_validator.unwrap();

            // Compare all fields
            assert_eq!(
                node.address, l1_validator.account,
                "Address mismatch for validator {}",
                node.address
            );

            assert_eq!(
                node.stake, l1_validator.stake,
                "Stake mismatch for validator {}: subscription={}, L1={}",
                node.address, node.stake, l1_validator.stake
            );

            assert_eq!(
                node.staking_key.to_string(),
                l1_validator.stake_table_key.to_string(),
                "Staking key mismatch for validator {}: subscription={}, L1={}",
                node.address,
                node.staking_key,
                l1_validator.stake_table_key
            );

            let l1_commission_ratio = Ratio::new(l1_validator.commission as usize, 10_000);
            assert_eq!(
                node.commission, l1_commission_ratio,
                "Commission mismatch for validator {}: subscription={:?}, L1={:?}",
                node.address, node.commission, l1_commission_ratio
            );

            // Verify wallets
            for (delegator_address, delegated_amount) in &l1_validator.delegators {
                match state_from_subscription
                    .wallet(*delegator_address, subscription_latest_block.hash)
                {
                    Ok((wallet, _)) => {
                        let wallet_delegation = wallet.nodes.get(&node.address).expect("not found");

                        assert_eq!(
                            wallet_delegation.amount, *delegated_amount,
                            "Delegation amount mismatch: delegator {delegator_address} to validator {}",
                            node.address,
                        );
                    }
                    Err(e) => {
                        panic!(
                            "Delegator {delegator_address} wallet not found in subscription state but exists in L1 validator delegators: {e}"
                        );
                    }
                }
            }
        }
    }
}
