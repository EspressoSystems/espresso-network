-- SQLite doesn't let you drop PK columns or restructure tables, so we make a temporary table
-- and copy the data from the old leaf2 table.

CREATE TABLE leaf2_new
(
    height     BIGINT PRIMARY KEY REFERENCES header (height) ON DELETE CASCADE,
    hash       VARCHAR NOT NULL UNIQUE,
    block_hash VARCHAR NOT NULL REFERENCES header (hash) ON DELETE CASCADE,
    leaf JSONB NOT NULL,
    qc   JSONB NOT NULL
);

INSERT INTO leaf2_new (height,hash,block_hash,leaf,qc) SELECT height,hash,block_hash,leaf,qc FROM leaf2;

DROP TABLE leaf2;

ALTER TABLE leaf2_new RENAME TO leaf2;
