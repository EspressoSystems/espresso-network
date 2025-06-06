DROP TABLE IF EXISTS aggregate;

CREATE TABLE aggregate (
    height BIGINT,
    namespace BIGINT,
    num_transactions BIGINT NOT NULL,
    payload_size BIGINT NOT NULL,
    PRIMARY KEY (height, namespace)
);
