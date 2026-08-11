# API snapshot tests

`test_api_snapshots` starts the native demo and compares the response body of every HTTP and WebSocket endpoint it
serves against a committed snapshot in `tests/snapshots/api/`. It exists so that a refactor of the API layer, in
particular the remaining tide-disco to axum migration, cannot change what an endpoint returns without the diff showing
up in a pull request.

What the suite pins:

- The body of each endpoint, normalized (see below). One snapshot file per endpoint.
- That each endpoint answers at all. A probe whose response is not a success status fails the test instead of recording
  the error, so a route that disappears is a failure rather than a new snapshot.
- That `/v0/x` and `/x` return the same body as `/v1/x`. Aliases are not snapshotted separately; they are fetched and
  compared against the canonical `/v1` body.
- The OpenAPI documents. `/v1/docs/openapi.json` on each node covers every route that node's serve mode exposes, so
  adding or removing a route changes a snapshot even if no probe names it.

## Running

The demo binaries have to be built and on `PATH`, exactly as for the other native-demo tests:

```sh
just build test
```

Verify against the committed snapshots. This is what CI runs:

```sh
cargo nextest run -p tests -E 'test(test_api_snapshots)' \
    --profile integration --no-capture --retries 0
```

Record or re-record:

```sh
INSTA_UPDATE=always cargo nextest run -p tests -E 'test(test_api_snapshots)' \
    --profile integration --no-capture --retries 0
```

Two environment variables help while iterating, since a demo startup dominates the runtime:

- `API_SNAPSHOT_FILTER=<substring>` runs only probes whose snapshot name contains the substring.
- `API_SNAPSHOT_EXTERNAL_DEMO=1` skips starting a demo and probes one that is already running (start it with
  `just demo-native`).

```sh
API_SNAPSHOT_EXTERNAL_DEMO=1 API_SNAPSHOT_FILTER=orchestrator \
    INSTA_UPDATE=always cargo nextest run -p tests -E 'test(test_api_snapshots)' \
    --profile integration --no-capture --retries 0
```

`INSTA_UPDATE` is refused when `CI=true`, so a CI run can only ever check snapshots.

## When a snapshot changes

The test reports every endpoint that changed in one run rather than stopping at the first, so one failure message lists
the whole blast radius of a change.

1. Read the list in the failure message, and the diffs in the run output or the JUnit artifact.
2. Re-record locally and read `git diff tests/snapshots/api/`. Each changed file is one endpoint whose response changed.
3. If the change is intended, commit the updated snapshots in the same pull request and say in the commit message what
   changed about the API. The snapshot diff is the reviewer's view of the API change, so it should be legible on its
   own.
4. A deleted snapshot file means an endpoint stopped answering. That needs an explicit reason: a dropped route is how
   this suite earns its keep.

Do not re-record to make CI green without reading the diff.

## Normalization

Bodies are normalized before comparison, because much of what the demo serves depends on how far the chain has run:

- `Exact` keeps the body as it is, with object keys sorted. Used for responses that are determined by the genesis file,
  such as anything at height 0. A diff means the encoding or the genesis changed.
- `Shape` keeps keys, nesting and types but replaces scalars with markers such as `<number>`, and collapses arrays to
  their distinct element shapes. Used for counters, live state and anything else whose values move. It still catches a
  renamed or dropped field.
- `MetricNames` reduces Prometheus output to its sorted metric and label names, dropping values.
- `Text` keeps a non-JSON body verbatim.

On top of a mode, a probe can name individual fields to rewrite:

- `mask_fields` replaces a field's value with `<masked>`. `"key"` matches at any depth, while `"parent.key"` and
  `"parent.*"` only match under that parent.
- `collapse_fields` turns an object whose key set depends on the run, such as a map of signatures by signer, into a
  deduplicated list of its values.

Masking is kept as narrow as the nondeterminism requires, because every masked field is a field the suite no longer
checks. The genesis header is the main case: it records the L1 block the demo's anvil happened to start from, so
`l1_finalized`'s hash and timestamp differ on every run, and so does every commitment computed over that header (`hash`,
`block_hash`, `leaf_commit`, `vote_commitment`). Those are masked; the merkle roots, payload commitment, chain config,
`ns_table` and the header's own `timestamp` are all still pinned value for value. The light client additionally returns
the certificate that decided a leaf, whose signature set depends on which votes arrived first, so its `signatures` are
masked too.

Paths never hard-code a hash or height. Placeholders like `{leaf_hash}`, `{tx_height}` and `{ns}` are resolved once per
run against the live chain, so probe declarations stay readable and stable. `{epoch}` is resolved by asking which epochs
the node will serve, because proof of stake registrations only take effect from epoch 3.

Because genesis-determined snapshots are recorded from a specific genesis file, changing `data/genesis/*.toml` or the
demo's `.env` legitimately changes them. That is intended, but it does mean a genesis change carries a snapshot
re-record.

If you add a probe, record it and then run the verify command against a _second_, freshly started demo. That is the only
thing that distinguishes a snapshot that is genuinely stable from one that merely agreed with itself once, and it is how
the masking above was found.

## Not covered

- The state prover, which runs one-shot in the demo and serves no HTTP listener.
- The dev node and the light-client query service, which the native demo does not start.
- Mutating routes. The orchestrator registers nodes and the builder claims blocks over `POST`, so calling them would
  perturb the network being measured. Only their read routes are probed.
- Binary (`application/octet-stream`) response encodings, and status codes and headers on error paths. Both are worth
  adding later; this suite deliberately compares successful bodies only.
- Four endpoints that cannot answer in this demo, each with the reason recorded next to where it would have been probed:
  the node validator's `details` socket (pushes nothing until its network view changes), the orchestrator's
  `/api/builders` (waits for more builders than the demo starts), the relay's `/v1/api/lateststate` (legacy light
  client, unsigned on version 0.4), and `catchup/chain-config/{commitment}` (the commitment would have to be recomputed
  in the test).
