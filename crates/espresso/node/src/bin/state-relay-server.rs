use clap::Parser;
use espresso_node::{SequencerApiVersion, state_signature::relay_server::run_relay_server};
use espresso_utils::logging;
use hotshot_types::light_client::DEFAULT_STAKE_TABLE_CAPACITY;
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
        env = "ESPRESSO_API_NODE_URL",
        default_value = "http://localhost:24000"
    )]
    pub sequencer_url: Url,

    /// Stake table capacity for the prover circuit
    #[clap(short, long, env = "ESPRESSO_STAKE_TABLE_CAPACITY", default_value_t = DEFAULT_STAKE_TABLE_CAPACITY)]
    pub stake_table_capacity: usize,

    #[clap(flatten)]
    logging: logging::Config,
}

fn main() {
    let migrated_envs = espresso_utils::env_compat::migrate_legacy_env_vars();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async_main(migrated_envs))
}

async fn async_main(migrated_envs: Vec<(&str, &str)>) {
    let args = Args::parse();
    args.logging.init();
    espresso_utils::env_compat::log_migrated_env_vars(&migrated_envs);

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
