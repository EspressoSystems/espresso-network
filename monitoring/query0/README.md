# query0 node monitoring (testnet/main)

Robust, **unattended** monitoring for the `query0` node, built to survive the
ephemeral Claude-Code-on-the-web container (which does **not** persist auth or
keep a process alive across resets, and has no built-in scheduler).

Two independent pieces, neither of which depends on a Claude session staying
alive:

| # | What                          | Mechanism                                              | Runs where        |
| - | ----------------------------- | ------------------------------------------------------ | ----------------- |
| 1 | **Timeout spike alert**       | Native Datadog monitor → Slack                         | Datadog (24/7)    |
| 2 | **Daily 08:00 health report** | Scheduled Claude session runs the digest + code review | Triggered session |

Source filter used everywhere (the pattern from the original request):

```
@aws.awslogs.logGroup:"/testnet/main/" @aws.awslogs.logStream:*0/query*
```

> `*0/query*` matches any stream ending in `0` before `/query` (`ecs/0/query`,
> `ecs/10/query`, …). Use `ecs/0/query*` for strictly node 0.

---

## 1. Timeout spike alert (native Datadog monitor)

**Requirement:** *alert on Slack when timeouts exceed the average of the last 24h.*

"Above the trailing-24h average" is a **dynamic baseline**, so the faithful
implementation is a Datadog **anomaly** monitor. A simple static-threshold
monitor is also provided as a zero-dependency quick start.

### Path 1 — anomaly monitor (recommended; dynamic baseline)

1. Create the log-based metric `espresso.query0.view_timed_out`
   (see [`datadog/log_metric.md`](datadog/log_metric.md)). `pup` can't create
   it; use the UI or API. It needs a few days of history to calibrate.
2. Edit [`datadog/timeout_anomaly_monitor.json`](datadog/timeout_anomaly_monitor.json):
   replace `@slack-REPLACE_WITH_YOUR_DD_SLACK_HANDLE` in `message` with your
   configured Datadog→Slack handle (see "Slack wiring" below).
3. Create it:
   ```bash
   pup monitors create monitoring/query0/datadog/timeout_anomaly_monitor.json
   ```
   (`pup` must be authenticated with a key/role that has **monitors_write**; the
   read-only login used for exploration cannot create monitors.) Or import the
   JSON via the Datadog UI.

The query flags an anomaly when the hourly `view timed out` count rises above its
expected band (algorithm `agile`, `seasonality='daily'`, `direction='above'`,
sustained over `last_1h`).

### Path 2 — static-threshold monitor (works immediately, no metric)

1. Get the real baseline so the threshold is sane:
   ```bash
   monitoring/query0/collect_query0_digest.sh 24h   # see the "view_timed_out" totals
   ```
   Set the threshold to roughly the 24h **hourly average** (or a small multiple,
   to taste).
2. Edit [`datadog/timeout_threshold_monitor.json`](datadog/timeout_threshold_monitor.json):
   set both the number in `query` (`... last("1h") > N`) **and**
   `options.thresholds.critical` to your `N` (and `warning` if you want one), and
   replace the Slack handle.
3. Create it:
   ```bash
   pup monitors create monitoring/query0/datadog/timeout_threshold_monitor.json
   ```

> The shipped JSON uses a **placeholder** `N = 50` — tune it before relying on it.

### Slack wiring

Datadog posts to Slack via the Datadog **Slack integration handle**, written
`@slack-<account>-<channel>` in the monitor `message` (it is **not** a Slack
user ID). Configure the integration once at *Datadog → Integrations → Slack*,
pick the destination channel, then put that handle in the monitor message. To
reach Lucas specifically, create/choose a channel he's in (DMs from monitors are
not supported the way channel posts are).

---

## 2. Daily 08:00 report (scheduled Claude session)

08:00 Europe/Warsaw (Lucas's tz) = **06:00 UTC**. Produces a health summary and,
on any issue, an in-depth **code-level** analysis against this repo, then DMs
Lucas (`U08HBM4HER4`) on Slack.

- Prompt the session runs: [`REPORT_PROMPT.md`](REPORT_PROMPT.md)
- Data collection it calls: [`collect_query0_digest.sh`](collect_query0_digest.sh)

### a) Persistent Datadog auth (required for unattended runs)

OAuth login does not survive container resets. Use API + App keys instead:

1. In Datadog, create an **API key** and an **Application key**. For least
   privilege, scope the App key (via a role) to read-only:
   `logs_read_data`, `logs_read_index_data`, `metrics_read`, `monitors_read`.
2. Add these as **environment variables / secrets** in your Claude Code web
   environment configuration (so every session, including scheduled ones, gets
   them):
   ```
   DD_API_KEY=<api key>
   DD_APP_KEY=<app key>
   DD_SITE=datadoghq.com
   ```
   Docs: https://code.claude.com/docs/en/claude-code-on-the-web
3. Verify in any session: `pup auth test` → should show `API Key: set`.

### b) Schedule the trigger

Create a **scheduled session** (cron) for this repo in Claude Code on the web,
daily at `0 6 * * *` (UTC), with a prompt such as:

> Follow the instructions in `monitoring/query0/REPORT_PROMPT.md`.

(See the Claude-Code-on-the-web docs above for creating scheduled/triggered
sessions.)

---

## What I need from you (checklist)

- [ ] Add `DD_API_KEY` / `DD_APP_KEY` / `DD_SITE` to the web environment config.
- [ ] Configure the Datadog→Slack integration and note the channel handle.
- [ ] Pick a path for the alert (anomaly = recommended; threshold = quickest) and
      create the monitor — or tell me to (see below).
- [ ] Create the daily `0 6 * * *` scheduled session pointing at `REPORT_PROMPT.md`.

## Want me to create the Datadog monitor for you?

Once a **monitors_write**-capable key is in place (either set `DD_API_KEY`/
`DD_APP_KEY` with write, or do a one-time `pup auth login --scopes
monitors_write,monitors_read,logs_read_data,metrics_read`), tell me and I will:
fill in the real threshold from live baseline data, set the Slack handle you give
me, run `pup monitors create`, and confirm it's live. I'll also pull the current
24h baseline so the report's first run has context.
