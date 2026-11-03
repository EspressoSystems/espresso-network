CREATE TABLE leaf2
(
    height     BIGINT NOT NULL REFERENCES header (height) ON DELETE CASCADE,
    view       BIGINT NOT NULL,
    hash       VARCHAR NOT NULL UNIQUE,
    block_hash VARCHAR NOT NULL REFERENCES header (hash) ON DELETE CASCADE,
    leaf JSONB NOT NULL,
    qc   JSONB NOT NULL,
    PRIMARY KEY (height, view)
);

CREATE TABLE types_migration ( 
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    completed bool NOT NULL DEFAULT false
);

INSERT INTO types_migration ("completed") VALUES (false);

CREATE TABLE vid2
(
    height BIGINT PRIMARY KEY REFERENCES header (height) ON DELETE CASCADE,
    common BYTEA  NOT NULL,
    share  BYTEA
);
