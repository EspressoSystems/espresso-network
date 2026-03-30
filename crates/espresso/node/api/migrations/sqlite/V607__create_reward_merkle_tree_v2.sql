
-- The new reward_merkle_tree table corresponds to `RewardMerkleTreeV2` with keccak hashing algorithm,
-- and is used starting from protocol version V4.

CREATE TABLE reward_merkle_tree_v2 (
  path JSONB NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id INT NOT NULL REFERENCES hash (id), 
  children JSONB, 
  children_bitvec BLOB, 
  idx JSONB, 
  entry JSONB,
  PRIMARY KEY (path, created)
);