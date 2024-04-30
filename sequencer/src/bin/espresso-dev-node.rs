use std::{io, time::Duration};

use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
use async_std::task::spawn;
use clap::Parser;
use es_version::SEQUENCER_VERSION;
use ethers::types::Address;
use futures::FutureExt;
use sequencer::{
    api::options,
    hotshot_commitment::{run_hotshot_commitment_task, CommitmentTaskOptions},
    persistence,
    test_helpers::TestNetwork,
    testing::TestConfig,
};
use sequencer_utils::{
    deployer::{deploy, Contract, Contracts},
    AnvilOptions,
};
use tide_disco::{error::ServerError, Api};
use url::Url;
use vbs::version::StaticVersionType;

#[derive(Clone, Debug, Parser)]
struct Args {
    /// A JSON-RPC endpoint for the L1 to deploy to. If this is not provided, an Avil node will be
    /// launched automatically.
    #[clap(short, long, env = "ESPRESSO_SEQUENCER_L1_PROVIDER")]
    rpc_url: Option<Url>,
    /// Mnemonic for an L1 wallet.
    ///
    /// This wallet is used to deploy the contracts, so the account indicated by ACCOUNT_INDEX must
    /// be funded with with ETH.
    #[clap(
        long,
        name = "MNEMONIC",
        env = "ESPRESSO_SEQUENCER_ETH_MNEMONIC",
        default_value = "test test test test test test test test test test test junk"
    )]
    mnemonic: String,
    /// Account index in the L1 wallet generated by MNEMONIC to use when deploying the contracts.
    #[clap(
        long,
        name = "ACCOUNT_INDEX",
        env = "ESPRESSO_DEPLOYER_ACCOUNT_INDEX",
        default_value = "0"
    )]
    account_index: u32,
    /// Port that the HTTP API will use.
    #[clap(long, env = "ESPRESSO_SEQUENCER_API_PORT", default_value = "8770")]
    sequencer_api_port: u16,
    /// Port to run the builder server on.
    #[clap(
        short,
        long,
        env = "ESPRESSO_BUILDER_SERVER_PORT",
        default_value = "8771"
    )]
    builder_port: u16,
    /// If provided, the service will run a basic HTTP server on the given port.
    ///
    /// The server provides healthcheck and version endpoints.
    #[clap(
        short,
        long,
        env = "ESPRESSO_COMMITMENT_TASK_PORT",
        default_value = "8772"
    )]
    commitment_task_port: u16,

    #[clap(flatten)]
    sql: persistence::sql::Options,
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();
    setup_backtrace();

    let opt = Args::parse();
    let options = options::Options::from(options::Http {
        port: opt.sequencer_api_port,
    })
    .status(Default::default())
    .state(Default::default())
    .submit(Default::default())
    .query_sql(Default::default(), opt.sql);

    let (url, _anvil) = if let Some(url) = opt.rpc_url {
        (url, None)
    } else {
        tracing::warn!("L1 url is not provided. running an anvil node");
        let instance = AnvilOptions::default().spawn().await;
        let url = instance.url();
        tracing::info!("l1 url: {}", url);
        (url, Some(instance))
    };

    let network = TestNetwork::new(
        options,
        [persistence::no_storage::NoStorage; TestConfig::NUM_NODES],
        url.clone(),
    )
    .await;

    let contracts = Contracts::new();

    tracing::info!("deploying the contracts");
    let contracts = deploy(
        url.clone(),
        opt.mnemonic.clone(),
        opt.account_index,
        true,
        network.light_client_genesis(),
        contracts,
    )
    .await?;

    let hotshot_address = contracts
        .get_contract_address(Contract::HotShot)
        .expect("Cannot get the hotshot contract address");
    tracing::info!("hotshot address: {}", hotshot_address);

    tracing::info!("starting the commitment server");
    start_commitment_server(opt.commitment_task_port, hotshot_address, SEQUENCER_VERSION).unwrap();

    tracing::info!("starting the builder server");
    let builder_address = "0xb0cfa4e5893107e2995974ef032957752bb526e9"
        .parse()
        .unwrap();
    start_builder_server(opt.builder_port, builder_address, SEQUENCER_VERSION).unwrap();

    let sequencer_url =
        Url::parse(format!("http://localhost:{}", opt.sequencer_api_port).as_str()).unwrap();
    let commitment_task_options = CommitmentTaskOptions {
        l1_provider: url,
        l1_chain_id: None,
        hotshot_address,
        sequencer_mnemonic: opt.mnemonic,
        sequencer_account_index: opt.account_index,
        query_service_url: Some(sequencer_url),
        request_timeout: Duration::from_secs(5),
        delay: None,
    };

    tracing::info!("starting hotshot commitment task");
    run_hotshot_commitment_task::<es_version::SequencerVersion>(&commitment_task_options).await;

    Ok(())
}

// In the test node binary, for now we don't need to run the builder. We just hardcode the builder address
// and expose the needed endpoint.
fn start_builder_server<Ver: StaticVersionType + 'static>(
    port: u16,
    builder_address: Address,
    bind_version: Ver,
) -> io::Result<()> {
    let mut app = tide_disco::App::<(), ServerError>::with_state(());
    let toml_str = r#"
[route.builder_address]
PATH = ["builderaddress"]
DOC = """
Get the builder address.
"""
    "#;
    let toml = toml::from_str::<toml::value::Value>(toml_str)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    let mut api = Api::<(), ServerError, Ver>::new(toml)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    api.get("builder_address", move |_, _| {
        async move { Ok(builder_address) }.boxed()
    })
    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    app.register_module("block_info", api)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    spawn(app.serve(format!("0.0.0.0:{port}"), bind_version));
    Ok(())
}

// Copied from `commitment_task::start_http_server`.
// TODO: Remove these redundant code
fn start_commitment_server<Ver: StaticVersionType + 'static>(
    port: u16,
    hotshot_address: Address,
    bind_version: Ver,
) -> io::Result<()> {
    let mut app = tide_disco::App::<(), ServerError>::with_state(());
    let toml = toml::from_str::<toml::value::Value>(include_str!("../../api/commitment_task.toml"))
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    let mut api = Api::<(), ServerError, Ver>::new(toml)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    api.get("gethotshotcontract", move |_, _| {
        async move { Ok(hotshot_address) }.boxed()
    })
    .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    app.register_module("api", api)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;

    spawn(app.serve(format!("0.0.0.0:{port}"), bind_version));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{process::Stdio, time::Duration};

    use async_compatibility_layer::art::async_sleep;
    use async_std::process::Command;
    use escargot::CargoBuild;
    use portpicker::pick_unused_port;
    use reqwest::StatusCode;
    use sequencer::{Header, Transaction};
    use sequencer_utils::AnvilOptions;

    // If this test failed and you are doing changes on the following stuff, please
    // sync your changes to [`espresso-sequencer-go`](https://github.com/EspressoSystems/espresso-sequencer-go)
    // and open a PR.
    // - APIs update
    // - Types (like `Header`) update
    #[async_std::test]
    async fn dev_node_test() {
        let anvil = AnvilOptions::default().spawn().await;
        let builder_port = pick_unused_port().unwrap();
        let commitment_task_port = pick_unused_port().unwrap();
        let api_port = pick_unused_port().unwrap();
        let postgres_port = pick_unused_port().unwrap();

        let mut db = Command::new("docker")
            .arg("compose")
            .arg("up")
            .arg("-d")
            .arg("sequencer-db")
            .env("ESPRESSO_SEQUENCER_DB_PORT", postgres_port.to_string())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();

        let mut child_process = CargoBuild::new()
            .bin("espresso-dev-node")
            .features("testing")
            .current_target()
            .run()
            .unwrap()
            .command()
            .env("ESPRESSO_SEQUENCER_L1_PROVIDER", anvil.url().to_string())
            .env("ESPRESSO_BUILDER_SERVER_PORT", builder_port.to_string())
            .env(
                "ESPRESSO_COMMITMENT_TASK_PORT",
                commitment_task_port.to_string(),
            )
            .env("ESPRESSO_SEQUENCER_API_PORT", api_port.to_string())
            .env("ESPRESSO_SEQUENCER_POSTGRES_HOST", "localhost")
            .env(
                "ESPRESSO_SEQUENCER_POSTGRES_PORT",
                postgres_port.to_string(),
            )
            .env("ESPRESSO_SEQUENCER_POSTGRES_USER", "root")
            .env("ESPRESSO_SEQUENCER_POSTGRES_PASSWORD", "password")
            .stdout(Stdio::null())
            .spawn()
            .unwrap();

        let builder_url = format!(
            "http://localhost:{}/block_info/builderaddress",
            builder_port
        );
        println!("builder url: {}", builder_url);

        let commitment_task_url = format!(
            "http://localhost:{}/api/hotshot_contract",
            commitment_task_port
        );
        println!("commitment task url: {}", commitment_task_url);

        let sequencer_get_header_url =
            format!("http://localhost:{}/availability/header/3", api_port);
        println!("sequencer url: {}", sequencer_get_header_url);

        // Waiting for the test node running completely
        async_sleep(Duration::from_secs(50)).await;

        let client = reqwest::Client::new();

        let builder_address = client
            .get(builder_url)
            .send()
            .await
            .unwrap()
            .json::<String>()
            .await
            .unwrap();

        let header = client
            .get(sequencer_get_header_url)
            .send()
            .await
            .unwrap()
            .json::<Header>()
            .await
            .unwrap();

        assert_eq!(
            format!("0x{:x}", header.fee_info.account().address()),
            builder_address
        );

        let hotshot_contract = client
            .get(commitment_task_url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(!hotshot_contract.is_empty());

        let sequencer_submit_url = format!("http://localhost:{}/submit", api_port);
        let tx = Transaction::new(100.into(), vec![1, 2, 3]);

        let resp = client
            .post(sequencer_submit_url)
            .body(serde_json::to_string(&tx).unwrap())
            .send()
            .await
            .unwrap()
            .status();
        assert_eq!(resp, StatusCode::OK);

        // These endpoints are currently used in `espresso-sequencer-go`. These checks
        // serve as reminders of syncing the API updates to go client repo when they change.
        {
            api_get_test(
                &client,
                format!("http://localhost:{}/status/block-height", api_port),
            )
            .await;
            api_get_test(
                &client,
                format!("http://localhost:{}/availability/header/1/3", api_port),
            )
            .await;
            api_get_test(
                &client,
                format!(
                    "http://localhost:{}/availability/block/3/namespace/100",
                    api_port
                ),
            )
            .await;
            api_get_test(
                &client,
                format!("http://localhost:{}/block-state/2/3", api_port),
            )
            .await
        }

        child_process.kill().unwrap();
        db.kill().unwrap();

        let _ = Command::new("docker")
            .arg("compose")
            .arg("down")
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
    }

    async fn api_get_test(client: &reqwest::Client, url: String) {
        let resp_status = client.get(url).send().await.unwrap().status();
        assert_eq!(resp_status, StatusCode::OK);
    }
}
