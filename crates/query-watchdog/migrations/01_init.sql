-- Times are INTEGER unix seconds. range_start/range_end are in the kind's unit (height or epoch).
CREATE TABLE missing_chunk (
    node TEXT NOT NULL,
    kind TEXT NOT NULL,
    range_start INTEGER NOT NULL,
    range_end INTEGER NOT NULL,
    first_missing_at INTEGER NOT NULL,
    retry_delay_secs INTEGER NOT NULL,
    next_retry_at INTEGER NOT NULL,
    failed_probes INTEGER NOT NULL,
    last_status TEXT NOT NULL,
    PRIMARY KEY (node, kind, range_start),
    CHECK (kind IN ('leaf', 'block', 'vid_common', 'lc_leaf_proof', 'aggregate', 'stake_table', 'state_cert', 'lc_stake_table')),
    CHECK (range_start >= 0 AND range_end > range_start),
    -- point and epoch kinds hold one unit per row
    CHECK (kind IN ('leaf', 'block', 'vid_common') OR range_end = range_start + 1),
    CHECK (retry_delay_secs BETWEEN 5 AND 600),
    CHECK (failed_probes >= 1),
    CHECK (last_status <> '')
);

CREATE TABLE node_progress (
    node TEXT PRIMARY KEY,
    follow_next_height INTEGER NOT NULL,
    -- start of the latest sweep; the next one starts one --sweep-interval later
    last_sweep_started_at INTEGER NULL,
    -- set while a sweep runs
    sweep_start_height INTEGER NULL,
    sweep_until_height INTEGER NULL,
    sweep_next_height INTEGER NULL,
    -- the one sweep of --sweep-once is finished
    sweep_done INTEGER NOT NULL,
    last_sweep_completed_at INTEGER NULL,
    CHECK (follow_next_height >= 0),
    CHECK (sweep_done IN (0, 1)),
    CHECK (
        (sweep_start_height IS NULL) = (sweep_next_height IS NULL)
        AND (sweep_next_height IS NULL) = (sweep_until_height IS NULL)
    ),
    CHECK (sweep_next_height IS NULL OR last_sweep_started_at IS NOT NULL),
    CHECK (sweep_done = 0 OR (sweep_next_height IS NULL AND last_sweep_completed_at IS NOT NULL)),
    CHECK (
        sweep_next_height IS NOT NULL
        OR (last_sweep_started_at IS NULL) = (last_sweep_completed_at IS NULL)
    )
);
