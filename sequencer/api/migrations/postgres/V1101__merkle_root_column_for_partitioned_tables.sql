ALTER TABLE header
    ADD column block_merkle_tree_root text
        GENERATED ALWAYS AS (coalesce(data->'fields'->>'block_merkle_tree_root', data->>'block_merkle_tree_root')) STORED NOT NULL,
    ADD column fee_merkle_tree_root text
        GENERATED ALWAYS AS (coalesce(data->'fields'->>'fee_merkle_tree_root', data->>'fee_merkle_tree_root')) STORED NOT NULL;
