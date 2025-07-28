
-- The new reward_merkle_tree table corresponds to `RewardMerkleTree` with keccak hashing algorithm,
-- and is used starting from protocol version V4.

CREATE TABLE reward_merkle_tree_v2 (
  path JSONB NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id INT NOT NULL REFERENCES hash (id), 
  children JSONB, 
  children_bitvec BIT(2), 
  idx JSONB, 
  entry JSONB
);

ALTER TABLE 
  reward_merkle_tree_v2
ADD 
  CONSTRAINT reward_merkle_tree_v2_pk  PRIMARY KEY (path, created);

CREATE INDEX reward_merkle_tree_v2_created ON reward_merkle_tree_v2 (created);