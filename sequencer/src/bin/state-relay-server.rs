use clap::Parser;
use ethers::types::U256;
use hotshot_state_prover::service::one_honest_threshold;
use sequencer::{state_signature::relay_server::run_relay_server, SequencerApiVersion};
use sequencer_utils::logging;
use vbs::version::StaticVersionType;

#[derive(Parser)]
struct Args {
    /// Port to run the server on.
    #[arg(
        short,
        long,
        env = "ESPRESSO_STATE_RELAY_SERVER_PORT",
        default_value = "8083"
    )]
    port: u16,

    /// Total amount of stake.
    /// WARNING: this is a temporary flag, should remove after integrating with stake table.
    /// Related issue: [https://github.com/EspressoSystems/espresso-sequencer/issues/1022]
    #[arg(
        long,
        env = "ESPRESSO_STATE_SIGNATURE_TOTAL_STAKE",
        default_value = "5"
    )]
    total_stake: u64,

    #[command(flatten)]
    logging: logging::Config,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    args.logging.init();

    let threshold = one_honest_threshold(U256::from(args.total_stake));

    tracing::info!(
        port = args.port,
        "starting state relay server, quorum threshold: {threshold}"
    );
    run_relay_server(
        None,
        threshold,
        format!("http://0.0.0.0:{}", args.port).parse().unwrap(),
        SequencerApiVersion::instance(),
    )
    .await
    .unwrap();
}
