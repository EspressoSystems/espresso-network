CREATE TABLE next_epoch_quorum_certificate (
    id bool PRIMARY KEY DEFAULT true,
    data BYTEA
);
REVOKE DELETE, TRUNCATE ON next_epoch_quorum_certificate FROM public;
