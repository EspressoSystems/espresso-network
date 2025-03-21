// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! An example program using libp2p
/// types used for this example
pub mod types;

use hotshot::helpers::initialize_logging;
use hotshot_example_types::{node_types::TestVersions, state_types::TestTypes};
use hotshot_orchestrator::client::ValidatorArgs;
use infra::{gen_local_address, BUILDER_BASE_PORT, VALIDATOR_BASE_PORT};
use tokio::spawn;
use tracing::instrument;

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

    // use configfile args
    let (config, orchestrator_url) = read_orchestrator_init_config::<TestTypes>();

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
        let builder_address = gen_local_address::<BUILDER_BASE_PORT>(i);
        let orchestrator_url = orchestrator_url.clone();
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
