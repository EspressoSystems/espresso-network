//! Update loop for query API state.

use std::{fmt::Debug, sync::Arc};

use anyhow::bail;
use async_trait::async_trait;
use derivative::Derivative;
use derive_more::From;
use espresso_types::v0::traits::NullEventConsumer;
use hotshot_query_service::{
    availability::{BlockInfo, UpdateAvailabilityData},
    data_source::UpdateDataSource,
    node::NodeDataSource,
    types::HeightIndexed,
};
use hotshot_types::{
    new_protocol::CoordinatorEvent, simple_certificate::LightClientStateUpdateCertificateV2,
};

use super::{
    StorageState,
    data_source::{SequencerDataSource, StateCertDataSource},
};
use crate::{EventConsumer, SeqTypes};

#[async_trait]
pub trait DecideSink: Send + Sync + Debug {
    /// Stores a decided block, with the state certificate of the epoch it is the root of.
    async fn append(
        &self,
        info: BlockInfo<SeqTypes>,
        state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
    ) -> anyhow::Result<()>;

    /// The first height a resuming node has to append.
    async fn block_height(&self) -> anyhow::Result<u64>;
}

#[async_trait]
impl<T> DecideSink for Box<T>
where
    T: DecideSink + ?Sized,
{
    async fn append(
        &self,
        info: BlockInfo<SeqTypes>,
        state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
    ) -> anyhow::Result<()> {
        (**self).append(info, state_cert).await
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        (**self).block_height().await
    }
}

pub trait ApiSink: EventConsumer + DecideSink {}

impl<T: EventConsumer + DecideSink> ApiSink for T {}

/// Without a query module there is nowhere to store decided blocks, so a node that hands them
/// here fails on the first one instead of reporting progress it never persisted.
#[async_trait]
impl DecideSink for NullEventConsumer {
    async fn append(
        &self,
        info: BlockInfo<SeqTypes>,
        _: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!(
            "cannot store block {}: the API has no query storage",
            info.height()
        )
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        Ok(0)
    }
}

#[derive(Derivative, From)]
#[derivative(Clone(bound = ""), Debug(bound = "D: Debug"))]
pub(crate) struct ApiEventConsumer<D> {
    inner: Arc<StorageState<D>>,
}

#[async_trait]
impl<D> EventConsumer for ApiEventConsumer<D>
where
    D: SequencerDataSource + Debug + Send + Sync + 'static,
{
    async fn handle_event(&self, event: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()> {
        // The ingester stores decided blocks, from the consensus store.
        if matches!(event, CoordinatorEvent::NewDecide { .. }) {
            return Ok(());
        }
        if let Err(height) = self.inner.update(event).await {
            bail!("failed to update API state after {height}: {event:?}",);
        }
        Ok(())
    }
}

#[async_trait]
impl<D> DecideSink for ApiEventConsumer<D>
where
    D: SequencerDataSource + UpdateAvailabilityData<SeqTypes> + Debug + Send + Sync + 'static,
{
    /// Stores the state certificate before the block, so that a block this node holds always
    /// has its certificate; storing either again is harmless.
    async fn append(
        &self,
        info: BlockInfo<SeqTypes>,
        state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
    ) -> anyhow::Result<()> {
        if let Some(cert) = state_cert {
            self.inner.insert_state_cert(*cert.epoch, cert).await?;
        }
        self.inner.append(info).await
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        Ok(NodeDataSource::block_height(&*self.inner).await? as u64)
    }
}

/// A [`DecideSink`] as the ingester writes to it.
pub(crate) struct IngestSink(pub Arc<dyn DecideSink>);

#[async_trait]
impl hotshot_query_service::ingest::Sink<SeqTypes> for IngestSink {
    async fn append(
        &self,
        info: BlockInfo<SeqTypes>,
        state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
    ) -> anyhow::Result<()> {
        self.0.append(info, state_cert).await
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        self.0.block_height().await
    }
}
