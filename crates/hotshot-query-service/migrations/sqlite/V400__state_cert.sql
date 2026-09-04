-- This table is used to store the finalized light client state cert
CREATE TABLE finalized_state_cert
(
    epoch BIGINT PRIMARY KEY,
    state_cert BLOB
);

-- This table is used for consensus to store the light client state cert indexed by view
CREATE TABLE state_cert
(
    view BIGINT PRIMARY KEY,
    state_cert BLOB
);
