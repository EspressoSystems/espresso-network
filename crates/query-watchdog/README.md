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
| `--retention-blocks` | `RETENTION_BLOCKS` | absent: archive, sweep `[0, tip)`; N: sweep `[tip - N, tip)` |
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
- `ESPRESSO_QUERY_WATCHDOG_DB_PATH` (default `/data/state.sqlite`) is the state file.
- `LITESTREAM_REPLICA_URL` is optional. When set, litestream restores and replicates the file.
- One task per replica path. Two writers corrupt the replica.

## Datadog

JSON logs (`RUST_LOG_FORMAT=json`) put event fields under `fields`.

1. Log setup: facets on `@fields.node` and `@fields.kind`, and a message remapper on `fields.message`.
2. Either a log monitor or a metric monitor for missing data:
   - Log monitor, multi alert grouped by `@fields.node,@fields.kind`.
     - Query: `service:<svc> status:warn "missing data"`.
     - Message: `{{[@fields.node].name}} {{[@fields.kind].name}} missing for {{log.attributes.[fields.oldest_age_seconds]}} s: {{log.attributes.[fields.url]}}`.
     - One WARN per node and kind per minute while the oldest missing range is older than `--warn-missing-after`. `url` is the GET URL of that range.
     - Syntax: docs.datadoghq.com/monitors/notify/variables/ (no `@` in `log.attributes`, brackets around dotted keys).
   - Metric monitor, no link: `max(last_5m):max:query_watchdog_oldest_missing_age_seconds{*} by {node,kind} > 60`. The metric name carries the namespace of the Datadog openmetrics check config.
3. Lag and liveness:
   - `max(last_5m):max:query_watchdog_follow_lag_blocks{*} by {node} > <blocks>`, and the same for `query_watchdog_merklized_state_lag_blocks`.
   - No-data monitor, or a rate of 0 on `query_watchdog_probes_total` per node: the watchdog is down or stuck.

## Schema

- Migrations are in `migrations/` and run at startup (`sqlx::migrate!`).
- Add a new `NN_name.sql`. Never edit an applied migration.
