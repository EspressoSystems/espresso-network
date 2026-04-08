DROP TABLE reward_merkle_tree_v2;
DROP TABLE reward_merkle_tree;

-- For Postgres, we execute this migration conditionally only if the `reward_merkle_tree_v2_data`
-- has already happened, allowing that migration to drop the old tables if it has yet to happen.
-- For SQLite, procedural if/then migrations are not supported, so we can't do this. Instead, we
-- unconditionally drop the old tables and mark the `reward_merkle_tree_v2_data` migration as
-- completed, effectively skipping it if it has not run yet. This should be fine as SQLite nodes
-- should not generally be storing full reward state anyways.
INSERT INTO epoch_migration (table_name, completed) VALUES ('reward_merkle_tree_v2_data', TRUE)
ON CONFLICT DO UPDATE SET completed = TRUE;
