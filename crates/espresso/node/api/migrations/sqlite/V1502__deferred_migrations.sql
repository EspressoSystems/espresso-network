-- Progress-tracking table for background DataBackfill migrations.
CREATE TABLE deferred_migrations (
    name         TEXT     PRIMARY KEY,
    started_at   TEXT     NOT NULL,
    completed_at TEXT,
    error        TEXT,
    last_offset  INTEGER
);
