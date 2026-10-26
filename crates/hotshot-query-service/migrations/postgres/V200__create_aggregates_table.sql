CREATE TABLE aggregate (
    height BIGINT PRIMARY KEY REFERENCES header (height) ON DELETE CASCADE,
    num_transactions BIGINT NOT NULL,
    payload_size BIGINT NOT NULL
);
