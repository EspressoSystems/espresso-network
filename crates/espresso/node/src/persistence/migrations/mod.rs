pub mod hash_id_bigint;

use hotshot_query_service::migration::MigrationRegistry;

use self::hash_id_bigint::{BackfillIds, BackfillRefs, Cleanup, CreateIndex};

/// Build the [`MigrationRegistry`] for the espresso node's deferred migrations.
pub fn build_registry() -> MigrationRegistry {
    let mut registry = MigrationRegistry::new().backfill(BackfillIds);
    for refs in BackfillRefs::all() {
        registry = registry.backfill(refs);
    }
    registry.deferred_schema(CreateIndex).cleanup(Cleanup)
}
