# espresso-telemetry

In-process node-operator telemetry. Opt-in via `--telemetry-enable` (default off).

## What we send

- **Logs**: every `tracing` event passing the OTel layer's `EnvFilter` (default `warn`;
  `ESPRESSO_TELEMETRY_LOG=warn,hotshot=info` to widen). Bridged via `opentelemetry-appender-tracing`.
- **Metrics**: every series in the node's `prometheus::Registry` — counters, gauges, histograms (summary/untyped
  rejected at encode).
- **Identity per request**: BLS-BN254 JWT signed by the staking key. Optional `node_name` / `company_name` claims.
  `service.instance.id = node_name` on log records.

## How we send it

| Signal  | Protocol          | Path            | Encoding                         | Cadence                         |
| ------- | ----------------- | --------------- | -------------------------------- | ------------------------------- |
| Logs    | OTLP/HTTP         | `/v1/logs`      | `application/x-protobuf`, gzip   | `BatchLogProcessor` (1s / ≤512) |
| Metrics | Prom remote-write | `/api/v1/write` | `application/x-protobuf`, snappy | `metrics_interval_secs` (60s)   |

- One base endpoint (`--telemetry-endpoint`), one JWT minted at startup, on `Authorization: Bearer <jwt>` for both
  signals.
- Logs: bounded mpsc inside the OTel layer (`try_send`; drop on full). Export retries 4× with exponential backoff (250ms
  / 1s / 4s) on transport errors and 5xx; 4xx and `AlreadyShutdown` are not retried. Survives ~1-5s proxy/aggregator
  restarts without loss.
- Metrics: dedicated `espresso-telemetry-metrics` thread with its own current-thread tokio runtime. Scrape → encode →
  snappy → POST. One final flush on shutdown.
- Neither path uses disk. Neither blocks consensus or the node's tokio runtime — log export runs on the OTel SDK's own
  thread; metrics on the dedicated thread above. Errors `warn!` only.

## Why

- **OTLP/HTTP for logs**: structured fields preserved end-to-end so the aggregator (Vector) can fan out to multiple
  sinks (S3, Datadog, …) without re-encoding. HTTPS-friendly through ALB.
- **Prom remote-write for metrics**: ships the existing `prometheus::Registry` unchanged. The alternative —
  re-instrumenting every call site against the OTel meter SDK — would be a much bigger diff for no functional gain.
- **Single auth proxy** in front of Vector verifies JWTs against the on-chain stake table. Vector stays unauthenticated;
  operators configure one URL plus a key they already have.
- **In-process push**: no operator-side agent (Vector / Otel Collector / node_exporter). Push works through
  NAT/firewalls; pull would need a scrape endpoint.
- **Opt-in, no PII**: identity is the staking key (already public); shipping is the operator's choice.

## Not in scope

- Token rotation. JWT TTL is 6 months and is **not refreshed mid-process** — restart-driven. Tighter window: lower the
  proxy's `--token-max-age` and accept the rotation cost.
- Disk buffering. Sustained outages drop events at the batch cadence.
- Tracing spans, profiling, events. Logs and metrics only.
- Non-Espresso sinks. The endpoint is the espresso auth proxy.
