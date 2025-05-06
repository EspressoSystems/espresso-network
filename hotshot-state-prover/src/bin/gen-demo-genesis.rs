use alloy::{hex::ToHexExt, sol_types::SolValue};
use clap::Parser;
use espresso_contract_deployer::network_config::light_client_genesis;
use hotshot_contract_adapter::sol_types::{LightClientStateSol, StakeTableStateSol};
use hotshot_types::light_client::STAKE_TABLE_CAPACITY;
use url::Url;

#[derive(Parser)]
struct Args {
    /// URL of the HotShot orchestrator.
    #[clap(
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
    let pi: (LightClientStateSol, StakeTableStateSol) =
        light_client_genesis(&args.orchestrator_url, STAKE_TABLE_CAPACITY)
            .await
            .unwrap();
    println!("{}", pi.abi_encode_params().encode_hex());
}
