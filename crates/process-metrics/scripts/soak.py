#!/usr/bin/env python3
"""Memory soak orchestration: boot compose, sample container + node metrics,
render a markdown summary."""

from __future__ import annotations

import argparse
import json
import logging
import os
import re
import shlex
import statistics
import subprocess
import sys
import time
import urllib.error
import urllib.request
from collections import defaultdict
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

log = logging.getLogger("memory-soak")

NODE_INDICES = (0, 1, 2, 3, 4)
PROGRESS_INTERVAL = 30
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
    skip_compose_up: bool

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
            skip_compose_up=os.environ.get("SKIP_COMPOSE_UP") == "1",
        )


@dataclass(frozen=True)
class Compose:
    docker_tag: str

    @property
    def base_args(self) -> list[str]:
        return [
            "docker", "compose",
            "--project-directory", str(REPO_ROOT),
            "--env-file", str(REPO_ROOT / ".env"),
            "-f", str(REPO_ROOT / "docker-compose.yaml"),
        ]  # fmt: skip

    def run(
        self,
        *args: str,
        check: bool = True,
        capture: bool = False,
    ) -> subprocess.CompletedProcess[str]:
        env = os.environ | {"DOCKER_TAG": self.docker_tag}
        return subprocess.run(
            self.base_args + list(args),
            cwd=REPO_ROOT,
            env=env,
            check=check,
            capture_output=capture,
            text=True,
        )

    def pull(self) -> None:
        self.run("pull", "--policy", "missing")

    def up(self) -> None:
        self.run("up", "-d")

    def services(self) -> list[str]:
        out = self.run("config", "--services", capture=True).stdout
        return [s for s in out.splitlines() if s]


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

    def ready(self) -> bool:
        try:
            with urllib.request.urlopen(
                f"{self.api_url}/v0/status/block-height", timeout=2.0
            ) as resp:
                return resp.status == 200
        except (urllib.error.URLError, TimeoutError, ConnectionError, OSError):
            return False

    def wait_ready(self, timeout: float = 300.0) -> None:
        poll_until(self.ready, f"{self} /v0/status/block-height", timeout)

    def scrape_metrics(self, timeout: float = 2.0) -> list[tuple[str, float]]:
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
        return out


def poll_until(
    check: Callable[[], bool],
    desc: str,
    timeout: float,
    interval: float = 2.0,
) -> None:
    deadline = time.monotonic() + timeout
    while True:
        if check():
            return
        if time.monotonic() > deadline:
            raise TimeoutError(f"Timed out after {timeout:g}s waiting for {desc}")
        time.sleep(interval)


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


def _collect_node_metrics(ts: int, node: Node) -> list[dict]:
    try:
        scraped = node.scrape_metrics()
    except (urllib.error.URLError, TimeoutError, ConnectionError, OSError) as e:
        log.debug(f"{node} metrics scrape failed at ts={ts}: {e}")
        return []
    return [
        {"ts": ts, "node": node.api_url, "metric": name, "value": value}
        for name, value in scraped
    ]


def sample_once(
    ts: int,
    nodes: list[Node],
    docker_path: Path,
    metrics_path: Path,
    executor: ThreadPoolExecutor,
) -> int:
    """Take one concurrent sample. Returns total rows written."""
    docker_fut = executor.submit(_collect_docker_stats, ts)
    node_futs = [executor.submit(_collect_node_metrics, ts, n) for n in nodes]

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
            sample_once(ts, nodes, docker_path, metrics_path, executor)
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


def p99(values: list[float]) -> float:
    if not values:
        raise ValueError("p99 of empty list")
    if len(values) < 2:
        return max(values)
    return statistics.quantiles(values, n=100)[98]


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


def render_service_table(by_name: dict[str, list[dict]]) -> str:
    lines = [
        "| Service | Min RSS | Avg RSS | Max RSS | p99 RSS | Avg CPU% | Max CPU% |",
        "|---------|---------|---------|---------|---------|----------|----------|",
    ]
    has_rows = False
    for name in sorted(by_name.keys()):
        samples = by_name[name]
        if not samples:
            lines.append(f"| {name} | n/a | n/a | n/a | n/a | n/a | n/a |")
            continue
        has_rows = True
        rss = [s["rss"] for s in samples]
        cpu = [s["cpu"] for s in samples]
        lines.append(
            f"| {name} | {human_bytes(min(rss))} | {human_bytes(statistics.fmean(rss))} | "
            f"{human_bytes(max(rss))} | {human_bytes(p99(rss))} | "
            f"{statistics.fmean(cpu):.1f} | {max(cpu):.1f} |"
        )

    if has_rows:
        totals: dict[int, float] = defaultdict(float)
        for samples in by_name.values():
            for s in samples:
                totals[s["ts"]] += s["rss"]
        ts_totals = list(totals.values())
        lines.append(
            f"| **Total (per-ts sum)** | {human_bytes(min(ts_totals))} | "
            f"{human_bytes(statistics.fmean(ts_totals))} | {human_bytes(max(ts_totals))} | "
            f"{human_bytes(p99(ts_totals))} | | |"
        )

    return "\n".join(lines)


def node_url_to_container(url: str) -> str | None:
    m = _NODE_PORT_RE.search(url)
    if not m:
        return None
    idx = int(m.group(1)) - 24000
    if idx < 0:
        return None
    return f"espresso-node-{idx}"


def render_crosscheck(
    by_name: dict[str, list[dict]],
    node_metrics: list[dict],
) -> str:
    per_node_rss: dict[str, float] = defaultdict(float)
    for m in node_metrics:
        try:
            if m["metric"] != "process_resident_memory_bytes":
                continue
            url = m["node"]
            val = float(m["value"])
        except (KeyError, ValueError, TypeError) as e:
            log.warning(f"skipping unparsable metric row: {e}")
            continue
        if val > per_node_rss[url]:
            per_node_rss[url] = val

    if not per_node_rss:
        return ""

    docker_max: dict[str, float] = {}
    for name, samples in by_name.items():
        if samples:
            docker_max[name] = max(s["rss"] for s in samples)

    url_to_container: dict[str, str] = {}
    for url in per_node_rss:
        c = node_url_to_container(url)
        if c is not None:
            url_to_container[url] = c

    lines = [
        "",
        "### Node RSS cross-check (docker stats vs in-process gauge)",
        "",
        "| Node | docker stats max RSS | process_resident_memory_bytes max | diff |",
        "|------|----------------------|-----------------------------------|------|",
    ]
    rows_by_container: dict[str, tuple[str, float | None, float | None]] = {}
    for url, container in url_to_container.items():
        d = docker_max.get(container)
        p = per_node_rss.get(url)
        rows_by_container[container] = (url, d, p)

    for c in docker_max:
        if c.startswith("espresso-node-") and c not in rows_by_container:
            rows_by_container[c] = (c, docker_max[c], None)

    for container in sorted(rows_by_container.keys()):
        _url, d, p = rows_by_container[container]
        d_str = human_bytes(d) if d is not None else "n/a"
        p_str = human_bytes(p) if p is not None else "n/a"
        if d is None or p is None:
            diff_str = "n/a"
        else:
            diff = p - d
            sign = "+" if diff >= 0 else "-"
            diff_str = f"{sign}{human_bytes(abs(diff))}"
        lines.append(f"| {container} | {d_str} | {p_str} | {diff_str} |")

    return "\n".join(lines)


def render_summary(
    docker_path: Path,
    metrics_path: Path,
    label: str,
    duration_seconds: int | None,
) -> str:
    if not docker_path.exists():
        raise FileNotFoundError(f"missing required file: {docker_path}")

    docker_rows = load_jsonl(docker_path)
    by_name = group_docker_stats(docker_rows)

    all_ts = [s["ts"] for samples in by_name.values() for s in samples]
    n_samples = len(docker_rows)
    if duration_seconds is None:
        duration_seconds = (max(all_ts) - min(all_ts)) if all_ts else 0

    peak = compute_peak_total(by_name)
    if peak is None:
        peak_str = "**Peak total memory: n/a**"
    else:
        peak_val, peak_ts = peak
        iso = datetime.fromtimestamp(peak_ts, tz=timezone.utc).isoformat()
        peak_str = f"**Peak total memory: {human_bytes(peak_val)}** (at {iso})"

    parts = [
        f"## Memory soak: {label} ({duration_seconds}s, {n_samples} samples)",
        "",
        peak_str,
        "",
        render_service_table(by_name),
    ]

    if metrics_path.exists():
        metric_rows = load_jsonl(metrics_path)
        if metric_rows:
            cross = render_crosscheck(by_name, metric_rows)
            if cross:
                parts.append(cross)

    return "\n".join(parts) + "\n"


# ---------- Orchestration ----------


def run_soak(config: Config) -> None:
    config.output_dir.mkdir(parents=True, exist_ok=True)
    os.environ.setdefault("ESPRESSO_NODE_GENESIS_FILE", config.genesis_file)
    os.environ.setdefault("ESPRESSO_SEQUENCER_GENESIS_FILE", config.genesis_file)
    load_project_env()

    compose = Compose(docker_tag=config.docker_tag)
    if not config.skip_compose_up:
        log.info(f"docker compose pull (tag={config.docker_tag})")
        compose.pull()
        log.info(f"docker compose up -d (tag={config.docker_tag})")
        compose.up()
    else:
        log.info("SKIP_COMPOSE_UP=1; assuming compose stack is already running")

    nodes = [Node.from_index(i) for i in NODE_INDICES]
    log.info(f"Waiting for {len(nodes)} nodes to become ready")
    for n in nodes:
        n.wait_ready()
        log.info(f"{n} ready")

    docker_path, metrics_path = run_sampling(config, nodes)

    summary = render_summary(
        docker_path, metrics_path, config.genesis_label, config.duration_seconds
    )
    sys.stdout.write(summary)

    summary_path = config.output_dir / "summary.md"
    summary_path.write_text(summary)
    log.info(f"wrote {summary_path}")

    if config.github_step_summary is not None:
        with config.github_step_summary.open("a") as f:
            f.write(summary)
        log.info(f"appended summary to {config.github_step_summary}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log-level", default="INFO")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    logging.basicConfig(
        level=args.log_level,
        format="%(levelname)s %(name)s: %(message)s",
        stream=sys.stderr,
    )

    if not (REPO_ROOT / ".env").exists():
        log.error(".env not found. Copy .env.docker.example to .env first.")
        return 1

    config = Config.from_env()
    run_soak(config)
    return 0


if __name__ == "__main__":
    sys.exit(main())
