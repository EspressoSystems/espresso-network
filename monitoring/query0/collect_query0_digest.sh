#!/usr/bin/env bash
#
# collect_query0_digest.sh — Gather a digest of the espresso testnet/main "query0"
# node logs from Datadog (via pup) for the daily report. READ-ONLY.
#
# Usage:
#   monitoring/query0/collect_query0_digest.sh [WINDOW]
#     WINDOW   Datadog relative range, default "24h" (e.g. 24h, 6h, 48h).
#
# Auth:
#   Requires pup to be authenticated. For unattended/scheduled runs, set
#   DD_API_KEY, DD_APP_KEY, DD_SITE as environment variables (persistent auth);
#   see monitoring/query0/README.md. For interactive runs, `pup auth login`.
#
# Output:
#   A human- and LLM-readable digest on stdout. Best-effort: a single failing
#   query never aborts the whole run; missing pieces are reported as 0/empty.
#
# Note on the stream filter: `*0/query*` is the pattern from the original request.
# It matches any log stream ending in "0" before "/query" (ecs/0/query, ecs/10/query,
# ecs/20/query, ...). Narrow to `ecs/0/query*` if you want strictly node 0.

set -uo pipefail

WINDOW="${1:-24h}"
BASE='@aws.awslogs.logGroup:"/testnet/main/" @aws.awslogs.logStream:*0/query*'

# --- tolerant count extractor: reads pup JSON on stdin, prints one integer ---
_count_from_json() {
  python3 - <<'PY'
import sys, json
try:
    data = json.load(sys.stdin)
except Exception:
    print(0); raise SystemExit
total = 0; found = False
def walk(o):
    global total, found
    if isinstance(o, dict):
        for k, v in o.items():
            if isinstance(v, (int, float)) and (k in ("count", "c0") or str(k).lower().endswith("count")):
                total += v; found = True
            else:
                walk(v)
    elif isinstance(o, list):
        for v in o:
            walk(v)
walk(data)
print(int(total) if found else 0)
PY
}

count() { # count "<extra query>" -> integer over $WINDOW
  local extra="${1:-}" q="$BASE"
  [ -n "$extra" ] && q="$BASE $extra"
  pup logs aggregate --compute=count --query="$q" --from="$WINDOW" --to=now --output=json 2>/dev/null | _count_from_json
}

count_window() { # count_window "<extra>" "<window>" -> integer over <window>
  local extra="${1:-}" win="${2:-1h}" q="$BASE"
  [ -n "$extra" ] && q="$BASE $extra"
  pup logs aggregate --compute=count --query="$q" --from="$win" --to=now --output=json 2>/dev/null | _count_from_json
}

sample() { # sample "<extra>" "<n>" -> raw JSON (truncated)
  local extra="${1:-}" n="${2:-15}" q="$BASE"
  [ -n "$extra" ] && q="$BASE $extra"
  pup logs search --query="$q" --from="$WINDOW" --to=now --limit "$n" --output=json 2>/dev/null | head -c 12000
}

echo "# query0 digest (testnet/main)  window=${WINDOW}  generated=$(date -u +%FT%TZ)"
echo
echo "Base filter: ${BASE}"
echo

echo "## auth"
pup auth status --output json 2>/dev/null | head -c 400 || echo "(pup auth status unavailable)"
echo; echo

echo "## volume"
echo "total_events_${WINDOW}: $(count '')"
echo
echo "### status breakdown (${WINDOW})"
pup logs aggregate --compute=count --group-by=status --query="$BASE" --from="$WINDOW" --to=now --output=table 2>/dev/null | head -20 || echo "(unavailable)"
echo

echo "## timeout signals (see crates/hotshot/task-impls/src/view_sync.rs)"
echo "view_timed_out (total ${WINDOW}): $(count '"view timed out"')"
echo "view_timed_out by window:"
for w in 1h 3h 6h 12h 24h; do
  printf '  last_%-4s %s\n' "$w" "$(count_window '"view timed out"' "$w")"
done
echo "starting_view_sync (error, >=2 consecutive): $(count '"Starting view sync protocol"')"
echo "too_many_consecutive (error, >=3 consecutive): $(count '"Too many consecutive timeouts"')"
echo

echo "## errors & crashes"
echo "status:error count: $(count 'status:error')"
echo "panic-ish count:    $(count '(panicked OR SIGABRT OR SIGSEGV)')"
echo
echo "### sample error logs (up to 20)"
sample 'status:error' 20
echo; echo
echo "### sample panic/abort logs (up to 10)"
sample '(panicked OR SIGABRT OR SIGSEGV)' 10
echo; echo

echo "# end of digest"
