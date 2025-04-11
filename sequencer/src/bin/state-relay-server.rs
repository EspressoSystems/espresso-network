use clap::Parser;
use hotshot_stake_table::config::STAKE_TABLE_CAPACITY;
use sequencer::{state_signature::relay_server::run_relay_server, SequencerApiVersion};
use sequencer_utils::logging;
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

    run_relay_server(
        None,
        args.sequencer_url,
        format!("http://0.0.0.0:{}", args.port).parse().unwrap(),
        SequencerApiVersion::instance(),
    )
    .await
    .unwrap();
}
