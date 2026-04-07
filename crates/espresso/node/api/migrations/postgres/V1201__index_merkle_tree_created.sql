CREATE INDEX IF NOT EXISTS block_merkle_tree_created ON block_merkle_tree (created);
CREATE INDEX IF NOT EXISTS fee_merkle_tree_created ON fee_merkle_tree (created);

-- These merkle trees are no longer used.
DROP TABLE reward_merkle_tree;
DROP TABLE reward_merkle_tree_v2;
