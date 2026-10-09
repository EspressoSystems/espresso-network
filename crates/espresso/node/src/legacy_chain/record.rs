//! Records `data/legacy-chains`. Only legacy consensus can produce these chains, so this runs
//! once, before it is deleted:
//!
//! ```sh
//! cargo test -p espresso-node --features embedded-db --lib record_legacy_chains -- --ignored
//! ```
//!
//! Each chain takes a few minutes, longer than nextest's slow-test limit. `LEGACY_CHAINS=v4,v5`
//! records only the chains named.

use std::time::Duration;

use alloy::primitives::U256;
use espresso_contract_deployer::{Contract, upgrade_stake_table_v2, upgrade_stake_table_v3};
use espresso_types::{GenesisHeader, L1Client, NamespaceId, Transaction, ValidatedState};
use futures::future::join_all;
use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
use hotshot_types::{traits::metrics::NoMetrics, utils::epoch_from_block_number, x25519};
use staking_cli::{
    demo::DelegationConfig, fetch_commission, update_commission, update_network_config,
};
use tempfile::TempDir;
use tokio::task::JoinHandle;
use versions::{DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_REWARD_VERSION, EPOCH_VERSION, Upgrade};

use super::*;
use crate::{
    api::test_helpers::{TestNetwork, TestNetworkConfigBuilder},
    catchup::StatePeers,
    genesis::{L1Finalized, StakeTableConfig},
    testing::{TestConfig, TestConfigBuilder, wait_for_epochs},
};

const NUM_NODES: usize = 5;
const EPOCH_HEIGHT: u64 = 10;
/// Long enough for rewards, state certificates and the stake table changes to span epochs.
const EPOCHS: u64 = 8;

struct Scenario {
    name: &'static str,
    upgrade: Upgrade,
    stake_table: StakeTableContractVersion,
    /// Upgrade the stake table contract to V2 and then V3 mid-chain, changing commissions and a
    /// network config along the way, as mainnet's contract did.
    contract_history: bool,
    epoch_height: u64,
    /// How many epochs to record at least. A chain with a contract history runs on until the
    /// last change has reached the stake table.
    epochs: u64,
}

#[ignore = "records data/legacy-chains, which only legacy consensus can produce"]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn record_legacy_chains() -> anyhow::Result<()> {
    let only = std::env::var("LEGACY_CHAINS").ok();
    for scenario in [
        Scenario {
            name: "v3",
            upgrade: Upgrade::trivial(EPOCH_VERSION),
            stake_table: StakeTableContractVersion::V2,
            contract_history: false,
            epoch_height: EPOCH_HEIGHT,
            epochs: EPOCHS,
        },
        Scenario {
            name: "v4",
            upgrade: Upgrade::trivial(DRB_AND_HEADER_UPGRADE_VERSION),
            stake_table: StakeTableContractVersion::V1,
            contract_history: true,
            epoch_height: EPOCH_HEIGHT,
            epochs: EPOCHS,
        },
        Scenario {
            name: "v5",
            upgrade: Upgrade::trivial(EPOCH_REWARD_VERSION),
            stake_table: StakeTableContractVersion::V2,
            contract_history: false,
            epoch_height: EPOCH_HEIGHT,
            epochs: EPOCHS,
        },
    ] {
        if only
            .as_ref()
            .is_some_and(|only| !only.split(',').any(|name| name == scenario.name))
        {
            continue;
        }
        record(scenario).await?;
    }
    Ok(())
}

async fn record(scenario: Scenario) -> anyhow::Result<()> {
    tracing::warn!(name = scenario.name, "recording legacy chain");
    let l1_dir = TempDir::new()?;
    let l1_state = l1_dir.path().join(L1_STATE);
    let anvil = Anvil::new()
        .args(["--slots-in-an-epoch", "0", "--balance", "1000000"])
        // Mine on a clock, as Ethereum does, so headers keep finalizing new L1 blocks and every
        // stake table change reaches an epoch.
        .args(["--block-time", "1"])
        .args(["--state-interval", "1"])
        .arg("--dump-state")
        .arg(&l1_state)
        .spawn();
    let network_config = TestConfigBuilder::default()
        .epoch_height(scenario.epoch_height)
        .anvil_provider(anvil)
        .build();

    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let port = reserve_tcp_port()?;
    let api_url: url::Url = format!("http://localhost:{port}").parse()?;
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(port)
                .catchup(Default::default())
                .submit(Default::default())
                .config(Default::default()),
        ))
        .network_config(network_config.clone())
        .persistences(persistence.clone())
        .catchups(std::array::from_fn(|_| {
            StatePeers::<SequencerApiVersion>::from_urls(
                vec![api_url.clone()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .pos_hook(
            DelegationConfig::MultipleDelegators,
            scenario.stake_table,
            scenario.upgrade,
        )
        .await?
        .build();
    let mut network = TestNetwork::new(config, scenario.upgrade).await;
    let client: Client<ClientErr, SequencerApiVersion> = Client::new(api_url);
    ensure!(client.connect(Some(Duration::from_secs(60))).await);

    let submitter = submit_transactions(client.clone());
    let mut events = network.server.event_stream();
    let mut network_config_update = None;
    if scenario.contract_history {
        wait_for_epochs(&mut events, scenario.epoch_height, 2).await;
        upgrade_to_v2_and_change_commissions(&mut network, &network_config).await?;
        wait_for_epochs(&mut events, scenario.epoch_height, 4).await;
        let update = upgrade_to_v3_and_change_network_config(&mut network, &network_config).await?;
        // A stake table change reaches the stake table two epochs after the epoch whose root
        // finalizes it on L1. Run on until that epoch and a couple more are recorded.
        let epoch = network
            .server
            .decided_leaf()
            .await
            .epoch(scenario.epoch_height)
            .context("the chain has epochs")?;
        wait_for_epochs(&mut events, scenario.epoch_height, epoch.u64() + 4).await;
        network_config_update = Some(update);
    }
    wait_for_epochs(&mut events, scenario.epoch_height, scenario.epochs).await;
    submitter.abort();

    let height: u64 = client.get("node/block-height").send().await?;
    let mut blocks = vec![];
    for h in 0..height {
        blocks.push(RecordedBlock {
            leaf: client.get(&format!("availability/leaf/{h}")).send().await?,
            block: client
                .get(&format!("availability/block/{h}"))
                .send()
                .await?,
            vid_common: client
                .get(&format!("availability/vid/common/{h}"))
                .send()
                .await?,
        });
    }
    // Every finished epoch has a certificate; the one in progress at the tip does not yet.
    let last_epoch = epoch_from_block_number(height - 1, scenario.epoch_height);
    let node_persistence = persistence[0].clone().create().await?;
    let mut state_certs = vec![];
    for epoch in 1..=last_epoch {
        if let Some(cert) = node_persistence.get_state_cert_by_epoch(epoch).await? {
            state_certs.push((epoch, cert));
        }
    }
    let genesis_state = network.server.node_state().genesis_state.clone();
    let chain = LegacyChain {
        name: scenario.name.into(),
        genesis: genesis(&network.cfg, &genesis_state, scenario.upgrade),
        network_config: network.server.network_config(),
        blocks,
        state_certs,
        network_config_update,
    };

    // Anvil dumps its state every second; let one more dump cover the last L1 block.
    sleep(Duration::from_secs(3)).await;
    chain.save(&l1_state)?;
    tracing::warn!(
        name = scenario.name,
        blocks = chain.blocks.len(),
        state_certs = chain.state_certs.len(),
        "recorded legacy chain"
    );
    Ok(())
}

impl LegacyChain {
    fn save(&self, l1_state: &Path) -> anyhow::Result<()> {
        let dir = chain_dir(&self.name);
        fs::create_dir_all(&dir)?;
        self.genesis.to_file(dir.join(GENESIS))?;
        fs::write(
            dir.join(NETWORK_CONFIG),
            serde_json::to_string_pretty(&self.network_config)?,
        )?;
        fs::write(dir.join(BLOCKS), serde_json::to_string(&self.blocks)?)?;
        fs::write(
            dir.join(STATE_CERTS),
            serde_json::to_string_pretty(&self.state_certs)?,
        )?;
        if let Some(update) = &self.network_config_update {
            fs::write(
                dir.join(NETWORK_CONFIG_UPDATE),
                serde_json::to_string_pretty(update)?,
            )?;
        }
        fs::copy(l1_state, dir.join(L1_STATE))?;
        Ok(())
    }
}

/// Two namespaces, so namespace proofs have a neighbour to exclude.
fn submit_transactions(client: Client<ClientErr, SequencerApiVersion>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut i = 0u64;
        loop {
            i += 1;
            let ns = NamespaceId::from(101u64 + i % 2);
            let tx = Transaction::new(ns, i.to_le_bytes().to_vec());
            if let Err(err) = client
                .post::<serde_json::Value>("submit/submit")
                .body_json(&tx)
                .unwrap()
                .send()
                .await
            {
                tracing::warn!("submitting a transaction failed: {err:#}");
            }
            sleep(Duration::from_millis(500)).await;
        }
    })
}

async fn upgrade_to_v2_and_change_commissions(
    network: &mut TestNetwork<persistence::sql::Options, NUM_NODES>,
    config: &TestConfig<NUM_NODES>,
) -> anyhow::Result<()> {
    let provider = network.cfg.anvil().unwrap();
    let deployer = network.cfg.signer().address();
    let contracts = network.contracts.as_mut().unwrap();
    upgrade_stake_table_v2(
        provider,
        L1Client::new(vec![network.cfg.l1_url()])?,
        contracts,
        deployer,
        deployer,
    )
    .await?;
    let st_addr = contracts.address(Contract::StakeTableProxy).unwrap();
    for (validator, provider) in config.validator_providers() {
        let commission = fetch_commission(provider.clone(), st_addr, validator).await?;
        update_commission(provider, st_addr, (commission.to_evm() + 100).try_into()?)
            .await?
            .get_receipt()
            .await?;
    }
    Ok(())
}

/// Registers a network config for the first validator and returns what it registered.
async fn upgrade_to_v3_and_change_network_config(
    network: &mut TestNetwork<persistence::sql::Options, NUM_NODES>,
    config: &TestConfig<NUM_NODES>,
) -> anyhow::Result<RecordedNetworkConfigUpdate> {
    let provider = network.cfg.anvil().unwrap();
    let contracts = network.contracts.as_mut().unwrap();
    upgrade_stake_table_v3(provider, contracts).await?;
    let st_addr = contracts.address(Contract::StakeTableProxy).unwrap();
    let (validator, validator_provider) = config.validator_providers().into_iter().next().unwrap();
    let update = RecordedNetworkConfigUpdate {
        validator,
        x25519_key: x25519::Keypair::generate()?.public_key(),
        // Legacy consensus does not dial these, so any address will do.
        p2p_addr: "127.0.0.1:9000".parse()?,
    };
    update_network_config(
        validator_provider,
        st_addr,
        update.x25519_key,
        update.p2p_addr.clone(),
    )
    .await?
    .get_receipt()
    .await?;
    Ok(update)
}

/// What `TestConfig::init_node` gives a validator, as a genesis file.
fn genesis(
    cfg: &TestConfig<NUM_NODES>,
    genesis_state: &ValidatedState,
    upgrade: Upgrade,
) -> Genesis {
    let hotshot = cfg.hotshot_config();
    let chain_config = genesis_state
        .chain_config
        .resolve()
        .expect("test states carry a full chain config");
    Genesis {
        base_version: upgrade.base,
        upgrade_version: upgrade.target,
        genesis_version: upgrade.base,
        epoch_height: Some(hotshot.epoch_height),
        drb_difficulty: Some(hotshot.drb_difficulty),
        drb_upgrade_difficulty: Some(hotshot.drb_upgrade_difficulty),
        epoch_start_block: Some(hotshot.epoch_start_block),
        stake_table_capacity: Some(hotshot.stake_table_capacity),
        chain_config,
        stake_table: StakeTableConfig {
            capacity: hotshot.stake_table_capacity,
        },
        accounts: [(
            TestConfig::<NUM_NODES>::builder_key().fee_account(),
            U256::MAX.into(),
        )]
        .into_iter()
        .collect(),
        l1_finalized: L1Finalized::Number { number: 0 },
        header: GenesisHeader {
            chain_config,
            ..Default::default()
        },
        upgrades: cfg.upgrades(),
        da_committees: None,
    }
}
