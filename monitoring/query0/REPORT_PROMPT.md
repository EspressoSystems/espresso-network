# Scheduled task: daily query0 node report (08:00 Europe/Warsaw = 06:00 UTC)

You are running as a **scheduled, non-interactive Claude Code session** in the
espresso-network repo. Your job is to report on the health of the testnet/main
**query0** node over the last 24 hours and, if anything went wrong, explain it at
the code level. Then deliver the report to Lucas on Slack.

## 0. Preconditions

- `pup` must be authenticated non-interactively. Verify with `pup auth test`; it
  should show `API Key: set`. If it shows `not set`, the `DD_API_KEY` /
  `DD_APP_KEY` / `DD_SITE` env vars are missing — do **not** attempt an
  interactive `pup auth login` (there is no browser). Instead post a short Slack
  message to Lucas (`U08HBM4HER4`) saying the daily report was skipped because
  Datadog credentials are not configured, and stop.

## 1. Collect the data

```bash
bash monitoring/query0/collect_query0_digest.sh 24h | tee /tmp/query0_digest.txt
```

If you want finer detail on any signal, run `pup logs search`/`pup logs aggregate`
directly (see `nix/pup/README.md`). The base filter is:

```
@aws.awslogs.logGroup:"/testnet/main/" @aws.awslogs.logStream:*0/query*
```

## 2. Decide overall status

- 🟢 **Healthy** — normal volume, no `error`-status spike, `view timed out` in
  the normal range, no panics/restarts, no `Too many consecutive timeouts`.
- 🟡 **Degraded** — elevated `view timed out` and/or some `Starting view sync
  protocol`, isolated errors, but the node kept making progress.
- 🔴 **Incident** — any `Too many consecutive timeouts! This shouldn't happen`,
  panics/crashes/restarts, sustained error spikes, or a gap in logs (node down).

## 3. If there were issues, do CODE-LEVEL root-cause analysis

This is the most important part. For every issue, explain the mechanism using the
actual espresso-network source — cite `path:line`. Key anchors for timeouts:

- `crates/hotshot/task-impls/src/view_sync.rs:540-585` — the `Timeout` handler.
  `num_timeouts_tracked` increments per consecutive timeout; **≥2** →
  `Starting view sync protocol` (error) and view-sync is triggered; **≥3** →
  `Too many consecutive timeouts! This shouldn't happen` (error). One
  `view timed out` (warn) = one view that missed its deadline.
- `crates/hotshot/types/src/error.rs` (`ViewTimedOut`) and
  `crates/hotshot/types/src/event.rs` (`ViewTimeout`) — the types involved.
- For other errors, grep the repo for the exact log string to find the emitting
  code, then read enough surrounding context to explain *why* it fired and what
  conditions cause it (e.g. leader down, network partition, L1/builder issues,
  storage errors). Use `crates/espresso/node/` and `crates/hotshot/` and the
  architecture notes in `CLAUDE.md`.

Correlate timing: do the timeouts cluster around specific views/leaders
(`leader_mnemonic` field) or a time window? Did they coincide with errors,
restarts, or a log gap? Form a concrete hypothesis, not a generic one.

## 4. Deliver the report to Slack (DM Lucas, `U08HBM4HER4`)

Send via the Slack tool (`slack_send_message`, channel_id `U08HBM4HER4`). Keep it
skimmable; put depth in the analysis section only when there is an issue.

Template:

```
*query0 daily report — <YYYY-MM-DD>, last 24h*  <🟢|🟡|🔴>

*Headline:* <one line>

*Metrics (24h)*
• Total events: <n>
• Status mix: <info/warn/error counts>
• view timed out: <n>  (by window: 1h <n> / 6h <n> / 24h <n>)
• Starting view sync protocol: <n>   • Too many consecutive timeouts: <n>
• Errors: <n>   • Panics/restarts: <n>

*What happened*
<2–5 sentences on the day. If healthy, say so plainly.>

*Issue analysis* (only if 🟡/🔴)
<in-depth, code-level: cite path:line, explain the mechanism and the most likely
cause, with the evidence from logs that supports it>

*Recommendation*
<actionable next step, or "none — node healthy">
```

If status is 🔴, also start the message with `<!here>` is **not** appropriate for
a DM — just send it; consider noting urgency in the headline.

## 5. Notes

- Read-only Datadog access; never attempt to modify Datadog or the node.
- This repo is fresh each run; do not rely on local state from prior runs.
- Keep the whole run within a few minutes; the digest script is the slow part.
