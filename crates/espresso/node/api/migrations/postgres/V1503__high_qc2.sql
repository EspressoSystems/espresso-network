-- Stores the new fast-finality protocol's locked QC (high QC).
-- A single row (id = true) holds the latest locked QC, persisted before each
-- phase-2 vote so it can be restored as the locked QC on restart.
CREATE TABLE high_qc2 (
    id bool PRIMARY KEY DEFAULT true,
    data BYTEA NOT NULL
);
REVOKE DELETE, TRUNCATE ON high_qc2 FROM public;
