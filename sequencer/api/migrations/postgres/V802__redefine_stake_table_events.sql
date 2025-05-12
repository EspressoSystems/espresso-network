DROP TABLE stake_table_events;

CREATE TABLE stake_table_events (
  l1_block BIGINT NOT NULL,
  log_index BIGINT NOT NULL,
  event JSONB NOT NULL,
  PRIMARY KEY (l1_block, log_index)
);