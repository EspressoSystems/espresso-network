use clap::Parser;
use hotshot_stake_table::config::STAKE_TABLE_CAPACITY;
use sequencer::{state_signature::relay_server::run_relay_server, SequencerApiVersion};
use sequencer_utils::logging;
use tide_disco::wait_for_server;
use url::Url;
use vbs::version::StaticVersionType;

#[derive(Parser)]
struct Args {
    /// Port to run the server on.
    #[clap(
        short,
        long,
        env = "ESPRESSO_STATE_RELAY_SERVER_PORT",
        default_value = "8083"
    )]
    port: u16,

    /// URL of a sequencer node that is currently providing the HotShot config.
    /// This is used to initialize the stake table.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_URL",
        default_value = "http://localhost:24000"
    )]
    pub sequencer_url: Url,

    /// Stake table capacity for the prover circuit
    #[clap(short, long, env = "ESPRESSO_SEQUENCER_STAKE_TABLE_CAPACITY", default_value_t = STAKE_TABLE_CAPACITY)]
    pub stake_table_capacity: usize,

    #[clap(flatten)]
    logging: logging::Config,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    args.logging.init();

    tracing::info!(port = args.port, "starting state relay server");

    // wait for 20 sec (interval 1 sec) for the sequencer to start.
    // sadly, we cannot specify this dependency in the `process-compose.yml` as it creates a cyclic dep:
    // seq0 requires relay server to post signatures to, and relay server requires seq0 to init and further update epoch-specific stake table
    wait_for_server(&args.sequencer_url, 20, 1000).await;

    run_relay_server(
        None,
        args.sequencer_url,
        args.stake_table_capacity as u64,
        format!("http://0.0.0.0:{}", args.port).parse().unwrap(),
        SequencerApiVersion::instance(),
    )
    .await
    .unwrap();
}
