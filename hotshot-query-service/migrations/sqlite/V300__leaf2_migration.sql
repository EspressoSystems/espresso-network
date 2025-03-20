CREATE TABLE leaf2
(
    height     BIGINT  PRIMARY KEY REFERENCES header (height) ON DELETE CASCADE,
    hash       VARCHAR NOT NULL UNIQUE,
    block_hash VARCHAR NOT NULL REFERENCES header (hash) ON DELETE CASCADE,
    leaf JSONB NOT NULL,
    qc   JSONB NOT NULL
);

CREATE TABLE leaf_migration ( 
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    completed bool NOT NULL DEFAULT false
);

INSERT INTO leaf_migration ("completed") VALUES (false);