use clap::Parser;
use ethers::abi::AbiEncode;
use hotshot_stake_table::config::STAKE_TABLE_CAPACITY;
use hotshot_state_prover::service::light_client_genesis;
use url::Url;

#[derive(Parser)]
struct Args {
    /// URL of the HotShot orchestrator.
    #[arg(
        short,
        long,
        env = "ESPRESSO_SEQUENCER_ORCHESTRATOR_URL",
        default_value = "http://localhost:8080"
    )]
    pub orchestrator_url: Url,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let pi = light_client_genesis(&args.orchestrator_url, STAKE_TABLE_CAPACITY)
        .await
        .unwrap();
    println!("{}", pi.encode_hex());
}
