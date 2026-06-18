-- hash.id was SERIAL (32-bit INTEGER), which exhausted its sequence at 2^31-1.
-- Widen the sequence, the primary key, and all foreign-key references to BIGINT.
ALTER SEQUENCE hash_id_seq AS BIGINT;

ALTER TABLE hash ALTER COLUMN id TYPE BIGINT;

ALTER TABLE fee_merkle_tree     ALTER COLUMN hash_id TYPE BIGINT;
ALTER TABLE block_merkle_tree   ALTER COLUMN hash_id TYPE BIGINT;
ALTER TABLE reward_merkle_tree  ALTER COLUMN hash_id TYPE BIGINT;
ALTER TABLE reward_merkle_tree_v2 ALTER COLUMN hash_id TYPE BIGINT;
