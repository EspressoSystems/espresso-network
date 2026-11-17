### New required setting: consensus store directory

Urgency: High

Every node now needs `ESPRESSO_NODE_CONSENSUS_STORE_PATH` (or `--consensus-store-path`): a directory on local, persistent
disk. The node keeps its votes, locks, undecided proposals and block payloads there, and waits for writes to it before it
votes, so a slow or network-attached disk slows down voting. A node without the setting does not start.

On the first start with the setting, the node copies this state from its existing storage (`storage-fs` or
`storage-sql`) into the directory, and refuses to start if that fails. From then on, consensus keeps this state only in
the directory. Keep the directory across restarts and upgrades: a node that loses it can vote twice in a view. Do not
point two nodes at the same directory.

### The query module is required, and a full decide log stops voting

Urgency: High

A node now refuses to start without the `query` module. Consensus keeps decided blocks in a decide log in the consensus
store until the query module's ingester has stored them, and only the ingester frees them.

The decide log may take up `ESPRESSO_NODE_DECIDE_LOG_LIMIT` bytes (or `--decide-log-limit`, default 10 GiB). When it is
full, because the ingester is down or falls behind, the node keeps running but stops sending its second-round votes,
and resumes once the ingester frees entries. Alert on `consensus_storage_decide_log_bytes` well before
`consensus_storage_decide_log_limit`, and make room for the limit on the disk of the consensus store directory.

### Removed settings: consensus storage pruning, the proposal fetcher and local catchup

Urgency: Medium

The old consensus storage no longer receives new data, so its pruning settings are gone, and so is the proposal fetcher.
Consensus restarts from the decided state in the consensus store and no longer reads state from the node's database, so
the timeout for those reads is gone too.
Remove these from the node's configuration:

- `--consensus-view-retention` / `ESPRESSO_NODE_CONSENSUS_VIEW_RETENTION`
- `--consensus-storage-target-retention` / `ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_RETENTION`
- `--consensus-storage-minimum-retention` / `ESPRESSO_NODE_CONSENSUS_STORAGE_MINIMUM_RETENTION`
- `--consensus-storage-target-usage` / `ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_USAGE`
- `--consensus-storage-usage-check-interval` / `ESPRESSO_NODE_CONSENSUS_STORAGE_USAGE_CHECK_INTERVAL`
- `--proposal-fetcher-num-workers` / `ESPRESSO_NODE_PROPOSAL_FETCHER_NUM_WORKERS`
- `--proposal-fetcher-fetch-timeout` / `ESPRESSO_NODE_PROPOSAL_FETCHER_FETCH_TIMEOUT`
- `--local-catchup-timeout` / `ESPRESSO_NODE_LOCAL_CATCHUP_TIMEOUT`

A node started with one of the flags refuses to start; the environment variables are ignored. The config endpoints no
longer report these settings, and the `decide_processor_*`, `proposal_fetcher_*`,
`internal_process_decided_events_duration` and `internal_append_{vid,da2,quorum2}_duration` metrics are gone.

### The genesis stake table comes from the genesis file

Urgency: Medium

A node takes the stake table the chain started with from its genesis file, from the `[[stake_table.genesis]]` entries.
The genesis files of mainnet and decaf carry it, so a node started with empty storage on these networks needs nothing
else. A node whose storage holds the network config it received from the orchestrator or a config peer keeps using that
config.

The orchestrator is gone, and nodes no longer fetch the config from peers. Remove these from the node's configuration:

- `--orchestrator-url` / `ESPRESSO_NODE_ORCHESTRATOR_URL`
- `--config-peers` / `ESPRESSO_NODE_CONFIG_PEERS`

A node started with one of the flags refuses to start; the environment variables are ignored. `/config/runtime` no
longer reports these settings, and the v2 runtime config leaves `orchestrator_url` and `config_peers` empty. `/config/hotshot`
keeps the fields the orchestrator set, which nodes of earlier releases need to join, but with fixed values, and the v2
HotShot config leaves `commit_sha`, `indexed_da` and `builder` empty. `submit-transactions` reads its node count from
`ESPRESSO_SUBMIT_TRANSACTIONS_NUM_NODES` instead of `ESPRESSO_ORCHESTRATOR_NUM_NODES`.

A network with its own genesis file must add its stake table to that file before it starts a node with empty storage,
with each node's staking key, state key, stake and DA membership, in the order of the network's genesis committee, as
`/config/hotshot` of a running node lists them. A follower derives the root of trust of its light client from the same
table unless `--light-client-genesis` pins one.

### Validators announce their API service

Urgency: High

A validator now needs `ESPRESSO_NODE_PUBLIC_API_URL` (or `--public-api-url`): the URL clients reach its API service at.
A validator without it refuses to start. The node signs a record with this URL and its staking key and sends it to every
consensus node at the start of each epoch, as the record for that epoch. Consensus keeps the records of the latest 20
epochs. The API service fetches them from consensus and serves them at `/v2/node/peers` (latest epoch, or `?epoch=`),
where a client checks each signature against the stake table of the record's epoch instead of trusting the node that
lists them.

If the API service authenticates requests, set `ESPRESSO_NODE_API_PUBLIC_KEY` (or `--api-public-key`) to its Ed25519
public key, base58. The record carries it, so clients learn which key to expect.

### Status routes come from stored blocks

Urgency: Low

The API service no longer reads consensus metrics for its status routes. `status/time-since-last-decide` is now the time
since the timestamp of the newest block the API service has stored, so it also grows while the ingester falls behind.
`status/success-rate` (`/v1` and `/v2`) is gone, since it divided by the current view, which only consensus has. The
`status` module now requires the `query` module.
