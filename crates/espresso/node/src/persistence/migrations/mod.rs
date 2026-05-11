pub mod hash_id_bigint;

use hotshot_query_service::migration::MigrationRegistry;

use self::hash_id_bigint::{
    HashTableBackfill, IndexHashIdBig, MerkleHashIdBackfill, RestoreHashConstraints,
};

/// Build the [`MigrationRegistry`] for the espresso node's deferred migrations.
pub fn build_registry() -> MigrationRegistry {
    MigrationRegistry::new()
        .backfill(HashTableBackfill)
        .backfill(MerkleHashIdBackfill)
        .deferred_schema(IndexHashIdBig)
        .cleanup(RestoreHashConstraints)
}
