CREATE TABLE epoch_drb_and_root (
    epoch BIGINT PRIMARY KEY,
    drb_result BLOB,
    block_header BLOB,
    stake BLOB
);
