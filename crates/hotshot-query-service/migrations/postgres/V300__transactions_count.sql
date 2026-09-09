ALTER TABLE payload
    ADD COLUMN num_transactions INTEGER;

-- Initialize the `num_transactions` column by counting transactions for each
-- existing payload.
UPDATE payload AS p
    SET (num_transactions) =
        (SELECT count(*) FROM transaction AS t where t.block_height = p.height)
    -- Don't set `num_transactions` (leave it NULL) for payloads we don't have
    -- yet.
    WHERE p.data IS NOT NULL;
