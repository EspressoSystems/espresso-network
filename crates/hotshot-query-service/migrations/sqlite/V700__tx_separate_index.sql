-- In SQLite, we have to create a new table since we are going to be using a different primary key,
-- and then copy the data over.
CREATE TABLE transactions2 (
    hash TEXT NOT NULL,
    -- Block containing this transaction.
    block_height BIGINT NOT NULL REFERENCES header(height) ON DELETE CASCADE,
    -- Index within block of the namespace containing this transaction.
    ns_index BIGINT NOT NULL,
    -- Namespace containing this transaction.
    ns_id BIGINT NOT NULL,
    -- Position within the namespace.
    position BIGINT NOT NULL,
    PRIMARY KEY (block_height, ns_id, position)
);

DROP TABLE transactions;
ALTER TABLE transactions2 RENAME TO transactions;
