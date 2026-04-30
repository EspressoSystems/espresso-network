//! Worked example of the storage-migration trait architecture.
//!
//! Demonstrates a fictional storage-shape change for a `scores` table. The
//! legacy table holds a height key and a bincode-encoded byte blob; the new
//! table holds the same height key and a structured JSONB value. Every
//! production trait and every test trait is exercised against a real
//! Postgres database.
//!
//! Run with:
//!
//! ```text
//! cargo run -p hotshot-query-service \
//!     --example migration_traits \
//!     --features "sql-data-source,testing"
//! ```
//!
//! Requires Docker on the host: the example spins up a Postgres container
//! via [`TmpDb`] and tears it down on exit. The example is deliberately
//! Postgres-only; `CREATE INDEX CONCURRENTLY` does not exist in SQLite, and
//! a real-world migration of this shape would only ship for the Postgres
//! backend.

use anyhow::Context;
use async_trait::async_trait;
use hotshot_query_service::{
    data_source::{
        Transaction as _, VersionedDataSource,
        storage::sql::{Db, SqlStorage, StorageConnectionType, Transaction, Write, testing::TmpDb},
    },
    migration::{
        CleanupMigration, DataBackfill, DeferredSchemaChange, DualReadAdapter, MigrationMeta,
        MigrationRegistry,
    },
    testing::migration::{AdapterTest, BackfillTest, DeferredSchemaTest},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Score {
    pub height: u64,
    pub value: u64,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct LegacyScoreRow {
    pub height: u64,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct NewScoreRow {
    pub height: u64,
    pub value: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct LegacyPayload {
    value: u64,
    label: String,
}

pub struct ScoreAdapter;

impl DualReadAdapter for ScoreAdapter {
    type View = Score;
    type Legacy = LegacyScoreRow;
    type New = NewScoreRow;

    fn view_from_legacy(legacy: Self::Legacy) -> Self::View {
        let payload: LegacyPayload =
            bincode::deserialize(&legacy.payload).expect("legacy_scores.payload is valid bincode");
        Score {
            height: legacy.height,
            value: payload.value,
            label: payload.label,
        }
    }

    fn view_from_new(new: Self::New) -> Self::View {
        Score {
            height: new.height,
            value: new.value["value"]
                .as_u64()
                .expect("scores.value.value is u64"),
            label: new.value["label"]
                .as_str()
                .expect("scores.value.label is string")
                .to_owned(),
        }
    }

    fn legacy_to_new(legacy: Self::Legacy) -> Self::New {
        let payload: LegacyPayload =
            bincode::deserialize(&legacy.payload).expect("legacy_scores.payload is valid bincode");
        NewScoreRow {
            height: legacy.height,
            value: serde_json::json!({ "value": payload.value, "label": payload.label }),
        }
    }
}

pub struct BackfillScores;

impl MigrationMeta for BackfillScores {
    fn name(&self) -> &'static str {
        "scores_v1_backfill"
    }
    fn order(&self) -> u32 {
        1
    }
}

#[async_trait]
impl DataBackfill for BackfillScores {
    type Adapter = ScoreAdapter;

    fn batch_size(&self) -> usize {
        4
    }

    async fn migrate_batch(
        &self,
        tx: &mut Transaction<Write>,
        offset: u64,
    ) -> anyhow::Result<Option<u64>> {
        let rows = sqlx::query(
            "SELECT height, payload FROM legacy_scores ORDER BY height LIMIT $1 OFFSET $2",
        )
        .bind(self.batch_size() as i64)
        .bind(offset as i64)
        .fetch_all(tx.as_mut())
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let n = rows.len();
        for row in rows {
            let height: i64 = row.try_get("height")?;
            let payload: Vec<u8> = row.try_get("payload")?;
            let new = ScoreAdapter::legacy_to_new(LegacyScoreRow {
                height: height as u64,
                payload,
            });
            sqlx::query(
                "INSERT INTO scores (height, value) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(new.height as i64)
            .bind(&new.value)
            .execute(tx.as_mut())
            .await?;
        }

        if n < self.batch_size() {
            Ok(None)
        } else {
            Ok(Some(offset + n as u64))
        }
    }
}

pub struct IndexScores;

impl MigrationMeta for IndexScores {
    fn name(&self) -> &'static str {
        "scores_v1_index"
    }
    fn order(&self) -> u32 {
        2
    }
}

#[async_trait]
impl DeferredSchemaChange for IndexScores {
    async fn run(&self, storage: &SqlStorage) -> anyhow::Result<()> {
        sqlx::query(
            "CREATE INDEX CONCURRENTLY IF NOT EXISTS scores_label_idx ON scores \
             ((value->>'label'))",
        )
        .execute(&storage.pool())
        .await?;
        Ok(())
    }
}

pub struct DropLegacyScores;

impl MigrationMeta for DropLegacyScores {
    fn name(&self) -> &'static str {
        "scores_v1_drop_legacy"
    }
    fn order(&self) -> u32 {
        1
    }
}

#[async_trait]
impl CleanupMigration for DropLegacyScores {
    fn requires(&self) -> &'static [&'static str] {
        &["scores_v1_backfill"]
    }

    async fn run(&self, tx: &mut Transaction<Write>) -> anyhow::Result<()> {
        sqlx::query("DROP TABLE legacy_scores")
            .execute(tx.as_mut())
            .await?;
        Ok(())
    }
}

impl AdapterTest for ScoreAdapter {
    fn equivalent_pair() -> (LegacyScoreRow, NewScoreRow) {
        let inner = LegacyPayload {
            value: 42,
            label: "alpha".to_owned(),
        };
        let payload = bincode::serialize(&inner).unwrap();
        let legacy = LegacyScoreRow { height: 7, payload };
        let new = NewScoreRow {
            height: 7,
            value: serde_json::json!({ "value": 42, "label": "alpha" }),
        };
        (legacy, new)
    }
}

#[async_trait]
impl BackfillTest for BackfillScores {
    async fn seed_legacy(&self, storage: &SqlStorage, n: usize) -> anyhow::Result<()> {
        let mut tx = storage.write().await?;
        sqlx::query("TRUNCATE legacy_scores, scores")
            .execute(tx.as_mut())
            .await?;
        for i in 0..n as u64 {
            let inner = LegacyPayload {
                value: i,
                label: format!("score-{i}"),
            };
            let payload = bincode::serialize(&inner)?;
            sqlx::query("INSERT INTO legacy_scores (height, payload) VALUES ($1, $2)")
                .bind(i as i64)
                .bind(&payload)
                .execute(tx.as_mut())
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn assert_all_readable_as_new(&self, storage: &SqlStorage) -> anyhow::Result<()> {
        let mut tx = storage.read().await?;
        let legacy_count: i64 = sqlx::query_scalar("SELECT count(*) FROM legacy_scores")
            .fetch_one(tx.as_mut())
            .await?;
        let new_count: i64 = sqlx::query_scalar("SELECT count(*) FROM scores")
            .fetch_one(tx.as_mut())
            .await?;
        anyhow::ensure!(
            legacy_count == new_count,
            "row counts diverge after backfill: legacy={legacy_count} new={new_count}",
        );
        let rows = sqlx::query("SELECT height, value FROM scores ORDER BY height")
            .fetch_all(tx.as_mut())
            .await?;
        for row in rows {
            let height: i64 = row.try_get("height")?;
            let value: serde_json::Value = row.try_get("value")?;
            let actual = ScoreAdapter::view_from_new(NewScoreRow {
                height: height as u64,
                value,
            });
            let expected = Score {
                height: height as u64,
                value: height as u64,
                label: format!("score-{}", height),
            };
            anyhow::ensure!(
                actual == expected,
                "row at height {height} read incorrectly: {actual:?} != {expected:?}",
            );
        }
        Ok(())
    }

    // assert_runs_to_completion, assert_idempotent, and assert_resumable are
    // default-implemented by BackfillTest in terms of seed_legacy,
    // assert_all_readable_as_new, and migrate_batch.
}

impl DeferredSchemaTest for IndexScores {}

async fn drop_index_if_exists(pool: &sqlx::Pool<Db>) -> anyhow::Result<()> {
    sqlx::query("DROP INDEX IF EXISTS scores_label_idx")
        .execute(pool)
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = TmpDb::init().await;
    let storage = SqlStorage::connect(db.config(), StorageConnectionType::Query)
        .await
        .context("connect to TmpDb")?;
    let pool = storage.pool();

    // The expand phase of any real migration ships as a refinery SQL file in
    // `migrations/{postgres,sqlite}/V{N}__name.sql`. The example does not have
    // its own migrations folder, so we run the equivalent DDL inline here to
    // simulate "release N has shipped and refinery has run." Pretend this
    // happens before the example starts.
    apply_expand_phase(&storage).await?;

    let registry = MigrationRegistry::new()
        .backfill(BackfillScores)
        .deferred_schema(IndexScores)
        .cleanup(DropLegacyScores);
    registry.validate().context("registry validation")?;
    println!("registry validated");
    let _ = registry; // registry consumed once the runner wires up

    ScoreAdapter::assert_adapter_laws();
    println!("adapter laws hold");

    BackfillScores
        .assert_runs_to_completion(&storage, 10)
        .await?;
    println!("backfill: assert_runs_to_completion(n=10) ok");

    BackfillScores
        .assert_runs_to_completion(&storage, 0)
        .await?;
    println!("backfill: assert_runs_to_completion(n=0) ok (empty source)");

    BackfillScores
        .assert_runs_to_completion(&storage, BackfillScores.batch_size())
        .await?;
    println!(
        "backfill: assert_runs_to_completion(n={}) ok (exact batch boundary)",
        BackfillScores.batch_size()
    );

    BackfillScores.assert_idempotent(&storage, 7).await?;
    println!("backfill: assert_idempotent(n=7) ok");

    BackfillScores.assert_resumable(&storage, 9).await?;
    println!("backfill: assert_resumable(n=9) ok");

    drop_index_if_exists(&pool).await?;
    IndexScores.assert_idempotent(&storage).await?;
    println!("deferred schema: rerun is idempotent");

    let mut tx = storage.write().await?;
    DropLegacyScores.run(&mut tx).await?;
    tx.commit().await?;
    println!("cleanup migration applied");

    println!("\nall migrations and tests passed");
    Ok(())
}

async fn apply_expand_phase(storage: &SqlStorage) -> anyhow::Result<()> {
    let mut tx = storage.write().await?;
    sqlx::query("CREATE TABLE legacy_scores (height BIGINT PRIMARY KEY, payload BYTEA NOT NULL)")
        .execute(tx.as_mut())
        .await?;
    sqlx::query("CREATE TABLE scores (height BIGINT PRIMARY KEY, value JSONB NOT NULL)")
        .execute(tx.as_mut())
        .await?;
    tx.commit().await?;
    Ok(())
}
