-- Times are INTEGER unix seconds. range_start/range_end are in the kind's unit (height or epoch).
CREATE TABLE missing_chunk (
    node TEXT NOT NULL,
    kind TEXT NOT NULL,
    range_start INTEGER NOT NULL,
    range_end INTEGER NOT NULL,
    first_missing_at INTEGER NOT NULL,
    retry_delay_secs INTEGER NOT NULL,
    next_retry_at INTEGER NOT NULL,
    failed_probes INTEGER NOT NULL DEFAULT 1,
    last_status TEXT NOT NULL,
    PRIMARY KEY (node, kind, range_start)
);

CREATE INDEX missing_chunk_due ON missing_chunk (node, next_retry_at);

CREATE INDEX missing_chunk_age ON missing_chunk (node, first_missing_at, range_start, kind);

CREATE TABLE node_progress (
    node TEXT PRIMARY KEY,
    follow_next_height INTEGER NOT NULL,
    -- set while waiting; both this and the running columns are null once a one-time sweep is done
    next_sweep_at INTEGER NULL,
    -- set while running
    sweep_until_height INTEGER NULL,
    sweep_next_height INTEGER NULL,
    sweep_started_at INTEGER NULL,
    last_sweep_completed_at INTEGER NULL,
    CHECK (next_sweep_at IS NULL OR sweep_next_height IS NULL)
);

CREATE TABLE anchor (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    height INTEGER NOT NULL,
    hash TEXT NOT NULL
);
