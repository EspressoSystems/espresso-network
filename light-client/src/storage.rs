use std::{future::Future, path::PathBuf};

use anyhow::{Context, Result};
use derive_more::{Display, From};
use espresso_types::SeqTypes;
use hotshot_query_service::{
    availability::{BlockId, LeafId, LeafQueryData},
    types::HeightIndexed,
};
use sqlx::{query, query_as, sqlite::SqlitePoolOptions, QueryBuilder, SqlitePool};

/// Different ways to ask the database for a leaf.
#[derive(Clone, Copy, Debug, Display, From)]
pub enum LeafRequest {
    /// Ask for a leaf with a given ID.
    #[display("leaf {_0}")]
    Leaf(LeafId<SeqTypes>),

    /// Ask for the leaf containing a header with a given ID.
    #[display("header {_0}")]
    Header(BlockId<SeqTypes>),
}

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
        leaf: impl Into<LeafRequest> + Send,
    ) -> impl Send + Future<Output = Result<Option<LeafQueryData<SeqTypes>>>>;

    /// Get all leaves in the range [start, end)
    fn get_leaves_in_range(
        &self,
        start: u32,
        end: u32,
    ) -> impl Send + Future<Output = Result<Vec<LeafQueryData<SeqTypes>>>>;

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
        id: impl Into<LeafRequest> + Send,
    ) -> Result<Option<LeafQueryData<SeqTypes>>> {
        let mut tx = self.pool.begin().await?;

        let mut q = QueryBuilder::new("SELECT height, data FROM leaf WHERE ");
        match id.into() {
            LeafRequest::Leaf(LeafId::Number(n)) | LeafRequest::Header(BlockId::Number(n)) => {
                q.push("height >= ")
                    .push_bind(n as i64)
                    .push("ORDER BY HEIGHT");
            },
            LeafRequest::Leaf(LeafId::Hash(h)) => {
                q.push("hash = ").push_bind(h.to_string());
            },
            LeafRequest::Header(BlockId::Hash(h)) => {
                q.push("block_hash = ").push_bind(h.to_string());
            },
            LeafRequest::Header(BlockId::PayloadHash(h)) => {
                q.push("payload_hash = ")
                    .push_bind(h.to_string())
                    .push("ORDER BY height");
            },
        }
        q.push(" LIMIT 1");

        let Some((height, data)) = q
            .build_query_as::<(i64, _)>()
            .fetch_optional(tx.as_mut())
            .await?
        else {
            return Ok(None);
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

    async fn get_leaves_in_range(
        &self,
        start_height: u32,
        end_height: u32,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        let mut tx = self.pool.begin().await?;

        let leaves = query_as::<_, (i64, serde_json::Value)>(
            "SELECT height, data FROM leaf WHERE height >= $1 AND height < $2 ORDER BY height",
        )
        .bind(start_height as i64)
        .bind(end_height as i64)
        .fetch_all(tx.as_mut())
        .await?
        .into_iter()
        .map(|(_height, data)| serde_json::from_value(data))
        .collect::<Result<Vec<_>, _>>()?;

        tx.commit().await?;

        Ok(leaves)
    }

    async fn insert_leaf(&self, leaf: LeafQueryData<SeqTypes>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        let height = leaf.height() as i64;
        let hash = leaf.hash().to_string();
        let block_hash = leaf.block_hash().to_string();
        let payload_hash = leaf.payload_hash().to_string();
        let data = serde_json::to_value(leaf)?;

        tracing::debug!(height, hash, "inserting leaf");
        let (id,): (i32,) = query_as(
            "INSERT INTO leaf (height, hash, block_hash, payload_hash, data) VALUES ($1, $2, $3, \
             $4, $5)
                    ON CONFLICT (height) DO UPDATE SET id = excluded.id
                    RETURNING id",
        )
        .bind(height)
        .bind(&hash)
        .bind(&block_hash)
        .bind(&payload_hash)
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
    use pretty_assertions::assert_eq;

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
        assert_eq!(
            db.leaf_upper_bound(BlockId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaf
        );
        assert_eq!(
            db.leaf_upper_bound(BlockId::Hash(leaf.block_hash()))
                .await
                .unwrap()
                .unwrap(),
            leaf
        );
        assert_eq!(
            db.leaf_upper_bound(BlockId::PayloadHash(leaf.payload_hash()))
                .await
                .unwrap()
                .unwrap()
                .payload_hash(),
            leaf.payload_hash()
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
        assert_eq!(
            db.leaf_upper_bound(BlockId::Hash(leaves[0].block_hash()))
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_leaf_upper_bound_least_upper_bound() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..=2).await;
        db.insert_leaf(leaves[2].clone()).await.unwrap();
        db.insert_leaf(leaves[1].clone()).await.unwrap();
        assert_eq!(
            db.leaf_upper_bound(LeafId::Number(0))
                .await
                .unwrap()
                .unwrap(),
            leaves[1]
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

    #[tokio::test]
    #[test_log::test]
    async fn test_get_leaves_in_range() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..5).await;
        for leaf in &leaves {
            db.insert_leaf(leaf.clone()).await.unwrap();
        }

        let fetched = db.get_leaves_in_range(1, 4).await.unwrap();
        assert_eq!(fetched, leaves[1..4]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_get_leaves_in_range_not_found() {
        let db = SqliteStorage::default().await.unwrap();

        let leaves = leaf_chain::<EpochVersion>(0..3).await;
        for leaf in &leaves {
            db.insert_leaf(leaf.clone()).await.unwrap();
        }

        let fetched = db.get_leaves_in_range(3, 5).await.unwrap();
        assert!(fetched.is_empty());
    }
}
