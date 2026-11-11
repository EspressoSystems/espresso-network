//! V1 database API.
//!
//! Diagnostic. `DatabaseService` mirrors this shape on v2, where field numbers are frozen, so
//! the fields here are no longer free to change.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Database table size information.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TableSize {
    pub table_name: String,
    pub row_count: i64,
    pub total_size_bytes: Option<i64>,
}

/// Status of a single deferred background migration.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MigrationStatus {
    pub name: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_offset: Option<i64>,
}

#[async_trait]
pub trait DatabaseApi {
    async fn get_table_sizes(&self) -> anyhow::Result<Vec<TableSize>>;
    async fn get_migration_status(&self) -> anyhow::Result<Vec<MigrationStatus>>;
}
