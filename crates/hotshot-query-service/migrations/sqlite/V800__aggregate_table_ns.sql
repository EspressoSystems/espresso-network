-- this table is dropped so that we can start building aggregator stats from block height 0
-- for each namespace
-- otherwise aggregator would have namespace stats for future blocks only

DROP TABLE IF EXISTS aggregate;

CREATE TABLE aggregate (
    height BIGINT,
    namespace BIGINT,
    num_transactions BIGINT NOT NULL,
    payload_size BIGINT NOT NULL,
    PRIMARY KEY (height, namespace) 
);