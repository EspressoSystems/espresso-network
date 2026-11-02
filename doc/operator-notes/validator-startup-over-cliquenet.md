### Validator startup catchup from the LC contract

Urgency: Medium

Validators no longer walk epochs at startup. A validator behind the stored epoch window is seeded from the LC contract
by its post-start catchup, over request-response, after about one view timeout.

- Upgrade nodes with SQL storage first: they serve the new `HeaderProof` request. Older peers and filesystem-storage
  nodes do not answer it. A far-behind validator needs at least one answering peer, else each seed attempt fails and is
  retried on the next catchup trigger.
- `--state-peers` is optional for validators.
