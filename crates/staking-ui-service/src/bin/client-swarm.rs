//! Simulates many simultaneous clients of the staking UI service.

use std::{
    collections::HashSet,
    fmt::{Debug, Display},
    iter,
    sync::Arc,
    time::{Duration, Instant},
};

use alloy::primitives::Address;
use async_lock::RwLock;
use clap::Parser;
use espresso_types::parse_duration;
use futures::{FutureExt, future::join_all};
use hotshot_query_service::metrics::PrometheusMetrics;
use hotshot_types::traits::metrics::{Counter, Histogram, Metrics as _};
use rand::{Rng, SeedableRng, rngs::StdRng};
use serde::de::DeserializeOwned;
use staking_ui_service::{
    Error, Result,
    input::espresso::{
        EspressoClient,
        client::{QueryServiceClient, QueryServiceOptions},
    },
    types::{
        common::L1BlockId,
        global::{
            ActiveNodeSetSnapshot, ActiveNodeSetUpdate, FullNodeSetSnapshot, FullNodeSetUpdate,
        },
        wallet::{WalletSnapshot, WalletUpdate},
    },
};
use surf_disco::Url;
use tide_disco::{App, Error as _, StatusCode};
use tokio::time::{sleep, timeout};
use toml::toml;
use tracing::instrument;
use tracing_subscriber::EnvFilter;
use vbs::version::{StaticVersion, StaticVersionType};

/// Simulate many simultaneous clients of the staking UI service.
#[derive(Clone, Debug, Parser)]
struct Options {
    /// URL for the staking UI service.
    #[clap(short, long)]
    url: Url,

    /// URL for the Espresso query service.
    #[clap(short, long)]
    espresso_url: Url,

    /// Number of simultaneous clients to simulate.
    #[clap(short, long, default_value = "1")]
    num_clients: usize,

    /// Interval between polls for each client.
    #[clap(long, default_value = "1s", value_parser = parse_duration)]
    polling_interval: Duration,

    /// Average client session duration.
    #[clap(long, default_value = "1m", value_parser = parse_duration)]
    session_length: Duration,

    /// Proportional randomness applied to all sampled durations.
    #[clap(long, default_value = "0.1")]
    jitter: f32,

    /// Port on which to host a status/metrics API.
    #[clap(short, long, default_value = "8081")]
    port: u16,
}

#[derive(Debug)]
struct Metrics {
    requests: Box<dyn Counter>,
    failures: Box<dyn Counter>,
    duration: Box<dyn Histogram>,
}

#[derive(Clone, Debug)]
struct InstrumentedHTTPClient {
    metrics: Arc<Metrics>,
    client: surf_disco::Client<Error, StaticVersion<0, 1>>,
}

impl InstrumentedHTTPClient {
    fn new(url: Url, metrics: Arc<Metrics>) -> Self {
        Self {
            client: surf_disco::Client::new(url),
            metrics,
        }
    }

    async fn get<T: Debug + DeserializeOwned>(
        &self,
        endpoint: impl Display,
        check_success: impl FnOnce(&Result<T>) -> Result<()>,
    ) -> Result<T> {
        tracing::debug!("> GET {endpoint}");
        let start = Instant::now();
        let result = self.client.get(&endpoint.to_string()).send().await;
        let elapsed = start.elapsed();
        tracing::debug!("< GET {endpoint} -> {result:?} ({elapsed:?})");

        self.metrics.requests.add(1);
        self.metrics
            .duration
            .add_point((elapsed.as_millis() as f64) / 1000.0);
        if let Err(err) = check_success(&result) {
            tracing::warn!("request failure ({endpoint}): {err:#}");
            self.metrics.failures.add(1);
        }

        result
    }
}

#[derive(Debug)]
struct Client {
    opt: Options,
    rng: StdRng,
    wallet: Address,
    inner: InstrumentedHTTPClient,
}

impl Client {
    fn new(opt: Options, inner: InstrumentedHTTPClient, id: u64, wallet: Address) -> Self {
        Self {
            inner,
            opt,
            rng: StdRng::seed_from_u64(id),
            wallet,
        }
    }

    #[instrument(skip(self), fields(id = %self.wallet))]
    async fn run(&mut self) {
        loop {
            let session_length = self.sample_duration(self.opt.session_length);
            tracing::info!(?session_length, "starting session");
            timeout(session_length, self.session()).await.ok();
        }
    }

    #[instrument(skip(self), fields(id = %self.wallet))]
    async fn session(&mut self) {
        // All sessions start by getting full snapshots.
        let mut l1_block: L1BlockId = self.get_infallible("l1/block/latest").await;
        self.get_forget::<FullNodeSetSnapshot>(format!("nodes/all/{}", l1_block.hash))
            .await;
        self.get_forget::<WalletSnapshot>(format!("wallet/{}/{}", self.wallet, l1_block.hash))
            .await;
        let mut espresso_block = self
            .get_infallible::<ActiveNodeSetSnapshot>("nodes/active")
            .await
            .espresso_block
            .block;

        // We proceed to stream updates until cancelled.
        loop {
            // Poll for next L1 block.
            if let Ok(next_l1_block) = self.poll(format!("l1/block/{}", l1_block.number + 1)).await
            {
                l1_block = next_l1_block;
                self.get_forget::<FullNodeSetUpdate>(format!(
                    "nodes/all/updates/{}",
                    l1_block.hash
                ))
                .await;
                self.get_forget::<WalletUpdate>(format!(
                    "wallet/{}/updates/{}",
                    self.wallet, l1_block.hash
                ))
                .await;
            }

            // Poll for next Espresso block.
            if self
                .poll::<ActiveNodeSetUpdate>(format!("nodes/active/updates/{}", espresso_block + 1))
                .await
                .is_ok()
            {
                espresso_block += 1;
                self.get_forget::<ActiveNodeSetUpdate>(format!(
                    "nodes/active/updates/{espresso_block}"
                ))
                .await;
                // self.get_forget::<ESPTokenAmount>(format!(
                //     "wallet/{}/rewards/{espresso_block}", self.wallet,
                // ))
                // .await;
            }
            self.sleep(self.opt.polling_interval).await;
        }
    }

    /// Make a GET request and retry until it succeeds, returning the results.
    async fn get_infallible<T: Debug + DeserializeOwned>(&mut self, endpoint: impl Display) -> T {
        loop {
            match self.get(&endpoint).await {
                Ok(res) => break res,
                Err(_) => {
                    self.sleep(self.opt.polling_interval).await;
                }
            }
        }
    }

    /// Make a GET request, but don't return the result, just record success/failure and duration.
    async fn get_forget<T: Debug + DeserializeOwned>(&mut self, endpoint: impl Display) {
        self.get::<T>(endpoint).await.ok();
    }

    /// Make a GET HTTP request.
    async fn get<T: Debug + DeserializeOwned>(&mut self, endpoint: impl Display) -> Result<T> {
        self.inner
            .get(endpoint, |res| match res {
                Ok(_) => Ok(()),
                Err(err) => Err(err.clone()),
            })
            .await
    }

    /// Make a GET HTTP request. 404 responses are returned but not logged as errors.
    async fn poll<T: Debug + DeserializeOwned>(&mut self, endpoint: impl Display) -> Result<T> {
        self.inner
            .get(endpoint, |res| match res {
                Ok(_) => Ok(()),
                Err(err) if err.status() == StatusCode::NOT_FOUND => Ok(()),
                Err(err) => Err(err.clone()),
            })
            .await
    }

    async fn sleep(&mut self, duration: Duration) {
        sleep(self.sample_duration(duration)).await;
    }

    fn sample_duration(&mut self, from: Duration) -> Duration {
        let jitter = self.rng.gen_range(-self.opt.jitter..=self.opt.jitter) as f64;
        Duration::from_millis(((from.as_millis() as f64) * (1f64 + jitter)) as u64)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let opt = Options::parse();

    let api = toml! {
        [route.metrics]
        PATH = ["/metrics"]
        METHOD = "METRICS"
        DOC = "Prometheus endpoint exposing metrics related to the performance of the service under test."
    };

    let metrics = PrometheusMetrics::default();
    let client = InstrumentedHTTPClient::new(
        opt.url.join("/v0/staking/")?,
        Arc::new(Metrics {
            requests: metrics.create_counter("requests".into(), None),
            failures: metrics.create_counter("failures".into(), None),
            duration: metrics.create_histogram("request_duration".into(), Some("s".into())),
        }),
    );

    // Get delegator addresses, so we have non-trivial wallets to simulate.
    let espresso =
        QueryServiceClient::new(QueryServiceOptions::new(opt.espresso_url.clone())).await?;
    let validators = espresso
        .stake_table_for_epoch(espresso.wait_for_epochs().await)
        .await?;
    let delegators = validators
        .values()
        .flat_map(|v| v.delegators.keys())
        .copied()
        .collect::<HashSet<_>>();

    let mut app = App::<_, Error>::with_state(Arc::new(RwLock::new(metrics)));
    app.module::<Error, StaticVersion<0, 1>>("status", api)?
        .metrics("metrics", |_, metrics| {
            async move { Ok(std::borrow::Cow::Borrowed(metrics)) }.boxed()
        })?;

    let port = opt.port;
    let server = async move {
        if let Err(err) = app
            .serve(format!("0.0.0.0:{port}"), StaticVersion::<0, 1>::instance())
            .await
        {
            tracing::error!("metrics server exited: {err:#}");
        }
    }
    .boxed();
    let futures = delegators
        .iter()
        .cycle()
        .take(opt.num_clients)
        .enumerate()
        .map(|(id, wallet)| {
            let mut client = Client::new(opt.clone(), client.clone(), id as u64, *wallet);
            async move { client.run().await }.boxed()
        });
    join_all(iter::once(server).chain(futures)).await;

    Ok(())
}
