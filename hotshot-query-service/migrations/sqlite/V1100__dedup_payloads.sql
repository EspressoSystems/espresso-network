-- Move VID shares into the `header` table. These are unique per block, since even if two blocks
-- have the same payload, we might get a different share based on our position in the stake table.
-- Thus we treat these as per-block metadata. The column is nullable since we might not ever get a
-- share for a given block.
ALTER TABLE header ADD COLUMN vid_share BLOB;
UPDATE header SET vid_share = vid2.share
    FROM vid2 WHERE header.height = vid2.height;

-- Add explicit `ns_table` column to header. This is necessary to reference the new deduplicated
-- payload table, since a payload is identified by both the VID payload commitment _and_ the
-- namespace table, which tells us how to interpret the payload as structured data.
ALTER TABLE header ADD COLUMN ns_table VARCHAR;
UPDATE header SET ns_table = data->'fields'->'ns_table'->>'bytes';

-- Index the column pair which will be used to reference the new payload table.
CREATE INDEX header_payload_hash_ns_table ON header (payload_hash, ns_table);

-- Re-index payload data by (hash, ns_table).
CREATE TABLE payload_temp (
    hash             VARCHAR NOT NULL,
    ns_table         VARCHAR NOT NULL,
    data             BLOB    NOT NULL,
    size             INTEGER NOT NULL,
    num_transactions INTEGER NOT NULL,
    PRIMARY KEY (hash, ns_table)
);
INSERT INTO payload_temp
    SELECT payload_hash, ns_table, payload.data, size, num_transactions
    FROM header
    JOIN payload ON header.height = payload.height
    GROUP BY payload_hash, ns_table;
DROP TABLE payload;
ALTER TABLE payload_temp RENAME TO payload;

-- Re-index VID data by hash.
CREATE TABLE vid_common (
    hash VARCHAR PRIMARY KEY,
    data BLOB    NOT NULL
);
INSERT INTO vid_common
    SELECT payload_hash, common
    FROM header
    JOIN vid2 ON header.height = vid2.height
    GROUP BY payload_hash;
DROP TABLE vid2;
