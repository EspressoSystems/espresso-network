-- This set of SQL(ite) statements defines the schema for the local database used by the light
-- client. Because light clients are local and (somewhat) ephemeral, we don't worry about
-- migrations or backwards compatibility. This file always represents the latest version of the
-- schema, the one compatible with the client code in this repository. Updates are handled by
-- deleting the local database, reapplying the schema, and then repopulating the database, as the
-- light client is always designed to be able to sync efficiently from scratch anyways.

CREATE TABLE leaf (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    height  BIGINT NOT NULL UNIQUE,
    hash    TEXT NOT NULL UNIQUE,
    data    JSONB NOT NULL
);
