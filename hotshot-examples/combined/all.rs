// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! An example program using both the web server and libp2p
/// types used for this example
pub mod types;

use std::path::Path;

use cdn_broker::{reexports::def::hook::NoMessageHook, Broker};
use cdn_marshal::Marshal;
use hotshot::{
    helpers::initialize_logging,
    traits::implementations::{KeyPair, TestingDef, WrappedSignatureKey},
    types::SignatureKey,
};
use hotshot_example_types::{node_types::TestVersions, state_types::TestTypes};
use hotshot_orchestrator::client::ValidatorArgs;
use hotshot_types::traits::node_implementation::NodeType;
use infra::{gen_local_address, BUILDER_BASE_PORT, VALIDATOR_BASE_PORT};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use tokio::spawn;
use tracing::{error, instrument};

use crate::{
    infra::{read_orchestrator_init_config, run_orchestrator, OrchestratorArgs},
    types::{Network, NodeImpl, ThisRun},
};

/// general infra used for this example
#[path = "../infra/mod.rs"]
pub mod infra;

#[tokio::main]
#[instrument]
async fn main() {
    // Initialize logging
    initialize_logging();

    let (config, orchestrator_url) = read_orchestrator_init_config::<TestTypes>();

    // The configuration we are using for testing is 2 brokers & 1 marshal
    // A keypair shared between brokers
    let (broker_public_key, broker_private_key) =
        <TestTypes as NodeType>::SignatureKey::generated_from_seed_indexed([0u8; 32], 1337);

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

    // 2 brokers
    for _ in 0..2 {
        // Get the ports to bind to
        let private_port = portpicker::pick_unused_port().expect("could not find an open port");
        let public_port = portpicker::pick_unused_port().expect("could not find an open port");

        // Extrapolate addresses
        let private_address = format!("127.0.0.1:{private_port}");
        let public_address = format!("127.0.0.1:{public_port}");

        let config: cdn_broker::Config<TestingDef<<TestTypes as NodeType>::SignatureKey>> =
            cdn_broker::Config {
                discovery_endpoint: discovery_endpoint.clone(),
                public_advertise_endpoint: public_address.clone(),
                public_bind_endpoint: public_address,
                private_advertise_endpoint: private_address.clone(),
                private_bind_endpoint: private_address,

                keypair: KeyPair {
                    public_key: WrappedSignatureKey(broker_public_key),
                    private_key: broker_private_key.clone(),
                },

                user_message_hook: NoMessageHook,
                broker_message_hook: NoMessageHook,

                metrics_bind_endpoint: None,
                ca_cert_path: None,
                ca_key_path: None,
                global_memory_pool_size: Some(1024 * 1024 * 1024),
            };

        // Create and spawn the broker
        spawn(async move {
            let broker: Broker<TestingDef<<TestTypes as NodeType>::SignatureKey>> =
                Broker::new(config).await.expect("broker failed to start");

            // Error if we stopped unexpectedly
            if let Err(err) = broker.start().await {
                error!("broker stopped: {err}");
            }
        });
    }

    // Get the port to use for the marshal
    let marshal_endpoint = config
        .cdn_marshal_address
        .clone()
        .expect("CDN marshal address must be specified");

    // Configure the marshal
    let marshal_config = cdn_marshal::Config {
        bind_endpoint: marshal_endpoint.clone(),
        discovery_endpoint,
        metrics_bind_endpoint: None,
        ca_cert_path: None,
        ca_key_path: None,
        global_memory_pool_size: Some(1024 * 1024 * 1024),
    };

    // Spawn the marshal
    spawn(async move {
        let marshal: Marshal<TestingDef<<TestTypes as NodeType>::SignatureKey>> =
            Marshal::new(marshal_config)
                .await
                .expect("failed to spawn marshal");

        // Error if we stopped unexpectedly
        if let Err(err) = marshal.start().await {
            error!("broker stopped: {err}");
        }
    });

    // orchestrator
    spawn(run_orchestrator::<TestTypes>(OrchestratorArgs {
        url: orchestrator_url.clone(),
        config: config.clone(),
    }));

    // nodes
    let mut nodes = Vec::new();
    for i in 0..config.config.num_nodes_with_stake.into() {
        // Calculate our libp2p advertise address, which we will later derive the
        // bind address from for example purposes.
        let advertise_address = gen_local_address::<VALIDATOR_BASE_PORT>(i);
        let orchestrator_url = orchestrator_url.clone();
        let builder_address = gen_local_address::<BUILDER_BASE_PORT>(i);

        let node = spawn(async move {
            infra::main_entry_point::<TestTypes, Network, NodeImpl, TestVersions, ThisRun>(
                ValidatorArgs {
                    url: orchestrator_url,
                    advertise_address: Some(advertise_address.to_string()),
                    builder_address: Some(builder_address),
                    network_config_file: None,
                },
            )
            .await;
        });
        nodes.push(node);
    }
    futures::future::join_all(nodes).await;
}
