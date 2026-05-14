pub mod hash_id_bigint;

use hotshot_query_service::migration::MigrationRegistry;

use self::hash_id_bigint::{BackfillIds, BackfillRefs, CreateIndex};

/// Build the [`MigrationRegistry`] for the espresso node's deferred migrations.
///
/// `Cleanup` is intentionally absent: per the storage-migrations design doc it must ship in a
/// release strictly after the backfills. It will be re-added once the write paths in
/// `hotshot-query-service` are updated to use the post-cleanup column names.
pub fn build_registry() -> MigrationRegistry {
    let mut registry = MigrationRegistry::new().backfill(BackfillIds);
    for refs in BackfillRefs::all() {
        registry = registry.backfill(refs);
    }
    registry.deferred_schema(CreateIndex)
}
