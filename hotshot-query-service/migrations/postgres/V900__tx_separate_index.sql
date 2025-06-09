CREATE FUNCTION json_4byte_le_to_integer(j JSONB)
    RETURNS BIGINT
    LANGUAGE plpgsql
    IMMUTABLE
AS $$
DECLARE
    len INT := jsonb_array_length(j);
BEGIN
    IF len <> 4 THEN
        RAISE 'expected JSON array of 4 bytes, got (%) instead', len;
    END IF;
    RETURN (j->0)::bigint 
         + (j->1)::bigint*256
         + (j->2)::bigint*256*256
         + (j->3)::bigint*256*256*256;
END;
$$;

ALTER TABLE transactions
    -- Add the new columns and populate from the existing JSON `idx` field.
    ADD COLUMN namespace BIGINT NOT NULL GENERATED ALWAYS AS (json_4byte_le_to_integer(idx -> 'ns_index')) STORED,
    ADD COLUMN position BIGINT NOT NULL GENERATED ALWAYS AS (json_4byte_le_to_integer(idx -> 'tx_index')) STORED;
ALTER TABLE transactions
    -- Drop the generated expressions for subsequently added rows, where the new columns should be
    -- explicit.
    ALTER COLUMN namespace DROP EXPRESSION,
    ALTER COLUMN position DROP EXPRESSION,
    -- Now that we have explicit columns for namespace and position within namespace, the JSON blob
    -- `idx` is no longer needed.
    DROP COLUMN idx,
    -- `idx` used to be part of the primary key. Now we have a primary key that combines the new
    -- namespace and position fields with block height. We sort on namespace before position so that
    -- when iterating, we get all the transactions within a given namespace consecutively.
    ADD PRIMARY KEY (block_height, namespace, position);

DROP FUNCTION json_4byte_le_to_integer;
