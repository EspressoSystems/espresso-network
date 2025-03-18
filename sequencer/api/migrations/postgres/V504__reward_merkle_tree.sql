CREATE TABLE reward_merkle_tree (
  path INTEGER[] NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id INT NOT NULL REFERENCES hash (id), 
  children INT[], 
  children_bitvec BIT(256), 
  index JSONB, 
  entry JSONB
);

ALTER TABLE 
  reward_merkle_tree 
ADD 
  CONSTRAINT reward_merkle_tree_pk  PRIMARY KEY (path, created);

CREATE INDEX reward_merkle_tree_created ON reward_merkle_tree (created);


ALTER TABLE header
ADD column reward_merkle_tree_root text
GENERATED ALWAYS AS (data->'fields'->>'reward_merkle_tree_root') STORED;