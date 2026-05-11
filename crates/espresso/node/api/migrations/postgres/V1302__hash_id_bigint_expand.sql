-- Expand phase for the hash.id INT -> BIGINT migration.
--
-- All statements here are O(1) catalog-only DDL (no table rewrites). The node
-- starts immediately after this migration completes. New hash inserts receive a
-- BIGINT id from hash_id_big_seq; new merkle rows store it in hash_id_big. Old
-- rows are left unchanged until the DataBackfill fills in the new columns.
--
-- We intentionally keep hash.id as the primary key (and its index) so that old-row
-- lookups during the migration window remain fast. New inserts use a sentinel
-- sequence (negative INT values) for id, which never conflict with the positive
-- legacy IDs (1..2147483647).

-- 1. Add new BIGINT column + sequence to hash (no table rewrite).
ALTER TABLE hash ADD COLUMN id_big BIGINT;
CREATE SEQUENCE hash_id_big_seq AS BIGINT;
-- setval(seq, N) sets the last-returned value to N, so the next nextval() returns N+1.
-- MAX(id) = 2147483647 (i32::MAX) on an exhausted table, so new IDs start at 2147483648.
-- The backfill sets id_big = id (1..2147483647), so the two ranges never overlap.
SELECT setval('hash_id_big_seq', COALESCE((SELECT MAX(id) FROM hash), 0));
ALTER TABLE hash ALTER COLUMN id_big SET DEFAULT nextval('hash_id_big_seq');

-- 2. Replace the exhausted INT sequence for id with a sentinel sequence.
--    Sentinel values are always negative; new inserts that omit id receive a unique
--    negative value. The PK constraint and its index are preserved.
CREATE SEQUENCE hash_id_sentinel_seq AS INT
    INCREMENT -1
    MINVALUE -2147483648
    MAXVALUE -1
    START WITH -1
    NO CYCLE;
ALTER TABLE hash ALTER COLUMN id DROP DEFAULT;
ALTER TABLE hash ALTER COLUMN id SET DEFAULT nextval('hash_id_sentinel_seq');

-- 3. Drop FK constraints (new merkle rows reference hash via hash_id_big, not hash.id).
ALTER TABLE fee_merkle_tree       DROP CONSTRAINT fee_merkle_tree_hash_id_fkey;
ALTER TABLE block_merkle_tree     DROP CONSTRAINT block_merkle_tree_hash_id_fkey;
ALTER TABLE reward_merkle_tree    DROP CONSTRAINT reward_merkle_tree_hash_id_fkey;
ALTER TABLE reward_merkle_tree_v2 DROP CONSTRAINT reward_merkle_tree_v2_hash_id_fkey;

-- 4. Add new nullable BIGINT columns to all four merkle tree tables.
ALTER TABLE fee_merkle_tree       ADD COLUMN hash_id_big BIGINT;
ALTER TABLE block_merkle_tree     ADD COLUMN hash_id_big BIGINT;
ALTER TABLE reward_merkle_tree    ADD COLUMN hash_id_big BIGINT;
ALTER TABLE reward_merkle_tree_v2 ADD COLUMN hash_id_big BIGINT;
