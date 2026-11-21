//! Repair: an API instance that repairs fills what the store lacks from other API services.
//!
//! The [`GapFiller`] fills final gaps and checks the certificates of filled heights, and the
//! [`ChainFollower`] writes the state of the recent heights consensus sent none for. Membership,
//! derived from L1, serves both. The chain itself, with the state, stake tables and discovery
//! records consensus derives, reaches the store through the storage instances; while consensus is
//! up and the store misses nothing, there is nothing to repair.
//!
//! Any number of instances can repair one store. They share the work through leases in the
//! store: an instance fills a gap, or checks the certificates of an epoch, only while it holds
//! its lease, and one instance at a time follows the state. A lease whose holder stops runs out,
//! and another instance takes the work over.

use std::time::Duration;

use anyhow::{Context, anyhow};
use chain_api::Tables;
use chain_store::ChainStore;
use espresso_genesis::Genesis;
use espresso_types::NodeState;
use tokio::{task::JoinSet, time::sleep};
use url::Url;

mod certs;
mod gaps;
mod membership;
mod peers;
mod state;
mod state_cert;
mod store_catchup;

pub use gaps::GapFiller;
pub use membership::{Membership, node_state};
pub use peers::Peers;
pub use state::ChainFollower;

/// The lease on following the state.
const STATE_LEASE: &str = "repair/state";

/// How long the lease on following the state lasts, and how often its holder renews it.
const STATE_LEASE_TTL: Duration = Duration::from_secs(30);
const STATE_LEASE_RENEWAL: Duration = Duration::from_secs(10);

/// How long to wait before following the state again after the follower failed.
const FOLLOWER_RETRY: Duration = Duration::from_secs(60);

/// Where repair finds what it needs besides the store.
#[derive(Clone, Debug)]
pub struct Repair {
    /// L1 RPC URLs to derive stake tables from.
    pub l1: Vec<Url>,

    /// API services to learn the validators' API services from, and to repair from until then.
    pub bootstrap: Vec<Url>,

    /// The URL other services reach this deployment's API at, which is never a peer.
    pub public_url: Option<Url>,
}

impl Repair {
    /// Repairs `store`, of the chain of `genesis`, as long as its parts run.
    pub async fn run(self, store: ChainStore, genesis: &Genesis) -> anyhow::Error {
        match self.start(store, genesis).await {
            Ok(mut tasks) => match tasks.join_next().await {
                Some(stopped) => {
                    stopped.unwrap_or_else(|err| anyhow!("a repair task failed: {err}"))
                },
                None => anyhow!("no repair task to run"),
            },
            Err(err) => err.context("starting repair"),
        }
    }

    async fn start(
        self,
        store: ChainStore,
        genesis: &Genesis,
    ) -> anyhow::Result<JoinSet<anyhow::Error>> {
        let state = node_state(genesis, self.l1.clone(), store.clone())
            .await
            .context("building the node state")?;
        let membership = Membership::new(&state)?;
        let holder = format!("{:016x}", rand::random::<u64>());
        tracing::info!(%holder, "repairing the store");
        let filler = GapFiller::new(store.clone(), self.peers(&store), Some(membership))
            .with_leases(holder.clone());
        let mut tasks = JoinSet::new();
        tasks.spawn(async move {
            filler.run().await;
            anyhow!("the gap filler stopped")
        });
        tasks.spawn(self.follow(store, state, holder));
        Ok(tasks)
    }

    fn peers(&self, store: &ChainStore) -> Peers {
        Peers::new(
            Tables::new(store.clone()),
            self.bootstrap.clone(),
            self.public_url.clone(),
        )
    }

    /// Follows the state whenever this instance holds the lease on it. A failing follower holds
    /// up only the state of heights consensus sent none for, so it is started again rather than
    /// stopping repair.
    async fn follow(self, store: ChainStore, state: NodeState, holder: String) -> anyhow::Error {
        loop {
            match store.lease(STATE_LEASE, &holder, STATE_LEASE_TTL).await {
                Ok(true) => {},
                Ok(false) => {
                    sleep(STATE_LEASE_RENEWAL).await;
                    continue;
                },
                Err(err) => {
                    tracing::warn!("taking the lease on the state failed: {err:#}");
                    sleep(STATE_LEASE_RENEWAL).await;
                    continue;
                },
            }
            tracing::info!("following the state");
            let follower = ChainFollower::new(store.clone(), state.clone(), self.peers(&store));
            tokio::select! {
                followed = follower.run() => match followed {
                    Ok(()) => return anyhow!("the chain follower stopped"),
                    Err(err) => {
                        tracing::error!("following the chain failed: {err:#}");
                        sleep(FOLLOWER_RETRY).await;
                    },
                },
                () = keep_lease(&store, &holder) => {
                    tracing::warn!("lost the lease on the state; another instance follows it");
                },
            }
        }
    }
}

/// Renews the lease on the state until it is lost, or cannot be renewed.
async fn keep_lease(store: &ChainStore, holder: &str) {
    loop {
        sleep(STATE_LEASE_RENEWAL).await;
        match store.lease(STATE_LEASE, holder, STATE_LEASE_TTL).await {
            Ok(true) => {},
            Ok(false) => return,
            Err(err) => {
                tracing::warn!("renewing the lease on the state failed: {err:#}");
                return;
            },
        }
    }
}
