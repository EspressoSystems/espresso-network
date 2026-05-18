-- Expand phase: rename old tables to _legacy, recreate with BIGINT keys.
-- This is an online schema change; the contract phase (dropping _legacy tables)
-- is a separate follow-up migration.

-- Step 1: rename tables that have FK deps on hash first.
ALTER TABLE fee_merkle_tree       RENAME TO fee_merkle_tree_legacy;
ALTER TABLE block_merkle_tree     RENAME TO block_merkle_tree_legacy;
ALTER TABLE reward_merkle_tree    RENAME TO reward_merkle_tree_legacy;
ALTER TABLE reward_merkle_tree_v2 RENAME TO reward_merkle_tree_v2_legacy;
ALTER TABLE hash                  RENAME TO hash_legacy;

-- Step 1b: rename the indexes that survived the table rename so they don't
-- collide with the identically-named indexes we create in Step 4.
-- PostgreSQL does NOT automatically rename indexes when a table is renamed.
ALTER INDEX fee_merkle_tree_created         RENAME TO fee_merkle_tree_legacy_created;
ALTER INDEX block_merkle_tree_created       RENAME TO block_merkle_tree_legacy_created;
ALTER INDEX reward_merkle_tree_created      RENAME TO reward_merkle_tree_legacy_created;
ALTER INDEX reward_merkle_tree_v2_created   RENAME TO reward_merkle_tree_v2_legacy_created;

-- Step 2: new hash table with BIGSERIAL.
CREATE TABLE hash (
    id    BIGSERIAL PRIMARY KEY,
    value BYTEA NOT NULL UNIQUE
);

-- Step 3: seed the sequence above the old max so new auto-IDs never collide
-- with rows we backfill from hash_legacy (which preserve their original INT ids).
SELECT setval(
    pg_get_serial_sequence('hash', 'id'),
    GREATEST(COALESCE((SELECT MAX(id) FROM hash_legacy), 1), 1)
);

-- Step 4: recreate Merkle-tree tables with BIGINT FK.
CREATE TABLE fee_merkle_tree (
    path            JSONB        NOT NULL,
    created         BIGINT       NOT NULL,
    hash_id         BIGINT       NOT NULL REFERENCES hash(id),
    children        JSONB,
    children_bitvec BIT VARYING,
    idx             JSONB,
    entry           JSONB,
    PRIMARY KEY (path, created)
);
CREATE INDEX fee_merkle_tree_created ON fee_merkle_tree (created);

CREATE TABLE block_merkle_tree (
    path            JSONB        NOT NULL,
    created         BIGINT       NOT NULL,
    hash_id         BIGINT       NOT NULL REFERENCES hash(id),
    children        JSONB,
    children_bitvec BIT VARYING,
    idx             JSONB,
    entry           JSONB,
    PRIMARY KEY (path, created)
);
CREATE INDEX block_merkle_tree_created ON block_merkle_tree (created);

CREATE TABLE reward_merkle_tree (
    path            JSONB        NOT NULL,
    created         BIGINT       NOT NULL,
    hash_id         BIGINT       NOT NULL REFERENCES hash(id),
    children        JSONB,
    children_bitvec BIT VARYING,
    idx             JSONB,
    entry           JSONB,
    PRIMARY KEY (path, created)
);
CREATE INDEX reward_merkle_tree_created ON reward_merkle_tree (created);

CREATE TABLE reward_merkle_tree_v2 (
    path            JSONB        NOT NULL,
    created         BIGINT       NOT NULL,
    hash_id         BIGINT       NOT NULL REFERENCES hash(id),
    children        JSONB,
    children_bitvec BIT VARYING,
    idx             JSONB,
    entry           JSONB,
    PRIMARY KEY (path, created)
);
CREATE INDEX reward_merkle_tree_v2_created ON reward_merkle_tree_v2 (created);
