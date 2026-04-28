//! Trait architecture for non-blocking SQL storage migrations.
//!
//! See `doc/storage-migrations.md` in the workspace root for the prose
//! description and authoring rules. This module defines only the trait
//! surface and the registry that organizes migrations into three buckets.
//! The actual runner that drives deferred work lives alongside the
//! [`SqlStorage`] in `data_source::storage::sql`.
//!
//! Fast pre-consensus DDL stays as plain refinery SQL files in
//! `migrations/{postgres,sqlite}/`; this module covers only the cases
//! refinery cannot handle. Each migration implements [`MigrationMeta`] for
//! its identity plus exactly one of the three kind traits:
//!
//! - [`DeferredSchemaChange`]: slow DDL (for example
//!   `CREATE INDEX CONCURRENTLY`), runs post-consensus, non-transactional.
//! - [`DataBackfill`]: batched data rewrite, runs post-consensus, paired with
//!   a [`DualReadAdapter`] that keeps readers correct during the backfill
//!   window.
//! - [`CleanupMigration`]: cleanup that drops legacy state once a paired
//!   backfill has shipped to every operator.
//!
//! The kind traits are directly object-safe; [`MigrationRegistry`] stores
//! `Box<dyn KindTrait>` for each bucket.

use std::collections::HashSet;

use anyhow::bail;
use async_trait::async_trait;

use crate::data_source::storage::sql::{SqlStorage, Transaction, Write};

/// Identity carried by every migration.
///
/// `name` is persisted in the `deferred_migrations` table and referenced from
/// [`CleanupMigration::requires`]. It must be globally unique and stable for
/// the lifetime of the migration. `order` is unique within each bucket and
/// determines the order in which migrations of the same kind run.
pub trait MigrationMeta: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn order(&self) -> u32;
}

/// Slow DDL run as a background task post-consensus, outside any transaction.
///
/// Required for operations such as `CREATE INDEX CONCURRENTLY` that forbid a
/// surrounding transaction. Implementations must be idempotent because the
/// runner may invoke `run` repeatedly across restarts; in practice this means
/// using `IF NOT EXISTS` and similar guards on every statement. Completion
/// is tracked by the runner in `deferred_migrations`; there is no separate
/// schema-level applied check.
#[async_trait]
pub trait DeferredSchemaChange: MigrationMeta {
    async fn run(&self, storage: &SqlStorage) -> anyhow::Result<()>;
}

/// Batched data rewrite run as a background task post-consensus.
///
/// Each batch is processed inside a write transaction; the runner persists the
/// returned offset in `deferred_migrations` between batches so a restart
/// resumes from the last committed point. A backfill is paired with a
/// [`DualReadAdapter`] that keeps reader code correct against the mixture of
/// legacy and new rows that exists during the migration window.
///
/// The associated `Adapter` type is exposed for type-checking the impl; it is
/// not visible through the registry. Reader code references the concrete
/// adapter type directly at the callsite and does not look it up via the
/// registry.
#[async_trait]
pub trait DataBackfill: MigrationMeta {
    type Adapter: DualReadAdapter;

    fn batch_size(&self) -> usize {
        1_000
    }

    async fn migrate_batch(
        &self,
        tx: &mut Transaction<Write>,
        offset: u64,
    ) -> anyhow::Result<Option<u64>>;
}

/// Cleanup run pre-consensus that drops legacy state once a paired backfill
/// has shipped to every operator.
///
/// `requires` returns the names of the deferred migrations and backfills that
/// must be marked completed in `deferred_migrations` before this cleanup may
/// run. The runner refuses to execute a cleanup whose requirements are unmet,
/// protecting an operator who upgrades across two releases without waiting
/// for the backfill to finish.
#[async_trait]
pub trait CleanupMigration: MigrationMeta {
    fn requires(&self) -> &'static [&'static str] {
        &[]
    }

    async fn run(&self, tx: &mut Transaction<Write>) -> anyhow::Result<()>;
}

/// Reader-side compatibility shim for the duration of a backfill.
///
/// Carries three associated types: [`DualReadAdapter::Legacy`] (the old
/// storage shape), [`DualReadAdapter::New`] (the new storage shape, what
/// writers produce and what the backfill rewrites legacy rows into), and
/// [`DualReadAdapter::View`] (the domain type callers consume, stable across
/// the migration window).
///
/// The two contract laws an implementation must satisfy:
///
/// 1. For any logically equivalent `(legacy, new)` pair,
///    `view_from_legacy(legacy)` equals `view_from_new(new)`.
/// 2. For any `legacy`,
///    `view_from_new(legacy_to_new(legacy))` equals `view_from_legacy(legacy)`.
///
/// Both laws are enforced by the
/// [`AdapterTest`](crate::testing::migration::AdapterTest) test trait.
pub trait DualReadAdapter: Send + Sync + 'static {
    type View;
    type Legacy;
    type New;

    fn view_from_legacy(legacy: Self::Legacy) -> Self::View;
    fn view_from_new(new: Self::New) -> Self::View;
    fn legacy_to_new(legacy: Self::Legacy) -> Self::New;
}

/// Builder-style inventory of all migrations the application registers.
///
/// Heterogeneous migrations land in the bucket matching their kind. After
/// every migration is registered, [`MigrationRegistry::validate`] enforces
/// global invariants:
///
/// - `MigrationMeta::name` is unique across all three buckets.
/// - `MigrationMeta::order` is unique within each bucket.
/// - Every name listed in a [`CleanupMigration::requires`] refers to a
///   registered deferred migration or backfill.
///
/// Validation is startup-time rather than compile-time because heterogeneous
/// trait objects preclude const evaluation across crates. A failure here
/// stops the node from starting, which is the desired outcome.
#[derive(Default)]
pub struct MigrationRegistry {
    deferred_schema: Vec<Box<dyn DeferredSchemaChange>>,
    backfills: Vec<Box<dyn DataBackfillErased>>,
    cleanups: Vec<Box<dyn CleanupMigration>>,
}

/// Object-safe view of [`DataBackfill`] used inside [`MigrationRegistry`].
///
/// Trait objects cannot carry an associated type, so the registry stores
/// backfills behind this erased trait. Reader code never sees this trait;
/// it references the concrete migration type and its `Adapter` directly.
/// The methods below are dead code until the deferred runner lands; the
/// runner invokes them when driving registered backfills.
#[async_trait]
#[allow(dead_code)]
trait DataBackfillErased: MigrationMeta {
    fn batch_size(&self) -> usize;
    async fn migrate_batch(
        &self,
        tx: &mut Transaction<Write>,
        offset: u64,
    ) -> anyhow::Result<Option<u64>>;
}

#[async_trait]
impl<T: DataBackfill> DataBackfillErased for T {
    fn batch_size(&self) -> usize {
        DataBackfill::batch_size(self)
    }
    async fn migrate_batch(
        &self,
        tx: &mut Transaction<Write>,
        offset: u64,
    ) -> anyhow::Result<Option<u64>> {
        DataBackfill::migrate_batch(self, tx, offset).await
    }
}

impl MigrationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deferred_schema<M: DeferredSchemaChange>(mut self, m: M) -> Self {
        self.deferred_schema.push(Box::new(m));
        self
    }

    pub fn backfill<M: DataBackfill>(mut self, m: M) -> Self {
        self.backfills.push(Box::new(m));
        self
    }

    pub fn cleanup<M: CleanupMigration>(mut self, m: M) -> Self {
        self.cleanups.push(Box::new(m));
        self
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        let mut all_names: HashSet<&'static str> = HashSet::new();
        let mut deferred_names: HashSet<&'static str> = HashSet::new();
        let mut deferred_orders: HashSet<u32> = HashSet::new();
        let mut backfill_orders: HashSet<u32> = HashSet::new();
        let mut cleanup_orders: HashSet<u32> = HashSet::new();

        for m in &self.deferred_schema {
            check_unique(
                "deferred_schema",
                m.name(),
                m.order(),
                &mut all_names,
                &mut deferred_orders,
            )?;
            deferred_names.insert(m.name());
        }
        for m in &self.backfills {
            check_unique(
                "backfill",
                m.name(),
                m.order(),
                &mut all_names,
                &mut backfill_orders,
            )?;
            deferred_names.insert(m.name());
        }
        for m in &self.cleanups {
            check_unique(
                "cleanup",
                m.name(),
                m.order(),
                &mut all_names,
                &mut cleanup_orders,
            )?;
        }

        for c in &self.cleanups {
            for required in c.requires() {
                if !deferred_names.contains(required) {
                    bail!(
                        "cleanup migration {} requires {} which is not a registered deferred or \
                         backfill migration",
                        c.name(),
                        required,
                    );
                }
            }
        }
        Ok(())
    }

    pub fn deferred_schema_migrations(&self) -> &[Box<dyn DeferredSchemaChange>] {
        &self.deferred_schema
    }

    pub fn cleanup_migrations(&self) -> &[Box<dyn CleanupMigration>] {
        &self.cleanups
    }
}

fn check_unique(
    bucket: &'static str,
    name: &'static str,
    order: u32,
    all_names: &mut HashSet<&'static str>,
    bucket_orders: &mut HashSet<u32>,
) -> anyhow::Result<()> {
    if !all_names.insert(name) {
        bail!("duplicate migration name {} (bucket {})", name, bucket);
    }
    if !bucket_orders.insert(order) {
        bail!("duplicate ORDER {} in bucket {}", order, bucket);
    }
    Ok(())
}

// TODO: wire the background runner that drives deferred work post-consensus
// alongside `SqlStorage::connect`. This module currently stops at the trait
// surface and the registry.

#[cfg(test)]
mod tests {
    use super::*;

    struct StubBackfill;
    impl MigrationMeta for StubBackfill {
        fn name(&self) -> &'static str {
            "stub_backfill"
        }
        fn order(&self) -> u32 {
            1
        }
    }
    struct StubAdapter;
    impl DualReadAdapter for StubAdapter {
        type View = ();
        type Legacy = ();
        type New = ();
        fn view_from_legacy(_: ()) {}
        fn view_from_new(_: ()) {}
        fn legacy_to_new(_: ()) {}
    }
    #[async_trait]
    impl DataBackfill for StubBackfill {
        type Adapter = StubAdapter;
        async fn migrate_batch(
            &self,
            _tx: &mut Transaction<Write>,
            _offset: u64,
        ) -> anyhow::Result<Option<u64>> {
            Ok(None)
        }
    }

    struct OkCleanup;
    impl MigrationMeta for OkCleanup {
        fn name(&self) -> &'static str {
            "ok_cleanup"
        }
        fn order(&self) -> u32 {
            2
        }
    }
    #[async_trait]
    impl CleanupMigration for OkCleanup {
        fn requires(&self) -> &'static [&'static str] {
            &["stub_backfill"]
        }
        async fn run(&self, _tx: &mut Transaction<Write>) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct DanglingCleanup;
    impl MigrationMeta for DanglingCleanup {
        fn name(&self) -> &'static str {
            "dangling_cleanup"
        }
        fn order(&self) -> u32 {
            3
        }
    }
    #[async_trait]
    impl CleanupMigration for DanglingCleanup {
        fn requires(&self) -> &'static [&'static str] {
            &["nonexistent"]
        }
        async fn run(&self, _tx: &mut Transaction<Write>) -> anyhow::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn empty_registry_is_valid() {
        MigrationRegistry::new().validate().unwrap();
    }

    #[test]
    fn duplicate_name_rejected() {
        struct A;
        impl MigrationMeta for A {
            fn name(&self) -> &'static str {
                "dup"
            }
            fn order(&self) -> u32 {
                1
            }
        }
        #[async_trait]
        impl DeferredSchemaChange for A {
            async fn run(&self, _: &SqlStorage) -> anyhow::Result<()> {
                Ok(())
            }
        }
        struct B;
        impl MigrationMeta for B {
            fn name(&self) -> &'static str {
                "dup"
            }
            fn order(&self) -> u32 {
                2
            }
        }
        #[async_trait]
        impl DeferredSchemaChange for B {
            async fn run(&self, _: &SqlStorage) -> anyhow::Result<()> {
                Ok(())
            }
        }
        let err = MigrationRegistry::new()
            .deferred_schema(A)
            .deferred_schema(B)
            .validate()
            .unwrap_err();
        assert!(err.to_string().contains("duplicate migration name"));
        assert!(err.to_string().contains("deferred_schema"));
    }

    #[test]
    fn duplicate_order_rejected() {
        struct A;
        impl MigrationMeta for A {
            fn name(&self) -> &'static str {
                "a"
            }
            fn order(&self) -> u32 {
                1
            }
        }
        #[async_trait]
        impl DeferredSchemaChange for A {
            async fn run(&self, _: &SqlStorage) -> anyhow::Result<()> {
                Ok(())
            }
        }
        struct B;
        impl MigrationMeta for B {
            fn name(&self) -> &'static str {
                "b"
            }
            fn order(&self) -> u32 {
                1
            }
        }
        #[async_trait]
        impl DeferredSchemaChange for B {
            async fn run(&self, _: &SqlStorage) -> anyhow::Result<()> {
                Ok(())
            }
        }
        let err = MigrationRegistry::new()
            .deferred_schema(A)
            .deferred_schema(B)
            .validate()
            .unwrap_err();
        assert!(err.to_string().contains("duplicate ORDER"));
    }

    #[test]
    fn missing_requires_rejected() {
        let err = MigrationRegistry::new()
            .cleanup(DanglingCleanup)
            .validate()
            .unwrap_err();
        assert!(err.to_string().contains("nonexistent"));
    }

    #[test]
    fn requires_resolved_against_backfill() {
        MigrationRegistry::new()
            .backfill(StubBackfill)
            .cleanup(OkCleanup)
            .validate()
            .unwrap();
    }
}
