CREATE TABLE reward_merkle_tree_v2_data (
    height BIGINT PRIMARY KEY,
    balances BLOB
);

CREATE TABLE reward_merkle_tree_v2_proofs (
    height BIGINT,
    account BLOB NOT NULL,
    proof BLOB,
    PRIMARY KEY (height, account)
);
