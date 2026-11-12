### Validator startup catchup from the LC contract

Urgency: Medium

Validators no longer walk epochs at startup. A validator behind the stored epoch window is seeded from the LC contract
by its post-start catchup, over request-response, after about one view timeout.

- Espresso's own nodes upgrade first and serve the seed over HTTP as state peers. Other nodes with SQL storage serve
  the new `HeaderProof` request over request-response; peers on older releases log `Received invalid message` for it
  until they upgrade, and filesystem-storage nodes do not answer it. A far-behind validator needs one answering
  cliquenet peer or state peer; else each attempt ends at the request timeout (30 s) and is retried on the next
  catchup trigger.
- `--state-peers` is optional for validators. When set, it serves the seed over HTTP (`/availability`,
  `/light-client`) after `--http-catchup-delay`; a validator whose cliquenet peers do not answer `HeaderProof` yet can
  set it.
- `--bootstrap-epoch-catchup-timeout` is ignored for validators.
- Followers are unchanged: they still walk epochs at startup.
