use std::{
    fmt,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command},
    str::FromStr,
    time::Duration,
};

use alloy::{
    primitives::{Address, U256},
    providers::ProviderBuilder,
};
use anyhow::{anyhow, Context, Result};
use client::SequencerClient;
use espresso_contract_deployer::build_signer;
use espresso_types::FeeAmount;
use futures::{
    future::{join_all, BoxFuture},
    FutureExt,
};
use hotshot_contract_adapter::sol_types::{EspTokenV2, LightClientV3, RewardClaim, StakeTableV2};
use sequencer::Genesis;
use surf_disco::Url;
use tokio::time::{sleep, timeout};

const RECIPIENT_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
const VALIDATOR0_ACCOUNT_INDEX: u32 = 20;

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
pub struct TestRuntime {
    pub config: TestConfig,
    pub builder_address: Address,
    pub reward_claim_address: Option<Address>,
    pub initial_height: u64,
    pub initial_txns: u64,
}

#[derive(Clone, Debug)]
pub struct TestConfig {
    pub load_generator_url: String,
    pub l1_endpoint: Url,
    pub sequencer_api_url: Url,
    pub prover_url: String,
    pub sequencer_clients: Vec<SequencerClient>,
    pub light_client_address: Address,
    pub stake_table_address: Address,
    pub recipient_address: Address,
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
    /// Block height at which to check rewards have been claimed (if Some)
    pub reward_claim_deadline_block_height: Option<u64>,
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
            reward_claim_deadline_block_height: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct TestState {
    pub block_height: Option<u64>,
    pub txn_count: u64,
    pub builder_balance: FeeAmount,
    pub recipient_balance: FeeAmount,
    pub light_client_finalized_block_height: u64,
    pub rewards_claimed: U256,
}

impl fmt::Display for TestState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let output = format!(
            "
        block_height: {}
        transactions: {}
        builder_balance: {}
        recipient_balance: {}
        light_client_finalized_block_height: {}
        rewards_claimed: {}
",
            self.block_height.unwrap(),
            self.txn_count,
            self.builder_balance,
            self.recipient_balance,
            self.light_client_finalized_block_height,
            self.rewards_claimed
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
    pub fn from_env(requirements: TestRequirements) -> Result<Self> {
        let load_generator_url =
            url_from_port(dotenvy::var("ESPRESSO_SUBMIT_TRANSACTIONS_PRIVATE_PORT")?)?;

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

        let light_client_address =
            dotenvy::var("ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS")?.parse::<Address>()?;
        let stake_table_address: Address =
            dotenvy::var("ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS")?.parse()?;

        Ok(Self {
            load_generator_url,
            l1_endpoint: Url::parse(&l1_provider_url)?,
            sequencer_api_url: Url::from_str(&sequencer_api_url)?,
            prover_url,
            sequencer_clients,
            light_client_address,
            stake_table_address,
            recipient_address: RECIPIENT_ADDRESS.parse::<Address>()?,
            requirements,
        })
    }

    /// Number of blocks to wait before deeming the test successful
    pub fn expected_block_height(&self) -> u64 {
        self.requirements.block_height_increment
    }

    pub fn expected_txn_count(&self) -> u64 {
        self.requirements.txn_count_increment
    }

    /// Get the validator0 address (ACCOUNT_INDEX=20)
    pub fn validator0_address() -> Address {
        let mnemonic = dotenvy::var("ESPRESSO_SEQUENCER_ETH_MNEMONIC")
            .expect("ESPRESSO_SEQUENCER_ETH_MNEMONIC not set");
        let signer = build_signer(&mnemonic, VALIDATOR0_ACCOUNT_INDEX);
        signer.address()
    }
}

impl TestRuntime {
    pub async fn initialize(config: TestConfig, timeout_duration: Duration) -> Result<Self> {
        let builder_url = {
            let url = url_from_port(dotenvy::var("ESPRESSO_BUILDER_SERVER_PORT")?)?;
            let url = Url::from_str(&url)?;
            wait_for_service(url.clone(), 1000, 200).await?;
            url.join("block_info/builderaddress")?
        };

        let builder_address = get_builder_address(builder_url).await;

        let client = SequencerClient::new(config.sequencer_api_url.clone());

        let (initial_height, initial_txns) = timeout(timeout_duration, async {
            loop {
                match (
                    client.get_height().await,
                    client.get_transaction_count().await,
                ) {
                    (Ok(height), Ok(txns)) => return (height, txns),
                    _ => {
                        sleep(Duration::from_millis(500)).await;
                    },
                }
            }
        })
        .await
        .context("timed out waiting for sequencer to be ready")?;

        let provider = ProviderBuilder::new().connect_http(config.l1_endpoint.clone());
        let reward_claim_address = async {
            let stake_table = StakeTableV2::new(config.stake_table_address, &provider);
            let token_address = stake_table.token().call().await.ok()?;

            let esp_token = EspTokenV2::new(token_address, &provider);
            let reward_claim_addr = esp_token.rewardClaim().call().await.ok()?;

            if reward_claim_addr == Address::ZERO {
                None
            } else {
                Some(reward_claim_addr)
            }
        }
        .await;

        let mut futures: Vec<BoxFuture<Result<String>>> =
            vec![wait_for_service(Url::from_str(&config.load_generator_url)?, 1000, 600).boxed()];

        for client in &config.sequencer_clients {
            futures.push(wait_for_sequencer_client(client.clone(), 500, 30).boxed());
        }

        join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            config,
            builder_address,
            reward_claim_address,
            initial_height,
            initial_txns,
        })
    }

    pub async fn from_requirements(requirements: TestRequirements) -> Result<Self> {
        let config = TestConfig::from_env(requirements)?;
        Self::initialize(config, Duration::from_secs(30)).await
    }

    /// Refresh the reward claim address from the contract
    /// Call this after the reward claim contract has been deployed
    pub async fn refresh_reward_claim_address(&mut self) -> Result<()> {
        println!("Refreshing reward claim address");
        let provider = ProviderBuilder::new().connect_http(self.config.l1_endpoint.clone());
        let stake_table = StakeTableV2::new(self.config.stake_table_address, &provider);
        println!("stake table address: {}", self.config.stake_table_address);
        let token_address = stake_table.token().call().await?;
        println!("token address: {token_address}");

        let esp_token = EspTokenV2::new(token_address, &provider);
        let reward_claim_addr = esp_token.rewardClaim().call().await?;
        println!("reward claim address: {reward_claim_addr}");

        self.reward_claim_address = if reward_claim_addr == Address::ZERO {
            None
        } else {
            Some(reward_claim_addr)
        };

        Ok(())
    }

    pub fn expected_block_height(&self) -> u64 {
        self.initial_height + self.config.expected_block_height()
    }

    pub fn expected_txn_count(&self) -> u64 {
        self.initial_txns + self.config.expected_txn_count()
    }

    /// Get the finalized block height from the light client
    pub async fn light_client_finalized_block_height(&self) -> u64 {
        let provider = ProviderBuilder::new().connect_http(self.config.l1_endpoint.clone());
        let light_client = LightClientV3::new(self.config.light_client_address, &provider);
        let finalized_state = light_client.finalizedState().call().await.unwrap();
        finalized_state.blockHeight
    }

    /// Return current state  of the test
    pub async fn test_state(&self) -> TestState {
        let client = SequencerClient::new(self.config.sequencer_api_url.clone());
        let block_height = client.get_height().await.ok();
        let txn_count = client.get_transaction_count().await.unwrap();

        let builder_balance = client
            .get_espresso_balance(self.builder_address, block_height)
            .await
            .unwrap();
        let recipient_balance = client
            .get_espresso_balance(self.config.recipient_address, block_height)
            .await
            .unwrap();

        let light_client_finalized_block_height = self.light_client_finalized_block_height().await;

        let rewards_claimed = self.claimed_rewards().await.unwrap_or(U256::ZERO);

        TestState {
            block_height,
            txn_count,
            builder_balance,
            recipient_balance,
            light_client_finalized_block_height,
            rewards_claimed,
        }
    }

    /// Check claimed rewards for validator0
    /// Returns an error if reward claim contract isn't available yet
    pub async fn claimed_rewards(&self) -> Result<U256> {
        let reward_claim_address = self
            .reward_claim_address
            .context("Reward claim address not available")?;

        let provider = ProviderBuilder::new().connect_http(self.config.l1_endpoint.clone());
        let reward_claim = RewardClaim::new(reward_claim_address, &provider);

        let validator_address = TestConfig::validator0_address();
        let claimed = reward_claim
            .claimedRewards(validator_address)
            .call()
            .await?;

        Ok(claimed)
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

async fn wait_for_sequencer_client(
    client: SequencerClient,
    interval: u64,
    timeout_duration: u64,
) -> Result<String> {
    timeout(Duration::from_secs(timeout_duration), async {
        loop {
            if client.get_height().await.is_ok() {
                return Ok("sequencer ready".to_string());
            }
            sleep(Duration::from_millis(interval)).await;
        }
    })
    .await
    .map_err(|e| anyhow!("Wait for sequencer client, timeout: {}", e))?
}

pub struct NativeDemo {
    child: Child,
}

impl Drop for NativeDemo {
    fn drop(&mut self) {
        println!("Terminating demo-native process");
        // Send SIGTERM to allow the EXIT trap in scripts/demo-native to run cleanup.
        // child.kill() sends SIGKILL which cannot be caught, so we use the kill command.
        let pid = self.child.id();
        let _ = Command::new("kill")
            .arg("-TERM")
            .arg(pid.to_string())
            .status();

        // Wait up to 20 seconds for graceful shutdown
        for i in 0..20 {
            match self.child.try_wait() {
                Ok(Some(_)) => {
                    println!("demo-native process exited after {} seconds", i);
                    return;
                },
                Ok(None) => {
                    println!("waiting for demo-native to exit");
                    std::thread::sleep(Duration::from_secs(1));
                },
                Err(e) => {
                    println!("Error checking process status: {}", e);
                    break;
                },
            }
        }

        // Force kill if still running after 10 seconds
        println!("Force killing demo-native process after timeout");
        let _ = self.child.kill();
        let _ = self.child.wait();
        println!("demo-native process terminated");
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

        // Clean up any leftover processes from previous test runs
        println!("Running cleanup before starting demo...");
        let cleanup_script = workspace_dir.join("scripts/cleanup-process-compose");
        let cleanup_status = Command::new("bash")
            .arg(&cleanup_script)
            .current_dir(workspace_dir)
            .status()
            .context("failed to execute cleanup script")?;

        if !cleanup_status.success() {
            return Err(anyhow!(
                "Failed to clean up before starting demo.\nThis usually means there are leftover \
                 processes from a previous run.\nTry running 'scripts/cleanup-process-compose' \
                 manually."
            ));
        }
        println!("Cleanup complete, starting demo...");

        let mut cmd = Command::new("bash");

        // Set default ESPRESSO_STATE_PROVER_UPDATE_INTERVAL for tests if not already set
        if std::env::var("ESPRESSO_STATE_PROVER_UPDATE_INTERVAL").is_err() {
            cmd.env("ESPRESSO_STATE_PROVER_UPDATE_INTERVAL", "20s");
        }

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

        Ok(Self { child })
    }
}
