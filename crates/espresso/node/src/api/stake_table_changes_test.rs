//! Integration tests for radical stake table changes, driving a real stake
//! table contract on a real L1 (anvil) while the network keeps deciding.
//!
//! Timing: the committee for epoch K is selected at the epoch root of epoch
//! K-2 from the events finalized on L1 at that root, and genesis seeds epochs
//! 1-2, so a change submitted during epoch E activates at E+2 at the earliest.

use std::{collections::HashSet, time::Duration};

use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256, utils::parse_ether},
    providers::{ProviderBuilder, ext::AnvilApi},
};
use espresso_contract_deployer::{Contract, upgrade_stake_table_v3};
use espresso_types::{
    AuthenticatedValidatorMap, Header, ValidatedState, v0::traits::SequencerPersistence,
};
use futures::{
    future::join_all,
    stream::{Stream, StreamExt},
};
use hotshot::types::EventType;
use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
use hotshot_query_service::{availability::LeafQueryData, types::HeightIndexed};
use hotshot_types::{
    new_protocol::CoordinatorEvent, traits::metrics::NoMetrics, utils::epoch_from_block_number,
};
use rstest::rstest;
use staking_cli::{
    Transaction as StakingTransaction, demo::DelegationConfig, update_network_config,
};
use surf_disco::Client;
use test_utils::reserve_tcp_port;
use tide_disco::error::ServerError;
use tokio::time::timeout;
use vbs::version::Version;
use versions::{
    DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_REWARD_VERSION, NEW_PROTOCOL_VERSION, Upgrade,
};

use super::{
    Options,
    data_source::{SequencerDataSource, testing::TestableSequencerDataSource},
    sql::DataSource as SqlDataSource,
    test_helpers::{
        TestNetwork, TestNetworkConfigBuilder, assert_node_live, assert_nodes_agree, committee_is,
        delegate_new, deregister_validators, register_validators, staking_addresses,
        wait_for_committee,
    },
};
use crate::{
    SeqTypes, SequencerApiVersion, SequencerContext,
    catchup::StatePeers,
    network,
    testing::{TestConfig, TestConfigBuilder, wait_for_epochs},
};

const V5: Upgrade = Upgrade::trivial(EPOCH_REWARD_VERSION);
const V6: Upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);

/// The first epoch whose committee is driven by the stake table contract
/// (epochs 1-2 are seeded from the genesis stake table).
const FIRST_CONTRACT_EPOCH: u64 = 3;

/// How many epochs we allow for a stake-table change to finalize on L1 and
/// reach a stake table snapshot before failing.
const MAX_ACTIVATION_EPOCHS: u64 = 10;

type SqlPersistence = <SqlDataSource as SequencerDataSource>::Options;

/// A running network for stake-table-change tests: SQL storage per node,
/// node 0 serving the query API, state-peers catchup pointed at node 0, and
/// only the validators at `registered` indices staked on the contract.
struct StakeTableTestNetwork<const NUM_NODES: usize> {
    network: TestNetwork<SqlPersistence, NUM_NODES>,
    client: Client<ServerError, SequencerApiVersion>,
    stake_table: Address,
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
            .catchups(std::array::from_fn(|_| {
                StatePeers::<SequencerApiVersion>::from_urls(
                    vec![format!("http://localhost:{api_port}").parse().unwrap()],
                    Default::default(),
                    Duration::from_secs(2),
                    &NoMetrics,
                )
            }));
        if let Some(supply) = token_supply {
            builder = builder.initial_token_supply(supply);
        }
        let config = builder
            .pos_hook_with_registered(delegation_config, stake_table_version, upgrade, registered)
            .await
            .unwrap()
            .build();

        let network = TestNetwork::new(config, upgrade).await;
        let stake_table = network
            .contracts
            .as_ref()
            .unwrap()
            .address(Contract::StakeTableProxy)
            .unwrap();

        let client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{api_port}").parse().unwrap());
        client.connect(Some(Duration::from_secs(30))).await;

        Self {
            network,
            client,
            stake_table,
            _storage: storage,
        }
    }

    /// Starts a network that performs a real protocol upgrade mid-run. The
    /// contracts must already be deployed via
    /// [`TestConfigBuilder::set_upgrades_with`] on `network_config`; genesis
    /// carries the upgraded chain config (required whenever the base version
    /// already has epochs).
    async fn start_upgrading(network_config: TestConfig<NUM_NODES>, upgrade: Upgrade) -> Self {
        // `set_upgrades_with` does not enable interval mining (`pos_hook`
        // does); without it the auto-mining L1 only produces blocks on
        // transactions and `l1_finalized` never advances.
        network_config
            .anvil()
            .expect("TestConfigBuilder starts an anvil")
            .anvil_set_interval_mining(1)
            .await
            .expect("interval mining");

        let genesis_state = ValidatedState {
            chain_config: network_config
                .get_upgrade_map()
                .chain_config(upgrade.target)
                .into(),
            ..Default::default()
        };

        let api_port = reserve_tcp_port().expect("No ports free for query service");

        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(api_port),
            ))
            .network_config(network_config.clone())
            .persistences(persistence)
            .states(std::array::from_fn(|_| genesis_state.clone()))
            .catchups(std::array::from_fn(|_| {
                StatePeers::<SequencerApiVersion>::from_urls(
                    vec![format!("http://localhost:{api_port}").parse().unwrap()],
                    Default::default(),
                    Duration::from_secs(2),
                    &NoMetrics,
                )
            }))
            .build();

        let network = TestNetwork::new(config, upgrade).await;
        let stake_table = network_config
            .contracts()
            .expect("set_upgrades_with deploys the contracts")
            .address(Contract::StakeTableProxy)
            .unwrap();

        let client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{api_port}").parse().unwrap());
        client.connect(Some(Duration::from_secs(30))).await;

        Self {
            network,
            client,
            stake_table,
            _storage: storage,
        }
    }

    /// The committee reported for `epoch` by node 0's query API.
    async fn committee(&self, epoch: u64) -> AuthenticatedValidatorMap {
        self.client
            .get::<AuthenticatedValidatorMap>(&format!("node/validators/{epoch}"))
            .send()
            .await
            .unwrap_or_else(|err| panic!("validators for epoch {epoch}: {err}"))
    }

    /// Streams decided leaves from the query service until one reaches
    /// `version` or newer; returns its height.
    async fn wait_for_version(&self, version: Version, deadline: Duration) -> u64 {
        let mut leaves = self
            .client
            .socket("availability/stream/leaves/0")
            .subscribe::<LeafQueryData<SeqTypes>>()
            .await
            .unwrap();
        timeout(deadline, async {
            loop {
                let leaf = leaves.next().await.unwrap().unwrap();
                if leaf.header().version() >= version {
                    break leaf.height();
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("the network did not upgrade to {version} in time"))
    }

    /// The header at `height`, retried while the query service catches up to
    /// the decided chain.
    async fn header_at(&self, height: u64) -> Header {
        for _ in 0..30 {
            if let Ok(header) = self
                .client
                .get::<Header>(&format!("availability/header/{height}"))
                .send()
                .await
            {
                return header;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        panic!("header {height} not served by the query service in time");
    }
}

/// Streams decided leaves from a node's coordinator events until one reaches
/// `version` or newer; returns its height. Unlike
/// [`StakeTableTestNetwork::wait_for_version`] this does not depend on the
/// query node staying live, so it also works when the query node is in the
/// outgoing cohort of a swap.
async fn wait_for_version_on_events(
    events: &mut (impl Stream<Item = CoordinatorEvent<SeqTypes>> + Unpin),
    version: Version,
    deadline: Duration,
) -> u64 {
    timeout(deadline, async {
        loop {
            let leaf = match events.next().await.unwrap() {
                CoordinatorEvent::LegacyEvent(hotshot::types::Event {
                    event: EventType::Decide { leaf_chain, .. },
                    ..
                }) => leaf_chain[0].leaf.clone(),
                CoordinatorEvent::NewDecide { leaf_infos, .. } => leaf_infos[0].leaf.clone(),
                _ => continue,
            };
            if leaf.block_header().version() >= version {
                break leaf.height();
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("the network did not upgrade to {version} in time"))
}

/// Blocks until the legacy consensus proposes an upgrade, asserting it
/// targets `version`.
async fn wait_for_upgrade_proposal<P: SequencerPersistence>(
    node: &SequencerContext<network::Memory, P>,
    version: Version,
) {
    let mut events = node
        .consensus_handle()
        .legacy_consensus()
        .read()
        .await
        .event_stream();
    loop {
        let event = events.next().await.unwrap();
        if let EventType::UpgradeProposal { proposal, .. } = event.event {
            let new_version = proposal.data.upgrade_proposal.new_version;
            assert_eq!(new_version, version, "unexpected upgrade target proposed");
            return;
        }
    }
}

/// Which point of a protocol upgrade the swap transactions are sent at.
#[derive(Clone, Copy, Debug)]
enum SwapTrigger {
    /// While epoch 2 is running, long before the upgrade window: the swap
    /// activates first and the network upgrades on the replaced committee.
    BeforeUpgrade,
    /// The moment the `UpgradeProposal` is observed, so the swap's activation
    /// lands inside the upgrade window.
    AtUpgradeProposal,
    /// After the first upgraded leaf is decided.
    AfterUpgrade,
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
        .epoch_start_block(0)
        .build();

    let net = StakeTableTestNetwork::start(
        network_config.clone(),
        version,
        StakeTableContractVersion::V3,
        DelegationConfig::MultipleDelegators,
        &outgoing,
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

/// Full set replacement at 0.5: see [`full_set_replacement`].
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_full_set_replacement_v5() -> anyhow::Result<()> {
    full_set_replacement(V5, 15).await
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
#[rstest]
#[case::v5(V5)]
#[case::v6(V6)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_grow_and_shrink(#[case] version: Upgrade) -> anyhow::Result<()> {
    const NUM_NODES: usize = 8;
    const EPOCH_HEIGHT: u64 = 15;
    const INITIAL_COUNT: usize = 4;
    let initial: Vec<usize> = (0..INITIAL_COUNT).collect();
    let joining: Vec<usize> = (INITIAL_COUNT..NUM_NODES).collect();

    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .build();

    let net = StakeTableTestNetwork::start(
        network_config.clone(),
        version,
        StakeTableContractVersion::V3,
        DelegationConfig::EqualAmounts,
        &initial,
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
/// members, threshold 4 of 5), exercising the boundary handoff. The V5 case
/// adds a whale phase: one enormous delegation pushes the minimum-stake
/// threshold (max stake / 1000) above everyone else's stake, shrinking the
/// committee to a single validator; undelegating it restores the full set.
#[rstest]
#[case::v5(V5)]
#[case::v6(V6)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_delegation_reshuffle(#[case] version: Upgrade) -> anyhow::Result<()> {
    const NUM_NODES: usize = 5;
    const EPOCH_HEIGHT: u64 = 15;
    let dropped: &[usize] = &[3, 4];
    // Every validator self-delegates exactly 100 ESP, so full undelegation
    // amounts and threshold math are deterministic.
    let stake = parse_ether("100").unwrap();
    let remaining: Vec<usize> = (0..NUM_NODES).filter(|i| !dropped.contains(i)).collect();

    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .build();

    let net = StakeTableTestNetwork::start(
        network_config.clone(),
        version,
        StakeTableContractVersion::V3,
        DelegationConfig::EqualAmounts,
        &(0..NUM_NODES).collect::<Vec<_>>(),
        // The whale phase needs far more ESP than the funding leaves over
        // from the default supply.
        Some(U256::from(1_000_000u64)),
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

    // Whale phase, V5 only: a single-validator committee at 0.6 would need
    // cliquenet window continuity reasoning that is out of scope here.
    if version.base < NEW_PROTOCOL_VERSION {
        let whale_stake = parse_ether("150000").unwrap();
        let (validator0, _) = providers[0];
        let whale = delegate_new(
            &network_config,
            token,
            net.stake_table,
            validator0,
            whale_stake,
        )
        .await?;

        // min stake = max stake / 1000 = ~150.1 ESP > 100 ESP, so every other
        // validator is displaced.
        let (whale_epoch, committee) = wait_for_committee(
            &net.client,
            &mut events,
            EPOCH_HEIGHT,
            return_epoch + 1,
            MAX_ACTIVATION_EPOCHS,
            committee_is(HashSet::from([validator0])),
        )
        .await;
        tracing::info!(
            whale_epoch,
            "whale delegation displaced all other validators"
        );
        assert_eq!(
            committee[&validator0].stake,
            whale_stake + stake,
            "whale-backed validator should hold its own and the whale's stake"
        );

        let receipt = StakingTransaction::Undelegate {
            stake_table: net.stake_table,
            validator: validator0,
            amount: whale_stake,
        }
        .send(&whale)
        .await?
        .get_receipt()
        .await?;
        anyhow::ensure!(receipt.status(), "whale undelegation reverted");
        wait_for_committee(
            &net.client,
            &mut events,
            EPOCH_HEIGHT,
            whale_epoch + 1,
            MAX_ACTIVATION_EPOCHS,
            committee_is(all_addrs),
        )
        .await;
        assert_node_live(&net.network.server, EPOCH_HEIGHT, 1).await;
    }

    Ok(())
}

/// Full set replacement across the 0.4 -> 0.5 (epoch reward) upgrade: the
/// network starts at 0.4 with committee {3,4,5}, upgrades to 0.5 mid-run,
/// and the committee is replaced wholesale by {0,1,2}. Depending on the
/// case, the swap is sent after the upgrade has activated or right at the
/// `UpgradeProposal`, so its activation lands inside the upgrade window
/// while the certificate is still pending.
///
/// Besides committee and liveness checks, this asserts that per-epoch reward
/// distribution (new at 0.5) works under the replaced committee: the total
/// distributed rewards strictly increase across post-swap epoch boundaries.
#[rstest]
#[case::after_upgrade(SwapTrigger::AfterUpgrade)]
#[case::straddling(SwapTrigger::AtUpgradeProposal)]
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_full_swap_across_epoch_reward_upgrade(
    #[case] trigger: SwapTrigger,
) -> anyhow::Result<()> {
    const NUM_NODES: usize = 6;
    const EPOCH_HEIGHT: u64 = 20;
    const UPGRADE_START_PROPOSING_VIEW: u64 = 65;
    let outgoing = [3, 4, 5];
    let incoming = [0, 1, 2];
    let upgrade = Upgrade::new(DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_REWARD_VERSION);

    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .set_upgrades_with(
            EPOCH_REWARD_VERSION,
            StakeTableContractVersion::V3,
            &outgoing,
        )
        .await
        .upgrade_proposing_views(UPGRADE_START_PROPOSING_VIEW, 1000)
        .build();

    let net = StakeTableTestNetwork::start_upgrading(network_config.clone(), upgrade).await;
    let incoming_addrs = staking_addresses(&network_config, &incoming);
    let mut events = net.network.server.event_stream();

    let swap = || async {
        register_validators(
            &network_config,
            net.stake_table,
            &incoming,
            DelegationConfig::MultipleDelegators,
        )
        .await?;
        deregister_validators(&network_config, net.stake_table, &outgoing).await?;
        anyhow::Ok(())
    };

    match trigger {
        SwapTrigger::AtUpgradeProposal => {
            wait_for_upgrade_proposal(&net.network.server, EPOCH_REWARD_VERSION).await;
            swap().await?;
            net.wait_for_version(EPOCH_REWARD_VERSION, Duration::from_secs(600))
                .await;
        },
        SwapTrigger::AfterUpgrade => {
            net.wait_for_version(EPOCH_REWARD_VERSION, Duration::from_secs(600))
                .await;
            swap().await?;
        },
        SwapTrigger::BeforeUpgrade => unreachable!("not a case of this test"),
    }

    let current_epoch = net
        .network
        .server
        .decided_leaf()
        .await
        .epoch(EPOCH_HEIGHT)
        .expect("epochs active")
        .u64();
    let (activation_epoch, _) = wait_for_committee(
        &net.client,
        &mut events,
        EPOCH_HEIGHT,
        current_epoch + 1,
        MAX_ACTIVATION_EPOCHS,
        committee_is(incoming_addrs),
    )
    .await;
    tracing::info!(activation_epoch, "full swap activated across the upgrade");

    // The post-upgrade, post-swap network must keep deciding and sequencing.
    assert_node_live(&net.network.server, EPOCH_HEIGHT, 2).await;
    assert_eq!(
        net.network
            .server
            .decided_leaf()
            .await
            .block_header()
            .version(),
        EPOCH_REWARD_VERSION
    );

    // Per-epoch rewards keep flowing under the replaced committee: totals at
    // consecutive post-swap epoch-final blocks strictly increase.
    let first = net
        .header_at((activation_epoch + 1) * EPOCH_HEIGHT)
        .await
        .total_reward_distributed()
        .expect("v5 headers carry the total distributed reward");
    let second = net
        .header_at((activation_epoch + 2) * EPOCH_HEIGHT)
        .await
        .total_reward_distributed()
        .expect("v5 headers carry the total distributed reward");
    assert!(
        first.0 > U256::ZERO,
        "no rewards distributed by the end of the first full post-swap epoch"
    );
    assert!(
        second.0 > first.0,
        "rewards stopped accruing under the replaced committee"
    );

    let incoming_nodes: Vec<_> = incoming.iter().map(|&i| net.network.node(i)).collect();
    assert_nodes_agree(&incoming_nodes, activation_epoch * EPOCH_HEIGHT).await;

    Ok(())
}

/// A single validator is deregistered right when the 0.4 -> 0.5 upgrade is
/// proposed, so the shrunken committee activates at one of the first
/// post-upgrade epoch boundaries — the boundaries whose reward calculations
/// every node performs through leaf-chain catchup, because no leader counts
/// were tracked while the epoch still ran under 0.4. The fetched chains
/// cross the shrink boundary, so each QC must be verified against its own
/// epoch's stake table (#4740): against a single stake table the
/// verification jams on the size mismatch and the network halts at the
/// boundary.
///
/// Most nodes are sequencer-only — node 0, the query node, never joins the
/// committee — mirroring the production topology where non-validators hit
/// this catchup path despite having been online the whole time.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_single_removal_across_epoch_reward_upgrade() -> anyhow::Result<()> {
    const NUM_NODES: usize = 8;
    const EPOCH_HEIGHT: u64 = 20;
    const UPGRADE_START_PROPOSING_VIEW: u64 = 65;
    // 3 of 8 nodes validate; the other 5, including the query node, follow
    // as sequencer-only.
    let registered = [5, 6, 7];
    let removed = 7;
    let upgrade = Upgrade::new(DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_REWARD_VERSION);

    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .set_upgrades_with(
            EPOCH_REWARD_VERSION,
            StakeTableContractVersion::V3,
            &registered,
        )
        .await
        .upgrade_proposing_views(UPGRADE_START_PROPOSING_VIEW, 1000)
        .build();

    let net = StakeTableTestNetwork::start_upgrading(network_config.clone(), upgrade).await;
    let remaining_addrs = staking_addresses(&network_config, &[5, 6]);
    let mut events = net.network.server.event_stream();

    wait_for_upgrade_proposal(&net.network.server, EPOCH_REWARD_VERSION).await;
    deregister_validators(&network_config, net.stake_table, &[removed]).await?;
    net.wait_for_version(EPOCH_REWARD_VERSION, Duration::from_secs(600))
        .await;

    let current_epoch = net
        .network
        .server
        .decided_leaf()
        .await
        .epoch(EPOCH_HEIGHT)
        .expect("epochs active")
        .u64();
    let (activation_epoch, _) = wait_for_committee(
        &net.client,
        &mut events,
        EPOCH_HEIGHT,
        current_epoch + 1,
        MAX_ACTIVATION_EPOCHS,
        committee_is(remaining_addrs),
    )
    .await;
    tracing::info!(activation_epoch, "removal activated across the upgrade");

    // The sequencer-only query node must keep applying blocks past the
    // shrink: its epoch rewards catchup has to verify the boundary-crossing
    // leaf chains.
    assert_node_live(&net.network.server, EPOCH_HEIGHT, 2).await;

    // Rewards keep flowing under the shrunken committee.
    let first = net
        .header_at((activation_epoch + 1) * EPOCH_HEIGHT)
        .await
        .total_reward_distributed()
        .expect("v5 headers carry the total distributed reward");
    let second = net
        .header_at((activation_epoch + 2) * EPOCH_HEIGHT)
        .await
        .total_reward_distributed()
        .expect("v5 headers carry the total distributed reward");
    assert!(
        first.0 > U256::ZERO,
        "no rewards distributed by the end of the first full post-removal epoch"
    );
    assert!(
        second.0 > first.0,
        "rewards stopped accruing under the shrunken committee"
    );

    let all_nodes: Vec<_> = (0..NUM_NODES).map(|i| net.network.node(i)).collect();
    assert_nodes_agree(&all_nodes, activation_epoch * EPOCH_HEIGHT).await;

    Ok(())
}

/// Full set replacement across the 0.5 -> 0.6 (new protocol / fast finality)
/// upgrade: the network starts at 0.5 with committee {0,1,2} and cuts over
/// to cliquenet-based consensus mid-run while the committee is replaced
/// wholesale by {3,4,5}.
///
/// The `before_cutover` case swaps early, so the cutover itself runs on the
/// freshly replaced committee; the `straddling` case sends the swap at the
/// `UpgradeProposal`, so the new committee activates right at or just after
/// the cutover epoch.
///
/// As in [`full_set_replacement`], node 0 — the query node — is in the
/// outgoing set to serve the incoming cohort's catchup; it stalls at the
/// cutover at the latest, so progress is observed through an incoming
/// node's event stream.
async fn full_swap_across_new_protocol_upgrade(trigger: SwapTrigger) -> anyhow::Result<()> {
    const NUM_NODES: usize = 6;
    const EPOCH_HEIGHT: u64 = 70;
    const UPGRADE_START_PROPOSING_VIEW: u64 = 3 * EPOCH_HEIGHT + 5;
    let outgoing = [0, 1, 2];
    let incoming = [3, 4, 5];
    let upgrade = Upgrade::new(EPOCH_REWARD_VERSION, NEW_PROTOCOL_VERSION);

    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .builder_timeout(Duration::from_millis(500))
        .set_upgrades_with(
            NEW_PROTOCOL_VERSION,
            StakeTableContractVersion::V3,
            &outgoing,
        )
        .await
        .upgrade_proposing_views(UPGRADE_START_PROPOSING_VIEW, 1000)
        .build();

    let net = StakeTableTestNetwork::start_upgrading(network_config.clone(), upgrade).await;
    let incoming_addrs = staking_addresses(&network_config, &incoming);
    let mut events = net.network.node(incoming[0]).event_stream();

    let swap = || async {
        register_validators(
            &network_config,
            net.stake_table,
            &incoming,
            DelegationConfig::MultipleDelegators,
        )
        .await?;
        deregister_validators(&network_config, net.stake_table, &outgoing).await?;
        anyhow::Ok(())
    };

    let pre_cutover_activation = match trigger {
        SwapTrigger::BeforeUpgrade => {
            // Swap while epoch 2 is running; the committee flips to the
            // incoming set well before the upgrade window opens.
            wait_for_epochs(&mut events, EPOCH_HEIGHT, 1).await;
            swap().await?;
            let (activation_epoch, _) = wait_for_committee(
                &net.client,
                &mut events,
                EPOCH_HEIGHT,
                FIRST_CONTRACT_EPOCH,
                MAX_ACTIVATION_EPOCHS,
                committee_is(incoming_addrs.clone()),
            )
            .await;
            tracing::info!(activation_epoch, "swap activated before the cutover");
            Some(activation_epoch)
        },
        SwapTrigger::AtUpgradeProposal => {
            wait_for_upgrade_proposal(&net.network.server, NEW_PROTOCOL_VERSION).await;
            swap().await?;
            None
        },
        SwapTrigger::AfterUpgrade => unreachable!("not a case of this test"),
    };

    let upgrade_height =
        wait_for_version_on_events(&mut events, NEW_PROTOCOL_VERSION, Duration::from_secs(600))
            .await;
    let cutover_epoch = epoch_from_block_number(upgrade_height, EPOCH_HEIGHT);
    tracing::info!(upgrade_height, cutover_epoch, "new protocol enabled");

    let activation_epoch = match pre_cutover_activation {
        Some(activation_epoch) => {
            // The swapped committee must have carried the network through the
            // cutover epoch itself.
            let committee = net.committee(cutover_epoch).await;
            assert_eq!(
                committee.keys().copied().collect::<HashSet<_>>(),
                incoming_addrs,
                "the cutover should run on the fully replaced committee"
            );
            activation_epoch
        },
        None => {
            let (activation_epoch, _) = wait_for_committee(
                &net.client,
                &mut events,
                EPOCH_HEIGHT,
                cutover_epoch,
                MAX_ACTIVATION_EPOCHS,
                committee_is(incoming_addrs.clone()),
            )
            .await;
            tracing::info!(activation_epoch, "swap activated around the cutover");
            activation_epoch
        },
    };

    // Every member of the new committee must be dialable via cliquenet.
    let committee = net.committee(activation_epoch.max(cutover_epoch)).await;
    for (address, validator) in &committee {
        assert!(
            validator.x25519_key.is_some() && validator.p2p_addr.is_some(),
            "incoming validator {address} is missing cliquenet connect info"
        );
    }

    // The new protocol on the replaced committee must keep crossing epoch
    // boundaries and sequencing transactions.
    assert_node_live(net.network.node(incoming[0]), EPOCH_HEIGHT, 2).await;
    let incoming_nodes: Vec<_> = incoming.iter().map(|&i| net.network.node(i)).collect();
    assert_nodes_agree(
        &incoming_nodes,
        activation_epoch.max(cutover_epoch) * EPOCH_HEIGHT,
    )
    .await;

    Ok(())
}

/// The swap completes under 0.5 legacy consensus, then the cutover runs on
/// the freshly replaced committee: see
/// [`full_swap_across_new_protocol_upgrade`].
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_full_swap_before_new_protocol_cutover() -> anyhow::Result<()> {
    full_swap_across_new_protocol_upgrade(SwapTrigger::BeforeUpgrade).await
}

/// The swap is sent at the `UpgradeProposal`, so it activates right after
/// the cutover — a 0.6-native handoff to a committee that never held the
/// previous epoch's payloads, carried by the boundary-state seeding (see
/// `test_stake_table_full_set_replacement_v6`).
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stake_table_full_swap_straddles_new_protocol_cutover() -> anyhow::Result<()> {
    full_swap_across_new_protocol_upgrade(SwapTrigger::AtUpgradeProposal).await
}

/// The liveness hazard of the cliquenet upgrade
/// (`AuthenticatedValidator::is_eligible`): validators without on-chain
/// connect info are still members of the committees selected before 0.6
/// activates — silently skipped by cliquenet but counted toward the quorum.
/// Here 4 of 5 validators publish their network config before the cutover
/// (80% of stake >= 2/3, so the chain stays live) and the fifth never does:
/// it must remain a member through the transition window, then drop out at
/// the first epoch whose root is a 0.6 header.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_new_protocol_upgrade_ineligible_validator_drops() -> anyhow::Result<()> {
    const NUM_NODES: usize = 5;
    const EPOCH_HEIGHT: u64 = 70;
    const UPGRADE_START_PROPOSING_VIEW: u64 = 3 * EPOCH_HEIGHT + 5;
    let upgrade = Upgrade::new(EPOCH_REWARD_VERSION, NEW_PROTOCOL_VERSION);
    let all: Vec<usize> = (0..NUM_NODES).collect();

    // Register everyone on StakeTable V2: no x25519 keys or p2p addresses on
    // chain, and equal stakes so the eligible fraction is exactly 4/5.
    let network_config = TestConfigBuilder::<NUM_NODES>::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .builder_timeout(Duration::from_millis(500))
        .set_upgrades_with(NEW_PROTOCOL_VERSION, StakeTableContractVersion::V2, &all)
        .await
        .upgrade_proposing_views(UPGRADE_START_PROPOSING_VIEW, 1000)
        .build();

    let net = StakeTableTestNetwork::start_upgrading(network_config.clone(), upgrade).await;
    let mut events = net.network.server.event_stream();
    wait_for_epochs(&mut events, EPOCH_HEIGHT, 1).await;

    // Upgrade the contract to V3 mid-run and publish connect info for all
    // validators except the last, well before any 0.6 epoch root.
    let deployer = ProviderBuilder::new()
        .wallet(EthereumWallet::from(network_config.signer()))
        .connect_http(network_config.l1_url());
    let mut contracts = network_config
        .contracts()
        .expect("set_upgrades_with deploys the contracts");
    upgrade_stake_table_v3(&deployer, &mut contracts)
        .await
        .expect("stake table upgrade to V3");

    let keys = network_config.staking_priv_keys();
    let providers = network_config.validator_providers();
    for i in 0..NUM_NODES - 1 {
        let receipt = update_network_config(
            &providers[i].1,
            net.stake_table,
            keys[i].x25519.public_key(),
            keys[i].p2p_addr.clone(),
        )
        .await?
        .get_receipt()
        .await?;
        anyhow::ensure!(
            receipt.status(),
            "network config update of validator {i} reverted"
        );
    }

    let upgrade_height = net
        .wait_for_version(NEW_PROTOCOL_VERSION, Duration::from_secs(600))
        .await;
    let cutover_epoch = epoch_from_block_number(upgrade_height, EPOCH_HEIGHT);
    tracing::info!(upgrade_height, cutover_epoch, "new protocol enabled");

    // The cutover epoch's committee was selected under 0.5 rules: all five
    // validators are members, the last one without connect info.
    let committee = net.committee(cutover_epoch).await;
    assert_eq!(
        committee.len(),
        NUM_NODES,
        "pre-cutover selection must not apply the eligibility filter"
    );
    let ineligible = providers[NUM_NODES - 1].0;
    assert!(
        committee[&ineligible].x25519_key.is_none(),
        "the last validator should still have no connect info on chain"
    );

    // From the first epoch whose root is a 0.6 header, the eligibility
    // filter drops it; the chain must stay live throughout.
    let eligible_addrs = staking_addresses(&network_config, &all[..NUM_NODES - 1]);
    let (drop_epoch, _) = wait_for_committee(
        &net.client,
        &mut events,
        EPOCH_HEIGHT,
        cutover_epoch + 1,
        MAX_ACTIVATION_EPOCHS,
        committee_is(eligible_addrs),
    )
    .await;
    tracing::info!(
        drop_epoch,
        "ineligible validator dropped from the committee"
    );
    assert!(
        drop_epoch <= cutover_epoch + 3,
        "the eligibility filter should apply within an epoch of the first 0.6 root"
    );

    assert_node_live(&net.network.server, EPOCH_HEIGHT, 2).await;
    let eligible_nodes: Vec<_> = (0..NUM_NODES - 1).map(|i| net.network.node(i)).collect();
    assert_nodes_agree(&eligible_nodes, drop_epoch * EPOCH_HEIGHT).await;

    Ok(())
}
