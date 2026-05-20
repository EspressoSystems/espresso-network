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

`scripts/soak.py` is a single end-to-end orchestrator: bring up the docker demo, sample `docker stats` + each node's
`/v0/status/metrics` every 1s, write a Markdown summary to `$GITHUB_STEP_SUMMARY` and a JSONL artifact.

| Script         | What it does                                                                                                             |
| -------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `soak.py`      | Compose up, wait for nodes, sample, summarize. Borrows `Compose`/`Node`/`poll_until` from `binary-upgrade-tests/run.py`. |
| `test_soak.py` | Unit tests for `render_summary`. Run with `python3 -m unittest`.                                                         |

Wired into `.github/workflows/build.yml` as the `memory-soak-pr` / `memory-soak-non-pr` jobs, gated on the docker build
steps. Runs on every PR, push, and tag. Matrix over the 2 genesis files (drb-header, epoch-reward). 300s soak per matrix
entry. Artifact retention: 90 days.

### Env vars

| Var                          | Default                        | Purpose                                         |
| ---------------------------- | ------------------------------ | ----------------------------------------------- |
| `DURATION_SECONDS`           | `300`                          | Sampling duration.                              |
| `DOCKER_TAG`                 | `main`                         | Docker compose image tag.                       |
| `ESPRESSO_NODE_GENESIS_FILE` | `genesis/demo-drb-header.toml` | Passed through to compose.                      |
| `GENESIS_LABEL`              | derived from genesis filename  | Heading on the summary.                         |
| `OUTPUT_DIR`                 | `./soak-samples`               | Where JSONL + `summary.md` are written.         |
| `SKIP_COMPOSE_UP`            | unset                          | If `1`, skip compose pull/up (already running). |
| `GITHUB_STEP_SUMMARY`        | (set by GH Actions)            | If set, summary is appended here too.           |

### Run locally

```bash
DURATION_SECONDS=30 python3 crates/process-metrics/scripts/soak.py
python3 -m unittest crates/process-metrics/scripts/test_soak.py
```
