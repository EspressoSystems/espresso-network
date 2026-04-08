DO
$$
BEGIN
   IF EXISTS (SELECT FROM epoch_migration WHERE table_name = 'reward_merkle_tree_v2_data' AND completed) THEN
      -- Reward Merkle tree data has already been migrated into new tables; we can drop the old
      -- Merkle tree tables.
      DROP TABLE reward_merkle_tree_v2;
      DROP TABLE reward_merkle_tree;
   ELSE
      -- The migration has not been completed yet. It will run when the node starts up, and will
      -- drop the old tables when completed.
   END IF;
END
$$
