# query-watchdog

Checks that archive query nodes serve every height, epoch object and aggregate they should, keeps the missing set in SQLite, and reports it through logs, metrics and a page.

## Run locally

```
cargo run -p query-watchdog -- --nodes https://query.decaf.testnet.espresso.network,http://localhost:24000 \
  --db-path tmp/query-watchdog/state.sqlite --retention-blocks 2000
```

- The `serving` INFO line prints the endpoint URLs.
- `/`: progress and missing ranges per node, with links to the failing GET.
- `/status/metrics`: Prometheus metrics, `query_watchdog_*`.
- `/healthcheck`: bare 200.
- Logs: `RUST_LOG` (default INFO), `RUST_LOG_FORMAT=json`.

## Options

| Flag | Env (`ESPRESSO_QUERY_WATCHDOG_`) | Default |
|---|---|---|
| `--nodes` | `NODES` | required, comma-separated base URLs without `/v1`, path ends with `/` |
| `--db-path` | `DB_PATH` | required |
| `--port` | `PORT` | 8080 |
| `--sweep-interval` | `SWEEP_INTERVAL` | 24h, time from one sweep start to the next |
| `--request-interval` | `REQUEST_INTERVAL` | 200ms, wait before each request of one node |
| `--retention-blocks` | `RETENTION_BLOCKS` | absent: archive, sweep `[0, tip)`; N: sweep `[tip - N, tip)`, start rounded up to a multiple of 100 |
| `--warn-missing-after` | `WARN_MISSING_AFTER` | 60s, age at which a `missing data` WARN is logged |

## Checked

Per 100-height chunk (follow from the tip, sweep over the window). Requests are HEAD.

| Kind | Unit | Route |
|---|---|---|
| `leaf`, `block`, `vid_common` | heights | `v1/availability/{leaf,block,vid/common}/{start}/{end}` |
| `lc_leaf_proof` | heights | `v1/light-client/leaf/{end - 1}` |
| `aggregate` | heights | `v1/node/transactions/count/{end - 1}` |
| `stake_table` | epochs | `v1/node/stake-table/{epoch}` |
| `state_cert` | epochs | `v1/catchup/{epoch}/state-cert` |
| `lc_stake_table` | epochs | `v1/light-client/stake-table/{epoch}` |

- Not checked: data contents, merklized state history, reward tree, `vid_share`, the transactions index.
- Gauges for the merklized state head lag and follow lag are exported.

## Docker

- Image: `docker/query-watchdog.Dockerfile`, entrypoint `docker/query-watchdog/entrypoint.sh`.
- The image sets `ESPRESSO_QUERY_WATCHDOG_PORT=80` and `ESPRESSO_QUERY_WATCHDOG_DB_PATH=/data/state.sqlite`.
- `LITESTREAM_REPLICA_URL` is optional. When set, litestream restores and replicates the file.
- One task per replica path. Two writers corrupt the replica.

## Known limitations

- Archive full sweep is slow: about 46 heights/s at `request_interval` 200ms (decaf, 21.9M heights: about 5.5 days). Sweeps then run back to back. The tip is covered by follow in real time.
- Probes start fetches on the watched node, so the watchdog also repairs. Missing age means "not fetchable for that long".
- `missing` counts whole chunks (up to 100 heights), an upper bound.
- Any non-200 counts as missing (404, 500, timeout, transport). An overloaded or unreachable node shows as missing data. `fetch_errors_total{status}` separates the causes.
- Old-epoch `stake_table` probes return 500 and start a stake-table catchup on the node. They resolve within about 20 s, below the 60 s alert threshold, but an archive sweep triggers one catchup per historical epoch.
- Merklized-state history is not swept (archive nodes prune it). Only the head lag is exported.
- Not checked: transactions index, namespace proofs, reward tree, `chain_config`, `vid_share`.
- `--retention-blocks` must not exceed the node's own retention, or pruned heights show as missing. The window start rounds up to a multiple of 100.
- One watchdog per replica path. Two writers corrupt the replica. Without `LITESTREAM_REPLICA_URL` the state is lost when the task restarts.
- Node URLs are assumed internal, without credentials.
- Route paths and the 100-height chunk are hard-coded copies of the server's routes and limits.
- The sweep ETA uses the rate since this process started sweeping and resets on restart.
- The tip is the max over all watched nodes. With a single node, a stuck node shows only as missing ranges, not as lag.

## Datadog

JSON logs (`RUST_LOG_FORMAT=json`) put event fields under `fields`. Metric names below are as Datadog receives them: `<namespace>` is set in the OpenMetrics check config, gauges keep their name, and counters lose `_total` and gain `.count`.

Recommended alert, with a link to the missing data:

1. Logs > Configuration: add facets on `@fields.node` and `@fields.kind`, and a message remapper with `fields.message` as source.
2. Monitors > New Monitor > Logs:
   - Query: `service:<watchdog service> status:warn "missing data"`.
   - Alert condition: count above 0 over the last 5 minutes.
   - Multi alert, grouped by `@fields.node` and `@fields.kind`.
   - The watchdog logs one WARN per node and kind per minute while the oldest missing range is older than `--warn-missing-after`.
3. Message:
   - Text: `{{[@fields.node].name}} {{[@fields.kind].name}} missing for {{log.attributes.[fields.oldest_age_seconds]}} s ({{log.attributes.[fields.missing]}} missing): {{log.attributes.[fields.url]}}`.
   - `url` is the GET URL of the oldest missing range. `missing` is in heights, or epochs for `stake_table`, `state_cert` and `lc_stake_table`.
   - Notify target: `@slack-<channel>`.
   - Syntax: docs.datadoghq.com/monitors/notify/variables/ (no `@` in `log.attributes`, brackets around dotted keys).

Companion monitors (Monitors > New Monitor > Metric):

| Monitor | Query | Threshold |
|---|---|---|
| Missing age, without the link | `max(last_5m):max:<namespace>.query_watchdog_oldest_missing_age_seconds{*} by {node,kind}` | above 60 |
| Watchdog down or stuck | `sum(last_10m):sum:<namespace>.query_watchdog_probes.count{*}.as_rate() by {node}` | equal 0, or notify on no data |
| Follow lag | `max(last_5m):max:<namespace>.query_watchdog_follow_lag_blocks{*} by {node}` | above `<blocks>` |
| Merklized state lag | `max(last_5m):max:<namespace>.query_watchdog_merklized_state_lag_blocks{*} by {node}` | above `<blocks>` |

## Schema

- Migrations are in `migrations/` and run at startup (`sqlx::migrate!`).
- Add a new `NN_name.sql`. Never edit an applied migration.
