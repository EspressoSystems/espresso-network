-- The generated columns for header merkle roots were originally created by extracting fields
-- `block_merkle_tree_root` and `fee_merkle_tree_root` from the header JSON. Post 0.1, though, the
-- header serialization changed so that these fields are now nested one level deeper:
-- `fields.block_merkle_tree_root` and `fields.fee_merkle_tree_root`. This migration alters the
-- generated column expression to use NULL coalescing to extract the value from either of these
-- paths depending on which version of the header we have.
--
-- Pre 17.x (we target Postgres >= 16.x), there is not explicit instruction for changing the
-- expression of a generated column, so the best we can do is drop and re-add the column with a
-- different expression.

ALTER TABLE header
    DROP column block_merkle_tree_root,
    ADD column block_merkle_tree_root text
        GENERATED ALWAYS AS (coalesce(data->'fields'->>'block_merkle_tree_root', data->>'block_merkle_tree_root')) STORED NOT NULL,
    DROP column fee_merkle_tree_root,
    ADD column fee_merkle_tree_root text
        GENERATED ALWAYS AS (coalesce(data->'fields'->>'fee_merkle_tree_root', data->>'fee_merkle_tree_root')) STORED NOT NULL;
