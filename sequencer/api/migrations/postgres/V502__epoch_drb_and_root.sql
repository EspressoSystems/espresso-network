CREATE TABLE epoch_drb_and_root (
    epoch BIGINT PRIMARY KEY,
    drb_result BYTEA,
    block_header BYTEA,
    stake BYTEA
);
