use std::path::PathBuf;

use alloy::providers::{Provider, ProviderBuilder};
use anyhow::{Context, Result};
use clap::Parser;
use deployment_info::{
    collect_deployment_info, load_addresses_from_env_file, write_deployment_info,
};
use tracing_subscriber::EnvFilter;
use url::Url;

#[derive(Debug, Parser)]
#[clap(
    name = "deployment-info",
    about = "Collect and output deployment information for Espresso Network contracts"
)]
struct Args {
    #[clap(
        long,
        env = "RPC_URL",
        help = "RPC URL for L1 provider. Defaults to publicnode for decaf/mainnet networks."
    )]
    rpc_url: Option<Url>,

    #[clap(long)]
    network: String,

    #[clap(long, help = "Path to .env file (defaults to .env)")]
    env_file: Option<PathBuf>,

    #[clap(
        long,
        help = "Output file path. If not provided, prints to stdout instead of writing to a file."
    )]
    output: Option<PathBuf>,

    #[clap(long, help = "Chain ID. If not provided, will be queried from the RPC")]
    chain_id: Option<u64>,
}

fn get_default_rpc_url(network: &str) -> Option<Url> {
    match network {
        "decaf" => "https://ethereum-sepolia-rpc.publicnode.com".parse().ok(),
        "hoodi" => "https://ethereum-hoodi-rpc.publicnode.com".parse().ok(),
        "mainnet" => "https://ethereum-rpc.publicnode.com".parse().ok(),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let addresses = load_addresses_from_env_file(args.env_file.as_deref())
        .context("Failed to load addresses from env file")?;

    let rpc_url = args
        .rpc_url
        .or_else(|| get_default_rpc_url(&args.network))
        .context(
            "RPC URL not provided and no default available for this network. Provide --rpc-url or \
             set ESPRESSO_SEQUENCER_L1_PROVIDER",
        )?;

    let chain_id = if let Some(id) = args.chain_id {
        id
    } else {
        let provider = ProviderBuilder::new().connect_http(rpc_url.clone());
        provider
            .get_chain_id()
            .await
            .context("Failed to query chain ID from RPC")?
    };

    let sanitized_url = format!(
        "{}://{}/..",
        rpc_url.scheme(),
        rpc_url.host_str().unwrap_or_default()
    );
    tracing::info!("Collecting deployment info for network: {}", args.network);
    tracing::info!("Using RPC: {}", sanitized_url);
    tracing::info!("Chain ID: {}", chain_id);

    let info = collect_deployment_info(rpc_url, addresses, chain_id)
        .await
        .context("Failed to collect deployment info")?;

    if let Some(output_path) = args.output {
        write_deployment_info(&info, &output_path)
            .context("Failed to write deployment info to file")?;
        tracing::info!("Successfully wrote deployment info to: {:?}", output_path);
    } else {
        let json = serde_json::to_string_pretty(&info)
            .context("Failed to serialize deployment info to JSON")?;
        println!("{}", json);
    }

    Ok(())
}
