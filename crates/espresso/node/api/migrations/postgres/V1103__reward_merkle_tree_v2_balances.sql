CREATE TABLE reward_merkle_tree_v2_data (
    height BIGINT PRIMARY KEY,
    balances BYTEA
);

CREATE TABLE reward_merkle_tree_v2_proofs (
    height BIGINT,
    account BYTEA NOT NULL,
    proof BYTEA
);

ALTER TABLE 
  reward_merkle_tree_v2_proofs 
ADD 
  CONSTRAINT reward_merkle_tree_v2_proofs_pk PRIMARY KEY (height, account);
