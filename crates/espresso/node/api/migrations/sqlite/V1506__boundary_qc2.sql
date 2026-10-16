-- Stores the new fast-finality protocol's latest epoch boundary certificate:
-- the own Certificate1 of the last block of an epoch, at the block's view.
-- A re-vote request and the next epoch's first block name it, so a node must
-- keep it across a restart to lead either. A single row (id = true) holds the
-- latest, ordered by epoch, then view.
CREATE TABLE boundary_qc2 (
    id bool PRIMARY KEY DEFAULT true,
    data BLOB NOT NULL
);
