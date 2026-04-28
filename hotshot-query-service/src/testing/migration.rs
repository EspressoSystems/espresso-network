//! Test trait extensions for migration implementations.
//!
//! Each test trait encodes invariants the corresponding production trait
//! cannot enforce on its own. Default-method assertions cover the
//! orchestration (drive batches, repeat runs, verify two adapter laws);
//! authors still write the bulk of the work — fixture seeding and the
//! shape-specific final-state checks.
//!
//! - [`AdapterTest`]: pure, no DB. Author supplies [`AdapterTest::equivalent_pair`].
//! - [`BackfillTest`]: needs a DB. Author supplies [`BackfillTest::seed_legacy`]
//!   and [`BackfillTest::assert_all_readable_as_new`]; the three assertion
//!   methods are default-implemented in terms of those two and `migrate_batch`.
//! - [`DeferredSchemaTest`]: needs a DB. The single assertion is
//!   default-implemented in terms of `self.run`.
//!
//! The design intent is captured in `doc/storage-migrations.md`.

use std::fmt::Debug;

use async_trait::async_trait;

use crate::{
    data_source::{Transaction as _, VersionedDataSource, storage::sql::SqlStorage},
    migration::{DataBackfill, DeferredSchemaChange, DualReadAdapter},
};

/// Test extension for [`DualReadAdapter`].
///
/// Pure: requires no database. Implementations supply
/// [`AdapterTest::equivalent_pair`] returning a logically equivalent
/// `(Legacy, New)` representation of the same domain value, and the default
/// methods verify the two adapter laws.
pub trait AdapterTest: DualReadAdapter
where
    Self::Legacy: Clone + Debug,
    Self::New: Clone + Debug,
    Self::View: PartialEq + Debug,
{
    /// A logically equivalent pair: the same domain value expressed in both
    /// storage representations.
    fn equivalent_pair() -> (Self::Legacy, Self::New);

    /// Law 1: both projections produce the same `View`.
    fn assert_views_match() {
        let (legacy, new) = Self::equivalent_pair();
        let via_legacy = Self::view_from_legacy(legacy);
        let via_new = Self::view_from_new(new);
        assert_eq!(
            via_legacy, via_new,
            "view_from_legacy and view_from_new disagree"
        );
    }

    /// Law 2: converting then reading equals reading directly.
    fn assert_conversion_roundtrip() {
        let (legacy, _) = Self::equivalent_pair();
        let direct = Self::view_from_legacy(legacy.clone());
        let converted = Self::view_from_new(Self::legacy_to_new(legacy));
        assert_eq!(
            direct, converted,
            "legacy_to_new is not view-preserving: view_from_legacy != \
             view_from_new(legacy_to_new)",
        );
    }

    /// Combined check; call this from a test.
    fn assert_adapter_laws() {
        Self::assert_views_match();
        Self::assert_conversion_roundtrip();
    }
}

/// Test extension for [`DataBackfill`].
///
/// Implementations supply two methods: [`BackfillTest::seed_legacy`]
/// populates the test database with `n` legacy-format rows, and
/// [`BackfillTest::assert_all_readable_as_new`] verifies that every seeded
/// row reads through the new path with the expected `View`. Both are
/// shape-specific and non-trivial. The three assertion methods are
/// default-implemented on top of them so authors avoid the batch driver
/// loop and transaction plumbing.
///
/// Note that [`BackfillTest::assert_resumable`] currently exercises only
/// the offset-passing path; it does not compare against an uninterrupted
/// baseline. A stronger resumability check arrives once the deferred
/// runner persists offsets through restarts.
#[async_trait]
pub trait BackfillTest: DataBackfill {
    /// Populate the test database with `n` legacy-format rows. Implementations
    /// should also clear any state left by a previous assertion run so
    /// successive calls are independent.
    async fn seed_legacy(&self, storage: &SqlStorage, n: usize) -> anyhow::Result<()>;

    /// Verify every seeded row is readable through the new path with the
    /// `View` produced by [`DualReadAdapter::view_from_legacy`].
    async fn assert_all_readable_as_new(&self, storage: &SqlStorage) -> anyhow::Result<()>;

    /// Drive the backfill to completion and verify the final state.
    async fn assert_runs_to_completion(
        &self,
        storage: &SqlStorage,
        n: usize,
    ) -> anyhow::Result<()> {
        self.seed_legacy(storage, n).await?;
        drive_to_completion(self, storage).await?;
        self.assert_all_readable_as_new(storage).await
    }

    /// Drive the backfill to completion twice; the second run must observe
    /// an already-completed state and not change the final result.
    async fn assert_idempotent(&self, storage: &SqlStorage, n: usize) -> anyhow::Result<()> {
        self.seed_legacy(storage, n).await?;
        drive_to_completion(self, storage).await?;
        drive_to_completion(self, storage).await?;
        self.assert_all_readable_as_new(storage).await
    }

    /// Drive a single batch, simulate a restart, then drive to completion.
    /// The final state must match a single uninterrupted run.
    async fn assert_resumable(&self, storage: &SqlStorage, n: usize) -> anyhow::Result<()> {
        self.seed_legacy(storage, n).await?;
        let mut tx = storage.write().await?;
        let _ = self.migrate_batch(&mut tx, 0).await?;
        tx.commit().await?;
        drive_to_completion(self, storage).await?;
        self.assert_all_readable_as_new(storage).await
    }
}

async fn drive_to_completion<B: BackfillTest + ?Sized>(
    backfill: &B,
    storage: &SqlStorage,
) -> anyhow::Result<()> {
    let mut offset = 0u64;
    loop {
        let mut tx = storage.write().await?;
        let next = backfill.migrate_batch(&mut tx, offset).await?;
        tx.commit().await?;
        match next {
            Some(new_offset) => offset = new_offset,
            None => return Ok(()),
        }
    }
}

/// Test extension for [`DeferredSchemaChange`].
///
/// One default-method assertion exercises idempotency: calling `run` twice
/// must not error. The migration's SQL must use `IF NOT EXISTS` (or
/// equivalent guards) for this to hold.
#[async_trait]
pub trait DeferredSchemaTest: DeferredSchemaChange {
    async fn assert_idempotent(&self, storage: &SqlStorage) -> anyhow::Result<()> {
        self.run(storage).await?;
        self.run(storage).await?;
        Ok(())
    }
}
