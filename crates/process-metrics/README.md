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
(table, peak total, memory-over-time mermaid chart) to `$GITHUB_STEP_SUMMARY` and a full-resolution `rss-over-time.png`
to the artifact dir. PEP 723 inline deps; run with `uv run`.

The CI workflow (`memory-soak-pr` / `memory-soak-non-pr` jobs in `.github/workflows/build.yml`) is responsible for
`docker compose up` and runs `scripts/smoke-test-demo` to gate readiness before invoking soak.py.

| Script         | What it does                                                         |
| -------------- | -------------------------------------------------------------------- |
| `soak.py`      | Sample, render summary + chart. Stdlib + matplotlib (via uv inline). |
| `test_soak.py` | Unit tests for `render_summary`. Run with `python3 -m unittest`.     |

Matrix over the 2 genesis files (drb-header V0.4, epoch-reward V0.5). 300s per matrix entry. Artifact retention: 90 days
for samples, 1 day for compose logs.

### Env vars

| Var                          | Default                        | Purpose                                   |
| ---------------------------- | ------------------------------ | ----------------------------------------- |
| `DURATION_SECONDS`           | `300`                          | Sampling duration.                        |
| `DOCKER_TAG`                 | `main`                         | Docker compose image tag (informational). |
| `ESPRESSO_NODE_GENESIS_FILE` | `genesis/demo-drb-header.toml` | Used to derive default label.             |
| `GENESIS_LABEL`              | basename of genesis file       | Heading on the summary.                   |
| `OUTPUT_DIR`                 | `./soak-samples`               | Where JSONL + summary.md + PNG land.      |
| `GITHUB_STEP_SUMMARY`        | (set by GH Actions)            | If set, summary is appended here too.     |

### Subcommands

- `soak.py sample` — collect data; stdlib only, runs with plain `python3`.
- `soak.py render` — read JSONL, write `summary.md` + `rss-over-time.png` + append to `$GITHUB_STEP_SUMMARY`; needs
  matplotlib (via uv inline deps).

Splitting lets the chart be re-rendered locally against saved samples without re-running the soak.

### Run locally (NixOS or otherwise)

The flake's `LD_LIBRARY_PATH` shellHook makes uv-installed wheels work on NixOS.

```bash
docker compose up -d
scripts/smoke-test-demo

DURATION_SECONDS=30 python3 crates/process-metrics/scripts/soak.py sample
uv run crates/process-metrics/scripts/soak.py render   # re-runnable on saved data

python3 -m unittest crates/process-metrics/scripts/test_soak.py
```
