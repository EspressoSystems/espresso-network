CREATE TABLE IF NOT EXISTS hash (
  id INTEGER PRIMARY KEY AUTOINCREMENT, value JSONB NOT NULL UNIQUE
);

CREATE TABLE fee_merkle_tree (
  path JSONB NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id INT NOT NULL REFERENCES hash (id), 
  children JSONB, 
  children_bitvec BLOB, 
  idx JSONB, 
  entry JSONB,
  PRIMARY KEY (path, created)
);

CREATE TABLE block_merkle_tree (
  path JSONB NOT NULL, 
  created BIGINT NOT NULL, 
  hash_id INT NOT NULL REFERENCES hash (id), 
  children JSONB, 
  children_bitvec BLOB, 
  idx JSONB, 
  entry JSONB,
  PRIMARY KEY (path, created)
);
