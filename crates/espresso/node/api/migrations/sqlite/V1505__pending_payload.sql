-- Block payloads this node obtained, by reconstruction or by building the block, held until a
-- decide delivers them to the query service. Rows are deleted once delivered.
CREATE TABLE pending_payload (
    view BIGINT PRIMARY KEY,
    payload_hash VARCHAR NOT NULL,
    header BLOB NOT NULL,
    payload BLOB NOT NULL
);
CREATE INDEX pending_payload_payload_hash_idx ON pending_payload (payload_hash);
