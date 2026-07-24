//! V1 hotshot-events API.
//!
//! Mirrors the tide-disco endpoints defined in `hotshot-events-service/api/hotshot_events.toml`.

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::Serialize;

#[async_trait]
pub trait HotShotEventsApi {
    type Event: Serialize + Send + Sync + 'static;
    type StartupInfo: Serialize + Send + Sync + 'static;

    async fn startup_info(&self) -> anyhow::Result<Self::StartupInfo>;

    async fn events(&self) -> anyhow::Result<BoxStream<'static, Self::Event>>;
}
