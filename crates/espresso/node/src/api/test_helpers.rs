use std::{cmp::max, time::Duration};

use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::{ProviderBuilder, ext::AnvilApi},
};
use committable::Committable;
use espresso_contract_deployer::{
    Contract, Contracts, DEFAULT_EXIT_ESCROW_PERIOD_SECONDS, builder::DeployerArgsBuilder,
    network_config::light_client_genesis_from_stake_table,
};
use espresso_types::{
    MOCK_SEQUENCER_VERSIONS, NamespaceId, ValidatedState,
    v0::traits::{NullEventConsumer, PersistenceOptions, StateCatchup},
};
use futures::{
    future::{FutureExt, join_all},
    stream::StreamExt,
};
use hotshot::types::{Event, EventType};
use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
use hotshot_types::{
    event::LeafInfo, light_client::LCV3StateSignatureRequestBody, new_protocol::CoordinatorEvent,
    traits::metrics::NoMetrics,
};
use itertools::izip;
use jf_merkle_tree_compat::{MerkleCommitment, MerkleTreeScheme};
use staking_cli::demo::{DelegationConfig, StakingTransactions};
use surf_disco::Client;
use tempfile::TempDir;
use test_utils::reserve_tcp_port;
use tide_disco::{Api, App, Error, StatusCode, error::ServerError};
use tokio::{spawn, task::JoinHandle, time::sleep};
use url::Url;
use vbs::version::{StaticVersion, StaticVersionType};
use versions::{EPOCH_VERSION, Upgrade};

use super::*;
use crate::{
    catchup::NullStateCatchup,
    network,
    persistence::no_storage,
    testing::{TestConfig, TestConfigBuilder, run_legacy_builder, wait_for_decide_on_handle},
};

pub const STAKE_TABLE_CAPACITY_FOR_TEST: usize = 10;

pub struct TestNetwork<P: PersistenceOptions, const NUM_NODES: usize> {
    pub server: SequencerContext<network::Memory, P::Persistence>,
    pub peers: Vec<SequencerContext<network::Memory, P::Persistence>>,
    pub cfg: TestConfig<{ NUM_NODES }>,
    // todo (abdul): remove this when fs storage is removed
    pub temp_dir: Option<TempDir>,
    pub contracts: Option<Contracts>,
}

pub struct TestNetworkConfig<const NUM_NODES: usize, P, C>
where
    P: PersistenceOptions,
    C: StateCatchup + 'static,
{
    state: [ValidatedState; NUM_NODES],
    persistence: [P; NUM_NODES],
    catchup: [C; NUM_NODES],
    network_config: TestConfig<{ NUM_NODES }>,
    api_config: Options,
    contracts: Option<Contracts>,
}

impl<const NUM_NODES: usize, P, C> TestNetworkConfig<{ NUM_NODES }, P, C>
where
    P: PersistenceOptions,
    C: StateCatchup + 'static,
{
    pub fn states(&self) -> [ValidatedState; NUM_NODES] {
        self.state.clone()
    }
}

#[derive(Clone)]
pub struct TestNetworkConfigBuilder<const NUM_NODES: usize, P, C>
where
    P: PersistenceOptions,
    C: StateCatchup + 'static,
{
    state: [ValidatedState; NUM_NODES],
    persistence: Option<[P; NUM_NODES]>,
    catchup: Option<[C; NUM_NODES]>,
    api_config: Option<Options>,
    network_config: Option<TestConfig<{ NUM_NODES }>>,
    contracts: Option<Contracts>,
    initial_token_supply: Option<U256>,
}

impl Default for TestNetworkConfigBuilder<5, no_storage::Options, NullStateCatchup> {
    fn default() -> Self {
        TestNetworkConfigBuilder {
            state: std::array::from_fn(|_| ValidatedState::default()),
            persistence: Some([no_storage::Options; 5]),
            catchup: Some(std::array::from_fn(|_| NullStateCatchup::default())),
            network_config: None,
            api_config: None,
            contracts: None,
            initial_token_supply: None,
        }
    }
}

impl<const NUM_NODES: usize>
    TestNetworkConfigBuilder<{ NUM_NODES }, no_storage::Options, NullStateCatchup>
{
    pub fn with_num_nodes()
    -> TestNetworkConfigBuilder<{ NUM_NODES }, no_storage::Options, NullStateCatchup> {
        TestNetworkConfigBuilder {
            state: std::array::from_fn(|_| ValidatedState::default()),
            persistence: Some([no_storage::Options; { NUM_NODES }]),
            catchup: Some(std::array::from_fn(|_| NullStateCatchup::default())),
            network_config: None,
            api_config: None,
            contracts: None,
            initial_token_supply: None,
        }
    }
}

impl<const NUM_NODES: usize, P, C> TestNetworkConfigBuilder<{ NUM_NODES }, P, C>
where
    P: PersistenceOptions,
    C: StateCatchup + 'static,
{
    pub fn states(mut self, state: [ValidatedState; NUM_NODES]) -> Self {
        self.state = state;
        self
    }

    pub fn initial_token_supply(mut self, supply: U256) -> Self {
        self.initial_token_supply = Some(supply);
        self
    }

    pub fn persistences<NP: PersistenceOptions>(
        self,
        persistence: [NP; NUM_NODES],
    ) -> TestNetworkConfigBuilder<{ NUM_NODES }, NP, C> {
        TestNetworkConfigBuilder {
            state: self.state,
            catchup: self.catchup,
            network_config: self.network_config,
            api_config: self.api_config,
            persistence: Some(persistence),
            contracts: self.contracts,
            initial_token_supply: self.initial_token_supply,
        }
    }

    pub fn api_config(mut self, api_config: Options) -> Self {
        self.api_config = Some(api_config);
        self
    }

    pub fn catchups<NC: StateCatchup + 'static>(
        self,
        catchup: [NC; NUM_NODES],
    ) -> TestNetworkConfigBuilder<{ NUM_NODES }, P, NC> {
        TestNetworkConfigBuilder {
            state: self.state,
            catchup: Some(catchup),
            network_config: self.network_config,
            api_config: self.api_config,
            persistence: self.persistence,
            contracts: self.contracts,
            initial_token_supply: self.initial_token_supply,
        }
    }

    pub fn network_config(mut self, network_config: TestConfig<{ NUM_NODES }>) -> Self {
        self.network_config = Some(network_config);
        self
    }

    pub fn contracts(mut self, contracts: Contracts) -> Self {
        self.contracts = Some(contracts);
        self
    }

    /// Setup for POS testing. Deploys contracts and adds the
    /// stake table address to state. Must be called before `build()`.
    pub async fn pos_hook(
        self,
        delegation_config: DelegationConfig,
        stake_table_version: StakeTableContractVersion,
        upgrade: Upgrade,
    ) -> anyhow::Result<Self> {
        if upgrade.base < EPOCH_VERSION && upgrade.target < EPOCH_VERSION {
            panic!("given version does not require pos deployment");
        };

        let network_config = self
            .network_config
            .as_ref()
            .expect("network_config is required");

        let l1_url = network_config.l1_url();
        let signer = network_config.signer();
        let deployer = ProviderBuilder::new()
            .wallet(EthereumWallet::from(signer.clone()))
            .connect_http(l1_url.clone());

        let blocks_per_epoch = network_config.hotshot_config().epoch_height;
        let epoch_start_block = network_config.hotshot_config().epoch_start_block;
        let (genesis_state, genesis_stake) = light_client_genesis_from_stake_table(
            &network_config.hotshot_config().hotshot_stake_table(),
            STAKE_TABLE_CAPACITY_FOR_TEST,
        )
        .unwrap();

        let mut contracts = Contracts::new();
        let args = DeployerArgsBuilder::default()
            .deployer(deployer.clone())
            .rpc_url(l1_url.clone())
            .mock_light_client(true)
            .genesis_lc_state(genesis_state)
            .genesis_st_state(genesis_stake)
            .blocks_per_epoch(blocks_per_epoch)
            .epoch_start_block(epoch_start_block)
            .exit_escrow_period(U256::from(max(
                blocks_per_epoch * 15 + 100,
                DEFAULT_EXIT_ESCROW_PERIOD_SECONDS,
            )))
            .multisig_pauser(signer.address())
            .token_name("Espresso".to_string())
            .token_symbol("ESP".to_string())
            .initial_token_supply(self.initial_token_supply.unwrap_or(U256::from(100000u64)))
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
            StakeTableContractVersion::V2 => args.deploy_to_stake_table_v2(&mut contracts).await,
            StakeTableContractVersion::V3 => args.deploy_to_stake_table_v3(&mut contracts).await,
        }
        .context("failed to deploy contracts")?;

        let stake_table_address = contracts
            .address(Contract::StakeTableProxy)
            .expect("StakeTableProxy address not found");

        StakingTransactions::create(
            l1_url.clone(),
            &deployer,
            stake_table_address,
            network_config.staking_priv_keys(),
            None,
            delegation_config,
        )
        .await
        .expect("stake table setup failed")
        .apply_all()
        .await
        .expect("send all txns failed");

        // enable interval mining with a 1s interval.
        // This ensures that blocks are finalized every second, even when there are no transactions.
        // It's useful for testing stake table updates,
        // which rely on the finalized L1 block number.
        if let Some(anvil) = network_config.anvil() {
            anvil
                .anvil_set_interval_mining(1)
                .await
                .expect("interval mining");
        }

        // Add stake table address to `ChainConfig` (held in state),
        // avoiding overwrite other values. Base fee is set to `0` to avoid
        // unnecessary catchup of `FeeState`.
        let state = self.state[0].clone();
        let chain_config = if let Some(cf) = state.chain_config.resolve() {
            ChainConfig {
                base_fee: 0.into(),
                stake_table_contract: Some(stake_table_address),
                ..cf
            }
        } else {
            ChainConfig {
                base_fee: 0.into(),
                stake_table_contract: Some(stake_table_address),
                ..Default::default()
            }
        };

        let state = ValidatedState {
            chain_config: chain_config.into(),
            ..state
        };
        Ok(self
            .states(std::array::from_fn(|_| state.clone()))
            .contracts(contracts))
    }

    pub fn build(self) -> TestNetworkConfig<{ NUM_NODES }, P, C> {
        TestNetworkConfig {
            state: self.state,
            persistence: self.persistence.unwrap(),
            catchup: self.catchup.unwrap(),
            network_config: self.network_config.unwrap(),
            api_config: self.api_config.unwrap(),
            contracts: self.contracts,
        }
    }
}

impl<P: PersistenceOptions, const NUM_NODES: usize> TestNetwork<P, { NUM_NODES }> {
    pub async fn new<C: StateCatchup + 'static>(
        cfg: TestNetworkConfig<{ NUM_NODES }, P, C>,
        upgrade: versions::Upgrade,
    ) -> Self {
        let mut cfg = cfg;
        let mut builder_tasks = Vec::new();

        let chain_config = cfg.state[0].chain_config.resolve();
        if chain_config.is_none() {
            tracing::warn!("Chain config is not set, using default max_block_size");
        }
        let (task, builder_url) = run_legacy_builder::<{ NUM_NODES }>(
            cfg.network_config.builder_port(),
            chain_config.map(|c| *c.max_block_size),
        )
        .await;
        builder_tasks.push(task);
        cfg.network_config
            .set_builder_urls(vec1::vec1![builder_url.clone()]);

        // add default storage if none is provided as query module is now required
        let mut opt = cfg.api_config.clone();
        let temp_dir = if opt.storage_fs.is_none() && opt.storage_sql.is_none() {
            let temp_dir = tempfile::tempdir().unwrap();
            opt = opt.query_fs(
                Default::default(),
                crate::persistence::fs::Options::new(temp_dir.path().to_path_buf()),
            );
            Some(temp_dir)
        } else {
            None
        };

        let mut nodes = join_all(
            izip!(cfg.state, cfg.persistence, cfg.catchup)
                .enumerate()
                .map(|(i, (state, persistence, state_peers))| {
                    let opt = opt.clone();
                    let cfg = &cfg.network_config;
                    let upgrades_map = cfg.upgrades();
                    async move {
                        if i == 0 {
                            opt.serve(|metrics, consumer, storage| {
                                let cfg = cfg.clone();
                                async move {
                                    Ok(cfg
                                        .init_node(
                                            0,
                                            state,
                                            persistence,
                                            Some(state_peers),
                                            storage,
                                            &*metrics,
                                            STAKE_TABLE_CAPACITY_FOR_TEST,
                                            consumer,
                                            upgrade,
                                            upgrades_map,
                                        )
                                        .await)
                                }
                                .boxed()
                            })
                            .await
                            .unwrap()
                        } else {
                            cfg.init_node(
                                i,
                                state,
                                persistence,
                                Some(state_peers),
                                None,
                                &NoMetrics,
                                STAKE_TABLE_CAPACITY_FOR_TEST,
                                NullEventConsumer,
                                upgrade,
                                upgrades_map,
                            )
                            .await
                        }
                    }
                    .boxed()
                }),
        )
        .await;

        let handle_0 = &nodes[0];

        // Hook the builder(s) up to the event stream from the first node
        for builder_task in builder_tasks {
            builder_task.start(Box::new(
                handle_0
                    .consensus_handle()
                    .legacy_consensus()
                    .read()
                    .await
                    .event_stream(),
            ));
        }

        for ctx in &nodes {
            ctx.start_consensus().await;
        }

        let server = nodes.remove(0);
        let peers = nodes;

        Self {
            server,
            peers,
            cfg: cfg.network_config,
            temp_dir,
            contracts: cfg.contracts,
        }
    }

    pub async fn stop_consensus(&mut self) {
        self.server.shutdown_consensus().await;

        for ctx in &mut self.peers {
            ctx.shutdown_consensus().await;
        }
    }
}

/// Test the status API with custom options.
///
/// The `opt` function can be used to modify the [`Options`] which are used to start the server.
/// By default, the options are the minimal required to run this test (configuring a port and
/// enabling the status API). `opt` may add additional functionality (e.g. adding a query module
/// to test a different initialization path) but should not remove or modify the existing
/// functionality (e.g. removing the status module or changing the port).
pub async fn status_test_helper(opt: impl FnOnce(Options) -> Options) {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

    let options = opt(Options::with_port(port));
    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(options)
        .network_config(network_config)
        .build();
    let _network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    client.connect(None).await;

    // The status API is well tested in the query service repo. Here we are just smoke testing
    // that we set it up correctly. Wait for a (non-genesis) block to be sequenced and then
    // check the success rate metrics.
    while client
        .get::<u64>("status/block-height")
        .send()
        .await
        .unwrap()
        <= 1
    {
        sleep(Duration::from_secs(1)).await;
    }
    let success_rate = client
        .get::<f64>("status/success-rate")
        .send()
        .await
        .unwrap();
    // If metrics are populating correctly, we should get a finite number. If not, we might get
    // NaN or infinity due to division by 0.
    assert!(success_rate.is_finite(), "{success_rate}");
    // We know at least some views have been successful, since we finalized a block.
    assert!(success_rate > 0.0, "{success_rate}");
}

/// Test the submit API with custom options.
///
/// The `opt` function can be used to modify the [`Options`] which are used to start the server.
/// By default, the options are the minimal required to run this test (configuring a port and
/// enabling the submit API). `opt` may add additional functionality (e.g. adding a query module
/// to test a different initialization path) but should not remove or modify the existing
/// functionality (e.g. removing the submit module or changing the port).
pub async fn submit_test_helper(opt: impl FnOnce(Options) -> Options) {
    let txn = Transaction::new(NamespaceId::from(1_u32), vec![1, 2, 3, 4]);

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

    let options = opt(Options::with_port(port).submit(Default::default()));
    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(options)
        .network_config(network_config)
        .build();
    let network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    let mut events = network.server.event_stream();

    client.connect(None).await;

    let hash = client
        .post("submit/submit")
        .body_json(&txn)
        .unwrap()
        .send()
        .await
        .unwrap();
    assert_eq!(txn.commit(), hash);

    // Wait for a Decide event containing transaction matching the one we sent
    wait_for_decide_on_handle(&mut events, &txn).await;
}

/// Test the state signature API.
pub async fn state_signature_test_helper(opt: impl FnOnce(Options) -> Options) {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let url = format!("http://localhost:{port}").parse().unwrap();

    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

    let options = opt(Options::with_port(port));
    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(options)
        .network_config(network_config)
        .build();
    let network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;

    let mut height: u64;
    // Wait for block >=2 appears
    // It's waiting for an extra second to make sure that the signature is generated
    loop {
        height = network.server.decided_leaf().await.height();
        sleep(std::time::Duration::from_secs(1)).await;
        if height >= 2 {
            break;
        }
    }
    // we cannot verify the signature now, because we don't know the stake table
    client
        .get::<LCV3StateSignatureRequestBody>(&format!("state-signature/block/{height}"))
        .send()
        .await
        .unwrap();
}

/// Test the catchup API with custom options.
///
/// The `opt` function can be used to modify the [`Options`] which are used to start the server.
/// By default, the options are the minimal required to run this test (configuring a port and
/// enabling the catchup API). `opt` may add additional functionality (e.g. adding a query module
/// to test a different initialization path) but should not remove or modify the existing
/// functionality (e.g. removing the catchup module or changing the port).
pub async fn catchup_test_helper(opt: impl FnOnce(Options) -> Options) {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

    let options = opt(Options::with_port(port));
    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(options)
        .network_config(network_config)
        .build();
    let network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    client.connect(None).await;

    // Wait for a few blocks to be decided.
    let mut events = network.server.event_stream();
    loop {
        if let CoordinatorEvent::LegacyEvent(Event {
            event: EventType::Decide { leaf_chain, .. },
            ..
        }) = events.next().await.unwrap()
            && leaf_chain
                .iter()
                .any(|LeafInfo { leaf, .. }| leaf.block_header().height() > 2)
        {
            break;
        }
    }

    // Stop consensus running on the node so we freeze the decided and undecided states.
    // We'll let it go out of scope here since it's a write lock.
    {
        network.server.shutdown_consensus().await;
    }

    // Undecided fee state: absent account.
    let leaf = network.server.decided_leaf().await;
    let height = leaf.height() + 1;
    let view = leaf.view_number() + 1;
    let res = client
        .get::<AccountQueryData>(&format!(
            "catchup/{height}/{}/account/{:x}",
            view.u64(),
            Address::default()
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.balance, U256::ZERO);
    assert_eq!(
        res.proof
            .verify(
                &network
                    .server
                    .state(view)
                    .await
                    .unwrap()
                    .fee_merkle_tree
                    .commitment()
            )
            .unwrap(),
        U256::ZERO,
    );

    // Undecided block state.
    let res = client
        .get::<BlocksFrontier>(&format!("catchup/{height}/{}/blocks", view.u64()))
        .send()
        .await
        .unwrap();
    let root = &network
        .server
        .state(view)
        .await
        .unwrap()
        .block_merkle_tree
        .commitment();
    BlockMerkleTree::verify(root, root.size() - 1, res)
        .unwrap()
        .unwrap();
}

pub async fn spawn_dishonest_peer_catchup_api() -> anyhow::Result<(Url, JoinHandle<()>)> {
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/catchup.toml")).unwrap();
    let mut api = Api::<(), hotshot_query_service::Error, SequencerApiVersion>::new(toml).unwrap();

    api.get("account", |_req, _state: &()| {
        async move {
            Result::<AccountQueryData, _>::Err(hotshot_query_service::Error::catch_all(
                StatusCode::BAD_REQUEST,
                "no account found".to_string(),
            ))
        }
        .boxed()
    })?
    .get("blocks", |_req, _state| {
        async move {
            Result::<BlocksFrontier, _>::Err(hotshot_query_service::Error::catch_all(
                StatusCode::BAD_REQUEST,
                "no block found".to_string(),
            ))
        }
        .boxed()
    })?
    .get("chainconfig", |_req, _state| {
        async move {
            Result::<ChainConfig, _>::Ok(ChainConfig {
                max_block_size: 300.into(),
                base_fee: 1.into(),
                fee_recipient: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
                    .parse()
                    .unwrap(),
                ..Default::default()
            })
        }
        .boxed()
    })?
    .get("leafchain", |_req, _state| {
        async move {
            Result::<Vec<Leaf2>, _>::Err(hotshot_query_service::Error::catch_all(
                StatusCode::BAD_REQUEST,
                "No leafchain found".to_string(),
            ))
        }
        .boxed()
    })?;

    let mut app = App::<_, hotshot_query_service::Error>::with_state(());
    app.with_version(env!("CARGO_PKG_VERSION").parse().unwrap());

    app.register_module::<_, _>("catchup", api).unwrap();

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let url: Url = Url::parse(&format!("http://localhost:{port}")).unwrap();

    let handle = spawn({
        let url = url.clone();
        async move {
            let _ = app.serve(url, SequencerApiVersion::instance()).await;
        }
    });

    Ok((url, handle))
}
