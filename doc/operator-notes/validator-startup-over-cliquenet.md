### Validator startup catchup from the LC contract

Urgency: Medium

Validators no longer walk epochs at startup. A validator behind the stored epoch window is seeded from the LC contract
by its post-start catchup, over request-response, after about one view timeout.

- Nodes with SQL storage serve the new `HeaderProof` request over request-response; peers on older releases log
  `Received invalid message` for it until they upgrade, and filesystem-storage nodes do not answer it. A seed
  request makes up to 5 attempts with backoff; an attempt lasts 6 s without state peers, and with state peers until
  every provider answered or failed, up to `--http-catchup-delay` plus `--catchup-base-timeout` per state peer. If no
  peer answers, the seed errors, a warning is logged and the catchup walks epochs from the held start as before. The backoff reads the environment variables
  `ESPRESSO_NODE_CATCHUP_BACKOFF_*` and `ESPRESSO_NODE_CATCHUP_{BASE,MAX}_RETRY_DELAY`, not the flags
  `--catchup-backoff-*` and `--catchup-{base,max}-retry-delay`; with `ESPRESSO_NODE_CATCHUP_BACKOFF_DISABLE` set a
  seed request makes one attempt.
- A validator does not start when it cannot read the validators active on L1. Previously the failure was logged and
  the node started without them as peers or responders.
- `--state-peers` is optional for validators. When set, the peers serve each seed request over HTTP
  (`/v1/catchup/{height}/leafchain`, `/v1/light-client/header/{root}/{height}`) `--http-catchup-delay` after each attempt
  starts; a validator whose cliquenet peers do not answer `HeaderProof` yet can set it. Both routes exist in earlier
  releases: any state peer with SQL storage serves them, with no upgrade.
- `/v1/node/all-validators/{epoch_number}/{offset}/{limit}` is empty for epochs a seed skipped.
- `--bootstrap-epoch-catchup-timeout` is ignored for validators.
- Followers are unchanged: they still walk epochs at startup.
