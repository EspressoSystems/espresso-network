-- Rename the existing reward_merkle_tree table to reward_merkle_tree_legacy
-- and create a new reward_merkle_tree table for the updated merkle tree.
--
-- The renamed table corresponds to the `RewardMerkleTreeLegacy` type,
-- which is used in protocol version V3.
--
-- The new reward_merkle_tree table corresponds to `RewardMerkleTree`,
-- and is used starting from protocol version V4.
ALTER TABLE reward_merkle_tree
RENAME TO reward_merkle_tree_legacy;


ALTER TABLE reward_merkle_tree_legacy
RENAME CONSTRAINT reward_merkle_tree_pk TO reward_merkle_tree_legacy_pk;

ALTER INDEX reward_merkle_tree_created
RENAME TO reward_merkle_tree_legacy_created;


CREATE TABLE reward_merkle_tree (
  path JSONB NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id INT NOT NULL REFERENCES hash (id), 
  children JSONB, 
  children_bitvec BIT(2), 
  idx JSONB, 
  entry JSONB
);

ALTER TABLE 
  reward_merkle_tree 
ADD 
  CONSTRAINT reward_merkle_tree_pk  PRIMARY KEY (path, created);

CREATE INDEX reward_merkle_tree_created ON reward_merkle_tree (created);