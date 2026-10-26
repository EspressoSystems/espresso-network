"""Block finalization timeline of the leader traces as one SVG: the median view, anchored at the
leader's `proposal_queued` (t = 0), with the next leader's pipeline one cycle later.

Every metric is a median over the given views. A metric whose events the traces lack is 0 and its
bars or markers collapse. `ns_disperse_end` and `vid_shares_unicast_start` are absent from some
traces; the leader's build then ends in one "disperse + fan-out" bar."""

import statistics
from dataclasses import dataclass
from pathlib import Path

from leadertrace import NS_PER_MS, Trace, leaders

# metric -> (event, where). "leader": L_V's own stamp of view V; "cluster": the median stamp over
# all nodes at view V; "leader_prev": L_V's stamp at view V - 1.
EVENTS = {
    "request_block": ("request_block_header_queued", "leader"),
    "block_built": ("block_built_applied", "leader"),
    "ns_disperse_start": ("ns_disperse_start", "leader"),
    "vid_disperse": ("ns_disperse_end", "leader"),
    "vid_unicast_start": ("vid_shares_unicast_start", "leader"),
    "vid_unicast_end": ("vid_shares_unicast_end", "leader"),
    "proposal_recv": ("proposal_validated_v_minus_1", "cluster"),
    "vote1_sent": ("vote1_v_minus_1_arrived", "cluster"),
    "vid_threshold_reached": ("threshold_share_reached_v_minus_1", "cluster"),
    "recover_decode_end": ("recover_v_minus_1_decode_end", "cluster"),
    "block_reconstructed": ("recover_v_minus_1_end", "cluster"),
    "cert1_formed": ("cert1_v_minus_1_input_dispatched", "cluster"),
    "vote2_sent": ("vote2_v_minus_1_queued", "cluster"),
    "cert2_formed": ("cert2_v_minus_1_input_dispatched", "cluster"),
    "cert1_at_L_V": ("cert1_v_minus_1_input_dispatched", "leader_prev"),
    "leader_recover_start": ("recover_v_minus_1_start", "leader_prev"),
    "leader_recover_decode_end": ("recover_v_minus_1_decode_end", "leader_prev"),
    "verify_at_L_V": ("recover_v_minus_1_end", "leader_prev"),
}

Metrics = dict[str, float]
ViewRow = dict[str, float | None]

TITLE = "Block V finalization timeline"
BUILD_PREP, DISPERSAL, SHARE_SIGN, FAN_OUT, HEADER = (
    "#94a3b8",
    "#2563eb",
    "#9333ea",
    "#ea580c",
    "#64748b",
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
        stamps = sorted(
            ts for node in trace if (ts := stamp(node, view, event)) is not None
        )
        return rel(stamps[round(0.5 * (len(stamps) - 1))]) if stamps else None

    return {"cycle": (queued - lead[view - 1][1]) / NS_PER_MS} | {
        name: metric(*spec) for name, spec in EVENTS.items()
    }


def timeline_metrics(rows: list[ViewRow]) -> Metrics:
    """Medians over the rows plus the durations the SVG labels. Missing values are 0."""
    if not rows:
        raise ValueError("no views for the timeline")

    def median(name: str) -> float:
        values = [v for row in rows if (v := row[name]) is not None]
        return statistics.median(values) if values else 0.0

    m = {"n_views": float(len(rows))}
    m |= {name: median(name) for name in ("cycle", *EVENTS)}
    m["share_wait_ms"] = max(0.0, m["vid_threshold_reached"] - m["proposal_recv"])
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
    m["cert1_wait_ms"] = max(0.0, m["cert1_formed"] - m["vote1_sent"])
    m["cert2_wait_ms"] = max(0.0, m["cert2_formed"] - m["vote2_sent"])
    # No leaf_decided in the cluster medians: Cert2 plus a broadcast tail of 10 ms.
    m["finality_ms"] = m["cert2_formed"] + 10
    m["throughput_vps"] = 1000.0 / m["cycle"] if m["cycle"] > 0 else 0.0
    return m


def build_segments(m: Metrics) -> list[tuple[str, str, str, str]]:
    """(label, start metric, end metric, color) of the leader's build, in event order."""
    if m["vid_disperse"] and m["vid_unicast_start"]:
        middle = [
            ("dispersal", "ns_disperse_start", "vid_disperse", DISPERSAL),
            ("share-sign", "vid_disperse", "vid_unicast_start", SHARE_SIGN),
            ("fan-out", "vid_unicast_start", "vid_unicast_end", FAN_OUT),
        ]
    else:
        middle = [
            (
                "disperse + fan-out",
                "ns_disperse_start",
                "vid_unicast_end",
                DISPERSAL,
            )
        ]
    return [
        ("build/prep", "request_block", "ns_disperse_start", BUILD_PREP),
        *middle,
        ("header", "vid_unicast_end", "block_built", HEADER),
    ]


@dataclass(frozen=True)
class Frame:
    """Time axis and lane geometry of the SVG."""

    t_min: int
    t_max: int
    width: int = 1700
    height: int = 710
    x_left: int = 220
    x_right: int = 1680
    plot_top: int = 100
    lane_h: int = 68
    bar_h: int = 26
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

    @property
    def plot_bottom(self) -> int:
        return self.plot_top + len(self.lanes) * self.lane_h

    def x(self, t: float) -> float:
        span = self.t_max - self.t_min
        if span <= 0:
            return self.x_left
        return self.x_left + (t - self.t_min) * (self.x_right - self.x_left) / span

    def w(self, t1: float, t2: float) -> float:
        return max(0, self.x(t2) - self.x(t1))

    def top(self, lane: str) -> int:
        return self.plot_top + self.lanes.index(lane) * self.lane_h

    def bar_y(self, lane: str) -> int:
        return self.top(lane) + (self.lane_h - self.bar_h) // 2 + 1

    def cy(self, lane: str) -> int:
        return self.top(lane) + self.lane_h // 2

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
        LEGEND.format(x=f.x_left, y=f.plot_bottom + 170),
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


def bar_label(f: Frame, t0: float, t1: float, y: int, body: str, fill: str, size: int):
    return text((f.x(t0) + f.x(t1)) / 2, y + f.bar_h // 2 + 4, body, size, fill, 600)


def svg_header(m: Metrics, f: Frame) -> str:
    subtitle = (
        f"{m['n_views']:.0f} views, t = 0 anchored at proposal_sent(V), "
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
    """L_{V+1} proposes once Cert1(V) has formed at it and its own build is done; the bar is the
    RX chain's share of the cycle. The CPU chain (the build, `-block_built`) lies under the
    lanes."""
    gate_y, height = 86, 10
    rx_ms = m["cert1_at_L_V"] + m["cycle"]
    pct = rx_ms / m["cycle"] if m["cycle"] > 0 else 0
    width = (f.x_right - f.x_left - 280) * pct
    return (
        text(
            f.x_left,
            gate_y - 3,
            "Cycle gate = max(RX chain, CPU chain) on L_{V+1} — both must finish before proposal_sent(V+1)",
            11,
            "#374151",
            600,
            "start",
        )
        + f'  <rect x="{f.x_left:.1f}" y="{gate_y}" width="{width:.1f}" height="{height}" fill="#8b5cf6"/>\n'
        + text(
            f.x_left + width / 2,
            gate_y + height - 1,
            f"RX chain: vote1 collect → Cert1(V) @ L_{{V+1}} = {rx_ms:.0f} ms ({pct * 100:.0f}% of cycle)",
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
        f'height="{f.lane_h}" fill="{fill}"/>\n'
        for lane, fill in zip(f.lanes, fills, strict=True)
    )
    out += (
        f'  <rect x="{f.x_left}" y="{f.plot_top}" width="{f.x_right - f.x_left}" '
        f'height="{len(f.lanes) * f.lane_h}" fill="none" stroke="#d1d5db" stroke-width="1"/>\n'
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
        "L_V": ("Leader L_V", "build → VID → share-sign → fan-out → sent V"),
        "NextRX": ("L_{V+1} vote1 collect", "vote1[V] → Cert1(V) on L_{V+1}"),
        "NextRecon": (
            "L_{V+1} recover V",
            "shares → decode → tx-commit  (CPU contention)",
        ),
        "NextCPU": (
            "L_{V+1} build V+1",
            "build → VID → share-sign → fan-out → sent V+1",
        ),
        "Recon": (
            "Recon (V, replicas)",
            "recv V → share-wait → decode → tx-commit",
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
        out += f'    <text x="210" y="{f.cy(lane) - 6}" text-anchor="end" font-weight="700" fill="{color}">{title}</text>\n'
        out += f'    <text x="210" y="{f.cy(lane) + 12}" text-anchor="end" font-size="11" fill="{sub_color}">{description}</text>\n'
    return out + "  </g>\n"


def build_bars(m: Metrics, f: Frame, offset: float, lane: str) -> str:
    """The leader's build spans shifted by `offset`, with a marker at block_built."""
    y = f.bar_y(lane)
    out = ""
    for label, start, end, color in build_segments(m):
        t0, t1 = m[start] + offset, m[end] + offset
        if t1 <= t0:
            continue
        out += rect(f.x(t0), y, f.w(t0, t1), f.bar_h, color)
        if f.w(t0, t1) > max(34, 4.5 * len(label)):
            out += bar_label(f, t0, t1, y, f"{label} {t1 - t0:.0f}", "white", 9)
    return out + dot(f.x(m["block_built"] + offset), f.cy(lane), 4, "#1e40af")


def leader_lane(m: Metrics, f: Frame) -> str:
    """L_V's work in the cycle before t = 0, ending in proposal_sent(V)."""
    y = f.bar_y("L_V")
    return (
        build_bars(m, f, 0.0, "L_V")
        + dot(f.x(0), f.cy("L_V"), 7, "#dc2626")
        + text(f.x(0), y - 6, "proposal_sent(V)", 11, "#dc2626", 700)
    )


def next_vote1_lane(m: Metrics, f: Frame) -> str:
    """Cycle driver 1: first vote1[V] arrival to Cert1(V) at L_{V+1}, which by symmetry is L_V's
    own Cert1[V-1] time shifted by one cycle."""
    y, cy = f.bar_y("NextRX"), f.cy("NextRX")
    start, end = m["vote1_sent"], m["cert1_at_L_V"] + m["cycle"]
    return (
        dot(f.x(start), cy, 5, "#1d4ed8")
        + text(f.x(start), y - 4, "first vote1[V] arrives", 10, "#1d4ed8", 600)
        + rect(f.x(start), y, f.w(start, end), f.bar_h, "#8b5cf6")
        + bar_label(
            f,
            start,
            end,
            y,
            f"collect vote1[V] + verify + aggregate Cert1(V)  ({end - start:.0f} ms)",
            "white",
            11,
        )
        + dot(f.x(end), cy, 6, "#dc2626")
        + text(
            f.x(end),
            y + f.bar_h + 12,
            "Cert1(V) at L_{V+1} (gates propose V+1)",
            11,
            "#dc2626",
            700,
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
        out += rect(f.x(start), y, f.w(start, decode), f.bar_h, "#10b981")
        if f.w(start, decode) > 50:
            out += bar_label(
                f, start, decode, y, f"recover-decode {decode - start:.0f}", "white", 10
            )
        out += rect(f.x(decode), y, f.w(decode, end), f.bar_h, "#d97706")
        if f.w(decode, end) > 50:
            out += bar_label(
                f, decode, end, y, f"tx-commit {end - decode:.0f}", "white", 10
            )
    else:
        out += rect(f.x(start), y, f.w(start, end), f.bar_h, "#10b981")
        if f.w(start, end) > 50:
            out += bar_label(
                f,
                start,
                end,
                y,
                f"recover V on L_{{V+1}} {end - start:.0f}",
                "white",
                10,
            )
    return (
        out
        + dot(f.x(end), cy, 5, "#065f46")
        + text(
            f.x(end), y + f.bar_h + 12, "payload V ready at L_{V+1}", 10, "#065f46", 600
        )
    )


def next_build_lane(m: Metrics, f: Frame) -> str:
    """Cycle driver 3: L_{V+1} builds V+1, the leader's build spans shifted by one cycle."""
    y, cycle = f.bar_y("NextCPU"), m["cycle"]
    return (
        build_bars(m, f, cycle, "NextCPU")
        + text(
            f.x(m["request_block"] + cycle),
            y - 4,
            "request_block(V+1)",
            10,
            "#92400e",
            600,
        )
        + dot(f.x(cycle), f.cy("NextCPU"), 6, "#dc2626")
        + text(f.x(cycle), y - 6, "proposal_sent(V+1)", 11, "#dc2626", 700)
    )


def recon_lane(m: Metrics, f: Frame) -> str:
    y, cy = f.bar_y("Recon"), f.cy("Recon")
    recv, reached = m["proposal_recv"], m["vid_threshold_reached"]
    decode_end, built = m["recover_decode_end"], m["block_reconstructed"]
    out = ""
    if m["share_wait_ms"] > 0:
        out += (
            dot(f.x(recv), cy, 5, "#b45309")
            + text(f.x(recv), y - 4, "proposal_recv(V)", 10, "#b45309", 600)
            + rect(f.x(recv), y, f.w(recv, reached), f.bar_h, "#bbf7d0")
            + bar_label(
                f,
                recv,
                reached,
                y,
                f"share wait {m['share_wait_ms']:.0f}",
                "#065f46",
                10,
            )
        )
    if m["recover_cpu_ms"] > 0:
        out += dot(f.x(reached), cy, 5, "#047857") + text(
            f.x(reached), y - 4, "vid_threshold_reached", 10, "#047857", 600
        )
        if m["recover_commit_ms"] > 0:
            out += rect(f.x(reached), y, f.w(reached, decode_end), f.bar_h, "#10b981")
            out += bar_label(
                f,
                reached,
                decode_end,
                y,
                f"recover-decode (par) {m['recover_decode_ms']:.0f}",
                "white",
                10,
            )
            out += rect(f.x(decode_end), y, f.w(decode_end, built), f.bar_h, "#d97706")
            out += bar_label(
                f,
                decode_end,
                built,
                y,
                f"txn-commit (Keccak) {m['recover_commit_ms']:.0f}",
                "white",
                10,
            )
        else:
            out += rect(f.x(reached), y, f.w(reached, built), f.bar_h, "#10b981")
            out += bar_label(
                f,
                reached,
                built,
                y,
                f"AvidmGf2::recover — {m['recover_cpu_ms']:.0f} ms",
                "white",
                11,
            )
    return (
        out
        + dot(f.x(built), cy, 5, "#065f46")
        + text(
            f.x(built), y + f.bar_h + 12, "block_reconstructed(V)", 10, "#065f46", 600
        )
    )


def deciding_lane(m: Metrics, f: Frame) -> str:
    """V's finality chain from cluster medians; it does not gate proposal_sent(V+1)."""
    y, cy = f.bar_y("Deciding"), f.cy("Deciding")
    v1, c1, v2, c2, fin = (
        m["vote1_sent"],
        m["cert1_formed"],
        m["vote2_sent"],
        m["cert2_formed"],
        m["finality_ms"],
    )
    out = (
        dot(f.x(v1), cy, 5, "#1d4ed8")
        + text(f.x(v1), y - 4, "vote1_sent (median)", 10, "#1d4ed8", 600)
        + rect(f.x(v1), y, f.w(v1, c1), f.bar_h, "#c4b5fd")
    )
    if f.w(v1, c1) > 60:
        out += bar_label(
            f, v1, c1, y, f"vote1 → Cert1 ({m['cert1_wait_ms']:.0f})", "#3730a3", 10
        )
    out += (
        dot(f.x(c1), cy, 5, "#7c3aed")
        + text(f.x(c1), y + f.bar_h + 12, "Cert1(V) median", 10, "#7c3aed", 600)
        + rect(f.x(c1), y, f.w(c1, v2), f.bar_h, "#fde68a")
    )
    if f.w(c1, v2) > 60:
        out += text(
            (f.x(c1) + f.x(v2)) / 2,
            y + f.bar_h // 2 + 4,
            "Cert1 → vote2",
            10,
            "#92400e",
        )
    out += (
        dot(f.x(v2), cy, 5, "#1d4ed8")
        + text(f.x(v2), y - 4, "vote2_sent (median)", 10, "#1d4ed8", 600)
        + rect(f.x(v2), y, f.w(v2, c2), f.bar_h, "#a78bfa")
    )
    if f.w(v2, c2) > 60:
        out += bar_label(
            f, v2, c2, y, f"vote2 → Cert2 ({m['cert2_wait_ms']:.0f})", "white", 10
        )
    return (
        out
        + dot(f.x(c2), cy, 5, "#7c3aed")
        + text(f.x(c2), y - 4, "Cert2(V) median", 10, "#7c3aed", 600)
        + rect(f.x(c2), y, f.w(c2, fin), f.bar_h, "#a7f3d0")
        + dot(f.x(fin), cy, 6, "#059669")
        + text(f.x(fin), y - 4, "leaf_decided(V) ✓", 11, "#059669", 700)
    )


def arrows(m: Metrics, f: Frame) -> str:
    """Red: the V to V+1 cycle chain. Gray dashed: dependencies that run in parallel."""
    cycle = m["cycle"]
    cert1_next = m["cert1_at_L_V"] + cycle
    _, _, recover_end = next_recover_ends(m)
    cy = f.cy
    red = [
        f"M {f.x(0):.1f} {cy('L_V') + 8} V {cy('Recon')} H {f.x(m['proposal_recv']) - 6:.1f}",
        f"M {f.x(m['vote1_sent']):.1f} {cy('Deciding') + 8} V {cy('NextRX')} H {f.x(m['vote1_sent']) + 6:.1f}",
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


LEGEND = """  <g transform="translate({x}, {y})" font-size="11" fill="#374151">
    <rect x="0"   y="0" width="14" height="14" fill="#94a3b8" rx="2"/>
    <text x="20"  y="11">build/prep</text>
    <rect x="95"  y="0" width="14" height="14" fill="#2563eb" rx="2"/>
    <text x="115" y="11">dispersal</text>
    <rect x="185" y="0" width="14" height="14" fill="#ea580c" rx="2"/>
    <text x="205" y="11">fan-out</text>
    <rect x="270" y="0" width="14" height="14" fill="#f59e0b" rx="2"/>
    <text x="290" y="11">network + replica validation</text>
    <rect x="470" y="0" width="14" height="14" fill="#bbf7d0" rx="2"/>
    <text x="490" y="11">share collection wait</text>
    <rect x="620" y="0" width="14" height="14" fill="#10b981" rx="2"/>
    <text x="640" y="11">recover CPU</text>
    <rect x="740" y="0" width="14" height="14" fill="#8b5cf6" rx="2"/>
    <text x="760" y="11">vote aggregation / cert</text>
    <rect x="910" y="0" width="14" height="14" fill="#e5e7eb" stroke="#9ca3af" stroke-dasharray="3 3"/>
    <text x="930" y="11">idle / slack</text>
    <line x1="1010" y1="7" x2="1040" y2="7" stroke="#dc2626" stroke-width="2.2" marker-end="url(#critArrow)"/>
    <text x="1048" y="11" font-weight="600" fill="#dc2626">critical path</text>
    <line x1="1160" y1="7" x2="1190" y2="7" stroke="#9ca3af" stroke-width="1.2" stroke-dasharray="3 3" marker-end="url(#grayArrow)"/>
    <text x="1198" y="11" fill="#6b7280">parallel dependency</text>
  </g>
"""
