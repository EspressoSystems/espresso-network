//! V1 status API.

use async_trait::async_trait;
use serde::Serialize;

/// Espresso's extension to the status API.
///
/// The base status surface (block height, view success rate, time since the last decide, and the
/// Prometheus metrics export) is served by `hotshot_query_service::status::router`, which the
/// binary mounts alongside this route; see [`crate::create_router_v1`].
#[async_trait]
pub trait StatusApiExtension {
    type Keys: Serialize + Send + Sync + 'static;

    async fn keys(&self) -> anyhow::Result<Self::Keys>;
}
