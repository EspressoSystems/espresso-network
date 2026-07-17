CREATE TABLE undecided_state (
    -- The ID is always set to 0. Setting it explicitly allows us to enforce with every insert or
    -- update that there is only a single entry in this table: the latest known state.
    id INT PRIMARY KEY,

    leaves BYTEA NOT NULL,
    state  BYTEA NOT NULL
);
