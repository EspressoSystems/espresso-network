# pup

CLI companion for Datadog. Packaged here so it's available in the dev shell.

Upstream: https://github.com/datadog-labs/pup

## Authentication (read-only)

Log in with read-only scopes only. This is enough for logs/metrics/monitors/dashboards exploration and cannot modify
Datadog state:

```bash
pup auth login --scopes metrics_read,logs_read_data,monitors_read,dashboards_read
```

- Access tokens last 1 hour; pup auto-refreshes them.
- Refresh tokens have a short lifetime set by Datadog; re-run `pup auth login` when it expires.
- `pup auth refresh` forces a refresh on demand.

## Examples

### Logs

Search for errors on a specific host/log stream:

```bash
pup logs search --query='host:"/testnet/decaf/" @aws.awslogs.logStream:ecs/3/query/* "failed to update state"'
```

Broader search with boolean operators:

```bash
pup logs search --query='host:"/testnet/decaf/" @aws.awslogs.logStream:ecs/3/query/* ("failed to update state" OR "fetching reward merkle tree" OR "migrating")'
```

Count logs:

```bash
pup logs aggregate --compute=count --query='host:"/testnet/decaf/" "failed to update state"'
```
