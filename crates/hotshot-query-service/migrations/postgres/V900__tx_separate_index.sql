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

CREATE FUNCTION read_ns_id(ns_table BYTEA, ns_index BIGINT)
    RETURNS BIGINT
    LANGUAGE plpgsql
    IMMUTABLE
AS $$
DECLARE
    start INT := 8*ns_index + 4;
BEGIN
    RETURN bytea_4byte_le_to_integer(substring(ns_table from start + 1 for 4));
END;
$$;

CREATE FUNCTION bytea_4byte_le_to_integer(b BYTEA)
    RETURNS BIGINT
    LANGUAGE plpgsql
    IMMUTABLE
AS $$
DECLARE
    len INT := length(b);
BEGIN
    IF len <> 4 THEN
        RAISE 'expected array of 4 bytes, got (%) instead', len;
    END IF;
    RETURN get_byte(b,0)::bigint
         + get_byte(b,1)::bigint*256
         + get_byte(b,2)::bigint*256*256
         + get_byte(b,3)::bigint*256*256*256;
END;
$$;

CREATE FUNCTION get_ns_table(h JSONB)
    RETURNS BYTEA
    LANGUAGE plpgsql
    IMMUTABLE
AS $$
DECLARE
    bytes VARCHAR := COALESCE(h #>> '{fields,ns_table,bytes}', h #>> '{ns_table,bytes}');
BEGIN
    RETURN decode(bytes, 'base64');
END;
$$;

ALTER TABLE transactions
    -- Add the new columns and populate from the existing JSON `idx` field.
    ADD COLUMN ns_index BIGINT NOT NULL GENERATED ALWAYS AS (json_4byte_le_to_integer(idx -> 'ns_index')) STORED,
    ADD COLUMN position BIGINT NOT NULL GENERATED ALWAYS AS (json_4byte_le_to_integer(idx -> 'tx_index')) STORED,
    -- Add a column for the namespace ID, which we will populate in a separate query from the
    -- corresponding namespace table.
    ADD COLUMN ns_id BIGINT;

-- Populate the `ns_id` column.
UPDATE transactions SET (ns_id) = (
    SELECT read_ns_id(get_ns_table(h.data), ns_index)
      FROM header AS h
     WHERE h.height = block_height
);

ALTER TABLE transactions
    -- Drop the generated expressions for subsequently added rows, where the new columns should be
    -- explicit.
    ALTER COLUMN ns_index DROP EXPRESSION,
    ALTER COLUMN position DROP EXPRESSION,
    -- Add a NOT NULL constraint for the ns_id column which we have now populated.
    ALTER COLUMN ns_id SET NOT NULL,
    -- Now that we have explicit columns for namespace and position within namespace, the JSON blob
    -- `idx` is no longer needed.
    DROP COLUMN idx,
    -- `idx` used to be part of the primary key. Now we have a primary key that combines the new
    -- namespace and position fields with block height. We sort on namespace before position so that
    -- when iterating, we get all the transactions within a given namespace consecutively.
    ADD PRIMARY KEY (block_height, ns_id, position);
