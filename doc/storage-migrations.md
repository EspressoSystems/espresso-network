# Storage Migrations

This document describes how to add new SQL storage migrations such that they do not block consensus startup.

## Why this exists

Until migration infrastructure was added in `hotshot_query_service::migration`, every refinery migration ran inside
`SqlStorage::connect()` before `SequencerContext::start_consensus()` was called. A migration that scanned a large table
or rebuilt a merkle tree would hold the node out of consensus for hours.

The architecture in this document keeps fast schema changes in the pre-consensus path and moves slow operations behind a
runner that executes after consensus is running. Consensus must remain fully functional while deferred migrations are in
progress, so any data-shape change ships with a dual-read adapter that keeps the reader correct against both the old and
new shapes for one release cycle.

This applies only to migrations introduced after the infrastructure landed. Existing refinery migrations are unchanged.

## Expand-migrate-contract

Any data-shape change in this system is split across (at least) two releases, following a pattern called
**expand-migrate-contract**. The motivation: at no point during the migration is the database in a state the running
code cannot read.

- **Expand** (release N): introduce the new shape alongside the old. Add the new table or column; do not remove anything
  yet. Writers start writing the new shape. Readers route through a dual-read adapter that returns the same domain value
  whether a given row is in the old shape or the new shape.
- **Migrate** (release N, background): a backfill rewrites every existing legacy row into the new shape. Runs as a
  deferred task while consensus continues; can take hours without blocking the node.
- **Contract** (release N+1): once the backfill has finished on every operator, a follow-up release drops the legacy
  columns or tables and removes the dual-read code path. The schema "contracts" back down.

The three phases map onto the migration system as follows: expand is a plain refinery SQL file (the same way every
existing schema change in this repo is authored), migrate is a `DataBackfill`, contract is a `CleanupMigration`.
`DeferredSchemaChange` is a separate kind that covers slow DDL (Data Definition Language, the SQL subset that creates or
alters schema objects: `CREATE TABLE`, `CREATE INDEX`, `ALTER`, `DROP`) — typically `CREATE INDEX CONCURRENTLY` — which
has no data-shape implication and needs no adapter.

### Why cleanup is a separate release

The cleanup phase ships in a _separate release_ from the expand and migrate phases. There are two reasons.

**Verification.** If `migrate_batch` returned `None` because of a backfill bug that silently skipped rows, the gap
between releases gives an operator time to sample the new columns and confirm completeness before the destructive step.
An auto-cleanup that fires at the end of the backfill makes a backfill bug silently destructive on every operator at
once, with no recovery path because the legacy data is gone.

**Rollback compatibility (forward-looking).** The repo does not support binary rollback today: refinery migrations are
up-only, there is no schema version negotiation between binary and DB, and no operator-facing rollback policy is
documented. Forward-only migrations make rollback impossible by construction.

This architecture is one of the pieces that has to be in place _before_ rollback can ever be supported. By keeping
legacy and new shapes coexisting for one release, the database stays readable by both the previous binary and the
current one. If we later add the rest of what rollback needs (schema-version checks, a documented downgrade window),
this design will already accommodate it. If we ship cleanup in the same release as expand, that future is closed off.

Concretely, the failure mode this prevents:

1. Operator upgrades to release N. Schema has both legacy and new columns.
2. Backfill runs for hours. Completes. Cleanup fires. Legacy columns dropped.
3. Operator hits a bug in N and tries to roll back to N-1.
4. N-1 only knows the legacy shape. Legacy columns are gone. The node will not start.

The two-release gap is what makes step 4 recoverable: in the version-N window, both shapes coexist, so any rollback
target that knows either shape works.

Mitigations exist (auto-cleanup with a configurable delay, per-migration opt-in, operator-triggered application) but
each shifts the risk rather than removing it. Until rollback is a supported operator capability and the verification gap
can be closed some other way, this document mandates the two-release pattern.

## The three migration kinds (plus refinery)

Fast pre-consensus DDL stays as a plain refinery SQL file in `migrations/{postgres,sqlite}/V{N}__name.sql`, the way
every existing schema change in the repo works today. Refinery already handles fast DDL well: SQL files, history table,
idempotency-by-version, embedded at compile time. The trait architecture below covers only the cases refinery cannot.

| Kind                   | Trait                  | Runs                       | Transactional | Use for                                                                                                                                |
| ---------------------- | ---------------------- | -------------------------- | ------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Fast pre-consensus DDL | (refinery SQL file)    | Pre-consensus, at startup  | Yes           | `CREATE TABLE`, nullable `ADD COLUMN`, small `ALTER`, `DROP`, `TRUNCATE` of small tables                                               |
| Deferred schema change | `DeferredSchemaChange` | Background, post-consensus | No            | Slow DDL: `CREATE INDEX CONCURRENTLY`, table rewrites without long locks                                                               |
| Data backfill          | `DataBackfill`         | Background, post-consensus | Per-batch     | Copying or transforming rows from a legacy shape into a new shape                                                                      |
| Cleanup migration      | `CleanupMigration`     | Pre-consensus, at startup  | Yes           | Dropping legacy columns or tables after a paired backfill has shipped to all operators (the contract phase of expand-migrate-contract) |

Pick the kind by what the migration physically does, not how long you think it will take. `CREATE INDEX` on a table that
is small today will block startup for hours once the table grows. The buckets exist so that the choice is explicit at
review time.

`CleanupMigration` is its own kind rather than a refinery file because the runner enforces its `REQUIRES` precondition
against the `deferred_migrations` table; refinery has no such hook.

## Authoring rules

A migration that does not satisfy these rules will leave the node either unable to start or unable to serve consensus
correctly during the migration window.

### Rules for `DeferredSchemaChange`

- Must run outside any transaction. `CREATE INDEX CONCURRENTLY` and similar operations forbid wrapping transactions.
- Must be idempotent. The runner may invoke `run` repeatedly across restarts. Use `IF NOT EXISTS` and similar guards on
  every statement. Completion is tracked by the runner in `deferred_migrations`; there is no separate schema-level
  applied check.
- For SQLite (where `CONCURRENTLY` is unsupported) the implementation may emit a plain `CREATE INDEX`. Authors implement
  two `impl` blocks behind `#[cfg(feature = "embedded-db")]` when the SQL differs.

### Rules for `DataBackfill`

- Must be batched. The trait carries a `batch_size()` method (default 1000).
- Must be resumable. Each call to `migrate_batch(offset)` returns the next offset, or `None` when there is no more work.
  The runner persists the offset in `deferred_migrations` between batches.
- Must be idempotent. A batch reapplied at the same offset must produce the same state.
- Must declare a `DualReadAdapter` (associated type `Adapter`). Reader code routes through the adapter while the
  backfill is in progress.

### Rules for `CleanupMigration`

- Must declare `requires()`, returning the names of the deferred migrations and backfills it cleans up after. Only data
  dependencies belong here; index builds and other performance-only migrations do not.
- The runner refuses to run a cleanup until every required name is marked completed in `deferred_migrations`. This
  protects an operator who upgrades across two releases without waiting for backfills to finish.
- Must ship in a release strictly after the release that introduced the paired backfill, and only once telemetry
  confirms the backfill has completed on every operator. There is no automation for this; it is a release-management
  decision.

## The dual-read adapter contract

A `DataBackfill` exists because the storage shape of some data changed. While the backfill is in progress, the database
contains a mixture of legacy rows (unmigrated) and new rows (migrated or freshly written). Reader code must produce
identical results regardless of which shape a row is in.

This is the role of `DualReadAdapter`. It carries three associated types:

- `Legacy`: the row as written before the migration.
- `New`: the row as written after the migration (and into which the backfill rewrites legacy rows).
- `View`: the domain type that callers consume. Stable across the migration window.

And three functions:

- `view_from_legacy(legacy) -> view`: project a legacy row into the domain type.
- `view_from_new(new) -> view`: project a new row into the domain type.
- `legacy_to_new(legacy) -> new`: convert in one direction. The backfill calls this. Writers always produce `New`
  directly.

The contract on these functions is:

1. For any logically equivalent `(legacy, new)` pair, `view_from_legacy(legacy)` equals `view_from_new(new)`.
2. For any `legacy`, `view_from_new(legacy_to_new(legacy))` equals `view_from_legacy(legacy)`.

The test traits in `hotshot_query_service::testing::migration` enforce both.

Once a backfill is marked complete in `deferred_migrations` on every operator (checked by release management, not the
runner), a follow-up release may drop the legacy path: remove the `view_from_legacy` callsite, ship a `CleanupMigration`
that drops the legacy columns or tables, and remove the adapter. This is the contract phase of expand-migrate-contract.

## Testing

Every production trait has a paired test trait under `testing::migration`. The test traits provide default-method
assertions for invariants the production traits cannot enforce on their own.

| Production trait       | Test trait           | Default-method invariants                             |
| ---------------------- | -------------------- | ----------------------------------------------------- |
| `DualReadAdapter`      | `AdapterTest`        | view equivalence, conversion roundtrip                |
| `DataBackfill`         | `BackfillTest`       | runs to completion, idempotent, resumable from offset |
| `DeferredSchemaChange` | `DeferredSchemaTest` | not applied before run, applied after run, idempotent |

Implementing a test trait requires only a small amount of boilerplate per migration: an `equivalent_pair()` for
`AdapterTest`, plus a `seed_legacy` and `assert_all_readable_as_new` for `BackfillTest`. The default methods do the
rest.

Run the tests with `cargo nextest run -p espresso-node persistence::migrations` or the equivalent for whichever crate
hosts the migration.

## The `deferred_migrations` table

The runner persists state for deferred and backfill migrations in a single table:

```sql
CREATE TABLE deferred_migrations (
    name         TEXT PRIMARY KEY,
    started_at   TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    error        TEXT,
    last_offset  BIGINT
);
```

- `name`: the migration's `NAME` constant.
- `started_at`: set on first invocation.
- `completed_at`: set when `migrate_batch` returns `None` or `is_applied` returns true.
- `error`: most recent error string, cleared on success. Surfaced via metrics and the `/status/migrations` endpoint.
- `last_offset`: backfill progress. `NULL` for `DeferredSchemaChange`.

This table is created by a refinery SQL file shipped alongside the runner (at
`migrations/{postgres,sqlite}/V{N}__deferred_migrations.sql`); it must be present before any deferred work runs.

## Operator visibility (planned)

The runner is planned to expose:

- Metrics: per-migration gauge for state (`pending`, `running`, `completed`, `errored`) and counter for batches
  processed.
- HTTP: `GET /status/migrations` returns the rows of `deferred_migrations` plus the static `description()` for each
  registered migration.
- Logs: each batch logs its offset and rows processed.

These hooks land alongside the deferred runner. Until then, operators inspect the `deferred_migrations` table directly
to know when it is safe to upgrade past a contract migration.

## Worked example

A runnable worked example lives at `hotshot-query-service/examples/migration_traits.rs`. It exercises every production
trait and every test trait against a real Postgres database (via the existing `TmpDb` test helper), using a fictional
`scores` table whose storage shape changes from a raw byte blob to a structured JSON value. Run it with:

```text
cargo run -p hotshot-query-service --example migration_traits \
    --features "sql-data-source,testing"
```
