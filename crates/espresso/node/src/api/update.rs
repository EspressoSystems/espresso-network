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
};
use hotshot_types::new_protocol::CoordinatorEvent;

use super::{StorageState, context::ApiContext, data_source::SequencerDataSource};
use crate::{EventConsumer, SeqTypes};

/// Where a node hands the blocks it decides to the query API.
#[async_trait]
pub trait DecideSink: Send + Sync + Debug {
    async fn append(&self, info: BlockInfo<SeqTypes>) -> anyhow::Result<()>;

    /// The number of blocks stored so far: the first height a node resuming has to append.
    async fn block_height(&self) -> anyhow::Result<u64>;
}

#[async_trait]
impl<T> DecideSink for Box<T>
where
    T: DecideSink + ?Sized,
{
    async fn append(&self, info: BlockInfo<SeqTypes>) -> anyhow::Result<()> {
        (**self).append(info).await
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        (**self).block_height().await
    }
}

/// Everything a node feeds the API with: consensus events from a validator, or decided blocks
/// from a node that follows the chain.
pub trait ApiSink: EventConsumer + DecideSink {}

impl<T: EventConsumer + DecideSink> ApiSink for T {}

#[async_trait]
impl DecideSink for NullEventConsumer {
    async fn append(&self, _info: BlockInfo<SeqTypes>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        Ok(0)
    }
}

#[derive(Derivative, From)]
#[derivative(Clone(bound = ""), Debug(bound = "D: Debug"))]
pub(crate) struct ApiEventConsumer<C, D>
where
    C: ApiContext,
{
    inner: Arc<StorageState<C, D>>,
}

#[async_trait]
impl<C, D> EventConsumer for ApiEventConsumer<C, D>
where
    C: ApiContext,
    D: SequencerDataSource + Debug + Send + Sync + 'static,
{
    async fn handle_event(&self, event: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()> {
        if let Err(height) = self.inner.update(event).await {
            bail!("failed to update API state after {height}: {event:?}",);
        }
        Ok(())
    }
}

#[async_trait]
impl<C, D> DecideSink for ApiEventConsumer<C, D>
where
    C: ApiContext,
    D: SequencerDataSource + UpdateAvailabilityData<SeqTypes> + Debug + Send + Sync + 'static,
{
    async fn append(&self, info: BlockInfo<SeqTypes>) -> anyhow::Result<()> {
        self.inner.append(info).await
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        Ok(NodeDataSource::block_height(&*self.inner).await? as u64)
    }
}
