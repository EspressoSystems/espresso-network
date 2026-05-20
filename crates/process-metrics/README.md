# process-metrics

## What it does

- Registers 5 Prometheus gauges on any binary that has a `hotshot_types::traits::metrics::Metrics` registry.
- Samples them every 5s via a long-running async task.
- Ships a CI soak harness (`scripts/`) that captures the gauges + `docker stats` for the whole docker demo and
  summarizes them as Markdown.

## Gauges

| Name                            | Unit    | Source                                                  |
| ------------------------------- | ------- | ------------------------------------------------------- |
| `process_resident_memory_bytes` | bytes   | `sysinfo::Process::memory()`                            |
| `process_virtual_memory_bytes`  | bytes   | `sysinfo::Process::virtual_memory()`                    |
| `process_open_fds`              | -       | `/proc/self/fd` entry count (Linux only; 0 elsewhere)   |
| `process_threads`               | -       | `/proc/self/task` entry count (Linux only; 0 elsewhere) |
| `process_uptime_seconds`        | seconds | wall clock since startup                                |

## Library usage

```rust
let pm = process_metrics::ProcessMetrics::new(metrics);
tokio::spawn(pm.run());
```

Drop the returned `JoinHandle` (or attach it to a task list) to control lifetime; `run()` loops forever.

Currently wired into:

- `espresso-node` (`crates/espresso/node/src/context.rs`)

## CI soak harness

`scripts/soak.py` samples `docker stats` + each node's `/v0/status/metrics` every 1s, then writes a Markdown summary
(table, peak total, memory-over-time mermaid chart + matching flowchart legend) to `$GITHUB_STEP_SUMMARY` and a
full-resolution `rss-over-time.png` to the artifact dir. PEP 723 inline deps; run with `uv run`.

The harness is wrapped by the `soak` just module (`crates/process-metrics/justfile`), exposed at the repo root as
`just soak::...`. The CI workflow (`memory-soak-pr` / `memory-soak-non-pr` jobs in `.github/workflows/build.yml`) just
calls these recipes.

| Script         | What it does                                                         |
| -------------- | -------------------------------------------------------------------- |
| `soak.py`      | Sample, render summary + chart. Stdlib + matplotlib (via uv inline). |
| `test_soak.py` | Unit tests for `render_summary`. Run with `just soak::test`.         |

Matrix over the 2 genesis files (drb-header V0.4, epoch-reward V0.5). 300s per matrix entry. Artifact retention: 90 days
for samples, 1 day for compose logs.

### Env vars

The just recipes default these via `env_var_or_default`, so CI only needs to set what varies per matrix entry.

| Var                            | Default                        | Purpose                                        |
| ------------------------------ | ------------------------------ | ---------------------------------------------- |
| `DURATION_SECONDS`             | `300`                          | Sampling duration.                             |
| `SMOKE_TIMEOUT`                | `600`                          | `soak::up` smoke-test gate timeout.            |
| `DOCKER_TAG`                   | `main`                         | Docker compose image tag (read from `.env`).   |
| `ESPRESSO_NODE_GENESIS_FILE`   | `genesis/demo-drb-header.toml` | Genesis file passed to docker compose + label. |
| `DELEGATION_CONFIG`            | `multiple-delegators`          | Stake table delegation config.                 |
| `NUM_DELEGATORS_PER_VALIDATOR` | `100`                          | Delegators per validator.                      |
| `GENESIS_LABEL`                | basename of genesis file       | Heading on the summary.                        |
| `OUTPUT_DIR`                   | `./soak-samples`               | Where JSONL + summary.md + PNG land.           |
| `SOAK_LOGS_DIR`                | `./soak-logs`                  | Where compose logs are dumped by `soak::logs`. |
| `GITHUB_STEP_SUMMARY`          | (set by GH Actions)            | If set, summary is appended here too.          |

### Recipes

| Recipe         | What it does                                                                    |
| -------------- | ------------------------------------------------------------------------------- |
| `soak::up`     | `docker compose pull/up --pull never` + gate on `scripts/smoke-test-demo`.      |
| `soak::sample` | Sample docker stats + each node's `/v0/status/metrics` for `$DURATION_SECONDS`. |
| `soak::render` | Render `summary.md` + `rss-over-time.png` + Mermaid chart from `$OUTPUT_DIR`.   |
| `soak::logs`   | Dump `docker compose logs` and `ps` to `$SOAK_LOGS_DIR`.                        |
| `soak::down`   | `docker compose down -v`.                                                       |
| `soak::run`    | End-to-end: `up` then `sample` then `render`. Does not auto-down.               |
| `soak::test`   | `python3 -m unittest test_soak`.                                                |
| `soak::fmt`    | `ruff format` the scripts.                                                      |
| `soak::lint`   | `ruff check` the scripts.                                                       |

`soak::sample` runs `soak.py sample` (stdlib only, plain `python3`); `soak::render` runs `soak.py render` via `uv run`
so matplotlib loads from PEP 723 inline deps. Splitting them lets you re-render the chart locally against saved samples
without re-running the soak.

### Run locally (NixOS or otherwise)

The flake's `LD_LIBRARY_PATH` shellHook makes uv-installed wheels work on NixOS.

```bash
just soak::run                          # up + sample + render, leaves demo running
just soak::sample                       # re-sample with current demo
just soak::render                       # re-render saved samples
just soak::down                         # tear down compose

DURATION_SECONDS=30 just soak::run      # shorter local soak
ESPRESSO_NODE_GENESIS_FILE=genesis/demo-epoch-reward.toml just soak::run

just soak::test                         # unit tests
just soak::fmt && just soak::lint       # format + lint
```
