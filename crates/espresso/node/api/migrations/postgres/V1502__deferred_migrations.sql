-- Progress-tracking table for background DataBackfill migrations.
CREATE TABLE deferred_migrations (
    name         TEXT        PRIMARY KEY,
    started_at   TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    error        TEXT,
    last_offset  BIGINT
);
