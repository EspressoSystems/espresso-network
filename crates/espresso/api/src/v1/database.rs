//! V1 database API.
//!
//! Diagnostic. `DatabaseService` mirrors this shape on v2, where field numbers are frozen, so
//! the fields here are no longer free to change.

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait DatabaseApi {
    type TableSizes: Serialize + Send + Sync + 'static;
    type MigrationStatus: Serialize + Send + Sync + 'static;

    async fn get_table_sizes(&self) -> anyhow::Result<Self::TableSizes>;
    async fn get_migration_status(&self) -> anyhow::Result<Self::MigrationStatus>;
}
