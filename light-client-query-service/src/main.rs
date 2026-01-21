use std::{cmp::max, fs, path::PathBuf, process::ExitCode, time::Duration};

use anyhow::{Context, Result};
use clap::Parser;
use espresso_types::parse_duration;
use hotshot_query_service::{
    availability::{self, BlockInfo, LeafId, UpdateAvailabilityData},
    fetching::provider::{AnyProvider, QueryServiceProvider},
    node, ApiState,
};
use light_client::{
    client::{Client, QueryServiceClient},
    state,
    storage::{LightClientSqliteOptions, Storage},
    LightClient,
};
use light_client_query_service::{init_logging, LogFormat};
use semver::Version;
use sequencer::{
    api::{data_source::SequencerDataSource, sql::DataSource},
    persistence::sql,
    SequencerApiVersion,
};
use tide_disco::{App, Url};
use tokio::{spawn, time::sleep};
use tracing::instrument;
use vbs::version::StaticVersionType;

/// Run an Espresso query service.
///
/// The light-client based query service connects to an untrusted node which is running HotShot
/// consensus or otherwise serving a query service API. This node itself does not need to be
/// participating in consensus. It fetches all the necessary information from the connected query
/// node and verifies it locally before storing it in the query database and serving it.
#[derive(Debug, Parser)]
struct Args {
    #[clap(flatten)]
    lc_db: LightClientSqliteOptions,

    #[clap(flatten)]
    lc_opt: state::LightClientOptions,

    #[clap(flatten)]
    ds_opt: sql::Options,

    #[clap(flatten)]
    poll_opt: PollingOptions,

    /// Light client genesis TOML file.
    #[clap(long = "light-client-genesis", env = "LIGHT_CLIENT_GENESIS")]
    genesis: PathBuf,

    /// URL for an untrusted Espresso query service.
    #[clap(long = "light-client-espresso-url", env = "LIGHT_CLIENT_ESPRESSO_URL")]
    espresso_url: Url,

    /// Port on which to serve the query API.
    #[clap(long, env = "QUERY_SERVICE_PORT")]
    api_port: u16,

    /// Formatting options for tracing.
    #[clap(long, env = "RUST_LOG_FORMAT")]
    log_format: Option<LogFormat>,
}

#[derive(Clone, Copy, Debug, Parser)]
struct PollingOptions {
    /// Interval between polling the upstream query service for new blocks.
    #[clap(long, env = "POLL_INTERVAL", value_parser = parse_duration, default_value = "1s")]
    poll_interval: Duration,

    /// Delay before retrying failed requests to the upstream query service.
    #[clap(long, env = "RETRY_DELAY", value_parser = parse_duration, default_value = "1s")]
    retry_delay: Duration,

    /// Maximum number of new leaves to process in a single polling update.
    #[clap(long, env = "MAX_LEAVES_PER_UPDATE", default_value = "10")]
    max_leaves_per_update: u64,
}

#[instrument(skip(lc, ds))]
async fn update<P, S>(lc: LightClient<P, S>, ds: DataSource, poll_opt: PollingOptions)
where
    P: Storage,
    S: Client,
{
    let mut height = loop {
        match lc.block_height().await {
            Ok(height) => break height,
            Err(err) => {
                tracing::error!("failed to fetch height from light client: {err:#}");
                sleep(poll_opt.retry_delay).await;
            },
        }
    };
    tracing::info!(height, "starting update task");
    loop {
        match try_update(&lc, &ds, height, poll_opt).await {
            Ok(new_height) => height = new_height,
            Err(err) => tracing::error!(height, "error while updating state: {err:#}"),
        }
        sleep(poll_opt.poll_interval).await;
    }
}

#[instrument(skip(lc, ds))]
async fn try_update<P, S>(
    lc: &LightClient<P, S>,
    ds: &DataSource,
    height: u64,
    poll_opt: PollingOptions,
) -> Result<u64>
where
    P: Storage,
    S: Client,
{
    let new_height = loop {
        let new_height = lc.block_height().await.context("getting block height")?;
        if new_height > height {
            break new_height;
        }

        tracing::debug!("waiting for height to increase");
        sleep(Duration::from_secs(1)).await;
    };
    tracing::debug!(
        height,
        new_height,
        "block height has increased, updating state"
    );

    // Store a maximum number of leaves in a given update. This allows us to move forward to the
    // next update quickly, even if the height has increased by a huge number (such as from 0 to the
    // current height, the first time this service has started). Still allowing for a limited number
    // of multiple leaves to be stored in a single update limits the risk of missing any leaves in
    // the common case where the block height hasn't increased by _that_ much.
    //
    // In any case, any leaves we miss here will be automatically backfilled by the query service,
    // in an asynchronous manner.
    let from = max(
        height,
        new_height.saturating_sub(poll_opt.max_leaves_per_update),
    );
    tracing::debug!(from, new_height, "saving new leaves");
    for block in from..new_height {
        let leaf = lc
            .fetch_leaf(LeafId::Number(block as usize))
            .await
            .context(format!("fetching leaf {block}"))?;
        let (payload, vid_common) = lc
            .fetch_block_and_vid_common_for_header(leaf.header().clone())
            .await
            .context(format!("fetching block {block}"))?;
        ds.append(BlockInfo::new(leaf, Some(payload), Some(vid_common), None))
            .await
            .context(format!("storing data for block {block}"))?;
    }
    Ok(new_height)
}

async fn run() -> Result<()> {
    let opt = Args::parse();
    init_logging(opt.log_format);

    // Initialize light client.
    let lc_db = opt
        .lc_db
        .connect()
        .await
        .context("connecting to light client database")?;
    let lc_server = QueryServiceClient::new(opt.espresso_url.clone());
    let lc_genesis_bytes = fs::read(opt.genesis).context("reading genesis file")?;
    let lc_genesis =
        toml::from_str(str::from_utf8(&lc_genesis_bytes).context("malformed genesis file")?)
            .context("malformed genesis file")?;
    let lc = LightClient::from_genesis_with_options(lc_db, lc_server, lc_genesis, opt.lc_opt);

    // Initialize query service.
    let provider = QueryServiceProvider::new(opt.espresso_url, SequencerApiVersion::instance());
    let provider = AnyProvider::default().with_provider(provider);
    let ds = DataSource::create(opt.ds_opt, provider, false)
        .await
        .context("connecting to API data source")?;

    // Use light client to update API database with new blocks.
    spawn(update(lc, ds.clone(), opt.poll_opt));

    // Run server.
    let mut app = App::<_, hotshot_query_service::Error>::with_state(ApiState::from(ds));
    let ver = SequencerApiVersion::instance();
    let api_ver: Version = "1.0.0".parse().unwrap();
    app.register_module(
        "availability",
        availability::define_api(&Default::default(), ver, api_ver.clone())?,
    )?
    .register_module("node", node::define_api(&Default::default(), ver, api_ver)?)?;
    app.serve(
        format!("0.0.0.0:{}", opt.api_port),
        SequencerApiVersion::instance(),
    )
    .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(err) = run().await {
        tracing::error!("{err:#}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
