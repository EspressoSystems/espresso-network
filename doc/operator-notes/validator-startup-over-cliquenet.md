### Validator startup catchup over cliquenet

Urgency: Medium

Validators now fill their stake table window at startup from cliquenet peers, anchored on the LC contract, instead of
over HTTP state peers.

- Upgrade nodes with SQL storage first: they serve the new `HeaderProof` request. Older peers and filesystem-storage
  nodes do not answer it. A starting validator needs at least one answering peer, else it exits at the startup deadline
  with `InstallEpoch`.
- A validator whose BLS key is not registered on the StakeTable contract exits with `OwnKeyNotRegistered`. A registered
  validator whose key is not yet in the LC contract's stake table for its epoch waits during startup, without deadline,
  and logs the WARN `own key is not in the LC contract stake table` once, then a WARN every 30 minutes.
- If the startup exits with `DeadlineExceeded` and `no cliquenet connection established`, check the firewall and the
  `p2p_addr` registered on the StakeTable contract.
