CREATE TABLE restart_view (
    -- The ID is always set to 0. Setting it explicitly allows us to enforce with every insert or
    -- update that there is only a single entry in this table: the view we should restart from.
    id INT PRIMARY KEY,
    view BIGINT
);