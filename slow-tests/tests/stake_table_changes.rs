//! Integration tests for radical stake table changes, driving a real stake
//! table contract on a real L1 (anvil) while the network keeps deciding.
//!
//! Timing: the committee for epoch K is selected at the epoch root of epoch
//! K-2 from the events finalized on L1 at that root, and genesis seeds epochs
//! 1-2, so a change submitted during epoch E activates at E+2 at the earliest.

use std::{collections::HashSet, time::Duration};

use alloy::primitives::{Address, U256, utils::parse_ether};
use espresso_contract_deployer::Contract;
use espresso_node::{
    SequencerApiVersion,
    api::{
        Options,
        data_source::{SequencerDataSource, testing::TestableSequencerDataSource},
        sql::DataSource as SqlDataSource,
        test_helpers::{
            TestNetwork, TestNetworkConfigBuilder, assert_node_live, assert_nodes_agree,
            committee_is, delegate_new, deregister_validators, register_validators,
            staking_addresses, wait_for_committee,
        },
    },
    catchup::StatePeers,
    testing::{TestConfig, TestConfigBuilder, wait_for_epochs},
};
use espresso_types::{AuthenticatedValidatorMap, PubKey, ValidatedState};
use futures::future::join_all;
use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
use hotshot_types::{
    addr::NetAddr,
    light_client::StateKeyPair,
    signature_key::BLSKeyPair,
    traits::{metrics::NoMetrics, signature_key::SignatureKey},
    x25519,
};
use http_client::{Client, error::ClientErr};
use slow_tests::BUILDER_TIMEOUT;
use staking_cli::{
    NodeSignatures, Transaction as StakingTransaction, demo::DelegationConfig,
    update_network_config,
};
use test_utils::reserve_tcp_port;
use tokio::time::timeout;
use versions::{NEW_PROTOCOL_VERSION, Upgrade};

const V6: Upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);

/// The first epoch whose committee is driven by the stake table contract
/// (epochs 1-2 are seeded from the genesis stake table).
const FIRST_CONTRACT_EPOCH: u64 = 3;

/// How many epochs we allow for a stake-table change to finalize on L1 and
/// reach a stake table snapshot before failing.
const MAX_ACTIVATION_EPOCHS: u64 = 10;

type SqlPersistence = <SqlDataSource as SequencerDataSource>::Options;

/// State-peers catchup pointed at node 0's query API.
fn node_catchup(api_port: u16) -> StatePeers<SequencerApiVersion> {
    StatePeers::from_urls(
        vec![format!("http://localhost:{api_port}").parse().unwrap()],
        Default::default(),
        Duration::from_secs(2),
        &NoMetrics,
    )
}

/// A running network for stake-table-change tests: SQL storage per node,
/// node 0 serving the query API, state-peers catchup pointed at node 0, and
/// only the validators at `registered` indices staked on the contract.
struct StakeTableTestNetwork<const NUM_NODES: usize> {
    network: TestNetwork<SqlPersistence, NUM_NODES>,
    client: Client<ClientErr, SequencerApiVersion>,
    stake_table: Address,
    api_port: u16,
    /// The upgrade the network was started with, reused when starting or
    /// restarting nodes.
    upgrade: Upgrade,
    /// Every node's genesis state (its chain config carries the stake table
    /// address), reused for deferred-started nodes.
    genesis_state: ValidatedState,
    // Keeps the temporary databases alive for the duration of the test.
    _storage: Vec<<SqlDataSource as TestableSequencerDataSource>::Storage>,
}

impl<const NUM_NODES: usize> StakeTableTestNetwork<NUM_NODES> {
    async fn start(
        network_config: TestConfig<NUM_NODES>,
        upgrade: Upgrade,
        stake_table_version: StakeTableContractVersion,
        delegation_config: DelegationConfig,
        registered: &[usize],
        // Nodes started later via [`Self::start_deferred_node`].
        deferred: &[usize],
        // Initial ESP supply in whole tokens; the deployer keeps whatever the
        // validator/delegator funding does not consume.
        token_supply: Option<U256>,
    ) -> Self {
        let api_port = reserve_tcp_port().expect("No ports free for query service");

        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let mut builder = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(api_port),
            ))
            .network_config(network_config)
            .persistences(persistence)
            .deferred_start(deferred)
            .catchups(std::array::from_fn(|_| node_catchup(api_port)));
        if let Some(supply) = token_supply {
            builder = builder.initial_token_supply(supply);
        }
        let config = builder
            .pos_hook_with_registered(delegation_config, stake_table_version, upgrade, registered)
            .await
            .unwrap()
            .build();
        let genesis_state = config.states()[0].clone();

        let network = TestNetwork::new(config, upgrade).await;
        let stake_table = network
            .contracts
            .as_ref()
            .unwrap()
            .address(Contract::StakeTableProxy)
            .unwrap();

        let client: Client<ClientErr, SequencerApiVersion> =
            Client::new(format!("http://localhost:{api_port}").parse().unwrap());
        client.connect(Some(Duration::from_secs(30))).await;

        Self {
            network,
            client,
            stake_table,
            api_port,
            upgrade,
            genesis_state,
            _storage: storage,
        }
    }

    /// Starts a node deferred at [`Self::start`], with its reserved SQL
    /// storage slot and catchup from node 0's query API.
    async fn start_deferred_node(&mut self, i: usize) {
        let persistence =
            <SqlDataSource as TestableSequencerDataSource>::persistence_options(&self._storage[i]);
        self.network
            .start_deferred_node(
                i,
                self.genesis_state.clone(),
                persistence,
                node_catchup(self.api_port),
                self.upgrade,
            )
            .await;
    }

    /// Restarts node `i` on the network's current configuration — picking up
    /// any rotated consensus keys or coordinator address — reusing its SQL
    /// storage slot and catchup from node 0's query API.
    async fn restart_node(&mut self, i: usize) {
        let persistence =
            <SqlDataSource as TestableSequencerDataSource>::persistence_options(&self._storage[i]);
        self.network
            .restart_node(
                i,
                self.genesis_state.clone(),
                persistence,
                node_catchup(self.api_port),
                self.upgrade,
            )
            .await;
    }

    /// The committee reported for `epoch` by node 0's query API, retried
    /// while the node finishes the epoch's snapshot: the membership endpoint
    /// errors rather than waits when the epoch's DRB is still being computed.
    async fn committee(&self, epoch: u64) -> AuthenticatedValidatorMap {
        let mut last_err = None;
        for _ in 0..60 {
            match self
                .client
                .get::<AuthenticatedValidatorMap>(&format!("node/validators/{epoch}"))
                .send()
                .await
            {
                Ok(committee) => return committee,
                Err(err) => last_err = Some(err),
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        panic!(
            "validators for epoch {epoch}: {}",
            last_err.expect("at least one attempt")
        );
    }
}

/// The committee {0..3} is replaced wholesale by the disjoint set {4..7};
/// the chain must keep deciding throughout.
///
/// Node 0 — the query node — is in the *outgoing* set on purpose: it
/// validated the pre-swap chain, so it can serve the incoming cohort's
/// state catchup at the handoff. At 0.6 the outgoing nodes legitimately
/// stall once dropped from the cliquenet peer windows, so nothing is
/// asserted on them after the swap and progress is observed through an
/// incoming node's event stream.
async fn full_set_replacement(version: Upgrade, epoch_height: u64) -> anyhow::Result<()> {
    const NUM_NODES: usize = 8;
    let outgoing = [0, 1, 2, 3];
    let incoming = [4, 5, 6, 7];

    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(epoch_height)
        .builder_timeout(BUILDER_TIMEOUT)
        .epoch_start_block(0)
        .build();

    let net = StakeTableTestNetwork::start(
        network_config.clone(),
        version,
        StakeTableContractVersion::V3,
        DelegationConfig::MultipleDelegators,
        &outgoing,
        &[],
        None,
    )
    .await;

    let outgoing_addrs = staking_addresses(&network_config, &outgoing);
    let incoming_addrs = staking_addresses(&network_config, &incoming);

    // Observe progress through an incoming node: it follows the chain
    // before its membership activates and keeps deciding afterwards.
    let mut events = net.network.node(incoming[0]).event_stream();

    // Send the swap while epoch 2 is running, so the events are finalized on
    // L1 well before the roots that fix epochs 4 and 5.
    wait_for_epochs(&mut events, epoch_height, 1).await;

    let epoch3 = net.committee(FIRST_CONTRACT_EPOCH).await;
    assert_eq!(
        epoch3.keys().copied().collect::<HashSet<_>>(),
        outgoing_addrs,
        "the first contract-driven committee should be exactly the initially registered set"
    );

    // Register the incoming set first so no snapshot can ever see an empty
    // stake table, then deregister every original validator.
    register_validators(
        &network_config,
        net.stake_table,
        &incoming,
        DelegationConfig::MultipleDelegators,
    )
    .await?;
    deregister_validators(&network_config, net.stake_table, &outgoing).await?;

    let (activation_epoch, committee) = wait_for_committee(
        &net.client,
        &mut events,
        epoch_height,
        FIRST_CONTRACT_EPOCH,
        MAX_ACTIVATION_EPOCHS,
        committee_is(incoming_addrs),
    )
    .await;
    tracing::info!(activation_epoch, "full set replacement activated");

    if version.base >= NEW_PROTOCOL_VERSION {
        // Cliquenet connects the committees of epochs {e-1, e, e+1} at epoch
        // e; the incoming nodes (members of genesis epochs 1-2) stay
        // continuously connected only if the swap activates by epoch 5.
        assert!(
            activation_epoch <= 5,
            "swap activated at epoch {activation_epoch}, too late for continuous cliquenet peer \
             windows"
        );
        for (address, validator) in &committee {
            assert!(
                validator.x25519_key.is_some() && validator.p2p_addr.is_some(),
                "incoming validator {address} is missing cliquenet connect info"
            );
        }
    }

    assert_node_live(net.network.node(incoming[0]), epoch_height, 2).await;
    let incoming_nodes: Vec<_> = incoming.iter().map(|&i| net.network.node(i)).collect();
    assert_nodes_agree(&incoming_nodes, activation_epoch * epoch_height).await;

    Ok(())
}

/// Full set replacement at 0.6: the incoming committee holds no pre-swap VID
/// shares and joins via the boundary handoff (the seeded Cert2-final
/// boundary state plus catchup from node 0). HotShot-layer counterpart:
/// `hotshot-new-protocol`'s `validator_set_replaced_at_epoch_boundary`.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_full_set_replacement_v6() -> anyhow::Result<()> {
    full_set_replacement(V6, 20).await
}

/// The committee starts as the initial cohort, grows to all 8 nodes when
/// the rest register mid-run, and shrinks back when they deregister again.
/// Node 0 is a member throughout, so the query API never depends on the
/// changing cohort. At 0.6 the grown committee's first block needs joiner
/// votes (4 continuing members are below the 6-of-8 threshold), so this
/// also exercises the boundary handoff for joins (see
/// `test_stake_table_full_set_replacement_v6`).
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_grow_and_shrink() -> anyhow::Result<()> {
    let version = V6;
    const NUM_NODES: usize = 8;
    const EPOCH_HEIGHT: u64 = 15;
    const INITIAL_COUNT: usize = 4;
    let initial: Vec<usize> = (0..INITIAL_COUNT).collect();
    let joining: Vec<usize> = (INITIAL_COUNT..NUM_NODES).collect();

    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .builder_timeout(BUILDER_TIMEOUT)
        .epoch_start_block(0)
        .build();

    let net = StakeTableTestNetwork::start(
        network_config.clone(),
        version,
        StakeTableContractVersion::V3,
        DelegationConfig::EqualAmounts,
        &initial,
        &[],
        None,
    )
    .await;

    let initial_addrs = staking_addresses(&network_config, &initial);
    let all_addrs = staking_addresses(&network_config, &(0..NUM_NODES).collect::<Vec<_>>());

    // Grow: register the second cohort while epoch 2 is running.
    let mut events = net.network.server.event_stream();
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 1).await;
    register_validators(
        &network_config,
        net.stake_table,
        &joining,
        DelegationConfig::EqualAmounts,
    )
    .await?;

    let (grow_epoch, _) = wait_for_committee(
        &net.client,
        &mut events,
        EPOCH_HEIGHT,
        FIRST_CONTRACT_EPOCH,
        MAX_ACTIVATION_EPOCHS,
        committee_is(all_addrs),
    )
    .await;
    tracing::info!(grow_epoch, "committee grew to the full node set");
    if version.base >= NEW_PROTOCOL_VERSION {
        assert!(
            grow_epoch <= 5,
            "grow activated at epoch {grow_epoch}, too late for continuous cliquenet peer windows"
        );
    }
    assert_node_live(&net.network.server, EPOCH_HEIGHT, 1).await;

    // Shrink: the second cohort deregisters again (but keeps running).
    deregister_validators(&network_config, net.stake_table, &joining).await?;
    let (shrink_epoch, committee) = wait_for_committee(
        &net.client,
        &mut events,
        EPOCH_HEIGHT,
        grow_epoch + 1,
        MAX_ACTIVATION_EPOCHS,
        committee_is(initial_addrs),
    )
    .await;
    tracing::info!(shrink_epoch, "committee shrank back to the initial set");
    assert_eq!(committee.len(), initial.len());

    assert_node_live(&net.network.server, EPOCH_HEIGHT, 2).await;
    let initial_nodes: Vec<_> = initial.iter().map(|&i| net.network.node(i)).collect();
    assert_nodes_agree(&initial_nodes, shrink_epoch * EPOCH_HEIGHT).await;

    Ok(())
}

/// No registration events at all: delegation moves alone reshape the active
/// set. Validators 3-4 fully undelegate (zero stake filters them out of
/// `select_active_validator_set`), then fresh delegations bring them back —
/// at 0.6 the first post-return block needs a rejoiner's vote (3 continuing
/// members, threshold 4 of 5), exercising the boundary handoff.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_delegation_reshuffle() -> anyhow::Result<()> {
    let version = V6;
    const NUM_NODES: usize = 5;
    const EPOCH_HEIGHT: u64 = 15;
    let dropped: &[usize] = &[3, 4];
    // Every validator self-delegates exactly 100 ESP, so full undelegation
    // amounts and threshold math are deterministic.
    let stake = parse_ether("100").unwrap();
    let remaining: Vec<usize> = (0..NUM_NODES).filter(|i| !dropped.contains(i)).collect();

    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .builder_timeout(BUILDER_TIMEOUT)
        .epoch_start_block(0)
        .build();

    let net = StakeTableTestNetwork::start(
        network_config.clone(),
        version,
        StakeTableContractVersion::V3,
        DelegationConfig::EqualAmounts,
        &(0..NUM_NODES).collect::<Vec<_>>(),
        &[],
        None,
    )
    .await;
    let token = net
        .network
        .contracts
        .as_ref()
        .unwrap()
        .address(Contract::EspTokenProxy)
        .unwrap();

    let all_addrs = staking_addresses(&network_config, &(0..NUM_NODES).collect::<Vec<_>>());
    let remaining_addrs = staking_addresses(&network_config, &remaining);
    let providers = network_config.validator_providers();

    // The dropped validators fully undelegate; with zero stake and no
    // delegators they must drop out of the active set.
    let mut events = net.network.server.event_stream();
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 1).await;
    for &i in dropped {
        let (validator, provider) = &providers[i];
        let receipt = StakingTransaction::Undelegate {
            stake_table: net.stake_table,
            validator: *validator,
            amount: stake,
        }
        .send(provider)
        .await?
        .get_receipt()
        .await?;
        anyhow::ensure!(receipt.status(), "undelegation of validator {i} reverted");
    }
    let (drop_epoch, _) = wait_for_committee(
        &net.client,
        &mut events,
        EPOCH_HEIGHT,
        FIRST_CONTRACT_EPOCH,
        MAX_ACTIVATION_EPOCHS,
        committee_is(remaining_addrs),
    )
    .await;
    tracing::info!(drop_epoch, "undelegated validators left the committee");

    // Fresh delegators bring them back. (The validators' own tokens are in
    // withdrawal escrow, so new stake has to come from new delegations.)
    for &i in dropped {
        delegate_new(
            &network_config,
            token,
            net.stake_table,
            providers[i].0,
            stake,
        )
        .await?;
    }
    let (return_epoch, _) = wait_for_committee(
        &net.client,
        &mut events,
        EPOCH_HEIGHT,
        drop_epoch + 1,
        MAX_ACTIVATION_EPOCHS,
        committee_is(all_addrs.clone()),
    )
    .await;
    tracing::info!(
        return_epoch,
        "re-delegated validators rejoined the committee"
    );
    assert_node_live(&net.network.server, EPOCH_HEIGHT, 1).await;

    Ok(())
}

/// A brand-new validator joins the running network: node 4 starts several
/// epochs in with no history, syncs through catchup from node 0's query
/// API, and must be selected into the committee and participate from its
/// activation epoch. (It is part of the genesis-seeded committees of epochs
/// 1-2 but offline for them, so its leader views there time out.)
async fn fresh_node_joins(version: Upgrade, epoch_height: u64) -> anyhow::Result<()> {
    const NUM_NODES: usize = 5;
    const FRESH: usize = 4;
    let initial = [0, 1, 2, 3];

    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(epoch_height)
        .builder_timeout(BUILDER_TIMEOUT)
        .epoch_start_block(0)
        .build();

    let mut net = StakeTableTestNetwork::start(
        network_config.clone(),
        version,
        StakeTableContractVersion::V3,
        DelegationConfig::EqualAmounts,
        &initial,
        &[FRESH],
        None,
    )
    .await;

    let all_addrs = staking_addresses(&network_config, &(0..NUM_NODES).collect::<Vec<_>>());
    let mut events = net.network.server.event_stream();
    wait_for_epochs(&mut events, epoch_height, 1).await;

    register_validators(
        &network_config,
        net.stake_table,
        &[FRESH],
        DelegationConfig::EqualAmounts,
    )
    .await?;
    net.start_deferred_node(FRESH).await;

    let (activation_epoch, committee) = wait_for_committee(
        &net.client,
        &mut events,
        epoch_height,
        FIRST_CONTRACT_EPOCH,
        MAX_ACTIVATION_EPOCHS,
        committee_is(all_addrs),
    )
    .await;
    tracing::info!(activation_epoch, "fresh validator joined the committee");

    if version.base >= NEW_PROTOCOL_VERSION {
        for (address, validator) in &committee {
            assert!(
                validator.x25519_key.is_some() && validator.p2p_addr.is_some(),
                "validator {address} is missing cliquenet connect info"
            );
        }
    }

    assert_node_live(&net.network.server, epoch_height, 2).await;

    let mut fresh_events = net.network.node(FRESH).event_stream();
    timeout(
        Duration::from_secs(600),
        wait_for_epochs(&mut fresh_events, epoch_height, activation_epoch),
    )
    .await
    .expect("the fresh node did not catch up to its activation epoch");
    assert_node_live(net.network.node(FRESH), epoch_height, 1).await;

    let all_nodes: Vec<_> = (0..NUM_NODES).map(|i| net.network.node(i)).collect();
    assert_nodes_agree(&all_nodes, activation_epoch * epoch_height).await;

    Ok(())
}

/// Fresh join at 0.6: the node is outside every cliquenet peer window until
/// its activation epoch's committees connect to it, so all of its syncing
/// happens in the epoch before its duties begin; see [`fresh_node_joins`].
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_fresh_node_joins_v6() -> anyhow::Result<()> {
    Box::pin(fresh_node_joins(V6, 20)).await
}

/// How [`rotate_validator`] rotates the validator's on-chain identity.
enum Rotation {
    /// A new cliquenet p2p address (`updateP2pAddr`, x25519 key unchanged),
    /// the way an operator moving a node to a new host would publish it.
    /// Peers merge the rotated connect info an epoch before its activation
    /// epoch and redial, so the rotated node remains a live participant
    /// throughout.
    P2pAddr,
    /// Fresh BLS and Schnorr keys (`updateConsensusKeysV2`) together with
    /// the x25519 key derived from the new BLS key (`updateNetworkConfig`).
    /// Until the rotation activates, the restarted node is a stranger to its
    /// peers — the old identity leaves the cliquenet peer windows and the
    /// new one joins via the epoch-boundary handoff — so the node has to
    /// follow through catchup and re-enter the committee under its new
    /// identity.
    ConsensusKeys,
}

/// A committee validator rotates part of its on-chain identity mid-run (see
/// [`Rotation`]) and restarts on the rotated configuration. The rotation
/// must reach an active committee snapshot, the chain must keep deciding
/// throughout, and the rotated node must decide past its activation epoch.
async fn rotate_validator(rotation: Rotation) -> anyhow::Result<()> {
    const NUM_NODES: usize = 5;
    const EPOCH_HEIGHT: u64 = 20;
    const ROTATED: usize = 1;

    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .builder_timeout(BUILDER_TIMEOUT)
        .epoch_start_block(0)
        .build();

    let mut net = StakeTableTestNetwork::start(
        network_config.clone(),
        V6,
        StakeTableContractVersion::V3,
        DelegationConfig::EqualAmounts,
        &(0..NUM_NODES).collect::<Vec<_>>(),
        &[],
        None,
    )
    .await;

    let mut events = net.network.server.event_stream();
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 1).await;

    let (account, provider) = network_config.validator_providers().remove(ROTATED);
    let activated: Box<dyn Fn(&AuthenticatedValidatorMap) -> bool> = match rotation {
        Rotation::P2pAddr => {
            let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
            let new_addr: NetAddr = format!("127.0.0.1:{port}").parse().expect("valid address");
            let receipt = StakingTransaction::UpdateP2pAddr {
                stake_table: net.stake_table,
                p2p_addr: new_addr.clone(),
            }
            .send(&provider)
            .await?
            .get_receipt()
            .await?;
            anyhow::ensure!(receipt.status(), "p2p address update reverted");

            net.network
                .cfg
                .set_coordinator_addr(ROTATED, new_addr.clone());
            Box::new(move |committee| {
                committee
                    .get(&account)
                    .is_some_and(|v| v.p2p_addr.as_ref() == Some(&new_addr))
            })
        },
        Rotation::ConsensusKeys => {
            let (new_pub, new_bls) = PubKey::generated_from_seed_indexed([1; 32], ROTATED as u64);
            let new_state = StateKeyPair::generate_from_seed_indexed([1; 32], ROTATED as u64);
            let new_x25519 = x25519::Keypair::derive_from::<PubKey>(&new_bls)
                .expect("x25519 keypair derivation should succeed");

            // Send both halves of the rotation before awaiting either
            // receipt, so no epoch root can land in between and pair the new
            // BLS key with the old x25519 key.
            let keys_tx = StakingTransaction::UpdateConsensusKeys {
                stake_table: net.stake_table,
                payload: NodeSignatures::create(
                    account,
                    &BLSKeyPair::from(new_bls.clone()),
                    &new_state,
                ),
                version: StakeTableContractVersion::V3,
            }
            .send(&provider)
            .await?;
            let config_tx = update_network_config(
                &provider,
                net.stake_table,
                new_x25519.public_key(),
                network_config.coordinator_addr(ROTATED),
            )
            .await?;
            anyhow::ensure!(
                keys_tx.get_receipt().await?.status(),
                "consensus keys update reverted"
            );
            anyhow::ensure!(
                config_tx.get_receipt().await?.status(),
                "network config update reverted"
            );

            net.network
                .cfg
                .set_consensus_keys(ROTATED, new_bls, new_state);
            let expected_x25519 = new_x25519.public_key();
            Box::new(move |committee| {
                committee.get(&account).is_some_and(|v| {
                    v.stake_table_key.as_ref() == Some(&new_pub)
                        && v.x25519_key == Some(expected_x25519)
                })
            })
        },
    };
    net.restart_node(ROTATED).await;

    let (activation_epoch, _) = wait_for_committee(
        &net.client,
        &mut events,
        EPOCH_HEIGHT,
        FIRST_CONTRACT_EPOCH,
        MAX_ACTIVATION_EPOCHS,
        activated,
    )
    .await;
    tracing::info!(activation_epoch, "rotation activated");

    assert_node_live(&net.network.server, EPOCH_HEIGHT, 2).await;

    let mut rotated_events = net.network.node(ROTATED).event_stream();
    timeout(
        Duration::from_secs(600),
        wait_for_epochs(&mut rotated_events, EPOCH_HEIGHT, activation_epoch),
    )
    .await
    .expect("the rotated node did not keep deciding past its activation epoch");
    assert_node_live(net.network.node(ROTATED), EPOCH_HEIGHT, 1).await;

    let all_nodes: Vec<_> = (0..NUM_NODES).map(|i| net.network.node(i)).collect();
    assert_nodes_agree(&all_nodes, activation_epoch * EPOCH_HEIGHT).await;

    Ok(())
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_rotate_p2p_address_v6() -> anyhow::Result<()> {
    Box::pin(rotate_validator(Rotation::P2pAddr)).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_rotate_consensus_keys_v6() -> anyhow::Result<()> {
    Box::pin(rotate_validator(Rotation::ConsensusKeys)).await
}
