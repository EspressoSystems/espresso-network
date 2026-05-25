-- Expand phase: create new BIGINT-keyed tables alongside the existing ones.
-- Old tables (hash, fee_merkle_tree, block_merkle_tree) are left untouched and
-- serve as the read fallback during the backfill window.
-- The contract phase (dropping old tables and renaming *_bigint to canonical names)
-- is a separate follow-up migration.

-- Drop reward merkle tree tables — unused and always empty across all deployments.
DROP TABLE reward_merkle_tree;
DROP TABLE reward_merkle_tree_v2;

-- New hash table with BIGSERIAL.
CREATE TABLE hash_bigint (
    id    BIGSERIAL PRIMARY KEY,
    value BYTEA NOT NULL UNIQUE
);

-- Seed the sequence above the old max so new auto-IDs never collide
-- with rows we backfill from hash (which preserve their original INT ids).
SELECT setval(
    pg_get_serial_sequence('hash_bigint', 'id'),
    GREATEST(COALESCE((SELECT MAX(id) FROM hash), 1), 1)
);

-- New Merkle-tree tables with BIGINT FK.
CREATE TABLE fee_merkle_tree_bigint (
    path            JSONB        NOT NULL,
    created         BIGINT       NOT NULL,
    hash_id         BIGINT       NOT NULL REFERENCES hash_bigint(id),
    children        JSONB,
    children_bitvec BIT VARYING,
    idx             JSONB,
    entry           JSONB,
    PRIMARY KEY (path, created)
);
CREATE INDEX fee_merkle_tree_bigint_created ON fee_merkle_tree_bigint (created);

CREATE TABLE block_merkle_tree_bigint (
    path            JSONB        NOT NULL,
    created         BIGINT       NOT NULL,
    hash_id         BIGINT       NOT NULL REFERENCES hash_bigint(id),
    children        JSONB,
    children_bitvec BIT VARYING,
    idx             JSONB,
    entry           JSONB,
    PRIMARY KEY (path, created)
);
CREATE INDEX block_merkle_tree_bigint_created ON block_merkle_tree_bigint (created);
