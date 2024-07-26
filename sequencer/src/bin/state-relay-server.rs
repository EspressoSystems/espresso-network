use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
use clap::Parser;
use es_version::SEQUENCER_VERSION;
use ethers::types::U256;
use hotshot_state_prover::service::one_honest_threshold;
use sequencer::state_signature::relay_server::run_relay_server;

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

    /// Total amount of stake.
    /// WARNING: this is a temporary flag, should remove after integrating with stake table.
    /// Related issue: [https://github.com/EspressoSystems/espresso-sequencer/issues/1022]
    #[clap(
        long,
        env = "ESPRESSO_STATE_SIGNATURE_TOTAL_STAKE",
        default_value = "5"
    )]
    total_stake: u64,
}

#[async_std::main]
async fn main() {
    setup_logging();
    setup_backtrace();

    let args = Args::parse();
    let threshold = one_honest_threshold(U256::from(args.total_stake));

    tracing::info!(
        port = args.port,
        "starting state relay server, quorum threshold: {threshold}"
    );
    run_relay_server(
        None,
        threshold,
        format!("http://0.0.0.0:{}", args.port).parse().unwrap(),
        SEQUENCER_VERSION,
    )
    .await
    .unwrap();
}
