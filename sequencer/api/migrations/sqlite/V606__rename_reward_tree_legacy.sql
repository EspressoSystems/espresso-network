-- Rename the table
ALTER TABLE reward_merkle_tree
RENAME TO reward_merkle_tree_legacy;


CREATE TABLE reward_merkle_tree (
  path JSONB NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id INT NOT NULL REFERENCES hash (id), 
  children JSONB, 
  children_bitvec BLOB, 
  idx JSONB, 
  entry JSONB,
  PRIMARY KEY (path, created)
);