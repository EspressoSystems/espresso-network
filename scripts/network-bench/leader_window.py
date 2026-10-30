"""One leader's action window across views V-2..V+2 as a PNG: the leader's work spans, the node's
CPU load in cores busy, and the primary interface's network rate with the send and receive phases of consensus.

The CPU and network panels read the samples of `leadertrace.read_cpu` and `read_net`; without
samples (a run before the sampler existed) a panel says so and the figure still renders."""

import itertools
from collections import defaultdict
from pathlib import Path

import numpy as np
from leadertrace import NS_PER_MS, Cpu, Net, Trace
from matplotlib import patches
from matplotlib.axes import Axes
from matplotlib.figure import Figure
from matplotlib.lines import Line2D

Leaders = dict[int, tuple[int, int]]

CERT1 = "cert1_v_minus_1_input_dispatched"

# (start event, end event, label, cpu heavy, tier). Hatched spans burn cores and shade the CPU
# panel; the others move bytes. Tier 1 is drawn below tier 0.
SPANS = (
    ("request_block_header_queued", "header_created_applied", "block-build", True, 0),
    ("ns_disperse_start", "ns_disperse_end", "disperse", True, 0),
    ("vid_shares_unicast_start", "vid_shares_unicast_end", "fan-out", False, 0),
    (
        "recover_v_minus_1_start",
        "recover_v_minus_1_decode_end",
        "recover-decode",
        True,
        1,
    ),
    ("recover_v_minus_1_decode_end", "recover_v_minus_1_end", "txn-commit", True, 1),
)
# Drawn instead of the two spans above for a view without `ns_disperse_end` or
# `vid_shares_unicast_start`.
DISPERSE_FAN_OUT = (
    "ns_disperse_start",
    "vid_shares_unicast_end",
    "disperse + fan-out",
    True,
    0,
)
SPLIT_EVENTS = ("ns_disperse_end", "vid_shares_unicast_start")
MARKS = (("proposal_queued", "*", "proposal_queued"), ("leaf_decided", "D", "decided"))
# (start event, end event, label, direction); drawn on the network panel's TX and RX rows.
NET_SPANS = (
    ("vid_shares_unicast_start", "vid_shares_unicast_end", "fan-out", "tx"),
    ("proposal_broadcast_start", "proposal_broadcast_end", "proposal bcast", "tx"),
    ("vote1_broadcast_start", "vote1_broadcast_end", "vote1 bcast (+share)", "tx"),
    (
        "cert1_v_minus_1_broadcast_start",
        "cert1_v_minus_1_broadcast_end",
        "cert1 bcast",
        "tx",
    ),
    (
        "vote2_v_minus_1_broadcast_start",
        "vote2_v_minus_1_broadcast_end",
        "vote2 bcast",
        "tx",
    ),
    (
        "vote1_v_minus_1_arrived",
        CERT1,
        "vote1 collect",
        "rx",
    ),
)
SPAN_COLORS = {
    "block-build": "#6366f1",
    "disperse": "#2563eb",
    "disperse + fan-out": "#2563eb",
    "fan-out": "#ea580c",
    "recover-decode": "#10b981",
    "txn-commit": "#d97706",
}
OFF_WINDOW_COLOR = "#a3a3a3"
CERT1_COLOR = "#ea580c"
CPU_SMOOTH_MS = 50
NET_BIN_MS = 50
TX_COLOR, RX_COLOR = "#dc2626", "#1d4ed8"


def write_leader_window(
    trace: Trace, lead: Leaders, view: int, cpu: Cpu, net: Net, path: Path
) -> None:
    """`view` must have a leader in `lead`; `cpu` and `net` are the samples of that leader's
    node, empty when it has none."""
    node = lead[view][0]
    events = trace[node]
    stamps = [
        ts for v in range(view - 2, view + 3) for ts in events.get(v, {}).values()
    ]
    if not stamps:
        raise ValueError(f"no events of views {view - 2}..{view + 2} on node {node}")
    t_start, t_end = min(stamps) - 50 * NS_PER_MS, max(stamps) + 100 * NS_PER_MS
    window = Window(t_start, t_end)

    fig = Figure(figsize=(15, 10))
    outer = fig.add_gridspec(3, 1, height_ratios=[2, 3, 3.0], hspace=0.30)
    top = fig.add_subplot(outer[0])
    cpu_ax = fig.add_subplot(outer[1], sharex=top)
    net_grid = outer[2].subgridspec(2, 1, height_ratios=[1.0, 2.0], hspace=0.05)
    net_events_ax = fig.add_subplot(net_grid[0], sharex=top)
    net_ax = fig.add_subplot(net_grid[1], sharex=top)

    colors = {
        view - 2: "#888",
        view - 1: "#3a7",
        view: "#d33",
        view + 1: "#37c",
        view + 2: "#888",
    }
    in_window = sorted(
        v
        for v, evs in events.items()
        if any(t_start <= ts <= t_end for ts in evs.values())
    )
    cpu_columns = draw_work(top, cpu_ax, events, in_window, colors, window)
    net_columns = net_spans(events, in_window, colors, window)
    draw_view_boundaries(top, cpu_ax, trace, lead, view, colors, window)
    top.set_title(
        f"Leader (node {node}) action window across views {view - 2}..{view + 2}"
        f", duty at V={view}",
        fontsize=11,
    )
    draw_cpu(cpu_ax, cpu, window)
    draw_net(net_events_ax, net_ax, node, net, net_columns, window)
    for x0, x1 in cpu_columns:
        top.axvspan(x0, x1, color="#fee2e2", alpha=0.35, zorder=1)
        cpu_ax.axvspan(x0, x1, color="#fee2e2", alpha=0.40, zorder=0)
    top.set_xlim(0, window.span_ms)
    fig.savefig(path, dpi=100)


class Window:
    def __init__(self, t_start: int, t_end: int) -> None:
        self.t_start, self.t_end = t_start, t_end
        self.span_ms = (t_end - t_start) / NS_PER_MS

    def ms(self, ts: int) -> float:
        return (ts - self.t_start) / NS_PER_MS

    def contains(self, ts: int) -> bool:
        return self.t_start <= ts <= self.t_end

    def clipped(self, ts_a: int, ts_b: int) -> tuple[float, float] | None:
        """(x, width) in ms of the span clipped to the window; None when empty."""
        if ts_b < self.t_start or ts_a > self.t_end:
            return None
        x = self.ms(max(ts_a, self.t_start))
        width = self.ms(min(ts_b, self.t_end)) - x
        return (x, width) if width > 0 else None


def view_spans(evs: dict[str, int]) -> tuple:
    """`SPANS` of one view; `DISPERSE_FAN_OUT` replaces the disperse and fan-out spans when the
    view lacks the events that split them."""
    if all(event in evs for event in SPLIT_EVENTS):
        return SPANS
    return (SPANS[0], DISPERSE_FAN_OUT, *SPANS[3:])


def draw_work(
    ax: Axes,
    cpu_ax: Axes,
    events: dict[int, dict[str, int]],
    in_window: list[int],
    colors: dict[int, str],
    window: Window,
) -> list[tuple[float, float]]:
    """The work spans and marks of the leader's events; returns the x ranges of CPU-heavy spans."""
    ax.axhspan(-0.5, 0.5, color="#fef2f2", zorder=0)
    cpu_columns = []
    used = set()
    for view in in_window:
        evs = events[view]
        color = colors.get(view, OFF_WINDOW_COLOR)
        for start, end, label, cpu, tier in view_spans(evs):
            if start not in evs or end not in evs:
                continue
            clipped = window.clipped(evs[start], evs[end])
            if clipped is None:
                continue
            x, width = clipped
            y_center, height = -0.20 - tier * 0.36, 0.30
            ax.add_patch(
                patches.FancyBboxPatch(
                    (x, y_center - height / 2),
                    width,
                    height,
                    boxstyle="round,pad=0,rounding_size=0.02",
                    facecolor=SPAN_COLORS[label],
                    edgecolor="#1f2937",
                    linewidth=0.6,
                    hatch="////" if cpu else None,
                    alpha=0.9,
                    zorder=2,
                )
            )
            used.add(label)
            if width >= 5.0 * len(label):
                ax.text(
                    x + width / 2,
                    y_center,
                    label,
                    ha="center",
                    va="center",
                    fontsize=8,
                    fontweight="bold",
                    color="white",
                    zorder=3,
                )
            if cpu:
                cpu_columns.append((x, x + width))
        for event, marker, _ in MARKS:
            if event in evs and window.contains(evs[event]):
                ax.scatter(
                    window.ms(evs[event]),
                    0.38,
                    marker=marker,
                    s=46,
                    color=color,
                    edgecolor="black",
                    linewidths=0.5,
                    zorder=4,
                )
    for y in (-0.5, 0.5):
        ax.axhline(y, color="#d1d5db", linewidth=0.7, alpha=0.8, zorder=1)
    ax.set_yticks([0], ["Leader work"], fontsize=11, fontweight="bold")
    ax.set_ylim(0.65, -0.85)
    ax.tick_params(axis="x", labelbottom=False)
    ax.grid(axis="x", alpha=0.3)
    draw_work_legend(ax, used)
    return cpu_columns


def draw_work_legend(ax: Axes, used: set[str]) -> None:
    operations = [
        patches.Patch(facecolor=color, edgecolor="#1f2937", linewidth=0.6, label=label)
        for label, color in SPAN_COLORS.items()
        if label in used
    ]
    marks = [
        Line2D(
            [0],
            [0],
            marker=marker,
            color="w",
            markerfacecolor="#666",
            markeredgecolor="black",
            markersize=7,
            label=label,
            linestyle="",
        )
        for _, marker, label in MARKS
    ]
    boundaries = [
        Line2D(
            [0],
            [0],
            color="#555",
            linestyle="-",
            linewidth=1.4,
            label="proposal_queued | view start",
        ),
        Line2D(
            [0],
            [0],
            color=CERT1_COLOR,
            linestyle=(0, (4, 3)),
            linewidth=1.1,
            label="cert1[V-1] formed",
        ),
    ]
    ax.legend(
        handles=[*operations, *marks, *boundaries],
        loc="lower center",
        bbox_to_anchor=(0.5, -0.18),
        fontsize=7.5,
        ncol=6,
        framealpha=0.92,
        columnspacing=1.5,
        handletextpad=0.5,
        borderpad=0.4,
    )


def net_spans(
    events: dict[int, dict[str, int]],
    in_window: list[int],
    colors: dict[int, str],
    window: Window,
) -> list[tuple[float, float, str, str, str]]:
    """(x0, x1, label, color, direction) of the network phases in the window."""
    columns = []
    for view in in_window:
        evs = events[view]
        for start, end, label, direction in NET_SPANS:
            if start not in evs or end not in evs:
                continue
            clipped = window.clipped(evs[start], evs[end])
            if clipped is not None:
                x, width = clipped
                color = colors.get(view, OFF_WINDOW_COLOR)
                columns.append((x, x + width, label, color, direction))
    return columns


def draw_view_boundaries(
    ax: Axes,
    cpu_ax: Axes,
    trace: Trace,
    lead: Leaders,
    view: int,
    colors: dict[int, str],
    window: Window,
) -> None:
    """A line at each proposal_queued in the window with its view id and the cycle since the
    previous view's, and a dashed line where each Cert1 formed at the next leader."""
    boundaries = sorted((ts, v) for v, (_, ts) in lead.items() if window.contains(ts))
    for ts, v in boundaries:
        color = colors.get(v, OFF_WINDOW_COLOR)
        ax.axvline(window.ms(ts), color=color, alpha=0.65, linewidth=1.4, zorder=5)
        cpu_ax.axvline(window.ms(ts), color=color, alpha=0.35, linewidth=1.0, zorder=4)
        ax.text(
            window.ms(ts) + 3,
            -0.55,
            f"V={v}" if v == view else f"{v}",
            color=color,
            fontsize=8.5,
            fontweight="bold" if v == view else "normal",
            va="bottom",
            ha="left",
            zorder=6,
        )
    for (ts_a, _), (ts_b, v) in itertools.pairwise(boundaries):
        if v - 1 not in lead:
            continue
        ax.annotate(
            "",
            xy=(window.ms(ts_b), -0.40),
            xytext=(window.ms(ts_a), -0.40),
            arrowprops={
                "arrowstyle": "<->",
                "color": "#555",
                "lw": 0.9,
                "shrinkA": 0,
                "shrinkB": 0,
            },
            zorder=5,
        )
        ax.text(
            (window.ms(ts_a) + window.ms(ts_b)) / 2,
            -0.43,
            f"{(ts_b - lead[v - 1][1]) / NS_PER_MS:.0f} ms",
            fontsize=7.5,
            color="#222",
            ha="center",
            va="bottom",
            bbox={"facecolor": "white", "edgecolor": "none", "alpha": 0.85, "pad": 0.5},
            zorder=6,
        )
    for ts in cert1_stamps(trace, lead, view):
        if window.contains(ts):
            ax.axvline(
                window.ms(ts),
                color=CERT1_COLOR,
                alpha=0.7,
                linestyle=(0, (4, 3)),
                linewidth=1.1,
                zorder=5,
            )
            cpu_ax.axvline(
                window.ms(ts),
                color=CERT1_COLOR,
                alpha=0.40,
                linestyle=(0, (4, 3)),
                linewidth=0.9,
                zorder=4,
            )


def cert1_stamps(trace: Trace, lead: Leaders, view: int) -> list[int]:
    """When Cert1(v) formed at L_{v+1}, for v in view-2..view+1. Rows are keyed by subject view,
    so it is row v of L_{v+1}'s trace: the parent cert that gates proposal v+1."""
    return [
        ts
        for v in range(view - 2, view + 2)
        if v + 1 in lead
        and (ts := trace[lead[v + 1][0]].get(v, {}).get(CERT1)) is not None
    ]


def cpu_series(cpu: Cpu, window: Window) -> tuple[np.ndarray, np.ndarray] | None:
    """(ms in window, cores busy) per interval ending in the window; None without one."""
    rows = np.array([row for row in cpu if window.contains(row[0])], dtype=np.int64)
    if len(rows) == 0:
        return None
    busy = rows[:, 2] * 1000.0 / rows[:, 1]
    return (rows[:, 0] - window.t_start) / NS_PER_MS, busy


def draw_cpu(ax: Axes, cpu: Cpu, window: Window) -> None:
    ax.tick_params(axis="x", labelbottom=False)
    ax.set_ylabel("CPU (cores busy)")
    series = cpu_series(cpu, window)
    if series is None:
        ax.text(
            0.5,
            0.5,
            "no cpu_node{N}.csv samples in the window",
            ha="center",
            va="center",
            color="#94a3b8",
            transform=ax.transAxes,
        )
        ax.set_title("process CPU load, no data", fontsize=10)
        return
    xs, busy = series
    ax.plot(
        xs,
        busy,
        color="#3b82f6",
        linewidth=0.7,
        alpha=0.30,
        drawstyle="steps-post",
        label="process CPU (cores)",
    )
    k = (
        max(1, round(CPU_SMOOTH_MS / float(np.median(np.diff(xs)))))
        if len(xs) > 1
        else 1
    )
    if 1 < k <= len(busy):
        ax.plot(
            xs,
            np.convolve(busy, np.ones(k) / k, mode="same"),
            color="#1d4ed8",
            linewidth=2.0,
            label=f"{CPU_SMOOTH_MS} ms avg",
        )
    ax.axhline(1.0, color="black", linestyle=":", linewidth=0.8, alpha=0.4)
    ax.set_ylim(0, max(1.1, float(busy.max()) * 1.1))
    ax.grid(axis="y", alpha=0.3)
    ax.set_title(
        "process CPU load (cores busy), cgroup usage_usec of the espresso-node container",
        fontsize=10,
    )
    ax.legend(loc="upper right", fontsize=8, framealpha=0.9)


def net_series(
    net: Net, window: Window
) -> tuple[np.ndarray, np.ndarray, np.ndarray] | None:
    """(ms in window, tx Gbps, rx Gbps) per interval ending in the window; None without one."""
    rows = np.array([row for row in net if window.contains(row[0])], dtype=np.int64)
    if len(rows) == 0:
        return None
    return (
        (rows[:, 0] - window.t_start) / NS_PER_MS,
        rows[:, 3] * 8.0 / rows[:, 1],
        rows[:, 2] * 8.0 / rows[:, 1],
    )


def net_binned(
    net: Net, window: Window
) -> tuple[np.ndarray, np.ndarray, np.ndarray] | None:
    """(ms of bin centers, tx Gbps, rx Gbps) over the whole NET_BIN_MS bins of the window, from
    the intervals ending in each bin."""
    bin_ns = NET_BIN_MS * NS_PER_MS
    bins = (window.t_end - window.t_start) // bin_ns
    tx, rx = np.zeros(bins), np.zeros(bins)
    for t_ns, _, rx_bytes, tx_bytes in net:
        index = (t_ns - window.t_start) // bin_ns
        if 0 <= index < bins:
            tx[index] += tx_bytes * 8.0 / bin_ns
            rx[index] += rx_bytes * 8.0 / bin_ns
    if not bins:
        return None
    return (np.arange(bins) + 0.5) * NET_BIN_MS, tx, rx


def draw_net(
    events_ax: Axes,
    ax: Axes,
    node: int,
    net: Net,
    columns: list[tuple[float, float, str, str, str]],
    window: Window,
) -> None:
    """`events_ax`: TX and RX phase bars above `ax`, which plots the rates in Gbps."""
    series = net_series(net, window)
    draw_net_events(events_ax, node, columns, window, series is not None)
    min_visible_ms = max(1.0, window.span_ms * 0.005)
    for x0, x1, _, color, _ in columns:
        ax.axvspan(
            x0, x0 + max(x1 - x0, min_visible_ms), color=color, alpha=0.18, zorder=0
        )
    peak = 0.0
    if series is None:
        message = (
            "no net samples in the window" if net else "no net_node{N}.csv samples"
        )
        ax.text(
            0.5,
            0.5,
            message,
            ha="center",
            va="center",
            fontsize=10,
            color="#888",
            transform=ax.transAxes,
        )
    else:
        xs, tx, rx = series
        ax.plot(xs, tx, color=TX_COLOR, linewidth=0.9, alpha=0.45, label="tx (raw)")
        ax.plot(xs, rx, color=RX_COLOR, linewidth=0.9, alpha=0.45, label="rx (raw)")
        peak = max(float(tx.max()), float(rx.max()))
        binned = net_binned(net, window)
        if binned is not None:
            bx, btx, brx = binned
            ax.plot(bx, btx, color=TX_COLOR, linewidth=1.6, label=f"tx {NET_BIN_MS} ms")
            ax.plot(bx, brx, color=RX_COLOR, linewidth=1.6, label=f"rx {NET_BIN_MS} ms")
        ax.legend(loc="upper right", fontsize=7.5, framealpha=0.9)
    ax.set_ylim(0, max(peak * 1.1, 1.2))
    ax.set_ylabel("network\nGbps")
    ax.set_xlabel("ms since the window start")
    ax.grid(axis="y", alpha=0.3)


def draw_net_events(
    ax: Axes,
    node: int,
    columns: list[tuple[float, float, str, str, str]],
    window: Window,
    have_samples: bool,
) -> None:
    """TX bars (labels above) over RX bars (labels inside), in axes-fraction rows."""
    tx_label_y = 0.78
    rows = {"tx": (0.45, 0.22), "rx": (0.08, 0.22)}
    ax.set_yticks([])
    for spine in ("top", "right", "left", "bottom"):
        ax.spines[spine].set_visible(False)
    ax.tick_params(axis="x", labelbottom=False, length=0)
    ax.set_title(
        f"network utilization on node {node} (tx/rx)"
        f"{'' if have_samples else ', no net samples'}",
        fontsize=10,
    )
    for direction, (bar_y, bar_h) in rows.items():
        ax.text(
            -0.008,
            bar_y + bar_h / 2,
            direction.upper(),
            ha="right",
            va="center",
            fontsize=8,
            fontweight="bold",
            color="#444",
            transform=ax.transAxes,
        )
    min_visible_ms = max(1.0, window.span_ms * 0.005)
    min_label_gap_ms = max(20.0, window.span_ms * 0.04)
    placed: dict[str, list[float]] = defaultdict(list)
    for x0, x1, label, color, direction in columns:
        bar_y, bar_h = rows[direction]
        width = max(x1 - x0, min_visible_ms)
        ax.add_patch(
            patches.Rectangle(
                (x0, bar_y),
                width,
                bar_h,
                facecolor=color,
                edgecolor="#333",
                linewidth=0.6,
                alpha=0.95,
                zorder=3,
                transform=ax.get_xaxis_transform(),
            )
        )
        center = x0 + width / 2
        if any(abs(center - x) <= min_label_gap_ms for x in placed[direction]):
            continue
        placed[direction].append(center)
        if direction == "tx":
            ax.annotate(
                label,
                xy=(center, bar_y + bar_h),
                xytext=(center, tx_label_y),
                xycoords=ax.get_xaxis_transform(),
                textcoords=ax.get_xaxis_transform(),
                ha="center",
                va="bottom",
                fontsize=7,
                color="#111",
                zorder=4,
                arrowprops={
                    "arrowstyle": "-",
                    "color": "#666",
                    "lw": 0.5,
                    "shrinkA": 0,
                    "shrinkB": 1,
                },
            )
        else:
            ax.text(
                center,
                bar_y + bar_h / 2,
                label,
                ha="center",
                va="center",
                fontsize=7,
                color="#111",
                zorder=4,
                transform=ax.get_xaxis_transform(),
            )
