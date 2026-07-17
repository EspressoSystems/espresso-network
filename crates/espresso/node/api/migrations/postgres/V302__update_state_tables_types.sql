
ALTER TABLE fee_merkle_tree
  ALTER COLUMN path TYPE JSONB USING array_to_json(path),
  ALTER COLUMN children TYPE JSONB USING array_to_json(children);

 ALTER TABLE fee_merkle_tree RENAME COLUMN index TO idx;


ALTER TABLE block_merkle_tree
  ALTER COLUMN path TYPE JSONB USING array_to_json(path),
  ALTER COLUMN children TYPE JSONB USING array_to_json(children);

ALTER TABLE block_merkle_tree RENAME COLUMN index TO idx;