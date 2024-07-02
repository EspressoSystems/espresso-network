CREATE TABLE IF NOT EXISTS hash (
  id SERIAL PRIMARY KEY, value BYTEA NOT NULL UNIQUE
);

CREATE TABLE fee_merkle_tree (
  pos LTREE NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id INT NOT NULL REFERENCES hash (id), 
  children INT[], 
  children_bitvec BIT(256), 
  index JSONB, 
  entry JSONB
);

ALTER TABLE 
  fee_merkle_tree 
ADD 
  CONSTRAINT fee_merkle_tree_pk PRIMARY KEY (pos, created);

CREATE INDEX fee_merkle_tree_path ON fee_merkle_tree USING GIST (pos);

CREATE TABLE block_merkle_tree (
  pos LTREE NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id INT NOT NULL REFERENCES hash (id), 
  children INT[], 
  children_bitvec BIT(3), 
  index JSONB, 
  entry JSONB
);

ALTER TABLE 
  block_merkle_tree 
ADD 
  CONSTRAINT block_merkle_tree_pk PRIMARY KEY (pos, created);

CREATE INDEX block_merkle_tree_path ON block_merkle_tree USING GIST (pos);
