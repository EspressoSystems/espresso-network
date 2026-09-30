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
  aws/justfile          just bench aws plan|run|status|collect|render|destroy
  aws/user-data.sh      cloud-init template, every host
  aws/terraform/        key pair, security group, instances
  test_*.py             just py::test
```

- `import netbench` as a sibling file; on hosts the same file sits in `/opt/bench`, system python3.
- Same `result.json` schema and `summary.md` from both drivers; `bench compare` works on either.

## Measurement

- Load: rates from `--steps` (MB/s), each held `--step-s`, round-robin over validators; per-run marker in every payload,
  inclusion found by scanning the query node's blocks.
- Default ramp: CI linear 4..16 MB/s; AWS x1.5 per step from 4 to 200 MB/s (the target), stops at the first failing
  step, then one refine step halfway back.
- Query node: pg_stat_database/checkpointer/wal/activity every 5 s, pg_stat_statements and settings at collect, slow
  statements (>200 ms) in the postgres log.
- Per step, second half judged:

| Rule              | Source                                                       | Fails when                                |
| ----------------- | ------------------------------------------------------------ | ----------------------------------------- |
| decided MB/s      | validators' `consensus_finalized_bytes_sum`, Theil-Sen slope | < 95% of offered                          |
| view timeouts     | `consensus_number_of_timeouts`                               | > 0                                       |
| consensus latency | submit until header on a validator                           | p50 > `--latency-target-ms`               |
| query lag         | header on query node minus header on a validator             | p50 > `--query-lag-target-ms`, or growing |

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

| Host      | Type (default)            | Runs                                                                                                     |
| --------- | ------------------------- | -------------------------------------------------------------------------------------------------------- |
| `ctl`     | `--ctl-type` c8g.2xlarge  | anvil, `deploy`, orchestrator, state-relay-server, `agent-drive`, `agent-host`                           |
| `node0`   | `--node-type` c8g.4xlarge | espresso-node `-- storage-journal -- storage-sql -- http -- query ...`, postgres container, `agent-host` |
| `node1..` | `--node-type`             | espresso-node `-- storage-journal -- http -- status -- submit -- catchup -- config`, `agent-host`        |

- Stake: equal, orchestrator self-registration; 5 nodes → quorum 4, lagging `node0` never stalls consensus.
- Peers: `node0` has state peers like every node and no API peers.
- Keys: test mnemonic, index 20 + i. No `keygen`, no `stake-for-demo`.
- Network: cliquenet over private IPs, libp2p over private DNS; SG allows ssh from `--operator-cidr` only.
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
| preflight   | tools, `sts` account, default VPC + DNS, type offered, vCPU quota, image digests + arm64, git clean, name unused                                   | any miss: exit 2                                       |
| plan        | render run dir, `tofu init/plan`, prices (Pricing API, 7-day cache, `--price` override), estimate                                                  | over `--max-usd`, declined, no tty w/o `--yes`: exit 2 |
| apply       | `tofu apply`, local state in run dir; instances terminate on shutdown, cloud-init arms `shutdown -P +TTL` first                                    | tf error classified, destroy, exit 3                   |
| provisioned | ssh + `cloud-init status --wait`, digests == manifest, render env/start.sh (need private IPs), rsync `/opt/bench`, start `agent-host`              |                                                        |
| services    | anvil (`eth_chainId`), deploy (code at genesis addresses), orchestrator + relay (`/healthcheck`), postgres (`pg_isready`)                          |                                                        |
| nodes       | `docker create` all, `docker start` at one wall-clock instant, record spread                                                                       | spread >= 2 s: noisy                                   |
| measuring   | `agent-drive` under `systemd-run`: wait heights, `netbench.drive_load`; laptop polls state, rsyncs every 60 s                                      | agent error: collect, exit 3                           |
| collecting  | stop agent, `docker stop`, per host logs.gz / inspect / cloud-init log / chrony / du / host.jsonl, pg stats, final rsync; every step under timeout |                                                        |
| report      | `netbench.analyze` + AWS validity → `result.json`, `summary.md`                                                                                    |                                                        |
| destroying  | `tofu destroy` x3, then tag sweep; `cost.json` from launch/terminate times; `INDEX.md` row                                                         | leftovers: exit 4                                      |

Exit: 0 valid, 1 invalid, 2 refused (nothing created), 3 failed then destroyed, 4 resources may remain.

### Failure paths

| Event                          | Handling                                                                                                                          |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| `tofu apply` fails             | classify (`VcpuLimitExceeded` + quota command, `InsufficientInstanceCapacity` + `--az`, `UnauthorizedOperation`), destroy, exit 3 |
| node never ready / agent error | collect all, failure summary with last log lines, destroy, exit 3                                                                 |
| Ctrl-C                         | finish current phase, bounded collect, destroy prompt (30 s, default yes; `--yes` skips), exit 3 or 4                             |
| destroy fails x3               | sweep by tag `espresso-bench-run=<name>`; leftovers → exit 4, `status --all` lists them                                           |
| laptop dies                    | agents keep running, TTL terminates instances; `status/collect/render/destroy DIR` recover                                        |
| state lost                     | `destroy --orphans`: list by tag (owner, launch, expiry), confirm, sweep; never another owner's live run                          |

### Artifacts

`tmp/aws-bench/<owner>-<yyyymmdd-hhmm>/`, never deleted:

```
manifest.json            argv, config, git rev, tools, account, AZ, AMI, digests, prices+sources, estimate, phase
events.jsonl driver.log  phase transitions, DEBUG log
terraform/               module copy, tfvars, plan.txt, terraform.tfstate
hosts.json               role, public/private IP, private DNS per host
genesis.toml topology.json agent.json config.json
hosts/<host>/            user-data.sh, node.env|ctl.env, start.sh, ready.json, <container>.log.gz,
                         cloud-init-output.log, chrony.txt, host.jsonl, pg-stats.json (node0)
                         pg-stats.jsonl pg-statements.json pg-settings.json (node0)
metrics.jsonl heights.jsonl consensus.jsonl load.jsonl load-meta.json steps.json
stake-table.json final-<node>.prom
run.json agent-state.json agent.log result.json summary.md cost.json
```

`tmp/aws-bench/INDEX.md`: one row per run (name, rev, tag, N, capacity, validity, exit, cost bound/actual, destroyed).

### Commands

```
just bench aws plan    --tag release-x [--nodes 5] [--offline --price c8g.4xlarge=0.71 --price c8g.2xlarge=0.36]
just bench aws run     --tag release-x [--nodes 5] [--steps 4,6,9,...] [--max-usd 10] [--yes]
just bench aws status  --all | DIR
just bench aws collect DIR
just bench aws render  DIR [--baseline FILE]
just bench aws destroy DIR | --orphans
```

- Needs: nix devShell (opentofu, awscli2), AWS profile with EC2 write (`--profile`, `--account`), `ssh-keygen`, rev
  pushed as `release-*` so CI publishes images.
- `--ssh-key auto` (default): ed25519 key generated per run in `<run>/ssh/`, private half deleted after destroy (kept
  with `--keep` or a failed destroy).
- `--ssh-key PATH`: existing key, PATH and PATH.pub must exist, never deleted.

### 100 nodes

- `--nodes 100` grows: host map, orchestrator node count, genesis capacity, peer lists (3 per node).
- Open: vCPU quota (100 x 16 + 8), capacity in one AZ, `--max-usd` ~ 80, ghcr pull storm (mirror on `ctl`), parallel
  metrics scrape, `metrics.jsonl` size.
