use async_trait::async_trait;
use derive_more::From;
use futures::stream::StreamExt;
use refinery_core::{
    Migration,
    traits::r#async::{AsyncMigrate, AsyncQuery, AsyncTransaction},
};
use sqlx::{Acquire, Executor, Row, pool::PoolConnection};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use super::{Db, queries::DecodeError};

/// Run migrations using a sqlx connection.
///
/// While SQLx has its own built-in migration functionality, we use Refinery, and alas we must
/// support existing deployed databases which are already using Refinery to handle migrations.
/// Rather than implement a tricky "migration of the migrations table", or supporting separate
/// migrations interfaces for databases deployed before and after the switch to SQLx, we continue
/// using Refinery. This wrapper implements the Refinery traits for SQLx types.
#[derive(Debug, From)]
pub(super) struct Migrator<'a> {
    conn: &'a mut PoolConnection<Db>,
}

#[async_trait]
impl AsyncTransaction for Migrator<'_> {
    type Error = sqlx::Error;

    async fn execute(&mut self, queries: &[&str]) -> sqlx::Result<usize> {
        let mut tx = self.conn.begin().await?;
        let mut count = 0;
        for query in queries {
            let res = tx.execute(*query).await?;
            count += res.rows_affected();
        }
        tx.commit().await?;
        Ok(count as usize)
    }
}

#[async_trait]
impl AsyncQuery<Vec<Migration>> for Migrator<'_> {
    async fn query(&mut self, query: &str) -> sqlx::Result<Vec<Migration>> {
        let mut tx = self.conn.begin().await?;

        let mut applied = Vec::new();
        let mut rows = tx.fetch(query);
        while let Some(row) = rows.next().await {
            let row = row?;
            let version = row.try_get(0)?;
            let applied_on: String = row.try_get(2)?;
            let applied_on = OffsetDateTime::parse(&applied_on, &Rfc3339)
                .decode_error("malformed migration timestamp")?;
            let checksum: String = row.get(3);

            applied.push(Migration::applied(
                version,
                row.try_get(1)?,
                applied_on,
                checksum
                    .parse::<u64>()
                    .decode_error("malformed migration checksum")?,
            ));
        }

        drop(rows);
        tx.commit().await?;
        Ok(applied)
    }
}

impl AsyncMigrate for Migrator<'_> {}
