use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use deployment_info::{
    collect_deployment_info, load_addresses_from_env_file, write_deployment_info,
};
use tracing_subscriber::EnvFilter;
use url::Url;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Network {
    Decaf,
    Hoodi,
    Mainnet,
}

impl Network {
    fn as_str(&self) -> &'static str {
        match self {
            Network::Decaf => "decaf",
            Network::Hoodi => "hoodi",
            Network::Mainnet => "mainnet",
        }
    }

    fn default_rpc_url(&self) -> Url {
        match self {
            Network::Decaf => "https://ethereum-sepolia-rpc.publicnode.com",
            Network::Hoodi => "https://ethereum-hoodi-rpc.publicnode.com",
            Network::Mainnet => "https://ethereum-rpc.publicnode.com",
        }
        .parse()
        .expect("hardcoded URL is valid")
    }
}

fn get_crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[derive(Debug, Parser)]
#[clap(
    name = "deployment-info",
    about = "Collect and output deployment information for Espresso Network contracts"
)]
struct Args {
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_PROVIDER",
        help = "RPC URL for L1 provider. Defaults to publicnode when --network is specified."
    )]
    rpc_url: Option<Url>,

    #[clap(
        long,
        value_enum,
        help = "Known network. Provides defaults for --rpc-url, --env-file, and --output."
    )]
    network: Option<Network>,

    #[clap(
        long,
        help = "Path to input .env file. Required unless --network is specified."
    )]
    env_file: Option<PathBuf>,

    #[clap(
        long,
        help = "Output file path. Required unless --network or --stdout is specified."
    )]
    output: Option<PathBuf>,

    #[clap(long, help = "Print to stdout instead of writing to a file")]
    stdout: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let crate_dir = get_crate_dir();

    let env_file = match (&args.env_file, &args.network) {
        (Some(path), _) => path.clone(),
        (None, Some(network)) => crate_dir.join(format!("{}.env", network.as_str())),
        (None, None) => bail!("Either --network or --env-file must be specified"),
    };

    let addresses = load_addresses_from_env_file(Some(&env_file))
        .context("Failed to load addresses from env file")?;

    let rpc_url = match (&args.rpc_url, &args.network) {
        (Some(url), _) => url.clone(),
        (None, Some(network)) => network.default_rpc_url(),
        (None, None) => bail!("Either --network or --rpc-url must be specified"),
    };

    let network_name = args
        .network
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| "custom".to_string());

    tracing::info!("Collecting deployment info for network: {}", network_name);
    tracing::info!("Using RPC URL: {}", rpc_url);
    tracing::info!("Reading addresses from: {:?}", env_file);

    let info = collect_deployment_info(rpc_url, network_name.clone(), addresses)
        .await
        .context("Failed to collect deployment info")?;

    if args.stdout {
        let json = serde_json::to_string_pretty(&info)
            .context("Failed to serialize deployment info to JSON")?;
        println!("{}", json);
    } else {
        let output_path = match (&args.output, &args.network) {
            (Some(path), _) => path.clone(),
            (None, Some(network)) => crate_dir.join(format!("{}.json", network.as_str())),
            (None, None) => bail!("Either --network, --output, or --stdout must be specified"),
        };

        write_deployment_info(&info, &output_path)
            .context("Failed to write deployment info to file")?;
        tracing::info!("Successfully wrote deployment info to: {:?}", output_path);
    }

    Ok(())
}
