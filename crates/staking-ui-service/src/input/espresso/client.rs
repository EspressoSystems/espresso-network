use std::time::Duration;

use bitvec::vec::BitVec;
use clap::Parser;
use espresso_types::{
    Leaf2, SeqTypes, ValidatorMap, config::PublicNetworkConfig, parse_duration, v0_3::RewardAmount,
    v0_4::RewardAccountV2,
};
use futures::{Stream, StreamExt, TryFutureExt, TryStreamExt, stream};
use hotshot_query_service::{availability::LeafQueryData, types::HeightIndexed};
use hotshot_types::utils::epoch_from_block_number;
use surf_disco::{Client, Url};
use tokio::time::{sleep, timeout};
use tracing::instrument;
use vbs::version::StaticVersion;

use crate::{
    Error, Result,
    error::ensure,
    input::espresso::EspressoClient,
    types::common::{Address, ESPTokenAmount},
};

/// The version used for serialized messages from the query service.
type FormatVersion = StaticVersion<0, 1>;

/// Configuration for a HotShot query service client.
#[derive(Debug, Parser)]
pub struct QueryServiceOptions {
    /// Reconnect WebSocket streams after this long without a new message.
    #[clap(
        long,
        env = "ESPRESSO_STAKING_SERVICE_STREAM_TIMEOUT",
        value_parser = parse_duration,
        default_value = "1m",
    )]
    pub stream_timeout: Duration,

    /// URL for an Espresso query service.
    #[clap(long = "espresso-url", env = "ESPRESSO_STAKING_SERVICE_ESPRESSO_URL")]
    pub url: Url,
}

impl QueryServiceOptions {
    /// Options to connect to the given URL, with default values for optional parameters.
    pub fn new(url: Url) -> Self {
        QueryServiceOptions::parse_from(["--", "--espresso-url", url.as_str()])
    }
}

#[derive(Clone, Debug)]
pub struct QueryServiceClient {
    inner: Client<hotshot_query_service::Error, FormatVersion>,
    epoch_start_block: u64,
    epoch_height: u64,
    stream_timeout: Duration,
}

impl QueryServiceClient {
    /// Connect to a query service at the given base URL.
    pub async fn new(opt: QueryServiceOptions) -> Result<Self> {
        let inner = Client::new(opt.url);

        // Get the epoch height. We need this for multiple endpoints, and it never changes, so we
        // fetch it now and cache it.
        let config: PublicNetworkConfig = inner.get("config/hotshot").send().await?;

        Ok(Self {
            inner,
            epoch_height: config.hotshot_config().blocks_per_epoch(),
            epoch_start_block: config.hotshot_config().epoch_start_block(),
            stream_timeout: opt.stream_timeout,
        })
    }

    /// A fallible leaf stream.
    ///
    /// This stream wraps a raw socket connection, which might encounter an error or end at any
    /// time. This can be further processed into an infinite, infallible stream by dropping errors
    /// and reconnecting when the stream ends (as in [`EspressoClient::leaves`]).
    fn fallible_leaves(
        &self,
        from: u64,
    ) -> impl Send + Unpin + Stream<Item = Result<(Leaf2, BitVec)>> + use<> {
        // Set up a raw WebSocket stream.
        let socket_stream = self
            .inner
            .socket(&format!("availability/stream/leaves/{from}"))
            .subscribe::<LeafQueryData<SeqTypes>>()
            .try_flatten_stream();

        // Map the result and error types.
        let socket_stream = socket_stream.map(|res| match res {
            Ok(leaf) => {
                let signers = leaf.qc().signatures.as_ref().ok_or_else(|| {
                    Error::internal().context(format!(
                        "QC for leaf {} is missing signers bitmap",
                        leaf.height()
                    ))
                })?;
                Ok((leaf.leaf().clone(), signers.1.clone()))
            }
            Err(err) => Err(Error::from(err)),
        });

        // This `try_unfold` accomplishes two things:
        // * As soon as the underlying WebSocket stream returns an error, this stream will yield the
        //   error and then terminate (this is the normal behavior of `try_unfold`). This will cause
        //   the wrapping infallible stream to try and reconnect, which is generally a good thing to
        //   do whenever we get an error from our connection.
        // * If we ever go longer than `stream_timeout` between values yielded by the WebSocket
        //   stream, we will yield an error message and then terminate, triggering a reconnection.
        //   This leads to a better failure mode in case the server goes away without sending a
        //   closing handshake: we will reestablish a connection after some delay, whereas the
        //   default behavior of the client library is to just block forever.
        let stream_timeout = self.stream_timeout;
        let try_stream =
            stream::try_unfold(socket_stream.boxed(), move |mut socket_stream| async move {
                match timeout(stream_timeout, socket_stream.try_next()).await {
                    Ok(Ok(next)) => Ok(next.map(|leaf| (leaf, socket_stream))),
                    Ok(Err(err)) => Err(err),
                    Err(timeout) => Err(Error::internal().context(format!(
                        "timed out waiting for message from WebSocket: {timeout}"
                    ))),
                }
            });

        // Make it `Unpin`
        try_stream.boxed()
    }
}

impl EspressoClient for QueryServiceClient {
    #[instrument(skip(self))]
    async fn wait_for_epochs(&self) -> u64 {
        // Wait until the first real epoch starts, which is 2 epochs after the "epoch" containing
        // epochs start block.
        let first_epoch = epoch_from_block_number(self.epoch_start_block, self.epoch_height) + 2;
        let first_block_in_epoch = (first_epoch - 1) * self.epoch_height + 1;

        // Check if we are already in a later epoch.
        let last_block = loop {
            match self.inner.get::<u64>("node/block-height").send().await {
                Ok(0) => {
                    tracing::info!("waiting for Espresso blocks");
                }
                Ok(height) => break height - 1,
                Err(err) => {
                    tracing::warn!("error getting block height, will retry: {err:#}");
                }
            }
            sleep(Duration::from_secs(1)).await;
        };
        if last_block >= first_block_in_epoch {
            let current_epoch = epoch_from_block_number(last_block, self.epoch_height);
            tracing::info!(
                last_block,
                first_epoch,
                first_block_in_epoch,
                current_epoch,
                "epochs have already started"
            );
            return current_epoch;
        }

        // We have not reached the first block in the first real epoch, wait for it.
        tracing::info!(
            last_block,
            first_epoch,
            first_block_in_epoch,
            "waiting for epochs to start"
        );
        self.leaves(first_block_in_epoch).next().await;

        first_epoch
    }

    async fn epoch_height(&self) -> Result<u64> {
        Ok(self.epoch_height)
    }

    async fn leaf(&self, height: u64) -> Result<Leaf2> {
        let leaf: LeafQueryData<SeqTypes> = self
            .inner
            .get(&format!("availability/leaf/{height}"))
            .send()
            .await?;
        Ok(leaf.leaf().clone())
    }

    async fn stake_table_for_epoch(&self, epoch: u64) -> Result<ValidatorMap> {
        let nodes: ValidatorMap = self
            .inner
            .get(&format!("node/validators/{epoch}"))
            .send()
            .await?;
        ensure!(
            !nodes.is_empty(),
            Error::internal().context(format!(
                "query node returned an empty stake table for epoch {epoch}"
            ))
        );
        Ok(nodes)
    }

    async fn block_reward(&self, epoch: u64) -> Result<ESPTokenAmount> {
        let reward: Option<RewardAmount> = self
            .inner
            .get(&format!("node/block-reward/epoch/{epoch}"))
            .send()
            .await?;

        reward.map(|r| r.0).ok_or_else(|| {
            Error::not_found().context(format!("block reward not found for epoch {epoch}"))
        })
    }

    #[instrument(skip(self))]
    async fn fetch_all_reward_accounts(
        &self,
        block: u64,
    ) -> Result<Vec<(Address, ESPTokenAmount)>> {
        let limit = 10_000_u64;
        let mut all_accounts = Vec::new();
        let mut offset = 0u64;

        loop {
            let mut attempt = 0;
            let accounts: Vec<(RewardAccountV2, RewardAmount)> = loop {
                attempt += 1;
                match self
                    .inner
                    .get(&format!("catchup/{block}/reward-amounts/{limit}/{offset}"))
                    .send()
                    .await
                {
                    Ok(accounts) => break accounts,
                    Err(e) if attempt < 3 => {
                        tracing::warn!(block, offset, error = %e, "failed to fetch reward accounts, retrying");
                        sleep(Duration::from_secs(1)).await;
                    }
                    Err(e) => return Err(e.into()),
                }
            };

            let num_fetched = accounts.len() as u64;
            tracing::debug!(block, offset, num_fetched, "fetched reward accounts batch");

            all_accounts.extend(
                accounts
                    .into_iter()
                    .map(|(account, amount)| (account.into(), amount.0)),
            );

            if num_fetched < limit {
                break;
            }

            offset += num_fetched;
        }

        tracing::warn!(
            block,
            total_accounts = all_accounts.len(),
            "fetched all reward accounts"
        );
        Ok(all_accounts)
    }

    fn leaves(&self, from: u64) -> impl Send + Unpin + Stream<Item = (Leaf2, BitVec)> {
        let fallible_stream = self.fallible_leaves(from);
        stream::unfold(
            (fallible_stream, self.clone(), from),
            |(mut fallible_stream, client, from)| async move {
                // Try to get the next leaf until we succeed.
                loop {
                    match fallible_stream.next().await {
                        Some(Ok(leaf)) => {
                            // On success advance `from` by 1, so that if we reconnect after this,
                            // we will start from the next leaf after this one.
                            tracing::debug!(from, ?leaf, "got new Espresso leaf");
                            return Some((leaf, (fallible_stream, client, from + 1)));
                        }
                        Some(Err(err)) => {
                            tracing::error!("error from leaf stream: {err:#}");
                        }
                        None => {
                            tracing::error!("leaf stream ended unexpectedly, reconnecting");
                            fallible_stream = client.fallible_leaves(from);
                        }
                    }

                    // If there was any kind of error, pause a bit before retrying.
                    sleep(Duration::from_secs(1)).await;
                }
            },
        )
        .boxed()
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::input::espresso::{
        State,
        testing::{EPOCH_HEIGHT, MemoryStorage, start_pos_network},
    };
    use crate::metrics::PrometheusMetrics;

    use super::*;

    use async_lock::RwLock;
    use espresso_types::{MaxSupportedVersion, SequencerVersions, traits::PersistenceOptions};
    use hotshot_query_service::data_source::SqlDataSource;
    use hotshot_types::{
        data::EpochNumber,
        traits::{
            block_contents::BlockHeader,
            node_implementation::{ConsensusTime, Versions},
        },
    };
    use portpicker::pick_unused_port;
    use sequencer::{
        api::{
            Options,
            data_source::testing::TestableSequencerDataSource,
            sql::DataSource,
            test_helpers::{TestNetwork, TestNetworkConfigBuilder},
        },
        testing::TestConfigBuilder,
    };
    use surf_disco::{Error, StatusCode};
    use tokio::task::spawn;

    type V = SequencerVersions<MaxSupportedVersion, MaxSupportedVersion>;

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epochs() {
        let port = pick_unused_port().expect("No ports free");
        let (mut network, _, _storage) = start_pos_network(port).await;

        let opt = QueryServiceOptions::new(format!("http://localhost:{port}").parse().unwrap());
        let client = QueryServiceClient::new(opt).await.unwrap();

        // Wait for epochs to start and check that the client returns the correct starting epoch
        // number.
        let first_epoch = client.wait_for_epochs().await;
        assert_eq!(first_epoch, 3);

        // Wait for the next epoch to start and check that `wait_for_epochs` then returns the
        // _current_ epoch number.
        let next_epoch = first_epoch + 1;
        let next_epoch_first_block = (next_epoch - 1) * EPOCH_HEIGHT + 1;
        client.leaves(next_epoch_first_block).next().await;
        assert_eq!(client.wait_for_epochs().await, next_epoch);

        // Check stake table.
        for epoch in first_epoch..=next_epoch {
            let epoch = EpochNumber::new(epoch);
            let expected = get_stake_table(&network, epoch).await;
            let stake_table = client.stake_table_for_epoch(*epoch).await.unwrap();
            assert_eq!(stake_table.len(), 1);
            assert_eq!(stake_table, expected);
        }

        network.stop_consensus().await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_epoch_height() {
        // Check that varying epoch heights correctly get picked up by the client.
        for epoch_height in [100, 200] {
            tracing::info!(epoch_height, "testing with new epoch height");
            let port = pick_unused_port().unwrap();

            let config = TestNetworkConfigBuilder::<1, _, _>::with_num_nodes()
                .api_config(Options::with_port(port).config(Default::default()))
                .network_config(
                    TestConfigBuilder::default()
                        .epoch_height(epoch_height)
                        .build(),
                )
                .build();
            let mut network = TestNetwork::new(config, V::new()).await;

            let opt = QueryServiceOptions::new(format!("http://localhost:{port}").parse().unwrap());
            let client = QueryServiceClient::new(opt).await.unwrap();
            assert_eq!(client.epoch_height().await.unwrap(), epoch_height);

            network.stop_consensus().await;
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_leaf() {
        let port = pick_unused_port().unwrap();

        let storage = DataSource::create_storage().await;
        let persistence =
            <DataSource as TestableSequencerDataSource>::persistence_options(&storage);

        let options =
            SqlDataSource::options(&storage, Options::with_port(port)).config(Default::default());
        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(options.clone())
            .persistences([persistence.clone()])
            .network_config(TestConfigBuilder::default().build())
            .build();
        let network = TestNetwork::new(config, V::new()).await;

        let opt = QueryServiceOptions::new(format!("http://localhost:{port}").parse().unwrap());
        let client = QueryServiceClient::new(opt).await.unwrap();

        let leaf = network.server.decided_leaf().await;
        assert_eq!(leaf, client.leaf(leaf.height()).await.unwrap());

        // Check an unavailable leaf.
        let err = client.leaf(1_000_000).await.unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_leaves_reconnect() {
        let port = pick_unused_port().unwrap();

        // We need persistence so that we can restart the network and have it resume where it left
        // off.
        let storage = DataSource::create_storage().await;
        let persistence =
            <DataSource as TestableSequencerDataSource>::persistence_options(&storage);

        let options =
            SqlDataSource::options(&storage, Options::with_port(port)).config(Default::default());
        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(options.clone())
            .persistences([persistence.clone()])
            .network_config(TestConfigBuilder::default().build())
            .build();
        let mut network = TestNetwork::new(config, V::new()).await;

        let opt = QueryServiceOptions {
            url: format!("http://localhost:{port}").parse().unwrap(),
            // Have a fast timeout since this test is going to intentionally disrupt the connection,
            // and we want it to recover and finish quickly.
            stream_timeout: Duration::from_secs(1),
        };
        let client = QueryServiceClient::new(opt).await.unwrap();
        let mut leaves = client.leaves(1);

        // Stream a few leaves before shutting down.
        wait_for_leaves(&client, &mut leaves, 1..5).await;

        // Interrupt the stream.
        network.stop_consensus().await;
        let reached_height = network.server.decided_leaf().await.height();
        tracing::info!(reached_height, "Shutting down network");
        drop(network);

        // If we keep consuming leaves we will eventually block (once we've consumed everything that
        // had already been buffered in the WebSocket connection).
        let mut next_leaf = 5;
        let mut leaves = Box::pin(leaves.peekable());
        loop {
            tracing::info!("checking if leaf {next_leaf} is buffered");
            match timeout(Duration::from_secs(5), leaves.as_mut().peek()).await {
                Ok(_) => {
                    // Consume the leaf we peeked.
                    let leaf = leaves.next().await.unwrap().0;
                    assert_eq!(leaf.height(), next_leaf);
                    next_leaf += 1;
                }
                Err(_) => {
                    // We shouldn't have been able to get farther than the server did.
                    assert!(next_leaf <= reached_height + 1);
                    break;
                }
            }
        }

        // Restart the server so the stream can reconnect.
        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(options)
            .persistences([persistence])
            .network_config(TestConfigBuilder::default().build())
            .build();
        let mut network = TestNetwork::new(config, V::new()).await;

        // The same stream should pick up exactly where we left off.
        wait_for_leaves(&client, &mut leaves, next_leaf..next_leaf + 5).await;

        // Drop leaves stream before shutting down network. This prevents a panic in the async-std
        // adapter within Tungstenite that occurs when the stream is still going but the tokio
        // runtime goes away. This doesn't affect the test results but it avoids an ugly stack dump.
        drop(leaves);
        network.stop_consensus().await;
    }

    async fn wait_for_leaves(
        client: &QueryServiceClient,
        leaves: &mut (impl Stream<Item = (Leaf2, BitVec)> + Unpin),
        range: impl IntoIterator<Item = u64>,
    ) {
        for i in range {
            tracing::info!(i, "wait for leaf");
            let leaf = leaves.next().await.unwrap().0;
            assert_eq!(leaf.height(), i);
            assert_eq!(leaf, client.leaf(i).await.unwrap());
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_with_state() {
        let port = pick_unused_port().expect("No ports free");
        let (mut network, _, _storage) = start_pos_network(port).await;
        let first_epoch = *network
            .server
            .decided_leaf()
            .await
            .epoch(EPOCH_HEIGHT)
            .unwrap();

        let opt = QueryServiceOptions::new(format!("http://localhost:{port}").parse().unwrap());
        let client = QueryServiceClient::new(opt).await.unwrap();
        let state = Arc::new(RwLock::new(
            State::new(
                MemoryStorage::default(),
                client.clone(),
                PrometheusMetrics::default(),
            )
            .await
            .unwrap(),
        ));
        let task = spawn(State::update_task(state.clone()));

        // Wait for the first iteration of the update task to populate the snapshot.
        while let Err(err) = state.read().await.active_node_set().await {
            tracing::info!("waiting for first snapshot to be ready: {err:#}");
            sleep(Duration::from_secs(1)).await;
        }

        // Wait for an epoch change and check that the client's snapshot is always consistent with
        // the Espresso block.
        loop {
            let snapshot = state.read().await.active_node_set().await.unwrap();
            tracing::info!(?snapshot, "checking snapshot");

            // Check leaf consistency.
            let leaf = client.leaf(snapshot.espresso_block.block).await.unwrap();
            assert_eq!(
                snapshot.espresso_block.epoch,
                *leaf.epoch(EPOCH_HEIGHT).unwrap()
            );
            assert_eq!(
                snapshot.espresso_block.timestamp,
                leaf.block_header().timestamp_millis()
            );

            // Check stats.
            //
            // There is only one node, so we cannot have a QC without this node participating. Thus
            // its voter participation must be 1.
            assert_eq!(snapshot.nodes[0].votes, snapshot.nodes[0].eligible_votes);
            // There still could be timeouts, so the leader participation might not be 1, but it
            // must be greater than 0 since at this point at least one leaf has been decided.
            assert!(snapshot.nodes[0].proposals > 0);

            if snapshot.espresso_block.epoch > first_epoch {
                tracing::info!(
                    height = leaf.height(),
                    first_epoch,
                    snapshot.espresso_block.epoch,
                    "changed epoch"
                );
                break;
            }
            tracing::info!(
                first_epoch,
                height = leaf.height(),
                view = ?leaf.view_number(),
                "waiting for epoch change"
            );
            sleep(Duration::from_secs(1)).await;
        }

        task.abort();
        task.await.ok();
        network.stop_consensus().await;
    }

    async fn get_stake_table<P: PersistenceOptions, const NUM_NODES: usize>(
        network: &TestNetwork<P, NUM_NODES, impl Versions>,
        epoch: EpochNumber,
    ) -> ValidatorMap {
        network
            .server
            .consensus()
            .read()
            .await
            .membership_coordinator
            .stake_table_for_epoch(Some(epoch))
            .await
            .unwrap()
            .coordinator
            .membership()
            .read()
            .await
            .active_validators(&epoch)
            .unwrap()
    }
}
