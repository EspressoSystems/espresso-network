### Validator startup catchup from the LC contract

Urgency: Medium

Validators no longer walk epochs at startup. A validator behind the stored epoch window is seeded from the LC contract
by its post-start catchup, over request-response, after about one view timeout.

- Nodes with SQL storage serve the new `HeaderProof` request over request-response; peers on older releases log
  `Received invalid message` for it until they upgrade, and filesystem-storage nodes do not answer it. A seed
  request makes up to 5 attempts of 6 s each, with backoff; if no peer answers, the seed errors, a warning is logged
  and the catchup walks epochs from the held start as before. The backoff reads the
  `ESPRESSO_NODE_CATCHUP_BACKOFF_*` environment variables, not the `--catchup-*` flags; with
  `ESPRESSO_NODE_CATCHUP_BACKOFF_DISABLE` set a seed request makes one attempt.
- A validator does not start when it cannot read the validators active on L1. Previously the failure was logged and
  the node started without them as peers or responders.
- `--state-peers` is optional for validators.
- `/v1/node/all-validators/{epoch_number}/{offset}/{limit}` is empty for epochs a seed skipped.
- `--bootstrap-epoch-catchup-timeout` is ignored for validators.
- Followers are unchanged: they still walk epochs at startup.
