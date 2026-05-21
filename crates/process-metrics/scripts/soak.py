#!/usr/bin/env python3
"""Memory soak sampling: sample container + node metrics, render a markdown
summary with a memory-over-time chart.

The CI workflow is responsible for `docker compose up` and gating readiness via
`scripts/smoke-test-demo`; this script only samples and renders.
"""

# /// script
# requires-python = ">=3.12"
# dependencies = [
#     "click",
#     "humanize",
#     "matplotlib>=3.9",
#     "pandas",
#     "prometheus-client",
#     "python-dotenv",
# ]
# ///

from __future__ import annotations

import json
import logging
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import click
import humanize
import matplotlib
import pandas as pd
from dotenv import load_dotenv
from prometheus_client.parser import text_string_to_metric_families

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

log = logging.getLogger("memory-soak")

NODE_INDICES = (0, 1, 2, 3, 4)
PROGRESS_INTERVAL = 30
MERMAID_MAX_POINTS = 30
METRIC_PREFIX = "consensus_"
NODE_BASE_PORT = 24000
MEM_UNIT_SCALE = {"KiB": 1024, "MiB": 1024**2, "GiB": 1024**3, "TiB": 1024**4}
# matplotlib tab10 first 5 - used for Mermaid + PNG so legend colors match.
PLOT_PALETTE = ("#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd")
METRIC_NAMES = frozenset(
    f"{METRIC_PREFIX}process_{s}"
    for s in (
        "resident_memory_bytes",
        "virtual_memory_bytes",
        "open_fds",
        "threads",
        "uptime_seconds",
    )
)

REPO_ROOT = Path(
    subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
)


# ---------- Sampling ----------


def scrape_node(idx: int, port: int) -> list[dict]:
    """Scrape one node's /v0/status/metrics endpoint."""
    url = f"http://localhost:{port}"
    try:
        with urllib.request.urlopen(f"{url}/v0/status/metrics", timeout=2.0) as resp:
            body = resp.read().decode()
    except (urllib.error.URLError, TimeoutError, ConnectionError, OSError) as e:
        log.debug(f"espresso-node-{idx} scrape failed: {e}")
        return []
    out: list[dict] = []
    for family in text_string_to_metric_families(body):
        if family.name not in METRIC_NAMES:
            continue
        for sample in family.samples:
            if sample.name == family.name:
                out.append({"node": url, "metric": sample.name, "value": sample.value})
    return out


def collect_docker_stats() -> list[dict]:
    try:
        raw = subprocess.run(
            ["docker", "stats", "--no-stream", "--format", "{{json .}}"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    except (subprocess.CalledProcessError, OSError) as e:
        log.debug(f"docker stats failed: {e}")
        return []
    rows = []
    for line in filter(None, (ln.strip() for ln in raw.splitlines())):
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError as e:
            log.debug(f"docker stats unparsable line: {e}")
    return rows


def _append_jsonl(f, ts: int, rows: list[dict]) -> None:
    for row in rows:
        row["ts"] = ts
        f.write(json.dumps(row) + "\n")


def run_sampling(output_dir: Path, duration_seconds: int) -> None:
    docker_path = output_dir / "docker-stats.jsonl"
    metrics_path = output_dir / "node-metrics.jsonl"
    docker_path.write_text("")
    metrics_path.write_text("")
    if duration_seconds <= 0:
        log.warning(f"duration_seconds={duration_seconds}; skipping sampling")
        return

    node_ports = [
        (i, int(os.environ[f"ESPRESSO_NODE_{i}_API_PORT"])) for i in NODE_INDICES
    ]
    t0 = time.time()
    next_progress = t0 + PROGRESS_INTERVAL
    samples = 0
    with (
        ThreadPoolExecutor(max_workers=6) as ex,
        docker_path.open("a") as docker_f,
        metrics_path.open("a") as metrics_f,
    ):
        while (now := time.time()) - t0 < duration_seconds:
            ts = int(now)
            d_fut = ex.submit(collect_docker_stats)
            n_futs = [ex.submit(scrape_node, i, p) for i, p in node_ports]
            _append_jsonl(docker_f, ts, d_fut.result())
            for fut in n_futs:
                _append_jsonl(metrics_f, ts, fut.result())
            samples += 1
            if now >= next_progress:
                log.info(f"sample t={int(now - t0)}s samples={samples}")
                next_progress += PROGRESS_INTERVAL
            if (sleep_for := (t0 + samples) - time.time()) > 0:
                time.sleep(sleep_for)

    log.info(f"done samples={samples} output={output_dir}")


# ---------- Summary rendering ----------


def _load_docker(path: Path) -> pd.DataFrame:
    """Load docker-stats.jsonl, parse MemUsage, filter to espresso-node-N."""
    if not path.exists() or path.stat().st_size == 0:
        return pd.DataFrame()
    df = pd.read_json(path, lines=True)
    if df.empty:
        return df
    short = df["Name"].astype(str).str.extract(r"espresso-node-(\d+)", expand=False)
    df = df.loc[short.notna()].copy()
    df["Name"] = "espresso-node-" + short.dropna()
    mem = df["MemUsage"].str.extract(r"([\d.]+)\s*(KiB|MiB|GiB|TiB)")
    df["rss"] = mem[0].astype(float) * mem[1].map(MEM_UNIT_SCALE)
    df["cpu"] = df["CPUPerc"].str.rstrip("%").astype(float)
    df["ts"] = df["ts"].astype(int)
    return df[["ts", "Name", "rss", "cpu"]]


def _process_rss_max(metrics_path: Path) -> dict[str, float]:
    """Map espresso-node-N -> max consensus_process_resident_memory_bytes."""
    if not metrics_path.exists() or metrics_path.stat().st_size == 0:
        return {}
    df = pd.read_json(metrics_path, lines=True)
    df = df[df["metric"] == METRIC_PREFIX + "process_resident_memory_bytes"]
    if df.empty:
        return {}
    port = df["node"].str.extract(r":(\d+)", expand=False).astype(int)
    df = df.assign(name="espresso-node-" + (port - NODE_BASE_PORT).astype(str))
    return df.groupby("name")["value"].max().to_dict()


def _hb(b: float) -> str:
    return humanize.naturalsize(b, binary=False)


def _render_table(df: pd.DataFrame, process_rss_max: dict[str, float]) -> str:
    lines = [
        "| Service | Max RSS (docker) | Max RSS (process gauge) | Max CPU% |",
        "|---------|------------------|-------------------------|----------|",
    ]
    if df.empty:
        return "\n".join(lines)

    agg = df.groupby("Name").agg(rss=("rss", "max"), cpu=("cpu", "max")).sort_index()
    docker_total = agg["rss"].sum()
    gauges = [process_rss_max.get(n) for n in agg.index]
    for (name, row), gauge in zip(agg.iterrows(), gauges):
        gauge_str = "n/a" if gauge is None else _hb(gauge)
        lines.append(f"| {name} | {_hb(row.rss)} | {gauge_str} | {row.cpu:.1f} |")
    present = [g for g in gauges if g is not None]
    gauge_total = _hb(sum(present)) if present else "n/a"
    lines.append(f"| **Total (sum)** | {_hb(docker_total)} | {gauge_total} | |")
    return "\n".join(lines)


def _series_mb(df: pd.DataFrame) -> dict[str, pd.DataFrame]:
    """Map name -> DataFrame[seconds, rss_mb] sorted by seconds (relative to min ts)."""
    if df.empty:
        return {}
    df = df.assign(seconds=df["ts"] - df["ts"].min(), rss_mb=df["rss"] / 1_000_000)
    return {
        n: g[["seconds", "rss_mb"]].sort_values("seconds").reset_index(drop=True)
        for n, g in df.groupby("Name")
    }


def _render_png(df: pd.DataFrame, label: str, out_path: Path) -> bool:
    series = _series_mb(df)
    if not series:
        return False
    fig, ax = plt.subplots(figsize=(12, 6), dpi=100)
    for i, name in enumerate(sorted(series)):
        s = series[name]
        color = PLOT_PALETTE[i % len(PLOT_PALETTE)]
        ax.plot(s["seconds"], s["rss_mb"], linewidth=1.2, color=color, label=name)
        ax.annotate(
            name,
            xy=(s["seconds"].iloc[-1], s["rss_mb"].iloc[-1]),
            xytext=(4, 0),
            textcoords="offset points",
            color=color,
            fontsize="x-small",
            va="center",
        )
    ax.legend(loc="upper left", fontsize="x-small", framealpha=0.85)
    ax.set(
        title=f"Memory soak: {label} (RSS over time)",
        xlabel="seconds",
        ylabel="RSS (MB)",
    )
    ax.grid(True, alpha=0.3)
    xmin, xmax = ax.get_xlim()
    ax.set_xlim(xmin, xmax + (xmax - xmin) * 0.08)
    fig.tight_layout()
    fig.savefig(out_path)
    plt.close(fig)
    return True


def _subsample(s: pd.DataFrame, n: int) -> pd.DataFrame:
    if len(s) <= n:
        return s
    idx = [int(i * len(s) / n) for i in range(n)]
    idx[-1] = len(s) - 1
    return s.iloc[idx].reset_index(drop=True)


def _render_mermaid(df: pd.DataFrame) -> str:
    series = _series_mb(df)
    if not series:
        return ""
    names = sorted(series)
    sub = {n: _subsample(series[n], MERMAID_MAX_POINTS) for n in names}
    max_x = max(int(s["seconds"].iloc[-1]) for s in sub.values())
    y_top = max(1.0, max(s["rss_mb"].max() for s in sub.values()) * 1.1)
    palette = ", ".join(PLOT_PALETTE[: len(names)])

    chart = [
        "```mermaid",
        f'%%{{init: {{"themeVariables": {{"xyChart": {{"plotColorPalette": "{palette}"}}}}}}}}%%',
        "xychart-beta",
        '    title "RSS over time (MB)"',
        f'    x-axis "seconds" 0 --> {max_x}',
        f'    y-axis "MB" 0 --> {y_top:.0f}',
        *(
            f'    line "{n}" [{", ".join(f"{y:.1f}" for y in sub[n]["rss_mb"])}]'
            for n in names
        ),
        "```",
        "",
        # xychart-beta has no native legend; render one as a flowchart whose
        # node fills come from the same palette so colors match line-for-line.
        "```mermaid",
        "flowchart LR",
    ]
    for i, name in enumerate(names):
        c = PLOT_PALETTE[i % len(PLOT_PALETTE)]
        chart += [
            f'    n{i}["{name}"]',
            f"    style n{i} fill:{c},color:#fff,stroke:{c}",
        ]
    chart.append("```")
    return "\n".join(chart)


def render_summary(
    docker_path: Path,
    metrics_path: Path,
    label: str,
    duration_seconds: int | None,
    output_dir: Path,
) -> str:
    if not docker_path.exists():
        raise FileNotFoundError(f"missing required file: {docker_path}")

    df = _load_docker(docker_path)
    n_samples = df.shape[0]
    if duration_seconds is None:
        duration_seconds = int(df["ts"].max() - df["ts"].min()) if not df.empty else 0

    header = f"## Memory soak: {label} ({duration_seconds}s, {n_samples} samples)"
    if df.empty:
        return f"{header}\n\n**No data collected.**\n"

    process_rss_max = _process_rss_max(metrics_path)

    parts = [header, "", _render_table(df, process_rss_max)]
    mermaid = _render_mermaid(df)
    if mermaid:
        parts.extend(["", "### Memory over time", "", mermaid])

    png_path = output_dir / "rss-over-time.png"
    try:
        wrote_png = _render_png(df, label, png_path)
    except Exception as e:
        log.warning(f"failed to render PNG chart: {e}")
        wrote_png = False
    if wrote_png:
        parts.extend(["", f"Full-resolution chart in artifact: `{png_path}`"])

    return "\n".join(parts) + "\n"


# ---------- CLI ----------

PathOpt = click.Path(path_type=Path)

_CTX = {
    "help_option_names": ["-h", "--help"],
    "show_default": True,
    "max_content_width": 100,
}


def opt(*args, **kw):
    """click.option with show_envvar=True baked in."""
    kw.setdefault("show_envvar", True)
    return click.option(*args, **kw)


@click.group(context_settings=_CTX)
@opt("--log-level", envvar="LOG_LEVEL", default="INFO")
def cli(log_level: str) -> None:
    """Sample container + node metrics, render a markdown summary."""
    logging.basicConfig(
        level=log_level,
        format="%(levelname)s %(name)s: %(message)s",
        stream=sys.stderr,
    )


@cli.command(context_settings=_CTX)
@opt("--duration-seconds", envvar="DURATION_SECONDS", default=300)
@opt(
    "--genesis-file",
    envvar="ESPRESSO_NODE_GENESIS_FILE",
    default="genesis/demo-drb-header.toml",
    help="set into env so .env interpolation works",
)
@opt(
    "--output-dir",
    envvar="OUTPUT_DIR",
    default=Path("./soak-samples"),
    type=PathOpt,
)
def sample(duration_seconds: int, genesis_file: str, output_dir: Path) -> None:
    """Scrape docker stats + each node's /v0/status/metrics into JSONL."""
    output_dir.mkdir(parents=True, exist_ok=True)
    os.environ.setdefault("ESPRESSO_NODE_GENESIS_FILE", genesis_file)
    os.environ.setdefault("ESPRESSO_SEQUENCER_GENESIS_FILE", genesis_file)

    env_path = REPO_ROOT / ".env"
    if not env_path.exists():
        log.error(".env not found. Copy .env.docker.example to .env first.")
        sys.exit(1)
    load_dotenv(env_path, override=False)

    run_sampling(output_dir, duration_seconds)


@cli.command()
@opt(
    "--output-dir",
    envvar="OUTPUT_DIR",
    default=Path("./soak-samples"),
    type=PathOpt,
    show_default=True,
    help="dir with docker-stats.jsonl + node-metrics.jsonl; also default output dir",
)
@opt(
    "--docker-stats",
    envvar="DOCKER_STATS_FILE",
    type=PathOpt,
    help="override input docker-stats.jsonl path",
)
@opt(
    "--node-metrics",
    envvar="NODE_METRICS_FILE",
    type=PathOpt,
    help="override input node-metrics.jsonl path",
)
@opt(
    "--out-dir",
    envvar="RENDER_OUT_DIR",
    type=PathOpt,
    help="override output dir for summary.md + rss-over-time.png",
)
@opt(
    "--label",
    envvar="GENESIS_LABEL",
    default=lambda: Path(os.environ.get("ESPRESSO_NODE_GENESIS_FILE", "soak")).stem,
    help="title label for the summary",
)
@opt(
    "--github-step-summary",
    envvar="GITHUB_STEP_SUMMARY",
    type=PathOpt,
    help="append summary to this file",
)
def render(
    output_dir: Path,
    docker_stats: Path | None,
    node_metrics: Path | None,
    out_dir: Path | None,
    label: str,
    github_step_summary: Path | None,
) -> None:
    """Render summary.md + rss-over-time.png + Mermaid chart from JSONL."""
    docker_path = docker_stats or output_dir / "docker-stats.jsonl"
    metrics_path = node_metrics or output_dir / "node-metrics.jsonl"
    out = out_dir or output_dir

    if not docker_path.exists():
        log.error(f"missing {docker_path}; run `soak.py sample` first")
        sys.exit(2)

    out.mkdir(parents=True, exist_ok=True)
    summary = render_summary(docker_path, metrics_path, label, None, out)
    sys.stdout.write(summary)

    summary_path = out / "summary.md"
    summary_path.write_text(summary)
    log.info(f"wrote {summary_path}")

    if github_step_summary is not None:
        with github_step_summary.open("a") as f:
            f.write(summary)
        log.info(f"appended summary to {github_step_summary}")


if __name__ == "__main__":
    cli()
