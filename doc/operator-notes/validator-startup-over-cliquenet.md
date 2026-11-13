### Validator startup catchup from the LC contract

Urgency: Medium

Validators and followers no longer walk epochs at startup. A validator behind the stored epoch window is seeded from
the LC contract by its post-start catchup, over request-response, after about one view timeout.

- Nodes with SQL storage serve the new `HeaderProof` request over request-response; peers on older releases log
  `Received invalid message` for it until they upgrade, and filesystem-storage nodes do not answer it. A seed
  request makes up to 5 attempts of 6 s each, with backoff; if no peer answers, the seed errors, a warning is logged
  and the catchup walks epochs from the held start as before. The backoff reads the
  `ESPRESSO_NODE_CATCHUP_BACKOFF_*` environment variables, not the backoff flags
  (`--catchup-backoff-*`, `--catchup-{base,max}-retry-delay`); with `ESPRESSO_NODE_CATCHUP_BACKOFF_DISABLE` set a
  seed request makes one attempt.
- A validator does not start when it cannot read the validators active on L1. Previously the failure was logged and
  the node started without them as peers or responders.
- Espresso's own nodes upgrade first and serve the seed over HTTP as state peers.
- `--state-peers` is optional for validators. When set, the peers serve each seed request over HTTP
  (`/v1/catchup/{height}/leafchain`, `/v1/light-client/header/{root}/{height}`) `--http-catchup-delay` after the request
  starts; a validator whose cliquenet peers do not answer `HeaderProof` yet can set it.
- `/v1/node/all-validators/{epoch_number}/{offset}/{limit}` is empty for epochs a seed skipped.
- `--bootstrap-epoch-catchup-timeout` is ignored for validators.
  For followers it bounds the wait for the stake table it needs, at startup and at each poll. Exceeding it or failing
  logs a warning and the next poll retries; a follower no longer exits at startup when the upstreams or L1 are down.
- Followers no longer walk epochs at startup either. They have no request-response: their state peers
  (`--state-peers`, or the upstreams when unset) serve the seed over HTTP at once, and `--http-catchup-delay` does not
  apply.
