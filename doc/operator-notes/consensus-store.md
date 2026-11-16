### New required setting: consensus store directory

Urgency: High

Every node now needs `ESPRESSO_NODE_CONSENSUS_STORE_PATH` (or `--consensus-store-path`): a directory on local, persistent
disk. The node keeps its votes, locks, undecided proposals and block payloads there, and waits for writes to it before it
votes, so a slow or network-attached disk slows down voting. A node without the setting does not start.

On the first start with the setting, the node copies this state from its existing storage (`storage-fs` or
`storage-sql`) into the directory, and refuses to start if that fails. From then on, consensus keeps this state only in
the directory. Keep the directory across restarts and upgrades: a node that loses it can vote twice in a view. Do not
point two nodes at the same directory.

### Removed settings: consensus storage pruning and the proposal fetcher

Urgency: Medium

The old consensus storage no longer receives new data, so its pruning settings are gone, and so is the proposal fetcher.
Remove these from the node's configuration:

- `--consensus-view-retention` / `ESPRESSO_NODE_CONSENSUS_VIEW_RETENTION`
- `--consensus-storage-target-retention` / `ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_RETENTION`
- `--consensus-storage-minimum-retention` / `ESPRESSO_NODE_CONSENSUS_STORAGE_MINIMUM_RETENTION`
- `--consensus-storage-target-usage` / `ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_USAGE`
- `--consensus-storage-usage-check-interval` / `ESPRESSO_NODE_CONSENSUS_STORAGE_USAGE_CHECK_INTERVAL`
- `--proposal-fetcher-num-workers` / `ESPRESSO_NODE_PROPOSAL_FETCHER_NUM_WORKERS`
- `--proposal-fetcher-fetch-timeout` / `ESPRESSO_NODE_PROPOSAL_FETCHER_FETCH_TIMEOUT`

A node started with one of the flags refuses to start; the environment variables are ignored. The config endpoints no
longer report these settings, and the `decide_processor_*`, `proposal_fetcher_*` and
`internal_process_decided_events_duration` metrics are gone.
