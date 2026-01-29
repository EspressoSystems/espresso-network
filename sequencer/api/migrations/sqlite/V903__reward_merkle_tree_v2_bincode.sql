CREATE TABLE reward_merkle_tree_v2_bincode (
    height BIGINT PRIMARY KEY,
    serialized_bytes BLOB
);

CREATE TABLE reward_merkle_tree_v2_proofs (
    height BIGINT,
    account BLOB NOT NULL,
    serialized_bytes BLOB,
    PRIMARY KEY (height, account)
);
