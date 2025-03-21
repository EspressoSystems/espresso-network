// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! The following is the main `Broker` binary, which just instantiates and runs
//! a `Broker` object.
use anyhow::Result;
use cdn_broker::{reexports::def::hook::NoMessageHook, Broker, Config};
use clap::Parser;
use hotshot::traits::implementations::{KeyPair, ProductionDef, WrappedSignatureKey};
use hotshot_example_types::node_types::TestTypes;
use hotshot_types::traits::{node_implementation::NodeType, signature_key::SignatureKey};
use sha2::Digest;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// The main component of the push CDN.
struct Args {
    /// The discovery client endpoint (including scheme) to connect to.
    /// With the local discovery feature, this is a file path.
    /// With the remote (redis) discovery feature, this is a redis URL (e.g. `redis://127.0.0.1:6789`).
    #[arg(short, long)]
    discovery_endpoint: String,

    /// The user-facing endpoint in `IP:port` form to bind to for connections from users
    #[arg(long, default_value = "0.0.0.0:1738")]
    public_bind_endpoint: String,

    /// The user-facing endpoint in `IP:port` form to advertise
    #[arg(long, default_value = "local_ip:1738")]
    public_advertise_endpoint: String,

    /// The broker-facing endpoint in `IP:port` form to bind to for connections from
    /// other brokers
    #[arg(long, default_value = "0.0.0.0:1739")]
    private_bind_endpoint: String,

    /// The broker-facing endpoint in `IP:port` form to advertise
    #[arg(long, default_value = "local_ip:1739")]
    private_advertise_endpoint: String,

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

    /// The seed for broker key generation
    #[arg(short, long, default_value_t = 0)]
    key_seed: u64,

    /// The size of the global memory pool (in bytes). This is the maximum number of bytes that
    /// can be allocated at once for all connections. A connection will block if it
    /// tries to allocate more than this amount until some memory is freed.
    /// Default is 1GB.
    #[arg(long, default_value_t = 1_073_741_824)]
    global_memory_pool_size: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
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

    // Generate the broker key from the supplied seed
    let key_hash = sha2::Sha256::digest(args.key_seed.to_le_bytes());
    let (public_key, private_key) =
        <TestTypes as NodeType>::SignatureKey::generated_from_seed_indexed(key_hash.into(), 1337);

    // Create config
    let broker_config: Config<ProductionDef<<TestTypes as NodeType>::SignatureKey>> = Config {
        ca_cert_path: args.ca_cert_path,
        ca_key_path: args.ca_key_path,

        discovery_endpoint: args.discovery_endpoint,
        metrics_bind_endpoint: args.metrics_bind_endpoint,
        keypair: KeyPair {
            public_key: WrappedSignatureKey(public_key),
            private_key,
        },

        user_message_hook: NoMessageHook,
        broker_message_hook: NoMessageHook,

        public_bind_endpoint: args.public_bind_endpoint,
        public_advertise_endpoint: args.public_advertise_endpoint,
        private_bind_endpoint: args.private_bind_endpoint,
        private_advertise_endpoint: args.private_advertise_endpoint,
        global_memory_pool_size: Some(args.global_memory_pool_size),
    };

    // Create new `Broker`
    // Uses TCP from broker connections and TCP+TLS for user connections.
    let broker = Broker::new(broker_config).await?;

    // Start the main loop, consuming it
    broker.start().await?;

    Ok(())
}
