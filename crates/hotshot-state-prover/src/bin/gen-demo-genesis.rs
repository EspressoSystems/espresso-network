use alloy::{hex::ToHexExt, sol_types::SolValue};
use clap::Parser;
use espresso_contract_deployer::network_config::light_client_genesis;
use hotshot_contract_adapter::sol_types::{LightClientStateSol, StakeTableStateSol};
use hotshot_types::light_client::DEFAULT_STAKE_TABLE_CAPACITY;
use url::Url;

#[derive(Parser)]
struct Args {
    /// URL of the HotShot orchestrator.
    #[clap(
        short,
        long,
        env = "ESPRESSO_NODE_ORCHESTRATOR_URL",
        default_value = "http://localhost:8080"
    )]
    pub orchestrator_url: Url,
}

fn main() {
    let migrated_envs = espresso_utils::env_compat::migrate_legacy_env_vars();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async_main(migrated_envs))
}

// `migrated_envs` unused because there's no logging in this binary.
// We have already printed warnings before.
async fn async_main(_migrated_envs: Vec<(&str, &str)>) {
    let args = Args::parse();
    let pi: (LightClientStateSol, StakeTableStateSol) =
        light_client_genesis(&args.orchestrator_url, DEFAULT_STAKE_TABLE_CAPACITY)
            .await
            .unwrap();
    println!("{}", pi.abi_encode_params().encode_hex());
}
