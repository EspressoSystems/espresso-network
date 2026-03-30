CREATE TABLE anchor_leaf (
    -- The ID is always set to 0. Setting it explicitly allows us to enforce with every insert or
    -- update that there is only a single entry in this table: the latest decided leaf.
    id INT PRIMARY KEY,

    height BIGINT,
    view   BIGINT,
    leaf   BYTEA,
    qc     BYTEA
);

CREATE TABLE highest_voted_view (
    -- The ID is always set to 0. Setting it explicitly allows us to enforce with every insert or
    -- update that there is only a single entry in this table: the latest known view.
    id INT PRIMARY KEY,

    view BIGINT
);

CREATE TABLE da_proposal (
    view BIGINT PRIMARY KEY,
    data BYTEA
);

CREATE TABLE vid_share (
    view BIGINT PRIMARY KEY,
    data BYTEA
);
