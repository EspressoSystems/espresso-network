//! HTTP server application for the staking UI service.

use std::sync::Arc;

use async_lock::RwLock;
use futures::FutureExt;
use tide_disco::{Api, App, api::ApiError};
use vbs::version::{StaticVersion, StaticVersionType};

use crate::{
    error::{Error, Result, ResultExt},
    input::l1::{self, L1Persistence},
};

type Version = StaticVersion<0, 1>;

/// HTTP server state.
#[derive(Clone, Debug)]
pub struct State<S> {
    l1: Arc<RwLock<l1::State<S>>>,
}

impl<S> State<S> {
    /// Set up an app with the given state.
    pub fn new(l1: Arc<RwLock<l1::State<S>>>) -> Self {
        Self { l1 }
    }
}

impl<S> State<S>
where
    S: L1Persistence + Sync + 'static,
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
                    "",
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

fn bind_handlers<S>(api: &mut Api<State<S>, Error, Version>) -> Result<(), ApiError>
where
    S: L1Persistence + Sync + 'static,
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
    })?;

    Ok(())
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::{
        input::l1::testing::MemoryStorage,
        types::{
            common::Address,
            wallet::{WalletSnapshot, WalletUpdate},
        },
    };
    use hotshot_contract_adapter::sol_types::StakeTableV2::StakeTableV2Events;
    use portpicker::pick_unused_port;
    use surf_disco::Client;
    use tide_disco::{Error as _, StatusCode};
    use tokio::{task::spawn, time::sleep};

    use crate::types::{common::U256, wallet::WalletDiff};
    use hotshot_contract_adapter::sol_types::StakeTableV2::{Delegated, Undelegated};

    use crate::{
        input::l1::{
            BlockInput, Snapshot,
            testing::{
                NoCatchup, VecStream, block_id, block_snapshot, subscribe_until,
                validator_registered_event,
            },
        },
        types::{
            common::{L1BlockId, NodeSetEntry},
            global::{FullNodeSetDiff, FullNodeSetSnapshot, FullNodeSetUpdate},
        },
    };

    use super::*;

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_l1_endpoints() {
        let port = pick_unused_port().unwrap();
        let url = format!("http://localhost:{port}").parse().unwrap();

        let l1 = l1::State::<MemoryStorage>::with_l1_block_range(1, 3);
        let state = State::new(Arc::new(RwLock::new(l1)));
        let task = spawn(state.serve(port));

        tracing::info!("waiting for service to become available");
        sleep(Duration::from_secs(1)).await;
        let client = Client::<Error, Version>::new(url);
        client.connect(None).await;

        // Get latest block.
        tracing::info!("test latest block");
        let block: L1BlockId = client.get("/l1/block/latest").send().await.unwrap();
        assert_eq!(block, block_id(2));

        // Get blocks by number.
        for number in 1..3 {
            tracing::info!(number, "test block by number");
            let block: L1BlockId = client
                .get(&format!("/l1/block/{number}"))
                .send()
                .await
                .unwrap();
            assert_eq!(block, block_id(number));
        }

        // Query for old block.
        tracing::info!("test old block");
        let err = client
            .get::<L1BlockId>("/l1/block/0")
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        // Query for future block.
        tracing::info!("test future block");
        let err = client
            .get::<L1BlockId>("/l1/block/3")
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
                MemoryStorage::default(),
                Snapshot::empty(block_snapshot(1)),
                &NoCatchup,
            )
            .await
            .unwrap(),
        ));

        // Register a node so that we have some non-empty state and updates.
        let node = validator_registered_event(rand::thread_rng());
        let node_entry = NodeSetEntry::from(&node);
        let mut inputs = VecStream::infinite();
        inputs.push(
            BlockInput::empty(2)
                .with_event(StakeTableV2Events::ValidatorRegisteredV2(node.clone())),
        );
        subscribe_until(&l1, inputs, |l1| l1.latest_l1_block().number == 2).await;

        let state = State::new(l1);
        let task = spawn(state.serve(port));

        tracing::info!("waiting for service to become available");
        sleep(Duration::from_secs(1)).await;
        let client = Client::<Error, Version>::new(url);
        client.connect(None).await;

        tracing::info!("genesis snapshot should be empty");
        let snapshot: FullNodeSetSnapshot = client
            .get(&format!("/nodes/all/{:x}", block_id(1).hash))
            .send()
            .await
            .unwrap();
        assert_eq!(snapshot.l1_block, block_snapshot(1).info());
        assert!(snapshot.nodes.is_empty());

        tracing::info!("updates should be unavailable for genesis state");
        let err = client
            .get::<FullNodeSetUpdate>(&format!("/nodes/all/updates/{:x}", block_id(1).hash))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::GONE);

        tracing::info!("next snapshot should contain the registered validator");
        let snapshot: FullNodeSetSnapshot = client
            .get(&format!("/nodes/all/{:x}", block_id(2).hash))
            .send()
            .await
            .unwrap();
        assert_eq!(snapshot.l1_block, block_snapshot(2).info());
        assert_eq!(snapshot.nodes, std::slice::from_ref(&node_entry));

        tracing::info!("next update should contain the registration event");
        let update: FullNodeSetUpdate = client
            .get(&format!("nodes/all/updates/{:x}", block_id(2).hash))
            .send()
            .await
            .unwrap();
        assert_eq!(update.l1_block, block_snapshot(2).info());
        assert_eq!(update.diff, [FullNodeSetDiff::NodeUpdate(node_entry)]);

        tracing::info!("queries at unknown block hash should return 404");
        let err = client
            .get::<FullNodeSetSnapshot>(&format!("nodes/all/{:x}", block_id(100).hash))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
        let err = client
            .get::<FullNodeSetUpdate>(&format!("nodes/all/updates/{:x}", block_id(100).hash))
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

        let state = State::new(l1.clone());
        let task = spawn(state.serve(port));

        let client = Client::<Error, Version>::new(url);
        client.connect(None).await;

        let unknown_address = Address::random();
        let block_2_hash = block_id(2).hash;

        // unknown address
        let err = client
            .get::<WalletSnapshot>(&format!("/wallet/{unknown_address}/{block_2_hash}"))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // unknown address
        let err = client
            .get::<WalletUpdate>(&format!("/wallet/{unknown_address}/updates/{block_2_hash}"))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // unknown block
        let err = client
            .get::<WalletSnapshot>(&format!("/wallet/{delegator}/{}", block_id(100).hash))
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        let snapshot: WalletSnapshot = client
            .get(&format!("/wallet/{delegator}/{block_2_hash}"))
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
            .get(&format!("/wallet/{delegator}/updates/{block_2_hash}"))
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
            .get(&format!("/wallet/{delegator}/{block_3_hash}"))
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
            .get(&format!("/wallet/{delegator}/updates/{block_3_hash}"))
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
}
