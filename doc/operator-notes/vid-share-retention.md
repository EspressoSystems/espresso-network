### VID shares are kept for 7 days

Urgency: Medium

Nodes with SQL consensus storage now keep their own VID shares for `ESPRESSO_NODE_CONSENSUS_STORAGE_VID_SHARE_RETENTION`
views (default 1209600, about 7 days at 0.5s views) instead of deleting them on decide. These shares are not counted
toward `ESPRESSO_NODE_CONSENSUS_STORAGE_TARGET_USAGE`, so consensus storage grows past that target until the window
fills. Check the free space on the consensus storage volume after upgrading, and lower
`ESPRESSO_NODE_CONSENSUS_STORAGE_VID_SHARE_RETENTION` if it is tight. File system storage is unchanged.
