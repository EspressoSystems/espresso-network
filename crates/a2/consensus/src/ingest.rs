//! The decide log as the ingester API serves it.

use std::{ops::Range, sync::Arc};

use async_trait::async_trait;
use committable::Committable;
use consensus_api::{
    BODY_FORMAT, Error, Source,
    proto::{Body, Decided as DecidedEntry},
};
use consensus_store::{Bytes, Store};
use hotshot_types::{data::EpochNumber, traits::node_implementation::NodeType};
use tokio::sync::watch;

use crate::store::{ConsensusStore, Decided};

/// A [`Source`] over a node's consensus store.
pub struct IngestSource<T, S> {
    store: ConsensusStore<T, S>,
    epoch_height: u64,
}

impl<T: NodeType, S: Store> IngestSource<T, S> {
    /// A source over `store`, whose chain has epochs of `epoch_height` blocks.
    pub fn new(store: ConsensusStore<T, S>, epoch_height: u64) -> Arc<Self> {
        Arc::new(Self {
            store,
            epoch_height,
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

    async fn service_records(
        &self,
        epoch: Option<u64>,
    ) -> Result<Option<(u64, Vec<Bytes>)>, Error> {
        let found = self
            .blocking(move |s| s.service_records(epoch.map(EpochNumber::new)))
            .await?;
        Ok(found.map(|(epoch, records)| (*epoch, records)))
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
        leaf: bincode::serialize(leaf)?,
        cert1: bincode::serialize(&decided.cert1)?,
        cert2: decided
            .cert2
            .as_ref()
            .map(bincode::serialize)
            .transpose()?
            .unwrap_or_default(),
        vid_common: decided
            .vid_common
            .as_ref()
            .map(bincode::serialize)
            .transpose()?
            .unwrap_or_default(),
        state_cert: decided
            .state_cert
            .as_ref()
            .map(bincode::serialize)
            .transpose()?
            .unwrap_or_default(),
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
