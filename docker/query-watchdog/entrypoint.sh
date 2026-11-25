#!/usr/bin/env bash
set -euo pipefail

: "${ESPRESSO_QUERY_WATCHDOG_DB_PATH:?}"

mkdir -p "$(dirname "${ESPRESSO_QUERY_WATCHDOG_DB_PATH}")"

if [[ -z "${LITESTREAM_REPLICA_URL:-}" ]]; then
  echo "LITESTREAM_REPLICA_URL not set, running without replication;" \
    "state is lost when the task restarts" >&2
  exec /bin/query-watchdog
fi

litestream restore -if-db-not-exists -if-replica-exists \
  "${ESPRESSO_QUERY_WATCHDOG_DB_PATH}"

exec litestream replicate -exec /bin/query-watchdog
