CREATE TABLE anchor_leaf (
    view   BIGINT PRIMARY KEY,
    leaf   JSONB,
    qc     JSONB
);

CREATE TABLE highest_voted_view (
    -- The ID is always set to 0. Setting it explicitly allows us to enforce with every insert or
    -- update that there is only a single entry in this table: the latest known view.
    id INT PRIMARY KEY,
    view BIGINT
);

CREATE TABLE da_proposal (
    view BIGINT PRIMARY KEY,
    data JSONB
);

CREATE TABLE vid_share (
    view BIGINT PRIMARY KEY,
    data JSONB
);
