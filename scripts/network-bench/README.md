# Network benchmark

Sends a rising load of 1 MB transactions to an espresso-node network. Reports capacity in MB/s, split into the consensus
limit and the query-node limit.

| Driver      | Network                                              | Entry point          |
| ----------- | ---------------------------------------------------- | -------------------- |
| `bench`     | 3 nodes on this machine, `process-compose.yaml` (CI) | `just bench run`     |
| `aws-bench` | N nodes plus a `ctl` host on EC2, one AZ             | `just bench aws run` |

<!-- regenerate: npx markdown-toc --maxdepth 2 --bullets - -i README.md -->

<!-- toc -->

- [Quick start](#quick-start)
- [Topology](#topology)
- [Requirements](#requirements)
- [Usage](#usage)
- [Options](#options)
- [Output](#output)
- [Measurement](#measurement)
- [AWS harness](#aws-harness)
- [Files](#files)
- [Related repos](#related-repos)

<!-- tocstop -->

## Quick start

`release-x`: a `release-*` branch with CI-built images. All commands run from the repo root.

**Capacity search (AWS)**

```
just bench aws run --tag release-x --latency decaf-2025 --search --max-usd 30
just bench aws run --tag release-x --latency decaf-2025 --search 150 --max-usd 30   # start rate in MB/s
```

**Fixed steps (AWS)**

```
just bench aws run --tag release-x --latency decaf-2025 --steps 50,60,80 --keep-going --max-usd 30
```

**Local (3 nodes, this machine)**

```
just bench build
just bench run
just bench run --steps 4,8,12,16 --keep-going
```

- `--latency decaf-2025` is the reference geography for comparable AWS runs. Without `--latency` all nodes share one AZ
  with no added delay, an upper bound far above a geo-distributed network. Profiles: [Latency model](#latency-model).
- `plan` takes the same flags as `run`: cost estimate and fleet dir, no AWS writes. `plan` checks against 60 USD unless
  `--max-usd` is given; pass the same `--max-usd` as the intended `run`.
- `run` prints the cost estimate and prompts; `--yes` skips the prompt (required without a tty).
- Results: `bench-state/aws/<fleet>/runs/<nn>-<name>/summary.md` (AWS), `bench-out/summary.md` (local).
- Exit codes: 0 valid, 1 invalid, 2 refused with nothing created, 3 failed (single-shot: destroyed; `run --fleet`: fleet
  left up), 4 resources may remain.
- Flags per command: `just bench aws <cmd> -h`, `just bench run -h`. Recipes: `just bench` or `just --list bench`.
- Several runs on one fleet: [Fleets](#fleets).
- Prerequisites: [Requirements](#requirements).

## Topology

```
laptop                       EC2, one AZ, private IPs
+--------------+  ssh/rsync  +-------------------------------------------------+
| aws-bench    |------------>| ctl   anvil - deploy - orchestrator - relay     |
|  plan / run  |             |       agent-drive: load, metrics, heights       |
|  tofu        |<--- out/ ---|       agent-host                                |
+--------------+             |           |  submit / poll                      |
       |                     |     +-----+------+-----------+                  |
       | tofu apply/destroy  |     v            v           v                  |
       v                     |   node0        node1  ...  nodeN-1              |
   AWS API                   |   validator    validator   validator            |
   tags: espresso-bench-run  |   + query      journal     journal              |
   TTL: shutdown -P +N min,  |   + postgres                                    |
   terminate on shutdown     |   agent-host on every node                      |
                             +-------------------------------------------------+
```

| Host      | Runs                                                                                         |
| --------- | -------------------------------------------------------------------------------------------- |
| `ctl`     | anvil, `deploy`, orchestrator, state-relay-server, `agent-drive`, `agent-host`               |
| `node0`   | espresso-node `-- storage-sql -- http -- query ...`, postgres container, `agent-host`        |
| `node1..` | espresso-node `-- storage-fs -- http -- status -- submit -- catchup -- config`, `agent-host` |

Modules shown are for `--consensus-storage fs` (default). With `journal`, both roles also get `storage-journal` first.

## Requirements

- nix devShell (opentofu, awscli2).
- AWS profile `timeboost-dev` (account 027574771971). Region eu-west-1 unless `--region` is given. Constants live in
  `aws-bench` and `aws/terraform/main.tf`.
- `ssh-keygen`.
- The rev is pushed as a `release-*` branch, so CI publishes the ghcr images.
- Local runs need `just bench build` first.

## Usage

### Capacity search

```
just bench aws plan --tag release-x --nodes 4 --db-modes colocated,volume --search --max-usd 30
just bench aws run --tag release-x --nodes 3 --search 120 --resolution-mb-s 2 --max-probes 16
```

- Output: a `Capacity` line with `+-resolution`, and `search: resolved after N probes`.
- `plan` and `run` refuse a search that the disks, the pg volume, the TTL or `--max-usd` cannot cover in the worst case.
  `--offered-gb` sets the byte budget and the disk size.
- `--search` refuses `--keep-going` and ignores `--steps`; more than one `--steps` value is refused.
- Algorithm: [Search](#search).

### Fixed steps

```
# default ramp, stops at the first failing step, then one refine step halfway back
just bench aws run --tag release-x
```

- `--keep-going` runs every step with no refine step, then stops the load and drains for up to
  `max(300 s, --tx-timeout-s)`. The summary reports this as the backlog drain time.
- Inclusion is read from the query node. A query lag above `--cap-s` throttles the load. A lag above `--tx-timeout-s`
  times transactions out. Both need to exceed the expected lag (e.g. 600).
- Consensus latency of a step with query lag above `--cap-s` is not comparable across runs.
- A long `--tx-timeout-s` raises the TTL bound; raise `--max-usd` when `plan` reports it over 10.
- Transactions of a lost payload stay pending until `--tx-timeout-s`, and the end of the run waits for them.
- The CI job does not fit this mode: its step timeout is 15 min.

### Local

```
just bench selftest --rate 250                          # driver ceiling against a null server, no network
just bench sysinfo                                      # runner info and CPU calibration only
just bench clean                                        # remove bench-out/ and storage of an interrupted run
```

- `selftest` runs the load driver against `nullserver.py`, which accepts submits and serves blocks of the received txs.
  It prints submitted MB/s, queue wait and the CPU of the driver, its scan processes and the server.
- Defaults differ from AWS: [Local bench flags](#local-bench-flags).

### Fleets

```
just bench aws up --tag release-x --nodes 4 --db-modes colocated,volume,rds --ttl-min 240 --max-usd 60
just bench aws run --fleet --query-db volume --search
just bench aws run --fleet --query-db colocated --steps 50,60,80 --keep-going
# named fleet, other image tag
just bench aws run --fleet lulu-20261001-074612 --query-db colocated --tag release-y
just bench aws run --fleet lulu-20261001-074612 --query-db rds --tag release-y --steps 50,60,80 --keep-going
# reset a dirty fleet or replace a stale lock, then measure
just bench aws run --fleet lulu-20261001-074612 --force
just bench aws down
just bench aws down lulu-20261001-074612 --yes          # --yes needs FLEET
```

- `run --fleet` locks the fleet, ships the agents, wipes journals, containers and the database, measures, collects into
  `runs/<nn>-<name>/` and returns the fleet to `idle`. Every run starts at height 0.
- `run --fleet` refuses with exit 2, nothing sent to the hosts, when: the phase is not `idle`, the lock is held, MODE is
  not in `--db-modes`, the TTL left is below the run's worst case plus lock-out, or a fleet flag is given.
- `--tag` differing from the fleet's makes the hosts pull those images by digest.
- `FLEET`: a name under `bench-state/aws/`, or a path when it contains `/`. Omitted: the only fleet in phase `idle`,
  `running`, `dirty`, `left-running`, `destroying` or `destroyed`, expired or not. Several or none are refused with the
  list. Applies to `run --fleet`, `down`, `status`, and to `collect` without `RUN_DIR` (that fleet's last run).

### Query database

`--query-engine` picks the query service's database, `--query-db` where it lives.

| `--query-engine` | `--query-db` | Database                            | Store                                                                           |
| ---------------- | ------------ | ----------------------------------- | ------------------------------------------------------------------------------- |
| `postgres`       | `colocated`  | container on node0                  | `/data/pg` on root gp3 (`--pg-iops`, `--pg-mbps`)                               |
| `postgres`       | `volume`     | container on node0                  | extra gp3 400 GiB (`--pg-iops`, `--pg-mbps`), ext4 by-id mount, dies with node0 |
| `postgres`       | `rds`        | RDS PostgreSQL db.m8g.4xlarge, 18.x | gp3 400 GiB or more, 12000 IOPS, 500 MB/s                                       |
| `sqlite`         | `colocated`  | embedded SQLite in the node         | `/data/journal/espresso/sqlite` on root gp3                                     |
| `sqlite`         | `volume`     | embedded SQLite in the node         | `/data/pg/sqlite` on the extra gp3 400 GiB volume (`--pg-iops`, `--pg-mbps`)    |
| `sqlite`         | `tmpfs`      | embedded SQLite in the node         | 8 GiB tmpfs (RAM) at `/store/espresso/sqlite`, lost when the container stops    |

Other combinations are refused. The default is `postgres` on `colocated`.

```
just bench aws run --tag release-x --nodes 4 --query-db volume --pg-mbps 1000 --yes
just bench aws run --fleet --query-db volume --node-env ESPRESSO_QUERY_PAYLOAD_DIR=/payload
```

- `--db-modes` (`plan`, `up`) lists the placements a fleet prepares. `--query-db` (`run`) picks one; `tmpfs` needs no
  preparation and runs on any fleet; it requires `--node-env ESPRESSO_QUERY_PAYLOAD_DIR=...` so payloads stay out of the
  tmpfs.
- A fleet with `rds` refuses `--pg-iops` and `--pg-mbps` values other than the defaults.
- `rds` needs IAM rights `iam:CreateRole`, `iam:PutRolePolicy`, `iam:PassRole`, `scheduler:CreateSchedule`. Without them
  apply fails, the fleet is destroyed, exit 3.
- The query node mounts `/data/pg/payload` as `/payload`. `ESPRESSO_QUERY_PAYLOAD_DIR=/payload` (experimental image
  feature, off by default) puts payload and VID share files there: on the `volume` store in `volume` mode, on node0's
  root disk otherwise. Size collected as `du-payload.txt`.

### Latency

```
just bench aws run --tag release-x --nodes 5 --latency decaf-2025
just bench aws run --tag release-x --nodes 5 --latency decaf-2025 --no-intra-latency   # cross-region delay only
just bench aws run --fleet --latency mainnet
just bench aws run --tag release-x --latency decaf-2025 --tcp-cc cubic   # cubic instead of bbr
```

- Shaped: node-to-node traffic, including catchup on 8080.
- `--tcp-cc` (default `bbr`, or `cubic`): congestion control of the nodes.
  - bbr stays in its startup mode while traffic is bursty and leaves it for good at the first overload; cross-region
    sockets then send 3-5x slower for the rest of the run.
  - cubic keeps small windows on the 158 ms links and backs off on rare losses: a 70 MB block run failed 150 MB/s.
  - `bbr_hold` (experiment): BBR held in its startup mode, built and loaded on each node by `aws/bbr-hold.sh` (kernel
    headers, mainline `tcp_bbr.c` of the kernel's version, ~1 min). Not a stock congestion control.
  - `bbr3`: BBR v3 of the XanMod kernel, installed on the node hosts with a reboot (~5 min, x86_64 only).
- `--mtu` (default 1500): interface MTU of the nodes. Traffic between AWS regions or over the internet carries at most
  1500 bytes; only one VPC gets jumbo frames (9001). Checked on every node after shaping.
  [AWS: network MTU](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/network_mtu.html).
- Never shaped: `ctl` traffic (orchestrator, L1, relay, submit, metrics), RDS, ssh.
- Model and profiles: [Latency model](#latency-model).

### Chaos

```
just bench aws run --tag release-x --chaos
just bench aws run --tag release-x --chaos --chaos-min 10 --chaos-rate 4 --chaos-seed 7 --chaos-kinds restart,kill
just bench aws up --tag release-x --nodes 22 --query-nodes 22 --node-type c8g.xlarge --ctl-type c8g.xlarge --ttl-min 180
just bench aws run --fleet --chaos --query-engine sqlite
```

- `--chaos` restarts, kills and wipes nodes, any node, while the load runs at a constant rate. Flags not given take the
  chaos shape: 22 nodes, every node a query node, `sqlite` colocated, `c8g.xlarge` nodes and `ctl`,
  `--latency decaf-2025`, `--chaos-min` steps of 60 s at `--chaos-rate` MB/s, `--keep-going`. `--nodes`,
  `--query-nodes`, `--node-type`, `--ctl-type`, `--latency` (`off`, `mainnet` or `decaf-2025`), `--submit-nodes` and
  `--query-engine` given on the command line are kept; `--latency` is per run, `up` does not take it; `--nodes N` alone
  gives N query nodes.
- Refused: `--steps`, `--step-s`, `--search`, `--keep-going` and `--warmup-s` (set by the chaos config); fewer than 7
  nodes; 2 or fewer query nodes; `--query-engine postgres`; `--query-db` other than `colocated`.
- Faults start after the warmup, one per 45 s, and stop 120 s before the load ends. Kinds are used in turn from
  `--chaos-kinds` (default `restart,kill,wipe`) on nodes in the order `--chaos-seed` shuffles them to.
  - `restart`: `docker restart`. `kill`: `docker kill`, `docker start` after 60 s. `wipe`: container logs saved to
    `/opt/bench/espresso-node.wiped.log.gz`, container removed, journal and payload dir emptied, container recreated
    from `node-rejoin.env` (`recreate.sh`), started.
- A fault is issued only while faulty nodes stay at most `(n-1)//3 - 1` (6 of 22), at least 2 query nodes are up and the
  target has 2 up peers; otherwise it waits for a recovery.
- Query nodes get `ESPRESSO_NODE_SYNC_STATUS_TTL=5s` in `node.env` and `node-rejoin.env`; the default 5 min cache would
  report a wiped node as synced from its empty database.
- Recovery gates, 2 consecutive 5 s ticks: a node rejoined when its height and voted view are near the tip of the up
  nodes; a query node then caught up when its height is near the up query nodes' tip and `sync-status` is fully synced
  (`missing` of blocks, leaves and vid_common all 0), counted only from 10 s after its `rejoined` event.
- `summary.md` and `driver.log` omit the capacity verdict under `--chaos`: steps with a dead leader do not measure it.
- The load driver submits to the next node when one is down; `submit_failovers` in `load-meta.json` counts the switches,
  `submit_errors` stays 0 unless every node refused a tx.
- A node not recovered within 300 s writes a `timeout` event and fails the run: exit 3, `summary.md` keeps the reason,
  the run is invalid. A node that crashes without a fault fails the run the same way.
- Output: `chaos.jsonl` (`fault`, `started`, `rejoined`, `caught_up`, `timeout`, `restored`; `ts` on ctl's clock;
  `caught_up` carries `missing` as [blocks, leaves, vid_common]; a query node's `timeout` carries `missing` when
  sync-status answered since the fault), a `### Chaos` section in `summary.md` with one row per fault and a totals line,
  `chaos:` lines in `driver.log` (a timeout line lists the last `missing` counts). Faulted nodes are exempt from the
  per-node scrape coverage and decided-blocks rules.
- Cost: about $5.4/h for the default fleet, about 40 min per single shot. `render` recomputes the Chaos section offline.
- Check `rss_peak_bytes` in `result.json` after the first run: `c8g.xlarge` has 8 GiB.

### Node build and config

```
just bench aws run --tag release-x --allocator mimalloc
just bench aws run --tag release-x --max-block-size 30mb
just bench aws run --tag release-x --node-env ESPRESSO_NODE_EMPTY_BLOCK_DELAY=50ms
just bench aws run --tag release-x --node-type c8i.4xlarge --ctl-type c8i.2xlarge   # Intel: amd64 AMI and images
just bench aws run --tag release-x --leader-trace
```

- Recommended settings and the branches that support them: [tuning.md](tuning.md).

- `--allocator NAME`: espresso-node from `espresso-node-alloc:<tag>-<allocator>`; other images stay `--tag`'s.
  - `build-allocators.yml` builds these images for release tags (`MAJOR.MINOR.PHASE.PATCH`), PRs that change the
    allocator build, and manual runs. Branches and `main` get none.
  - Branch images (all four allocators, ~30-60 min):
    `gh workflow run build-allocators.yml --repo EspressoSystems/espresso-network --ref <branch>`. The tag is then
    `<branch>-<allocator>`.
  - Each image is `espresso-node:main` with the branch's two binaries, so its revision label is main's.
- `--max-block-size`: genesis `max_block_size` of both chain configs; part of the config hash, shown in the summary.
  - Mainnet uses `10mb`. Local `genesis.toml` uses `100mb`.
  - On 5 x c8g.4xlarge the block interval grows superlinearly above about 60 MB blocks. Blocks at a 100 MB cap decide at
    about 93 MB/s, 45 to 60 MB blocks at about 160 MB/s.
- `--node-env KEY=VALUE`: repeatable; added last to every node's environment, overriding the harness's value. Taken
  verbatim and listed in the summary, so not for secrets. Part of the config hash.
- `--submit-nodes N`: nodes receiving txs, validators first, then `node0`. Default: all nodes, `node0` included;
  `nodes - 1` keeps txs off the query node.
  - With `node0` submitting, query lag, the query-node rule and `node0` CPU are not comparable across `--submit-nodes`
    values.
- `--node-type`, `--ctl-type`: both types share one architecture. Preflight picks the Ubuntu AMI and image platform to
  match. Type choice: [instance-types.md](instance-types.md).
- `--leader-trace`: nodes get `ESPRESSO_NODE_LEADER_TRACE_DIR=/trace` (host `/opt/bench/trace`).
  - Collected: `hosts/<name>/trace/leader_trace_node*.csv`.
  - `node_id` in the file name and rows is the orchestrator-assigned node index, not the host number; the host is the
    directory it was collected from. Per-view plots name both.
  - `parent Cert1 wait` (header created until the leaf commitment): `maybe_propose` waits for the parent's Cert1, not
    leader-local work.
  - `trace-plots RUN_DIR` (uv script, matplotlib; also run by `render`) writes `trace/leader_path.png`,
    `trace/leader_path_typical.png`, `trace/leader_path_worst.png`, `trace/finality.png`, `trace/stats.json`,
    `trace/leader_path.md` (segment medians per load step).
  - With `steps.json`, plots and stats cover only views whose t0 lies in a step's measured window `t_mid`..`t_end`.

### Baselines

```
just bench aws render bench-state/aws/<fleet>/runs/02-volume --baseline bench-state/aws/<fleet>/runs/01-volume/result.json
just bench render bench-out --baseline main-network-bench.json
just bench compare bench-out/result.json main-network-bench.json
```

- `render` re-analyzes a run dir and rewrites `result.json` and `summary.md`.
- A baseline is a `result.json` or `nextest-ci stats-fetch` output.
- Both drivers write the same `result.json` schema, so `compare` works across them.
- A `noisy` run never serves as a baseline.

### Status and cleanup

```
just bench aws status lulu-20261001-074612   # phase, time left, runs, lock holder, instances, pg volume, rds, cost
just bench aws status --all                  # tagged resources per fleet, latest expiry, orphan reason
just bench aws list                          # local fleet dirs, newest first; no AWS calls
just bench aws collect                       # re-collect into the last run of the only selectable fleet
just bench aws collect bench-state/aws/<fleet>/runs/02-volume
just bench aws destroy --orphans             # list orphans by tag, confirm, sweep
just bench aws prune --older-than 14         # delete old local fleet dirs that hold no AWS resources
```

- `--region` applies to `status`, `destroy` and `prune`. Fleet commands read the region from the fleet's manifest.
- `collect RUN_DIR` takes only the fleet's last run, with the fleet `idle` or `left-running`.
- After the laptop dies: `render RUN_DIR` on the rsynced files, then `down` or `destroy --orphans`. `collect` needs the
  fleet `idle` or `left-running`. Details: [Failure paths](#failure-paths).

### Publishing

```
just bench aws publish bench-state/aws/<fleet>/runs/01-run   # retry, or backfill older runs
```

- `run` (single-shot and `--fleet`) publishes automatically. `--no-publish` skips it.
- Destination: `runs/<fleet>/<run>/` of
  [espresso-network-bench-results](https://github.com/EspressoSystems/espresso-network-bench-results).
  `--results-remote` or env `BENCH_RESULTS_REMOTE` replaces the remote.
- Auth: plain `git` over https with the ambient credential helper, never prompting. Commit signing and `user.name` /
  `user.email` come from the user's git config.
- Published files: [Artifacts](#published-files).

## Options

### aws-bench commands

| Command   | Action                                                                                                     | AWS writes |
| --------- | ---------------------------------------------------------------------------------------------------------- | ---------- |
| `plan`    | render the fleet dir, print the cost estimate                                                              | n          |
| `up`      | plan, confirm, provision a fleet, leave it `idle`                                                          | y          |
| `run`     | plan, confirm, provision, measure, collect, publish, destroy; with `--fleet`: measure on a fleet from `up` | y          |
| `down`    | destroy a fleet (`tofu destroy` x3, tag sweep, fleet `cost.json`); exit 0 / 4                              | y          |
| `status`  | one fleet's progress, or `--all` tagged resources                                                          | n          |
| `list`    | local fleet dirs, newest first                                                                             | n          |
| `collect` | re-collect logs and samples from live hosts                                                                | n          |
| `render`  | re-analyze a run dir, rewrite its summary; `--baseline`                                                    | n          |
| `publish` | push run dirs to the results repo                                                                          | n          |
| `destroy` | `--orphans`: sweep orphaned fleets                                                                         | y          |
| `prune`   | `--older-than DAYS`: delete old local fleet dirs                                                           | n          |

`agent-drive` (`ctl`: load driver) and `agent-host` (every host: `/proc` and cgroup sampler) run on the hosts only.

### Run flags

Where:

- **all**: `plan`, `up`, `run`, `run --fleet`.
- **provision**: `plan`, `up`, `run`; refused by `run --fleet` (taken from the fleet).
- **per run**: `plan`, `run`, `run --fleet`; refused by `up`.
- **run**: `run` only.

| Flag                                  | Default                        | Where        | Meaning                                                                                       |
| ------------------------------------- | ------------------------------ | ------------ | --------------------------------------------------------------------------------------------- |
| `--tag`                               | required; fleet's on `--fleet` | all          | ghcr image tag pushed by CI                                                                   |
| `--allocator`                         | none                           | all          | `jemalloc`, `mimalloc`, `snmalloc`, `tcmalloc`                                                |
| `--nodes`                             | 5                              | provision    | validator count, `node0` included                                                             |
| `--query-nodes`                       | 1                              | provision    | query service nodes, the first `K` of `--nodes`; above 1 needs `--query-db colocated`         |
| `--node-type`                         | `c8g.4xlarge`                  | provision    | node instance type                                                                            |
| `--ctl-type`                          | `c8g.2xlarge`                  | provision    | `ctl` instance type                                                                           |
| `--root-gb`                           | `auto`                         | provision    | root volume GB; `auto` sizes from the ramp, or from `--offered-gb` with `--search`            |
| `--pg-iops`, `--pg-mbps`              | 12000, 500                     | provision    | node0 root volume and `volume` store                                                          |
| `--region`                            | eu-west-1                      | provision    | AWS region                                                                                    |
| `--max-usd`                           | `run` 10; `plan`, `up` 60      | provision    | refuse when the cost bound exceeds it                                                         |
| `--ttl-min`                           | `plan`, `run` auto; `up` 180   | provision    | minutes until hosts terminate; `auto`: worst case plus margin                                 |
| `--db-modes`                          | `colocated`                    | `plan`, `up` | stores the fleet prepares                                                                     |
| `--steps`                             | x1.5 from 4 to 200             | all          | MB/s per step, increasing                                                                     |
| `--step-s`                            | 30; `--search` 60              | all          | seconds per step                                                                              |
| `--cap-s`                             | 5; `--search` 60               | all          | in-flight cap, in seconds of the step's load                                                  |
| `--tx-timeout-s`                      | 30; `--search` 60              | all          | tx timeout                                                                                    |
| `--warmup-s`                          | 60                             | all          | warmup at the first step's rate                                                               |
| `--submit-workers`                    | 32                             | all          | submit threads; part of the config hash                                                       |
| `--namespaces`                        | 16                             | all          | namespaces the load spreads over, round robin from 10000; part of the config hash             |
| `--heartbeat-tx-s`                    | 50                             | all          | 8-byte txs per second for the whole run, 0 for none                                           |
| `--keep-going`                        | off                            | all          | run every step, then drain                                                                    |
| `--chaos`                             | off                            | per run      | fault nodes during the load; see [Chaos](#chaos)                                              |
| `--chaos-min`                         | 10                             | per run      | minutes of load, one 60 s step each                                                           |
| `--chaos-rate`                        | 4                              | per run      | MB/s of every step                                                                            |
| `--chaos-seed`                        | 42                             | per run      | seed of the order nodes are faulted in                                                        |
| `--chaos-kinds`                       | `restart,kill,wipe`            | per run      | comma separated, used in turn                                                                 |
| `--max-block-size`                    | `50mb`                         | per run      | genesis `max_block_size`                                                                      |
| `--node-env KEY=VALUE`                | none                           | per run      | repeatable; node environment                                                                  |
| `--leader-trace`, `--no-leader-trace` | off                            | per run      | leader trace CSVs and plots                                                                   |
| `--submit-nodes`                      | nodes                          | per run      | nodes receiving txs, 1..nodes                                                                 |
| `--latency`                           | `off`                          | per run      | `off`, `decaf-2025`, `mainnet`; `decaf-2025` with `--chaos`                                   |
| `--no-intra-latency`                  | off                            | per run      | with `--latency`: no same-location delay                                                      |
| `--tcp-cc`                            | bbr                            | per run      | with `--latency`: TCP congestion control, `bbr`, `cubic`, `bbr_hold` or `bbr3`                |
| `--mtu`                               | 1500                           | per run      | with `--latency`: interface MTU of the nodes                                                  |
| `--consensus-storage`                 | `fs`                           | per run      | `fs`, `journal` (experimental, not in `main` images); query node uses `storage-sql` with `fs` |
| `--fleet [FLEET]`                     | none                           | run          | measure on a fleet from `up`                                                                  |
| `--query-engine`                      | `postgres`                     | run          | `postgres`, `sqlite`                                                                          |
| `--query-db`                          | `colocated`                    | run          | `colocated`, `volume`, `rds`, `tmpfs` (sqlite: not `rds`)                                     |
| `--force`                             | off                            | run          | with `--fleet`: reset a dirty fleet, replace a stale lock                                     |
| `--yes`                               | off                            | run          | skip the prompt, required without a tty; also on `up`, `down`, `destroy`, `prune`             |
| `--no-publish`                        | off                            | run          | skip publishing                                                                               |
| `--results-remote`                    | results repo                   | run          | git remote, also on `publish`; env `BENCH_RESULTS_REMOTE`                                     |

### Search flags

Where: all. Each flag other than `--search` is refused without `--search`.

| Flag                | Default | Meaning                                                  |
| ------------------- | ------- | -------------------------------------------------------- |
| `--search [START]`  | 100     | first probe in MB/s; ~0.8x a known capacity saves probes |
| `--resolution-mb-s` | 5       | stop when `failed_at - capacity` is at most this         |
| `--max-probes`      | 12      | probe budget                                             |
| `--offered-gb`      | 150     | offered-bytes budget; also sizes the disks               |

### Local bench flags

| Flag                    | Default           | Meaning                             |
| ----------------------- | ----------------- | ----------------------------------- |
| `--steps`               | 4,6,8,10,12,14,16 | MB/s per step                       |
| `--step-s`              | 30                | seconds per step                    |
| `--submit-nodes`        | 3                 | nodes receiving txs                 |
| `--workers`             | 6                 | submit threads                      |
| `--cap-s`               | 5                 | in-flight cap, in seconds of load   |
| `--tx-timeout-s`        | 30                | tx timeout                          |
| `--warmup-s`            | 60                | warmup at the first step's rate     |
| `--seed`                | 42                | tx pool seed                        |
| `--latency-target-ms`   | 1000              | consensus latency rule              |
| `--query-lag-target-ms` | 1000              | query lag rule                      |
| `--keep-going`          | off               | run every step, then drain          |
| `--heartbeat-tx-s`      | 0                 | 8-byte txs per second               |
| `--tx-size`             | 1000000           | bytes per tx                        |
| `--baseline`            | none              | `result.json` or stats-fetch output |
| `--pr`                  | none              | PR number recorded in the result    |
| `--ready-timeout`       | 420               | seconds to wait for the network     |

- `selftest`: `--rate` 250 MB/s, `--step-s` 20, `--submit-workers` 32, `--out` keeps `load.jsonl` and `steps.json`.
- `--out`, `--storage-root`, `--bin-dir` are set by the recipe from the justfile env: `BENCH_OUT` (`bench-out`),
  `BENCH_STORAGE_ROOT` (`/tmp`), `BENCH_BIN_DIR` (`$CARGO_TARGET_DIR/release`, else `target/release`).
- `--keep-going` has a `--no-keep-going` form.

## Output

Progress: one fixed-width line every 10 s, totals at the end.

| Field   | Content                                                                                                 |
| ------- | ------------------------------------------------------------------------------------------------------- |
| phase   | warmup, step or probe, drain, done; rate, elapsed/length                                                |
| `sub`   | submitted MB/s, pending, timed out in the window                                                        |
| `cns`   | decided MB/s and share of submitted, validator height, block interval and size, p50 submit until header |
| `qry`   | MB/s scanned from the query node, its height, lag behind the validators                                 |
| `vto N` | view timeouts, shown only when nonzero                                                                  |

- AWS `run` prints each new phase with its detail, and every line of `agent.log` prefixed with the agent's phase, read
  from `ctl` every 5 s.
- Step details report `queued` (pacer output, MB/s), `queue wait` (queued until a thread sends) and `submit rtt` (send
  until response).
- A step submitting < 95% of its rate gets a cause line. Diagnostic only, no verdict uses it.
  - queued < 95%: `in-flight cap reached` if the pacer found the cap full, else `pacer late (controller CPU)`.
  - otherwise: `submit workers busy` (queue wait p50 > 100 ms), else `slow submit responses`.
- Capacity line: `Capacity **190 MB/s** (+-7.5, confirmed): ...`. The value is the highest passing rate below the lowest
  failing one (`< F` when none passed, `>= R` when none failed), overall and per side: consensus-only rules,
  query-node-only rules. `+-N` is `failed_at - capacity`. `confirmed` appears after a passing `--search` confirm probe.
- `steps.json` is rewritten after every probe, so a run cut short by the TTL keeps its probes.

## Measurement

### Load

- Rates from `--steps` (MB/s), each held `--step-s`, round-robin over the submit nodes.
- Each payload: a 16 B per-run marker, an 8 B id, and `tx_size - 24` bytes cut at a random offset from a seeded pool of
  256 txs' size, base64-encoded once. Requests are built from slices of the encoded pool.
- Inclusion: scans of the query node's blocks for the marker. Scans (fetch, JSON, base64, marker search) run in 4
  processes (`SCAN_PROCESSES`), stdlib only.
- Namespaces 10000 and 10015: the leader disperses a block's namespaces in parallel.
- Pacer: one tx every `tx_size / rate`. A late pacer sends every tx that came due, up to 1 s of load (`CATCHUP_S`);
  schedule lost beyond that stays lost. The in-flight cap applies. Txs are sent only before the step's end.

### Step rules

Each step is judged over its second half. A step stopped early is judged over its last 10 s.

| Rule              | Source                                                       | Fails when                          |
| ----------------- | ------------------------------------------------------------ | ----------------------------------- |
| decided MB/s      | validators' `consensus_finalized_bytes_sum`, Theil-Sen slope | < 80% of submitted                  |
| view timeouts     | `consensus_number_of_timeouts`                               | > 0                                 |
| consensus latency | submit until header on a validator                           | p50 > 1000 ms                       |
| query lag         | header on query node minus header on a validator             | p50 > 1000 ms, or growing > 50 ms/s |

- Early stop (not with `--keep-going`): from 10 s into a step, decided < 80% of submitted over the last 10 s stops the
  step, which fails on the decided rule.
- Without `--keep-going` the ramp stops at the first failing step, then runs one refine step halfway back.

### Drain

- Runs after a failing step before the refine step, after a failing search probe before the next probe, and after a
  `--keep-going` run.
- Queued transactions not yet sent are dropped. Nothing new is submitted.
- Done when: < 0.5 tx decided over 5 s, the query node shows 3 blocks past that point, and a counter sample after them
  is still idle. Then up to 10 s for pending transactions.
- Gives up (`drain timeout`) after 30 s without a new validator height or 300 s in total. The final drain of a
  `--keep-going` run caps at `max(300 s, --tx-timeout-s)`.
- Transactions still pending carry into the next probe until their timeout.

### Heartbeat

- A thread submits `--heartbeat-tx-s` 8-byte unmarked transactions per second, round-robin over the submit nodes, for
  the whole run.
- Purpose: a leader with an empty buffer sleeps `empty_block_delay` (500 ms) before building. After a drain the backlog
  of that sleep fills the next blocks and can collapse a probe below the warm capacity.
- Adds about 600 B/s to decided bytes. Scans skip heartbeat transactions.
- The config refuses a heartbeat whose bytes over a drain's idle window reach a quarter of `tx_size`.
- The first failed heartbeat submit is logged; the count goes to `load-meta.json` as `heartbeat_errors`.
- Part of the config hash when on.

### Search

- A probe is one `--step-s` step judged by the step rules.
- Climb x1.25 from START until a probe fails, bisect to `--resolution-mb-s`, confirm the result over `2 x step_s`.
- A collapsed probe (decided < 0.5 x submitted) is followed by a re-run at the highest pass below it, or at START when
  none passed. A failing re-run fails that rate: a collapsed re-run moves to the next lower pass, a soft fail is
  bisected. No pass left below stops the search (`degraded after overload at <rate>`).
- Query-bound with consensus unbounded: a second search on the consensus side, a lower bound tied to query lag.
- Stop reasons: `resolved`, `probe budget`, `disk budget`, `drain timeout`, `degraded after overload`, `below start`,
  `generator throttled`. Recorded in `load-meta.json`, `result.json` and the summary.

### Samples

- Per node: CPU, RSS, tokio busy, top ops from `/v1/status/metrics` (`consensus_`, `journal_`, `sql`, `storage`, ...).
- Per host (AWS): CPU, steal, memory, disk and net rates, per-container CPU and memory (cgroups).
- Per node host (AWS), `sockets.jsonl`: every 1 s the established cliquenet sockets from `ss -tinmO` (Send-Q, Recv-Q,
  cwnd, bbr, pacing and delivery rate, notsent, retrans, skmem verbatim) and TCP counters (retransmits, timeouts, memory
  pressure, drops); `tc -s qdisc` (netem queues) every 5 s. Not analyzed yet; read with a script.
- Query node: `pg_stat_database`, checkpointer, wal and activity every 5 s; `pg_stat_statements` and settings at
  collect; statements > 200 ms in the postgres log.

### Validity

| Verdict   | Condition                                                                                                           |
| --------- | ------------------------------------------------------------------------------------------------------------------- |
| `invalid` | not ready (height < 5), scrape coverage < 90%, a node decided nothing, host digest mismatch, clock offset > 1 s     |
| `noisy`   | steal > 5%, calibration drift > 10%, start spread >= 2 s, clock offset > 50 ms, busy controller, pg settings differ |

- A `noisy` run is kept and never serves as a baseline.

## AWS harness

### Fleet

- Price: preflight reads each type's on-demand Linux price in the region from the AWS Pricing API
  (`pricing get-products`, endpoint us-east-1). A type without a single matching price is refused.
- Arch: preflight reads it from `describe-instance-types` and picks the AMI and image platform (`linux/arm64` or
  `linux/amd64`). Recorded as `arch` in the manifest.
- vCPUs: an Intel vCPU is a hyperthread (c8i.4xlarge: 16 vCPU = 8 cores). A Graviton vCPU is a physical core
  (c8g.4xlarge: 16 cores).
- Stake: equal, orchestrator self-registration. 5 nodes give quorum 4, so a lagging `node0` never stalls consensus.
- Peers: every node has state peers. Query nodes set `ESPRESSO_NODE_API_PEERS` to the other query nodes; the single
  query node has none.
- Keys: test mnemonic, index 20 + i. No `keygen`, no `stake-for-demo`.
- Network: cliquenet over private IPs, libp2p over private DNS. The security group allows ssh from this machine's IP
  only (checkip).
- Images: `ghcr.io/espressosystems/espresso-network/<component>:<--tag>` plus foundry and postgres. Preflight resolves
  digests, hosts pull by digest, the manifest records them.
- ssh: every fleet gets an ed25519 key in `<fleet>/ssh/`. The private half is deleted after destroy and kept after a
  failed destroy.

### Run phases

```
preflight -> plan -> confirm $ -> apply -> provisioned -> [shaping] -> services -> nodes -> measuring
    |          |          |                                                                    |
  exit 2     exit 2     exit 2                                                                 v
                                  destroying <- report <- collecting <------------+
                                      |
                        exit 0/1, or 3 (failed, destroyed), or 4 (resources remain)
```

| Phase       | Does                                                                                                                                         | Gate                                                   |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| preflight   | tools, `sts` account, type arch, price and offered, image digests and arch                                                                   | any miss: exit 2                                       |
| plan        | render run dir, `tofu init/plan`, estimate at Pricing API instance prices and the `PRICES` constants (eu-west-1 on-demand)                   | over `--max-usd`, declined, no tty w/o `--yes`: exit 2 |
| apply       | `tofu apply`, local state in run dir; instances terminate on shutdown, cloud-init arms `shutdown -P +TTL` first                              | last tf stderr line, destroy, exit 3                   |
| provisioned | ssh and `cloud-init status --wait`, digests == manifest, render env/start.sh, rsync `/opt/bench`, start `agent-host`                         |                                                        |
| shaping     | `--latency` only: one tc netem leaf per peer on every node, then ping probes against the expected RTT                                        | probe off by > max(2 ms, 10 %): collect, exit 3        |
| services    | anvil (`eth_chainId`), deploy (code at genesis addresses), orchestrator and relay (`/healthcheck`), postgres (`pg_isready`)                  |                                                        |
| nodes       | `docker create` all, `docker start` at one wall-clock instant, record spread                                                                 | spread >= 2 s: noisy                                   |
| measuring   | `agent-drive` under `systemd-run`: wait heights, `netbench.drive_load`; laptop polls state, rsyncs every 60 s                                | agent error: collect, exit 3                           |
| collecting  | stop agent, `docker stop`, per host logs.gz, inspect, cloud-init log, chrony, du, host.jsonl, pg stats, final rsync; each step under timeout |                                                        |
| report      | `netbench.analyze` and AWS validity: `result.json`, `summary.md`                                                                             |                                                        |
| destroying  | `tofu destroy` x3, then tag sweep; `cost.json` from launch/terminate times; `INDEX.md` row                                                   | leftovers: exit 4                                      |

### Failure paths

| Event                          | Handling                                                                                                                               |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| `tofu apply` fails             | log the `Error:` block on one line (else the last stderr line), destroy, exit 3                                                        |
| node never ready / agent error | collect all, failure summary with last log lines, destroy (`run --fleet`: fleet left up), exit 3                                       |
| Ctrl-C                         | finish current phase, bounded collect, destroy, exit 3 or 4                                                                            |
| Ctrl-C again                   | SIGINT to a `tofu` command in flight (stops and writes state; a further one exits it); third skips the collection                      |
| destroy fails x3               | sweep by tag `espresso-bench-run=<name>`; leftovers: exit 4, `status --all` lists them                                                 |
| laptop dies                    | agents keep running; TTL ends instances and the pg volume; a schedule deletes the rds instance 5 min before the TTL                    |
| laptop dies, recovery          | `status/down FLEET`, `render RUN_DIR`; `collect` needs the fleet `idle` or `left-running`; `run --fleet --force` replaces a stale lock |
| reset fails on a fleet         | fleet phase `dirty`, lock kept; `run --fleet [FLEET] --force` resets again, or `down [FLEET]`                                          |
| state lost                     | `destroy --orphans`: list by tag (owner, launch, expiry), confirm, sweep; never another owner's live run                               |

### Fleet state

- Phases: `idle`, `running`, `dirty` (reset failed), then `done`. `planned` after `plan`, `left-running` after a failed
  destroy.
- `fleet.lock` holds pid, hostname and run name while a run holds the fleet. `status [FLEET]` shows whether the holder
  is alive.
- Validator root volumes: gp3 6000 IOPS, 500 MB/s; validators write about 1 byte per decided byte.
- Postgres settings come from `pg_tuning`. `pg-settings.json` is checked against them.
- Postgres memory settings (`shared_buffers`, `effective_cache_size`, `maintenance_work_mem`, `autovacuum_work_mem`)
  scale with node0's memory, read by preflight. rds keeps fixed 32 GiB values.
- `run --fleet` clears any previous qdisc during reset and restores the TCP sysctls and MTU that shaping changed, so a
  run without `--latency` measures an unshaped fleet.
- Cost bound: rate x (TTL + destroy + rds delete).

### Latency model

`--latency <profile>` gives every node a virtual location and shapes node-to-node egress so each pair sees its
real-world RTT. Without it, nodes see about 0.1 ms (one AZ).

| Profile      | Locations                                                                            | Cross RTT                            | Intra RTT |
| ------------ | ------------------------------------------------------------------------------------ | ------------------------------------ | --------- |
| `off`        | none                                                                                 |                                      |           |
| `decaf-2025` | 38:28:22:8:4 over eu-central-1, ap-southeast-1, us-east-1, ap-southeast-2, sa-east-1 | measured, `latency-matrix.csv`       | 10 ms     |
| `mainnet`    | mainnet validator cities by node count, `mainnet-locations.json`                     | `max(1 ms, 0.0157 ms/km x distance)` | 1 ms      |

- Shaping: HTB root with an unshaped default class; per peer one class, netem leaf and u32 filter keyed on the peer's
  private IP; half the directed RTT on each side.
- `decaf-2025`: the split of the 2025 benchmark network that emulated Decaf (robnet, espresso-deploy, gitbook benchmarks
  page). The derivation of the counts is not recorded. Decaf in 2026 is 69 % Europe.
- `mainnet`: 92 validators on 2026-10-05; Europe 74 nodes (70 % stake), North America 15, Asia 3. Regenerate with
  `uv run --script scripts/network-bench/mainnet-locations`, which sends validator IPs to ip-api.com. The file holds
  only city, country, lat/lon, node count and stake share.
- 0.0157 ms/km: least-squares fit through the origin over the 56 measured AWS pairs (RMSE 33 ms). Fibre at 2/3 c is
  0.0100 ms/km, so 1.57x path stretch. Measured pairs take precedence over the model. The model covers city pairs only.
- Intra RTT applies between nodes sharing a location. 10 ms is the low end of the 5 to 45 ms band measured between
  European mainnet nodes.
- Placement: nodes fill locations in profile order by largest remainder. 5 nodes on `decaf-2025`: eu-central-1 2,
  ap-southeast-1 2, us-east-1 1.
- Host settings applied with shaping: `--tcp-cc` (cubic with `fq_codel`, or bbr with `fq`), `tcp_rmem` and `tcp_wmem`
  max 256 MB, `tcp_notsent_lowat=131072`, `tcp_slow_start_after_idle=0`, `tcp_mtu_probing=1`, `--mtu` on the default
  interface. A 128 MB window covers a 100 MB proposal or 5 Gbps x 330 ms. Ubuntu's 4 MB cap holds one flow near 25 MB/s
  at 158 ms.
- Recorded: `deployment.latency` in `manifest.json` and `result.json` (profile, intra, nodes per location, assignment,
  matrix sha256, probes, `sysctls`, `mtu`), a `- latency:` bullet in `summary.md`, the matrix sha256 in `config_hash`.
- `ena-allowance.txt` per host shows whether AWS dropped packets at the instance bandwidth cap.

### Cleanup and tags

- Every resource carries `espresso-bench-run=<fleet>`, `-owner` and `-expires`.
- Hosts and the pg volume terminate at the TTL. An EventBridge one-shot schedule deletes the rds instance 5 min earlier.
- `destroy --orphans` covers fleets past expiry, in a terminal phase, or of this owner without `fleet.json`. It never
  touches a live fleet with state.
- Sweep order: delete schedule group, instances, rds instance (waits), its subnet and parameter groups, volumes,
  security group, key pair, scheduler IAM role (path `/espresso-bench/`).
- The IAM role is not in the tag API. `status --all` lists roles by path and shows a role whose fleet has no other
  resource as `role without resources`. Without `iam:ListRoles` roles are not listed (warning).
- A fleet whose `fleet.json` was renamed or deleted is `no local state` for its owner. `destroy --orphans` sweeps it.
- `prune` deletes a fleet dir, results included, only when: older than `DAYS` (>= 1), phase `planned`, `done` or
  `swept`, no `fleet.lock`, and AWS lists no resource or scheduler role tagged with its name. It needs AWS credentials.
  Other old dirs are kept and listed with the reason. The prompt states the dir and run count. `INDEX.md` keeps the
  rows.

### Artifacts

Fleet dir `bench-state/aws/<owner>-<yyyymmdd-hhmmss>/`, deleted only by `prune`:

```
fleet.json               argv, config, git rev, account, AZ, arch, AMI, digests, estimate, phase, hosts_info
events.jsonl driver.log  phase transitions, DEBUG log
terraform/               module copy, tfvars, plan.txt, terraform.tfstate
hosts.json               role, public/private IP, private DNS per host
hosts/<host>/            user-data.sh, ready.json
ssh/                     fleet ed25519 key
fleet.lock               pid, hostname, run name; present while a run holds the fleet
rds.json (0600)          rds fleets: endpoint, identifier, password
cost.json                expected, bound, actual USD of the fleet (instances, volume, rds)
runs/01-run/             one measurement
  manifest.json          fleet.json copy plus fleet, start_spread_s, config_hash, latency (profile, probes)
  genesis.toml topology.json config.json
  hosts/<host>/          node.env|ctl.env, start.sh, agent.json, <container>.log.gz, collect-<k>/ (`collect`),
                         cloud-init-output.log, chrony.txt, host.jsonl, sockets.jsonl (nodes), ena-allowance.txt,
                         node-rejoin.env, recreate.sh (--chaos), espresso-node.wiped.log.gz (wiped nodes)
                         trace/ (--leader-trace)
                         pg-stats.json pg-stats.jsonl pg-statements.json pg-settings.json du-payload.txt (node0)
  cloudwatch/            ec2-node0.json (EBS balance, every run); rds.json (rds runs)
  rds-logs/              postgres logs of the run window (rds runs)
  metrics.jsonl heights.jsonl consensus.jsonl load.jsonl steps.json
  load-meta.json         start_height, max_in_flight, cap_waits, submit_errors, submit_failovers, heartbeat_errors,
                         missing_payloads, drain_s, refine_skipped, stop_reason, marker
  chaos.jsonl            --chaos: one record per fault, start, rejoin, catch-up, timeout and restore
  stake-table.json final-<node>.prom
  run.json agent-state.json agent.log result.json summary.md throughput.png
  trace/                 trace-plots output (--leader-trace)
```

- `bench-state/aws/INDEX.md`: one row per run (fleet/run, rev, tag, N, db, latency, capacity, validity, exit, run cost).
- `throughput.png` (`throughput-plot RUN_DIR`): decided and query node MB/s, block size and interval, consensus latency
  and txs in flight. Shown at the top of `summary.md`.
- `bench-state/` is git-ignored and per worktree. tfstate and the ssh key of a fleet exist only in the worktree that ran
  `up`. `status --all` and `destroy --orphans` see every fleet through AWS tags.
- `destroy --orphans` in another worktree offers a live fleet as `no local state`.
- `git worktree remove` and `git clean -x` delete `bench-state/`. `down` comes first; with the state already lost,
  `destroy --orphans` sweeps the fleet.

#### Migrating tmp/aws-bench/

- `status --all`, then `down` each live fleet from its `tmp/aws-bench/<fleet>` path.
- Only terminal fleet dirs move (`mv tmp/aws-bench/<fleet> bench-state/aws/`); a live fleet dir has absolute user-data
  paths.

#### Published files

- Pushed: `summary.md`, `result.json`, `cost.json`, `throughput.png`, `trace/*.png`, `trace/leader_path.md`,
  `trace/stats.json`, `index-row.json`, a reduced `manifest.json`. Other files stay local. Symlinks are refused.
- A workflow in the results repo rebuilds its `INDEX.md` and `README.md` (leaderboard, recent runs, totals).
- The reduced `manifest.json` keeps `fleet`, `created_at`, `git_rev`, `query_db`, `images`, `config` without `node_env`,
  and each host's name, role and instance type. `summary.md` is published as written and lists `--node-env` values.
- A failed publish logs a warning with the retry command. The exit code stays the run's.
- `publish`: a run dir without `index-row.json` takes its row from `bench-state/aws/INDEX.md`; none there is an error.
  Unchanged content commits nothing. Every dir is tried; exit non-zero if any failed.

### 100 nodes

- `--nodes 100` grows: host map, orchestrator node count, genesis capacity, peer lists (3 per node).
- Open: vCPU quota (100 x 16 + 8), capacity in one AZ, `--max-usd` ~ 80, ghcr pull storm (mirror on `ctl`), parallel
  metrics scrape, `metrics.jsonl` size.

## Files

```
scripts/network-bench/
  netbench.py            shared core: config, staircase, search, height polling, inclusion tracking,
                         metrics scrape, analysis, validity, compare, summary
  bench                  local driver: preflight, process-compose, host sampling, selftest
  aws-bench              AWS driver (laptop) and host agents (agent-drive, agent-host)
  nullserver.py          null node for `selftest`
  latency.py             --latency profiles: node placement, RTT matrix, per-node tc script, probes
  latency-matrix.csv     56 measured AWS region pairs (espresso-deploy 34b35f6)
  mainnet-locations      regenerates mainnet-locations.json from the mainnet stake table
  mainnet-locations.json mainnet validator cities: nodes, stake share, lat/lon (no IPs)
  throughput-plot        throughput.png of a run dir (uv script)
  trace-plots            leader trace plots of a run dir (uv script)
  leadertrace.py         leader critical-path breakdown used by trace-plots
  instance-types.md      leader block-build time and cost per block per instance type
  genesis.toml           0.6, 100 MB blocks, 1 wei base fee
  process-compose.yaml   local 3-node network
  justfile               `just bench <recipe>`; `aws` forwards to aws-bench
  aws/user-data.sh       cloud-init template, every host
  aws/terraform/         key pair, security group, instances, pg volume, rds instance, delete schedule
  tests/                 just py::test
```

- Scripts `import netbench` as a sibling file. On hosts the same file sits in `/opt/bench`, run by system python3.

## Related repos

- [espresso-network-bench-results](https://github.com/EspressoSystems/espresso-network-bench-results): results of this
  benchmark
- [espresso-deploy](https://github.com/EspressoSystems/espresso-deploy): cross-region 100-node AWS benchmark
- [network-deploy](https://github.com/EspressoSystems/network-deploy): deployment of all Espresso networks, has a load
  generator
- [espresso-stack-benchmarks](https://github.com/EspressoSystems/espresso-stack-benchmarks): Espresso Stack chain
  benchmarks
- [vid-bench](https://github.com/EspressoSystems/vid-bench): VID benchmarks
- [allocator-benchmarks](https://github.com/EspressoSystems/allocator-benchmarks)
