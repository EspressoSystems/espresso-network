CREATE TABLE reward_merkle_tree (
  path JSONB NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id JSONB NOT NULL,
  children JSONB, 
  children_bitvec BLOB, 
  idx JSONB, 
  entry JSONB,
  PRIMARY KEY (path)
);

ALTER TABLE header
ADD COLUMN reward_merkle_tree_root TEXT
GENERATED ALWAYS AS (json_extract(data, '$.fields.reward_merkle_tree_root')) STORED;
