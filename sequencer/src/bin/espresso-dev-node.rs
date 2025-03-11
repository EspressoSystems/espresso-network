use std::{collections::BTreeMap, io, iter::once, sync::Arc, time::Duration};

use async_trait::async_trait;
use clap::Parser;
use contract_bindings_ethers::light_client_mock::LightClientMock;
use espresso_types::{parse_duration, MarketplaceVersion, SequencerVersions, V0_1};
use ethers::{
    middleware::{MiddlewareBuilder, SignerMiddleware},
    providers::{Http, Middleware, Provider},
    signers::{coins_bip39::English, MnemonicBuilder, Signer},
    types::{Address, H160, U256},
};
use futures::{future::BoxFuture, stream::FuturesUnordered, FutureExt, StreamExt};
use hotshot_state_prover::service::{
    one_honest_threshold, run_prover_service_with_stake_table, StateProverConfig,
};
use hotshot_types::traits::stake_table::{SnapshotVersion, StakeTableScheme};
use portpicker::pick_unused_port;
use sequencer::{
    api::{
        options,
        test_helpers::{TestNetwork, TestNetworkConfigBuilder, STAKE_TABLE_CAPACITY_FOR_TEST},
    },
    persistence,
    state_signature::relay_server::run_relay_server,
    testing::TestConfigBuilder,
    SequencerApiVersion,
};
use sequencer_utils::{
    deployer::{deploy, is_proxy_contract, Contract, Contracts},
    logging, AnvilOptions,
};
use serde::{Deserialize, Serialize};
use tide_disco::{error::ServerError, method::ReadState, Api, Error as _, StatusCode};
use tokio::spawn;
use url::Url;
use vbs::version::StaticVersionType;

#[derive(Clone, Debug, Parser)]
struct Args {
    /// A JSON-RPC endpoint for the L1 to deploy to.
    /// If this is not provided, an Avil node will be  launched automatically.
    #[arg(short, long, env = "ESPRESSO_SEQUENCER_L1_PROVIDER")]
    rpc_url: Option<Url>,

    /// Request rate when polling L1.
    #[arg(
        short,
        long,
        env = "ESPRESSO_SEQUENCER_L1_POLLING_INTERVAL",
        default_value = "7s",
        value_parser = parse_duration
    )]
    l1_interval: Duration,

    /// Mnemonic for an L1 wallet.
    ///
    /// This wallet is used to deploy the contracts,
    /// so the account indicated by ACCOUNT_INDEX must be funded with with ETH.
    #[arg(
        long,
        name = "MNEMONIC",
        env = "ESPRESSO_SEQUENCER_ETH_MNEMONIC",
        default_value = "test test test test test test test test test test test junk"
    )]
    mnemonic: String,
    /// Account index in the L1 wallet generated by MNEMONIC to use when deploying the contracts.
    #[arg(
        long,
        name = "ACCOUNT_INDEX",
        env = "ESPRESSO_DEPLOYER_ACCOUNT_INDEX",
        default_value = "0"
    )]
    account_index: u32,

    /// Address for the multisig wallet that will be the admin
    ///
    /// This the multisig wallet that will be upgrade contracts and execute admin only functions on contracts
    #[arg(
        long,
        name = "MULTISIG_ADDRESS",
        env = "ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS"
    )]
    multisig_address: Option<H160>,

    /// The frequency of updating the light client state, expressed in update interval
    #[arg( long, value_parser = parse_duration, default_value = "20s", env = "ESPRESSO_STATE_PROVER_UPDATE_INTERVAL")]
    update_interval: Duration,

    /// Interval between retries if a state update fails
    #[arg(long , value_parser = parse_duration, default_value = "2s", env = "ESPRESSO_STATE_PROVER_RETRY_INTERVAL")]
    retry_interval: Duration,

    /// Optional list of URLs representing alternate chains
    /// where the dev node will deploy LC contracts and submit LC state updates.
    ///
    /// Useful for test environments involving L3s.
    #[arg(long, env = "ESPRESSO_DEPLOYER_ALT_CHAIN_PROVIDERS", num_args = 1.., value_delimiter = ',')]
    alt_chain_providers: Vec<Url>,

    /// Optional list of mnemonics for the alternate chains.
    /// If there are fewer mnemonics provided than chains, the base MNEMONIC will be used.
    #[arg(long, env = "ESPRESSO_DEPLOYER_ALT_MNEMONICS", num_args = 1.., value_delimiter = ',')]
    alt_mnemonics: Vec<String>,

    /// Alternate account indices generated by the mnemonics to use when deploying the contracts.
    /// If there are fewer indices provided than chains, the base ACCOUNT_INDEX will be used.
    #[arg(long, env = "ESPRESSO_SEQUENCER_DEPLOYER_ALT_INDICES")]
    alt_account_indices: Vec<u32>,

    /// Optional list of multisig addresses for the alternate chains.
    /// If there are fewer multisig addresses provided than chains, the base MULTISIG_ADDRESS will be used.
    #[arg(long, env = "ESPRESSO_DEPLOYER_ALT_MULTISIG_ADDRESSES", num_args = 1.., value_delimiter = ',')]
    alt_multisig_addresses: Vec<H160>,

    /// The frequency of updating the light client state for alt chains.
    /// If there are fewer intervals provided than chains, the base update interval will be used.
    #[arg(long, value_parser = parse_duration, env = "ESPRESSO_STATE_PROVER_ALT_UPDATE_INTERVALS", num_args = 1.., value_delimiter = ',')]
    alt_prover_update_intervals: Vec<Duration>,

    /// Interval between retries if a state update fails for alt chains
    /// If there are fewer intervals provided than chains, the base update interval will be used.
    #[arg(long, value_parser = parse_duration, env = "ESPRESSO_STATE_PROVER_ALT_RETRY_INTERVALS", num_args = 1.., value_delimiter = ',')]
    alt_prover_retry_intervals: Vec<Duration>,

    /// Port that the HTTP API will use.
    #[arg(long, env = "ESPRESSO_SEQUENCER_API_PORT")]
    sequencer_api_port: u16,

    /// Maximum concurrent connections allowed by the HTTP API server.
    #[arg(long, env = "ESPRESSO_SEQUENCER_MAX_CONNECTIONS")]
    sequencer_api_max_connections: Option<usize>,

    /// Port for connecting to the builder.
    #[arg(short, long, env = "ESPRESSO_BUILDER_PORT")]
    builder_port: Option<u16>,

    /// Port for connecting to the prover.
    #[arg(short, long, env = "ESPRESSO_PROVER_PORT")]
    prover_port: Option<u16>,

    /// Port for the dev node.
    ///
    /// This is used to provide tools and information to facilitate developers debugging.
    #[arg(short, long, env = "ESPRESSO_DEV_NODE_PORT", default_value = "20000")]
    dev_node_port: u16,

    /// Port for connecting to the builder.
    #[arg(
        short,
        long,
        env = "ESPRESSO_DEV_NODE_MAX_BLOCK_SIZE",
        default_value = "1000000"
    )]
    max_block_size: u64,

    #[command(flatten)]
    sql: persistence::sql::Options,

    #[command(flatten)]
    logging: logging::Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli_params = Args::parse();

    let Args {
        rpc_url,
        mnemonic,
        account_index,
        multisig_address,
        alt_chain_providers,
        alt_mnemonics,
        alt_account_indices,
        alt_multisig_addresses,
        sequencer_api_port,
        sequencer_api_max_connections,
        builder_port,
        prover_port,
        dev_node_port,
        sql,
        logging,
        update_interval,
        retry_interval,
        alt_prover_retry_intervals,
        alt_prover_update_intervals,
        l1_interval,
        max_block_size,
    } = cli_params;

    logging.init();

    let api_options = options::Options::from(options::Http {
        port: sequencer_api_port,
        max_connections: sequencer_api_max_connections,
    })
    .submit(Default::default())
    .query_sql(Default::default(), sql);

    let (l1_url, _anvil) = if let Some(url) = rpc_url {
        (url, None)
    } else {
        tracing::warn!("L1 url is not provided. running an anvil node");
        let instance = AnvilOptions::default().spawn().await;
        let url = instance.url();
        tracing::info!("l1 url: {}", url);
        (url, Some(instance))
    };

    let relay_server_port = pick_unused_port().unwrap();
    let relay_server_url: Url = format!("http://localhost:{}", relay_server_port)
        .parse()
        .unwrap();

    let network_config = TestConfigBuilder::default()
        .marketplace_builder_port(builder_port)
        .state_relay_url(relay_server_url.clone())
        .l1_url(l1_url.clone())
        .build();

    const NUM_NODES: usize = 2;
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(api_options)
        .network_config(network_config)
        .with_max_block_size(max_block_size)
        .build();

    let network =
        TestNetwork::new(config, SequencerVersions::<MarketplaceVersion, V0_1>::new()).await;
    let st = network.cfg.stake_table();
    let total_stake = st.total_stake(SnapshotVersion::LastEpochStart).unwrap();
    let config = network.cfg.hotshot_config();

    tracing::info!("Hotshot config {config:?}");

    let lc_genesis = network.light_client_genesis();

    let contracts = Contracts::new();
    let mut light_client_addresses = vec![];
    let mut prover_ports = Vec::new();
    let mut mock_contracts = BTreeMap::new();
    let mut handles = FuturesUnordered::new();
    // deploy contract for L1 and each alt chain
    for (url, mnemonic, account_index, multisig_address, update_interval, retry_interval) in once((
        l1_url.clone(),
        mnemonic.clone(),
        account_index,
        multisig_address,
        update_interval,
        retry_interval,
    ))
    .chain(
        alt_chain_providers
            .iter()
            .zip(alt_mnemonics.into_iter().chain(std::iter::repeat(mnemonic)))
            .zip(
                alt_account_indices
                    .into_iter()
                    .chain(std::iter::repeat(account_index)),
            )
            .zip(
                alt_multisig_addresses
                    .into_iter()
                    .map(Some)
                    .chain(std::iter::repeat(multisig_address)),
            )
            .zip(
                alt_prover_update_intervals
                    .into_iter()
                    .chain(std::iter::repeat(update_interval)),
            )
            .zip(
                alt_prover_retry_intervals
                    .into_iter()
                    .chain(std::iter::repeat(retry_interval)),
            )
            .map(|(((((url, mnc), idx), mlts), update), retry)| {
                (url.clone(), mnc, idx, mlts, update, retry)
            }),
    ) {
        tracing::info!("deploying the contract for provider: {url:?}");

        let contracts = deploy(
            url.clone(),
            l1_interval,
            mnemonic.clone(),
            account_index,
            multisig_address,
            true,
            None,
            async { Ok(lc_genesis.clone()) }.boxed(),
            None,
            contracts.clone(),
            None, // initial stake table
        )
        .await?;

        let provider = Provider::<Http>::try_from(url.as_str())
            .unwrap()
            .interval(l1_interval);
        let chain_id = provider.get_chainid().await.unwrap().as_u64();

        let wallet = MnemonicBuilder::<English>::default()
            .phrase(mnemonic.as_str())
            .index(account_index)
            .expect("error building wallet")
            .build()
            .expect("error opening wallet")
            .with_chain_id(chain_id);

        let light_client_address = contracts
            .get_contract_address(Contract::LightClientProxy)
            .unwrap();

        if !is_proxy_contract(&provider, light_client_address)
            .await
            .expect("Failed to determine if light client contract is a proxy")
        {
            panic!("Light Client contract's address is not a proxy");
        }

        mock_contracts.insert(
            chain_id,
            LightClientMock::new(
                light_client_address,
                Arc::new(provider.with_signer(wallet.clone())),
            ),
        );
        light_client_addresses.push((chain_id, light_client_address));

        let prover_port = prover_port.unwrap_or_else(|| pick_unused_port().unwrap());

        prover_ports.push(prover_port);

        let prover_config = StateProverConfig {
            relay_server: relay_server_url.clone(),
            update_interval,
            retry_interval,
            sequencer_url: "http://localhost".parse().unwrap(),
            port: Some(prover_port),
            stake_table_capacity: STAKE_TABLE_CAPACITY_FOR_TEST as usize,
            provider: url.clone(),
            light_client_address,
            signing_key: wallet.signer().clone(),
        };

        let prover_handle = spawn(run_prover_service_with_stake_table(
            prover_config,
            SequencerApiVersion::instance(),
            Arc::new(st.clone()),
        ));
        handles.push(prover_handle);
    }

    let relay_server_handle = spawn(async move {
        let _ = run_relay_server(
            None,
            one_honest_threshold(total_stake),
            format!("http://0.0.0.0:{relay_server_port}")
                .parse()
                .unwrap(),
            SequencerApiVersion::instance(),
        )
        .await;

        Ok(())
    });
    handles.push(relay_server_handle);

    // we remove the first entry which is for L1 light client contract
    // so only alt chain light client addresses are left
    let (_, l1_lc) = light_client_addresses.remove(0);
    let l1_prover_port = prover_ports.remove(0);
    // we remove the first entry which is for primary L1 chain signing key
    // so only alt signing keys are left

    let dev_info = DevInfo {
        builder_url: network.cfg.hotshot_config().builder_urls[0].clone(),
        sequencer_api_port,
        l1_prover_port,
        l1_url,
        l1_light_client_address: l1_lc,
        alt_chains: alt_chain_providers
            .into_iter()
            .zip(light_client_addresses)
            .zip(prover_ports)
            .map(
                |((provider_url, (chain_id, light_client_address)), prover_port)| AltChainInfo {
                    chain_id,
                    provider_url,
                    light_client_address,
                    prover_port,
                },
            )
            .collect(),
    };

    let dev_node_handle = spawn(run_dev_node_server(
        dev_node_port,
        mock_contracts,
        dev_info,
        SequencerApiVersion::instance(),
    ));
    handles.push(dev_node_handle);

    // if any of the async task is complete then dev node binary exits
    if (handles.next().await).is_some() {
        tracing::error!("exiting dev node");
        drop(network);
    }

    Ok(())
}

// ApiState is passed to the tide disco app so avoid cloning the contracts for each endpoint
pub struct ApiState<S: Signer + Clone + 'static>(
    BTreeMap<u64, LightClientMock<SignerMiddleware<Provider<Http>, S>>>,
);

#[async_trait]
impl<S: Signer + Clone + 'static> ReadState for ApiState<S> {
    type State = ApiState<S>;
    async fn read<T>(
        &self,
        op: impl Send + for<'a> FnOnce(&'a Self::State) -> BoxFuture<'a, T> + 'async_trait,
    ) -> T {
        op(self).await
    }
}

async fn run_dev_node_server<ApiVer: StaticVersionType + 'static, S: Signer + Clone + 'static>(
    port: u16,
    contracts: BTreeMap<u64, LightClientMock<SignerMiddleware<Provider<Http>, S>>>,
    dev_info: DevInfo,
    bind_version: ApiVer,
) -> anyhow::Result<()> {
    let mut app = tide_disco::App::<_, ServerError>::with_state(ApiState(contracts));
    let toml =
        toml::from_str::<toml::value::Value>(include_str!("../../api/espresso_dev_node.toml"))
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    let mut api = Api::<_, ServerError, ApiVer>::new(toml)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    api.get("devinfo", move |_, _| {
        let info = dev_info.clone();
        async move { Ok(info.clone()) }.boxed()
    })
    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?
    .at("sethotshotdown", move |req, state: &ApiState<S>| {
        async move {
            let body = req
                .body_auto::<SetHotshotDownReqBody, ApiVer>(ApiVer::instance())
                .map_err(ServerError::from_request_error)?;

            // if chain id is not provided, primary L1 light client is used
            let contract = if let Some(chain_id) = body.chain_id {
                state.0.get(&chain_id).ok_or_else(|| {
                    ServerError::catch_all(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "light client contract not found for chain id {chain_id}".to_string(),
                    )
                })?
            } else {
                let (_, contract) = state.0.first_key_value().ok_or_else(|| {
                    ServerError::catch_all(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "L1 light client contract not found ".to_string(),
                    )
                })?;

                contract
            };

            let contract_call = contract.set_hot_shot_down_since(U256::from(body.height));

            contract_call.send().await.map_err(|err| {
                ServerError::catch_all(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            })?;
            Ok(())
        }
        .boxed()
    })
    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?
    .at("sethotshotup", move |req, state| {
        async move {
            let chain_id = req
                .body_auto::<Option<SetHotshotUpReqBody>, ApiVer>(ApiVer::instance())
                .map_err(ServerError::from_request_error)?
                .map(|b| b.chain_id);

            // if chain id is not provided, we use the base L1 light client contract
            let contract = if let Some(chain_id) = chain_id {
                state.0.get(&chain_id).ok_or_else(|| {
                    ServerError::catch_all(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "light client ontract not found".to_string(),
                    )
                })?
            } else {
                let (_, light_client_address) = state.0.first_key_value().ok_or_else(|| {
                    ServerError::catch_all(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "l1 light client ontract not found".to_string(),
                    )
                })?;

                light_client_address
            };

            contract.set_hot_shot_up().send().await.map_err(|err| {
                ServerError::catch_all(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            })?;
            Ok(())
        }
        .boxed()
    })
    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    app.register_module("api", api)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    app.serve(format!("0.0.0.0:{port}"), bind_version).await?;

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DevInfo {
    pub builder_url: Url,
    pub sequencer_api_port: u16,
    pub l1_prover_port: u16,
    pub l1_url: Url,
    pub l1_light_client_address: Address,
    pub alt_chains: Vec<AltChainInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AltChainInfo {
    pub chain_id: u64,
    pub provider_url: Url,
    pub light_client_address: Address,
    pub prover_port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
struct SetHotshotDownReqBody {
    // return l1 light client address if not provided
    pub chain_id: Option<u64>,
    pub height: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SetHotshotUpReqBody {
    pub chain_id: u64,
}

#[cfg(test)]
mod tests {
    use std::{process::Child, sync::Arc, time::Duration};

    use committable::{Commitment, Committable};
    use contract_bindings_ethers::light_client::LightClient;
    use escargot::CargoBuild;
    use espresso_types::{BlockMerkleTree, Header, SeqTypes, Transaction};
    use ethers::{providers::Middleware, types::U256};
    use futures::{StreamExt, TryStreamExt};
    use hotshot_query_service::availability::{
        BlockQueryData, TransactionQueryData, VidCommonQueryData,
    };
    use jf_merkle_tree::MerkleTreeScheme;
    use portpicker::pick_unused_port;
    use rand::Rng;
    use sequencer::{api::endpoints::NamespaceProofQueryData, SequencerApiVersion};
    use sequencer_utils::{init_signer, test_utils::setup_test, Anvil, AnvilOptions};
    use surf_disco::Client;
    use tide_disco::error::ServerError;
    use tokio::time::sleep;
    use url::Url;
    use vbs::version::StaticVersion;

    use crate::{AltChainInfo, DevInfo, SetHotshotDownReqBody, SetHotshotUpReqBody};

    const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";
    const NUM_ALT_CHAIN_PROVIDERS: usize = 1;

    pub struct BackgroundProcess(Child);

    impl Drop for BackgroundProcess {
        fn drop(&mut self) {
            self.0.kill().unwrap();
        }
    }

    // If this test failed and you are doing changes on the following stuff, please
    // sync your changes to [`espresso-sequencer-go`](https://github.com/EspressoSystems/espresso-sequencer-go)
    // and open a PR.
    // - APIs update
    // - Types (like `Header`) update
    #[tokio::test(flavor = "multi_thread")]
    async fn slow_dev_node_test() {
        setup_test();

        let builder_port = pick_unused_port().unwrap();
        let api_port = pick_unused_port().unwrap();
        let dev_node_port = pick_unused_port().unwrap();
        let instance = AnvilOptions::default().spawn().await;
        let l1_url = instance.url();

        let tmp_dir = tempfile::tempdir().unwrap();

        let process = CargoBuild::new()
            .bin("espresso-dev-node")
            .features("testing embedded-db")
            .current_target()
            .run()
            .unwrap()
            .command()
            .env("ESPRESSO_SEQUENCER_L1_PROVIDER", l1_url.to_string())
            .env("ESPRESSO_BUILDER_PORT", builder_port.to_string())
            .env("ESPRESSO_SEQUENCER_API_PORT", api_port.to_string())
            .env("ESPRESSO_SEQUENCER_ETH_MNEMONIC", TEST_MNEMONIC)
            .env("ESPRESSO_DEPLOYER_ACCOUNT_INDEX", "0")
            .env("ESPRESSO_DEV_NODE_PORT", dev_node_port.to_string())
            .env(
                "ESPRESSO_SEQUENCER_STORAGE_PATH",
                tmp_dir.path().as_os_str(),
            )
            .env("ESPRESSO_SEQUENCER_DATABASE_MAX_CONNECTIONS", "25")
            .env("ESPRESSO_DEV_NODE_MAX_BLOCK_SIZE", "500000")
            .spawn()
            .unwrap();

        let process = BackgroundProcess(process);

        let api_client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{api_port}").parse().unwrap());
        api_client.connect(None).await;

        tracing::info!("waiting for blocks");
        let _ = api_client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(5)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        let builder_api_client: Client<ServerError, StaticVersion<0, 1>> =
            Client::new(format!("http://localhost:{builder_port}").parse().unwrap());
        builder_api_client.connect(None).await;

        let tx = Transaction::new(100_u32.into(), vec![1, 2, 3]);

        let hash: Commitment<Transaction> = builder_api_client
            .post("txn_submit/submit")
            .body_json(&tx)
            .unwrap()
            .send()
            .await
            .unwrap();

        let tx_hash = tx.commit();
        assert_eq!(hash, tx_hash);

        let mut tx_result = api_client
            .get::<TransactionQueryData<SeqTypes>>(&format!(
                "availability/transaction/hash/{tx_hash}",
            ))
            .send()
            .await;
        while tx_result.is_err() {
            sleep(Duration::from_secs(1)).await;
            tracing::warn!("waiting for tx");

            tx_result = api_client
                .get::<TransactionQueryData<SeqTypes>>(&format!(
                    "availability/transaction/hash/{}",
                    tx_hash
                ))
                .send()
                .await;
        }

        let large_tx = Transaction::new(100_u32.into(), vec![0; 20000]);
        let large_hash: Commitment<Transaction> = api_client
            .post("submit/submit")
            .body_json(&large_tx)
            .unwrap()
            .send()
            .await
            .unwrap();

        let tx_hash = large_tx.commit();
        assert_eq!(large_hash, tx_hash);

        let mut tx_result = api_client
            .get::<TransactionQueryData<SeqTypes>>(&format!(
                "availability/transaction/hash/{tx_hash}",
            ))
            .send()
            .await;
        while tx_result.is_err() {
            tracing::info!("waiting for large tx");
            sleep(Duration::from_secs(1)).await;

            tx_result = api_client
                .get::<TransactionQueryData<SeqTypes>>(&format!(
                    "availability/transaction/hash/{}",
                    tx_hash
                ))
                .send()
                .await;
        }

        {
            // transactions with size larger than max_block_size result in an error
            let extremely_large_tx = Transaction::new(100_u32.into(), vec![0; 7 * 1000 * 1000]);
            api_client
                .post::<Commitment<Transaction>>("submit/submit")
                .body_json(&extremely_large_tx)
                .unwrap()
                .send()
                .await
                .unwrap_err();

            // Now we send a small transaction to make sure this transaction can be included in a hotshot block.
            let tx = Transaction::new(100_u32.into(), vec![0; 3]);
            let tx_hash: Commitment<Transaction> = api_client
                .post("submit/submit")
                .body_json(&tx)
                .unwrap()
                .send()
                .await
                .unwrap();

            let mut result = api_client
                .get::<TransactionQueryData<SeqTypes>>(&format!(
                    "availability/transaction/hash/{tx_hash}",
                ))
                .send()
                .await;
            while result.is_err() {
                sleep(Duration::from_secs(1)).await;

                result = api_client
                    .get::<TransactionQueryData<SeqTypes>>(&format!(
                        "availability/transaction/hash/{}",
                        tx_hash
                    ))
                    .send()
                    .await;
            }
        }

        let tx_block_height = tx_result.unwrap().block_height();

        // Check the namespace proof
        let proof = api_client
            .get::<NamespaceProofQueryData>(&format!(
                "availability/block/{tx_block_height}/namespace/100"
            ))
            .send()
            .await
            .unwrap();
        assert!(proof.proof.is_some());

        // These endpoints are currently used in `espresso-sequencer-go`. These checks
        // serve as reminders of syncing the API updates to go client repo when they change.
        {
            api_client
                .get::<u64>("status/block-height")
                .send()
                .await
                .unwrap();

            api_client
                .get::<Header>("availability/header/3")
                .send()
                .await
                .unwrap();

            api_client
                .get::<VidCommonQueryData<SeqTypes>>(&format!(
                    "availability/vid/common/{tx_block_height}"
                ))
                .send()
                .await
                .unwrap();

            while api_client
                .get::<<BlockMerkleTree as MerkleTreeScheme>::MembershipProof>("block-state/3/2")
                .send()
                .await
                .is_err()
            {
                sleep(Duration::from_secs(1)).await;
            }
        }

        let dev_node_client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{dev_node_port}").parse().unwrap());
        dev_node_client.connect(None).await;

        // Check the dev node api
        {
            tracing::info!("checking the dev node api");
            let dev_info = dev_node_client
                .get::<DevInfo>("api/dev-info")
                .send()
                .await
                .unwrap();

            let light_client_address = dev_info.l1_light_client_address;

            let signer = init_signer(&l1_url, TEST_MNEMONIC, 0).await.unwrap();
            let light_client = LightClient::new(light_client_address, Arc::new(signer.clone()));

            while light_client
                .get_hot_shot_commitment(U256::from(1))
                .call()
                .await
                .is_err()
            {
                tracing::info!("waiting for commitment");
                sleep(Duration::from_secs(3)).await;
            }

            let height = signer.get_block_number().await.unwrap().as_u64();
            dev_node_client
                .post::<()>("api/set-hotshot-down")
                .body_json(&SetHotshotDownReqBody {
                    chain_id: None,
                    height: height - 1,
                })
                .unwrap()
                .send()
                .await
                .unwrap();

            while !light_client
                .lag_over_escape_hatch_threshold(U256::from(height), U256::from(0))
                .call()
                .await
                .unwrap_or(false)
            {
                tracing::info!("waiting for setting hotshot down");
                sleep(Duration::from_secs(3)).await;
            }

            dev_node_client
                .post::<()>("api/set-hotshot-up")
                .body_json(&())
                .unwrap()
                .send()
                .await
                .unwrap();

            while light_client
                .lag_over_escape_hatch_threshold(U256::from(height), U256::from(0))
                .call()
                .await
                .unwrap_or(true)
            {
                tracing::info!("waiting for setting hotshot up");
                sleep(Duration::from_secs(3)).await;
            }
        }

        drop(process);
    }

    async fn alt_chain_providers() -> (Vec<Anvil>, Vec<Url>) {
        let mut providers = Vec::new();
        let mut urls = Vec::new();

        for _ in 0..NUM_ALT_CHAIN_PROVIDERS {
            let mut rng = rand::thread_rng();

            let anvil = AnvilOptions::default()
                .chain_id(rng.gen_range(2..u32::MAX) as u64)
                .spawn()
                .await;
            let url = anvil.url();

            providers.push(anvil);
            urls.push(url);
        }

        (providers, urls)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn slow_dev_node_multiple_lc_providers_test() {
        setup_test();

        let builder_port = pick_unused_port().unwrap();
        let api_port = pick_unused_port().unwrap();
        let dev_node_port = pick_unused_port().unwrap();

        let instance = AnvilOptions::default().chain_id(1).spawn().await;
        let l1_url = instance.url();

        let (alt_providers, alt_chain_urls) = alt_chain_providers().await;

        let alt_chains_env_value = alt_chain_urls
            .iter()
            .map(|url| url.as_str())
            .collect::<Vec<&str>>()
            .join(",");

        let tmp_dir = tempfile::tempdir().unwrap();

        let process = CargoBuild::new()
            .bin("espresso-dev-node")
            .features("testing embedded-db")
            .current_target()
            .run()
            .unwrap()
            .command()
            .env("ESPRESSO_SEQUENCER_L1_PROVIDER", l1_url.to_string())
            .env("ESPRESSO_BUILDER_PORT", builder_port.to_string())
            .env("ESPRESSO_SEQUENCER_API_PORT", api_port.to_string())
            .env("ESPRESSO_SEQUENCER_ETH_MNEMONIC", TEST_MNEMONIC)
            .env("ESPRESSO_DEPLOYER_ACCOUNT_INDEX", "0")
            .env("ESPRESSO_DEV_NODE_PORT", dev_node_port.to_string())
            .env(
                "ESPRESSO_DEPLOYER_ALT_CHAIN_PROVIDERS",
                alt_chains_env_value,
            )
            .env(
                "ESPRESSO_SEQUENCER_STORAGE_PATH",
                tmp_dir.path().as_os_str(),
            )
            .env("ESPRESSO_SEQUENCER_DATABASE_MAX_CONNECTIONS", "25")
            .spawn()
            .unwrap();

        let process = BackgroundProcess(process);

        let api_client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{api_port}").parse().unwrap());
        api_client.connect(None).await;

        tracing::info!("waiting for blocks");
        let _ = api_client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(5)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        let dev_node_client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{dev_node_port}").parse().unwrap());
        dev_node_client.connect(None).await;

        // Check the dev node api
        {
            tracing::info!("checking the dev node api");
            let dev_info = dev_node_client
                .get::<DevInfo>("api/dev-info")
                .send()
                .await
                .unwrap();

            let light_client_address = dev_info.l1_light_client_address;

            let signer = init_signer(&l1_url, TEST_MNEMONIC, 0).await.unwrap();
            let light_client = LightClient::new(light_client_address, Arc::new(signer.clone()));

            while light_client
                .get_hot_shot_commitment(U256::from(1))
                .call()
                .await
                .is_err()
            {
                tracing::info!("waiting for commitment");
                sleep(Duration::from_secs(3)).await;
            }

            for AltChainInfo {
                provider_url,
                light_client_address,
                chain_id,
                ..
            } in dev_info.alt_chains
            {
                tracing::info!("checking hotshot commitment for {chain_id}");

                let signer = init_signer(&provider_url, TEST_MNEMONIC, 0).await.unwrap();
                let light_client = LightClient::new(light_client_address, Arc::new(signer.clone()));

                while light_client
                    .get_hot_shot_commitment(U256::from(1))
                    .call()
                    .await
                    .is_err()
                {
                    tracing::info!("waiting for commitment");
                    sleep(Duration::from_secs(3)).await;
                }

                let height = signer.get_block_number().await.unwrap().as_u64();
                dev_node_client
                    .post::<()>("api/set-hotshot-down")
                    .body_json(&SetHotshotDownReqBody {
                        chain_id: Some(chain_id),
                        height: height - 1,
                    })
                    .unwrap()
                    .send()
                    .await
                    .unwrap();

                while !light_client
                    .lag_over_escape_hatch_threshold(U256::from(height), U256::from(0))
                    .call()
                    .await
                    .unwrap_or(false)
                {
                    tracing::info!("waiting for setting hotshot down");
                    sleep(Duration::from_secs(3)).await;
                }

                dev_node_client
                    .post::<()>("api/set-hotshot-up")
                    .body_json(&SetHotshotUpReqBody { chain_id })
                    .unwrap()
                    .send()
                    .await
                    .unwrap();

                while light_client
                    .lag_over_escape_hatch_threshold(U256::from(height), U256::from(0))
                    .call()
                    .await
                    .unwrap_or(true)
                {
                    tracing::info!("waiting for setting hotshot up");
                    sleep(Duration::from_secs(3)).await;
                }
            }
        }

        drop(process);
        drop(alt_providers);
    }
}
