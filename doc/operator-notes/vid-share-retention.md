### VID shares are kept for 7 days

Urgency: Medium

Nodes with SQL consensus storage now keep their own VID shares for `ESPRESSO_NODE_CONSENSUS_STORAGE_VID_SHARE_RETENTION`
views (default 1209600, about 7 days at 0.5s views) instead of deleting them on decide. These shares are not counted
toward `ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_USAGE`, so consensus storage grows past that target until the window
fills. File system storage is unchanged.

A share's size depends mostly on the node's stake, not on block size: each view costs about 400 bytes, plus 600 to 700
bytes per namespace in the block for every 0.1% of total stake the node holds. Estimated space for a full window, from
the last week of blocks and the current stake tables:

| Network | Node                         | VID shares for 7 days |
| ------- | ---------------------------- | --------------------- |
| Mainnet | median stake (0.5% of total) | about 2 GB            |
| Mainnet | largest stake (8% of total)  | about 24 GB           |
| Decaf   | median stake (3% of total)   | about 10 GB           |
| Decaf   | largest stake (7% of total)  | about 23 GB           |

Busier blocks with more namespaces raise these figures in proportion. Check the free space on the consensus storage
volume after upgrading, and lower `ESPRESSO_NODE_CONSENSUS_STORAGE_VID_SHARE_RETENTION` if it is tight. Keep it at or
above `ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_RETENTION` (302000 by default), or shares can be deleted before their
decide event is processed.
