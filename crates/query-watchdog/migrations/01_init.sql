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
    -- start of the latest sweep; the next one starts one --sweep-interval later
    last_sweep_started_at INTEGER NULL,
    -- set while a sweep runs
    sweep_until_height INTEGER NULL,
    sweep_next_height INTEGER NULL,
    -- the one sweep of --sweep-once is finished
    sweep_done INTEGER NOT NULL DEFAULT 0,
    last_sweep_completed_at INTEGER NULL,
    CHECK ((sweep_until_height IS NULL) = (sweep_next_height IS NULL)),
    CHECK (sweep_done = 0 OR sweep_next_height IS NULL),
    CHECK (sweep_next_height IS NULL OR last_sweep_started_at IS NOT NULL)
);
