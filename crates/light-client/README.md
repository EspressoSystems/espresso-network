# light-client

Verifies Espresso data fetched from untrusted query nodes against the stake table, and stores what it verified. A
[follower](../../doc/light-client-follower.md) follows the chain through it, and a SQL query node fetches missing data
through it.

## Metrics

The light client reports under `light_client_` in `/v1/status/metrics`, on a follower and on any SQL query node whose
query service has fetched through it:

- `latest_stake_table_epoch`: the latest epoch with a verified stake table.
- `stake_table_backlog`: epochs left in the stake table catchups that are running, summed over catchups that can
  overlap, 0 when none is.
- `stake_table_catchup_duration`: seconds to fetch, verify and store one epoch of stake table events.
- `follower_block_height`: on a follower, one more than the latest leaf height the light client has verified, like
  `node/block-height`. It stays 0 on a query node, which never asks the light client for the chain height.
- `verification_failures`: upstream responses that failed a proof or commitment check.
- `fetch_requests`: fetches of missing data by the query service through the light client.
- `fetch_failures`: those fetches that failed, for any reason.
- `db_read_duration`, `db_write_duration`: seconds per read from and write to the light client database, including the
  wait for a connection and, for writes, the commit.

The first verification with an empty light client database replays stake tables from the first dynamic epoch, so
`stake_table_backlog` rises by the number of epochs to replay and falls to 0. On a query node the light client is built
on the first fetch of missing data, so this can happen long after startup. A light client database on a network
filesystem such as EFS shows up as slow `db_write_duration`, since every SQLite commit syncs to it.

## Alerts

Suggestions for the light client on a query node. The thresholds are starting points, to tune against what a healthy
deployment shows.

- **Verification failure.** An upstream served data that failed a proof or commitment check, or the light client has a
  bug. A healthy network never raises it.

  ```
  increase(light_client_verification_failures[15m]) > 0
  ```

- **Stake table catchup stalled.** A catchup is running but has not finished an epoch in 15 minutes. This only sees long
  catchups. One that fails within a few seconds never shows up here, but the fetch that needed it fails too, so the
  fetch alert fires instead.

  ```
  max_over_time(light_client_stake_table_backlog[15m]) > 0
    and increase(light_client_stake_table_catchup_duration_count[15m]) == 0
  ```

- **Fetches failing**, for 15 minutes. Most fetches of missing data fail. The query service retries them, so this
  matters only when it lasts. The second condition keeps a single failure on a quiet node from firing it.

  ```
  rate(light_client_fetch_failures[10m]) / rate(light_client_fetch_requests[10m]) > 0.5
    and increase(light_client_fetch_failures[10m]) > 10
  ```

- **Stake table catchup slow.** Epochs take over 10 seconds each on average. One normally takes about half a second, so
  a fresh database on a chain with thousands of epochs takes about an hour to catch up.

  ```
  rate(light_client_stake_table_catchup_duration_sum[15m])
    / rate(light_client_stake_table_catchup_duration_count[15m]) > 10
  ```

- **Slow light client storage**, for 15 minutes. The mean write takes over 2 seconds.

  ```
  rate(light_client_db_write_duration_sum[5m]) / rate(light_client_db_write_duration_count[5m]) > 2
  ```

The storage alert uses the mean rather than a quantile because the histograms use the default Prometheus buckets, which
end at 10 seconds, and Datadog ingests the buckets as plain counts it cannot take a quantile of. In Datadog, counters
arrive as `<name>.count` and the conditions joined by `and` become composite monitors.

A light client that cannot open its database exports no `light_client_` metrics at all, since they are registered once
the database opens, so none of these fire. The only sign is the log line
`light client provider unavailable, retrying on the next fetch`.
