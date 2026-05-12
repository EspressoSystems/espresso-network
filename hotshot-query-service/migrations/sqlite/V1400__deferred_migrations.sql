CREATE TABLE IF NOT EXISTS deferred_migrations (
    name     TEXT    PRIMARY KEY,
    progress INTEGER NOT NULL DEFAULT 0,
    done_at  TEXT
);
