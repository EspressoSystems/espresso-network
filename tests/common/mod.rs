use std::{
    fmt,
    io::{stderr, stdout, Write},
    path::{Path, PathBuf},
    process::{Child, Command},
    str::FromStr,
    time::Duration,
};

use alloy::{
    eips::BlockNumberOrTag,
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
};
use anyhow::{anyhow, Context, Result};
use client::SequencerClient;
use espresso_types::FeeAmount;
use futures::future::join_all;
use sequencer::Genesis;
use surf_disco::Url;
use tokio::time::{sleep, timeout};

// TODO add to .env
const RECIPIENT_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

pub fn load_genesis_file(path: impl AsRef<Path>) -> Result<Genesis> {
    // Because we use nextest with the archive feature on CI we need to use the **runtime**
    // value of CARGO_MANIFEST_DIR.
    let crate_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("CARGO_MANIFEST_DIR is set")
            .clone(),
    );
    Genesis::from_file(crate_dir.join("..").join(path))
}

#[derive(Clone, Debug)]
pub struct TestConfig {
    pub load_generator_url: String,
    pub l1_endpoint: Url,
    pub espresso: SequencerClient,
    pub builder_address: Address,
    pub recipient_address: Address,
    pub light_client_address: Address,
    pub prover_url: String,
    pub sequencer_clients: Vec<SequencerClient>,
    pub initial_height: u64,
    pub initial_txns: u64,
    pub requirements: TestRequirements,
}

#[derive(Clone, Debug)]
pub struct TestRequirements {
    pub block_height_increment: u64,
    pub txn_count_increment: u64,
    /// Fail the test after this interval if requirement not met yet.
    pub global_timeout: Duration,
    /// Panic if no block seen for this interval, we will panic fail relatively quickly.
    pub block_timeout: Duration,
    pub max_consecutive_blocks_without_tx: u64,
}

impl Default for TestRequirements {
    fn default() -> Self {
        Self {
            block_height_increment: 10,
            txn_count_increment: 10,
            global_timeout: Duration::from_secs(60),
            // TODO: on the CI we are quite resource constraint and for longer runs we do get a few
            // timeouts which lead to occasional drop in block times.
            block_timeout: Duration::from_secs(45),
            max_consecutive_blocks_without_tx: 10,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct TestState {
    pub block_height: Option<u64>,
    pub txn_count: u64,
    pub builder_balance: FeeAmount,
    pub recipient_balance: FeeAmount,
    pub light_client_update: u64,
}

impl fmt::Display for TestState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = format!(
            "
        block_height: {}
        transactions: {}
        builder_balance: {}
        recipient_balance: {}
        light_client_updated: {}
",
            self.block_height.unwrap(),
            self.txn_count,
            self.builder_balance,
            self.recipient_balance,
            self.light_client_update
        );

        write!(f, "{output}")
    }
}

fn url_from_port(port: String) -> Result<String> {
    Ok(format!(
        "{}://{}:{}",
        dotenvy::var("INTEGRATION_TEST_PROTO")?,
        dotenvy::var("INTEGRATION_TEST_HOST")?,
        port
    ))
}

impl TestConfig {
    pub async fn new(requirements: TestRequirements) -> Result<Self> {
        // Varies between v0 and v3.
        let load_generator_url =
            url_from_port(dotenvy::var("ESPRESSO_SUBMIT_TRANSACTIONS_PRIVATE_PORT")?)?;

        let builder_url = {
            let url = url_from_port(dotenvy::var("ESPRESSO_BUILDER_SERVER_PORT")?)?;
            let url = Url::from_str(&url)?;
            wait_for_service(url.clone(), 1000, 200).await.unwrap();
            url.join("block_info/builderaddress").unwrap()
        };

        let builder_address = get_builder_address(builder_url).await;

        let l1_provider_url = url_from_port(dotenvy::var("ESPRESSO_SEQUENCER_L1_PORT")?)?;
        let sequencer_api_url = url_from_port(dotenvy::var("ESPRESSO_SEQUENCER1_API_PORT")?)?;
        let sequencer_clients = [
            dotenvy::var("ESPRESSO_SEQUENCER0_API_PORT")?,
            dotenvy::var("ESPRESSO_SEQUENCER1_API_PORT")?,
        ]
        .iter()
        .map(|port| url_from_port(port.clone()).unwrap())
        .collect::<Vec<String>>()
        .iter()
        .map(|url| SequencerClient::new(Url::from_str(url).unwrap()))
        .collect::<Vec<SequencerClient>>();

        let prover_url = url_from_port(dotenvy::var("ESPRESSO_PROVER_SERVICE_PORT")?)?;

        let light_client_proxy_address =
            dotenvy::var("ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS")?;

        println!("Waiting on Builder Address");

        let client = SequencerClient::new(Url::from_str(&sequencer_api_url).unwrap());
        let initial_height = client.get_height().await.unwrap();
        let initial_txns = client.get_transaction_count().await.unwrap();

        Ok(Self {
            load_generator_url,
            l1_endpoint: Url::parse(&l1_provider_url).unwrap(),
            espresso: client,
            light_client_address: light_client_proxy_address.parse::<Address>().unwrap(),
            builder_address,
            recipient_address: RECIPIENT_ADDRESS.parse::<Address>().unwrap(),
            prover_url,
            sequencer_clients,
            initial_height,
            initial_txns,
            requirements,
        })
    }

    /// Number of blocks to wait before deeming the test successful
    pub fn expected_block_height(&self) -> u64 {
        self.initial_height + self.requirements.block_height_increment
    }

    pub fn expected_txn_count(&self) -> u64 {
        self.initial_txns + self.requirements.txn_count_increment
    }

    /// Get the latest block where we see a light client update
    pub async fn latest_light_client_update(&self) -> u64 {
        let provider = ProviderBuilder::new().connect_http(self.l1_endpoint.clone());
        let filter = Filter::new()
            .from_block(BlockNumberOrTag::Earliest)
            .address(self.light_client_address);
        // block number for latest light client update
        provider
            .get_logs(&filter)
            .await
            .unwrap()
            .last()
            .unwrap()
            .block_number
            .unwrap()
    }

    /// Return current state  of the test
    pub async fn test_state(&self) -> TestState {
        let block_height = self.espresso.get_height().await.ok();
        let txn_count = self.espresso.get_transaction_count().await.unwrap();

        let builder_balance = self
            .espresso
            .get_espresso_balance(self.builder_address, block_height)
            .await
            .unwrap();
        let recipient_balance = self
            .espresso
            .get_espresso_balance(self.recipient_address, block_height)
            .await
            .unwrap();

        let light_client_update = self.latest_light_client_update().await;

        TestState {
            block_height,
            txn_count,
            builder_balance,
            recipient_balance,
            light_client_update,
        }
    }

    /// Check if services are healthy
    pub async fn readiness(&self) -> Result<Vec<String>> {
        join_all(vec![
            wait_for_service(Url::from_str(&self.load_generator_url).unwrap(), 1000, 600),
            wait_for_service(Url::from_str(&self.prover_url).unwrap(), 1000, 300),
        ])
        .await
        .into_iter()
        .collect::<Result<Vec<String>>>()
    }
}

/// Get Address from builder
pub async fn get_builder_address(url: Url) -> Address {
    for _ in 0..5 {
        // Try to get builder address somehow
        if let Ok(body) = reqwest::get(url.clone()).await {
            return body.json::<Address>().await.unwrap();
        } else {
            sleep(Duration::from_millis(400)).await
        }
    }
    panic!("Error: Failed to retrieve address from builder!");
}

/// [wait_for_service] will check to see if a service, identified by the given
/// Url, is available, by checking it's health check endpoint.  If the health
/// check does not any time before the timeout, then the service will return
/// an [Err] with the relevant error.
///
/// > Note: This function only waits for a single health check pass before
/// > returning an [Ok] result.
async fn wait_for_service(url: Url, interval: u64, timeout_duration: u64) -> Result<String> {
    // utilize the correct path for the health check
    let Ok(url) = url.join("/healthcheck") else {
        return Err(anyhow!("Wait for service, could not join url: {}", url));
    };

    timeout(Duration::from_secs(timeout_duration), async {
        loop {
            // Ensure that we get a response from the server
            let Ok(response) = reqwest::get(url.clone()).await else {
                sleep(Duration::from_millis(interval)).await;
                continue;
            };

            // Check the status code of the response
            if !response.status().is_success() {
                // The server did not return a success
                sleep(Duration::from_millis(interval)).await;
                continue;
            }

            return response.text().await.map_err(|e| {
                anyhow!(
                    "Wait for service, could not decode response: ({}) {}",
                    url,
                    e
                )
            });
        }
    })
    .await
    .map_err(|e| anyhow!("Wait for service, timeout: ({}) {}", url, e))?
}

pub struct NativeDemo {
    _child: Child,
}

impl Drop for NativeDemo {
    fn drop(&mut self) {
        // It would be preferable to send a SIGINT or similar to the process that we started
        // originally but despite quite some effort this never worked for the process-compose
        // process started from within the scripts/demo-native script.
        //
        // Using `process-compose down` seems to pretty reliably stop the process-compose process
        // and all the services it started.
        println!("Terminating process compose");
        let res = Command::new("process-compose")
            .arg("down")
            .stdout(stdout())
            .stderr(stderr())
            .spawn()
            .expect("process-compose runs")
            .wait()
            .unwrap();
        println!("process-compose down exited with: {res}");
    }
}

impl NativeDemo {
    pub(crate) fn run(
        process_compose_extra_args: Option<String>,
        env_overrides: Option<Vec<(String, String)>>,
    ) -> anyhow::Result<Self> {
        // Because we use nextest with the archive feature on CI we need to use the **runtime**
        // value of CARGO_MANIFEST_DIR.
        let crate_dir = PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR is set")
                .clone(),
        );
        let workspace_dir = crate_dir.parent().expect("crate_dir has a parent");

        let mut cmd = Command::new("bash");
        if let Some(overrides) = env_overrides {
            for (key, value) in overrides {
                println!("applying env override: {key}={value}");
                cmd.env(key, value);
            }
        }
        cmd.arg("scripts/demo-native")
            .current_dir(workspace_dir)
            .arg("--tui=false");

        if let Some(args) = process_compose_extra_args {
            cmd.args(args.split(' '));
        }

        // Save output to file if PC_LOGS if that's set.
        let log_path = std::env::var("NATIVE_DEMO_LOGS").unwrap_or_else(|_| {
            tempfile::NamedTempFile::new()
                .expect("tempfile creation succeeds")
                .into_temp_path()
                .to_string_lossy()
                .to_string()
        });

        println!("Writing native demo logs to file: {log_path}");

        let is_ci = std::env::var("CI").unwrap_or_default() == "true";

        // Open file in append mode if CI, otherwise truncate
        let mut log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(is_ci)
            .truncate(!is_ci)
            .write(true)
            .open(&log_path)
            .context("unable to open log file")?;
        writeln!(log_file, "==== process-compose logs =====")?;
        log_file.flush()?;

        // Redirect both stdout and stderr to the same file
        cmd.stdout(
            log_file
                .try_clone()
                .context("unable to clone log file for stdout")?,
        );
        cmd.stderr(log_file);

        println!("Spawning: {cmd:?}");
        let mut child = cmd.spawn().context("failed to spawn command")?;

        // Wait for three seconds and check if process has already exited so we don't waste time
        // waiting for results later. The native demo takes quite some time to get functional so we
        // wait for a while before checking if the process has exited.
        for _ in 0..10 {
            if let Some(exit_code) = child.try_wait()? {
                return Err(anyhow!("process-compose exited early with: {}", exit_code));
            }
            println!("Waiting for process-compose to start ...");
            std::thread::sleep(Duration::from_secs(1));
        }

        println!("process-compose started ...");

        Ok(Self { _child: child })
    }
}
