use std::{future::Future, path::PathBuf};

use anyhow::{Context, Result};
use espresso_types::SeqTypes;
use hotshot_query_service::{
    availability::{LeafId, LeafQueryData},
    types::HeightIndexed,
};
use sqlx::{query, query_as, sqlite::SqlitePoolOptions, SqlitePool};

/// Client-side database for a [`LightClient`].
pub trait Storage: Sized + Send + Sync + 'static {
    /// Create a default, empty instance of the state.
    ///
    /// This is an async, fallible version of [`Default::default`]. If `Self: Default`, this is
    /// equivalent to `ready(Ok(<Self as Default>::default()))`.
    fn default() -> impl Send + Future<Output = Result<Self>>;

    /// Get the earliest available leaf which is later than or equal to the requested leaf.
    ///
    /// This will either be the leaf requested, or can be used as the known-finalized endpoint in a
    /// leaf chain proving that requested leaf is finalized (after the requested leaf is fetched
    /// from elsewhere).
    ///
    /// If there is no known leaf later than the requested leaf, the result is [`None`].
    fn leaf_upper_bound(
        &self,
        leaf: LeafId<SeqTypes>,
    ) -> impl Send + Future<Output = Result<Option<LeafQueryData<SeqTypes>>>>;

    /// Add a leaf to the cache.
    ///
    /// This may result in an older leaf being removed.
    fn insert_leaf(&self, leaf: LeafQueryData<SeqTypes>)
        -> impl Send + Future<Output = Result<()>>;
}

#[derive(Clone, Debug)]
pub struct SqliteOptions {
    /// Maximum number of simultaneous DB connections to allow.
    pub num_connections: u32,

    /// Maximum number of leaves to cache in the local DB.
    pub num_leaves: u32,

    /// Create or open storage that is persisted on the file system.
    ///
    /// If not present, the database will exist only in memory and will be destroyed when the
    /// [`SqlitePersistence`] object is dropped.
    pub path: Option<PathBuf>,
}

impl Default for SqliteOptions {
    fn default() -> Self {
        Self {
            num_connections: 5,
            num_leaves: 100,
            path: None,
        }
    }
}

impl SqliteOptions {
    /// Create or connect to a database with the given options.
    pub async fn connect(self) -> Result<SqliteStorage> {
        let path = match &self.path {
            Some(path) => path.to_str().context("invalid file path")?,
            None => ":memory:",
        };
        let pool = SqlitePoolOptions::default()
            .max_connections(self.num_connections)
            .connect(path)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(SqliteStorage {
            pool,
            num_leaves: self.num_leaves,
        })
    }
}

/// [`Storage`] based on a SQLite database.
#[derive(Clone, Debug)]
pub struct SqliteStorage {
    pool: SqlitePool,
    num_leaves: u32,
}

impl Storage for SqliteStorage {
    async fn default() -> Result<Self> {
        SqliteOptions::default().connect().await
    }

    async fn leaf_upper_bound(
        &self,
        id: LeafId<SeqTypes>,
    ) -> Result<Option<LeafQueryData<SeqTypes>>> {
        let mut tx = self.pool.begin().await?;
        let (height, data): (i64, _) = match id {
            LeafId::Number(n) => {
                let Some((height, data)) = query_as(
                    "SELECT height, data FROM leaf WHERE height >= $1 ORDER BY height LIMIT 1",
                )
                .bind(n as i64)
                .fetch_optional(tx.as_mut())
                .await?
                else {
                    return Ok(None);
                };
                (height, data)
            },
            LeafId::Hash(h) => {
                let Some((height, data)) =
                    query_as("SELECT height, data FROM leaf WHERE hash = $1 LIMIT 1")
                        .bind(h.to_string())
                        .fetch_optional(tx.as_mut())
                        .await?
                else {
                    return Ok(None);
                };
                (height, data)
            },
        };
        let leaf = serde_json::from_value(data)?;

        // Mark this leaf as recently used.
        let (id,): (i32,) = query_as("SELECT max(id) + 1 FROM leaf")
            .fetch_one(tx.as_mut())
            .await?;
        query("UPDATE leaf SET id = $1 WHERE height = $2")
            .bind(id)
            .bind(height)
            .execute(tx.as_mut())
            .await?;
        tx.commit().await?;

        Ok(Some(leaf))
    }

    async fn insert_leaf(&self, leaf: LeafQueryData<SeqTypes>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let height = leaf.height() as i64;
        let hash = leaf.hash().to_string();
        let data = serde_json::to_value(leaf)?;

        tracing::debug!(height, hash, "inserting leaf");
        let (id,): (i32,) = query_as(
            "INSERT INTO leaf (height, hash, data) VALUES ($1, $2, $3)
                    ON CONFLICT (height) DO UPDATE SET id = excluded.id
                    RETURNING id",
        )
        .bind(height)
        .bind(&hash)
        .bind(data)
        .fetch_one(tx.as_mut())
        .await
        .context("inserting new leaf")?;
        tracing::debug!(height, hash, id, "inserted leaf");

        // Delete the oldest leaves as necessary until the number of leaves stored does not exceed
        // `num_leaves`.
        let (num_leaves,): (u32,) = query_as("SELECT count(*) FROM leaf")
            .fetch_one(tx.as_mut())
            .await
            .context("counting leaves")?;
        let to_delete = num_leaves.saturating_sub(self.num_leaves);
        if to_delete > 0 {
            let (id_to_delete,): (i64,) =
                query_as("SELECT id FROM leaf ORDER BY id LIMIT 1 OFFSET $1")
                    .bind(to_delete - 1)
                    .fetch_one(tx.as_mut())
                    .await
                    .context("finding timestamp for GC")?;
            tracing::info!(id_to_delete, "garbage collecting {to_delete} leaves");
            let res = query("DELETE FROM leaf WHERE id <= $1")
                .bind(id_to_delete)
                .execute(tx.as_mut())
                .await
                .context("deleting old leaves")?;
            tracing::info!("deleted {} leaves", res.rows_affected());
        }

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use espresso_types::EpochVersion;

    use super::*;
    use crate::testing::leaf_chain;

    #[tokio::test]
    #[test_log::test]
    async fn test_leaf_upper_bound_exact() {
        let db = SqliteStorage::default().await.unwrap();

        let leaf = leaf_chain::<EpochVersion>(0..1).await.remove(0);
        db.insert_leaf(leaf.clone()).await.unwrap();
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaf
        );
        assert_eq!(
            db.leaf_upper_bound(LeafId::Hash(leaf.hash()))
                .await
                .unwrap()
                .unwrap(),
            leaf
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_leaf_upper_bound_loose() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..=1).await;
        db.insert_leaf(leaves[1].clone()).await.unwrap();
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaves[1]
        );
        // Searching by hash either gives an exact match or fails, there is no way of "upper
        // bounding" a hash.
        assert_eq!(
            db.leaf_upper_bound(LeafId::Hash(leaves[0].hash()))
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_leaf_upper_bound_not_found() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..=1).await;
        db.insert_leaf(leaves[0].clone()).await.unwrap();
        assert_eq!(db.leaf_upper_bound(LeafId::Number(1)).await.unwrap(), None);
        assert_eq!(
            db.leaf_upper_bound(LeafId::Hash(leaves[1].hash()))
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_gc_last_inserted() {
        let db = SqliteOptions {
            num_leaves: 1,
            ..Default::default()
        }
        .connect()
        .await
        .unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..=1).await;
        db.insert_leaf(leaves[1].clone()).await.unwrap();
        db.insert_leaf(leaves[0].clone()).await.unwrap();

        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaves[0]
        );
        assert_eq!(db.leaf_upper_bound(LeafId::Number(1)).await.unwrap(), None);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_gc_last_selected() {
        let db = SqliteOptions {
            num_leaves: 2,
            ..Default::default()
        }
        .connect()
        .await
        .unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..=2).await;
        db.insert_leaf(leaves[0].clone()).await.unwrap();
        db.insert_leaf(leaves[1].clone()).await.unwrap();

        // Select leaf 0, making it more recently used than leaf 1.
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaves[0]
        );

        // Insert a third leaf, causing the least recently used (leaf 1) to be garbage collected.
        db.insert_leaf(leaves[2].clone()).await.unwrap();

        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaves[0]
        );
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(1))
                .await
                .unwrap()
                .unwrap(),
            leaves[2]
        );
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(2))
                .await
                .unwrap()
                .unwrap(),
            leaves[2]
        );
    }
}
