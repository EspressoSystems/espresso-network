# query-watchdog

Checks that archive query nodes serve every height, epoch object and aggregate they should, keeps the missing set in
SQLite, and reports it through logs, metrics and a page.

## Run locally

```
cargo run -p query-watchdog -- --nodes https://query.decaf.testnet.espresso.network,http://localhost:24000 \
  --db-path tmp/query-watchdog/state.sqlite --retention-blocks 2000
```

- The `serving` INFO line prints the endpoint URLs.
- `/`: progress and missing ranges per node, with links to the failing GET.
- `/status/metrics`: Prometheus metrics, `query_watchdog_*`.
- `/healthcheck`: 200 with `{"status":"available"}`.
- Logs: `RUST_LOG` (default INFO), `RUST_LOG_FORMAT=json`.

## Options

| Flag                    | Env (`ESPRESSO_QUERY_WATCHDOG_`) | Default                                                                                                                    |
| ----------------------- | -------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `--nodes`               | `NODES`                          | required, comma-separated base URLs without `/v1`, path ends with `/`                                                      |
| `--db-path`             | `DB_PATH`                        | platform data directory, `state.sqlite` (Linux: `~/.local/share/query-watchdog/state.sqlite`); an error when there is none |
| `--port`                | `PORT`                           | 8080                                                                                                                       |
| `--sweep-interval`      | `SWEEP_INTERVAL`                 | 24h, time from one sweep start to the next                                                                                 |
| `--request-interval`    | `REQUEST_INTERVAL`               | 200ms, wait before each request of one node                                                                                |
| `--retention-blocks`    | `RETENTION_BLOCKS`               | absent: archive, sweep `[0, tip)`; N: sweep `[tip - N, tip)`, start rounded up to a multiple of 100                        |
| `--alert-missing-after` | `ALERT_MISSING_AFTER`            | 60s, minimum age of missing data for the alert                                                                             |

## Checked

Per 100-height chunk (follow from the tip, sweep over the window). Probes are HEAD requests.

| Kind                          | Unit    | Route                                                   |
| ----------------------------- | ------- | ------------------------------------------------------- |
| `leaf`, `block`, `vid_common` | heights | `v1/availability/{leaf,block,vid/common}/{start}/{end}` |
| `lc_leaf_proof`               | heights | `v1/light-client/leaf/{end - 1}`                        |
| `aggregate`                   | heights | `v1/node/transactions/count/{end - 1}`                  |
| `stake_table`                 | epochs  | `v1/node/stake-table/{epoch}`                           |
| `state_cert`                  | epochs  | `v1/catchup/{epoch}/state-cert`                         |
| `lc_stake_table`              | epochs  | `v1/light-client/stake-table/{epoch}`                   |

- A 404 on a `leaf`, `block` or `vid_common` range is refined to the exact missing heights: GET reads the first missing
  height from the 404 message, then galloping and binary search over single heights find the end of the gap, repeated to
  the end of the range. `lc_leaf_proof` and `aggregate` count one height per missing chunk, per-epoch kinds one epoch.
- Gauges for the merklized state head lag and follow lag are exported.

## Log levels

| Level                                 | Meaning                                                                                    |
| ------------------------------------- | ------------------------------------------------------------------------------------------ |
| ERROR, target `query_watchdog::alert` | missing data, the only line that pages                                                     |
| ERROR, other targets                  | the watchdog itself is failing; the process exits                                          |
| WARN                                  | degraded or transient (`node unhealthy`: non-404 failures in the last minute), never pages |
| INFO                                  | lifecycle: config, start, sweep start and end, one summary per node per minute             |
| DEBUG                                 | per request                                                                                |

The alert line `missing data` has the fields `node`, `kind`, `missing`, `oldest_age_seconds`, `failed_probes`,
`range_start`, `range_end` and `url` (the GET URL of the oldest qualifying range). It is logged at most once per node
and kind per minute, for the oldest row of the kind that meets all of:

- the last failure was a 404 (for `leaf`, `block`, `vid_common` one that named a height, so the range is exact); 500,
  timeout and transport failures never qualify
- at least 3 consecutive failed probes
- first missing at least `--alert-missing-after` ago
- not within 10 blocks of the tip (per-epoch kinds: the epoch's last block is at least 10 below the tip)

## Docker

- Image: `docker/query-watchdog.Dockerfile`, entrypoint `docker/query-watchdog/entrypoint.sh`.
- Build locally: `scripts/build-docker-images-native --image query-watchdog`.
- The image sets `ESPRESSO_QUERY_WATCHDOG_PORT=80` and `ESPRESSO_QUERY_WATCHDOG_DB_PATH=/data/state.sqlite`.
- `/data` is not a volume. Mount one (`-v <dir>:/data`) or use litestream to keep state across containers.

## State replication (litestream)

The SQLite file holds the missing ranges, their first-missing times and sweep progress. Losing it resets alert ages and
restarts the sweep from the window start. [Litestream](https://litestream.io) can stream it to S3 so a replaced Fargate
task resumes where the old one stopped.

- Optional. Without `LITESTREAM_REPLICA_URL` the entrypoint logs one line and runs the watchdog alone on local state.
- With `LITESTREAM_REPLICA_URL` (for example `s3://bucket/query-watchdog/decaf`):
  1. `litestream restore -if-db-not-exists -if-replica-exists` fetches the last replica when the local file is absent.
  2. `litestream replicate -exec /bin/query-watchdog` runs the watchdog and streams changes (about 1 s lag).
- Config: `docker/query-watchdog/litestream.yml` (one db, one `replica:` with `url: ${LITESTREAM_REPLICA_URL}`).
- Credentials: AWS SDK default chain, on Fargate the task role. The role needs read and write on the replica prefix.
- One writer per replica path. Run the ECS service with `minimumHealthyPercent=0` and `maximumPercent=100`; two writers
  corrupt the replica.
- Local test: `-e LITESTREAM_REPLICA_URL=file:///replica -v <dir>:/replica`.
- Litestream is installed from the pinned release `.deb` (`LITESTREAM_VERSION` in the Dockerfile).

## Known limitations

- Archive full sweep is slow: about 100 / (5 x (`request_interval` + response time)) heights/s, about 46/s at 200 ms.
  The rate holds for clean chunks. Each gap in a range kind adds a HEAD, a GET and about 2 x log2(gap length)
  single-height probes. Sweeps then run back to back. The tip is covered by follow in real time. The page ETA and
  `sweep_eta_seconds` show live numbers.
- Probes start fetches on the watched node, so the watchdog also repairs. Missing age means "not fetchable for that
  long".
- Exact gaps assume one contiguous gap per search step: holes inside a gap are counted as missing up to the first
  present height found. A failure other than 404 (500, timeout, transport), or a 404 message that does not read
  `<kind> <height> not found`, leaves the rest of the range at chunk level, an upper bound. A retry of an exact gap
  probes its last height first and keeps the whole row while that height is missing: heights repaired at the front stay
  counted, and the row keeps its age and can still alert.
- Any non-200 counts as missing (404, 500, timeout, transport). An overloaded or unreachable node shows as missing data.
  `fetch_errors_total{status}` separates the causes.
- Old-epoch `stake_table` probes return 500 and start a stake-table catchup on the node. They resolve within about 20 s.
  They never alert (only 404 does), but an archive sweep triggers one catchup per historical epoch and a
  `node unhealthy` WARN each minute while they fail.
- Not checked: data contents, merklized-state history (archive nodes prune it), transactions index, namespace proofs,
  reward tree, `chain_config`, `vid_share`.
- `--retention-blocks` must not exceed the node's own retention, or pruned heights show as missing.
- One database per node set: reusing a path with a different `--nodes` list deletes the other set's rows at startup, so
  use `--db-path` per network.
- Without `LITESTREAM_REPLICA_URL` the state is lost when the task restarts.
- Node URLs are assumed internal, without credentials.
- Route paths and the 100-height chunk are hard-coded copies of the server's routes and limits.
- The sweep ETA uses the rate since this process started sweeping and resets on restart.
- The tip is the max over all watched nodes. A single node that is stuck but answering keeps the tip flat: no probes
  above it, no missing ranges, no lag; only `tip_height` stays flat. With several nodes, the stuck node's follow probes
  above its height go missing and the follow lag grows.

## Datadog

JSON logs (`RUST_LOG_FORMAT=json`) put event fields under `fields`. Metric names below are as Datadog receives them:
`<namespace>` is set in the OpenMetrics check config, gauges keep their name, and counters lose `_total` and gain
`.count`.

Recommended alert, with a link to the missing data:

1. Logs > Configuration: add facets on `@fields.node` and `@fields.kind`.
2. Monitors > New Monitor > Logs:
   - Query: `service:<watchdog service> @target:"query_watchdog::alert" status:error`.
   - Alert condition: count above 0 over the last 5 minutes.
   - Multi alert, grouped by `@fields.node` and `@fields.kind`.
3. Message:
   - Text:
     `{{[@fields.node].name}} {{[@fields.kind].name}} missing for {{log.attributes.[fields.oldest_age_seconds]}} s ({{log.attributes.[fields.missing]}} missing): {{log.attributes.[fields.url]}}`.
   - `missing` is in heights, or epochs for `stake_table`, `state_cert` and `lc_stake_table`.
   - Notify target: `@slack-<channel>`.
   - Syntax: docs.datadoghq.com/monitors/notify/variables/ (no `@` in `log.attributes`, brackets around dotted keys).

Companion monitors (Monitors > New Monitor > Metric):

| Monitor                       | Query                                                                                      | Threshold                                                                                                                                    |
| ----------------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Missing age, without the link | `max(last_5m):max:<namespace>.query_watchdog_oldest_missing_age_seconds{*} by {node,kind}` | above 60, matching `--alert-missing-after`; does not page: it also counts non-404 failures (500, transport, unrefined) and data near the tip |
| Watchdog down or stuck        | `sum(last_10m):sum:<namespace>.query_watchdog_probes.count{*} by {node}.as_count()`        | equal 0, or notify on no data                                                                                                                |
| Tip flat                      | `change(max(last_10m),last_10m):max:<namespace>.query_watchdog_tip_height{*}`              | equal 0                                                                                                                                      |
| Follow lag                    | `max(last_5m):max:<namespace>.query_watchdog_follow_lag_blocks{*} by {node}`               | above `<blocks>`                                                                                                                             |
| Merklized state lag           | `max(last_5m):max:<namespace>.query_watchdog_merklized_state_lag_blocks{*} by {node}`      | above `<blocks>`                                                                                                                             |

## Schema

- Migrations are in `migrations/` and run at startup (`sqlx::migrate!`).
- Add a new `NN_name.sql`. Never edit an applied migration.
