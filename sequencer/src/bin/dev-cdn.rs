//! The following is the main `Broker` binary, which just instantiates and runs
//! a `Broker` object.

use std::path::Path;

use anyhow::Result;
use cdn_broker::{
    Broker, Config as BrokerConfig,
    reexports::{crypto::signature::KeyPair, def::hook::NoMessageHook},
};
use cdn_marshal::{Config as MarshalConfig, Marshal};
use clap::Parser;
use espresso_types::SeqTypes;
use hotshot_types::traits::{node_implementation::NodeType, signature_key::SignatureKey};
use portpicker::pick_unused_port;
use rand::{RngCore, SeedableRng, rngs::StdRng};
use sequencer::network::cdn::{TestingDef, WrappedSignatureKey};
use tokio::spawn;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// A minified development version of the CDN
struct Args {
    /// The port we should be accessible on
    #[arg(short, long, default_value = "1738")]
    port: u16,
}

#[tokio::main]
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
    let (public_key, private_key) =
        <SeqTypes as NodeType>::SignatureKey::generated_from_seed_indexed([0; 32], 1337);

    // Get the OS temporary directory
    let temp_dir = std::env::temp_dir();

    // Create an SQLite file inside of the temporary directory
    let discovery_endpoint = temp_dir
        .join(Path::new(&format!(
            "test-{}.sqlite",
            StdRng::from_entropy().next_u64()
        )))
        .to_string_lossy()
        .into_owned();

    // Acquire unused ports for the broker to use
    let broker_public_port = pick_unused_port().expect("failed to find free port for broker");
    let broker_private_port = pick_unused_port().expect("failed to find free port for broker");

    // Configure the broker
    let broker_config: BrokerConfig<TestingDef<SeqTypes>> = BrokerConfig {
        public_advertise_endpoint: format!("127.0.0.1:{broker_public_port}"),
        public_bind_endpoint: format!("127.0.0.1:{broker_public_port}"),
        private_advertise_endpoint: format!("127.0.0.1:{broker_private_port}"),
        private_bind_endpoint: format!("127.0.0.1:{broker_private_port}"),

        metrics_bind_endpoint: None,
        discovery_endpoint: discovery_endpoint.clone(),
        keypair: KeyPair {
            public_key: WrappedSignatureKey(public_key),
            private_key,
        },

        user_message_hook: NoMessageHook,
        broker_message_hook: NoMessageHook,

        ca_cert_path: None,
        ca_key_path: None,
        global_memory_pool_size: Some(1024 * 1024 * 1024),
    };

    // Configure the marshal
    let marshal_config = MarshalConfig {
        bind_endpoint: format!("127.0.0.1:{}", args.port),
        metrics_bind_endpoint: None,
        discovery_endpoint: discovery_endpoint.clone(),
        ca_cert_path: None,
        ca_key_path: None,
        global_memory_pool_size: Some(1024 * 1024 * 1024),
    };

    // Create a new `Broker`
    // Uses TCP for broker connections and Quic for user connections.
    let broker = Broker::new(broker_config).await?;

    // Create a new `Marshal`
    // Uses TCP for broker connections and Quic for user connections.
    let marshal = Marshal::<TestingDef<SeqTypes>>::new(marshal_config).await?;

    // Spawn the tasks
    let broker_jh = spawn(broker.start());
    let marshal_jh = spawn(marshal.start());

    // Await on both
    let _ = broker_jh.await;
    let _ = marshal_jh.await;

    Ok(())
}
