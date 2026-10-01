# Network benchmark

Load staircase of 1 MB transactions against an espresso-node network. Output: capacity in MB/s, split into the consensus
limit and the query-node limit.

| Driver      | Network                                              | Command              |
| ----------- | ---------------------------------------------------- | -------------------- |
| `bench`     | 3 nodes on this machine, `process-compose.yaml` (CI) | `just bench run`     |
| `aws-bench` | N nodes + controller on EC2, one AZ                  | `just bench aws run` |

## Layout

```
scripts/network-bench/
  netbench.py           shared core: config, staircase, height polling, inclusion tracking,
                        metrics scrape, analysis, validity, compare, summary
  bench                 local driver: preflight, process-compose, host sampling
  aws-bench             AWS driver (laptop) + host agents (agent-drive, agent-host)
  genesis.toml          0.6, 100 MB blocks, 1 wei base fee
  process-compose.yaml  local 3-node network
  justfile              recipe `aws` forwards to aws-bench: just bench aws <verb>
  aws/user-data.sh      cloud-init template, every host
  aws/terraform/        key pair, security group, instances, pg volume, rds instance, delete schedule
  test_*.py             just py::test
```

- `import netbench` as a sibling file; on hosts the same file sits in `/opt/bench`, system python3.
- Same `result.json` schema and `summary.md` from both drivers; `bench compare` works on either.

## Measurement

- Load: rates from `--steps` (MB/s), each held `--step-s`, round-robin over validators; per-run marker in every payload,
  inclusion found by scanning the query node's blocks.
- Default ramp: CI linear 4..16 MB/s; AWS x1.5 per step from 4 to 200 MB/s (the target), stops at the first failing
  step, then one refine step halfway back.
- `--keep-going`: every step runs whatever its verdict, no refine step; then the load stops until the query node caught
  up (at most 600 s), reported as the backlog drain time. Set `--cap-s` and `--tx-timeout-s` above the expected lag
  (e.g. 600): inclusion is read from the query node, so a lag above `--cap-s` throttles the load and one above
  `--tx-timeout-s` times transactions out. Consensus latency of a lagging step is not reliable. Transactions of a lost
  payload stay pending until `--tx-timeout-s`, which the end of the run waits for. Not for the CI job: its step timeout
  is 15 min.
- `--node-env KEY=VALUE` (repeatable; not an `up` flag, pass it to `run --fleet`): added to every node's environment,
  overriding the harness's own value; taken verbatim, not for secrets; listed in the summary's deployment block and part
  of the config hash.
- Query node: pg_stat_database/checkpointer/wal/activity every 5 s, pg_stat_statements and settings at collect, slow
  statements (>200 ms) in the postgres log.
- Per step, second half judged:

| Rule              | Source                                                       | Fails when                |
| ----------------- | ------------------------------------------------------------ | ------------------------- |
| decided MB/s      | validators' `consensus_finalized_bytes_sum`, Theil-Sen slope | < 80% of submitted        |
| view timeouts     | `consensus_number_of_timeouts`                               | > 0                       |
| consensus latency | submit until header on a validator                           | p50 > 1000 ms             |
| query lag         | header on query node minus header on a validator             | p50 > 1000 ms, or growing |

- Capacity: highest passing rate; overall, consensus-only rules, query-node-only rules.
- Per node: CPU, RSS, tokio busy, top ops from `/v1/status/metrics` (`consensus_`, `journal_`, `sql`, `storage`, ...).
- Per host (AWS): CPU, steal, memory, disk and net rates, per-container CPU and memory (cgroups).
- `invalid`: not ready, low scrape coverage, a node decided nothing, host digest mismatch, clock off > 1 s.
- `noisy`: steal, calibration drift, start spread >= 2 s, clock off > 50 ms, busy controller. Kept, never a baseline.

## AWS harness

### Fleet

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

| Host      | Type          | Runs                                                                                                     |
| --------- | ------------- | -------------------------------------------------------------------------------------------------------- |
| `ctl`     | `c8g.2xlarge` | anvil, `deploy`, orchestrator, state-relay-server, `agent-drive`, `agent-host`                           |
| `node0`   | `c8g.4xlarge` | espresso-node `-- storage-journal -- storage-sql -- http -- query ...`, postgres container, `agent-host` |
| `node1..` | `c8g.4xlarge` | espresso-node `-- storage-journal -- http -- status -- submit -- catchup -- config`, `agent-host`        |

- Stake: equal, orchestrator self-registration; 5 nodes → quorum 4, lagging `node0` never stalls consensus.
- Peers: `node0` has state peers like every node and no API peers.
- Keys: test mnemonic, index 20 + i. No `keygen`, no `stake-for-demo`.
- Network: cliquenet over private IPs, libp2p over private DNS; SG allows ssh from this machine's IP only (checkip).
- Images: `ghcr.io/espressosystems/espresso-network/<component>:<--tag>` (CI, `release-*` branch) + foundry + postgres;
  preflight resolves digests, hosts pull by digest, manifest records them.

### Run phases

```
preflight -> plan -> confirm $ -> apply -> provisioned -> services -> nodes -> measuring
    |          |          |                                                       |
  exit 2     exit 2     exit 2                                                    v
                                  destroying <- report <- collecting <------------+
                                      |
                        exit 0/1, or 3 (failed, destroyed), or 4 (resources remain)
```

| Phase       | Does                                                                                                                                               | Gate                                                   |
| ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| preflight   | tools, `sts` account, type offered, image digests + arm64                                                                                          | any miss: exit 2                                       |
| plan        | render run dir, `tofu init/plan`, estimate at the `PRICES` constants (eu-west-1 on-demand)                                                         | over `--max-usd`, declined, no tty w/o `--yes`: exit 2 |
| apply       | `tofu apply`, local state in run dir; instances terminate on shutdown, cloud-init arms `shutdown -P +TTL` first                                    | last tf stderr line, destroy, exit 3                   |
| provisioned | ssh + `cloud-init status --wait`, digests == manifest, render env/start.sh (need private IPs), rsync `/opt/bench`, start `agent-host`              |                                                        |
| services    | anvil (`eth_chainId`), deploy (code at genesis addresses), orchestrator + relay (`/healthcheck`), postgres (`pg_isready`)                          |                                                        |
| nodes       | `docker create` all, `docker start` at one wall-clock instant, record spread                                                                       | spread >= 2 s: noisy                                   |
| measuring   | `agent-drive` under `systemd-run`: wait heights, `netbench.drive_load`; laptop polls state, rsyncs every 60 s                                      | agent error: collect, exit 3                           |
| collecting  | stop agent, `docker stop`, per host logs.gz / inspect / cloud-init log / chrony / du / host.jsonl, pg stats, final rsync; every step under timeout |                                                        |
| report      | `netbench.analyze` + AWS validity → `result.json`, `summary.md`                                                                                    |                                                        |
| destroying  | `tofu destroy` x3, then tag sweep; `cost.json` from launch/terminate times; `INDEX.md` row                                                         | leftovers: exit 4                                      |

Exit: 0 valid, 1 invalid, 2 refused (nothing created), 3 failed then destroyed, 4 resources may remain.

### Failure paths

| Event                          | Handling                                                                                                               |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------- |
| `tofu apply` fails             | log the last stderr line (e.g. `VcpuLimitExceeded`), destroy, exit 3                                                   |
| node never ready / agent error | collect all, failure summary with last log lines, destroy, exit 3                                                      |
| Ctrl-C                         | finish current phase, bounded collect, destroy, exit 3 or 4                                                            |
| destroy fails x3               | sweep by tag `espresso-bench-run=<name>`; leftovers → exit 4, `status --all` lists them                                |
| laptop dies                    | agents keep running, TTL ends instances (and the pg volume), a schedule deletes the rds instance 5 min before the TTL; |
|                                | `status/down FLEET`, `collect/render RUN` recover; `run --fleet --force` replaces a stale lock                         |
| reset fails on a fleet         | fleet phase `dirty`, lock kept; `run --fleet DIR --force` resets again, or `down DIR`                                  |
| state lost                     | `destroy --orphans`: list by tag (owner, launch, expiry), confirm, sweep; never another owner's live run               |

### Fleets and query databases

`up` provisions a fleet that stays idle; `run --fleet` measures on it as often as the TTL allows. `run` without
`--fleet` is `up`, one measurement, `down`.

| Command                              | Does                                                                                                     |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| `up --db-modes colocated,volume,rds` | preflight, cost bound at `--ttl-min` (default 180), confirm, apply, prepare stores, phase `idle`, exit 0 |
| `run --fleet DIR --query-db MODE`    | lock, ship agents, reset chain and database, measure, collect into `runs/<nn>-<name>/`, back to `idle`   |
| `down DIR`                           | `tofu destroy` x3, tag sweep, fleet `cost.json`; exit 0 or 4                                             |

- `run --fleet` refuses: phase not `idle`, lock held, MODE not in `--db-modes`, TTL left below the run's worst case plus
  lock-out, fleet flags given. Exit 2, nothing sent to the hosts. `--tag` differing from the fleet's pulls by digest.
- Each run wipes journals, containers and the database; every run starts at height 0.
- `fleet.lock` holds pid and hostname; `status DIR` shows whether the holder is alive. Phases: `idle`, `running`,
  `dirty` (reset failed), then `done`; `planned` after `plan`, `left-running` after a failed destroy.

| `--query-db` | Postgres                            | Store of `/data/pg`                                  |
| ------------ | ----------------------------------- | ---------------------------------------------------- |
| `colocated`  | container on node0                  | root gp3 (`--pg-iops`, `--pg-mbps`)                  |
| `volume`     | container on node0                  | extra gp3 400 GiB, ext4 by-id mount, dies with node0 |
| `rds`        | RDS PostgreSQL db.m8g.4xlarge, 18.x | gp3 400 GiB or more, 12000 IOPS, 500 MB/s            |

- Same `PG_TUNING` settings in every mode; `pg-settings.json` is checked against them (`noisy` on a difference).
- rds needs IAM rights `iam:CreateRole`, `iam:PutRolePolicy`, `iam:PassRole`, `scheduler:CreateSchedule`; without them
  apply fails, the fleet is destroyed, exit 3.
- Cost: the fleet bound is rate x (TTL + destroy + rds delete).

### Cleanup

Every resource carries `espresso-bench-run=<fleet>`, `-owner` and `-expires`. Hosts and the pg volume terminate at the
TTL. An EventBridge one-shot schedule deletes the rds instance 5 min earlier.

| Command             | Covers                                                                                                        |
| ------------------- | ------------------------------------------------------------------------------------------------------------- |
| `status DIR`        | phase, time left, runs, lock holder (alive or dead), instances, pg volume, rds state, cost                    |
| `status --all`      | tagged resources per fleet (count per kind), latest expiry, orphan reason                                     |
| `destroy --orphans` | fleets past expiry, in a terminal phase, or of this owner without `fleet.json`; never a live fleet with state |
| `list`              | local fleet dirs, newest first: phase, created, expires, time left, cost, runs, last run; no AWS calls        |

- Sweep order: delete schedule group, instances, rds instance (waits), its subnet and parameter groups, volumes,
  security group, key pair, scheduler IAM role (path `/espresso-bench/`).
- The IAM role is not in the tag API: `status --all` lists roles by path and shows one whose fleet has no other resource
  as `role without resources`. Without `iam:ListRoles` roles are not listed (warning).
- A fleet whose `fleet.json` was renamed or deleted is `no local state` for its owner; `destroy --orphans` sweeps it.

### Artifacts

`bench-state/aws/<owner>-<yyyymmdd-hhmmss>/` (the fleet dir), never deleted:

```
fleet.json               argv, config, git rev, account, AZ, AMI, digests, estimate, phase, hosts_info
events.jsonl driver.log  phase transitions, DEBUG log
terraform/               module copy, tfvars, plan.txt, terraform.tfstate
hosts.json               role, public/private IP, private DNS per host
hosts/<host>/            user-data.sh, ready.json
fleet.lock               pid, hostname, run name; present while a run holds the fleet
rds.json (0600)          rds fleets: endpoint, identifier, password
cost.json                expected, bound, actual USD of the fleet (instances, volume, rds)
runs/01-run/             one measurement
  manifest.json          fleet.json copy plus fleet, start_spread_s, config_hash
  genesis.toml topology.json config.json
  hosts/<host>/          node.env|ctl.env, start.sh, agent.json, <container>.log.gz, collect-<k>/ (`collect`),
                         cloud-init-output.log, chrony.txt, host.jsonl, pg-stats.json (node0)
                         pg-stats.jsonl pg-statements.json pg-settings.json (node0)
  cloudwatch/            ec2-node0.json (EBS balance, every run); rds.json (rds runs)
  rds-logs/              postgres logs of the run window (rds runs)
  metrics.jsonl heights.jsonl consensus.jsonl load.jsonl load-meta.json steps.json
  stake-table.json final-<node>.prom
  run.json agent-state.json agent.log result.json summary.md
```

`bench-state/aws/INDEX.md`: one row per run (fleet/run, rev, tag, N, db, capacity, validity, exit, run cost).

- `bench-state/` is git-ignored and per worktree: tfstate and the ssh key of a fleet exist only in the worktree that ran
  `up`. `status --all` and `destroy --orphans` see every fleet through AWS tags.
- `destroy --orphans` in another worktree offers a live fleet as `no local state`.
- `git worktree remove` and `git clean -x` delete `bench-state/`. Run `down` first; if the state is already lost,
  `destroy --orphans` sweeps the fleet.
- Migration from `tmp/aws-bench/`: run `status --all`, then `down` each live fleet from its old `tmp/aws-bench/<fleet>`
  path. Move only terminal fleet dirs (`mv tmp/aws-bench/<fleet> bench-state/aws/`); a live fleet dir cannot move
  because its user-data paths are absolute.

### Commands

```
just bench aws plan    --tag release-x [--nodes 5]
just bench aws run     --tag release-x [--nodes 5] [--steps 4,6,9,...] [--query-db MODE] [--max-usd 10] [--yes]
just bench aws up      --tag release-x --db-modes colocated,volume,rds [--ttl-min 180] [--max-usd 60] [--yes]
just bench aws run     --fleet [FLEET] --query-db MODE [--tag release-y] [--node-env KEY=VALUE] [--force] [--yes]
just bench aws down    [FLEET] [--yes]              # --yes needs FLEET
just bench aws status  --all | [FLEET]
just bench aws collect [RUN_DIR]               # the fleet's last run, fleet idle or left-running
just bench aws render  RUN_DIR [--baseline FILE]
just bench aws destroy --orphans
just bench aws list
```

- `FLEET`: a name under `bench-state/aws/`, or a path when it contains `/`. Omitted: the only fleet in phase `idle`,
  `running`, `dirty`, `left-running`, `destroying` or `destroyed`, expired or not; several or none is refused with the
  list. `collect` without `RUN_DIR` takes that fleet's last run.

- Needs: nix devShell (opentofu, awscli2), AWS profile `timeboost-dev` (account 027574771971, eu-west-1; constants in
  `aws-bench` and `aws/terraform/main.tf`), `ssh-keygen`, rev pushed as `release-*` so CI publishes images.
- Every fleet gets an ed25519 key in `<fleet>/ssh/`; the private half is deleted after destroy (kept after a failed
  destroy).

### 100 nodes

- `--nodes 100` grows: host map, orchestrator node count, genesis capacity, peer lists (3 per node).
- Open: vCPU quota (100 x 16 + 8), capacity in one AZ, `--max-usd` ~ 80, ghcr pull storm (mirror on `ctl`), parallel
  metrics scrape, `metrics.jsonl` size.
