// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! A validator using both the web server and libp2p

use clap::Parser;
use hotshot::helpers::initialize_logging;
use hotshot_example_types::{node_types::TestVersions, state_types::TestTypes};
use hotshot_orchestrator::client::ValidatorArgs;
use local_ip_address::local_ip;
use tracing::{debug, instrument};

use crate::types::{Network, NodeImpl, ThisRun};

/// types used for this example
pub mod types;

/// general infra used for this example
#[path = "../infra/mod.rs"]
pub mod infra;

#[tokio::main]
#[instrument]
async fn main() {
    // Initialize logging
    initialize_logging();

    let mut args = ValidatorArgs::parse();

    // If we did not set the advertise address, use our local IP and port 8000
    let local_ip = local_ip().expect("failed to get local IP");
    args.advertise_address = Some(args.advertise_address.unwrap_or(format!("{local_ip}:8000")));

    debug!("connecting to orchestrator at {:?}", args.url);
    infra::main_entry_point::<TestTypes, Network, NodeImpl, TestVersions, ThisRun>(args).await;
}
