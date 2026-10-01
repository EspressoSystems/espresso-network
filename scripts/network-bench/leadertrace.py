"""Phase breakdown of consensus views from the per-node `leader_trace_node{N}.csv` files that
espresso-node writes under `ESPRESSO_NODE_LEADER_TRACE_DIR`. Pure functions, stdlib only.

Every row is keyed by the subject view the event is about. The leader of view V is the node
whose trace has `proposal_queued` for V. Phases of V are measured from t0 = the previous leader's
`proposal_queued`. Phases that span hosts (idle, vote1->cert1, finality) assume chrony-synced
clocks; a negative idle indicates skew."""

import csv
import itertools
import statistics
from collections import Counter
from collections.abc import Iterable
from pathlib import Path

# node -> view -> event -> first ts_ns
Trace = dict[int, dict[int, dict[str, int]]]

HEADER = ["view", "node_id", "event", "ts_ns"]
PHASES = ("idle", "block build", "disperse+send", "vote1->cert1", "cert1->decided")
LEADER_STAMPS = (
    "request_block_header_queued",
    "header_created_applied",
    "proposal_queued",
)
CERT1 = "cert1_v_minus_1_input_dispatched"
DECIDED = "leaf_decided"
DEFAULT_WARMUP = 10
NS_PER_MS = 1_000_000


def read_traces(paths: Iterable[Path]) -> Trace:
    trace: Trace = {}
    for path in paths:
        with path.open(newline="") as f:
            reader = csv.reader(f)
            if next(reader, None) != HEADER:
                raise ValueError(f"{path}:1: header is not {','.join(HEADER)}")
            for row in reader:
                try:
                    view, node, event, ts = row
                    stamps = trace.setdefault(int(node), {}).setdefault(int(view), {})
                    stamps.setdefault(event, int(ts))
                except ValueError as exc:
                    raise ValueError(f"{path}:{reader.line_num}: {row}: {exc}") from exc
    return trace


def leaders(trace: Trace) -> dict[int, tuple[int, int]]:
    """View to (leader node, proposal_queued ts_ns)."""
    found: dict[int, tuple[int, int]] = {}
    for node, views in trace.items():
        for view, events in views.items():
            if "proposal_queued" not in events:
                continue
            if view in found:
                raise ValueError(
                    f"view {view} has proposal_queued on nodes {found[view][0]} and {node}"
                )
            found[view] = (node, events["proposal_queued"])
    return found


def view_phases(
    trace: Trace, warmup: int = DEFAULT_WARMUP
) -> tuple[dict[int, list[float]], Counter[str]]:
    """View to the durations in ms of `PHASES`, and the reasons views were skipped."""
    lead = leaders(trace)
    phases: dict[int, list[float]] = {}
    skipped: Counter[str] = Counter()
    for view in sorted(v for v in lead if v >= warmup):
        stamps = _stamps(trace, lead, view)
        if isinstance(stamps, str):
            skipped[stamps] += 1
            continue
        durations = [(b - a) / NS_PER_MS for a, b in itertools.pairwise(stamps)]
        negative = [name for name, d in zip(PHASES, durations) if d < 0]
        if negative:
            skipped[f"negative {negative[0]}"] += 1
            continue
        phases[view] = durations
    return phases, skipped


def _stamps(
    trace: Trace, lead: dict[int, tuple[int, int]], view: int
) -> list[float] | str:
    """[t0, queued, built, proposal, cert1, decided] in ns, or why the view has none."""
    if view - 1 not in lead:
        return "no t0 anchor"
    leader = trace[lead[view][0]][view]
    cert1 = [v[view][CERT1] for v in trace.values() if CERT1 in v.get(view, {})]
    absent = [name for name in LEADER_STAMPS if name not in leader]
    if not cert1:
        absent.append(CERT1)
    if DECIDED not in leader:
        absent.append(DECIDED)
    if absent:
        return f"missing {absent[0]}"
    return [
        lead[view - 1][1],
        *(leader[name] for name in LEADER_STAMPS),
        statistics.median(cert1),
        leader[DECIDED],
    ]


def cluster_finality_ms(
    trace: Trace, warmup: int = DEFAULT_WARMUP
) -> tuple[dict[int, float], Counter[str]]:
    """View to the time from the leader's proposal_queued until a quorum of 2N/3 + 1 of the N
    nodes in the trace had decided it, and the counts of views left out by reason. A node writes
    one `leaf_decided` row per decide, for the newest decided view only, so this covers just the
    views that were the newest in a decide."""
    quorum = len(trace) * 2 // 3 + 1
    lead = leaders(trace)
    decisions: dict[int, list[int]] = {}
    for views in trace.values():
        for view, events in views.items():
            if DECIDED in events:
                decisions.setdefault(view, []).append(events[DECIDED])
    finality: dict[int, float] = {}
    skipped: Counter[str] = Counter()
    for view in sorted(v for v in lead.keys() | decisions.keys() if v >= warmup):
        if view not in lead:
            skipped["no leader"] += 1
        elif view not in decisions:
            skipped["no decision"] += 1
        elif len(decisions[view]) < quorum:
            skipped["below quorum"] += 1
        else:
            decided = sorted(decisions[view])[quorum - 1]
            finality[view] = (decided - lead[view][1]) / NS_PER_MS
    return finality, skipped
