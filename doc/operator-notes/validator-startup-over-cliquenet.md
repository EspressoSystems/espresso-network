### Validator startup catchup over cliquenet

Urgency: Medium

Validators now fill their stake table window at startup from cliquenet peers, anchored on the LC contract, instead of
over HTTP state peers.

- Upgrade nodes with SQL storage first: they serve the new `HeaderProof` request. Older peers and filesystem-storage
  nodes do not answer it. A starting validator needs at least one answering peer, else it exits at the 15 minute startup
  deadline with `InstallEpoch`.
- Validators must be reachable over cliquenet. The warning
  `no inbound cliquenet connections; check firewall and registered p2p_addr` means outbound connections work but nothing
  was accepted: check the firewall and the `p2p_addr` registered on the StakeTable contract.
- A validator whose BLS key is not registered on the StakeTable contract exits with `OwnKeyNotRegistered`. A registered
  validator that is not yet in the active set waits during startup and logs
  `own key registered but not in bootstrap peers; waiting` once per L1 finalized block.
- `ESPRESSO_NODE_BOOTSTRAP_EPOCH_CATCHUP_TIMEOUT` bounds one step of the startup (an L1 read, one request batch, one
  certificate wait). The startup as a whole exits after 15 minutes.
