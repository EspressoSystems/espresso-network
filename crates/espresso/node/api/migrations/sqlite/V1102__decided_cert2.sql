-- cert2 finality certificates emitted by the new protocol.
CREATE TABLE decided_cert2 (
    view BIGINT PRIMARY KEY,
    data BLOB NOT NULL
);
