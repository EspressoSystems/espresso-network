pub mod hash_id_bigint;

use hotshot_query_service::migration::MigrationRegistry;

use self::hash_id_bigint::{BackfillIds, BackfillRefs, Cleanup, CreateIndex};

/// Build the [`MigrationRegistry`] for the espresso node's deferred migrations.
pub fn build_registry() -> MigrationRegistry {
    MigrationRegistry::new()
        .backfill(BackfillIds)
        .backfill(BackfillRefs)
        .deferred_schema(CreateIndex)
        .cleanup(Cleanup)
}
