-- Used by: get_all_reward_accounts() in sequencer/src/api/sql.rs

CREATE INDEX reward_merkle_tree_v2_idx_created
ON reward_merkle_tree_v2 (idx, created DESC);
