//! Update loop for query API state.

use super::{data_source::SequencerDataSource, StorageState};
use crate::{network, persistence::SequencerPersistence, SeqTypes};
use async_std::sync::{Arc, RwLock};
use futures::stream::{Stream, StreamExt};
use hotshot::types::Event;
use hotshot_query_service::data_source::{UpdateDataSource, VersionedDataSource};
use vbs::version::StaticVersionType;

pub(super) async fn update_loop<N, P, D, Ver: StaticVersionType>(
    state: Arc<RwLock<StorageState<N, P, D, Ver>>>,
    mut events: impl Stream<Item = Event<SeqTypes>> + Unpin,
) where
    N: network::Type,
    P: SequencerPersistence,
    D: SequencerDataSource + Send + Sync,
{
    tracing::debug!("waiting for event");
    while let Some(event) = events.next().await {
        let mut state = state.write().await;

        // If update results in an error, revert to undo partial state changes. We will continue
        // streaming events, as we can update our state based on future events and then filling in
        // the missing part of the state later, by fetching from a peer.
        if let Err(err) = update_state(&mut *state, &event).await {
            tracing::error!(
                ?event,
                %err,
                "failed to update API state",
            );
            state.revert().await;
        }
    }
    tracing::warn!("end of HotShot event stream, updater task will exit");
}

async fn update_state<N, P, D, Ver: StaticVersionType>(
    state: &mut StorageState<N, P, D, Ver>,
    event: &Event<SeqTypes>,
) -> anyhow::Result<()>
where
    N: network::Type,
    P: SequencerPersistence,
    D: SequencerDataSource + Send + Sync,
{
    state.update(event).await?;
    state.commit().await?;

    Ok(())
}
