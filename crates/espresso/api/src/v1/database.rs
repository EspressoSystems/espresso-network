//! V1 database API.
//!
//! Mirrors the tide-disco endpoints defined in `crates/espresso/node/api/database.toml`.
//! Diagnostic-only; not required to be byte-identical with the tide-disco response.

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait DatabaseApi {
    type TableSizes: Serialize + Send + Sync + 'static;

    async fn get_table_sizes(&self) -> anyhow::Result<Self::TableSizes>;
}
