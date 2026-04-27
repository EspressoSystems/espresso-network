# Pruning Issue Reproduction

Reproducing proposal timeouts caused by query service pruning.

## Run the demo

Use the process-compose override to enable pruning and slow delete triggers on node-0:

```bash
just demo-native-drb-header -f scripts/pruning-debug/process-compose.override.yaml --tui=false 2>&1 | tee tmp/log-prune.txt
```

## Slow delete triggers

`slow-delete-triggers.sql` adds `pg_sleep` triggers to tables pruned by the query service. Default delay is 0.05s per
deleted row. The override file installs them automatically.

Manual install (after tables are created):

```bash
PGPASSWORD=password psql -h localhost -p 5432 -U root -d espresso -f scripts/pruning-debug/slow-delete-triggers.sql
```

Override delay for a specific table (new connections only):

```bash
PGPASSWORD=password psql -h localhost -p 5432 -U root -d espresso -c 'ALTER DATABASE espresso SET "slow_delete.header" = '\''1'\'';'
```

Override delay globally (all connections after reload):

```bash
PGPASSWORD=password psql -h localhost -p 5432 -U root -d espresso -c 'ALTER SYSTEM SET "slow_delete.header" = '\''1'\'';'
PGPASSWORD=password psql -h localhost -p 5432 -U root -d espresso -c 'SELECT pg_reload_conf();'
```

Reset:

```bash
PGPASSWORD=password psql -h localhost -p 5432 -U root -d espresso -c 'ALTER SYSTEM RESET "slow_delete.header";'
PGPASSWORD=password psql -h localhost -p 5432 -U root -d espresso -c 'SELECT pg_reload_conf();'
```

## Monitor postgres activity

Log active queries every 2 seconds:

```bash
env PGPASSWORD=password bash -c 'while true; do echo "--- $(date -Iseconds) ---"; psql -h localhost -p 5432 -U root -d espresso -c "SELECT pid, state, wait_event_type, wait_event,
                    left(query, 80) as query, now() - query_start as duration FROM pg_stat_activity WHERE datname = '\''espresso'\'' AND state != '\''idle'\'' ORDER BY query_start"; sleep 2; done' 2>&1 | tee tmp/pg_activity.log
```

## Useful log queries

Slow header queries and timeouts:

```bash
rg -a "slow state.*max\(height\)|fetching missing acc|timed out" tmp/log-prune.txt
```

Pruner activity:

```bash
rg -a "pruner.*(slow statement|Pruned to|pruner run|error)" tmp/log-prune.txt
```

Catchup timeouts:

```bash
rg -a "local provider timed out" tmp/log-prune.txt
```

Node-0 timeout votes:

```bash
rg -a "espresso-node-0.*sending timeout vote" tmp/log-prune.txt
```

SafeSnapshot waits in pg_activity log:

```bash
grep SafeSnapshot tmp/pg_activity.log
```
