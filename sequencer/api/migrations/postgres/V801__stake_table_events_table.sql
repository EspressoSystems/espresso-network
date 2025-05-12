CREATE TABLE stake_table_events (
  id INTEGER PRIMARY KEY CHECK (id = 0),
  l1_block BIGINT NOT NULL,
  data JSONB NOT NULL
);