-- Rename tables to *_bigint for consistency with the postgres schema.
-- SQLite integers are already 64-bit so this is a naming-only change.
ALTER TABLE hash              RENAME TO hash_bigint;
ALTER TABLE fee_merkle_tree   RENAME TO fee_merkle_tree_bigint;
ALTER TABLE block_merkle_tree RENAME TO block_merkle_tree_bigint;

-- Drop reward merkle tree tables — unused and always empty across all deployments.
DROP TABLE IF EXISTS reward_merkle_tree;
DROP TABLE IF EXISTS reward_merkle_tree_v2;
