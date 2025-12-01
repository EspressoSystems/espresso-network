//! Simulates many simultaneous clients of the staking UI service.

use std::{
    cmp::max,
    collections::HashSet,
    fmt::{Debug, Display},
    sync::Arc,
    time::{Duration, Instant},
};

use alloy::primitives::Address;
use async_lock::RwLock;
use clap::Parser;
use espresso_types::parse_duration;
use futures::{
    FutureExt, SinkExt, StreamExt,
    channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded},
    future::{Either, join_all},
    pin_mut, stream,
};
use hotshot_query_service::metrics::PrometheusMetrics;
use hotshot_types::traits::metrics::{Counter, CounterFamily, Gauge, Histogram, Metrics as _};
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
use tokio::{
    task::spawn,
    time::{sleep, timeout},
};
use toml::toml;
use tracing::instrument;
use tracing_subscriber::EnvFilter;
use vbs::version::{StaticVersion, StaticVersionType};

/// Simulate many simultaneous clients of the staking UI service.
#[derive(Clone, Debug, Parser)]
struct Options {
    /// URL for the staking UI service.
    #[clap(short, long, env = "STAKING_CLIENT_SWARM_STAKING_SERVICE")]
    url: Url,

    /// URL for the Espresso query service.
    #[clap(short, long, env = "STAKING_CLIENT_SWARM_QUERY_SERVICE")]
    espresso_url: Url,

    /// Number of simultaneous clients to simulate.
    #[clap(
        short,
        long,
        default_value = "1",
        env = "STAKING_CLIENT_SWARM_NUM_CLIENTS"
    )]
    num_clients: usize,

    /// Interval between polls for each client.
    #[clap(
        long,
        default_value = "1s",
        value_parser = parse_duration,
        env = "STAKING_CLIENT_SWARM_POLLING_INTERVAL",
    )]
    polling_interval: Duration,

    /// Average client session duration.
    #[clap(
        long,
        default_value = "1m",
        value_parser = parse_duration,
        env = "STAKING_CLIENT_SWARM_SESSION_LENGTH",
    )]
    session_length: Duration,

    /// Proportional randomness applied to all sampled durations.
    #[clap(long, default_value = "0.1", env = "STAKING_CLIENT_SWARM_JITTER")]
    jitter: f32,

    /// Port on which to host a status/metrics API.
    #[clap(short, long, default_value = "8081", env = "STAKING_CLIENT_SWARM_PORT")]
    port: u16,
}

#[derive(Clone, Copy, Debug, Default)]
struct WindowMetrics {
    // The total number of requests up to and including this tick.
    requests: usize,
    // The total number of failures up to and including this tick.
    failures: usize,
    // The total time spent in requests up to and including in this tick.
    total_duration: Duration,
    // The maximum request duration _within_ this tick.
    max_duration: Duration,
}

// Number of seconds in our moving average statistics.
const ROLLUP: usize = 5;

#[derive(Debug)]
struct Metrics {
    requests: Box<dyn Counter>,
    failures: Box<dyn Counter>,
    duration: Box<dyn Histogram>,
    slow_requests: Box<dyn CounterFamily>,

    // Computed metrics
    elapsed: Box<dyn Counter>,
    rollup: Box<dyn Gauge>,
    requests_per_sec: Box<dyn Gauge>,
    requests_per_failure: Box<dyn Gauge>,
    duration_avg: Box<dyn Gauge>,
    duration_max: Box<dyn Gauge>,

    // Circular buffer of metrics after each tick in the rollup window. We use this to compute
    // moving averages.
    window: [WindowMetrics; ROLLUP],
    // Pointer to the current tick in the circular window buffer.
    rollup_pointer: usize,
}

#[derive(Clone, Copy, Debug, derive_more::Display)]
enum Endpoint {
    L1Latest,
    L1Block,
    ActiveNodesSnapshot,
    ActiveNodesUpdate,
    FullNodesSnapshot,
    FullNodesUpdate,
    WalletSnapshot,
    WalletUpdate,
    // Rewards,
}

impl Metrics {
    fn new(registry: &PrometheusMetrics) -> Self {
        let metrics = Self {
            requests: registry.create_counter("requests".into(), None),
            failures: registry.create_counter("failures".into(), None),
            duration: registry.create_histogram("request_duration".into(), Some("s".into())),
            slow_requests: registry
                .counter_family("slow_requests".into(), vec!["endpoint".to_string()]),
            elapsed: registry.create_counter("elapsed".into(), Some("s".into())),
            rollup: registry.create_gauge("rollup".into(), Some("s".into())),
            requests_per_sec: registry.create_gauge("requests_per_sec".into(), None),
            requests_per_failure: registry.create_gauge("requests_per_failure".into(), None),
            duration_avg: registry.create_gauge("duration_avg".into(), Some("ms".into())),
            duration_max: registry.create_gauge("duration_max".into(), Some("ms".into())),

            window: Default::default(),
            rollup_pointer: 0,
        };

        // Set constant rollup metric for visibility.
        metrics.rollup.set(ROLLUP);

        metrics
    }

    async fn update(mut self, requests: UnboundedReceiver<RequestResult>) {
        let tick = stream::unfold((), |()| async move {
            sleep(Duration::from_secs(1)).await;
            Some(((), ()))
        });
        let stream = stream::select(requests.map(Either::Left), tick.map(Either::Right));
        pin_mut!(stream);
        while let Some(next) = stream.next().await {
            match next {
                Either::Left(result) => {
                    let cur_tick = &mut self.window[self.rollup_pointer];

                    self.requests.add(1);
                    cur_tick.requests += 1;

                    self.duration
                        .add_point((result.elapsed.as_millis() as f64) / 1000.0);
                    cur_tick.total_duration += result.elapsed;
                    cur_tick.max_duration = max(cur_tick.max_duration, result.elapsed);

                    if !result.success {
                        self.failures.add(1);
                        cur_tick.failures += 1;
                    }

                    if result.elapsed > Duration::from_secs(1) {
                        tracing::warn!("slow request ({}, {:?})", result.endpoint, result.elapsed);
                        self.slow_requests
                            .create(vec![result.endpoint.to_string()])
                            .add(1);
                    }
                }
                Either::Right(_) => {
                    // Timer tick. Update computed stats.
                    self.elapsed.add(1);

                    // The tick at the start of the current `ROLLUP`-length window.
                    let prev_pointer = (self.rollup_pointer + 1) % ROLLUP;
                    let prev_tick = &self.window[prev_pointer];
                    let curr_tick = &self.window[self.rollup_pointer];

                    // Compute moving average.
                    let delta_requests = curr_tick.requests - prev_tick.requests;
                    let delta_failures = curr_tick.failures - prev_tick.failures;
                    let delta_duration = curr_tick.total_duration - prev_tick.total_duration;
                    let requests_per_sec = delta_requests / ROLLUP;
                    let requests_per_failure = if delta_failures == 0 {
                        usize::MAX
                    } else {
                        delta_requests / delta_failures
                    };
                    let avg_duration = if delta_requests == 0 {
                        usize::MAX
                    } else {
                        (delta_duration / (delta_requests as u32)).as_millis() as usize
                    };
                    self.requests_per_sec.set(requests_per_sec);
                    self.requests_per_failure.set(requests_per_failure);
                    self.duration_avg.set(avg_duration);

                    // Max duration is the max of all ticks in the current window, since each tick
                    // only stores the max duration within that tick.
                    self.duration_max.set(
                        self.window
                            .iter()
                            .map(|tick| tick.max_duration)
                            .max()
                            .unwrap_or_default()
                            .as_millis() as usize,
                    );

                    // Update moving average. `prev_tick` falls out of the window, and that slot
                    // becomes the new `curr_tick`.
                    self.window[prev_pointer] = WindowMetrics {
                        max_duration: Duration::ZERO,
                        ..*curr_tick
                    };
                    self.rollup_pointer = prev_pointer;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct RequestResult {
    endpoint: Endpoint,
    success: bool,
    elapsed: Duration,
}

impl RequestResult {
    fn new(endpoint: Endpoint, elapsed: Duration) -> Self {
        Self {
            endpoint,
            success: true,
            elapsed,
        }
    }
}

#[derive(Clone, Debug)]
struct InstrumentedHTTPClient {
    results: UnboundedSender<RequestResult>,
    client: surf_disco::Client<Error, StaticVersion<0, 1>>,
}

impl InstrumentedHTTPClient {
    fn new(url: Url, results: UnboundedSender<RequestResult>) -> Self {
        Self {
            client: surf_disco::Client::new(url),
            results,
        }
    }

    async fn get<T: Debug + DeserializeOwned>(
        &mut self,
        endpoint: Endpoint,
        path: impl Display,
        check_success: impl FnOnce(&Result<T>) -> Result<()>,
    ) -> Result<T> {
        tracing::debug!("> GET {path}");
        let start = Instant::now();
        let ret = self.client.get(&path.to_string()).send().await;
        let elapsed = start.elapsed();
        tracing::debug!("< GET {path} -> {ret:?} ({elapsed:?})");

        let mut result = RequestResult::new(endpoint, elapsed);
        if let Err(err) = check_success(&ret) {
            tracing::warn!("request failure ({path}): {err:#}");
            result.success = false;
        }
        self.results.send(result).await.ok();

        ret
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
        let mut l1_block: L1BlockId = self
            .get_infallible(Endpoint::L1Latest, "l1/block/latest")
            .await;
        self.get_forget::<FullNodeSetSnapshot>(
            Endpoint::FullNodesSnapshot,
            format!("nodes/all/{}", l1_block.hash),
        )
        .await;
        self.get_forget::<WalletSnapshot>(
            Endpoint::WalletSnapshot,
            format!("wallet/{}/{}", self.wallet, l1_block.hash),
        )
        .await;
        let mut espresso_block = self
            .get_infallible::<ActiveNodeSetSnapshot>(Endpoint::ActiveNodesSnapshot, "nodes/active")
            .await
            .espresso_block
            .block;

        // We proceed to stream updates until cancelled.
        loop {
            // Poll for next L1 block.
            if let Ok(next_l1_block) = self
                .poll(
                    Endpoint::L1Block,
                    format!("l1/block/{}", l1_block.number + 1),
                )
                .await
            {
                l1_block = next_l1_block;
                self.get_forget::<FullNodeSetUpdate>(
                    Endpoint::FullNodesUpdate,
                    format!("nodes/all/updates/{}", l1_block.hash),
                )
                .await;
                self.get_forget::<WalletUpdate>(
                    Endpoint::WalletUpdate,
                    format!("wallet/{}/updates/{}", self.wallet, l1_block.hash),
                )
                .await;
            }

            // Poll for next Espresso block.
            if self
                .poll::<ActiveNodeSetUpdate>(
                    Endpoint::ActiveNodesUpdate,
                    format!("nodes/active/updates/{}", espresso_block + 1),
                )
                .await
                .is_ok()
            {
                espresso_block += 1;
                // self.get_forget::<ESPTokenAmount>(Endpoint::Rewards, format!(
                //     "wallet/{}/rewards/{espresso_block}", self.wallet,
                // ))
                // .await;
            }
            self.sleep(self.opt.polling_interval).await;
        }
    }

    /// Make a GET request and retry until it succeeds, returning the results.
    async fn get_infallible<T: Debug + DeserializeOwned>(
        &mut self,
        endpoint: Endpoint,
        path: impl Display,
    ) -> T {
        loop {
            match self.get(endpoint, &path).await {
                Ok(res) => break res,
                Err(_) => {
                    self.sleep(self.opt.polling_interval).await;
                }
            }
        }
    }

    /// Make a GET request, but don't return the result, just record success/failure and duration.
    async fn get_forget<T: Debug + DeserializeOwned>(
        &mut self,
        endpoint: Endpoint,
        path: impl Display,
    ) {
        self.get::<T>(endpoint, path).await.ok();
    }

    /// Make a GET HTTP request.
    async fn get<T: Debug + DeserializeOwned>(
        &mut self,
        endpoint: Endpoint,
        path: impl Display,
    ) -> Result<T> {
        self.inner
            .get(endpoint, path, |res| match res {
                Ok(_) => Ok(()),
                Err(err) => Err(err.clone()),
            })
            .await
    }

    /// Make a GET HTTP request. 404 responses are returned but not logged as errors.
    async fn poll<T: Debug + DeserializeOwned>(
        &mut self,
        endpoint: Endpoint,
        path: impl Display,
    ) -> Result<T> {
        self.inner
            .get(endpoint, path, |res| match res {
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

    let registry = PrometheusMetrics::default();
    let metrics = Metrics::new(&registry);
    let (send_result, recv_result) = unbounded();
    let client = InstrumentedHTTPClient::new(opt.url.join("/v0/staking/")?, send_result);

    let mut app = App::<_, Error>::with_state(Arc::new(RwLock::new(registry)));
    app.module::<Error, StaticVersion<0, 1>>("status", api)?
        .metrics("metrics", |_, registry| {
            async move { Ok(std::borrow::Cow::Borrowed(registry)) }.boxed()
        })?;

    let port = opt.port;
    let server = spawn(async move {
        if let Err(err) = app
            .serve(format!("0.0.0.0:{port}"), StaticVersion::<0, 1>::instance())
            .await
        {
            tracing::error!("metrics server exited: {err:#}");
        }
    })
    .boxed();
    let metrics_task = spawn(async move {
        metrics.update(recv_result).await;
    })
    .boxed();

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
    let clients = delegators
        .iter()
        .cycle()
        .take(opt.num_clients)
        .enumerate()
        .map(|(id, wallet)| {
            let mut client = Client::new(opt.clone(), client.clone(), id as u64, *wallet);
            async move {
                client.run().await;
                Ok(())
            }
            .boxed()
        });

    join_all([server, metrics_task].into_iter().chain(clients)).await;

    Ok(())
}
