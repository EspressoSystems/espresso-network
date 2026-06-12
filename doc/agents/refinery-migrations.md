# Refinery Migrations and Deferred Backfills

Guidance for AI coding agents (and humans) working on storage migrations in this repo.
Not loaded into the default agent context — read this file when adding or modifying
a storage migration.

## ⚠️ Refinery Migrations Block Startup — Avoid Large Data Operations

Refinery migrations run synchronously at node startup, before the node joins consensus. Any migration
that does significant data work (bulk inserts, table rewrites, large backfills) will delay or prevent
the node from participating in consensus, which is unacceptable in production.

**Rule: Refinery migrations must be fast and schema-only.** Safe operations: `CREATE TABLE`,
`CREATE INDEX CONCURRENTLY`, `ALTER TABLE ... ADD COLUMN` with a nullable/defaulted column, `DROP TABLE`.
Unsafe: any DML that touches a number of rows proportional to database size (`UPDATE`, `INSERT ... SELECT`,
`DELETE` across large tables).

## Deferred Backfill Pattern for Large Data Migrations

When a migration requires transforming or copying a significant amount of existing data, use the
`DataBackfill` trait (`hotshot-query-service/src/migration.rs`) instead of doing the work in Refinery.
This runs the data work in a background task after the node has started and joined consensus.

**When to use this pattern:** any migration where the amount of work is proportional to the size of the
existing database (e.g. copying rows to a new table, recomputing a column, reformatting data).

**How to implement a deferred backfill:**

1. **Add a Refinery migration** that creates the new destination table (schema only, no data).
2. **Update read paths** to check the new table first and fall back to the old table, so both partially-
   and fully-migrated states serve correct data.
3. **Update write paths** to write only to the new table going forward.
4. **Implement `DataBackfill`** in `crates/espresso/node/src/persistence/migrations.rs`:
   - `name()` must be globally unique and stable (it is persisted in `deferred_migrations`).
   - `run_batch()` receives a cursor `offset` and returns `Some(next_offset)` to continue or `None`
     when done. Use keyset pagination (not OFFSET) to avoid O(n²) query cost.
   - **Delete from the source table in the same transaction** as the insert into the destination, to
     avoid doubling storage during the migration.
   - Use `requires()` to declare dependencies on other backfills that must complete first.
5. **Register the migration** in `hash_bigint_migrations()` (or an equivalent registry function).
6. **Drop the old table** in a future Refinery migration once the backfill is confirmed complete and
   the read fallback is no longer needed.

**Progress is tracked** in the `deferred_migrations` table and exposed at `GET database/migration-status`.

**Storage caveat:** if the new table has FK constraints that require a lookup table to be fully populated
before any rows can be inserted, that lookup table will be doubled in storage for the duration of the
migration. This is unavoidable without dropping the FK (a schema change). Document the expected peak
storage increase in the PR when this applies.
