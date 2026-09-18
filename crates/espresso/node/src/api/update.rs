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
    types::HeightIndexed,
};
use hotshot_types::new_protocol::CoordinatorEvent;

use super::{StorageState, context::ApiContext, data_source::SequencerDataSource};
use crate::{EventConsumer, SeqTypes};

/// Where a node hands the blocks it decides to the query API.
#[async_trait]
pub trait DecideSink: Send + Sync + Debug {
    async fn append(&self, info: BlockInfo<SeqTypes>) -> anyhow::Result<()>;
}

pub trait ApiSink: EventConsumer + DecideSink {}

impl<T: EventConsumer + DecideSink> ApiSink for T {}

/// Without a query module there is nowhere to store decided blocks, so a node that hands them
/// here fails on the first one instead of reporting progress it never persisted.
#[async_trait]
impl DecideSink for NullEventConsumer {
    async fn append(&self, info: BlockInfo<SeqTypes>) -> anyhow::Result<()> {
        bail!(
            "cannot store block {}: the API has no query storage",
            info.height()
        )
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
}
