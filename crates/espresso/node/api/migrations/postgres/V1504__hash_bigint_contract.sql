-- Contract phase of the bigint hash-id migration: V1501 created BIGINT-keyed `*_bigint` copies of
-- these tables and a background backfill moved the rows across. Drop the INT-keyed originals and
-- rename the `*_bigint` tables back onto the original names. Assumes the backfill completed
-- everywhere; rows still in the dropped tables are lost.

DROP TABLE fee_merkle_tree, block_merkle_tree, hash;

ALTER TABLE hash_bigint              RENAME TO hash;
ALTER TABLE fee_merkle_tree_bigint   RENAME TO fee_merkle_tree;
ALTER TABLE block_merkle_tree_bigint RENAME TO block_merkle_tree;

ALTER INDEX fee_merkle_tree_bigint_created   RENAME TO fee_merkle_tree_created;
ALTER INDEX block_merkle_tree_bigint_created RENAME TO block_merkle_tree_created;

ALTER SEQUENCE hash_bigint_id_seq RENAME TO hash_id_seq;

ALTER TABLE hash RENAME CONSTRAINT hash_bigint_pkey TO hash_pkey;
ALTER TABLE hash RENAME CONSTRAINT hash_bigint_value_key TO hash_value_key;
ALTER TABLE fee_merkle_tree
    RENAME CONSTRAINT fee_merkle_tree_bigint_pkey TO fee_merkle_tree_pkey;
ALTER TABLE fee_merkle_tree
    RENAME CONSTRAINT fee_merkle_tree_bigint_hash_id_fkey TO fee_merkle_tree_hash_id_fkey;
ALTER TABLE block_merkle_tree
    RENAME CONSTRAINT block_merkle_tree_bigint_pkey TO block_merkle_tree_pkey;
ALTER TABLE block_merkle_tree
    RENAME CONSTRAINT block_merkle_tree_bigint_hash_id_fkey TO block_merkle_tree_hash_id_fkey;
