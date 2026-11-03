//! The query service on an embedded SQLite file.

use hotshot_query_service::data_source::sql::sqlite as qs;

use crate::persistence::sql::sqlite as persistence;

#[expect(
    clippy::duplicate_mod,
    reason = "each backend compiles the shared tree against its own `Db`"
)]
#[path = "../sql.rs"]
mod imp;

pub use imp::*;
