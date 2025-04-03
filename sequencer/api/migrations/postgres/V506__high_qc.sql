CREATE TABLE high_qc2 (
    id bool PRIMARY KEY DEFAULT true,
    data BYTEA
);
REVOKE DELETE, TRUNCATE ON high_qc2 FROM public;
