# Inspecting live chains

Query-service base URLs:

- Mainnet: `https://query.main.net.espresso.network`
- Decaf testnet: `https://query.decaf.testnet.espresso.network`

Routes are declared in `crates/espresso/api/src/axum/routes.rs` and served under `/v1/` or `/v2/`. Unversioned and
`/v0/` paths are rewritten to `/v1/` before routing (`rewrite_legacy_uri`, `crates/espresso/api/src/axum.rs:245`).

- `/v1/status/block-height`
- `/v1/status/metrics` - Prometheus text. `consensus_genesis{base_version,upgrade_version,genesis_version}` is the
  protocol version the node runs; `consensus_version{rev,desc,timestamp}` is the build. Registered in
  `crates/espresso/node/src/lib.rs:255`.
- `/v1/config/runtime` - node runtime config, including the parsed `genesis`
- `/v1/config/hotshot` - HotShot config, including `libp2p_config.bootstrap_nodes`
- `/v1/availability/header/{height}` - block header (`version`, `l1_finalized`, `timestamp_millis`)
- `/v1/availability/leaf/{height}`
- `/v1/node/transactions/count`
- `/v1/catchup/{height}/{view}/...` - state proofs
- `/v1/docs/openapi.json` - full route list; `/v1` serves Swagger UI

`/version` is unversioned and returns the crate version, not the protocol version. There is no `/status/version` route.

`scripts/fetch-network-facts <dir>` snapshots the above for both networks.
