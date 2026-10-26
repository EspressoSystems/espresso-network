"""Leader critical-path breakdown of consensus views from the per-node `leader_trace_node{N}.csv` files that
espresso-node writes under `ESPRESSO_NODE_LEADER_TRACE_DIR`, and readers of the `cpu_node{N}.csv` and
`net_node{N}.csv` samples aws-bench writes beside them. Pure functions, stdlib only.

Every row is keyed by the subject view the event is about. The leader of view V is the node
whose trace has `proposal_queued` for V. Values that span hosts (wait to propose, finality, and
binning views into load steps by controller time) assume chrony-synced clocks."""

import csv
import itertools
from collections import Counter
from collections.abc import Iterable
from pathlib import Path

# node -> view -> event -> first ts_ns
Trace = dict[int, dict[int, dict[str, int]]]
# (t_ns, user_us, sys_us) per tick
Cpu = list[tuple[int, int, int]]
# interface -> (t_ns, rx_bytes, tx_bytes) per tick
Net = dict[str, list[tuple[int, int, int]]]

HEADER = ["view", "node_id", "event", "ts_ns"]
CPU_HEADER = ["t_ns", "kind", "id", "user_us", "sys_us"]
NET_HEADER = ["t_ns", "iface", "rx_bytes", "tx_bytes"]
# (segment, end event on the leader of V); each segment starts where the previous one ends
LEADER_PATH = (
    ("wait to propose", "request_block_header_queued"),
    ("block build", "block_built_applied"),
    ("header", "header_created_applied"),
    # The leaf commitment needs the parent's Cert1: this segment waits on the votes of V - 1,
    # it is not leader-local work.
    ("parent Cert1 wait", "leaf2_commit_computed"),
    ("commit + sign", "proposal_queued"),
    ("outbox wait", "proposal_broadcast_start"),
    ("proposal broadcast", "proposal_broadcast_end"),
    ("validate own proposal", "proposal_validated_v_minus_1"),
    ("validated -> vote1 sent", "vote1_broadcast_start"),
    ("collect vote1 -> Cert1", "cert1_v_minus_1_input_dispatched"),
    ("Cert1 -> vote2 sent", "vote2_v_minus_1_broadcast_end"),
    ("collect vote2 -> Cert2", "cert2_v_minus_1_input_dispatched"),
    ("Cert2 -> decided", "leaf_decided"),
)
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


def read_csv_rows(path: Path, header: list[str]) -> list[list[str]]:
    with path.open(newline="") as f:
        reader = csv.reader(f)
        if next(reader, None) != header:
            raise ValueError(f"{path}:1: header is not {','.join(header)}")
        return list(reader)


def read_cpu(path: Path) -> Cpu:
    """`cpu_node{N}.csv` of aws-bench's sampler: (t_ns, user_us, sys_us) per tick, the CPU time
    of the node process since the previous tick."""
    rows = []
    for t_ns, kind, _, user_us, sys_us in read_csv_rows(path, CPU_HEADER):
        if kind != "proc":
            raise ValueError(f"{path}: unexpected kind {kind!r}")
        rows.append((int(t_ns), int(user_us), int(sys_us)))
    return sorted(rows)


def read_net(path: Path) -> Net:
    """`net_node{N}.csv`: interface to (t_ns, rx_bytes, tx_bytes) per tick, the bytes since the
    previous tick."""
    ifaces: Net = {}
    for t_ns, iface, rx, tx in read_csv_rows(path, NET_HEADER):
        ifaces.setdefault(iface, []).append((int(t_ns), int(rx), int(tx)))
    return {iface: sorted(rows) for iface, rows in ifaces.items()}


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


def view_times(trace: Trace) -> dict[int, float]:
    """View to the leader's `proposal_queued` in unix seconds."""
    return {view: ts / 1e9 for view, (_, ts) in leaders(trace).items()}


def leader_path(
    trace: Trace, warmup: int = DEFAULT_WARMUP
) -> tuple[dict[int, list[float]], Counter[str]]:
    """View to the durations in ms of the `LEADER_PATH` segments, and the counts of views left
    out by reason.

    t0 is the previous leader's `proposal_queued`, so "wait to propose" is measured across hosts
    and includes clock skew. All other segments use the clock of the leader of V. Durations are
    kept as measured: events are not strictly ordered, so some segments can be negative (seen:
    Cert1 -> vote2 sent, Cert2 -> decided)."""
    lead = leaders(trace)
    path: dict[int, list[float]] = {}
    skipped: Counter[str] = Counter()
    for view in sorted(v for v in lead if v >= warmup):
        if view - 1 not in lead:
            skipped["no t0 anchor"] += 1
            continue
        events = trace[lead[view][0]][view]
        absent = [event for _, event in LEADER_PATH if event not in events]
        if absent:
            skipped[f"missing {absent[0]}"] += 1
            continue
        stamps = [lead[view - 1][1], *(events[event] for _, event in LEADER_PATH)]
        path[view] = [(b - a) / NS_PER_MS for a, b in itertools.pairwise(stamps)]
    return path, skipped


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
