// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! The whitelist is an adaptor that is able to update the allowed public keys for
//! all brokers. Right now, we do this by asking the orchestrator for the list of
//! allowed public keys. In the future, we will pull the stake table from the L1.

use std::{str::FromStr, sync::Arc};

use anyhow::{Context, Result};
use cdn_broker::reexports::discovery::{DiscoveryClient, Embedded, Redis};
use clap::Parser;
use hotshot_example_types::node_types::TestTypes;
use hotshot_orchestrator::client::OrchestratorClient;
use hotshot_types::{network::NetworkConfig, traits::signature_key::SignatureKey};
use surf_disco::Url;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// The main component of the push CDN.
struct Args {
    /// The discovery client endpoint (including scheme) to connect to.
    /// With the local discovery feature, this is a file path.
    /// With the remote (redis) discovery feature, this is a redis URL (e.g. `redis://127.0.0.1:6789`).
    #[arg(short, long)]
    discovery_endpoint: String,

    /// The URL the orchestrator is running on. This should be something like `http://localhost:5555`
    #[arg(short, long)]
    orchestrator_url: String,

    /// Whether or not to use the local discovery client
    #[arg(short, long)]
    local_discovery: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse the command line arguments
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create a new `OrchestratorClient` from the supplied URL
    let orchestrator_client = OrchestratorClient::new(
        Url::from_str(&args.orchestrator_url).with_context(|| "Invalid URL")?,
    );

    // Attempt to get the config from the orchestrator.
    // Loops internally until the config is received.
    let config: NetworkConfig<TestTypes> = orchestrator_client.get_config_after_collection().await;

    tracing::info!("Received config from orchestrator");

    // Extrapolate the state_ver_keys from the config and convert them to a compatible format
    let whitelist = config
        .config
        .known_nodes_with_stake
        .iter()
        .map(|k| Arc::from(k.stake_table_entry.stake_key.to_bytes()))
        .collect();

    if args.local_discovery {
        <Embedded as DiscoveryClient>::new(args.discovery_endpoint, None)
            .await?
            .set_whitelist(whitelist)
            .await?;
    } else {
        <Redis as DiscoveryClient>::new(args.discovery_endpoint, None)
            .await?
            .set_whitelist(whitelist)
            .await?;
    }

    tracing::info!("Posted config to discovery endpoint");

    Ok(())
}
