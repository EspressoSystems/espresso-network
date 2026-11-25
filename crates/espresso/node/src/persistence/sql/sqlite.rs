//! Persistence on an embedded SQLite file.

use std::path::Path;

use anyhow::Context as _;
use hotshot_query_service::{data_source::storage::sql::sqlite as qs, include_migrations};
use light_client::storage::LightClientDbOptions;
use sqlx::{Decode, Row, Type};

#[expect(
    clippy::duplicate_mod,
    reason = "each backend compiles the shared tree against its own `Db`"
)]
#[path = "../sql.rs"]
mod imp;

pub use imp::*;
use qs::{Config, SqlStorage};

use super::Options as SqlOptions;
#[cfg(any(test, feature = "testing"))]
use super::SqliteOptions;
use crate::{
    api::options::Query,
    persistence::storage_probe::{self, SqlitePragmas, StorageProbe},
};

impl BackendOptions for qs::Db {
    fn apply(mut cfg: Config, opt: &SqlOptions) -> anyhow::Result<Config> {
        let path = opt
            .sqlite_options
            .path
            .clone()
            .context("ESPRESSO_NODE_STORAGE_PATH must be set for an embedded database")?;
        if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("failed to create directory: {dir:?}"))?;
        }

        cfg = cfg.migrations(include_migrations!(
            "$CARGO_MANIFEST_DIR/api/migrations/sqlite"
        ));

        Ok(cfg.db_path(path))
    }

    fn light_client_db(_cfg: &Config, query: &Query) -> LightClientDbOptions {
        LightClientDbOptions::Sqlite(query.sqlite_light_client_db())
    }

    async fn reset_light_client(_cfg: &Config) -> anyhow::Result<()> {
        Ok(())
    }

    async fn storage_probe(
        db: &SqlStorage,
        opt: &SqlOptions,
    ) -> anyhow::Result<Option<StorageProbe>> {
        let pragmas = read_pragmas(&db.pool()).await;
        let path = opt
            .sqlite_options
            .path
            .clone()
            .context("ESPRESSO_NODE_STORAGE_PATH must be set for an embedded database")?;
        Ok(Some(
            storage_probe::probe(sqlite_probe_dir(&path), pragmas).await?,
        ))
    }
}

/// The directory `storage_probe::probe` should classify for a SQLite database at `path`. Falls
/// back to `.` when `path` has no parent, e.g. a bare relative filename, or the empty `PathBuf`
/// from `SqliteOptions::default()`.
fn sqlite_probe_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

async fn read_pragma<T>(pool: &sqlx::Pool<qs::Db>, pragma: &str) -> Option<T>
where
    T: for<'a> Decode<'a, qs::Db> + Type<qs::Db>,
{
    match sqlx::query(pragma).fetch_one(pool).await {
        Ok(row) => row.try_get(0).ok(),
        Err(err) => {
            tracing::debug!(pragma, ?err, "storage probe: pragma query failed");
            None
        },
    }
}

/// The first failure short-circuits the rest: a pragma query only fails if the pool is unusable.
async fn read_pragmas(pool: &sqlx::Pool<qs::Db>) -> Option<SqlitePragmas> {
    let journal_mode = read_pragma(pool, "PRAGMA journal_mode").await?;
    // SQLite reports these two back as their numeric settings, not the names used to set them.
    let synchronous = match read_pragma::<i64>(pool, "PRAGMA synchronous").await? {
        0 => "off",
        1 => "normal",
        2 => "full",
        3 => "extra",
        _ => "unknown",
    };
    let auto_vacuum = match read_pragma::<i64>(pool, "PRAGMA auto_vacuum").await? {
        0 => "none",
        1 => "full",
        2 => "incremental",
        _ => "unknown",
    };
    let page_size: i64 = read_pragma(pool, "PRAGMA page_size").await?;

    Some(SqlitePragmas {
        journal_mode,
        synchronous,
        auto_vacuum,
        page_size: page_size as u64,
    })
}

#[cfg(any(test, feature = "testing"))]
pub(crate) fn tmp_options(db: &qs::testing::TmpDb) -> SqlOptions {
    SqliteOptions {
        path: Some(db.path()),
    }
    .into()
}

#[cfg(test)]
mod test {
    use std::path::{Path, PathBuf};

    use espresso_types::traits::{PersistenceOptions as _, SequencerPersistence as _};
    use hotshot_query_service::metrics::PrometheusMetrics;
    use hotshot_types::traits::metrics::Metrics;

    use super::{
        Config, SqlOptions, SqliteOptions, imp::Persistence, read_pragmas, sqlite_probe_dir,
    };
    use crate::persistence::tests::TestablePersistence as _;

    #[test]
    fn embedded_db_without_a_storage_path_is_an_error() {
        let opt = SqlOptions {
            embedded_db: true,
            sqlite_options: SqliteOptions { path: None },
            ..SqlOptions::default()
        };

        let Err(err) = Config::try_from(&opt) else {
            panic!("an embedded database with no storage path was accepted");
        };
        assert!(
            format!("{err:#}").contains("ESPRESSO_NODE_STORAGE_PATH"),
            "{err:#}"
        );
    }

    #[tokio::test]
    async fn read_pragmas_falls_back_to_none_on_query_error() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(":memory:")
            .await
            .unwrap();
        pool.close().await;

        assert!(read_pragmas(&pool).await.is_none());
    }

    #[tokio::test]
    async fn read_pragmas_reads_live_values() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect(":memory:")
            .await
            .unwrap();

        let pragmas = read_pragmas(&pool).await.expect("pragmas readable");

        assert_eq!(pragmas.journal_mode, "memory");
        assert_ne!(pragmas.synchronous, "unknown");
        assert!(pragmas.page_size > 0);
    }

    #[test]
    fn sqlite_probe_dir_falls_back_to_cwd_without_a_parent() {
        // `SqliteOptions::default()`'s `path` is empty, whose `parent()` is `Some("")`, not `None`.
        assert_eq!(sqlite_probe_dir(&PathBuf::new()), Path::new("."));
        assert_eq!(sqlite_probe_dir(Path::new("database")), Path::new("."));
        assert_eq!(
            sqlite_probe_dir(Path::new("/var/lib/espresso/sqlite/database")),
            Path::new("/var/lib/espresso/sqlite")
        );
    }

    /// The probe is taken in `create()` and only reaches the exported registry through
    /// `enable_metrics`, which `init_node` calls.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_storage_probe_reaches_exported_registry() {
        let tmp = Persistence::tmp_storage().await;
        let mut persistence = Persistence::options(&tmp).create().await.unwrap();

        // Dropping WAL from `sqlite_options()` should fail this assertion.
        assert_eq!(
            persistence
                .probe
                .as_ref()
                .and_then(|probe| probe.pragmas.as_ref())
                .map(|pragmas| pragmas.journal_mode.as_str()),
            Some("wal")
        );

        let metrics = PrometheusMetrics::default();
        persistence.enable_metrics(&*Metrics::subgroup(&metrics, "consensus".to_string()));

        let exported = metrics.export().unwrap();
        assert!(exported.contains("consensus_disk_info"), "{exported}");
        assert!(exported.contains("backend=\"sqlite\""), "{exported}");
        assert!(
            exported.contains("consensus_disk_fsync_micros"),
            "{exported}"
        );
    }
}
