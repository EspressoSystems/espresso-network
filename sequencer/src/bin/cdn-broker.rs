//! The following is the main `Broker` binary, which just instantiates and runs
//! a `Broker` object.
use anyhow::Result;
use cdn_broker::reexports::crypto::signature::KeyPair;
use cdn_broker::{Broker, Config};
use clap::Parser;
use hotshot_types::traits::node_implementation::NodeType;
use hotshot_types::traits::signature_key::SignatureKey;
use sequencer::network::cdn::{ProductionDef, WrappedSignatureKey};
use sequencer::SeqTypes;
use sha2::Digest;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// The main component of the push CDN.
struct Args {
    /// The discovery client endpoint (including scheme) to connect to.
    /// With the local discovery feature, this is a file path.
    /// With the remote (redis) discovery feature, this is a redis URL (e.g. `redis://127.0.0.1:6789`).
    #[arg(short, long, env = "ESPRESSO_CDN_BROKER_DISCOVERY_ENDPOINT")]
    discovery_endpoint: String,

    /// The user-facing endpoint in `IP:port` form to bind to for connections from users
    #[arg(
        long,
        default_value = "0.0.0.0:1738",
        env = "ESPRESSO_CDN_BROKER_PUBLIC_BIND_ENDPOINT"
    )]
    public_bind_endpoint: String,

    /// The user-facing endpoint in `IP:port` form to advertise
    #[arg(
        long,
        default_value = "local_ip:1738",
        env = "ESPRESSO_CDN_BROKER_PUBLIC_ADVERTISE_ENDPOINT"
    )]
    public_advertise_endpoint: String,

    /// The broker-facing endpoint in `IP:port` form to bind to for connections from  
    /// other brokers
    #[arg(
        long,
        default_value = "0.0.0.0:1739",
        env = "ESPRESSO_CDN_BROKER_PRIVATE_BIND_ENDPOINT"
    )]
    private_bind_endpoint: String,

    /// The broker-facing endpoint in `IP:port` form to advertise
    #[arg(
        long,
        default_value = "local_ip:1739",
        env = "ESPRESSO_CDN_BROKER_PRIVATE_ADVERTISE_ENDPOINT"
    )]
    private_advertise_endpoint: String,

    /// The endpoint to bind to for externalizing metrics (in `IP:port` form). If not provided,
    /// metrics are not exposed.
    #[arg(short, long, env = "ESPRESSO_CDN_BROKER_METRICS_BIND_ENDPOINT")]
    metrics_bind_endpoint: Option<String>,

    /// The path to the CA certificate
    /// If not provided, a local, pinned CA is used
    #[arg(long, env = "ESPRESSO_CDN_BROKER_CA_CERT_PATH")]
    ca_cert_path: Option<String>,

    /// The path to the CA key
    /// If not provided, a local, pinned CA is used
    #[arg(long, env = "ESPRESSO_CDN_BROKER_CA_KEY_PATH")]
    ca_key_path: Option<String>,

    /// The seed for broker key generation
    #[arg(short, long, default_value_t = 0)]
    key_seed: u64,
}
#[async_std::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize tracing
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        tracing_subscriber::fmt().json().init();
    } else {
        tracing_subscriber::fmt().init();
    }

    // Generate the broker key from the supplied seed
    let key_hash = sha2::Sha256::digest(args.key_seed.to_le_bytes());
    let (public_key, private_key) =
        <SeqTypes as NodeType>::SignatureKey::generated_from_seed_indexed(key_hash.into(), 1337);

    // Create config
    let broker_config: Config<ProductionDef<SeqTypes>> = Config {
        ca_cert_path: args.ca_cert_path,
        ca_key_path: args.ca_key_path,

        discovery_endpoint: args.discovery_endpoint,
        metrics_bind_endpoint: args.metrics_bind_endpoint,
        keypair: KeyPair {
            public_key: WrappedSignatureKey(public_key),
            private_key,
        },

        public_bind_endpoint: args.public_bind_endpoint,
        public_advertise_endpoint: args.public_advertise_endpoint,
        private_bind_endpoint: args.private_bind_endpoint,
        private_advertise_endpoint: args.private_advertise_endpoint,
    };

    // Create new `Broker`
    // Uses TCP from broker connections and Quic for user connections.
    let broker = Broker::new(broker_config).await?;

    // Start the main loop, consuming it
    broker.start().await?;

    Ok(())
}
