//! HTTP server application for the staking UI service.

use std::sync::Arc;

use async_lock::RwLock;
use futures::FutureExt;
use tide_disco::{Api, App, api::ApiError};
use vbs::version::{StaticVersion, StaticVersionType};

use crate::{
    error::{Error, Result, ResultExt, ensure},
    input::{
        espresso::{self, EspressoClient, EspressoPersistence},
        l1::{self, L1Persistence, metadata::MetadataFetcher},
    },
};

type Version = StaticVersion<0, 1>;

/// HTTP server state.
#[derive(Clone, Debug)]
pub struct State<L, E> {
    l1: Arc<RwLock<L>>,
    espresso: Arc<RwLock<E>>,
}

impl<L, E> State<L, E> {
    /// Set up an app with the given state.
    pub fn new(l1: Arc<RwLock<L>>, espresso: Arc<RwLock<E>>) -> Self {
        Self { l1, espresso }
    }
}

type AppState<LS, LM, ES, EC> = State<l1::State<LS, LM>, espresso::State<ES, EC>>;

impl<LS, LM, ES, EC> AppState<LS, LM, ES, EC>
where
    LS: L1Persistence + Sync + 'static,
    LM: MetadataFetcher + Send + Sync + 'static,
    ES: EspressoPersistence + Send + Sync + 'static,
    EC: EspressoClient + 'static,
{
    /// Run the app.
    ///
    /// Unless there is some catastrophic error, this future will never resolve. It is best spawned
    /// as a background task, or awaited as the main task of the process.
    pub async fn serve(self, port: u16) -> Result<()> {
        let mut app = App::<_, Error>::with_state(self);

        {
            let mut api = app
                .module::<Error, Version>(
                    "staking",
                    toml::from_str::<toml::Value>(include_str!("../api/api.toml"))
                        .context(Error::internal)?,
                )
                .context(Error::internal)?;
            bind_handlers(&mut api).context(Error::internal)?;
        }

        app.serve(format!("0.0.0.0:{port}"), Version::instance())
            .await
            .context(Error::internal)
    }
}

fn bind_handlers<LS, LM, ES, EC>(
    api: &mut Api<AppState<LS, LM, ES, EC>, Error, Version>,
) -> Result<(), ApiError>
where
    LS: L1Persistence + Sync + 'static,
    LM: MetadataFetcher + Send + Sync + 'static,
    ES: EspressoPersistence + Send + Sync + 'static,
    EC: EspressoClient + 'static,
{
    api.at("l1_block_latest", |_, state| {
        async move { Ok(state.l1.read().await.latest_l1_block()) }.boxed()
    })?
    .at("l1_block", |req, state| {
        async move {
            let number = req.integer_param("number")?;
            state.l1.read().await.l1_block(number)
        }
        .boxed()
    })?
    .at("full_node_set_snapshot", |req, state| {
        async move {
            let hash = req
                .string_param("hash")?
                .parse()
                .context(Error::bad_request)?;
            let (node_set, l1_block) = { state.l1.read().await.full_node_set(hash)? };
            Ok(node_set.into_snapshot(l1_block))
        }
        .boxed()
    })?
    .at("full_node_set_update", |req, state| {
        async move {
            let hash = req
                .string_param("hash")?
                .parse()
                .context(Error::bad_request)?;
            state.l1.read().await.full_node_set_update(hash)
        }
        .boxed()
    })?
    .at("wallet_snapshot", |req, state| {
        async move {
            let address = req
                .string_param("address")?
                .parse()
                .context(Error::bad_request)?;
            let hash = req
                .string_param("hash")?
                .parse()
                .context(Error::bad_request)?;
            let (wallet, l1_block) = { state.l1.read().await.wallet(address, hash)? };
            Ok(wallet.into_snapshot(l1_block))
        }
        .boxed()
    })?
    .at("wallet_update", |req, state| {
        async move {
            let address = req
                .string_param("address")?
                .parse()
                .context(Error::bad_request)?;
            let hash = req
                .string_param("hash")?
                .parse()
                .context(Error::bad_request)?;
            state.l1.read().await.wallet_update(address, hash)
        }
        .boxed()
    })?
    .at("active_node_set_snapshot", |_, state| {
        async move { state.espresso.read().await.active_node_set().await }.boxed()
    })?
    .at("active_node_set_snapshot_indexed", |req, state| {
        async move {
            let block: u64 = req.integer_param("block")?;
            let espresso = state.espresso.read().await;
            let latest = espresso.latest_espresso_block()?;
            if block < latest {
                return Err(Error::gone().context(format!(
                    "request block {block} is too old (earliest available is {latest})"
                )));
            } else if block > latest {
                return Err(Error::not_found().context(format!(
                    "request block {block} is not yet available (latest available is {latest})"
                )));
            }

            let snapshot = espresso.active_node_set().await?;
            // Sanity check that we got the right snapshot (since we hold a read lock on the
            // Espresso state between checking the block number and getting the snapshot, this
            // should always be true).
            ensure!(
                snapshot.espresso_block.block == block,
                Error::internal().context(format!(
                    "internal inconsistency: snapshot returned for block {} is not for latest block {block}",
                    snapshot.espresso_block.block
                ))
            );
            Ok(snapshot)
        }
        .boxed()
    })?
    .at("active_node_set_update", |req, state| {
        async move {
            let block = req.integer_param("block")?;
            state.espresso.read().await.active_node_set_update(block)
        }
        .boxed()
    })?
    .at("wallet_rewards", |req, state| {
        async move {
            let address = req
                .string_param("address")?
                .parse()
                .context(Error::bad_request)?;
            let block: u64 = req.integer_param("block")?;
            state.espresso.read().await.lifetime_rewards(address, block).await
        }
        .boxed()
    })?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::{
        input::l1::testing::{MemoryStorage, NoMetadata},
        types::wallet::{WalletSnapshot, WalletUpdate},
    };
    use alloy::primitives::Address;
    use futures::future::try_join;
    use portpicker::pick_unused_port;
    use pretty_assertions::assert_eq;
    use surf_disco::Client;
    use tide_disco::{Error as _, StatusCode};
    use tokio::{task::spawn, time::sleep};

    use crate::types::{common::U256, wallet::WalletDiff};
    use hotshot_contract_adapter::sol_types::StakeTableV2::{
        Delegated, StakeTableV2Events, Undelegated,
    };

    use crate::{
        input::{
            espresso::testing::{MemoryStorage as EspressoStorage, MockEspressoClient},
            l1::{
                BlockInput, Snapshot,
                testing::{
                    MemoryStorage as L1Storage, NoCatchup, VecStream, block_id, block_snapshot,
                    subscribe_until, validator_registered_event,
                },
            },
        },
        types::{
            common::{L1BlockId, NodeSetEntry},
            global::{
                ActiveNodeSetDiff, ActiveNodeSetSnapshot, ActiveNodeSetUpdate, FullNodeSetDiff,
                FullNodeSetSnapshot, FullNodeSetUpdate,
            },
        },
    };

    use super::*;

    /// Generate a trivial Espresso state.
    ///
    /// This is useful for testing L1-based APIs, which don't depend on Espresso at all.
    async fn empty_espresso_state() -> espresso::State<EspressoStorage, MockEspressoClient> {
        let mut client = MockEspressoClient::new(1).await;
        // We need just 1 block, to start the epoch.
        client.push_leaf(0, [true]).await;

        let storage = EspressoStorage::default();
        espresso::State::new(storage, client).await.unwrap()
    }

    /// Generate a trivial L1 state.
    ///
    /// This is useful for testing Espresso-based APIs, which don't depend on the L1 at all.
    async fn empty_l1_state() -> l1::State<L1Storage, NoMetadata> {
        l1::State::new(
            Default::default(),
            NoMetadata,
            Snapshot::empty(block_snapshot(0)),
            &NoCatchup,
        )
        .await
        .unwrap()
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_l1_endpoints() {
        let port = pick_unused_port().unwrap();
        let url = format!("http://localhost:{port}").parse().unwrap();

        let l1 = l1::State::<L1Storage, NoMetadata>::with_l1_block_range(1, 3);
        let espresso = empty_espresso_state().await;
        let state = State::new(Arc::new(RwLock::new(l1)), Arc::new(RwLock::new(espresso)));
        let task = spawn(state.serve(port));

        tracing::info!("waiting for service to become available");
        sleep(Duration::from_secs(1)).await;
        let client = Client::<Error, Version>::new(url);
        client.connect(None).await;

        // Get latest block.
        tracing::info!("test latest block");
        let block: L1BlockId = client.get("staking/l1/block/latest").send().await.unwrap();
        assert_eq!(block, block_id(2));

        // Get blocks by number.
        for number in 1..3 {
            tracing::info!(number, "test block by number");
            let block: L1BlockId = client
                .get(&format!("staking/l1/block/{number}"))
                .send()
                .await
                .unwrap();
            assert_eq!(block, block_id(number));
        }

        // Query for old block.
        tracing::info!("test old block");
        let err = client
            .get::<L1BlockId>("staking/l1/block/0")
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        // Query for future block.
        tracing::info!("test future block");
        let err = client
            .get::<L1BlockId>("staking/l1/block/3")
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        task.abort();
        let _ = task.await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_full_node_set_endpoints() {
        let port = pick_unused_port().unwrap();
        let url = format!("http://localhost:{port}").parse().unwrap();

        // Start with an empty state.
        let l1 = Arc::new(RwLock::new(
            l1::State::new(
                L1Storage::default(),
                NoMetadata,
                Snapshot::empty(block_snapshot(1)),
                &NoCatchup,
            )
            .await
            .unwrap(),
        ));

        // Register a node so that we have some non-empty state and updates.
        let mut node = validator_registered_event(rand::thread_rng());
        node.metadataUri = "".into();
        let node_entry = NodeSetEntry::from_event_no_metadata(&node);
        let mut inputs = VecStream::infinite();
        inputs.push(
            BlockInput::empty(2)
                .with_event(StakeTableV2Events::ValidatorRegisteredV2(node.clone())),
        );
        subscribe_until(&l1, inputs, |l1| l1.latest_l1_block().number == 2).await;

        let state = State::new(l1, Arc::new(RwLock::new(empty_espresso_state().await)));
        let task = spawn(state.serve(port));

        tracing::info!("waiting for service to become available");
        sleep(Duration::from_secs(1)).await;
        let client = Client::<Error, Version>::new(url);
        client.connect(None).await;

        tracing::info!("genesis snapshot should be empty");
        let snapshot: FullNodeSetSnapshot = client
            .get(&format!("staking/nodes/all/{:x}", block_id(1).hash))
            .send()
            .await
            .unwrap();
        assert_eq!(snapshot.l1_block, block_snapshot(1).info());
        assert!(snapshot.nodes.is_empty());

        tracing::info!("updates should be unavailable for genesis state");
        let err = client
            .get::<FullNodeSetUpdate>(&format!("staking/nodes/all/updates/{:x}", block_id(1).hash))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        tracing::info!("next snapshot should contain the registered validator");
        let snapshot: FullNodeSetSnapshot = client
            .get(&format!("staking/nodes/all/{:x}", block_id(2).hash))
            .send()
            .await
            .unwrap();
        assert_eq!(snapshot.l1_block, block_snapshot(2).info());
        assert_eq!(snapshot.nodes, std::slice::from_ref(&node_entry));

        tracing::info!("next update should contain the registration event");
        let update: FullNodeSetUpdate = client
            .get(&format!("staking/nodes/all/updates/{:x}", block_id(2).hash))
            .send()
            .await
            .unwrap();
        assert_eq!(update.l1_block, block_snapshot(2).info());
        assert_eq!(
            update.diff,
            [FullNodeSetDiff::NodeUpdate(Arc::new(node_entry))]
        );

        tracing::info!("queries at unknown block hash should return 404");
        let err = client
            .get::<FullNodeSetSnapshot>(&format!("stakingnodes/all/{:x}", block_id(100).hash))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        let err = client
            .get::<FullNodeSetUpdate>(&format!(
                "staking/nodes/all/updates/{:x}",
                block_id(100).hash
            ))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        task.abort();
        let _ = task.await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_wallet_endpoints() {
        let port = pick_unused_port().unwrap();
        let url = format!("http://localhost:{port}").parse().unwrap();

        let l1 = Arc::new(RwLock::new(
            l1::State::new(
                MemoryStorage::default(),
                NoMetadata,
                Snapshot::empty(block_snapshot(1)),
                &NoCatchup,
            )
            .await
            .unwrap(),
        ));

        let delegator = Address::random();
        let validator = validator_registered_event(rand::thread_rng());
        let validator_address = validator.account;

        let mut inputs = VecStream::infinite();

        inputs.push(
            BlockInput::empty(2)
                .with_event(StakeTableV2Events::ValidatorRegisteredV2(validator.clone()))
                .with_event(StakeTableV2Events::Delegated(Delegated {
                    delegator,
                    validator: validator_address,
                    amount: U256::from(1000),
                })),
        );

        inputs.push(
            BlockInput::empty(3).with_event(StakeTableV2Events::Undelegated(Undelegated {
                delegator,
                validator: validator_address,
                amount: U256::from(400),
            })),
        );

        subscribe_until(&l1, inputs, |l1| l1.latest_l1_block().number == 3).await;

        let state = State::new(
            l1.clone(),
            Arc::new(RwLock::new(empty_espresso_state().await)),
        );
        let task = spawn(state.serve(port));

        let client = Client::<Error, Version>::new(url);
        client.connect(None).await;

        let unknown_address = Address::random();
        let block_2_hash = block_id(2).hash;

        // unknown address
        let err = client
            .get::<WalletSnapshot>(&format!("staking/wallet/{unknown_address}/{block_2_hash}"))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // unknown address
        let err = client
            .get::<WalletUpdate>(&format!(
                "staking/wallet/{unknown_address}/updates/{block_2_hash}"
            ))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // unknown block
        let err = client
            .get::<WalletSnapshot>(&format!(
                "staking/wallet/{delegator}/{}",
                block_id(100).hash
            ))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        let snapshot: WalletSnapshot = client
            .get(&format!("staking/wallet/{delegator}/{block_2_hash}"))
            .send()
            .await
            .unwrap();

        assert_eq!(snapshot.l1_block, block_snapshot(2).info());
        assert_eq!(snapshot.nodes.len(), 1);
        assert_eq!(snapshot.nodes[0].delegator, delegator);
        assert_eq!(snapshot.nodes[0].node, validator_address);
        assert_eq!(snapshot.nodes[0].amount, U256::from(1000));
        assert_eq!(snapshot.pending_undelegations.len(), 0);
        assert_eq!(snapshot.pending_exits.len(), 0);
        assert_eq!(snapshot.claimed_rewards, U256::ZERO);

        // Test update at block 2
        let update: WalletUpdate = client
            .get(&format!(
                "staking/wallet/{delegator}/updates/{block_2_hash}"
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(update.l1_block, block_snapshot(2).info());
        assert_eq!(update.diff.len(), 1);
        assert!(matches!(
            &update.diff[0],
            WalletDiff::DelegatedToNode(d) if d.delegator == delegator
                && d.node == validator_address
                && d.amount == U256::from(1000)
        ));

        tracing::info!("test wallet snapshot after undelegation");
        let block_3_hash = block_id(3).hash;
        let snapshot: WalletSnapshot = client
            .get(&format!("staking/wallet/{delegator}/{block_3_hash}"))
            .send()
            .await
            .unwrap();

        assert_eq!(snapshot.l1_block, block_snapshot(3).info());
        assert_eq!(snapshot.nodes.len(), 1);
        assert_eq!(snapshot.nodes[0].amount, U256::from(600)); // 1000 - 400
        assert_eq!(snapshot.pending_undelegations.len(), 1);
        assert_eq!(snapshot.pending_undelegations[0].delegator, delegator);
        assert_eq!(snapshot.pending_undelegations[0].node, validator_address);
        assert_eq!(snapshot.pending_undelegations[0].amount, U256::from(400));

        tracing::info!("test wallet update for undelegation");
        let update: WalletUpdate = client
            .get(&format!(
                "staking/wallet/{delegator}/updates/{block_3_hash}"
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(update.l1_block, block_snapshot(3).info());
        assert_eq!(update.diff.len(), 1);
        assert!(matches!(
            &update.diff[0],
            WalletDiff::UndelegatedFromNode(w) if w.delegator == delegator
                && w.node == validator_address
                && w.amount == U256::from(400)
        ));

        task.abort();
        let _ = task.await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_wallet_rewards() {
        let port = pick_unused_port().unwrap();
        let url = format!("http://localhost:{port}/v0/staking/")
            .parse()
            .unwrap();

        let mut espresso = MockEspressoClient::new(2).await;

        espresso.push_leaf(0, [true, true]).await;
        espresso.push_leaf(0, [true, true]).await;
        espresso.push_leaf(0, [true, true]).await;

        let last_leaf = espresso.last_leaf().0.clone();
        let epoch = espresso.current_epoch();
        let nodes = espresso.stake_table_for_epoch(epoch).await.unwrap();
        let node_0_address = nodes[0].account;
        let node_1_address = nodes[1].account;

        let espresso_state = espresso::State::new(EspressoStorage::default(), espresso)
            .await
            .unwrap();
        let espresso = Arc::new(RwLock::new(espresso_state));
        let espresso_update_task = espresso::State::update_task(espresso.clone());

        let l1 = empty_l1_state().await;

        let state = State::new(Arc::new(RwLock::new(l1)), espresso.clone());
        let server_task = spawn(state.serve(port));
        let update_task = spawn(espresso_update_task);

        sleep(Duration::from_secs(2)).await;

        let client = Client::<Error, Version>::new(url);
        client.connect(None).await;

        // Query rewards for node 0
        let rewards_0: U256 = client
            .get(&format!(
                "wallet/{node_0_address}/rewards/{}",
                last_leaf.height()
            ))
            .send()
            .await
            .unwrap();

        tracing::info!(?rewards_0, "node 0 rewards");

        // Query rewards for node 1
        let rewards_1: U256 = client
            .get(&format!(
                "wallet/{node_1_address}/rewards/{}",
                last_leaf.height()
            ))
            .send()
            .await
            .unwrap();

        tracing::info!(?rewards_1, "node 1 rewards");

        // Each validator should have earned some rewards
        assert!(rewards_0 > U256::ZERO, "node 0 should have earned rewards");
        assert!(rewards_1 > U256::ZERO, "node 1 should have earned rewards");

        // With 3 blocks produced and 1 ESP per block, total rewards distributed should be 3 ESP
        let total_rewards = rewards_0 + rewards_1;
        let expected_total = U256::from(3_000_000_000_000_000_000u128);
        assert_eq!(
            total_rewards, expected_total,
            "total rewards should equal 3 ESP for 3 blocks"
        );

        let unknown_address = Address::random();
        let unknown_rewards: U256 = client
            .get(&format!(
                "wallet/{unknown_address}/rewards/{}",
                last_leaf.height()
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(
            unknown_rewards,
            U256::ZERO,
            "unknown address should have zero rewards"
        );

        let err = client
            .get::<U256>(&format!(
                "wallet/{node_0_address}/rewards/{}",
                last_leaf.height() + 100
            ))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        server_task.abort();
        update_task.abort();
        let _ = server_task.await;
        let _ = update_task.await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_active_node_endpoints() {
        let port = pick_unused_port().unwrap();
        let url = format!("http://localhost:{port}/v0/staking/")
            .parse()
            .unwrap();

        let mut espresso = MockEspressoClient::new(2).await;
        for _ in 0..10 {
            // Skip a lot of views between each leaf, so that every leader gets charged with some
            // missed slots, which we can observe in the resulting statistics.
            espresso.push_leaf(100, [true, false]).await;
        }
        let last_leaf = espresso.last_leaf().0.clone();
        let epoch = espresso.current_epoch();
        let epoch_height = espresso.epoch_height();
        let nodes = espresso.stake_table_for_epoch(epoch).await.unwrap();

        let espresso = espresso::State::new(EspressoStorage::default(), espresso)
            .await
            .unwrap();
        // Handle the leaves we gave it.
        let espresso = Arc::new(RwLock::new(espresso));
        let update_task = espresso::State::update_task(espresso.clone());

        let l1 = empty_l1_state().await;
        let state = State::new(Arc::new(RwLock::new(l1)), espresso);
        let server_task = state.serve(port);

        let task = spawn(try_join(update_task, server_task));

        tracing::info!("waiting for service to become available");
        sleep(Duration::from_secs(1)).await;
        let client = Client::<Error, Version>::new(url);
        client.connect(None).await;

        // Wait until we process the Espresso blocks.
        let snapshot = loop {
            let snapshot = client
                .get::<ActiveNodeSetSnapshot>("nodes/active")
                .send()
                .await
                .unwrap();
            assert!(snapshot.espresso_block.block <= last_leaf.height());
            assert_eq!(snapshot.nodes.len(), 2);
            assert_eq!(snapshot.nodes[0].address, nodes[0].account);
            assert_eq!(snapshot.nodes[1].address, nodes[1].account);

            if snapshot.espresso_block.block == last_leaf.height() {
                tracing::info!(?snapshot, "got final active node set");
                break snapshot;
            }

            tracing::info!(?snapshot, "waiting for final active node snapshot");
            sleep(Duration::from_secs(1)).await;
        };

        // Sanity check the node statistics.
        assert!(snapshot.nodes[0].proposals > 0);
        assert!(snapshot.nodes[0].proposals < snapshot.nodes[0].slots);
        assert_eq!(snapshot.nodes[0].votes, snapshot.nodes[0].eligible_votes);

        assert!(snapshot.nodes[1].proposals > 0);
        assert!(snapshot.nodes[1].proposals < snapshot.nodes[1].slots);
        assert_eq!(snapshot.nodes[1].votes, 0);

        // Check indexed snapshot endpoint.
        let indexed_snapshot = client
            .get::<ActiveNodeSetSnapshot>(&format!("nodes/active/{}", last_leaf.height()))
            .send()
            .await
            .unwrap();
        assert_eq!(indexed_snapshot, snapshot);

        // Check old and future indexded snapshots.
        let err = client
            .get::<ActiveNodeSetSnapshot>(&format!("nodes/active/{}", last_leaf.height() - 1))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);
        let err = client
            .get::<ActiveNodeSetSnapshot>(&format!("nodes/active/{}", last_leaf.height() + 1))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // We can get the updates for blocks in this epoch.
        for i in 1..=10 {
            let block = (epoch - 1) * epoch_height + i;
            let update = client
                .get::<ActiveNodeSetUpdate>(&format!("nodes/active/updates/{block}"))
                .send()
                .await
                .unwrap();
            assert_eq!(update.espresso_block.block, block);
            assert_eq!(update.espresso_block.epoch, epoch);
            if i == 1 {
                assert_eq!(update.diff.len(), 2);
                assert!(matches!(update.diff[0], ActiveNodeSetDiff::NewEpoch(_)));
                assert!(matches!(update.diff[1], ActiveNodeSetDiff::NewBlock { .. }));
            } else {
                assert_eq!(update.diff.len(), 1);
                assert!(matches!(update.diff[0], ActiveNodeSetDiff::NewBlock { .. }));
            }
        }

        // Updates from the previous epoch are gone.
        let err = client
            .get::<ActiveNodeSetUpdate>(&format!(
                "nodes/active/updates/{}",
                (epoch - 1) * epoch_height
            ))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        // Updates from future blocks are not available.
        let err = client
            .get::<ActiveNodeSetUpdate>(&format!(
                "nodes/active/updates/{}",
                (epoch - 1) * epoch_height + 11
            ))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        task.abort();
        let _ = task.await;
    }
}
