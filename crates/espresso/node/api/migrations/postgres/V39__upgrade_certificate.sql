CREATE TABLE upgrade_certificate (
    id bool PRIMARY KEY DEFAULT true,
    data BYTEA
);
REVOKE DELETE, TRUNCATE ON upgrade_certificate FROM public;
