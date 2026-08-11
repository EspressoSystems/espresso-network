//! V1 hotshot-events API.
//!
//! Mirrors the endpoints of the legacy `hotshot-events-service` API (its tide-disco
//! `hotshot_events.toml`, since removed).

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
