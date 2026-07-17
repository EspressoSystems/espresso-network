CREATE TABLE eqc (
    id bool PRIMARY KEY DEFAULT true,
    data BYTEA
);
REVOKE DELETE, TRUNCATE ON eqc FROM public;
