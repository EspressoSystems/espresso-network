//! Update loop for query API state.

use std::{fmt::Debug, sync::Arc};

use anyhow::bail;
use async_trait::async_trait;
use derivative::Derivative;
use derive_more::From;
use espresso_types::{CoordinatorEvent, PubKey, v0::traits::SequencerPersistence};
use hotshot_query_service::data_source::UpdateDataSource;
use hotshot_types::traits::network::ConnectedNetwork;

use super::{StorageState, data_source::SequencerDataSource};
use crate::{EventConsumer, SeqTypes};

#[derive(Derivative, From)]
#[derivative(Clone(bound = ""), Debug(bound = "D: Debug"))]
pub(crate) struct ApiEventConsumer<N, P, D>
where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
{
    inner: Arc<StorageState<N, P, D>>,
}

#[async_trait]
impl<N, P, D> EventConsumer for ApiEventConsumer<N, P, D>
where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
    D: SequencerDataSource + Debug + Send + Sync + 'static,
{
    async fn handle_event(&self, event: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()> {
        if let Err(height) = self.inner.update(event).await {
            bail!("failed to update API state after {height}: {event:?}",);
        }
        Ok(())
    }
}
