ALTER TABLE transaction
  RENAME TO transactions;

ALTER TABLE transactions
  RENAME COLUMN index TO idx;