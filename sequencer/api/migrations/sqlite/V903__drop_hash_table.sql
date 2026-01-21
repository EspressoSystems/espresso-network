-- Migrate merkle tree tables from using hash table references to storing hashes directly.
-- SQLite doesn't support ALTER COLUMN TYPE, so we recreate tables.

-- fee_merkle_tree
CREATE TABLE fee_merkle_tree_new (
  path JSONB NOT NULL,
  created BIGINT NOT NULL,
  hash_id JSONB NOT NULL,
  children JSONB,
  children_bitvec BLOB,
  idx JSONB,
  entry JSONB,
  PRIMARY KEY (path)
);

INSERT INTO fee_merkle_tree_new (path, created, hash_id, children, children_bitvec, idx, entry)
SELECT
  f.path,
  f.created,
  h.value,
  f.children,
  f.children_bitvec,
  f.idx,
  f.entry
FROM fee_merkle_tree f
LEFT JOIN hash h ON h.id = f.hash_id;

DROP TABLE fee_merkle_tree;
ALTER TABLE fee_merkle_tree_new RENAME TO fee_merkle_tree;

-- block_merkle_tree
CREATE TABLE block_merkle_tree_new (
  path JSONB NOT NULL,
  created BIGINT NOT NULL,
  hash_id JSONB,
  children JSONB,
  children_bitvec BLOB,
  idx JSONB,
  entry JSONB,
  PRIMARY KEY (path)
);

INSERT INTO block_merkle_tree_new (path, created, hash_id, children, children_bitvec, idx, entry)
SELECT
  b.path,
  b.created,
  h.value,
  b.children,
  b.children_bitvec,
  b.idx,
  b.entry
FROM block_merkle_tree b
LEFT JOIN hash h ON h.id = b.hash_id;

DROP TABLE block_merkle_tree;
ALTER TABLE block_merkle_tree_new RENAME TO block_merkle_tree;

-- reward_merkle_tree: no longer used (replaced by reward_merkle_tree_v2), just drop it
DROP TABLE IF EXISTS reward_merkle_tree;

-- reward_merkle_tree_v2: migrate hash_id from INT reference to JSONB value
CREATE TABLE reward_merkle_tree_v2_new (
  path JSONB NOT NULL,
  created BIGINT NOT NULL,
  hash_id JSONB NOT NULL,
  children JSONB,
  children_bitvec BLOB,
  idx JSONB,
  entry JSONB,
  PRIMARY KEY (path)
);

INSERT INTO reward_merkle_tree_v2_new (path, created, hash_id, children, children_bitvec, idx, entry)
SELECT
  r.path,
  r.created,
  h.value,
  r.children,
  r.children_bitvec,
  r.idx,
  r.entry
FROM reward_merkle_tree_v2 r
LEFT JOIN hash h ON h.id = r.hash_id;

DROP TABLE reward_merkle_tree_v2;
ALTER TABLE reward_merkle_tree_v2_new RENAME TO reward_merkle_tree_v2;

-- Drop the hash table
DROP TABLE hash;
