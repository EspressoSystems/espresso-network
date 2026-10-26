CREATE TABLE leaf (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    height        BIGINT NOT NULL UNIQUE,
    hash          TEXT NOT NULL UNIQUE,
    block_hash    TEXT NOT NULL UNIQUE,
    payload_hash  TEXT NOT NULL,
    data          JSONB NOT NULL,
    last_used     BIGINT NOT NULL DEFAULT 0
);
CREATE INDEX leaf_payload_hash_height ON leaf (payload_hash, height);
CREATE INDEX leaf_last_used ON leaf (last_used);

CREATE TABLE stake_table_epoch (
    epoch                            BIGINT PRIMARY KEY,
    epoch_root_protocol_version      TEXT   NOT NULL,
    next_epoch_root_protocol_version TEXT   NOT NULL
);

CREATE TABLE stake_table_validator (
    epoch        BIGINT  NOT NULL REFERENCES stake_table_epoch (epoch) ON DELETE CASCADE,
    idx          BIGINT  NOT NULL,
    data         JSONB   NOT NULL,
    PRIMARY KEY (epoch, idx)
);

CREATE TABLE stake_table_bls_key (
    key   TEXT   PRIMARY KEY,
    epoch BIGINT NOT NULL
);
CREATE INDEX stake_table_bls_key_epoch ON stake_table_bls_key (epoch);

CREATE TABLE stake_table_schnorr_key (
    key   TEXT   PRIMARY KEY,
    epoch BIGINT NOT NULL
);
CREATE INDEX stake_table_schnorr_key_epoch ON stake_table_schnorr_key (epoch);

CREATE TABLE stake_table_x25519_key (
    key   TEXT   PRIMARY KEY,
    epoch BIGINT NOT NULL
);
CREATE INDEX stake_table_x25519_key_epoch ON stake_table_x25519_key (epoch);

CREATE TABLE stake_table_exit (
    address TEXT   PRIMARY KEY,
    epoch   BIGINT NOT NULL
);
CREATE INDEX stake_table_exit_epoch ON stake_table_exit (epoch);
