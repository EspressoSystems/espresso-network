#![cfg(test)]

use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
    time::Duration,
};

use alloy::{
    network::EthereumWallet,
    node_bindings::Anvil,
    primitives::Address,
    providers::{
        ext::AnvilApi,
        fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
        layers::AnvilProvider,
        Provider, ProviderBuilder, RootProvider,
    },
    signers::local::LocalSigner,
};
use anyhow::bail;
use cdn_broker::{
    reexports::{crypto::signature::KeyPair, def::hook::NoMessageHook},
    Broker, Config as BrokerConfig,
};
use cdn_marshal::{Config as MarshalConfig, Marshal};
use clap::Parser;
use committable::{Commitment, Committable};
use derivative::Derivative;
use espresso_contract_deployer::{
    builder::DeployerArgsBuilder, network_config::light_client_genesis_from_stake_table, Contract,
    Contracts,
};
use espresso_types::{
    eth_signature_key::EthKeyPair, traits::PersistenceOptions, v0_3::ChainConfig, EpochVersion,
    FeeAccount, L1Client, Leaf2, PrivKey, PubKey, SeqTypes, SequencerVersions, Transaction, V0_0,
};
use futures::{
    future::{join_all, try_join_all, BoxFuture, FutureExt},
    stream::{BoxStream, StreamExt},
};
use hotshot::traits::implementations::derive_libp2p_peer_id;
use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
use hotshot_orchestrator::run_orchestrator;
use hotshot_testing::{
    block_builder::{SimpleBuilderImplementation, TestBuilderImplementation},
    test_builder::BuilderChange,
};
use hotshot_types::{
    data::EpochNumber,
    event::{Event, EventType},
    light_client::StateKeyPair,
    network::{Libp2pConfig, NetworkConfig},
    traits::{node_implementation::ConsensusTime, signature_key::SignatureKey},
    PeerConfig,
};
use itertools::Itertools;
use options::Modules;
use portpicker::pick_unused_port;
use run::init_with_storage;
use sequencer_utils::test_utils::setup_test;
use staking_cli::demo::{setup_stake_table_contract_for_test, DelegationConfig};
use surf_disco::{error::ClientError, Url};
use tempfile::TempDir;
use tokio::{
    task::{spawn, JoinHandle},
    time::{sleep, timeout},
};
use vbs::version::Version;
use vec1::vec1;

use super::*;
use crate::{
    api::{
        self, data_source::testing::TestableSequencerDataSource, options::Query,
        test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
    },
    genesis::{L1Finalized, StakeTableConfig},
    network::cdn::{TestingDef, WrappedSignatureKey},
    testing::{staking_priv_keys, wait_for_decide_on_handle},
    SequencerApiVersion,
};
type MockSequencerVersions = SequencerVersions<EpochVersion, V0_0>;
async fn test_restart_helper(network: (usize, usize), restart: (usize, usize), cdn: bool) {
    setup_test();

    let mut network = TestNetwork::new(network.0, network.1, cdn).await;

    // Let the network get going.
    network.check_progress().await;
    // Restart some combination of nodes and ensure progress resumes.
    network.restart(restart.0, restart.1).await;

    network.shut_down().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_1_da_with_cdn() {
    test_restart_helper((2, 3), (1, 0), true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_1_regular_with_cdn() {
    test_restart_helper((2, 3), (0, 1), true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_f_with_cdn() {
    test_restart_helper((4, 6), (1, 2), true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_f_minus_1_with_cdn() {
    test_restart_helper((4, 6), (1, 1), true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_f_plus_1_with_cdn() {
    test_restart_helper((4, 6), (1, 3), true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_2f_with_cdn() {
    test_restart_helper((4, 6), (1, 5), true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_2f_minus_1_with_cdn() {
    test_restart_helper((4, 6), (1, 4), true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_2f_plus_1_with_cdn() {
    test_restart_helper((4, 6), (2, 5), true).await;
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_all_with_cdn() {
    test_restart_helper((2, 8), (2, 8), true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_all_da_with_cdn() {
    test_restart_helper((2, 8), (2, 0), true).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_1_da_without_cdn() {
    test_restart_helper((2, 3), (1, 0), false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_1_regular_without_cdn() {
    test_restart_helper((2, 3), (0, 1), false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_f_without_cdn() {
    test_restart_helper((4, 6), (1, 2), false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_f_minus_1_without_cdn() {
    test_restart_helper((4, 6), (1, 1), false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_f_plus_1_without_cdn() {
    test_restart_helper((4, 6), (1, 3), false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_2f_without_cdn() {
    test_restart_helper((4, 6), (1, 5), false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_2f_minus_1_without_cdn() {
    test_restart_helper((4, 6), (1, 4), false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_2f_plus_1_without_cdn() {
    test_restart_helper((4, 6), (2, 5), false).await;
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_all_without_cdn() {
    test_restart_helper((2, 8), (2, 8), false).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_all_da_without_cdn() {
    test_restart_helper((2, 8), (2, 0), false).await;
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn slow_test_restart_staggered() {
    setup_test();

    let mut network = TestNetwork::new(4, 6, false).await;

    // Check that the builder works at the beginning.
    network.check_builder().await;

    // Restart nodes in a staggered fashion, so that progress never halts, but eventually every node
    // has been restarted. This can lead to a situation where no node has the full validated state
    // in memory, so we will need a pretty advanced form of catchup in order to make progress and
    // process blocks after this.
    for i in 0..4 {
        network.restart_and_progress([i], []).await;
    }
    // Restart the remaining regular nodes.
    for i in 0..6 {
        network.restart_and_progress([], [i]).await;
    }

    // Check that we can still build blocks after the restart.
    network.check_builder().await;

    network.shut_down().await;
}

#[derive(Clone, Copy, Debug)]
struct NetworkParams<'a> {
    genesis_file: &'a Path,
    orchestrator_port: u16,
    cdn_port: u16,
    l1_provider: &'a str,
    peer_ports: &'a [u16],
    api_ports: &'a [u16],
}

#[derive(Clone, Debug)]
struct NodeParams {
    api_port: u16,
    libp2p_port: u16,
    staking_key: PrivKey,
    state_key: StateKeyPair,
    is_da: bool,
}

impl NodeParams {
    fn new(ports: &mut PortPicker, i: u64, is_da: bool) -> Self {
        Self {
            api_port: ports.pick(),
            libp2p_port: ports.pick(),
            staking_key: PubKey::generated_from_seed_indexed([0; 32], i).1,
            state_key: StateKeyPair::generate_from_seed_indexed([0; 32], i),
            is_da,
        }
    }
}

#[derive(Debug)]
struct TestNode<S: TestableSequencerDataSource> {
    storage: S::Storage,
    context: Option<
        SequencerContext<
            network::Production,
            <S::Options as PersistenceOptions>::Persistence,
            MockSequencerVersions,
        >,
    >,
    modules: Modules,
    opt: Options,
    num_nodes: usize,
    reference_state: Arc<RwLock<BTreeMap<u64, Commitment<Leaf2>>>>,
    /// Number of epochs to wait after restart before running progress check.
    wait_for_epoch: EpochNumber,
}

impl<S: TestableSequencerDataSource> TestNode<S> {
    #[tracing::instrument]
    async fn new(network: NetworkParams<'_>, node: &NodeParams) -> Self {
        tracing::info!(?network, ?node, "creating node");

        let opts = api::Options::from(api::options::Http::with_port(node.api_port));
        let storage = S::create_storage().await;
        let opt = S::options(&storage, opts);

        let mut modules = Modules {
            http: Some(api::options::Http::with_port(node.api_port)),
            query: Some(Default::default()),
            storage_fs: opt.storage_fs,
            storage_sql: opt.storage_sql,
            ..Default::default()
        };
        if node.is_da {
            modules.query = Some(Query {
                peers: network
                    .api_ports
                    .iter()
                    .map(|port| format!("http://127.0.0.1:{port}").parse().unwrap())
                    .collect(),
            });
        }

        let mut opt = Options::parse_from([
            "sequencer",
            "--private-staking-key",
            &node
                .staking_key
                .to_tagged_base64()
                .expect("valid tagged-base64")
                .to_string(),
            "--private-state-key",
            &node
                .state_key
                .sign_key_ref()
                .to_tagged_base64()
                .expect("valid tagged-base64")
                .to_string(),
            "--genesis-file",
            &network.genesis_file.display().to_string(),
            "--orchestrator-url",
            &format!("http://localhost:{}", network.orchestrator_port),
            "--libp2p-bind-address",
            &format!("0.0.0.0:{}", node.libp2p_port),
            "--libp2p-advertise-address",
            &format!("127.0.0.1:{}", node.libp2p_port),
            "--cdn-endpoint",
            &format!("127.0.0.1:{}", network.cdn_port),
            "--state-peers",
            &network
                .peer_ports
                .iter()
                .map(|port| format!("http://127.0.0.1:{port}"))
                .join(","),
            "--l1-provider-url",
            network.l1_provider,
            "--l1-polling-interval",
            "1s",
        ]);
        opt.is_da = node.is_da;
        Self {
            storage,
            modules,
            opt,
            num_nodes: network.peer_ports.len(),
            context: None,
            reference_state: Default::default(),
            wait_for_epoch: EpochNumber::new(3),
        }
    }

    fn stop(&mut self) -> BoxFuture<()> {
        async {
            if let Some(mut context) = self.context.take() {
                tracing::info!(node_id = context.node_id(), "stopping node");
                context.shut_down().await;
            }
        }
        .boxed()
    }

    fn start(&mut self) -> BoxFuture<()>
    where
        S::Storage: Send,
    {
        async {
            tracing::info!("starting node");

            // If we are starting a node which had already been started and stopped, we may need to
            // delay a bit for the OS to reclaim the node's P2P port. Otherwise initialization of
            // libp2p may fail with "address already in use". Thus, retry the node initialization
            // with a backoff.
            let mut retries = 5;
            let mut delay = Duration::from_secs(1);
            let genesis = Genesis::from_file(&self.opt.genesis_file).unwrap();
            let ctx = loop {
                match init_with_storage(
                    genesis.clone(),
                    self.modules.clone(),
                    self.opt.clone(),
                    S::persistence_options(&self.storage),
                    MockSequencerVersions::new(),
                )
                .await
                {
                    Ok(ctx) => break ctx,
                    Err(err) => {
                        tracing::error!(retries, ?delay, "initialization failed: {err:#}");
                        if retries == 0 {
                            panic!("initialization failed too many times");
                        }

                        sleep(delay).await;
                        delay *= 2;
                        retries -= 1;
                    },
                }
            };

            tracing::info!(node_id = ctx.node_id(), "starting consensus");
            ctx.start_consensus().await;
            self.context = Some(ctx);
        }
        .boxed()
    }

    async fn event_stream(&self) -> Option<BoxStream<Event<SeqTypes>>> {
        if let Some(ctx) = &self.context {
            Some(ctx.event_stream().await.boxed())
        } else {
            None
        }
    }

    fn node_id(&self) -> Option<u64> {
        let Some(context) = &self.context else {
            return None;
        };
        Some(context.node_id())
    }

    fn check_progress_with_timeout(&self) -> BoxFuture<anyhow::Result<()>> {
        async {
            let Some(context) = &self.context else {
                tracing::info!("skipping progress check on stopped node");
                return Ok(());
            };
            let node_id = context.node_id();
            let next_view_timeout = {
                context
                    .consensus()
                    .read()
                    .await
                    .hotshot
                    .config
                    .next_view_timeout
            };
            // Give enough time for every node to propose, with every view timing out. This is
            // conservative: of course if we actually make progress, not every view will time out,
            // and we will take less than this amount of time.
            let timeout_duration =
                4 * Duration::from_millis(next_view_timeout) * (self.num_nodes as u32);
            match timeout(timeout_duration, self.check_progress()).await {
                Ok(res) => res,
                Err(_) => bail!("timed out waiting for progress on node {node_id}"),
            }
        }
        .boxed()
    }

    async fn check_progress(&self) -> anyhow::Result<()> {
        let Some(context) = &self.context else {
            tracing::info!("skipping progress check on stopped node");
            return Ok(());
        };

        let num_nodes = {
            context
                .consensus()
                .read()
                .await
                .hotshot
                .config
                .num_nodes_with_stake
        };
        let node_id = context.node_id();
        tracing::info!(node_id, num_nodes, "waiting for progress from node");

        // Wait for a block proposed by this node. This proves that the node is tracking consensus
        // (getting Decide events) and participating (able to propose).
        let mut events = context.event_stream().await;
        while let Some(event) = events.next().await {
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };

            for leaf in leaf_chain.iter() {
                let height = leaf.leaf.height();

                // Check that this nodes proposals are decided
                if leaf.leaf.view_number().u64() % (num_nodes.get() as u64) == node_id {
                    tracing::info!(node_id, height, "got leaf proposed by this node");
                    return Ok(());
                }
                tracing::info!(
                    node_id,
                    height,
                    view = leaf.leaf.view_number().u64(),
                    "leaf not proposed by this node"
                );
            }
        }

        bail!("node {node_id} event stream ended unexpectedly");
    }

    /// Collect the first 50 committed leaves from the event stream for this node,
    /// and write them into the test node state
    /// This is later used to verify that the node's state is consistent
    async fn populate_state_from_event_stream(&self) -> anyhow::Result<()> {
        let Some(context) = &self.context else {
            tracing::info!("skipping state check on stopped node");
            return Ok(());
        };

        let node_id = context.node_id();
        tracing::info!(node_id, "verifying state of node");

        let mut events = context.event_stream().await;
        let mut collected_leaves = 0;
        let mut state_write = self.reference_state.write().await;

        while let Some(event) = events.next().await {
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };

            {
                for leaf in leaf_chain.iter() {
                    let leaf = leaf.leaf.clone();
                    let height = leaf.height();
                    state_write.insert(height, leaf.commit());

                    tracing::info!("node_id={node_id} state height= {height}");
                    collected_leaves += 1;
                }

                if collected_leaves == 30 {
                    return Ok(());
                }
            }
        }

        bail!("node {node_id} event stream ended unexpectedly");
    }

    async fn check_builder(&self, port: u16) {
        tracing::info!("testing builder liveness");

        // Configure the builder to shut down in 50 views, so we don't leak resources or ports.
        let ctx = self.context.as_ref().unwrap();
        let down_view = ctx.consensus().read().await.cur_view().await + 50;

        // Start a builder.
        let url: Url = format!("http://localhost:{port}").parse().unwrap();
        let task = <SimpleBuilderImplementation as TestBuilderImplementation<SeqTypes>>::start(
            self.num_nodes,
            format!("http://0.0.0.0:{port}").parse().unwrap(),
            (),
            [(down_view.u64(), BuilderChange::Down)]
                .into_iter()
                .collect(),
        )
        .await;
        task.start(Box::new(ctx.event_stream().await));

        // Wait for the API to start serving.
        let client = surf_disco::Client::<ClientError, SequencerApiVersion>::new(url);
        assert!(
            client.connect(Some(Duration::from_secs(60))).await,
            "timed out connecting to builder API"
        );

        // Submit a transaction and wait for it to be sequenced.
        let mut events = ctx.event_stream().await;
        let tx = Transaction::random(&mut rand::thread_rng());
        ctx.submit_transaction(tx.clone()).await.unwrap();
        let (block, _) = timeout(
            Duration::from_secs(60),
            wait_for_decide_on_handle(&mut events, &tx),
        )
        .await
        .expect("timed out waiting for transaction to be sequenced");
        tracing::info!(block, "transaction sequenced");

        // Wait until the builder is cleaned up.
        while ctx.consensus().read().await.cur_view().await <= down_view {
            sleep(Duration::from_secs(1)).await;
        }
    }

    /// Wait for the given Epoch.
    async fn wait_for_epoch(&self) {
        let epoch = self.wait_for_epoch;
        let Some(context) = &self.context else {
            tracing::info!("skipping progress check on stopped node");
            return;
        };

        let node_id = context.node_id();
        tracing::info!(node_id, "waiting for epoch: {epoch:?}");
        let mut events = context.event_stream().await;

        let timeout_duration = Duration::from_secs(60);
        timeout(timeout_duration, async {
            while let Some(event) = events.next().await {
                let EventType::Decide { qc, .. } = event.event else {
                    continue;
                };
                if qc.data.epoch >= Some(epoch) {
                    tracing::info!(node_id, "reached epoch: {epoch:?}");
                    break;
                }
            }
        })
        .await
        .expect("timed out waiting for epoch after restart");
    }
}

type AnvilFillProvider = AnvilProvider<
    FillProvider<
        JoinFill<
            alloy::providers::Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        RootProvider,
    >,
>;

#[derive(Derivative)]
#[derivative(Debug)]
struct TestNetwork {
    da_nodes: Vec<TestNode<api::sql::DataSource>>,
    regular_nodes: Vec<TestNode<api::sql::DataSource>>,
    tmp: TempDir,
    builder_port: u16,
    orchestrator_task: Option<JoinHandle<()>>,
    broker_task: Option<JoinHandle<()>>,
    marshal_task: Option<JoinHandle<()>>,
    #[derivative(Debug = "ignore")]
    anvil: AnvilFillProvider,
}

impl Drop for TestNetwork {
    fn drop(&mut self) {
        if let Some(task) = self.orchestrator_task.take() {
            task.abort();
        }
        if let Some(task) = self.broker_task.take() {
            task.abort();
        }
        if let Some(task) = self.marshal_task.take() {
            task.abort();
        }
    }
}

impl TestNetwork {
    async fn new(da_nodes: usize, regular_nodes: usize, cdn: bool) -> Self {
        let mut ports = PortPicker::default();

        let tmp = TempDir::new().unwrap();
        let genesis_file_path = tmp.path().join("genesis.toml");

        let mut genesis = Genesis {
            chain_config: Default::default(),
            // TODO we apparently have two `capacity` configurations
            stake_table: StakeTableConfig {
                capacity: STAKE_TABLE_CAPACITY_FOR_TEST,
            },
            l1_finalized: L1Finalized::Number { number: 20 },
            header: Default::default(),
            upgrades: Default::default(),
            base_version: Version { major: 0, minor: 3 },
            upgrade_version: Version { major: 0, minor: 3 },
            epoch_height: Some(15),
            drb_difficulty: None,
            epoch_start_block: Some(1),
            // TODO we apparently have two `capacity` configurations
            stake_table_capacity: Some(STAKE_TABLE_CAPACITY_FOR_TEST),
            drb_upgrade_difficulty: None,
            // Start with a funded account, so we can test catchup after restart.
            accounts: [(builder_account(), 1000000000.into())]
                .into_iter()
                .collect(),
            genesis_version: Version { major: 0, minor: 1 },
        };

        let node_params = (0..da_nodes + regular_nodes)
            .map(|i| NodeParams::new(&mut ports, i as u64, i < da_nodes))
            .collect::<Vec<_>>();

        let orchestrator_port = ports.pick();
        let builder_port = ports.pick();
        let orchestrator_task = Some(start_orchestrator(
            orchestrator_port,
            &node_params,
            builder_port,
        ));

        let cdn_dir = tmp.path().join("cdn");
        let cdn_port = ports.pick();
        let broker_task = if cdn {
            Some(start_broker(&mut ports, &cdn_dir).await)
        } else {
            None
        };
        let marshal_task = if cdn {
            Some(start_marshal(&cdn_dir, cdn_port).await)
        } else {
            None
        };

        let anvil_port = ports.pick();
        let anvil = Anvil::new()
            .args(["--slots-in-an-epoch", "1"])
            .port(anvil_port)
            .spawn();
        let anvil_endpoint = anvil.endpoint();

        let l1_client = L1Client::anvil(&anvil).expect("create l1 client");
        let anvil = AnvilProvider::new(l1_client.clone().provider, Arc::new(anvil));

        let api_ports = node_params
            .iter()
            .take(da_nodes)
            .map(|node| node.api_port)
            .collect::<Vec<_>>();
        let peer_ports = node_params
            .iter()
            .map(|node| node.api_port)
            .collect::<Vec<_>>();
        let network_params = NetworkParams {
            genesis_file: &genesis_file_path,
            orchestrator_port,
            cdn_port,
            l1_provider: &anvil_endpoint,
            api_ports: &api_ports,
            peer_ports: &peer_ports,
        };

        let mut network = Self {
            da_nodes: join_all(
                (0..da_nodes).map(|i| TestNode::new(network_params, &node_params[i])),
            )
            .await,
            regular_nodes: join_all(
                (0..regular_nodes)
                    .map(|i| TestNode::new(network_params, &node_params[i + da_nodes])),
            )
            .await,
            tmp,
            builder_port,
            orchestrator_task,
            broker_task,
            marshal_task,
            anvil,
        };

        // Deploy stake contracts and delegate.
        let stake_table_address = network.deploy(&genesis).await.unwrap();

        // Add contract address to `ChainConfig`.
        let chain_config = ChainConfig {
            base_fee: 1.into(),
            stake_table_contract: Some(stake_table_address),
            ..Default::default()
        };
        genesis.chain_config = chain_config;
        genesis.to_file(&genesis_file_path).unwrap();

        let finalized = l1_client
            .get_block(alloy::eips::BlockId::finalized())
            .full()
            .await
            .unwrap();
        let head = l1_client
            .get_block(alloy::eips::BlockId::latest())
            .full()
            .await
            .unwrap();

        tracing::info!(
            "latest block head: {}, latest finalized: {}",
            head.unwrap().header.number,
            finalized.unwrap().header.number
        );

        join_all(
            network
                .da_nodes
                .iter_mut()
                .map(TestNode::start)
                .chain(network.regular_nodes.iter_mut().map(TestNode::start)),
        )
        .await;

        network
    }

    /// Deploy stake contracts and delegate.
    async fn deploy(&self, genesis: &Genesis) -> anyhow::Result<Address> {
        let stake_table_version = StakeTableContractVersion::V2;
        let delegation_config = DelegationConfig::EqualAmounts;

        let anvil_instance = &self.anvil.anvil();
        let l1_url: reqwest::Url = anvil_instance.endpoint().parse().unwrap();

        let l1_signer_key = anvil_instance.keys()[0].clone();
        let signer = LocalSigner::from(l1_signer_key);

        let deployer = ProviderBuilder::new()
            .wallet(EthereumWallet::from(signer.clone()))
            .on_http(l1_url.clone());

        let blocks_per_epoch = genesis.epoch_height;
        let epoch_start_block = genesis.epoch_start_block;

        let staking_keys: Vec<(BLSPrivKey, StateKeyPair)> = self
            .da_nodes
            .iter()
            .chain(self.regular_nodes.iter())
            .map(|node| {
                let keys = node.opt.private_keys().unwrap();
                (keys.0, StateKeyPair::from_sign_key(keys.1))
            })
            .collect();

        let (bls, state): (Vec<BLSPrivKey>, Vec<StateKeyPair>) =
            staking_keys.clone().into_iter().unzip();
        let staking_priv_keys = staking_priv_keys(&bls, &state, staking_keys.len());

        let hss_staking: Vec<PeerConfig<SeqTypes>> = staking_keys
            .iter()
            .map(|(bls, state)| PeerConfig {
                stake_table_entry: BLSPubKey::from_private(bls).stake_table_entry(U256::from(1)),
                state_ver_key: state.ver_key(),
            })
            .collect();

        let (genesis_state, genesis_stake) = light_client_genesis_from_stake_table(
            &hss_staking.into(),
            STAKE_TABLE_CAPACITY_FOR_TEST,
        )
        .unwrap();

        let mut contracts = Contracts::new();
        let args = DeployerArgsBuilder::default()
            .deployer(deployer.clone())
            .mock_light_client(true)
            .genesis_lc_state(genesis_state)
            .genesis_st_state(genesis_stake)
            .blocks_per_epoch(blocks_per_epoch.unwrap())
            .epoch_start_block(epoch_start_block.unwrap())
            .multisig_pauser(signer.address())
            .token_name("Espresso".to_string())
            .token_symbol("ESP".to_string())
            .initial_token_supply(U256::from(3590000000u64))
            .ops_timelock_delay(U256::from(0))
            .ops_timelock_admin(signer.address())
            .ops_timelock_proposers(vec![signer.address()])
            .ops_timelock_executors(vec![signer.address()])
            .safe_exit_timelock_delay(U256::from(10))
            .safe_exit_timelock_admin(signer.address())
            .safe_exit_timelock_proposers(vec![signer.address()])
            .safe_exit_timelock_executors(vec![signer.address()])
            .build()
            .unwrap();

        match stake_table_version {
            StakeTableContractVersion::V1 => args.deploy_to_stake_table_v1(&mut contracts).await,
            StakeTableContractVersion::V2 => args.deploy_all(&mut contracts).await,
        }
        .context("failed to deploy contracts")?;

        let stake_table_address = contracts
            .address(Contract::StakeTableProxy)
            .expect("StakeTableProxy address not found");
        let token_addr = contracts
            .address(Contract::EspTokenProxy)
            .expect("EspTokenProxy address not found");

        tracing::info!(?stake_table_address, ?token_addr);

        setup_stake_table_contract_for_test(
            l1_url.clone(),
            &deployer,
            stake_table_address,
            staking_priv_keys,
            delegation_config,
        )
        .await
        .expect("stake table setup failed");

        self.anvil
            .anvil_set_interval_mining(1)
            .await
            .expect("interval mining");

        Ok(stake_table_address)
    }

    async fn wait_for_epoch(&self) {
        join_all(
            self.da_nodes
                .iter()
                .map(TestNode::wait_for_epoch)
                .chain(self.regular_nodes.iter().map(TestNode::wait_for_epoch)),
        )
        .await;
    }

    async fn check_progress(&self) {
        try_join_all(
            self.da_nodes
                .iter()
                .map(TestNode::check_progress_with_timeout)
                .chain(
                    self.regular_nodes
                        .iter()
                        .map(TestNode::check_progress_with_timeout),
                ),
        )
        .await
        .unwrap();
    }

    /// Check that state has not diverged between nodes and that all nodes were
    /// checked. Mostly useful in tests that do not restart all nodes, as those
    /// cases confirm that state has not regressed.
    async fn check_state(&self) {
        // populate each test node's state
        try_join_all(
            self.da_nodes
                .iter()
                .map(TestNode::populate_state_from_event_stream)
                .chain(
                    self.regular_nodes
                        .iter()
                        .map(TestNode::populate_state_from_event_stream),
                ),
        )
        .await
        .unwrap();

        let mut nodes_iter = self.da_nodes.iter().chain(self.regular_nodes.iter());

        let first_node = nodes_iter.next().unwrap();
        let ref_id = first_node.node_id().expect("Node id not found");
        let ref_state = first_node.reference_state.read().await.clone();

        // assert that all the nodes have same leaves from their event streams
        // this also ensures validated state consistency
        // Note: Nodes may have started consuming the event stream at different points,
        // since the stream might have been partially processed before this check.
        // Therefore, we only compare leaves at heights that are present in both
        // the current node and the reference state.
        for node in nodes_iter {
            let node_id = node.node_id().expect("Node id not found");
            let state = node.reference_state.read().await.clone();

            for (height, commitment) in state.iter() {
                if let Some(ref_commitment) = ref_state.get(height) {
                    assert_eq!(
                        ref_commitment, commitment,
                        "State mismatch between node {node_id} and reference node {ref_id}"
                    );
                }
            }
        }
    }

    async fn check_builder(&self) {
        self.da_nodes[0].check_builder(self.builder_port).await;
    }

    /// Restart indicated number of DA and non-DA nodes.
    ///
    /// If possible (less than a quorum of nodes have been stopped), check that remaining nodes can
    /// still make progress without the restarted nodes. In any case, check that the network as a
    /// whole makes progress once the restarted nodes are back online.
    async fn restart(&mut self, da_nodes: usize, regular_nodes: usize) {
        self.restart_helper(0..da_nodes, 0..regular_nodes, false)
            .await;
        self.wait_for_epoch().await;
        self.check_progress().await;
        self.check_state().await;
    }

    /// Restart indicated nodes, ensuring progress is maintained at all times.
    ///
    /// This is a lighter weight version of [`restart`](Self::restart). While the former includes
    /// heavy checks that all nodes are progressing, which makes it useful as a stress test, this
    /// function does the minimum required to check that progress is maintained at all times across
    /// the network as a whole. This makes it a useful building block for more complex patterns,
    /// like a staggered restart.
    async fn restart_and_progress(
        &mut self,
        da_nodes: impl IntoIterator<Item = usize>,
        regular_nodes: impl IntoIterator<Item = usize>,
    ) {
        self.restart_helper(da_nodes, regular_nodes, true).await;

        // Just wait for one decide after the restart, so we don't restart subsequent nodes too
        // quickly.
        tracing::info!("waiting for progress after restart");
        let mut events = self.da_nodes[0].event_stream().await.unwrap();
        let timeout_duration = Duration::from_secs((2 * self.num_nodes()) as u64);
        timeout(timeout_duration, async {
            loop {
                let event = events
                    .next()
                    .await
                    .expect("event stream terminated unexpectedly");
                let EventType::Decide { leaf_chain, .. } = event.event else {
                    continue;
                };
                tracing::info!(?leaf_chain, "got decide, chain is progressing");
                break;
            }
        })
        .await
        .expect("timed out waiting for progress after restart");
    }

    async fn restart_helper(
        &mut self,
        da_nodes: impl IntoIterator<Item = usize>,
        regular_nodes: impl IntoIterator<Item = usize>,
        assert_progress: bool,
    ) {
        let da_nodes = da_nodes.into_iter().collect::<Vec<_>>();
        let regular_nodes = regular_nodes.into_iter().collect::<Vec<_>>();
        tracing::info!(?da_nodes, ?regular_nodes, "shutting down nodes");

        join_all(
            select(&mut self.da_nodes, &da_nodes)
                .map(TestNode::stop)
                .chain(select(&mut self.regular_nodes, &regular_nodes).map(TestNode::stop)),
        )
        .await;

        // We use 3n/4 + 1 as the quorum threshold (fault tolerance f = n/4), even though the
        // theoretical fault tolerance of HotStuff consensus is n/3, because our implementation does
        // not currently re-randomize the order of leaders, and requires 4 consecutive honest
        // leaders to commit. Thus, with 1/4 or more of the nodes dishonest, you could get unlucky
        // and have one dishonest leader every 4, thus preventing consensus from progressing.
        let quorum_threshold = 3 * self.num_nodes() / 4 + 1;
        let da_threshold = 2 * self.da_nodes.len() / 3 + 1;
        if self.num_nodes() - da_nodes.len() - regular_nodes.len() > quorum_threshold
            && self.da_nodes.len() - da_nodes.len() >= da_threshold
        {
            // If we are shutting down less than f nodes, the remaining nodes should be able to make
            // progress, and we will check that is the case.
            //
            // Note that not every node will be able to commit leaves, because a node requires the
            // cooperation of the node after it to commit its proposal. But, as long as we have shut
            // down fewer than the fault tolerance, at least *some* node will have a correct node
            // after it and will be able to commit. Thus, we just grab an event stream and look for
            // any decide.
            tracing::info!("waiting for remaining nodes to progress");
            // Find the first DA node we _didn't_ shut down.
            let da_node = self
                .da_nodes
                .iter()
                .enumerate()
                .find_map(|(i, node)| {
                    if da_nodes.contains(&i) {
                        None
                    } else {
                        Some(node)
                    }
                })
                .unwrap();
            let mut events = da_node.event_stream().await.unwrap();

            // Wait for a few decides, the first couple may be from before the restart.
            for _ in 0..5 {
                let timeout_duration = Duration::from_secs((2 * self.num_nodes()) as u64);
                timeout(timeout_duration, async {
                    loop {
                        let event = events
                            .next()
                            .await
                            .expect("event stream terminated unexpectedly");
                        let EventType::Decide { leaf_chain, .. } = event.event else {
                            continue;
                        };
                        tracing::info!(?leaf_chain, "got decide, chain is progressing");
                        break;
                    }
                })
                .await
                .expect("timed out waiting for progress with nodes down");
            }
        } else {
            assert!(
                !assert_progress,
                "test requested that progress continue after shutdown, but also requested that \
                 too many nodes be shut down: {}/{} DA, {}/{} regular",
                da_nodes.len(),
                self.da_nodes.len(),
                regular_nodes.len(),
                self.regular_nodes.len(),
            );

            // Make sure there is a brief delay before restarting the nodes; we need the OS to
            // have time to clean up the ports they were using.
            tracing::info!(
                "shut down too many nodes to make progress; will continue after a brief delay"
            );
            sleep(Duration::from_secs(2)).await;
        }

        join_all(
            select(&mut self.da_nodes, &da_nodes)
                .map(TestNode::start)
                .chain(select(&mut self.regular_nodes, &regular_nodes).map(TestNode::start)),
        )
        .await;
    }

    async fn shut_down(mut self) {
        tracing::info!("shutting down test network");
        join_all(
            self.da_nodes
                .iter_mut()
                .map(TestNode::stop)
                .chain(self.regular_nodes.iter_mut().map(TestNode::stop)),
        )
        .await;
    }

    fn num_nodes(&self) -> usize {
        self.da_nodes.len() + self.regular_nodes.len()
    }
}

fn start_orchestrator(port: u16, nodes: &[NodeParams], builder_port: u16) -> JoinHandle<()> {
    // We don't run a builder in these tests, so use a very short timeout before nodes decide to
    // build an empty block on their own.
    let builder_timeout = Duration::from_millis(100);
    // These tests frequently have nodes down and views failing, so we use a fairly short view
    // timeout.
    let view_timeout = Duration::from_secs(2);

    let num_nodes = nodes.len();
    let bootstrap_nodes = nodes
        .iter()
        .map(|node| {
            let port = node.libp2p_port;
            let peer_id = derive_libp2p_peer_id::<PubKey>(&node.staking_key).unwrap();
            let addr = format!("/ip4/127.0.0.1/udp/{port}/quic-v1")
                .parse()
                .unwrap();
            (peer_id, addr)
        })
        .collect();

    let mut config = NetworkConfig::<SeqTypes> {
        indexed_da: false,
        libp2p_config: Some(Libp2pConfig { bootstrap_nodes }),
        ..Default::default()
    };
    config.config.num_nodes_with_stake = num_nodes.try_into().unwrap();
    config.config.da_staked_committee_size = num_nodes;
    config.config.known_nodes_with_stake = vec![];
    config.config.known_da_nodes = vec![];
    config.config.next_view_timeout = view_timeout.as_millis() as u64;
    config.config.builder_timeout = builder_timeout;
    config.config.builder_urls = vec1![format!("http://localhost:{builder_port}").parse().unwrap()];

    let bind = format!("http://0.0.0.0:{port}").parse().unwrap();
    spawn(async move {
        match run_orchestrator(config, bind).await {
            Ok(()) => tracing::warn!("orchestrator exited"),
            Err(err) => tracing::error!(%err, "orchestrator failed"),
        }
    })
}

async fn start_broker(ports: &mut PortPicker, dir: &Path) -> JoinHandle<()> {
    let (public_key, private_key) = PubKey::generated_from_seed_indexed([0; 32], 1337);
    let public_port = ports.pick();
    let private_port = ports.pick();
    let broker_config: BrokerConfig<TestingDef<SeqTypes>> = BrokerConfig {
        public_advertise_endpoint: format!("127.0.0.1:{public_port}"),
        public_bind_endpoint: format!("127.0.0.1:{public_port}"),
        private_advertise_endpoint: format!("127.0.0.1:{private_port}"),
        private_bind_endpoint: format!("127.0.0.1:{private_port}"),

        metrics_bind_endpoint: None,
        discovery_endpoint: dir.display().to_string(),
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

    spawn(async move {
        match Broker::new(broker_config).await.unwrap().start().await {
            Ok(()) => tracing::warn!("broker exited"),
            Err(err) => tracing::error!("broker failed: {err:#}"),
        }
    })
}

async fn start_marshal(dir: &Path, port: u16) -> JoinHandle<()> {
    let marshal_config = MarshalConfig {
        bind_endpoint: format!("0.0.0.0:{port}"),
        metrics_bind_endpoint: None,
        discovery_endpoint: dir.display().to_string(),
        ca_cert_path: None,
        ca_key_path: None,
        global_memory_pool_size: Some(1024 * 1024 * 1024),
    };

    spawn(async move {
        match Marshal::<TestingDef<SeqTypes>>::new(marshal_config)
            .await
            .unwrap()
            .start()
            .await
        {
            Ok(()) => tracing::warn!("marshal exited"),
            Err(err) => tracing::error!("marshal failed: {err:#}"),
        }
    })
}

/// Allocator for unused ports.
///
/// While portpicker is able to pick ports that are currently unused by the OS, its allocation is
/// random, and it may return the same port twice if that port is still unused by the OS the second
/// time. This test suite allocates many ports, and it is often convenient to allocate many in a
/// batch, before starting the services that listen on them, so that the first port selected is not
/// "in use" when we select later ports in the same batch.
///
/// This object keeps track not only of ports in use by the OS, but also ports it has already given
/// out, for which there may not yet be any listener. Thus, it is safe to use this to allocate many
/// ports at once, without a collision.
#[derive(Debug, Default)]
struct PortPicker {
    allocated: HashSet<u16>,
}

impl PortPicker {
    fn pick(&mut self) -> u16 {
        loop {
            let port = pick_unused_port().unwrap();
            if self.allocated.insert(port) {
                break port;
            }
            tracing::warn!(
                port,
                "picked port which is already allocated, will try again. If this error persists, \
                 try reducing the number of ports being used."
            );
        }
    }
}

fn builder_key_pair() -> EthKeyPair {
    use hotshot_types::traits::signature_key::BuilderSignatureKey;
    FeeAccount::generated_from_seed_indexed([1; 32], 0).1
}

fn builder_account() -> FeeAccount {
    builder_key_pair().fee_account()
}

fn select<'a, T>(nodes: &'a mut [T], is: &'a [usize]) -> impl Iterator<Item = &'a mut T> {
    nodes
        .iter_mut()
        .enumerate()
        .filter_map(|(i, elem)| if is.contains(&i) { Some(elem) } else { None })
}
