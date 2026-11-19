//! A consensus node: consensus over its store, on the network of the validators, with the
//! consensus API for its ingester and API service.

use std::{
    collections::HashSet,
    net::SocketAddr,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail, ensure};
use consensus::{
    coordinator::{Coordinator, error::Severity},
    discovery::ServiceRecord,
    network::{Cliquenet, Role, message_limit},
    store::{ConsensusStore, Import},
};
use espresso_genesis::Genesis;
use espresso_types::{PubKey, SeqTypes, v0_3::StakeTableError};
use hotshot_types::{
    addr::NetAddr,
    consensus::ConsensusMetricsValue,
    data::EpochNumber,
    drb::drb_difficulty_selector,
    epoch_membership::EpochMembershipCoordinator,
    initializer::{HotShotInitializer, load_start_epoch_info},
    light_client::{StateKeyPair, StateSignKey},
    message::UpgradeLock,
    network::NetworkConfig,
    signature_key::BLSPrivKey,
    traits::{election::Membership as _, metrics::Metrics, signature_key::SignatureKey},
    upgrade_config::UpgradeConfig,
    x25519,
};
use tokio::sync::mpsc::unbounded_channel;
use url::Url;
use vbs::version::Version;
use versions::{NEW_PROTOCOL_VERSION, Upgrade};

use crate::{
    decided_state::DecidedStateWriter, initializer::initializer, membership_store::MembershipStore,
    node_state::node_state,
};

/// How long a view may take before the node times out.
const VIEW_TIMEOUT: Duration = Duration::from_secs(10);

/// What a consensus node needs to run.
pub struct Config {
    pub genesis: Genesis,
    pub staking_key: BLSPrivKey,
    pub state_key: StateSignKey,
    pub x25519_key: x25519::SecretKey,
    pub l1: Vec<Url>,

    /// The directory of the consensus store.
    pub store: PathBuf,

    /// Where to listen for the other consensus nodes. They dial the address the stake table
    /// gives: the genesis file's for the genesis committee, the one registered on L1 otherwise.
    pub bind: NetAddr,

    /// Where to serve the consensus API.
    pub consensus_api: SocketAddr,

    /// The URL of this node's API service, and the key it authenticates requests with, which the
    /// node's signed record tells the other validators.
    pub api_url: Url,
    pub api_key: Option<[u8; 32]>,

    /// How many bytes the decide log may take up before the node holds its vote.
    pub decide_log_limit: u64,

    /// How long a leader waits for transactions before proposing an empty block.
    pub empty_block_delay: Duration,
}

/// Runs a consensus node until consensus stops, which it does only on a critical error.
pub async fn run(config: Config, metrics: Box<dyn Metrics>) -> anyhow::Result<()> {
    let Config {
        genesis,
        staking_key,
        state_key,
        x25519_key,
        l1,
        store,
        bind,
        consensus_api,
        api_url,
        api_key,
        decide_log_limit,
        empty_block_delay,
    } = config;
    let public_key = PubKey::from_private(&staking_key);
    let state_keys = StateKeyPair::from_sign_key(state_key);
    tracing::info!(%public_key, "starting a consensus node");
    let network_config = network_config(&genesis, &public_key)?;
    let upgrade = Upgrade::new(genesis.base_version, genesis.upgrade_version);

    let (store, restored) = ConsensusStore::open(store, async || Ok(nothing_to_import())).await?;
    let membership_store = MembershipStore::new(store.clone());
    let state = node_state(
        &genesis,
        &network_config,
        l1,
        membership_store.clone(),
        &*metrics,
    )
    .await?;
    let membership = state.coordinator.clone();
    let initializer = initializer(state, upgrade, &store, &restored, &membership_store).await?;
    let upgrade_lock = restored_upgrade_lock(upgrade, &initializer);
    let version = running_version(&upgrade_lock);
    ensure!(
        version >= NEW_PROTOCOL_VERSION,
        "the chain runs version {version}, before the new protocol ({NEW_PROTOCOL_VERSION}), the \
         only consensus this node runs"
    );
    seed_membership(&membership, &network_config, version, &initializer).await;

    let block_sizes = genesis.block_sizes();
    let largest = *block_sizes
        .values()
        .max()
        .context("genesis sets no block size")?;
    let mut network = Cliquenet::create(
        format!("espresso-{}", genesis.chain_config.chain_id),
        public_key,
        x25519::Keypair::from(&x25519_key),
        bind,
        [],
        Some(message_limit(largest)),
        upgrade_lock.clone(),
        dyn_clone::clone_box(&*metrics),
    )
    .await?;
    connect_startup_peers(&mut network, &membership, &initializer, version, public_key).await?;

    let record = ServiceRecord {
        epoch: EpochNumber::genesis(),
        api_url: api_url.to_string(),
        api_key,
        // A restart replaces the record sent before it, since the clock has moved on.
        sequence: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
    };
    let anchor_cert2 = restored.anchor.as_ref().and_then(|a| a.cert2.clone());
    let decided_state = DecidedStateWriter::new(&store, initializer.anchor_leaf())
        .context("reading the decided state")?;
    // Requests from other nodes outside consensus, which this node does not answer.
    let (external_tx, mut external_rx) = unbounded_channel();
    tokio::spawn(async move { while external_rx.recv().await.is_some() {} });
    let config = &network_config.config;
    let coordinator = Coordinator::maker()
        .membership_coordinator(membership)
        .network(network)
        .initializer(&initializer)
        .upgrade_lock(upgrade_lock)
        .public_key(public_key)
        .private_key(staking_key)
        .state_private_key(state_keys.sign_key())
        .stake_table_capacity(genesis.stake_table.capacity)
        .timeout_duration(VIEW_TIMEOUT)
        .empty_block_delay(empty_block_delay)
        .block_sizes(block_sizes)
        .store(store.clone())
        .restored(restored)
        .decide_log_limit(decide_log_limit)
        .decided_state(Box::new(decided_state))
        .metrics(&*metrics)
        .consensus_metrics(ConsensusMetricsValue::new(&*metrics))
        .maybe_anchor_cert2(anchor_cert2)
        .upgrade_config(UpgradeConfig {
            start_proposing_view: config.start_proposing_view,
            stop_proposing_view: config.stop_proposing_view,
            start_voting_view: config.start_voting_view,
            stop_voting_view: config.stop_voting_view,
            start_proposing_time: config.start_proposing_time,
            stop_proposing_time: config.stop_proposing_time,
            start_voting_time: config.start_voting_time,
            stop_voting_time: config.stop_voting_time,
        })
        .external_tx(external_tx)
        .service_record(record)
        .make();

    let (addr, server) = consensus::ingest::bind(
        store,
        initializer.epoch_height(),
        coordinator.client_api().clone(),
        consensus_api,
    )
    .with_context(|| format!("binding the consensus API to {consensus_api}"))?;
    tracing::info!(%addr, "serving the consensus API");
    let server = tokio::spawn(server);

    let result = run_coordinator(coordinator).await;
    server.abort();
    result
}

/// Drives `coordinator` until it fails critically.
async fn run_coordinator(mut coordinator: Coordinator<SeqTypes>) -> anyhow::Result<()> {
    coordinator.start();
    loop {
        match coordinator.next_consensus_input().await {
            Ok(input) => coordinator.apply_consensus(input),
            Err(err) if err.severity == Severity::Critical => {
                coordinator.stop().await;
                bail!("consensus stopped: {err}");
            },
            Err(err) => tracing::warn!(%err, "consensus input failed"),
        }
        while let Some(output) = coordinator.outbox_mut().pop_front() {
            match coordinator.process_consensus_output(output) {
                Ok(_) => {},
                Err(err) if err.severity == Severity::Critical => {
                    coordinator.stop().await;
                    bail!("consensus stopped: {err}");
                },
                Err(err) => tracing::warn!(%err, "consensus output failed"),
            }
        }
    }
}

/// The network config of the chain `genesis` starts, for the node with `key`.
fn network_config(genesis: &Genesis, key: &PubKey) -> anyhow::Result<NetworkConfig<SeqTypes>> {
    let mut network_config = genesis
        .network_config(key)
        .context("the genesis file has no stake table")?;
    let config = &mut network_config.config;
    if let Some(upgrade) = genesis.upgrades.get(&genesis.upgrade_version) {
        upgrade.set_hotshot_config_parameters(config);
    }
    config.epoch_height = genesis.epoch_height.unwrap_or_default();
    config.drb_difficulty = genesis.drb_difficulty.unwrap_or_default();
    config.drb_upgrade_difficulty = genesis.drb_upgrade_difficulty.unwrap_or_default();
    config.epoch_start_block = genesis.epoch_start_block.unwrap_or_default();
    config.stake_table_capacity = genesis.stake_table.capacity;
    if let Some(committees) = &genesis.da_committees {
        config.da_committees = committees.clone();
    }
    Ok(network_config)
}

/// A decided upgrade certificate at or below the base is of a completed upgrade; kept, it would
/// make the lock's version fail once the target moves on.
fn restored_upgrade_lock(
    upgrade: Upgrade,
    initializer: &HotShotInitializer<SeqTypes>,
) -> UpgradeLock<SeqTypes> {
    let decided = initializer
        .decided_upgrade_certificate()
        .filter(|cert| cert.data.new_version > upgrade.base)
        .cloned();
    UpgradeLock::from_certificate(upgrade, &decided)
}

/// The version the chain runs: the decided upgrade's, or the base if none was decided.
fn running_version(upgrade_lock: &UpgradeLock<SeqTypes>) -> Version {
    upgrade_lock
        .decided_upgrade_cert()
        .map(|cert| cert.data.new_version)
        .unwrap_or(upgrade_lock.upgrade().base)
}

/// Gives membership the DRB difficulty, the DA committees the chain runs by now, and the DRB
/// results the store holds.
async fn seed_membership(
    membership: &EpochMembershipCoordinator<SeqTypes>,
    network_config: &NetworkConfig<SeqTypes>,
    version: Version,
    initializer: &HotShotInitializer<SeqTypes>,
) {
    let config = &network_config.config;
    membership.set_drb_difficulty_selector(drb_difficulty_selector(config));
    for committee in &config.da_committees {
        if version >= committee.start_version {
            membership
                .membership()
                .add_da_committee(committee.start_epoch.into(), committee.committee.clone());
        }
    }
    load_start_epoch_info(membership, initializer.start_epoch_info()).await;
}

/// Dials the stake tables around the anchor's epoch, and the validators active on L1 now that
/// those do not hold.
async fn connect_startup_peers(
    network: &mut Cliquenet<SeqTypes>,
    membership: &EpochMembershipCoordinator<SeqTypes>,
    initializer: &HotShotInitializer<SeqTypes>,
    version: Version,
    own_key: PubKey,
) -> anyhow::Result<()> {
    let anchor_epoch = initializer
        .anchor_leaf()
        .epoch(initializer.epoch_height())
        .unwrap_or_else(EpochNumber::genesis);
    if let Err(err) = network.apply_epoch(anchor_epoch, membership) {
        tracing::warn!(%anchor_epoch, %err, "cannot dial the anchor's stake tables");
    }
    let state = initializer.instance_state();
    if state.chain_config.stake_table_contract.is_none() {
        return Ok(());
    }
    let l1_block = state
        .l1_client
        .finalized_number_within_wait()
        .await
        .context("reading the finalized L1 block")?;
    let validators = match membership
        .membership()
        .fetcher()
        .validator_set_at(l1_block, version)
        .await
    {
        Ok(set) => set,
        // A stake table without a valid validator leaves nothing to dial.
        Err(err)
            if matches!(
                err.downcast_ref::<StakeTableError>(),
                Some(StakeTableError::NoValidValidators)
            ) =>
        {
            return Ok(());
        },
        Err(err) => return Err(err.context("reading the validators active on L1")),
    };
    let held: HashSet<PubKey> = network.sender().peer_keys();
    let targets = validators
        .active_validators()
        .values()
        .filter(|v| *v.stake_table_key() != own_key && !held.contains(v.stake_table_key()))
        .filter_map(|v| {
            let x25519 = v.x25519_key?;
            let addr = v.p2p_addr.clone()?;
            Some((*v.stake_table_key(), (x25519.into(), addr)))
        })
        .collect();
    network
        .add_peers(Role::Active, targets)
        .context("dialing the validators active on L1")?;
    Ok(())
}

/// What a node takes over into a new store: nothing, since an a2 node keeps all its state there
/// from the start.
fn nothing_to_import() -> Import<SeqTypes> {
    Import {
        proposals: Vec::new(),
        locked_qc: None,
        boundary_qc: None,
        acted_view: None,
        restart_view: None,
        upgrade_cert: None,
        state_cert: None,
        payloads: Vec::new(),
        anchor: None,
        app: Vec::new(),
    }
}
