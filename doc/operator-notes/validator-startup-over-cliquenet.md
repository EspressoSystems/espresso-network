### Validator startup catchup from the LC contract

Urgency: Medium

Validators no longer walk epochs at startup. A validator behind the stored epoch window is seeded from the LC contract
by its post-start catchup, over request-response, after about one view timeout.

- Nodes with SQL storage serve the new `HeaderProof` request over request-response; peers on older releases log
  `Received invalid message` for it until they upgrade, and filesystem-storage nodes do not answer it. The peer
  requests of one seed attempt share one 30 s budget; if no peer answers within it the attempt fails and is retried on
  the next catchup trigger.
- `--state-peers` is optional for validators.
- `--bootstrap-epoch-catchup-timeout` is ignored for validators.
- Followers are unchanged: they still walk epochs at startup.
