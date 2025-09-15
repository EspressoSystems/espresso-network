CREATE TABLE stake_table_validators (
    epoch BIGINT NOT NULL,
    address TEXT NOT NULL,
    validator JSON NOT NULL,
    PRIMARY KEY (epoch, address)
);