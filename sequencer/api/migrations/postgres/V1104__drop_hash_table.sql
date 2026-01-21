-- Migrate merkle tree tables from using hash table references to storing hashes directly.
-- This allows us to drop the hash table which was a performance bottleneck.
-- Note: V302 already converted children from INT[] to JSONB (array of hash IDs as JSON).
-- We keep children as JSONB but store the actual hash values as JSON arrays of numbers
-- (matching serde_json's serialization of Vec<Vec<u8>>).

-- Helper function to convert bytea to JSON array of integers (matching serde_json format)
CREATE OR REPLACE FUNCTION bytea_to_json_array(b bytea) RETURNS jsonb AS $$
DECLARE
  result jsonb := '[]'::jsonb;
  i int;
BEGIN
  FOR i IN 0..length(b)-1 LOOP
    result := result || to_jsonb(get_byte(b, i));
  END LOOP;
  RETURN result;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- fee_merkle_tree: migrate hash_id to BYTEA, update children JSONB to contain actual hashes
ALTER TABLE fee_merkle_tree
  DROP CONSTRAINT IF EXISTS fee_merkle_tree_hash_id_fkey,
  ADD COLUMN hash_id_new BYTEA;

UPDATE fee_merkle_tree f
SET hash_id_new = h.value
FROM hash h
WHERE h.id = f.hash_id;

-- Update children JSONB to contain actual hash values as JSON arrays of numbers
UPDATE fee_merkle_tree f
SET children = (
  SELECT jsonb_agg(bytea_to_json_array(h.value) ORDER BY ordinality)
  FROM jsonb_array_elements_text(f.children) WITH ORDINALITY AS elem(id, ordinality)
  JOIN hash h ON h.id = elem.id::int
)
WHERE f.children IS NOT NULL AND f.children != 'null'::jsonb;

ALTER TABLE fee_merkle_tree
  DROP COLUMN hash_id;

ALTER TABLE fee_merkle_tree
  RENAME COLUMN hash_id_new TO hash_id;

ALTER TABLE fee_merkle_tree
  ALTER COLUMN hash_id SET NOT NULL;

-- block_merkle_tree: migrate hash_id to BYTEA, update children JSONB to contain actual hashes
ALTER TABLE block_merkle_tree
  DROP CONSTRAINT IF EXISTS block_merkle_tree_hash_id_fkey,
  ADD COLUMN hash_id_new BYTEA;

UPDATE block_merkle_tree b
SET hash_id_new = h.value
FROM hash h
WHERE h.id = b.hash_id;

-- Update children JSONB to contain actual hash values as JSON arrays of numbers
UPDATE block_merkle_tree b
SET children = (
  SELECT jsonb_agg(bytea_to_json_array(h.value) ORDER BY ordinality)
  FROM jsonb_array_elements_text(b.children) WITH ORDINALITY AS elem(id, ordinality)
  JOIN hash h ON h.id = elem.id::int
)
WHERE b.children IS NOT NULL AND b.children != 'null'::jsonb;

ALTER TABLE block_merkle_tree
  DROP COLUMN hash_id;

ALTER TABLE block_merkle_tree
  RENAME COLUMN hash_id_new TO hash_id;

ALTER TABLE block_merkle_tree
  ALTER COLUMN hash_id SET NOT NULL;

-- reward_merkle_tree: no longer used (replaced by reward_merkle_tree_v2), just drop it
DROP TABLE IF EXISTS reward_merkle_tree;

-- reward_merkle_tree_v2: migrate hash_id to BYTEA, update children JSONB to contain actual hashes
ALTER TABLE reward_merkle_tree_v2
  DROP CONSTRAINT IF EXISTS reward_merkle_tree_v2_hash_id_fkey,
  ADD COLUMN hash_id_new BYTEA;

UPDATE reward_merkle_tree_v2 r
SET hash_id_new = h.value
FROM hash h
WHERE h.id = r.hash_id;

-- Update children JSONB to contain actual hash values as JSON arrays of numbers
UPDATE reward_merkle_tree_v2 r
SET children = (
  SELECT jsonb_agg(bytea_to_json_array(h.value) ORDER BY ordinality)
  FROM jsonb_array_elements_text(r.children) WITH ORDINALITY AS elem(id, ordinality)
  JOIN hash h ON h.id = elem.id::int
)
WHERE r.children IS NOT NULL AND r.children != 'null'::jsonb;

ALTER TABLE reward_merkle_tree_v2
  DROP COLUMN hash_id;

ALTER TABLE reward_merkle_tree_v2
  RENAME COLUMN hash_id_new TO hash_id;

ALTER TABLE reward_merkle_tree_v2
  ALTER COLUMN hash_id SET NOT NULL;

-- Drop the hash table now that nothing references it
DROP TABLE hash;

-- Clean up helper function
DROP FUNCTION bytea_to_json_array(bytea);
