-- Payloads reconstructed (or built) for a view, held until a decide replays them to the query
-- service. Separate from da_proposal2, whose rows are consumed by the decide that covers their view.
CREATE TABLE reconstructed_payload (
    view BIGINT PRIMARY KEY,
    header BLOB NOT NULL,
    payload BLOB NOT NULL
);
