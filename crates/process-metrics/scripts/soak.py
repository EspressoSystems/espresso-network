#!/usr/bin/env python3
"""Memory soak sampling: sample container + node metrics, render a markdown
summary with a memory-over-time chart.

The CI workflow is responsible for `docker compose up` and gating readiness via
`scripts/smoke-test-demo`; this script only samples and renders.
"""

# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "matplotlib>=3.9",
# ]
# ///

from __future__ import annotations

import argparse
import json
import logging
import os
import re
import shlex
import subprocess
import sys
import time
import urllib.error
import urllib.request
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

log = logging.getLogger("memory-soak")

NODE_INDICES = (0, 1, 2, 3, 4)
PROGRESS_INTERVAL = 30
MERMAID_MAX_POINTS = 30
METRIC_NAMES = frozenset(
    (
        "process_resident_memory_bytes",
        "process_virtual_memory_bytes",
        "process_open_fds",
        "process_threads",
        "process_uptime_seconds",
    )
)

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)

_MEM_UNIT_BYTES = {
    "B": 1,
    "KB": 1_000,
    "MB": 1_000_000,
    "GB": 1_000_000_000,
    "TB": 1_000_000_000_000,
    "KIB": 1024,
    "MIB": 1024**2,
    "GIB": 1024**3,
    "TIB": 1024**4,
}
_MEM_RE = re.compile(r"^\s*([0-9]+(?:\.[0-9]+)?)\s*([A-Za-z]+)\s*$")
_NODE_PORT_RE = re.compile(r":(\d+)(?:/|$)")
_ESPRESSO_NODE_RE = re.compile(r"espresso-node-(\d+)")
_METRIC_LINE_RE = re.compile(
    r"^(process_[a-z_]+)(?:\{[^}]*\})?\s+(-?[0-9]+(?:\.[0-9]+)?(?:[eE][-+]?[0-9]+)?)\s*$"
)


@dataclass(frozen=True)
class Config:
    duration_seconds: int
    docker_tag: str
    genesis_file: str
    genesis_label: str
    output_dir: Path
    github_step_summary: Path | None

    @classmethod
    def from_env(cls) -> Config:
        genesis_file = os.environ.get(
            "ESPRESSO_NODE_GENESIS_FILE", "genesis/demo-drb-header.toml"
        )
        default_label = Path(genesis_file).stem
        gss = os.environ.get("GITHUB_STEP_SUMMARY")
        return cls(
            duration_seconds=int(os.environ.get("DURATION_SECONDS", "300")),
            docker_tag=os.environ.get("DOCKER_TAG", "main"),
            genesis_file=genesis_file,
            genesis_label=os.environ.get("GENESIS_LABEL", default_label),
            output_dir=Path(os.environ.get("OUTPUT_DIR", "./soak-samples")),
            github_step_summary=Path(gss) if gss else None,
        )


@dataclass(frozen=True)
class Node:
    index: int
    api_url: str

    def __str__(self) -> str:
        return f"espresso-node-{self.index}"

    @classmethod
    def from_index(cls, index: int) -> Node:
        var = f"ESPRESSO_NODE_{index}_API_PORT"
        port = os.environ.get(var)
        if not port:
            raise RuntimeError(f"Env var {var} not set")
        return cls(index=index, api_url=f"http://localhost:{port}")

    def scrape_metrics(self, timeout: float = 2.0) -> tuple[str, list[tuple[str, float]]]:
        with urllib.request.urlopen(
            f"{self.api_url}/v0/status/metrics", timeout=timeout
        ) as resp:
            body = resp.read().decode()
        out: list[tuple[str, float]] = []
        for line in body.splitlines():
            if not line.startswith("process_"):
                continue
            m = _METRIC_LINE_RE.match(line)
            if not m:
                continue
            name = m.group(1)
            if name not in METRIC_NAMES:
                continue
            out.append((name, float(m.group(2))))
        return body, out


def load_project_env() -> None:
    """Source the repo .env via bash so ${VAR} interpolation works, then copy
    .env-defined keys into os.environ without clobbering existing values."""
    path = REPO_ROOT / ".env"
    keys: set[str] = set()
    for raw in path.read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        keys.add(line.split("=", 1)[0].strip())

    result = subprocess.run(
        ["bash", "-c", f"set -a; . {shlex.quote(str(path))}; env -0"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
    )
    for entry in result.stdout.decode().split("\0"):
        k, sep, v = entry.partition("=")
        if sep and k in keys:
            os.environ.setdefault(k, v)


# ---------- Sampling ----------


def _collect_docker_stats(ts: int) -> list[dict]:
    try:
        raw = subprocess.run(
            ["docker", "stats", "--no-stream", "--format", "{{json .}}"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    except (subprocess.CalledProcessError, OSError) as e:
        log.debug(f"docker stats failed at ts={ts}: {e}")
        return []

    out: list[dict] = []
    for line in raw.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as e:
            log.debug(f"docker stats unparsable line at ts={ts}: {e}")
            continue
        row["ts"] = ts
        out.append(row)
    return out


_NODE_DIAGNOSED: set[int] = set()


def _collect_node_metrics(ts: int, node: Node, output_dir: Path) -> list[dict]:
    try:
        body, scraped = node.scrape_metrics()
    except (urllib.error.URLError, TimeoutError, ConnectionError, OSError) as e:
        if node.index not in _NODE_DIAGNOSED:
            _NODE_DIAGNOSED.add(node.index)
            log.info(f"{node} first metrics scrape failed: {e}")
        return []
    if node.index not in _NODE_DIAGNOSED:
        _NODE_DIAGNOSED.add(node.index)
        log.info(f"{node} first scrape: {len(body)} bytes, {len(scraped)} process_* matched")
        dump = output_dir / f"raw-metrics-{node}.txt"
        dump.write_text(body)
        log.info(f"saved first raw response to {dump}")
    return [
        {"ts": ts, "node": node.api_url, "metric": name, "value": value}
        for name, value in scraped
    ]


def sample_once(
    ts: int,
    nodes: list[Node],
    docker_path: Path,
    metrics_path: Path,
    output_dir: Path,
    executor: ThreadPoolExecutor,
) -> int:
    """Take one concurrent sample. Returns total rows written."""
    docker_fut = executor.submit(_collect_docker_stats, ts)
    node_futs = [
        executor.submit(_collect_node_metrics, ts, n, output_dir) for n in nodes
    ]

    docker_rows = docker_fut.result()
    node_rows: list[dict] = []
    for fut in node_futs:
        node_rows.extend(fut.result())

    written = 0
    if docker_rows:
        with docker_path.open("a") as f:
            for row in docker_rows:
                f.write(json.dumps(row) + "\n")
                written += 1
    if node_rows:
        with metrics_path.open("a") as f:
            for row in node_rows:
                f.write(json.dumps(row) + "\n")
                written += 1
    return written


def run_sampling(config: Config, nodes: list[Node]) -> tuple[Path, Path]:
    docker_path = config.output_dir / "docker-stats.jsonl"
    metrics_path = config.output_dir / "node-metrics.jsonl"
    docker_path.write_text("")
    metrics_path.write_text("")

    if config.duration_seconds <= 0:
        log.warning(f"duration_seconds={config.duration_seconds}; skipping sampling")
        return docker_path, metrics_path

    t0 = time.time()
    next_progress = t0 + PROGRESS_INTERVAL
    interval = 1.0
    samples = 0
    with ThreadPoolExecutor(max_workers=6) as executor:
        while True:
            now = time.time()
            elapsed = now - t0
            if elapsed >= config.duration_seconds:
                break

            ts = int(now)
            sample_once(
                ts, nodes, docker_path, metrics_path, config.output_dir, executor
            )
            samples += 1

            if now >= next_progress:
                d_rows = _line_count(docker_path)
                m_rows = _line_count(metrics_path)
                log.info(
                    f"sample t={int(elapsed)}s docker-rows={d_rows} metric-rows={m_rows}"
                )
                next_progress += PROGRESS_INTERVAL

            target = t0 + samples * interval
            sleep_for = target - time.time()
            if sleep_for > 0:
                time.sleep(sleep_for)

    log.info(
        f"done samples={samples} docker-rows={_line_count(docker_path)} "
        f"metric-rows={_line_count(metrics_path)} output={config.output_dir}"
    )
    return docker_path, metrics_path


def _line_count(path: Path) -> int:
    if not path.exists():
        return 0
    with path.open() as f:
        return sum(1 for _ in f)


# ---------- Summary rendering ----------


def parse_mem_bytes(s: str) -> float:
    m = _MEM_RE.match(s)
    if not m:
        raise ValueError(f"unparsable memory string: {s!r}")
    value = float(m.group(1))
    unit = m.group(2).upper()
    if unit not in _MEM_UNIT_BYTES:
        raise ValueError(f"unknown memory unit: {unit!r} in {s!r}")
    return value * _MEM_UNIT_BYTES[unit]


def parse_mem_usage_left(mem_usage: str) -> float:
    return parse_mem_bytes(mem_usage.split("/", 1)[0].strip())


def parse_percent(s: str) -> float:
    return float(s.rstrip("%").strip())


def human_bytes(b: float) -> str:
    if b < 1_000:
        return f"{int(round(b))} B"
    if b < 1_000_000:
        v = b / 1_000
        return f"{v:.0f} kB" if v == int(v) else f"{v:.2f} kB"
    if b < 1_000_000_000:
        v = b / 1_000_000
        return f"{v:.0f} MB" if v == int(v) else f"{v:.2f} MB"
    if b < 1_000_000_000_000:
        v = b / 1_000_000_000
        return f"{v:.0f} GB" if v == int(v) else f"{v:.2f} GB"
    v = b / 1_000_000_000_000
    return f"{v:.0f} TB" if v == int(v) else f"{v:.2f} TB"


def load_jsonl(path: Path) -> list[dict]:
    out: list[dict] = []
    with path.open() as f:
        for lineno, raw in enumerate(f, start=1):
            line = raw.strip()
            if not line:
                continue
            try:
                out.append(json.loads(line))
            except json.JSONDecodeError as e:
                log.warning(f"skipping malformed line {lineno} in {path}: {e}")
    return out


def group_docker_stats(rows: list[dict]) -> dict[str, list[dict]]:
    by_name: dict[str, list[dict]] = defaultdict(list)
    for row in rows:
        try:
            name = row["Name"]
            rss = parse_mem_usage_left(row["MemUsage"])
            cpu = parse_percent(row["CPUPerc"])
            ts = int(row["ts"])
        except (KeyError, ValueError) as e:
            log.warning(f"skipping unparsable docker-stats row: {e}")
            continue
        if not name:
            continue
        by_name[name].append({"ts": ts, "rss": rss, "cpu": cpu})
    return by_name


def compute_peak_total(by_name: dict[str, list[dict]]) -> tuple[float, int] | None:
    totals: dict[int, float] = defaultdict(float)
    for samples in by_name.values():
        for s in samples:
            totals[s["ts"]] += s["rss"]
    if not totals:
        return None
    peak_ts, peak_val = max(totals.items(), key=lambda kv: kv[1])
    return peak_val, peak_ts


def node_url_to_container(url: str) -> str | None:
    m = _NODE_PORT_RE.search(url)
    if not m:
        return None
    idx = int(m.group(1)) - 24000
    if idx < 0:
        return None
    return f"espresso-node-{idx}"


def short_node_name(raw: str) -> str | None:
    """Return canonical `espresso-node-N` for any docker name containing it.

    Returns None if `raw` is empty or doesn't match an espresso-node container.
    """
    if not raw:
        return None
    m = _ESPRESSO_NODE_RE.search(raw)
    if not m:
        return None
    return f"espresso-node-{m.group(1)}"


def filter_espresso_nodes(
    by_name: dict[str, list[dict]],
) -> dict[str, list[dict]]:
    """Filter and rename docker rows to espresso-node-N keys only."""
    out: dict[str, list[dict]] = defaultdict(list)
    for name, samples in by_name.items():
        short = short_node_name(name)
        if short is None:
            continue
        out[short].extend(samples)
    return dict(out)


def per_node_process_rss(node_metrics: list[dict]) -> dict[str, float]:
    """Map espresso-node-N -> max process_resident_memory_bytes."""
    by_container: dict[str, float] = {}
    for m in node_metrics:
        try:
            if m["metric"] != "process_resident_memory_bytes":
                continue
            url = m["node"]
            val = float(m["value"])
        except (KeyError, ValueError, TypeError) as e:
            log.warning(f"skipping unparsable metric row: {e}")
            continue
        container = node_url_to_container(url)
        if container is None:
            continue
        if val > by_container.get(container, 0.0):
            by_container[container] = val
    return by_container


def render_service_table(
    nodes_by_name: dict[str, list[dict]],
    process_rss_max: dict[str, float],
) -> str:
    """Render the espresso-node-only summary table.

    Columns: Service, Max RSS (docker), Max RSS (process gauge), Max CPU%.
    """
    lines = [
        "| Service | Max RSS (docker) | Max RSS (process gauge) | Max CPU% |",
        "|---------|------------------|-------------------------|----------|",
    ]
    docker_total = 0.0
    gauge_total = 0.0
    gauge_count = 0
    for name in sorted(nodes_by_name.keys()):
        samples = nodes_by_name[name]
        if not samples:
            lines.append(f"| {name} | n/a | n/a | n/a |")
            continue
        rss = [s["rss"] for s in samples]
        cpu = [s["cpu"] for s in samples]
        d_max = max(rss)
        docker_total += d_max
        gauge = process_rss_max.get(name)
        if gauge is not None:
            gauge_total += gauge
            gauge_count += 1
            gauge_str = human_bytes(gauge)
        else:
            gauge_str = "n/a"
        lines.append(
            f"| {name} | {human_bytes(d_max)} | {gauge_str} | {max(cpu):.1f} |"
        )

    if nodes_by_name:
        gauge_total_str = human_bytes(gauge_total) if gauge_count > 0 else "n/a"
        lines.append(
            f"| **Total (sum)** | {human_bytes(docker_total)} | {gauge_total_str} | |"
        )

    return "\n".join(lines)


def _build_series(
    by_name: dict[str, list[dict]],
) -> tuple[list[str], dict[str, list[tuple[int, float]]], int]:
    """Return (sorted_names, name -> [(rel_seconds, rss_mb)], min_ts).

    rss is converted to SI megabytes; timestamps are relative to the earliest
    sample across all services.
    """
    all_ts = [s["ts"] for samples in by_name.values() for s in samples]
    if not all_ts:
        return [], {}, 0
    min_ts = min(all_ts)
    series: dict[str, list[tuple[int, float]]] = {}
    for name, samples in by_name.items():
        if not samples:
            continue
        points = sorted(
            ((int(s["ts"]) - min_ts, s["rss"] / 1_000_000) for s in samples),
            key=lambda p: p[0],
        )
        series[name] = points
    return sorted(series.keys()), series, min_ts


def render_rss_png(by_name: dict[str, list[dict]], label: str, out_path: Path) -> bool:
    """Render an RSS-over-time PNG for espresso-node containers.

    Returns True if a chart was written.
    """
    names, series, _ = _build_series(by_name)
    if not names:
        return False

    try:
        import matplotlib
    except ImportError as e:
        log.debug(f"matplotlib not installed; skipping PNG chart: {e}")
        return False

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig, ax = plt.subplots(figsize=(12, 6), dpi=100)
    for name in names:
        xs = [p[0] for p in series[name]]
        ys = [p[1] for p in series[name]]
        (line,) = ax.plot(xs, ys, linewidth=1.2)
        ax.annotate(
            name,
            xy=(xs[-1], ys[-1]),
            xytext=(4, 0),
            textcoords="offset points",
            color=line.get_color(),
            fontsize="x-small",
            va="center",
            ha="left",
        )
    ax.set_title(f"Memory soak: {label} (RSS over time)")
    ax.set_xlabel("seconds")
    ax.set_ylabel("RSS (MB)")
    ax.grid(True, alpha=0.3)
    # Pad the right side so end-of-line annotations don't clip.
    xmin, xmax = ax.get_xlim()
    ax.set_xlim(xmin, xmax + (xmax - xmin) * 0.08)
    fig.tight_layout()
    fig.savefig(out_path)
    plt.close(fig)
    return True


def _subsample(points: list[tuple[int, float]], max_n: int) -> list[tuple[int, float]]:
    if len(points) <= max_n:
        return points
    step = len(points) / max_n
    out: list[tuple[int, float]] = []
    for i in range(max_n):
        out.append(points[int(i * step)])
    # ensure last point preserved
    if out[-1] != points[-1]:
        out[-1] = points[-1]
    return out


def render_mermaid_chart(by_name: dict[str, list[dict]]) -> str:
    names, series, _ = _build_series(by_name)
    if not names:
        return ""

    subsampled = {name: _subsample(series[name], MERMAID_MAX_POINTS) for name in names}
    max_x = max(pts[-1][0] for pts in subsampled.values())
    all_ys = [y for pts in subsampled.values() for _, y in pts]
    y_max = max(all_ys) if all_ys else 0.0
    y_top = max(1.0, y_max * 1.1)

    lines = [
        "```mermaid",
        "xychart-beta",
        '    title "RSS over time (MB)"',
        f'    x-axis "seconds" 0 --> {max_x}',
        f'    y-axis "MB" 0 --> {y_top:.0f}',
    ]
    for name in names:
        ys = [f"{y:.1f}" for _, y in subsampled[name]]
        lines.append(f'    line "{name}" [{", ".join(ys)}]')
    lines.append("```")
    return "\n".join(lines)


def render_summary(
    docker_path: Path,
    metrics_path: Path,
    label: str,
    duration_seconds: int | None,
    output_dir: Path,
) -> str:
    if not docker_path.exists():
        raise FileNotFoundError(f"missing required file: {docker_path}")

    docker_rows = load_jsonl(docker_path)
    by_name = group_docker_stats(docker_rows)

    all_ts = [s["ts"] for samples in by_name.values() for s in samples]
    n_samples = len(docker_rows)
    if duration_seconds is None:
        duration_seconds = (max(all_ts) - min(all_ts)) if all_ts else 0

    header = f"## Memory soak: {label} ({duration_seconds}s, {n_samples} samples)"

    if not by_name:
        return f"{header}\n\n**No data collected.**\n"

    peak = compute_peak_total(by_name)
    if peak is None:
        peak_str = "**Peak total memory (all containers): n/a**"
    else:
        peak_val, peak_ts = peak
        iso = datetime.fromtimestamp(peak_ts, tz=timezone.utc).isoformat()
        peak_str = (
            f"**Peak total memory (all containers): {human_bytes(peak_val)}** "
            f"(at {iso})"
        )

    nodes_by_name = filter_espresso_nodes(by_name)

    process_rss_max: dict[str, float] = {}
    if metrics_path.exists():
        metric_rows = load_jsonl(metrics_path)
        if metric_rows:
            process_rss_max = per_node_process_rss(metric_rows)

    parts = [
        header,
        "",
        peak_str,
        "",
        render_service_table(nodes_by_name, process_rss_max),
    ]

    mermaid = render_mermaid_chart(nodes_by_name)
    if mermaid:
        parts.extend(["", "### Memory over time", "", mermaid])

    png_path = output_dir / "rss-over-time.png"
    try:
        wrote_png = render_rss_png(nodes_by_name, label, png_path)
    except Exception as e:
        log.warning(f"failed to render PNG chart: {e}")
        wrote_png = False
    if wrote_png:
        parts.extend(["", f"Full-resolution chart in artifact: `{png_path.name}`"])

    return "\n".join(parts) + "\n"


# ---------- Subcommands ----------


def cmd_sample(config: Config) -> int:
    config.output_dir.mkdir(parents=True, exist_ok=True)
    os.environ.setdefault("ESPRESSO_NODE_GENESIS_FILE", config.genesis_file)
    os.environ.setdefault("ESPRESSO_SEQUENCER_GENESIS_FILE", config.genesis_file)

    if not (REPO_ROOT / ".env").exists():
        log.error(".env not found. Copy .env.docker.example to .env first.")
        return 1
    load_project_env()

    nodes = [Node.from_index(i) for i in NODE_INDICES]
    docker_path, metrics_path = run_sampling(config, nodes)
    log.info(
        f"sample done docker-rows={_line_count(docker_path)} "
        f"metric-rows={_line_count(metrics_path)} output={config.output_dir}"
    )
    return 0


def cmd_render(config: Config) -> int:
    docker_path = config.output_dir / "docker-stats.jsonl"
    metrics_path = config.output_dir / "node-metrics.jsonl"

    if not docker_path.exists():
        log.error(f"missing {docker_path}; run `soak.py sample` first")
        return 2

    config.output_dir.mkdir(parents=True, exist_ok=True)

    summary = render_summary(
        docker_path,
        metrics_path,
        config.genesis_label,
        None,
        config.output_dir,
    )
    sys.stdout.write(summary)

    summary_path = config.output_dir / "summary.md"
    summary_path.write_text(summary)
    log.info(f"wrote {summary_path}")

    if config.github_step_summary is not None:
        with config.github_step_summary.open("a") as f:
            f.write(summary)
        log.info(f"appended summary to {config.github_step_summary}")
    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    sample_p = subparsers.add_parser(
        "sample", help="sample docker stats + node /metrics into OUTPUT_DIR"
    )
    sample_p.add_argument("--log-level", default="INFO")

    render_p = subparsers.add_parser(
        "render", help="render summary.md + rss-over-time.png from OUTPUT_DIR"
    )
    render_p.add_argument("--log-level", default="INFO")

    return parser.parse_args()


def main() -> int:
    args = parse_args()
    logging.basicConfig(
        level=args.log_level,
        format="%(levelname)s %(name)s: %(message)s",
        stream=sys.stderr,
    )

    config = Config.from_env()
    if args.command == "sample":
        return cmd_sample(config)
    if args.command == "render":
        return cmd_render(config)
    log.error(f"unknown command: {args.command}")
    return 2


if __name__ == "__main__":
    sys.exit(main())
