//! The following is the main `Marshal` binary, which just instantiates and runs
//! a `Marshal` object.

use anyhow::{Context, Result};
use cdn_marshal::{Config, Marshal};
use clap::Parser;
use espresso_types::{SeqTypes, parse_size};
use sequencer::network::cdn::ProductionDef;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// The main component of the push CDN.
struct Args {
    /// The discovery client endpoint (including scheme) to connect to
    #[arg(short, long, env = "ESPRESSO_CDN_MARSHAL_DISCOVERY_ENDPOINT")]
    discovery_endpoint: String,

    /// The port to bind to for connections (from users)
    #[arg(
        short,
        long,
        default_value_t = 1737,
        env = "ESPRESSO_CDN_MARSHAL_BIND_PORT"
    )]
    bind_port: u16,

    /// The endpoint to bind to for externalizing metrics (in `IP:port` form). If not provided,
    /// metrics are not exposed.
    #[arg(short, long, env = "ESPRESSO_CDN_MARSHAL_METRICS_BIND_ENDPOINT")]
    metrics_bind_endpoint: Option<String>,

    /// The path to the CA certificate
    /// If not provided, a local, pinned CA is used
    #[arg(long, env = "ESPRESSO_CDN_MARSHAL_CA_CERT_PATH")]
    ca_cert_path: Option<String>,

    /// The path to the CA key
    /// If not provided, a local, pinned CA is used
    #[arg(long, env = "ESPRESSO_CDN_MARSHAL_CA_KEY_PATH")]
    ca_key_path: Option<String>,

    /// The size of the global memory pool. This is the maximum number of bytes that
    /// can be allocated at once for all connections. A connection will block if it
    /// tries to allocate more than this amount until some memory is freed.
    #[arg(
        long,
        default_value = "1GB",
        value_parser = parse_size,
        env = "ESPRESSO_CDN_MARSHAL_GLOBAL_MEMORY_POOL_SIZE"
    )]
    global_memory_pool_size: u64,
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

    // Cast the memory pool size to a `usize`
    let global_memory_pool_size =
        usize::try_from(args.global_memory_pool_size).with_context(|| {
            format!(
                "Failed to convert global memory pool size to usize: {}",
                args.global_memory_pool_size
            )
        })?;

    // Create a new `Config`
    let config = Config {
        discovery_endpoint: args.discovery_endpoint,
        bind_endpoint: format!("0.0.0.0:{}", args.bind_port),
        metrics_bind_endpoint: args.metrics_bind_endpoint,
        ca_cert_path: args.ca_cert_path,
        ca_key_path: args.ca_key_path,
        global_memory_pool_size: Some(global_memory_pool_size),
    };

    // Create new `Marshal` from the config
    let marshal = Marshal::<ProductionDef<SeqTypes>>::new(config).await?;

    // Start the main loop, consuming it
    marshal.start().await?;

    Ok(())
}
