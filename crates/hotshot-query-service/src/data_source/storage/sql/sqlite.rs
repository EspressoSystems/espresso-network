// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

//! SQL storage backed by an embedded SQLite file.

use std::{path::PathBuf, time::Duration};

use sqlx::{
    pool::{Pool, PoolOptions},
    sqlite::SqliteConnectOptions,
};

pub type Db = sqlx::Sqlite;

#[expect(
    clippy::duplicate_mod,
    reason = "each backend compiles the shared tree against its own `Db`"
)]
#[path = "../sql.rs"]
mod imp;

pub use imp::*;

use crate::{include_migrations, sqlite_options::sqlite_options};

impl Backend for Db {
    type ConnectOptions = SqliteConnectOptions;

    const KIND: DbKind = DbKind::Sqlite;
    const MAX_FN: &'static str = "MAX";

    fn default_connect_options() -> SqliteConnectOptions {
        sqlite_options()
    }

    fn migrations() -> Vec<Migration> {
        include_migrations!("$CARGO_MANIFEST_DIR/migrations/sqlite").collect()
    }

    fn delete_file(opts: &SqliteConnectOptions) -> std::io::Result<()> {
        std::fs::remove_file(opts.get_filename())
    }

    fn session_hooks(pool: PoolOptions<Db>, _schema: String) -> PoolOptions<Db> {
        pool
    }

    fn log_conflict_sessions(_pool: Pool<Db>, _op: &'static str) {}
}

impl StateQueries for Db {}

impl From<SqliteConnectOptions> for Config {
    fn from(db_opt: SqliteConnectOptions) -> Self {
        Self::with_connect_options(db_opt)
    }
}

impl Config {
    pub fn busy_timeout(mut self, timeout: Duration) -> Self {
        self.db_opt = self.db_opt.busy_timeout(timeout);
        self
    }

    pub fn db_path(mut self, path: PathBuf) -> Self {
        self.db_opt = self.db_opt.filename(path);
        self
    }

    /// not supported for SQLite.
    pub fn statement_timeout(self, _timeout: Duration) -> Self {
        self
    }
}

#[cfg(all(any(test, feature = "testing"), not(target_os = "windows")))]
mod tmp_db {
    use std::path::PathBuf;

    use super::{Config, testing::test_migrations};

    #[derive(Debug)]
    pub struct TmpDb {
        db_path: PathBuf,
    }

    impl TmpDb {
        pub async fn init() -> Self {
            let file = tempfile::Builder::new()
                .prefix("sqlite-")
                .suffix(".db")
                .tempfile()
                .unwrap();
            let (_, db_path) = file.keep().unwrap();
            Self { db_path }
        }

        pub fn path(&self) -> PathBuf {
            self.db_path.clone()
        }

        pub fn config(&self) -> Config {
            Config::default()
                .db_path(self.db_path.clone())
                .migrations(test_migrations())
        }
    }

    impl Drop for TmpDb {
        fn drop(&mut self) {
            std::fs::remove_file(self.db_path.clone()).unwrap();
        }
    }
}

#[cfg(all(test, not(target_os = "windows")))]
mod test {
    use std::str::FromStr;

    use super::Config;

    #[test]
    fn test_config_from_str() {
        let cfg = Config::from_str("sqlite://data.db").unwrap();
        assert_eq!(cfg.db_opt.get_filename().to_string_lossy(), "data.db");
    }
}
