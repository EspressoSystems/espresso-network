# query-watchdog

**This service should not be required.** A query node should know which data it lacks, fill the gaps itself and report
them. Today it does not, so this external watchdog fills the role:

- The node has no complete check of what it serves. `v2/node/sync-status` covers only blocks, leaves and VID common, is
  cached for 5 minutes and misses a tail above the node's highest header. Light client proofs, state certs and stake
  tables are not covered at all.
- Nothing on the node alerts on missing data, so a gap is found when a rollup or follower asks for it.
- The node's own gap filling runs in the background and reports nothing. A probe that hits a gap starts a fetch from
  peers, so the watchdog also repairs.

Remove the watchdog once the node reports and repairs its own gaps.

The watchdog checks that archive query nodes serve every height and epoch object they should, keeps the missing set in
memory (SQLite holds its image), and reports it through a paging metric, logs and a page.

## Run locally

```
cargo run -p query-watchdog -- --nodes https://query.decaf.testnet.espresso.network,http://localhost:24000 \
  --db-path tmp/query-watchdog/state.sqlite --retention-blocks 2000
```

- The `serving` INFO line prints the endpoint URLs.
- `/`: progress and missing ranges per node, with links to the failing request.
- `/status/metrics`: Prometheus metrics, `query_watchdog_*`; `query_watchdog_confirmed_missing{node}` pages.
- `/healthcheck`: 200 with `{"status":"available"}`.
- Logs: `RUST_LOG` (default INFO), `RUST_LOG_FORMAT=json`.

## Options

| Flag                    | Env (`ESPRESSO_QUERY_WATCHDOG_`) | Default                                                                                                                                                                                                                         |
| ----------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--nodes`               | `NODES`                          | required, comma-separated base URLs without `/v1`, path ends with `/`                                                                                                                                                           |
| `--db-path`             | `DB_PATH`                        | platform data directory, `state.sqlite` (Linux: `~/.local/share/query-watchdog/state.sqlite`); an error when there is none                                                                                                      |
| `--port`                | `PORT`                           | 8080                                                                                                                                                                                                                            |
| `--sweep-interval`      | `SWEEP_INTERVAL`                 | 1 week (at most 1 year), time from one sweep start to the next                                                                                                                                                                  |
| `--sweep-once`          | `SWEEP_ONCE`                     | off; sweep once, then only follow the tip (`--sweep-interval` is ignored; a node not yet `done`, a `waiting` one from an earlier `--sweep-interval` run included, starts its one sweep at once; a restart does not sweep again) |
| `--request-interval`    | `REQUEST_INTERVAL`               | 200ms, wait before each request of one node                                                                                                                                                                                     |
| `--retention-blocks`    | `RETENTION_BLOCKS`               | absent: archive, sweep `[0, tip)`; N: sweep `[tip - N, tip - 10)`, start rounded up to a multiple of 100                                                                                                                        |
| `--alert-missing-after` | `ALERT_MISSING_AFTER`            | 60s (at most 1 year), minimum age before a confirmed unit counts in `confirmed_missing`                                                                                                                                         |
| `--reset`               | `RESET`                          | off; deletes the stored state at startup and always logs a WARN                                                                                                                                                                 |

## Checked

Per chunk of at most 100 heights. Follow probes the heights settled since its last chunk (10 blocks behind the node's
own height), up to 100 per chunk, so near the tip a chunk is a few heights and `lc_leaf_proof` samples the last height
of each such chunk. The sweep covers its window in chunks of 100 and so samples one height per 100 for `lc_leaf_proof`.
Probes are HEAD requests.

| Kind                          | Unit    | Route                                                   |
| ----------------------------- | ------- | ------------------------------------------------------- |
| `leaf`, `block`, `vid_common` | heights | `v1/availability/{leaf,block,vid/common}/{start}/{end}` |
| `lc_leaf_proof`               | heights | `v1/light-client/leaf/{end - 1}`                        |
| `stake_table`                 | epochs  | `v1/node/stake-table/{epoch}`                           |
| `state_cert`                  | epochs  | `v1/catchup/{epoch}/state-cert`, then see below         |
| `lc_stake_table`              | epochs  | `v1/light-client/stake-table/{epoch}`                   |

- A 404 on a `leaf`, `block` or `vid_common` range is refined to the exact missing heights with HEADs only: a binary
  search over prefix ranges (at most 7 HEADs per 100 heights) finds the first missing height, then galloping and binary
  search over single heights find the end of the gap, repeated from a HEAD of the rest. Any 404 counts, whatever its
  body. `lc_leaf_proof` probes the last height of the chunk and counts one height per missing row, per-epoch kinds one
  epoch.
- `state_cert` takes two steps. The first HEAD reads the node's database only: 200 is present, 404 goes to step 2,
  anything else fails as for the other kinds. Step 2 is a HEAD of `v1/availability/state-cert-v2/{epoch}` with a 5 s
  timeout; the node fetches the cert from peers and stores it, so the probe heals a node whose peers have the cert. 200
  is present and counts `query_watchdog_local_misses_total{node,kind}` (no alert; it shows a node that does not store
  certs itself). A 404, 500 or timeout is not found: a cert no peer can serve within 5 s counts as missing. Any other
  status (502, 503, 504, 429, ...) and any other transport error (connect refused, reset) fails. `probes_total` counts
  both requests. `fetch_errors_total` records one 404 per unhealed probe (the real step 2 status is in the DEBUG log)
  and nothing when step 2 heals. The node's own peer fetch waits up to 40 s
  (`crates/espresso/node/src/api/state.rs:777`), plus a stake-table catchup for epochs not loaded
  (`crates/espresso/node/src/api.rs:607`). When no peer has the cert the watchdog usually sees its 5 s timeout, and
  old-epoch certs in an archive sweep can time out at 5 s even when peers have them, then heal on a later retry.
- A stake table of epoch `e` is probed with the chunk holding block `(e - 2) * epoch_height`, the epoch boundary five
  blocks after the root block the node builds it from (`root_block_in_epoch` subtracts 5); a state cert with the chunk
  holding `e * epoch_height`. The alert margin below uses the same block.
- Heights are judged against each node's own block height: a failed poll keeps the old height, an answered lower height
  is followed (a node restored from an older backup is lagging, not missing data). Nothing above the node's own height
  minus 10 is probed by follow or the sweep or counted, and the retention window follows the node's own height. The tip
  poller polls every node; `query_watchdog_tip_height` is the highest height and `query_watchdog_node_lag_blocks{node}`
  the distance of a node from it.
- The sweep covers the window up to 10 blocks below the node's own height (follow covers the rest). It pauses while the
  last follow or sweep chunk had a range or point request fail with other than 404 (500, timeout, transport); epoch
  probe failures never pause it, since old-epoch objects answer 500 while the node catches up. The next fully answered
  chunk resumes it. While paused it sends one chunk a minute to test the node. Follow and retries go on.
- Gauges for the merklized state head lag and follow lag are exported.

### Retries

Every missing unit is in one of two states: failed (no 404 since it went missing) or not found (a 404 answered, with the
number of 404s since). A not-found unit with at least 3 404s is confirmed; every other unit is confirming.

| Old state \ probe | Present | 404                                                  | 500, timeout, transport                              |
| ----------------- | ------- | ---------------------------------------------------- | ---------------------------------------------------- |
| none              | none    | not found, 1 404, retry in 5 s                       | failed, 1 failure, retry in 5 s                      |
| failed            | none    | not found, starts fresh (1 404, 5 s)                 | failed, count + 1, delay doubles (cap 600 s)         |
| not found, n 404s | none    | n + 1 404s, delay doubles (cap 60 s once n + 1 >= 3) | n 404s kept, delay doubles (cap 60 s when confirmed) |

- Delays are whole seconds from 5 s, doubling up to 600 s; confirmed units cap at 60 s. A failure never demotes a
  not-found unit.
- Each unit keeps its own state: a gap over units of two rows continues each unit's state, and units the gap newly
  covers start fresh. A retried row splits into pieces of at most 100 heights, so the page lists the pieces.
- Confirming rows are retried at their next time, at most 100 heights per probe. The order (next time, kind, first unit)
  applies within each queue: not-found rows go first, failed rows last (see the retry step below).
- Of the confirmed rows only the representative, the due row with the earliest next time (the longest overdue), is
  retried, at most once a minute, so confirmed rows are re-probed in rotation. When the representative is found present,
  every confirmed row is re-checked once (the recovery queue, memory only). Confirmed rows outside these keep a stale
  next time, are re-checked by the next sweep (never under `--sweep-once`) or by the recovery queue after a repair, and
  can keep counting until then. Follow never revisits rows.
- One retry step starts probes within 5 s (`RETRY_BUDGET`): due not-found rows below confirmation, the recovery queue,
  the representative (only while the recovery queue is empty), the recovery queue again, then due failed rows. Failed
  rows never count, so they never delay red or green. Follow and the sweep keep the rest of the loop iteration.
- Capacity at the default request cost (about 0.3 s, about 16 requests per retry step; a still-missing exact range piece
  costs 3 requests, a refilled piece or point 1, an epoch 1, a `state_cert` up to 2, 5 s on a timeout): a new gap is
  retried at about +5 s and +15 s and the node turns red about 60 to 120 s after the gap is found, whatever the number
  of confirmed rows. After a repair the representative is found present within 60 s; the recovery queue then re-checks
  about 120 pieces a minute during a sweep and about 200 without one, so N confirmed pieces take about N / 120 to N /
  200 minutes, then the monitor window of 5 minutes.
- Request timeouts: range HEAD 60 s (the node may wait 500 ms per height), single HEAD 10 s, JSON GET 5 s, peer-fetch
  HEAD (`state_cert` step 2) 5 s.

## Log levels

| Level | Meaning                                                                                    |
| ----- | ------------------------------------------------------------------------------------------ |
| ERROR | the watchdog itself is failing; the process exits                                          |
| WARN  | degraded or transient (`node unhealthy`: non-404 failures in the last minute), never pages |
| INFO  | lifecycle: config, start, sweep start and end, one `summary` per node per minute           |
| DEBUG | per request                                                                                |

The `summary` line has `node`, `errors` and `forgotten` since the previous line (`forgotten` is 0 in archive mode),
`confirmed_missing`, `sweep_eta` and the summary text with, per kind, `missing`, `confirmed` and `oldest_age`. The line
comes every 60 s per node, also while a node hangs.

Paging is a metric, `query_watchdog_confirmed_missing{node}`, written every 15 s: the units (heights and epochs) of the
node that are

- confirmed: at least 3 probes answered 404 since the first 404 (500, timeout and transport only delay retries)
- first missing at least `--alert-missing-after` ago
- not within 10 blocks of the node's own height (per-epoch kinds: the block the object depends on, see above, is at
  least 10 below it)

Design assumption: data is expected to be fetchable once it is 10 blocks below the node's own height and has been
missing 60 s; anything still 404 after that counts as missing. Per-kind availability delays are not modelled (design
decision).

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
- More than one watchdog (for redundancy): run separate ECS services, each with its own `LITESTREAM_REPLICA_URL`. They
  are independent; the Datadog monitor groups by node, so they raise one alert.
- Local test: `-e LITESTREAM_REPLICA_URL=file:///replica -v <dir>:/replica`.
- Litestream is installed from the pinned release `.deb` (`LITESTREAM_VERSION` in the Dockerfile).

## Known limitations

- The missing set lives in memory and SQLite is its write-through image, loaded once at startup. Adjacent missing ranges
  of one kind with equal state show as one row on the page. A retried row splits into pieces of at most 100 heights and
  stays split, so the page lists the pieces; a sweep chunk across two pieces is searched again by bisection, and the
  units keep their state. The link probes the first 100 heights of the row.
- The page shows what each node watch last published, so its ages are as of that publish, at most one loop step (follow,
  retry or sweep) old. Before a node has started it reads `not started`.
- Archive full sweep is slow: each 100-height chunk costs 4 probes plus the epoch probes whose block lies in it, each
  `request_interval` plus response time, and every loop iteration also spends time on follow and retries. Each gap in a
  range kind adds about 7 + 2 x log2(gap length) HEADs. The tip is covered by follow in real time. The page ETA and
  `sweep_eta_seconds` show the live rate.
- Probes start fetches on the watched node, so the watchdog also repairs. Missing age means "not fetchable for that
  long".
- Exact gaps assume one contiguous gap per search step: holes inside a gap are counted as missing up to the first
  present height found. A failure other than 404 (500, timeout, transport) leaves the rest of the range failed, an upper
  bound. A retry of an exact gap probes the first and the last height of each retried piece of at most 100 heights and
  keeps the piece while both are missing: heights repaired in the middle stay counted, and the row keeps its age and can
  still count.
- Any 404 counts, also one that is not the query service's (a proxy page, an unmounted route, a wrong base URL): such a
  node turns red after 3 404s and the age.
- Any non-200 counts as missing in the page and `missing` (404, 500, timeout, transport). Only confirmed 404s count in
  `confirmed_missing`. An overloaded or unreachable node shows as missing data. `fetch_errors_total{status}` separates
  the causes, except a `state_cert` step 2 failure that counts as not found (404, 500, timeout), which is recorded
  as 404.
- One red per node: a new gap on a node that is already red does not page again.
- `confirmed_missing` sums heights and epochs and can overstate until the representative is found present or the sweep
  re-checks the confirmed rows.
- Old-epoch `stake_table` probes return 500 and start a stake-table catchup on the node. They resolve within about 20 s.
  They never count (only 404 does), but an archive sweep triggers one catchup per historical epoch and a
  `node unhealthy` WARN each minute while they fail.
- The node's state cert route cancels its peer fetch when the client disconnects and answers 500 instead of 404 when no
  peer has the cert; both are node issues. The 5 s timeout can cancel a fetch that would have finished.
- Not checked: data contents, aggregates (transaction count, payload size), merklized-state history (archive nodes prune
  it), transactions index, namespace proofs, reward tree, `chain_config`, `vid_share`.
- `--retention-blocks` must not exceed the node's own retention, or pruned heights show as missing.
- State is per node URL. Rows of nodes no longer in `--nodes` are forgotten at startup. Pointing the same URL or state
  file at another network is user error: use `--reset` (deletes the state at startup, always with a WARN, so pass it
  once) or another `--db-path`.
- Without `LITESTREAM_REPLICA_URL` the state is lost when the task restarts. `--reset` deletes rows, so litestream
  replicates the reset; to retire a replica, point `LITESTREAM_REPLICA_URL` at a new path.
- Node URLs must not carry credentials (a startup error): the URL appears on the page and in debug logs.
- Route paths and the 100-height chunk are hard-coded copies of the server's routes and limits.
- The sweep ETA uses the rate since this process started sweeping and resets on restart.
- A restart loop at startup can mean every watched node is unreachable; once running, an outage shows as
  `node unhealthy` and missing data instead.
- A lagging or stuck node never turns red: nothing above its own height is probed or counted. It shows in
  `node_lag_blocks` and, when it stops advancing, in the follow lag against its own height staying flat.
- A node that only times out or answers 500 never turns red, since no missing data is confirmed. It shows as a
  `node unhealthy` WARN and in `fetch_errors_total{status}`; the companion monitor below pages on those.
- Request timeouts are per request kind (range HEAD 60 s, single HEAD 10 s, JSON GET 5 s, peer-fetch HEAD 5 s); a
  hanging node holds its loop for that long per request, while gauges and the `summary` line continue.

## Datadog

JSON logs (`RUST_LOG_FORMAT=json`) put event fields under `fields`. Metric names below are as Datadog receives them:
`<namespace>` is set in the OpenMetrics check config (scrape of `/status/metrics`, every 15 s by default), gauges keep
their name, and counters lose `_total` and gain `.count`.

Recommended alert, with a link to the watchdog page:

1. Monitors > New Monitor > Metric:
   - Query: `max(last_5m):max:<namespace>.query_watchdog_confirmed_missing{*} by {node} > 0`.
   - Multi alert by `node`. The monitor resolves once the gauge has been 0 for the window, 5 minutes after the last
     confirmed unit is repaired (the representative is found present within a minute, then the recovery queue re-checks
     the confirmed rows, see Retries).
2. Message:
   - Text: `{{node.name}} has {{value}} confirmed missing units: https://<watchdog host>/`.
   - Notify target: `@slack-<channel>`.

Companion monitors (Monitors > New Monitor > Metric):

| Monitor                | Query                                                                                               | Threshold                     |
| ---------------------- | --------------------------------------------------------------------------------------------------- | ----------------------------- |
| Watchdog down or stuck | `sum(last_10m):sum:<namespace>.query_watchdog_probes.count{*} by {node}.as_count()`                 | equal 0, or notify on no data |
| Fetch errors           | `sum(last_10m):sum:<namespace>.query_watchdog_fetch_errors.count{!status:404} by {node}.as_count()` | above `<errors>`              |
| Node lag               | `max(last_5m):max:<namespace>.query_watchdog_node_lag_blocks{*} by {node}`                          | above `<blocks>`              |
| Tip flat               | `change(max(last_10m),last_10m):max:<namespace>.query_watchdog_tip_height{*}`                       | equal 0                       |
| Follow lag             | `max(last_5m):max:<namespace>.query_watchdog_follow_lag_blocks{*} by {node}`                        | above `<blocks>`              |
| Merklized state lag    | `max(last_5m):max:<namespace>.query_watchdog_merklized_state_lag_blocks{*} by {node}`               | above `<blocks>`              |

## Schema

- Migrations are in `migrations/` and run at startup (`sqlx::migrate!`).
- Add a new `NN_name.sql`. Never edit an applied migration. `01_init.sql` was edited in place before the first deploy
  (the `aggregate` kind left its `kind` CHECK); from the first deploy on this rule holds. A state file from an earlier
  build fails at startup in the migration checksum; delete it.
- `missing_chunk` holds one row per range of a range kind (`leaf`, `block`, `vid_common`) and one row per height or
  epoch for the other kinds (`range_end = range_start + 1`). CHECK constraints reject unknown kinds, empty ranges and
  retry delays outside 5 to 600 s.
- `node_progress` holds `sweep_start_height` with the other sweep heights; CHECK constraints keep the sweep columns
  consistent.
