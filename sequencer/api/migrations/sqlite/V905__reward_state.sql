-- Table to store reward account balances at each height

CREATE TABLE reward_state (
  height BIGINT NOT NULL,
  account JSONB NOT NULL,
  balance JSONB NOT NULL,
  PRIMARY KEY (height, account)
);

CREATE INDEX reward_state_height_idx ON reward_state (height);
