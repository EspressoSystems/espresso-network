DROP TABLE stake_table_events;

-- Stores the stake table events from the contract.  
-- Each event is uniquely identified by a combination of `l1_block` and `log_index`
CREATE TABLE stake_table_events (
  l1_block BIGINT NOT NULL,
  log_index BIGINT NOT NULL,
  event JSONB NOT NULL,
  PRIMARY KEY (l1_block, log_index)
);

-- Tracks the last L1 block that has been finalized and whose events have been processed and stored.  
-- This tracking is necessary to determine the starting point for fetching new contract events. 
CREATE TABLE stake_table_events_l1_block (
  id INTEGER PRIMARY KEY CHECK (id = 0),
  last_l1_block BIGINT NOT NULL
);