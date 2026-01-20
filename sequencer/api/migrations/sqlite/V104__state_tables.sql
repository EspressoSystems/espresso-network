CREATE TABLE fee_merkle_tree (
  path JSONB NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id JSONB NOT NULL, 
  children JSONB, 
  children_bitvec BLOB, 
  idx JSONB, 
  entry JSONB,
  PRIMARY KEY (path)
);

CREATE TABLE block_merkle_tree (
  path JSONB NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id JSONB, 
  children JSONB, 
  children_bitvec BLOB, 
  idx JSONB, 
  entry JSONB,
  PRIMARY KEY (path)
);
