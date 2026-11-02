"""Block finalization timeline of the leader traces as one SVG: the median view, anchored at the
leader's `proposal_queued` (t = 0), with the next leader's pipeline one cycle later.

Every metric is a median over the given views. A metric whose events the traces lack is 0 in the
arithmetic, so its bars or markers collapse, and `present_<metric>` is 0; a leader build bar with
an absent endpoint is not drawn. The leader build metrics are medians over the views that have
every event of the layout the majority of views supports: split (`ns_disperse_end` and
`vid_shares_unicast_start` present) or one "disperse + fan-out" bar, with or without the
`header_created_applied` and `leaf2_commit_computed` tail (header, parent Cert1 wait, commit +
sign).

Each leader build lane has three sub-rows: the sequential path (build/prep, header, parent Cert1
wait, commit + sign), dispersal and fan-out. Dispersal and fan-out overlap in time, so
they are drawn in parallel rows; share-sign shows in the fan-out row only when it is positive.
A label that does not fit its bar goes beside, above or below it where the lane has room, else it
is centered and overflows the bar. Bars shorter than 1 ms are not drawn."""

import math
import statistics
from dataclasses import dataclass
from pathlib import Path

from leadertrace import DECIDED, NS_PER_MS, Trace, leaders

# metric -> (event, where). "leader": L_V's own stamp of view V; "cluster": the median stamp over
# all nodes at view V; "leader_prev": L_V's stamp at view V - 1.
EVENTS = {
    "request_block": ("request_block_header_queued", "leader"),
    "block_built": ("block_built_applied", "leader"),
    "header_created": ("header_created_applied", "leader"),
    "leaf2_commit": ("leaf2_commit_computed", "leader"),
    "ns_disperse_start": ("ns_disperse_start", "leader"),
    "vid_disperse": ("ns_disperse_end", "leader"),
    "vid_unicast_start": ("vid_shares_unicast_start", "leader"),
    "vid_unicast_end": ("vid_shares_unicast_end", "leader"),
    "proposal_validated": ("proposal_validated_v_minus_1", "cluster"),
    "vote1_arrived": ("vote1_v_minus_1_arrived", "cluster"),
    "vid_threshold_reached": ("threshold_share_reached_v_minus_1", "cluster"),
    "recover_decode_end": ("recover_v_minus_1_decode_end", "cluster"),
    "block_reconstructed": ("recover_v_minus_1_end", "cluster"),
    "cert1_formed": ("cert1_v_minus_1_input_dispatched", "cluster"),
    "vote2_sent": ("vote2_v_minus_1_queued", "cluster"),
    "cert2_formed": ("cert2_v_minus_1_input_dispatched", "cluster"),
    "leaf_decided": (DECIDED, "cluster"),
    "cert1_at_L_V": ("cert1_v_minus_1_input_dispatched", "leader_prev"),
    "leader_recover_start": ("recover_v_minus_1_start", "leader_prev"),
    "leader_recover_decode_end": ("recover_v_minus_1_decode_end", "leader_prev"),
    "verify_at_L_V": ("recover_v_minus_1_end", "leader_prev"),
}

Metrics = dict[str, float]
# Leader build events by group: always drawn, the dispersal/fan-out split, the header tail.
BUILD_EVENTS = ("request_block", "block_built", "ns_disperse_start", "vid_unicast_end")
SPLIT_EVENTS = ("vid_disperse", "vid_unicast_start")
TAIL_EVENTS = ("header_created", "leaf2_commit")
BUILD_GROUP_EVENTS = BUILD_EVENTS + SPLIT_EVENTS + TAIL_EVENTS
ViewRow = dict[str, float | None]

TITLE = "Block V finalization timeline"
BUILD_PREP, DISPERSAL, SHARE_SIGN, FAN_OUT, HEADER = (
    "#94a3b8",
    "#2563eb",
    "#9333ea",
    "#ea580c",
    "#64748b",
)
CERT1_WAIT, COMMIT_SIGN = "#e11d48", "#0f766e"
SHARE_WAIT, RECOVER, COMMIT, COLLECT = "#bbf7d0", "#10b981", "#d97706", "#8b5cf6"
VOTE1_CERT1, CERT1_VOTE2, VOTE2_CERT2, CERT2_DECIDED = (
    "#c4b5fd",
    "#fde68a",
    "#a78bfa",
    "#a7f3d0",
)


def write_timeline(trace: Trace, views: list[int], path: Path) -> None:
    metrics = timeline_metrics(view_rows(trace, views))
    path.write_text(timeline_svg(metrics), encoding="utf-8")


def view_rows(trace: Trace, views: list[int]) -> list[ViewRow]:
    """Per view, `cycle` (ms from the previous leader's proposal_queued) and every `EVENTS`
    metric in ms relative to this view's proposal_queued, None when absent. Cluster metrics are
    the median stamp across the nodes that have the event."""
    lead = leaders(trace)
    return [view_row(trace, lead, view) for view in views]


def view_row(trace: Trace, lead: dict[int, tuple[int, int]], view: int) -> ViewRow:
    leader, queued = lead[view]

    def stamp(node: int, subject: int, event: str) -> int | None:
        return trace[node].get(subject, {}).get(event)

    def rel(ts: int | None) -> float | None:
        return None if ts is None else (ts - queued) / NS_PER_MS

    def metric(event: str, where: str) -> float | None:
        if where == "leader":
            return rel(stamp(leader, view, event))
        if where == "leader_prev":
            return rel(stamp(leader, view - 1, event))
        stamps = [ts for node in trace if (ts := stamp(node, view, event)) is not None]
        return rel(statistics.median_low(stamps)) if stamps else None

    return {"cycle": (queued - lead[view - 1][1]) / NS_PER_MS} | {
        name: metric(*spec) for name, spec in EVENTS.items()
    }


def timeline_metrics(rows: list[ViewRow]) -> Metrics:
    """Medians over the rows plus the durations the SVG labels. Missing values are 0, with
    `present_<name>` telling them from a median of 0."""
    if not rows:
        raise ValueError("no views for the timeline")
    layout = build_layout(rows)
    complete = [row for row in rows if all(row[name] is not None for name in layout)]

    def median(name: str) -> float | None:
        if name in BUILD_GROUP_EVENTS:
            source = complete if name in layout else []
        else:
            source = rows
        values = [v for row in source if (v := row[name]) is not None]
        return statistics.median(values) if values else None

    medians = {name: median(name) for name in ("cycle", *EVENTS)}
    m = {"n_views": float(len(rows)), "proposal_queued": 0.0}
    m |= {name: value or 0.0 for name, value in medians.items()}
    m |= {
        f"present_{name}": float(value is not None) for name, value in medians.items()
    }
    m["present_proposal_queued"] = 1.0
    m["share_wait_ms"] = max(0.0, m["vid_threshold_reached"] - m["proposal_validated"])
    if m["recover_decode_end"]:
        m["recover_decode_ms"] = max(
            0.0, m["recover_decode_end"] - m["vid_threshold_reached"]
        )
        m["recover_commit_ms"] = max(
            0.0, m["block_reconstructed"] - m["recover_decode_end"]
        )
    else:
        m["recover_decode_ms"] = max(
            0.0, m["block_reconstructed"] - m["vid_threshold_reached"]
        )
        m["recover_commit_ms"] = 0.0
    m["recover_cpu_ms"] = m["recover_decode_ms"] + m["recover_commit_ms"]
    m["cert1_wait_ms"] = max(0.0, m["cert1_formed"] - m["vote1_arrived"])
    m["cert2_wait_ms"] = max(0.0, m["cert2_formed"] - m["vote2_sent"])
    m["finality_ms"] = m["leaf_decided"]
    m["throughput_vps"] = 1000.0 / m["cycle"] if m["cycle"] > 0 else 0.0
    return m


def build_layout(rows: list[ViewRow]) -> tuple[str, ...]:
    """The richest set of leader build events that at least half of the views have in full."""
    for extra in (SPLIT_EVENTS + TAIL_EVENTS, SPLIT_EVENTS, TAIL_EVENTS, ()):
        events = BUILD_EVENTS + extra
        have = sum(all(row[name] is not None for name in events) for row in rows)
        if have and have * 2 >= len(rows):
            return events
    return BUILD_EVENTS


def build_segments(m: Metrics) -> list[tuple[int, str, str, str, str]]:
    """(row, label, start metric, end metric, color) of the leader's build whose endpoints are
    both present. Row 0 is the sequential path, row 1 dispersal, row 2 fan-out."""
    if m["present_vid_disperse"] and m["present_vid_unicast_start"]:
        middle = [
            (1, "dispersal", "ns_disperse_start", "vid_disperse", DISPERSAL),
            (2, "share-sign", "vid_disperse", "vid_unicast_start", SHARE_SIGN),
            (2, "fan-out", "vid_unicast_start", "vid_unicast_end", FAN_OUT),
        ]
    else:
        middle = [
            (1, "disperse + fan-out", "ns_disperse_start", "vid_unicast_end", DISPERSAL)
        ]
    segments = [
        (0, "build/prep", "request_block", "block_built", BUILD_PREP),
        *middle,
        (0, "header", "block_built", "header_created", HEADER),
        (0, "parent Cert1 wait", "header_created", "leaf2_commit", CERT1_WAIT),
        (0, "commit + sign", "leaf2_commit", "proposal_queued", COMMIT_SIGN),
    ]
    return [
        segment
        for segment in segments
        if m[f"present_{segment[2]}"] and m[f"present_{segment[3]}"]
    ]


@dataclass(frozen=True)
class Frame:
    """Time axis and lane geometry of the SVG."""

    t_min: int
    t_max: int
    width: int = 1700
    height: int = 870
    x_left: int = 340
    x_right: int = 1680
    plot_top: int = 100
    lane_h: int = 68
    bar_h: int = 26
    # The leader build lanes hold three stacked rows of thin bars below a label strip.
    build_lanes: tuple[str, ...] = ("L_V", "NextCPU")
    build_lane_h: int = 100
    label_h: int = 26
    row_h: int = 20
    row_pitch: int = 24
    lanes: tuple[str, ...] = (
        "L_V",
        "NextRX",
        "NextRecon",
        "NextCPU",
        "Recon",
        "Deciding",
    )

    @classmethod
    def of(cls, m: Metrics) -> "Frame":
        t_left = min(0, m["request_block"]) - 30
        t_max = int((max(m["finality_ms"], m["cycle"]) + 80 + 49) // 50) * 50
        return cls(t_min=int(t_left // 50) * 50, t_max=t_max)

    def lane_height(self, lane: str) -> int:
        return self.build_lane_h if lane in self.build_lanes else self.lane_h

    @property
    def plot_bottom(self) -> int:
        return self.plot_top + sum(self.lane_height(lane) for lane in self.lanes)

    def x(self, t: float) -> float:
        span = self.t_max - self.t_min
        if span <= 0:
            return self.x_left
        return self.x_left + (t - self.t_min) * (self.x_right - self.x_left) / span

    def w(self, t1: float, t2: float) -> float:
        return max(0, self.x(t2) - self.x(t1))

    def top(self, lane: str) -> int:
        above = self.lanes[: self.lanes.index(lane)]
        return self.plot_top + sum(self.lane_height(other) for other in above)

    def bar_height(self, lane: str) -> int:
        return self.row_h if lane in self.build_lanes else self.bar_h

    def bar_y(self, lane: str, row: int = 0) -> int:
        if lane in self.build_lanes:
            return self.top(lane) + self.label_h + row * self.row_pitch
        return self.top(lane) + (self.lane_h - self.bar_h) // 2 + 1

    def cy(self, lane: str, row: int = 0) -> int:
        return self.bar_y(lane, row) + self.bar_height(lane) // 2

    def mid(self, lane: str) -> int:
        return self.top(lane) + self.lane_height(lane) // 2

    def ticks(self, step: int = 100) -> list[int]:
        first = int(self.t_min // step) * step
        return [t for t in range(first, self.t_max + 1, step) if t >= self.t_min]


def timeline_svg(m: Metrics) -> str:
    f = Frame.of(m)
    parts = [
        svg_header(m, f),
        cycle_gate(m, f),
        lane_backgrounds(f),
        time_axis(m, f),
        lane_labels(f),
        leader_lane(m, f),
        next_vote1_lane(m, f),
        next_recover_lane(m, f),
        next_build_lane(m, f),
        recon_lane(m, f),
        deciding_lane(m, f),
        arrows(m, f),
        brackets(m, f),
        legend(f),
        "</svg>\n",
    ]
    return "".join(parts)


def rect(x: float, y: float, w: float, h: float, fill: str, rx: int = 3) -> str:
    return f'  <rect x="{x:.1f}" y="{y}" width="{w:.1f}" height="{h}" fill="{fill}" rx="{rx}"/>\n'


def text(
    x: float,
    y: float,
    body: str,
    size: int,
    fill: str,
    weight: int | None = None,
    anchor: str = "middle",
) -> str:
    bold = f' font-weight="{weight}"' if weight else ""
    return f'  <text x="{x:.1f}" y="{y}" text-anchor="{anchor}" font-size="{size}"{bold} fill="{fill}">{body}</text>\n'


def dot(x: float, cy: float, r: int, fill: str) -> str:
    stroke = 2 if r >= 6 else 1.5
    return f'  <circle cx="{x:.1f}" cy="{cy}" r="{r}" fill="{fill}" stroke="white" stroke-width="{stroke}"/>\n'


def line(x1: float, y1: float, x2: float, y2: float, stroke: str, width: float) -> str:
    return f'  <line x1="{x1:.1f}" y1="{y1}" x2="{x2:.1f}" y2="{y2}" stroke="{stroke}" stroke-width="{width}"/>\n'


def path(d: str, stroke: str, width: float, marker: str, dashed: bool = False) -> str:
    dash = ' stroke-dasharray="3 3"' if dashed else ""
    return f'  <path d="{d}" stroke="{stroke}" stroke-width="{width}"{dash} fill="none" marker-end="url(#{marker})"/>\n'


def label_width(body: str, size: int) -> float:
    return 0.6 * size * len(body)


def fits(f: Frame, t0: float, t1: float, body: str, size: int) -> bool:
    return f.w(t0, t1) >= label_width(body, size) + 8


def bar(
    f: Frame,
    t0: float,
    t1: float,
    y: int,
    h: int,
    color: str,
    body: str,
    size: int,
    fill: str = "white",
    outside: str | None = None,
    placed: list[tuple[float, float, int]] | None = None,
) -> str:
    """A bar with its label centered inside. A label that does not fit goes to the `outside`
    side ("right"; "above", starting at the bar start; "below", ending at the bar end), else it
    stays centered and overflows the bar. An outside label that would overlap one already in
    `placed` (x start, x end, baseline) is dropped."""
    out = rect(f.x(t0), y, f.w(t0, t1), h, color)
    x0, x1 = f.x(t0), f.x(t1)
    middle, base = (x0 + x1) / 2, y + h // 2 + 4
    if fits(f, t0, t1, body, size) or outside is None:
        return out + text(middle, base, body, size, fill, 600)
    x, base, anchor = {
        "right": (x1 + 4, base, "start"),
        "above": (x0, y - 4, "start"),
        "below": (x1, y + h + 12, "end"),
    }[outside]
    width = label_width(body, size)
    span = (x, x + width) if anchor == "start" else (x - width, x)
    if placed is not None:
        if any(s < span[1] and span[0] < e and b == base for s, e, b in placed):
            return out
        placed.append((*span, base))
    return out + text(x, base, body, size, "#374151", 600, anchor)


def svg_header(m: Metrics, f: Frame) -> str:
    subtitle = (
        f"{m['n_views']:.0f} views, t = 0 anchored at proposal_queued(V), "
        "all values medians from the leader traces"
    )
    summary = (
        f"per-view cycle = {m['cycle']:.0f} ms · finality of V ≈ {m['finality_ms']:.0f} ms"
        f" · throughput ≈ {m['throughput_vps']:.2f} v/s"
    )
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {f.width} {f.height}"
     font-family="ui-sans-serif, -apple-system, system-ui, 'Segoe UI', Roboto, sans-serif">
  <defs>
    <marker id="critArrow" viewBox="0 0 10 10" refX="9" refY="5"
            markerWidth="9" markerHeight="9" orient="auto">
      <path d="M0,0 L10,5 L0,10 Z" fill="#dc2626"/>
    </marker>
    <marker id="grayArrow" viewBox="0 0 10 10" refX="9" refY="5"
            markerWidth="8" markerHeight="8" orient="auto">
      <path d="M0,0 L10,5 L0,10 Z" fill="#9ca3af"/>
    </marker>
  </defs>

""" + (
        text(f.width // 2, 32, TITLE, 20, "#111827", 700)
        + text(f.width // 2, 54, subtitle, 12, "#6b7280")
        + text(f.width // 2, 72, summary, 12, "#6b7280")
    )


def cycle_gate(m: Metrics, f: Frame) -> str:
    """L_{V+1} proposes once Cert1(V) has formed at it; the bar is the time from proposal_queued(V)
    to that Cert1 as a share of the cycle, capped at the full width."""
    gate_y, height = 86, 10
    cert1_ms = m["cert1_at_L_V"] + m["cycle"]
    pct = cert1_ms / m["cycle"] if m["cycle"] > 0 else 0
    width = (f.x_right - f.x_left - 280) * min(pct, 1)
    return (
        text(
            f.x_left,
            gate_y - 3,
            "Cycle gate: Cert1(V) at L_{V+1} gates proposal_queued(V+1)",
            11,
            "#374151",
            600,
            "start",
        )
        + rect(f.x_left, gate_y, width, height, COLLECT, rx=0)
        + text(
            f.x_left + width / 2,
            gate_y + height - 1,
            f"proposal_queued(V) to Cert1(V) at L_{{V+1}} = {cert1_ms:.0f} ms ({pct * 100:.0f}% of cycle)",
            9,
            "white",
            600,
        )
    )


def lane_backgrounds(f: Frame) -> str:
    # The three L_{V+1} lanes drive the cycle and are tinted yellow.
    fills = ["#eef2ff", "#fef3c7", "#fef3c7", "#fef3c7", "#f9fafb", "#ecfdf5"]
    out = "".join(
        f'  <rect x="{f.x_left}" y="{f.top(lane)}" width="{f.x_right - f.x_left}" '
        f'height="{f.lane_height(lane)}" fill="{fill}"/>\n'
        for lane, fill in zip(f.lanes, fills, strict=True)
    )
    out += (
        f'  <rect x="{f.x_left}" y="{f.plot_top}" width="{f.x_right - f.x_left}" '
        f'height="{f.plot_bottom - f.plot_top}" fill="none" stroke="#d1d5db" stroke-width="1"/>\n'
    )
    out += '  <g stroke="#e5e7eb" stroke-width="1">\n'
    out += "".join(
        f'    <line x1="{f.x(t):.1f}" y1="{f.plot_top}" x2="{f.x(t):.1f}" y2="{f.plot_bottom}"/>\n'
        for t in f.ticks()
        if t != 0
    )
    return out + "  </g>\n"


def time_axis(m: Metrics, f: Frame) -> str:
    axis_y = f.plot_bottom + 20
    tick_y, label_y = axis_y + 18, axis_y + 35
    milestones_bottom = f.plot_bottom + 120
    out = '  <g stroke-dasharray="4 4">\n'
    for t, color, width in (
        (0, "#dc2626", 1.5),
        (m["cycle"], "#7c3aed", 1.2),
        (m["finality_ms"], "#059669", 1.5),
    ):
        out += (
            f'    <line x1="{f.x(t):.1f}" y1="{f.plot_top}" x2="{f.x(t):.1f}" '
            f'y2="{milestones_bottom}" stroke="{color}" stroke-width="{width}"/>\n'
        )
    out += "  </g>\n"
    out += line(f.x_left, axis_y, f.x_right, axis_y, "#374151", 1.4)
    out += '  <g font-size="11" fill="#6b7280" text-anchor="middle">\n'
    for t in f.ticks():
        weight = ' font-weight="700" fill="#dc2626"' if t == 0 else ""
        label = f"{t:+d}" if t > 0 else str(t)
        out += f'    <text x="{f.x(t):.1f}" y="{tick_y}"{weight}>{label}</text>\n'
    for t, color in ((m["cycle"], "#7c3aed"), (m["finality_ms"], "#059669")):
        out += f'    <text x="{f.x(t):.1f}" y="{label_y}" font-weight="700" fill="{color}">{t:.0f}</text>\n'
    return out + "  </g>\n"


def lane_labels(f: Frame) -> str:
    labels = {
        "L_V": ("Leader L_V", "build → VID ∥ fan-out → Cert1 wait → queued V"),
        "NextRX": ("L_{V+1} vote1 collect", "vote1[V] → Cert1(V) on L_{V+1}"),
        "NextRecon": (
            "L_{V+1} recover V",
            "shares → decode → txn-commit  (CPU contention)",
        ),
        "NextCPU": (
            "L_{V+1} build V+1",
            "build → VID ∥ fan-out → Cert1 wait → queued V+1",
        ),
        "Recon": (
            "Recon (V, replicas)",
            "validate V → share-wait → decode → txn-commit",
        ),
        "Deciding": (
            "Deciding (median)",
            "vote1 → Cert1 → vote2 → Cert2 → decide (off cycle)",
        ),
    }
    out = '  <g font-size="13" fill="#111827">\n'
    for lane, (title, description) in labels.items():
        driver = lane in ("NextRX", "NextRecon", "NextCPU")
        color = "#92400e" if driver else "#111827"
        sub_color = "#92400e" if driver else "#6b7280"
        out += f'    <text x="{f.x_left - 10}" y="{f.mid(lane) - 6}" text-anchor="end" font-weight="700" fill="{color}">{title}</text>\n'
        out += f'    <text x="{f.x_left - 10}" y="{f.mid(lane) + 12}" text-anchor="end" font-size="11" fill="{sub_color}">{description}</text>\n'
    return out + "  </g>\n"


def build_bars(m: Metrics, f: Frame, offset: float, lane: str) -> str:
    """The leader's build spans shifted by `offset`, with a marker at block_built."""
    out, placed = "", []
    for row, label, start, end, color in build_segments(m):
        t0, t1 = m[start] + offset, m[end] + offset
        if t1 - t0 < 1:
            continue
        out += bar(
            f,
            t0,
            t1,
            f.bar_y(lane, row),
            f.bar_height(lane),
            color,
            f"{label} {t1 - t0:.0f}",
            9,
            outside="above" if row == 0 else "right",
            placed=placed,
        )
    if not m["present_block_built"]:
        return out
    return out + dot(f.x(m["block_built"] + offset), f.cy(lane), 4, "#1e40af")


def leader_lane(m: Metrics, f: Frame) -> str:
    """L_V's work in the cycle before t = 0, ending in proposal_queued(V)."""
    cy = f.cy("L_V")
    return (
        build_bars(m, f, 0.0, "L_V")
        + dot(f.x(0), cy, 7, "#dc2626")
        + text(f.x(0) + 12, cy + 4, "proposal_queued(V)", 11, "#dc2626", 700, "start")
    )


def next_vote1_lane(m: Metrics, f: Frame) -> str:
    """Cycle driver 1: median vote1[V] arrival to Cert1(V) at L_{V+1}, which by symmetry is L_V's
    own Cert1[V-1] time shifted by one cycle."""
    y, cy = f.bar_y("NextRX"), f.cy("NextRX")
    start, end = m["vote1_arrived"], m["cert1_at_L_V"] + m["cycle"]
    return (
        dot(f.x(start), cy, 5, "#1d4ed8")
        + text(f.x(start), y - 4, "vote1[V] arrives (median)", 10, "#1d4ed8", 600)
        + bar(
            f,
            start,
            end,
            y,
            f.bar_h,
            COLLECT,
            f"collect vote1 + Cert1  ({end - start:.0f} ms)",
            11,
        )
        + dot(f.x(end), cy, 6, "#dc2626")
        + text(
            f.x(end) - 10,
            y + f.bar_h + 12,
            "Cert1(V) at L_{V+1} (gates propose V+1)",
            11,
            "#dc2626",
            700,
            "end",
        )
    )


def next_recover_ends(m: Metrics) -> tuple[float, float, float]:
    """Start, decode end and end of L_{V+1}'s recovery of V: L_V's recovery of V-1 shifted by
    one cycle."""
    cycle = m["cycle"]
    return (
        m["leader_recover_start"] + cycle,
        m["leader_recover_decode_end"] + cycle,
        m["verify_at_L_V"] + cycle,
    )


def recover_bars(f: Frame, y: int, start: float, decode: float, end: float) -> str:
    """Decode and commit bars. Labels that do not fit go below, joined, ending at the bar end."""
    parts = [
        (start, decode, RECOVER, f"recover-decode {decode - start:.0f}"),
        (decode, end, COMMIT, f"txn-commit {end - decode:.0f}"),
    ]
    out, unfit = "", []
    for t0, t1, color, body in parts:
        if t1 <= t0:
            continue
        out += rect(f.x(t0), y, f.w(t0, t1), f.bar_h, color)
        if fits(f, t0, t1, body, 10):
            out += text(
                (f.x(t0) + f.x(t1)) / 2, y + f.bar_h // 2 + 4, body, 10, "white", 600
            )
        else:
            unfit.append(body)
    if unfit:
        out += text(
            f.x(end) - 6, y + f.bar_h + 12, " + ".join(unfit), 10, "#374151", 600, "end"
        )
    return out


def next_recover_lane(m: Metrics, f: Frame) -> str:
    """Cycle driver 2: L_{V+1} reconstructs V's payload before it can build on it; it shares
    the CPU with the build below."""
    y, cy = f.bar_y("NextRecon"), f.cy("NextRecon")
    start, decode, end = next_recover_ends(m)
    if not (start and end and end > start):
        return ""
    out = dot(f.x(start), cy, 5, "#047857") + text(
        f.x(start), y - 4, "threshold_share_reached", 10, "#047857", 600
    )
    if decode > start and end > decode:
        out += recover_bars(f, y, start, decode, end)
    else:
        out += bar(
            f,
            start,
            end,
            y,
            f.bar_h,
            RECOVER,
            f"recover V on L_{{V+1}} {end - start:.0f}",
            10,
            outside="below",
        )
    return (
        out
        + dot(f.x(end), cy, 5, "#065f46")
        + text(
            f.x(end) + 12,
            y + f.bar_h + 12,
            "payload V ready at L_{V+1}",
            10,
            "#065f46",
            600,
            "start",
        )
    )


def next_build_lane(m: Metrics, f: Frame) -> str:
    """Cycle driver 3: L_{V+1} builds V+1, the leader's build spans shifted by one cycle."""
    y, cycle = f.bar_y("NextCPU"), m["cycle"]
    cy = f.cy("NextCPU")
    return (
        build_bars(m, f, cycle, "NextCPU")
        + text(
            f.x(m["request_block"] + cycle),
            y - 4,
            "request_block(V+1)",
            10,
            "#92400e",
            600,
            "end",
        )
        + dot(f.x(cycle), cy, 6, "#dc2626")
        + text(
            f.x(cycle) + 12, cy + 4, "proposal_queued(V+1)", 11, "#dc2626", 700, "start"
        )
    )


def recon_lane(m: Metrics, f: Frame) -> str:
    y, cy = f.bar_y("Recon"), f.cy("Recon")
    recv, reached = m["proposal_validated"], m["vid_threshold_reached"]
    decode_end, built = m["recover_decode_end"], m["block_reconstructed"]
    out = ""
    if m["share_wait_ms"] > 0:
        out += (
            dot(f.x(recv), cy, 5, "#b45309")
            + text(f.x(recv), y - 4, "proposal_validated(V)", 10, "#b45309", 600, "end")
            + bar(
                f,
                recv,
                reached,
                y,
                f.bar_h,
                SHARE_WAIT,
                f"share wait {m['share_wait_ms']:.0f}",
                10,
                "#065f46",
            )
        )
    if m["recover_cpu_ms"] > 0:
        out += dot(f.x(reached), cy, 5, "#047857") + text(
            f.x(reached), y - 4, "vid_threshold_reached", 10, "#047857", 600, "start"
        )
        out += recover_bars(
            f,
            y,
            reached,
            decode_end if m["recover_commit_ms"] > 0 else built,
            built,
        )
    return (
        out
        + dot(f.x(built), cy, 5, "#065f46")
        + text(
            f.x(built) + 8,
            y + f.bar_h + 12,
            "block_reconstructed(V)",
            10,
            "#065f46",
            600,
            "start",
        )
    )


def deciding_lane(m: Metrics, f: Frame) -> str:
    """V's finality chain from cluster medians; it does not gate proposal_queued(V+1)."""
    y, cy = f.bar_y("Deciding"), f.cy("Deciding")
    v1, c1, v2, c2, fin = (
        m["vote1_arrived"],
        m["cert1_formed"],
        m["vote2_sent"],
        m["cert2_formed"],
        m["finality_ms"],
    )
    out = (
        dot(f.x(v1), cy, 5, "#1d4ed8")
        + text(f.x(v1), y - 4, "vote1_arrived (median)", 10, "#1d4ed8", 600)
        + bar(
            f,
            v1,
            c1,
            y,
            f.bar_h,
            VOTE1_CERT1,
            f"vote1 → Cert1 ({m['cert1_wait_ms']:.0f})",
            10,
            "#3730a3",
        )
        + dot(f.x(c1), cy, 5, "#7c3aed")
        + text(f.x(c1), y + f.bar_h + 12, "Cert1(V) median", 10, "#7c3aed", 600)
        + rect(f.x(c1), y, f.w(c1, v2), f.bar_h, CERT1_VOTE2)
        + dot(f.x(v2), cy, 5, "#1d4ed8")
        + text(f.x(v2), y - 4, "vote2_sent (median)", 10, "#1d4ed8", 600)
        + bar(
            f,
            v2,
            c2,
            y,
            f.bar_h,
            VOTE2_CERT2,
            f"vote2 → Cert2 ({m['cert2_wait_ms']:.0f})",
            10,
        )
    )
    return (
        out
        + dot(f.x(c2), cy, 5, "#7c3aed")
        + text(f.x(c2), y - 4, "Cert2(V) median", 10, "#7c3aed", 600, "end")
        + rect(f.x(c2), y, f.w(c2, fin), f.bar_h, CERT2_DECIDED)
        + dot(f.x(fin), cy, 6, "#059669")
        + text(f.x(fin) + 8, y - 4, "leaf_decided(V) ✓", 11, "#059669", 700, "start")
    )


def arrows(m: Metrics, f: Frame) -> str:
    """Red: the V to V+1 cycle chain. Gray dashed: dependencies that run in parallel."""
    cycle = m["cycle"]
    cert1_next = m["cert1_at_L_V"] + cycle
    _, _, recover_end = next_recover_ends(m)
    cy = f.cy
    red = [
        f"M {f.x(0):.1f} {cy('L_V') + 8} V {cy('Recon')} H {f.x(m['proposal_validated']) - 6:.1f}",
        f"M {f.x(m['vote1_arrived']):.1f} {cy('Deciding') + 8} V {cy('NextRX')} H {f.x(m['vote1_arrived']) + 6:.1f}",
        f"M {f.x(cert1_next) + 4:.1f} {cy('NextRX') + 8} V {cy('NextCPU')} H {f.x(cycle) - 6:.1f}",
    ]
    gray = [
        f"M {f.x(m['block_built'] + cycle) + 2:.1f} {cy('NextCPU')} H {f.x(cycle) - 6:.1f}",
        f"M {f.x(m['block_reconstructed']):.1f} {cy('Recon') + 8} V {cy('Deciding')} H {f.x(m['vote2_sent']) - 6:.1f}",
    ]
    if recover_end:
        gray.insert(
            1,
            f"M {f.x(recover_end) + 4:.1f} {cy('NextRecon')} V {cy('NextCPU')} H {f.x(cycle) - 6:.1f}",
        )
    return "".join(path(d, "#dc2626", 2, "critArrow") for d in red) + "".join(
        path(d, "#9ca3af", 1.2, "grayArrow", dashed=True) for d in gray
    )


def bracket(f: Frame, end: float, y: int, color: str, body: str) -> str:
    x0, x1 = f.x(0), f.x(end)
    return (
        line(x0, y, x1, y, color, 1.5)
        + line(x0, y - 6, x0, y + 6, color, 1.5)
        + line(x1, y - 6, x1, y + 6, color, 1.5)
        + text((x0 + x1) / 2, y + 18, body, 12, color, 600)
    )


def brackets(m: Metrics, f: Frame) -> str:
    ratio = m["finality_ms"] / m["cycle"] if m["cycle"] > 0 else 0
    return bracket(
        f,
        m["cycle"],
        f.plot_bottom + 80,
        "#7c3aed",
        f"per-view cycle = {m['cycle']:.0f} ms     (throughput ≈ {m['throughput_vps']:.2f} v/s)",
    ) + bracket(
        f,
        m["finality_ms"],
        f.plot_bottom + 120,
        "#059669",
        f"finality of V ≈ {m['finality_ms']:.0f} ms     (≈ {ratio:.2f} × per-view cycle, pipeline depth = 2)",
    )


LEGEND_SWATCHES = (
    ("build/prep", BUILD_PREP),
    ("dispersal", DISPERSAL),
    ("share-sign", SHARE_SIGN),
    ("fan-out", FAN_OUT),
    ("header", HEADER),
    ("parent Cert1 wait", CERT1_WAIT),
    ("commit + sign", COMMIT_SIGN),
    ("share wait", SHARE_WAIT),
    ("recover-decode", RECOVER),
    ("txn-commit", COMMIT),
    ("collect vote1, aggregate Cert1", COLLECT),
    ("vote1 to Cert1", VOTE1_CERT1),
    ("Cert1 to vote2", CERT1_VOTE2),
    ("vote2 to Cert2", VOTE2_CERT2),
    ("Cert2 to decided", CERT2_DECIDED),
)
LEGEND_COLUMNS, LEGEND_COLUMN_W, LEGEND_ROW_H = 5, 270, 22


def legend(f: Frame) -> str:
    """Swatches of the colors drawn, then the two arrow styles."""
    out = f'  <g transform="translate({f.x_left}, {f.plot_bottom + 170})" font-size="11" fill="#374151">\n'
    for i, (label, color) in enumerate(LEGEND_SWATCHES):
        row, col = divmod(i, LEGEND_COLUMNS)
        x, y = col * LEGEND_COLUMN_W, row * LEGEND_ROW_H
        out += f'    <rect x="{x}" y="{y}" width="14" height="14" fill="{color}" rx="2"/>\n'
        out += f'    <text x="{x + 20}" y="{y + 11}">{label}</text>\n'
    y = math.ceil(len(LEGEND_SWATCHES) / LEGEND_COLUMNS) * LEGEND_ROW_H
    out += f"""    <line x1="0" y1="{y + 7}" x2="30" y2="{y + 7}" stroke="#dc2626" stroke-width="2.2" marker-end="url(#critArrow)"/>
    <text x="38" y="{y + 11}" font-weight="600" fill="#dc2626">critical path</text>
    <line x1="{LEGEND_COLUMN_W}" y1="{y + 7}" x2="{LEGEND_COLUMN_W + 30}" y2="{y + 7}" stroke="#9ca3af" stroke-width="1.2" stroke-dasharray="3 3" marker-end="url(#grayArrow)"/>
    <text x="{LEGEND_COLUMN_W + 38}" y="{y + 11}" fill="#6b7280">parallel dependency</text>
  </g>
"""
    return out
