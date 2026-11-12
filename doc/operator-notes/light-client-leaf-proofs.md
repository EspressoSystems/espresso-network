### Upgrade query nodes before light clients and followers

Urgency: Medium

Upgrade the query nodes a light client or follower reads from (the follower's `--peers`) to this release before the
light client or follower itself.

A light client on this release only accepts a finality proof for a block from before protocol version 0.6 when the query
node serving it runs this release too. Against an older query node, requests for those blocks fail and a light client
without a stored stake table cannot catch up from genesis. Blocks from version 0.6 on are not affected.
