CREATE TABLE fee_merkle_tree (
  path INTEGER[] NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id BYTEA NOT NULL, 
  children BYTEA[256], 
  children_bitvec BIT(256), 
  index JSONB, 
  entry JSONB
);

ALTER TABLE 
  fee_merkle_tree 
ADD 
  CONSTRAINT fee_merkle_tree_pk PRIMARY KEY (path, created);

CREATE INDEX fee_merkle_tree_created ON fee_merkle_tree (created);

CREATE TABLE block_merkle_tree (
  path INTEGER[] NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id BYTEA NOT NULL, 
  children BYTEA[3], 
  children_bitvec BIT(3), 
  index JSONB, 
  entry JSONB
);

ALTER TABLE 
  block_merkle_tree 
ADD 
  CONSTRAINT block_merkle_tree_pk PRIMARY KEY (path, created);

CREATE INDEX block_merkle_tree_created ON block_merkle_tree (created);
