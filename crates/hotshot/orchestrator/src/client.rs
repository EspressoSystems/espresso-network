// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    time::Duration,
};

use clap::Parser;
use futures::{Future, FutureExt};
use hotshot_types::{
    benchmarking::{LeaderViewStats, ReplicaViewStats},
    network::{NetworkConfig, NetworkConfigSource},
    traits::node_implementation::{ConsensusTime, NodeType},
    PeerConfig, ValidatorConfig,
};
use libp2p_identity::PeerId;
use multiaddr::Multiaddr;
use serde::{Deserialize, Serialize};
use surf_disco::{error::ClientError, Client};
use tide_disco::Url;
use tokio::time::sleep;
use tracing::{info, instrument};
use vbs::BinarySerializer;

use crate::OrchestratorVersion;

/// Holds the client connection to the orchestrator
pub struct OrchestratorClient {
    /// the client
    pub client: surf_disco::Client<ClientError, OrchestratorVersion>,
}

/// Struct describing a benchmark result
#[derive(Serialize, Deserialize, Clone)]
pub struct BenchResults<V: ConsensusTime> {
    pub node_index: u64,
    #[serde(bound(deserialize = "V: ConsensusTime"))]
    pub leader_view_stats: BTreeMap<V, LeaderViewStats<V>>,
    #[serde(bound(deserialize = "V: ConsensusTime"))]
    pub replica_view_stats: BTreeMap<V, ReplicaViewStats<V>>,
    #[serde(bound(deserialize = "V: ConsensusTime"))]
    pub latencies_by_view: BTreeMap<V, i128>,
    #[serde(bound(deserialize = "V: ConsensusTime"))]
    pub sizes_by_view: BTreeMap<V, i128>,
    #[serde(bound(deserialize = "V: ConsensusTime"))]
    pub timeouts: BTreeSet<V>,
    pub total_time_millis: i128,
}

impl<V: ConsensusTime> Default for BenchResults<V> {
    fn default() -> Self {
        Self {
            node_index: 0,
            leader_view_stats: BTreeMap::new(),
            replica_view_stats: BTreeMap::new(),
            latencies_by_view: BTreeMap::new(),
            sizes_by_view: BTreeMap::new(),
            timeouts: BTreeSet::new(),
            total_time_millis: 0,
        }
    }
}
// VALIDATOR

#[derive(Parser, Debug, Clone)]
#[command(
    name = "Multi-machine consensus",
    about = "Simulates consensus among multiple machines"
)]
/// Arguments passed to the validator
pub struct ValidatorArgs {
    /// The address the orchestrator runs on
    pub url: Url,
    /// The optional advertise address to use for Libp2p
    pub advertise_address: Option<String>,
    /// Optional address to run builder on. Address must be accessible by other nodes
    pub builder_address: Option<SocketAddr>,
    /// An optional network config file to save to/load from
    /// Allows for rejoining the network on a complete state loss
    #[arg(short, long)]
    pub network_config_file: Option<String>,
}

/// arguments to run multiple validators
#[derive(Parser, Debug, Clone)]
pub struct MultiValidatorArgs {
    /// Number of validators to run
    pub num_nodes: u16,
    /// The address the orchestrator runs on
    pub url: Url,
    /// The optional advertise address to use for Libp2p
    pub advertise_address: Option<String>,
    /// An optional network config file to save to/load from
    /// Allows for rejoining the network on a complete state loss
    #[arg(short, long)]
    pub network_config_file: Option<String>,
}

/// Asynchronously retrieves a `NetworkConfig` from an orchestrator.
/// The retrieved one includes correct `node_index` and peer's public config.
///
/// # Errors
/// If we are unable to get the configuration from the orchestrator
pub async fn get_complete_config<TYPES: NodeType>(
    client: &OrchestratorClient,
    mut validator_config: ValidatorConfig<TYPES>,
    libp2p_advertise_address: Option<Multiaddr>,
    libp2p_public_key: Option<PeerId>,
) -> anyhow::Result<(
    NetworkConfig<TYPES>,
    ValidatorConfig<TYPES>,
    NetworkConfigSource,
)> {
    // get the configuration from the orchestrator
    let run_config: NetworkConfig<TYPES> = client
        .post_and_wait_all_public_keys::<TYPES>(
            &mut validator_config,
            libp2p_advertise_address,
            libp2p_public_key,
        )
        .await;

    info!(
        "Retrieved config; our node index is {}. DA committee member: {}",
        run_config.node_index, validator_config.is_da
    );
    Ok((
        run_config,
        validator_config,
        NetworkConfigSource::Orchestrator,
    ))
}

impl ValidatorArgs {
    /// Constructs `ValidatorArgs` from `MultiValidatorArgs` and a node index.
    ///
    /// If `network_config_file` is present in `MultiValidatorArgs`, it appends the node index to it to create a unique file name for each node.
    ///
    /// # Arguments
    ///
    /// * `multi_args` - A `MultiValidatorArgs` instance containing the base arguments for the construction.
    /// * `node_index` - A `u16` representing the index of the node for which the args are being constructed.
    ///
    /// # Returns
    ///
    /// This function returns a new instance of `ValidatorArgs`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // NOTE this is a toy example,
    /// // the user will need to construct a multivalidatorargs since `new` does not exist
    /// # use hotshot_orchestrator::client::MultiValidatorArgs;
    /// let multi_args = MultiValidatorArgs::new();
    /// let node_index = 1;
    /// let instance = Self::from_multi_args(multi_args, node_index);
    /// ```
    #[must_use]
    pub fn from_multi_args(multi_args: MultiValidatorArgs, node_index: u16) -> Self {
        Self {
            url: multi_args.url,
            advertise_address: multi_args.advertise_address,
            builder_address: None,
            network_config_file: multi_args
                .network_config_file
                .map(|s| format!("{s}-{node_index}")),
        }
    }
}

impl OrchestratorClient {
    /// Creates the client that will connect to the orchestrator
    #[must_use]
    pub fn new(url: Url) -> Self {
        let client = surf_disco::Client::<ClientError, OrchestratorVersion>::new(url);
        // TODO ED: Add healthcheck wait here
        OrchestratorClient { client }
    }

    /// Get the config from the orchestrator.
    /// If the identity is provided, register the identity with the orchestrator.
    /// If not, just retrieving the config (for passive observers)
    ///
    /// # Panics
    /// if unable to convert the node index from usize into u64
    /// (only applicable on 32 bit systems)
    ///
    /// # Errors
    /// If we were unable to serialize the Libp2p data
    #[allow(clippy::type_complexity)]
    pub async fn get_config_without_peer<TYPES: NodeType>(
        &self,
        libp2p_advertise_address: Option<Multiaddr>,
        libp2p_public_key: Option<PeerId>,
    ) -> anyhow::Result<NetworkConfig<TYPES>> {
        // Serialize our (possible) libp2p-specific data
        let request_body = vbs::Serializer::<OrchestratorVersion>::serialize(&(
            libp2p_advertise_address,
            libp2p_public_key,
        ))?;

        let identity = |client: Client<ClientError, OrchestratorVersion>| {
            // We need to clone here to move it into the closure
            let request_body = request_body.clone();
            async move {
                let node_index: Result<u16, ClientError> = client
                    .post("api/identity")
                    .body_binary(&request_body)
                    .expect("failed to set request body")
                    .send()
                    .await;

                node_index
            }
            .boxed()
        };
        let node_index = self.wait_for_fn_from_orchestrator(identity).await;

        // get the corresponding config
        let f = |client: Client<ClientError, OrchestratorVersion>| {
            async move {
                let config: Result<NetworkConfig<TYPES>, ClientError> = client
                    .post(&format!("api/config/{node_index}"))
                    .send()
                    .await;
                config
            }
            .boxed()
        };

        let mut config = self.wait_for_fn_from_orchestrator(f).await;
        config.node_index = From::<u16>::from(node_index);

        Ok(config)
    }

    /// Post to the orchestrator and get the latest `node_index`
    /// Then return it for the init validator config
    /// # Panics
    /// if unable to post
    #[instrument(skip_all, name = "orchestrator node index for validator config")]
    pub async fn get_node_index_for_init_validator_config(&self) -> u16 {
        let cur_node_index = |client: Client<ClientError, OrchestratorVersion>| {
            async move {
                let cur_node_index: Result<u16, ClientError> = client
                    .post("api/get_tmp_node_index")
                    .send()
                    .await
                    .inspect_err(|err| tracing::error!("{err}"));

                cur_node_index
            }
            .boxed()
        };
        self.wait_for_fn_from_orchestrator(cur_node_index).await
    }

    /// Requests the configuration from the orchestrator with the stipulation that
    /// a successful call requires all nodes to be registered.
    ///
    /// Does not fail, retries internally until success.
    #[instrument(skip_all, name = "orchestrator config")]
    pub async fn get_config_after_collection<TYPES: NodeType>(&self) -> NetworkConfig<TYPES> {
        // Define the request for post-register configurations
        let get_config_after_collection = |client: Client<ClientError, OrchestratorVersion>| {
            async move {
                let result = client
                    .post("api/post_config_after_peer_collected")
                    .send()
                    .await;

                if let Err(ref err) = result {
                    tracing::error!("{err}");
                }

                result
            }
            .boxed()
        };

        // Loop until successful
        self.wait_for_fn_from_orchestrator(get_config_after_collection)
            .await
    }

    /// Registers a builder URL with the orchestrator
    ///
    /// # Panics
    /// if unable to serialize `address`
    pub async fn post_builder_addresses(&self, addresses: Vec<Url>) {
        let send_builder_f = |client: Client<ClientError, OrchestratorVersion>| {
            let request_body = vbs::Serializer::<OrchestratorVersion>::serialize(&addresses)
                .expect("Failed to serialize request");

            async move {
                let result: Result<_, ClientError> = client
                    .post("api/builder")
                    .body_binary(&request_body)
                    .unwrap()
                    .send()
                    .await
                    .inspect_err(|err| tracing::error!("{err}"));
                result
            }
            .boxed()
        };
        self.wait_for_fn_from_orchestrator::<_, _, ()>(send_builder_f)
            .await;
    }

    /// Requests a builder URL from orchestrator
    pub async fn get_builder_addresses(&self) -> Vec<Url> {
        // Define the request for post-register configurations
        let get_builder = |client: Client<ClientError, OrchestratorVersion>| {
            async move {
                let result = client.get("api/builders").send().await;

                if let Err(ref err) = result {
                    tracing::error!("{err}");
                }

                result
            }
            .boxed()
        };

        // Loop until successful
        self.wait_for_fn_from_orchestrator(get_builder).await
    }

    /// Sends my public key to the orchestrator so that it can collect all public keys
    /// And get the updated config
    /// Blocks until the orchestrator collects all peer's public keys/configs
    /// # Panics
    /// if unable to post
    #[instrument(skip(self), name = "orchestrator public keys")]
    pub async fn post_and_wait_all_public_keys<TYPES: NodeType>(
        &self,
        validator_config: &mut ValidatorConfig<TYPES>,
        libp2p_advertise_address: Option<Multiaddr>,
        libp2p_public_key: Option<PeerId>,
    ) -> NetworkConfig<TYPES> {
        let pubkey: Vec<u8> =
            PeerConfig::<TYPES>::to_bytes(&validator_config.public_config()).clone();
        let da_requested: bool = validator_config.is_da;

        // Serialize our (possible) libp2p-specific data
        let request_body = vbs::Serializer::<OrchestratorVersion>::serialize(&(
            pubkey,
            libp2p_advertise_address,
            libp2p_public_key,
        ))
        .expect("failed to serialize request");

        // register our public key with the orchestrator
        let (node_index, is_da): (u64, bool) = loop {
            let result = self
                .client
                .post(&format!("api/pubkey/{da_requested}"))
                .body_binary(&request_body)
                .expect("Failed to form request")
                .send()
                .await
                .inspect_err(|err| tracing::error!("{err}"));

            if let Ok((index, is_da)) = result {
                break (index, is_da);
            }

            sleep(Duration::from_millis(250)).await;
        };

        validator_config.is_da = is_da;

        // wait for all nodes' public keys
        let wait_for_all_nodes_pub_key = |client: Client<ClientError, OrchestratorVersion>| {
            async move {
                client
                    .get("api/peer_pub_ready")
                    .send()
                    .await
                    .inspect_err(|err| tracing::error!("{err}"))
            }
            .boxed()
        };
        self.wait_for_fn_from_orchestrator::<_, _, ()>(wait_for_all_nodes_pub_key)
            .await;

        let mut network_config = self.get_config_after_collection().await;

        network_config.node_index = node_index;

        network_config
    }

    /// Tells the orchestrator this validator is ready to start
    /// Blocks until the orchestrator indicates all nodes are ready to start
    /// # Panics
    /// Panics if unable to post.
    #[instrument(skip(self), name = "orchestrator ready signal")]
    pub async fn wait_for_all_nodes_ready(&self, peer_config: Vec<u8>) -> bool {
        let send_ready_f = |client: Client<ClientError, OrchestratorVersion>| {
            let pk = peer_config.clone();
            async move {
                let result: Result<_, ClientError> = client
                    .post("api/ready")
                    .body_binary(&pk)
                    .unwrap()
                    .send()
                    .await
                    .inspect_err(|err| tracing::error!("{err}"));
                result
            }
            .boxed()
        };
        self.wait_for_fn_from_orchestrator::<_, _, ()>(send_ready_f)
            .await;

        let wait_for_all_nodes_ready_f = |client: Client<ClientError, OrchestratorVersion>| {
            async move { client.get("api/start").send().await }.boxed()
        };
        self.wait_for_fn_from_orchestrator(wait_for_all_nodes_ready_f)
            .await
    }

    /// Sends the benchmark metrics to the orchestrator
    /// # Panics
    /// Panics if unable to post
    #[instrument(skip_all, name = "orchestrator metrics")]
    pub async fn post_bench_results<TYPES: NodeType>(
        &self,
        bench_results: BenchResults<TYPES::View>,
    ) {
        let _send_metrics_f: Result<(), ClientError> = self
            .client
            .post("api/results")
            .body_json(&bench_results)
            .unwrap()
            .send()
            .await
            .inspect_err(|err| tracing::warn!("{err}"));
    }

    /// Generic function that waits for the orchestrator to return a non-error
    /// Returns whatever type the given function returns
    #[instrument(skip_all, name = "waiting for orchestrator")]
    async fn wait_for_fn_from_orchestrator<F, Fut, GEN>(&self, f: F) -> GEN
    where
        F: Fn(Client<ClientError, OrchestratorVersion>) -> Fut,
        Fut: Future<Output = Result<GEN, ClientError>>,
    {
        loop {
            let client = self.client.clone();
            let res = f(client).await;
            match res {
                Ok(x) => break x,
                Err(err) => {
                    tracing::info!("{err}");
                    sleep(Duration::from_millis(250)).await;
                },
            }
        }
    }
}
