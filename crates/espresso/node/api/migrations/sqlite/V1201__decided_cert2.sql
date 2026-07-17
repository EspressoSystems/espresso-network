-- Stores Certificate2 finality certificates emitted by the new fast finality protocol.
-- Rows are written when the coordinator handles a new leaf decide
CREATE TABLE decided_cert2 (
    view BIGINT PRIMARY KEY,
    data BLOB NOT NULL
);
