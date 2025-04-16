// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! The following is the main `Marshal` binary, which just instantiates and runs
//! a `Marshal` object.

use anyhow::Result;
use cdn_marshal::{Config, Marshal};
use clap::Parser;
use hotshot::traits::implementations::ProductionDef;
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::traits::node_implementation::NodeType;
use tracing_subscriber::EnvFilter;

// TODO: forall, add logging where we need it

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// The main component of the push CDN.
struct Args {
    /// The discovery client endpoint (including scheme) to connect to
    #[arg(short, long)]
    discovery_endpoint: String,

    /// The port to bind to for connections (from users)
    #[arg(short, long, default_value_t = 1737)]
    bind_port: u16,

    /// The endpoint to bind to for externalizing metrics (in `IP:port` form). If not provided,
    /// metrics are not exposed.
    #[arg(short, long)]
    metrics_bind_endpoint: Option<String>,

    /// The path to the CA certificate
    /// If not provided, a local, pinned CA is used
    #[arg(long)]
    ca_cert_path: Option<String>,

    /// The path to the CA key
    /// If not provided, a local, pinned CA is used
    #[arg(long)]
    ca_key_path: Option<String>,

    /// The size of the global memory pool (in bytes). This is the maximum number of bytes that
    /// can be allocated at once for all connections. A connection will block if it
    /// tries to allocate more than this amount until some memory is freed.
    /// Default is 1GB.
    #[arg(long, default_value_t = 1_073_741_824)]
    global_memory_pool_size: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize tracing
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    }

    // Create a new `Config`
    let config = Config {
        discovery_endpoint: args.discovery_endpoint,
        bind_endpoint: format!("0.0.0.0:{}", args.bind_port),
        metrics_bind_endpoint: args.metrics_bind_endpoint,
        ca_cert_path: args.ca_cert_path,
        ca_key_path: args.ca_key_path,
        global_memory_pool_size: Some(args.global_memory_pool_size),
    };

    // Create new `Marshal` from the config
    let marshal =
        Marshal::<ProductionDef<<TestTypes as NodeType>::SignatureKey>>::new(config).await?;

    // Start the main loop, consuming it
    marshal.start().await?;

    Ok(())
}
