### Upgrade query nodes before the nodes that read from them

Urgency: Medium

A node on this release cannot read finality proofs for blocks from before protocol version 0.6 from a query node on an
older release. This applies to every node that fetches from query peers (`--peers`, `ESPRESSO_NODE_API_PEERS`): query
nodes, followers and standalone light clients.

Upgrade query nodes first, and give each upgraded node at least one peer that is already on this release. Until then it
cannot fetch those blocks from peers, and a light client without a stored stake table cannot catch up from genesis.
Blocks from version 0.6 on are not affected.
