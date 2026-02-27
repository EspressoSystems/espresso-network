CREATE TABLE leaf (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    height        BIGINT NOT NULL UNIQUE,
    hash          TEXT NOT NULL UNIQUE,
    block_hash    TEXT NOT NULL UNIQUE,
    payload_hash  TEXT NOT NULL,
    data    JSONB NOT NULL
);
CREATE INDEX leaf_payload_hash_height ON leaf (payload_hash, height);

-- This table just keeps track of which epochs we have a stake table for.
CREATE TABLE stake_table_epoch (
    epoch BIGINT PRIMARY KEY
);

-- This table holds the actual entries for each stake table.
CREATE TABLE stake_table_validator (
    epoch        BIGINT  NOT NULL REFERENCES stake_table_epoch (epoch) ON DELETE CASCADE,
    idx          INTEGER NOT NULL,
    data         JSONB   NOT NULL,
    PRIMARY KEY (epoch, idx)
);

-- This table tracks used BLS keys for each stake table.
--
-- It is cumulative, meaning each BLS key is only added once, with the epoch number where it is
-- first used, but it belongs to the `used_bls_keys` set in each stake table after that epoch as
-- well.
CREATE TABLE stake_table_bls_key (
    key   TEXT   PRIMARY KEY,
    epoch BIGINT NOT NULL
);
CREATE INDEX stake_table_bls_key_epoch ON stake_table_bls_key (epoch);

-- This table tracks used Schnorr keys for each stake table.
--
-- It is cumulative, meaning each key is only added once, with the epoch number where it is first
-- used, but it belongs to the `used_schnorr_keys` set in each stake table after that epoch as well.
CREATE TABLE stake_table_schnorr_key (
    key   TEXT   PRIMARY KEY,
    epoch BIGINT NOT NULL
);
CREATE INDEX stake_table_schnorr_key_epoch ON stake_table_schnorr_key (epoch);

-- This table tracks exiting validators for each stake table.
--
-- It is cumulative, meaning each address is only added once, with the epoch number where it first
-- exits, but it belongs to the `validator_exits` set in each stake table after that epoch as well.
CREATE TABLE stake_table_exit (
    address TEXT   PRIMARY KEY,
    epoch   BIGINT NOT NULL
);
CREATE INDEX stake_table_exit_epoch ON stake_table_exit (epoch);
