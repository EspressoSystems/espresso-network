# Binary Upgrade Test

Swaps docker images on a running demo network at a single protocol version, from a pinned base release to a target tag,
asserting the network keeps producing and serving blocks.

## What it does

- Extracts `docker-compose.yaml` + `.env` from the `BASE_TAG` git revision into a temp dir.
- Brings the stack up on `BASE_TAG`. Runs the demo smoke test.
- Recreates each `espresso-node-N` one at a time on `UPGRADE_TAG`. After each roll, every node must catch up to a height
  sampled just before that roll.
- Bulk-recreates the remaining long-running espresso services on `UPGRADE_TAG`.
- Asserts every running service whose image is published under `ghcr.io/espressosystems/espresso-network/` is on
  `UPGRADE_TAG`.
- Runs the demo smoke test again.

## Run

The repo's `.env` must exist (`cp .env.docker.example .env` or use the dev shell). Ensure docker is running.

    just binary-upgrade-tests::run
    just binary-upgrade-tests::run --scenario catchup-from-old-fs
    BASE_TAG=20260505 UPGRADE_TAG=main just binary-upgrade-tests::run
    KEEP_RUNNING=1 just binary-upgrade-tests::run            # leave compose stack up

`docker compose down -v` runs on exit unless `KEEP_RUNNING=1`, destroying local demo state.

## Scenarios

Pick with `--scenario` (default `vanilla`). CI runs each as a separate matrix job.

| Scenario              | What it adds on top of vanilla                                                                                                                                                                                     |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `vanilla`             | Roll each node 0..4 to UPGRADE one at a time, then bulk-upgrade the rest.                                                                                                                                          |
| `catchup-from-old-fs` | Roll node 4 (FS-backed) first, wipe its storage, restart on UPGRADE with `ESPRESSO_NODE_CONFIG_PEERS`, wait for it to catch up while peers are still on BASE. Then finish vanilla.                                 |
| `catchup-from-old-pg` | Same shape but wipes node 1 (postgres-backed) plus its `espresso-node-db-1` data.                                                                                                                                  |
| `catchup-from-new-fs` | Finish vanilla, then wipe node 4 + restart with `ESPRESSO_NODE_CONFIG_PEERS`, wait for catchup from all-UPGRADE peers.                                                                                             |
| `catchup-from-new-pg` | Finish vanilla, then wipe node 1 + db-1, restart, wait for catchup.                                                                                                                                                |
| `first-start`         | Finish vanilla, then start a fresh `espresso-node-5` on BASE_TAG with `ESPRESSO_NODE_CONFIG_PEERS` and no orchestrator URL. Verifies the older binary can deserialize `/v0/config/hotshot` from the newer network. |

## Inputs

| env          | default                                                                                            |
| ------------ | -------------------------------------------------------------------------------------------------- |
| BASE_TAG     | latest YYYYMMDD tag in the repo, or the previous one when HEAD is itself on a YYYYMMDD release tag |
| UPGRADE_TAG  | main                                                                                               |
| KEEP_RUNNING | 0                                                                                                  |
| UPGRADE_PULL | 0                                                                                                  |

## CI

- PRs: `binary-upgrade-test-pr` in `.github/workflows/build.yml` loads PR-built tar artifacts and runs with
  `UPGRADE_TAG=pr-<num>`.
- Manual: Actions -> "Binary Upgrade Test" -> Run with custom tags.

## Scope

- A **binary upgrade** test: same protocol version on both sides, only images swap. Protocol upgrade (HotShot
  `UpgradeProposal` / `UpgradeCertificate`) is covered by `tests/upgrades.rs`.
- Genesis is `data/genesis/demo-drb-header.toml` (V0.4, no upgrade configured), so headers stay at V0.4 throughout.

## What's checked

- Demo smoke test passes before any roll and after the full upgrade: block height, transaction count, light client
  updates, and fee recipient balance all advance; builder balance decreases; total balance is conserved; builder
  healthcheck is reachable.
- After each node roll, all five nodes catch up past a pre-roll reference height. Query-enabled nodes are also required
  to make the new block fully retrievable via the availability API (catches "header indexed but payload/VID missing"
  regressions).
- After the bulk upgrade, every running espresso-network service is on `UPGRADE_TAG`.

## Not yet asserted (TODO)

- Reward claim flow works against `RewardClaim` on L1 pre and post upgrade.
- Light-client `authRoot` advances past the upgrade boundary.
- Network crosses at least one full epoch boundary after the upgrade (epoch root state update, stake table sync,
  epoch-rooted reward tree commit).
