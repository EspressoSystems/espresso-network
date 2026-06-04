# Log-based metric: `espresso.query0.view_timed_out`

The **anomaly** monitor (`timeout_anomaly_monitor.json`) alerts on this metric.
`pup` cannot create log-based metrics (its `logs metrics` subcommand is
read-only: `get`/`list`/`delete` only), so create this one of two ways.

## Definition

| Field        | Value                                                                            |
| ------------ | -------------------------------------------------------------------------------- |
| Metric name  | `espresso.query0.view_timed_out`                                                 |
| Filter query | `@aws.awslogs.logGroup:"/testnet/main/" @aws.awslogs.logStream:*0/query* "view timed out"` |
| Calculation  | Count of all matching logs (`count`)                                             |
| Group by     | (none)                                                                           |

## Option A — Datadog UI

1. **Logs → Configuration → Generate Metrics → New Metric**.
2. Define the filter query above; set calculation to **Count**; name it
   `espresso.query0.view_timed_out`. Save.
3. New log-based metrics start collecting from creation time; the anomaly monitor
   needs a few days of history to calibrate its baseline. Until then, rely on the
   static-threshold monitor (`timeout_threshold_monitor.json`).

## Option B — API (needs an App key with `logs_write_metrics`)

```bash
curl -sS -X POST "https://api.${DD_SITE:-datadoghq.com}/api/v2/logs/config/metrics" \
  -H "DD-API-KEY: ${DD_API_KEY:?}" \
  -H "DD-APPLICATION-KEY: ${DD_APP_KEY:?}" \
  -H "Content-Type: application/json" \
  -d '{
    "data": {
      "type": "logs_metrics",
      "id": "espresso.query0.view_timed_out",
      "attributes": {
        "compute": { "aggregation_type": "count" },
        "filter": { "query": "@aws.awslogs.logGroup:\"/testnet/main/\" @aws.awslogs.logStream:*0/query* \"view timed out\"" },
        "group_by": []
      }
    }
  }'
```

Verify afterwards with: `pup logs metrics get espresso.query0.view_timed_out`.
