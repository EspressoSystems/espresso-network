CREATE TABLE data_migrations (
    name TEXT NOT NULL,
    table_name TEXT NOT NULL,
    completed BOOLEAN NOT NULL DEFAULT false,
    migrated_rows BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (name, table_name)
);

INSERT INTO data_migrations (name, table_name) VALUES
    ('validator_authenticated', 'epoch_drb_and_root'),
    ('validator_authenticated', 'stake_table_validators');
