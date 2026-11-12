### Validator startup catchup from the LC contract

Urgency: Medium

Validators and followers no longer walk epochs at startup. A validator behind the stored epoch window is seeded from
the LC contract by its post-start catchup, over request-response, after about one view timeout.

- Espresso's own nodes upgrade first and serve the seed over HTTP as state peers. Other nodes with SQL storage serve
  the new `HeaderProof` request over request-response; peers on older releases log `Received invalid message` for it
  until they upgrade, and filesystem-storage nodes do not answer it. The peer requests of one seed attempt share one
  30 s budget; if no peer answers within it the attempt fails and is retried on the next catchup trigger. State peers
  are eligible `--http-catchup-delay` after a seed fetch starts, or at once when request-response is not available.
- `--state-peers` is optional for validators. When set, it serves the seed over HTTP (`/availability`,
  `/light-client`) after `--http-catchup-delay`; a validator whose cliquenet peers do not answer `HeaderProof` yet can
  set it.
- `--bootstrap-epoch-catchup-timeout` is ignored for validators.
  For followers it bounds the wait per poll for the stake table it needs; exceeding it fails the poll, which is retried.
- Followers no longer walk epochs at startup either. They have no request-response: their upstreams (the query
  module's `--peers`) serve the seed over HTTP at once, and `--http-catchup-delay` does not apply.
- A stake table hash mismatch stops the node; restart it after fixing the L1 RPC provider.
