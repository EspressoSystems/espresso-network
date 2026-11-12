### Validator startup catchup from the LC contract

Urgency: Medium

Validators and followers no longer walk epochs at startup. A validator behind the stored epoch window is
seeded from the LC contract by its post-start catchup, over request-response, after about one view timeout.

- Espresso's own nodes upgrade first and serve the seed over HTTP as state peers. Other nodes with SQL storage serve
  the new `HeaderProof` request over request-response; peers on older releases log `Received invalid message` for it
  until they upgrade, and filesystem-storage nodes do not answer it. The peer requests of one seed attempt share
  one 30 s budget; if no peer answers within it the attempt fails and is retried on the next catchup trigger. State
  peers answer only after `--http-catchup-delay`.
- `--state-peers` is optional for validators. When set, it serves the seed over HTTP (`/availability`,
  `/light-client`) after `--http-catchup-delay`; a validator whose cliquenet peers do not answer `HeaderProof` yet can
  set it.
- `--bootstrap-epoch-catchup-timeout` is ignored for validators.
- Followers no longer walk epochs at startup either: they seed from the LC contract through their upstreams.
