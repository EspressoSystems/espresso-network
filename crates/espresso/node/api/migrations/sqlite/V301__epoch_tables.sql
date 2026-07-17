CREATE TABLE anchor_leaf2 (
    view   BIGINT PRIMARY KEY,
    leaf  BLOB,
    qc   BLOB
);

 
CREATE TABLE da_proposal2 (
    view BIGINT PRIMARY KEY,
    payload_hash VARCHAR,
    data BLOB
);

CREATE TABLE vid_share2 (
    view BIGINT PRIMARY KEY,
    payload_hash VARCHAR,
    data BLOB
);


CREATE TABLE quorum_proposals2 (
    view BIGINT PRIMARY KEY,
    leaf_hash VARCHAR,
    data BLOB
);

CREATE UNIQUE INDEX quorum_proposals2_leaf_hash_idx ON quorum_proposals (leaf_hash);
CREATE INDEX da_proposal2_payload_hash_idx ON da_proposal (payload_hash);
CREATE INDEX vid_share2_payload_hash_idx ON vid_share (payload_hash);
 
CREATE TABLE quorum_certificate2 (
    view BIGINT PRIMARY KEY,
    leaf_hash VARCHAR NOT NULL,
    data BLOB NOT NULL
);

CREATE INDEX quorum_certificate2_leaf_hash_idx ON quorum_certificate (leaf_hash);

CREATE TABLE epoch_migration (
    table_name TEXT PRIMARY KEY,
    completed bool NOT NULL DEFAULT FALSE
);

INSERT INTO epoch_migration (table_name) VALUES ('anchor_leaf'), ('da_proposal'), ('vid_share'), ('quorum_proposals'), ('quorum_certificate');