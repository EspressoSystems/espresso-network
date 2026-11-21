//! The decide log as the ingester API serves it.

use std::{ops::Range, sync::Arc};

use async_trait::async_trait;
use committable::Committable;
use consensus_api::{
    BODY_FORMAT, Error, Source,
    proto::{Body, Decided as DecidedEntry, State},
};
use consensus_store::{Bytes, Store};
use hotshot_types::{data::EpochNumber, traits::node_implementation::NodeType};
use tokio::sync::watch;

use crate::store::{ConsensusStore, Decided};

/// A [`Source`] over a node's consensus store.
pub struct IngestSource<T, S> {
    store: ConsensusStore<T, S>,
    epoch_height: u64,
    stake_tables: Arc<dyn StakeTables>,
}

/// The stake tables the node has built, which the application encodes.
#[async_trait]
pub trait StakeTables: Send + Sync + 'static {
    /// The encoded stake table of `epoch`, if the node has built it.
    async fn stake_table(&self, epoch: EpochNumber) -> anyhow::Result<Option<Bytes>>;
}

impl<T: NodeType, S: Store> IngestSource<T, S> {
    /// A source over `store`, whose chain has epochs of `epoch_height` blocks, sending the stake
    /// tables of `stake_tables`.
    pub fn new(
        store: ConsensusStore<T, S>,
        epoch_height: u64,
        stake_tables: Arc<dyn StakeTables>,
    ) -> Arc<Self> {
        Arc::new(Self {
            store,
            epoch_height,
            stake_tables,
        })
    }

    /// Runs `f` on the store on a blocking thread.
    async fn blocking<R: Send + 'static>(
        &self,
        f: impl FnOnce(&ConsensusStore<T, S>) -> anyhow::Result<R> + Send + 'static,
    ) -> Result<R, Error> {
        let store = self.store.clone();
        Ok(tokio::task::spawn_blocking(move || f(&store)).await??)
    }
}

#[async_trait]
impl<T: NodeType, S: Store> Source for IngestSource<T, S> {
    async fn decided(&self, from: u64, limit: usize) -> Result<Vec<DecidedEntry>, Error> {
        let epoch_height = self.epoch_height;
        let entries = self.blocking(move |s| s.decided_from(from, limit)).await?;
        entries
            .into_iter()
            .map(|(decided, size)| entry(decided, size, epoch_height))
            .collect()
    }

    async fn earliest(&self) -> Result<Option<u64>, Error> {
        self.blocking(|s| s.earliest_decided()).await
    }

    fn changes(&self) -> watch::Receiver<u64> {
        self.store.decide_log_writes()
    }

    async fn ack(&self, ranges: Vec<Range<u64>>) -> Result<(), Error> {
        Ok(self.store.ack(&ranges).await?)
    }

    async fn payload(&self, height: u64) -> Result<Option<Bytes>, Error> {
        self.blocking(move |s| s.decided_payload(height)).await
    }

    async fn stake_table(&self, epoch: u64) -> Result<Option<Bytes>, Error> {
        Ok(self
            .stake_tables
            .stake_table(EpochNumber::new(epoch))
            .await?)
    }

    async fn service_records(&self, epoch: u64) -> Result<Vec<Bytes>, Error> {
        let found = self
            .blocking(move |s| s.service_records(Some(EpochNumber::new(epoch))))
            .await?;
        Ok(found
            .filter(|(found, _)| **found == epoch)
            .map(|(_, records)| records)
            .unwrap_or_default())
    }
}

/// A decide log entry in the API's form.
fn entry<T: NodeType>(
    decided: Decided<T>,
    payload_size: Option<u64>,
    epoch_height: u64,
) -> Result<DecidedEntry, Error> {
    let leaf = &decided.leaf;
    let body = Body {
        format: BODY_FORMAT,
        leaf: bincode::serialize(leaf)?.into(),
        cert1: bincode::serialize(&decided.cert1)?.into(),
        cert2: decided
            .cert2
            .as_ref()
            .map(bincode::serialize)
            .transpose()?
            .unwrap_or_default()
            .into(),
        vid_common: decided
            .vid_common
            .as_ref()
            .map(bincode::serialize)
            .transpose()?
            .unwrap_or_default()
            .into(),
        state_cert: decided
            .state_cert
            .as_ref()
            .map(bincode::serialize)
            .transpose()?
            .unwrap_or_default()
            .into(),
        state: decided.state.map(|state| State {
            frontier: state.frontier.into(),
            chain_config: state.chain_config.into(),
            rewards: state.rewards.map(Into::into),
        }),
    };
    Ok(DecidedEntry {
        height: leaf.height(),
        view: *leaf.view_number(),
        epoch: leaf.epoch(epoch_height).map_or(0, |e| *e),
        leaf_commitment: leaf.commit().to_string(),
        parent_commitment: leaf.parent_commitment().to_string(),
        payload_commitment: leaf.payload_commitment().to_string(),
        payload_size,
        body: Some(body),
    })
}
