ALTER TABLE header
ADD column block_merkle_tree_root text
GENERATED ALWAYS AS (data->>'block_merkle_tree_root') STORED NOT NULL;

ALTER TABLE header
ADD column fee_merkle_tree_root text
GENERATED ALWAYS as (data->>'fee_merkle_tree_root') STORED NOT NULL;

CREATE INDEX header_block_merkle_tree_root_idx ON header (block_merkle_tree_root);
CREATE INDEX header_fee_merkle_tree_root_idx ON header (fee_merkle_tree_root);