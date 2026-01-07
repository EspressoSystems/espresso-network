CREATE TABLE leaf (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    height        BIGINT NOT NULL UNIQUE,
    hash          TEXT NOT NULL UNIQUE,
    block_hash    TEXT NOT NULL UNIQUE,
    payload_hash  TEXT NOT NULL,
    data    JSONB NOT NULL
);
CREATE INDEX leaf_payload_hash_height ON leaf (payload_hash, height);
