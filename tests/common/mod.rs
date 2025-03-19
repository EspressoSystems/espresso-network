use std::{
    fmt,
    fs::File,
    io::{stderr, stdout},
    path::PathBuf,
    process::{Child, Command},
    str::FromStr,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use client::SequencerClient;
use dotenvy::var;
use espresso_types::{FeeAmount, FeeVersion, MarketplaceVersion};
use ethers::prelude::*;
use futures::{
    future::{join_all, BoxFuture},
    FutureExt,
};
use hotshot_types::{network::PeerConfigKeys, traits::signature_key::StakeTableEntryType};
use sequencer_utils::stake_table::{
    update_stake_table, PermissionedStakeTableUpdate, StakerIdentity,
};
use surf_disco::Url;
use tokio::time::{sleep, timeout};
use vbs::version::StaticVersionType;

const L1_PROVIDER_RETRY_INTERVAL: Duration = Duration::from_secs(1);
// TODO add to .env
const RECIPIENT_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
/// Duration in seconds to wait before declaring the chain deceased.
const SEQUENCER_BLOCKS_TIMEOUT: u64 = 300;

#[derive(Clone, Debug)]
pub struct TestConfig {
    pub load_generator_url: String,
    pub provider: Provider<Http>,
    pub espresso: SequencerClient,
    pub builder_address: Address,
    pub recipient_address: Address,
    pub light_client_address: Address,
    pub prover_url: String,
    pub expected_block_height: u64,
    pub timeout: f64,
    pub sequencer_version: u8,
    pub sequencer_clients: Vec<SequencerClient>,
    pub initial_height: u64,
    pub initial_txns: u64,
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

        write!(f, "{}", output)
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
    pub async fn new() -> Result<Self> {
        // We need to know sequencer version we are testing in order
        // to test against the correct services. It's tempting to try
        // and proceed against any working services, but we run the
        // risk of testing against the wrong ones. For example, legacy
        // builder may be alive during marketplace test.
        //
        // If no version is specified, we default to V2,
        // which is the initial mainnet version without any upgrades.
        let sequencer_version: u8 = dotenvy::var("INTEGRATION_TEST_SEQUENCER_VERSION")
            .map(|v| v.parse().unwrap())
            .unwrap_or(FeeVersion::version().minor as u8);

        // Varies between v0 and v3.
        let load_generator_url = if sequencer_version >= MarketplaceVersion::version().minor as u8 {
            url_from_port(dotenvy::var(
                "ESPRESSO_SUBMIT_TRANSACTIONS_PRIVATE_RESERVE_PORT",
            )?)?
        } else {
            url_from_port(dotenvy::var("ESPRESSO_SUBMIT_TRANSACTIONS_PRIVATE_PORT")?)?
        };

        // TODO test both builders (probably requires some refactoring).
        let builder_url = if sequencer_version as u16 >= MarketplaceVersion::version().minor {
            let url = url_from_port(dotenvy::var("ESPRESSO_RESERVE_BUILDER_SERVER_PORT")?)?;
            let url = Url::from_str(&url)?;
            wait_for_service(url.clone(), 1000, 200).await.unwrap();

            url.join("bundle_info/builderaddress").unwrap()
        } else {
            let url = url_from_port(dotenvy::var("ESPRESSO_BUILDER_SERVER_PORT")?)?;
            let url = Url::from_str(&url)?;
            wait_for_service(url.clone(), 1000, 200).await.unwrap();

            url.join("block_info/builderaddress").unwrap()
        };

        let builder_address = get_builder_address(builder_url).await;

        let l1_provider_url = url_from_port(dotenvy::var("ESPRESSO_SEQUENCER_L1_PORT")?)?;
        let sequencer_api_url = url_from_port(dotenvy::var("ESPRESSO_SEQUENCER1_API_PORT")?)?;
        let sequencer_clients = [
            dotenvy::var("ESPRESSO_SEQUENCER_API_PORT")?,
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

        // Number of blocks to wait before deeming the test successful
        let expected_block_height = dotenvy::var("INTEGRATION_TEST_EXPECTED_BLOCK_HEIGHT")?
            .parse::<u64>()
            .unwrap();

        // Set the time out. Give a little more leeway when we have a
        // large `expected_block_height`.
        let timeout = expected_block_height as f64
            + SEQUENCER_BLOCKS_TIMEOUT as f64
            + (expected_block_height as f64).sqrt();

        println!("Waiting on Builder Address");

        let client = SequencerClient::new(Url::from_str(&sequencer_api_url).unwrap());
        let initial_height = client.get_height().await.unwrap();
        let initial_txns = client.get_transaction_count().await.unwrap();

        Ok(Self {
            load_generator_url,
            provider: Provider::<Http>::try_from(l1_provider_url)?
                .interval(L1_PROVIDER_RETRY_INTERVAL),
            espresso: client,
            light_client_address: light_client_proxy_address.parse::<Address>().unwrap(),
            builder_address,
            recipient_address: RECIPIENT_ADDRESS.parse::<Address>().unwrap(),
            prover_url,
            expected_block_height,
            timeout,
            sequencer_version,
            sequencer_clients,
            initial_height,
            initial_txns,
        })
    }

    /// Number of blocks to wait before deeming the test successful
    pub fn expected_block_height(&self) -> u64 {
        self.expected_block_height
    }
    /// Get the latest block where we see a light client update
    pub async fn latest_light_client_update(&self) -> u64 {
        let filter = Filter::new()
            .from_block(BlockNumber::Earliest)
            .address(self.light_client_address);
        // block number for latest light client update
        let latest_light_client_update = self
            .provider
            .get_logs(&filter)
            .await
            .unwrap()
            .last()
            .unwrap()
            .block_number;

        latest_light_client_update.unwrap().as_u64()
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

/*
        EPOCH V3
*/

pub async fn test_stake_table_update(clients: Vec<SequencerClient>) -> Result<()> {
    let l1_port = var("ESPRESSO_SEQUENCER_L1_PORT")?;
    let account_index = var("ESPRESSO_DEPLOYER_ACCOUNT_INDEX")?;
    let contract_address = var("ESPRESSO_SEQUENCER_PERMISSIONED_STAKE_TABLE_ADDRESS")?;
    let client = clients[0].clone();

    let assert_change =
        |u: PermissionedStakeTableUpdate| -> BoxFuture<'static, anyhow::Result<()>> {
            let client = client.clone();
            let l1_port = l1_port.clone();
            let account_index = account_index.clone();
            let contract_address = contract_address.clone();
            async move {
                let epoch_before_update = client.current_epoch().await?.context("curr epoch")?;
                tracing::warn!("current_epoch={epoch_before_update:?}");

                let current_stake_table = client.stake_table(epoch_before_update).await?;

                let removed = u.stakers_to_remove.len();
                let added = u.new_stakers.len();

                update_stake_table(
                    format!("http://localhost:{l1_port}").parse()?,
                    Duration::from_secs(7),
                    "test test test test test test test test test test test junk".to_string(),
                    account_index.parse()?,
                    contract_address.parse()?,
                    u.clone(),
                )
                .await?;

                loop {
                    sleep(Duration::from_secs(10)).await;
                    let epoch = client.current_epoch().await?.context("curr epoch")?;
                    tracing::info!("current_epoch={epoch:?}");
                    if epoch > epoch_before_update + 2 {
                        let stake_table = client.stake_table(epoch).await?;
                        tracing::info!("stake_table={stake_table:?}");
                        assert_eq!(
                            stake_table.len(),
                            current_stake_table.len() + added - removed
                        );

                        for added in &u.new_stakers {
                            assert!(
                                stake_table
                                    .iter()
                                    .any(|st| st.stake_key == added.stake_table_key),
                                "staker {} not found",
                                added.stake_table_key
                            );
                        }

                        for removed in &u.stakers_to_remove {
                            assert!(
                                stake_table
                                    .iter()
                                    .all(|st| st.stake_key != removed.stake_table_key),
                                "staker {} found",
                                removed.stake_table_key
                            );
                        }

                        break;
                    }
                }

                anyhow::Result::<_>::Ok(())
            }
            .boxed()
        };

    let config = client.config().await?.hotshot_config();

    // currently stake table update does not support DA node member changes
    let stake_table = config.known_nodes_with_stake();
    let da_members = config.known_da_nodes();

    // filtering out DA nodes
    let non_da_stakers: Vec<_> = stake_table
        .into_iter()
        .filter(|x| !da_members.contains(x))
        .collect();

    let node = non_da_stakers.first().context("no non da staker found")?;
    let one_removed = PermissionedStakeTableUpdate::new(
        vec![],
        vec![StakerIdentity {
            stake_table_key: node.stake_table_entry.stake_key,
        }],
    );

    // remove one node
    assert_change(one_removed)
        .await
        .expect("failed to remove one node");

    // add back the removed node
    let added = PermissionedStakeTableUpdate::new(
        vec![PeerConfigKeys {
            stake_table_key: *node.stake_table_entry.key(),
            state_ver_key: node.state_ver_key.clone(),
            stake: node.stake_table_entry.stake().as_u64(),
            da: false,
        }],
        vec![],
    );

    assert_change(added).await.expect("failed to add a node");

    Ok(())
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
        println!("process-compose down exited with: {}", res);
    }
}

impl NativeDemo {
    pub(crate) fn run(process_compose_extra_args: Option<String>) -> anyhow::Result<Self> {
        // Because we use nextest with the archive feature on CI we need to use the **runtime**
        // value of CARGO_MANIFEST_DIR.
        let crate_dir = PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("CARGO_MANIFEST_DIR is set")
                .clone(),
        );
        let workspace_dir = crate_dir.parent().expect("crate_dir has a parent");

        let mut cmd = Command::new("bash");
        cmd.arg("scripts/demo-native")
            .current_dir(workspace_dir)
            .arg("--tui=false");

        if let Some(args) = process_compose_extra_args {
            cmd.args(args.split(' '));
        }

        // Save output to file if PC_LOGS if that's set.
        let log_path = std::env::var("PC_LOGS").unwrap_or_else(|_| {
            tempfile::NamedTempFile::new()
                .expect("tempfile creation succeeds")
                .into_temp_path()
                .to_string_lossy()
                .to_string()
        });

        println!("Writing native demo logs to file: {}", log_path);
        let outputs = File::create(log_path).context("unable to create log file")?;
        cmd.stdout(outputs);

        println!("Spawning: {:?}", cmd);
        let mut child = cmd.spawn().context("failed to spawn command")?;

        // Wait for three seconds and check if process has already exited so we don't waste time
        // waiting for results later.
        std::thread::sleep(Duration::from_secs(3));
        if let Some(exit_code) = child.try_wait()? {
            return Err(anyhow!("process-compose exited early with: {}", exit_code));
        }

        println!("process-compose started ...");

        Ok(Self { _child: child })
    }
}
