-- In SQLite, we have to create a new table since we are going to be using a different primary key,
-- and then copy the data over.
CREATE TABLE transactions2 (
    hash TEXT NOT NULL,
    -- Block containing this transaction.
    block_height BIGINT NOT NULL REFERENCES header(height) ON DELETE CASCADE,
    -- Namespace containing this transaction.
    namespace BIGINT NOT NULL,
    -- Position within the namespace.
    position BIGINT NOT NULL,
    PRIMARY KEY (block_height, namespace, position)
);

INSERT INTO transactions2 (hash, block_height, namespace, position)
SELECT
    hash,
    block_height,

    -- SQLite doesn't support functions, so this bit is a little messier than in Postgres.
    idx->>'$.ns_index[0]'
  + idx->>'$.ns_index[1]'*256
  + idx->>'$.ns_index[2]'*256*256
  + idx->>'$.ns_index[3]'*256*256*256,

    idx->>'$.tx_index[0]'
  + idx->>'$.tx_index[1]'*256
  + idx->>'$.tx_index[2]'*256*256
  + idx->>'$.tx_index[3]'*256*256*256
FROM transactions;

DROP TABLE transactions;
ALTER TABLE transactions2 RENAME TO transactions;
