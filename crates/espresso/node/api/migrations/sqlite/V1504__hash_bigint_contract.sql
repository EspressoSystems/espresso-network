-- Contract phase counterpart to the postgres V1504. SQLite ids are already 64-bit, so V1501 here
-- was a pure rename with no backfill; undo it.

ALTER TABLE hash_bigint              RENAME TO hash;
ALTER TABLE fee_merkle_tree_bigint   RENAME TO fee_merkle_tree;
ALTER TABLE block_merkle_tree_bigint RENAME TO block_merkle_tree;
