-- The id of the blob directory this database stores payloads and VID shares in. The row is written
-- on the first connect with a blob directory; a directory with another id is refused.
CREATE TABLE blob_store (
    id       INTEGER PRIMARY KEY,
    store_id TEXT NOT NULL
);
