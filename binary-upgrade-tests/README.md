# Binary Upgrade Test

Boots docker-compose on the pinned mainnet release `20260505`, then rolls each `espresso-node-N` and the rest of the
services to a target tag (e.g. `main` or `pr-1234`), running `scripts/smoke-test-demo` before and after.

The `docker-compose.yaml` and `.env` for the base phase are extracted from the `BASE_TAG` git revision into `./tmp/`.
That file uses the legacy `ESPRESSO_SEQUENCER_*` env names natively read by the old binary; the new binary maps them to
current names via `crates/espresso/utils/src/env_compat.rs`, so the same compose file works for both images during the
rolling upgrade.

## Local

The repo's `.env` must exist (the dev shell or `cp .env.docker.example .env` populates it). The script bails with a
clear error otherwise.

    just binary-upgrade-test                                # uses defaults
    BASE_TAG=20260505 UPGRADE_TAG=main ./binary-upgrade-tests/run.sh
    UPGRADE_PULL=1 UPGRADE_TAG=20260601 ./binary-upgrade-tests/run.sh
    KEEP_RUNNING=1 ./binary-upgrade-tests/run.sh           # leave compose up

The script runs `docker compose down -v` on exit (unless `KEEP_RUNNING=1`), which destroys local demo state.

## Inputs

| env            | default  |
| -------------- | -------- |
| BASE_TAG       | 20260505 |
| UPGRADE_TAG    | main     |
| KEEP_RUNNING   | 0        |
| UPGRADE_PULL   | 0        |
| SETTLE_SECONDS | 30       |

## CI

- PRs: `binary-upgrade-test-pr` job in `.github/workflows/build.yml` loads the PR-built tar artifacts and runs this
  script with `UPGRADE_TAG=pr-<num>`.
- Manual: Actions tab -> "Binary Upgrade Test" -> Run with custom tags.

## Scope

This is a **binary upgrade** test: it swaps docker images on a running network at the same protocol version. It is not a
**protocol upgrade** test (which exercises HotShot's `UpgradeProposal` / `UpgradeCertificate` flow to transition the
network from one protocol version to the next). Protocol upgrades are covered by `tests/upgrades.rs`.

The genesis is `data/genesis/demo-drb-header.toml` (V0.4, no upgrade configured), so headers stay at V0.4 throughout.

## Asserts

- `scripts/smoke-test-demo` passes before any roll and after the full upgrade: block height, transaction count, and
  light client updates all increase; builder balance decreases; recipient balance increases; balance is conserved;
  builder healthcheck reachable.
- After each rolled `espresso-node-N`, a stable monitor node's `/node/block-height` advances by at least 2 within 120s.
- All five `espresso-node-N` containers run the upgrade image after the roll; the remaining long-running services pinned
  to `${DOCKER_TAG}` (orchestrator, builder, prover, CDN, state-relay, submit-transactions, nasty-client,
  node-validator) run the upgrade image after the bulk recreate.

## Not yet asserted (TODO)

- Reward claim flow works against `RewardClaim` on L1 pre and post upgrade.
- Light-client `authRoot` advances past the upgrade boundary.
- Network crosses at least one full epoch boundary after the upgrade (epoch root state update, stake table sync,
  epoch-rooted reward tree commit).
