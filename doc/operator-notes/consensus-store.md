### New required setting: consensus store directory

Urgency: High

Every node now needs `ESPRESSO_NODE_CONSENSUS_STORE_PATH` (or `--consensus-store-path`): a directory on local, persistent
disk. The node keeps its votes, locks, undecided proposals and block payloads there, and waits for writes to it before it
votes, so a slow or network-attached disk slows down voting. A node without the setting does not start.

On the first start with the setting, the node copies this state from its existing storage (`storage-fs` or
`storage-sql`) into the directory, and refuses to start if that fails. From then on, consensus keeps this state only in
the directory. Keep the directory across restarts and upgrades: a node that loses it can vote twice in a view. Do not
point two nodes at the same directory.
