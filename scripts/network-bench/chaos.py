"""Chaos mode of `aws-bench run --chaos`: the event schema of `chaos.jsonl`, the pure fault
scheduler and its probe, fault rows with their intervals, and the parts of the chaos report.

Stdlib only: `aws-bench`, `netbench.py` and `throughput-plot` import it, and the hosts get it
next to them.
"""

import json
import random
import shlex
import statistics
from collections.abc import Collection, Iterable, Mapping, Sequence
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Literal, NotRequired, TypedDict

# Up query nodes kept at every fault: one serves scans and heights while another catches up.
QUERY_FLOOR = 2
# No faults in the last seconds of the load, so recoveries finish before it ends.
TAIL_S = 120.0
LAG_BLOCKS = 2
LAG_VIEWS = 3
GATE_TICKS = 2
# Longer than SYNC_STATUS_TTL: a cached sync-status from before the rejoin has expired.
SYNC_STATUS_TTL_S = 5.0
SYNC_SETTLE_S = 2 * SYNC_STATUS_TTL_S

ChaosKind = Literal["restart", "kill", "wipe"]

# Missing blocks, leaves and vid_common of a query node's `sync-status`.
Missing = tuple[int, int, int]
MISSING_NAMES = ("blocks", "leaves", "vid_common")


@dataclass(frozen=True)
class ChaosConfig:
    minutes: int = 10
    rate_mb_s: float = 4.0
    seed: int = 42
    kinds: tuple[ChaosKind, ...] = ("restart", "kill", "wipe")
    gap_s: float = 45.0
    kill_down_s: float = 60.0
    recover_timeout_s: float = 300.0


ChaosEventKind = Literal[
    "fault", "started", "rejoined", "caught_up", "timeout", "restored"
]


class ChaosEvent(TypedDict):
    """A line of `chaos.jsonl`."""

    ts: float
    iso: str
    event: ChaosEventKind
    node: str
    kind: ChaosKind | None
    height: int | None
    after_s: float | None
    missing: NotRequired[Missing]


def chaos_event(
    ts: float,
    event: ChaosEventKind,
    node: str,
    kind: ChaosKind | None,
    height: int | None,
    after_s: float | None,
    missing: Missing | None = None,
) -> ChaosEvent:
    event_ = ChaosEvent(
        ts=ts,
        iso=datetime.fromtimestamp(ts, UTC).isoformat(),
        event=event,
        node=node,
        kind=kind,
        height=height,
        after_s=after_s,
    )
    if missing is not None:
        event_["missing"] = missing
    return event_


def read_events(run_dir: Path) -> list[ChaosEvent]:
    """Empty for a run without `chaos.jsonl`."""
    path = run_dir / "chaos.jsonl"
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text().splitlines()]


def chaos_log_line(event: ChaosEvent) -> str:
    node, after_s = event["node"], event["after_s"] or 0.0
    match event["event"]:
        case "fault":
            return f"{event['kind']} {node} at height {event['height']}"
        case "started":
            return f"{node} started after {after_s:.0f} s"
        case "rejoined":
            return f"{node} rejoined after {after_s:.0f} s"
        case "caught_up":
            return f"{node} caught up after {after_s:.0f} s"
        case "timeout":
            line = f"{node} not recovered after {after_s:.0f} s"
            if "missing" in event:
                counts = ", ".join(
                    f"{n} {c}"
                    for n, c in zip(MISSING_NAMES, event["missing"], strict=True)
                )
                line += f" (missing {counts})"
            return line
        case "restored":
            return f"{node} restored"


NodeHealth = Literal["up", "down", "recovering", "catching_up"]


class NodeState(TypedDict):
    """`since` is the fault time: gates and timeouts measure from it. `rejoined` is the time of
    the `rejoined` event. `last_missing` is the last `Missing` answered since the fault."""

    health: NodeHealth
    kind: ChaosKind | None
    since: float
    streak: int
    rejoined: float
    last_missing: Missing | None


class ChaosState(TypedDict):
    nodes: dict[str, NodeState]
    order: list[str]
    cursor: int
    faults: int
    next_fault_at: float
    stop_at: float


class ChaosObservation(TypedDict):
    """`None`: the probe got no answer. `query_height` and `missing` only have the query nodes."""

    height: dict[str, int | None]
    voted_view: dict[str, int | None]
    query_height: dict[str, int | None]
    missing: dict[str, Missing | None]


class ChaosAction(TypedDict):
    node: str
    op: Literal["restart", "kill", "start", "wipe"]


class ChaosStep(TypedDict):
    state: ChaosState
    actions: list[ChaosAction]
    events: list[ChaosEvent]
    error: str | None


def fault_budget(n: int) -> int:
    """Most nodes faulty at once with faulty stake strictly below f = (n-1)//3."""
    return max(0, (n - 1) // 3 - 1)


def chaos_init(
    cfg: ChaosConfig,
    nodes: Sequence[str],
    t_load: float,
    warmup_s: float,
    load_s: float,
) -> ChaosState:
    order = list(nodes)
    random.Random(cfg.seed).shuffle(order)
    first = t_load + warmup_s
    return ChaosState(
        nodes={
            name: NodeState(
                health="up",
                kind=None,
                since=t_load,
                streak=0,
                rejoined=t_load,
                last_missing=None,
            )
            for name in nodes
        },
        order=order,
        cursor=0,
        faults=0,
        next_fault_at=first,
        stop_at=first + load_s - TAIL_S,
    )


def chaos_step(
    state: ChaosState,
    cfg: ChaosConfig,
    queries: Collection[str],
    peers: dict[str, list[str]],
    obs: ChaosObservation,
    now: float,
) -> ChaosStep:
    """One controller tick. Pure: `state` is not modified, `now` is in the ctl epoch."""
    nodes = {name: s.copy() for name, s in state["nodes"].items()}
    events: list[ChaosEvent] = []
    actions: list[ChaosAction] = []

    def emit(
        event: ChaosEventKind,
        name: str,
        after_s: float | None = None,
        missing: Missing | None = None,
    ) -> None:
        events.append(
            chaos_event(
                now,
                event,
                name,
                nodes[name]["kind"],
                obs["height"][name],
                after_s,
                missing,
            )
        )

    for name in queries:
        if (seen := obs["missing"][name]) is not None:
            nodes[name]["last_missing"] = seen

    up = [name for name, s in nodes.items() if s["health"] == "up"]
    tip = _tip(obs["height"], up)
    tip_view = _tip(obs["voted_view"], up)
    query_tip = _tip(obs["query_height"], [name for name in up if name in queries])
    if tip is not None:
        for name, s in nodes.items():
            after_s = now - s["since"]
            if s["health"] == "down" and now >= s["since"] + cfg.kill_down_s:
                actions.append({"node": name, "op": "start"})
                s.update(health="recovering", streak=0)
                emit("started", name, after_s)
            elif s["health"] == "recovering":
                good = _near(obs["height"][name], tip, LAG_BLOCKS) and _near(
                    obs["voted_view"][name], tip_view, LAG_VIEWS
                )
                s["streak"] = s["streak"] + 1 if good else 0
                if s["streak"] >= GATE_TICKS:
                    nxt: NodeHealth = "catching_up" if name in queries else "up"
                    emit("rejoined", name, after_s)
                    s.update(
                        health=nxt,
                        kind=None if nxt == "up" else s["kind"],
                        streak=0,
                        rejoined=now,
                    )
            elif s["health"] == "catching_up" and query_tip is not None:
                good = (
                    now >= s["rejoined"] + SYNC_SETTLE_S
                    and _near(obs["query_height"][name], query_tip, LAG_BLOCKS)
                    and obs["missing"][name] == (0, 0, 0)
                )
                s["streak"] = s["streak"] + 1 if good else 0
                if s["streak"] >= GATE_TICKS:
                    emit("caught_up", name, after_s, obs["missing"][name])
                    s.update(health="up", kind=None, streak=0)

    error = None
    for name, s in nodes.items():
        if s["health"] != "up" and now >= s["since"] + cfg.recover_timeout_s:
            emit("timeout", name, now - s["since"], s["last_missing"])
            error = (
                f"{name} ({s['kind']}) not recovered within "
                f"{cfg.recover_timeout_s:.0f} s, health {s['health']}"
            )

    cursor, faults, next_fault_at = (
        state["cursor"],
        state["faults"],
        state["next_fault_at"],
    )
    if (
        tip is not None
        and error is None
        and next_fault_at <= now < state["stop_at"]
        and (i := _fault_target(state["order"], cursor, nodes, queries, peers))
        is not None
    ):
        name = state["order"][i]
        kind = cfg.kinds[faults % len(cfg.kinds)]
        nodes[name].update(
            health="down" if kind == "kill" else "recovering",
            kind=kind,
            since=now,
            streak=0,
            last_missing=None,
        )
        actions.append({"node": name, "op": kind})
        emit("fault", name)
        cursor, faults, next_fault_at = (
            (i + 1) % len(state["order"]),
            faults + 1,
            now + cfg.gap_s,
        )

    return ChaosStep(
        state=ChaosState(
            nodes=nodes,
            order=state["order"],
            cursor=cursor,
            faults=faults,
            next_fault_at=next_fault_at,
            stop_at=state["stop_at"],
        ),
        actions=actions,
        events=events,
        error=error,
    )


def _tip(values: Mapping[str, int | None], names: Iterable[str]) -> int | None:
    seen = [v for name in names if (v := values[name]) is not None]
    return max(seen, default=None)


def _near(value: int | None, tip: int | None, lag: int) -> bool:
    return value is not None and tip is not None and value >= tip - lag


def _fault_target(
    order: list[str],
    cursor: int,
    nodes: dict[str, NodeState],
    queries: Collection[str],
    peers: dict[str, list[str]],
) -> int | None:
    """Index in `order` of the first node from `cursor` on whose fault keeps the budget, the up
    query floor and 2 up peers; `None` when there is none."""
    faulty = sum(s["health"] != "up" for s in nodes.values())
    if faulty + 1 > fault_budget(len(nodes)):
        return None
    up_queries = sum(nodes[q]["health"] == "up" for q in queries)
    for step in range(len(order)):
        i = (cursor + step) % len(order)
        name = order[i]
        if nodes[name]["health"] != "up":
            continue
        if up_queries - (name in queries) < QUERY_FLOOR:
            continue
        if sum(nodes[p]["health"] == "up" for p in peers[name]) < 2:
            continue
        return i
    return None


# `is_fully_synced` is a method of `SyncStatusQueryData`, not a serialized field: it holds when
# all three counts are 0.
MISSING_JQ = '"' + ",".join(rf"\(.{name}.missing)" for name in MISSING_NAMES) + '"'


def probe_command(urls: Mapping[str, str], queries: Collection[str]) -> str:
    """Shell run on ctl: one background job per node of `urls` printing `<node> <height> <voted
    view>`, plus `<query height> <blocks,leaves,vid_common missing>` for query nodes; `-` for a
    field that did not answer.
    Nodes run in parallel so a hung node costs one curl timeout, not one per field."""
    lines = []
    for name, url in urls.items():
        base = shlex.quote(url)
        height = f"$(curl -sf -m 2 {base}/v1/status/block-height)"
        view = (
            f"$(curl -sf -m 2 {base}/v1/status/metrics | "
            "awk '$1==\"consensus_last_voted_view\"{print $2}')"
        )
        fields = ["${h:--}", "${v:--}"]
        script = f"h={height}; v={view}; "
        if name in queries:
            script += (
                f"q=$(curl -sf -m 2 {base}/v1/node/block-height); "
                f"s=$(curl -sf -m 2 {base}/v1/node/sync-status | jq -r {shlex.quote(MISSING_JQ)}); "
            )
            fields += ["${q:--}", "${s:--}"]
        lines.append(f'({script}echo "{name} {" ".join(fields)}") &')
    return "\n".join([*lines, "wait"])


def parse_probe(
    stdout: str, nodes: Iterable[str], queries: Collection[str]
) -> ChaosObservation:
    """Raises `ValueError` for a node without a line or with missing fields."""
    rows = {
        line.split()[0]: line.split() for line in stdout.splitlines() if line.strip()
    }
    obs = ChaosObservation(height={}, voted_view={}, query_height={}, missing={})
    for name in nodes:
        fields = rows.get(name)
        if fields is None or len(fields) != (5 if name in queries else 3):
            raise ValueError(
                f"probe output has no complete line for {name}: {stdout!r}"
            )
        obs["height"][name] = _probe_int(fields[1])
        obs["voted_view"][name] = _probe_int(fields[2])
        if name in queries:
            obs["query_height"][name] = _probe_int(fields[3])
            obs["missing"][name] = _probe_missing(fields[4])
    return obs


def _probe_missing(field: str) -> Missing | None:
    if field == "-":
        return None
    blocks, leaves, vid_common = map(int, field.split(","))
    return blocks, leaves, vid_common


def _probe_int(field: str) -> int | None:
    return None if field == "-" else int(float(field))


@dataclass(slots=True)
class FaultRow:
    """One fault and the time of each later event of its node; `None` while it has not
    happened. `missing` is the last count an event of the node carried."""

    node: str
    kind: ChaosKind
    fault: float
    started: float | None = None
    rejoined: float | None = None
    caught_up: float | None = None
    timeout: float | None = None
    restored: float | None = None
    missing: Missing | None = None


def fault_rows(events: Iterable[ChaosEvent]) -> list[FaultRow]:
    """One row per fault, in the order of the faults."""
    rows: list[FaultRow] = []
    latest: dict[str, FaultRow] = {}
    for e in events:
        if e["event"] == "fault":
            if e["kind"] is None:
                raise ValueError(f"fault event without a kind: {e}")
            latest[e["node"]] = FaultRow(e["node"], e["kind"], e["ts"])
            rows.append(latest[e["node"]])
            continue
        row, ts = latest[e["node"]], e["ts"]
        match e["event"]:
            case "started":
                row.started = ts
            case "rejoined":
                row.rejoined = ts
            case "caught_up":
                row.caught_up = ts
            case "timeout":
                row.timeout = ts
            case "restored":
                row.restored = ts
        if "missing" in e:
            row.missing = e["missing"]
    return rows


Phase = Literal["down", "recovering", "catching up"]


def fault_windows(
    rows: Iterable[FaultRow], end: float
) -> list[tuple[str, ChaosKind, float, float]]:
    """(node, kind, start, stop) per fault: from the fault to the rejoin, else to the timeout,
    else to `end`."""
    return [(r.node, r.kind, r.fault, _first(r.rejoined, r.timeout, end)) for r in rows]


def max_concurrent_faulty(
    rows: Sequence[FaultRow], events: Sequence[ChaosEvent]
) -> int:
    """Most rows open at once; a row is open from its fault to its catch-up, or to its rejoin
    when it never catches up (a validator), or to the last event."""
    end = max((e["ts"] for e in events), default=0.0)
    spans = [(r.fault, _first(r.caught_up, r.rejoined, end)) for r in rows]
    return max((sum(a <= start < b for a, b in spans) for start, _ in spans), default=0)


def phase_segments(row: FaultRow, end: float) -> list[tuple[Phase, float, float]]:
    """(phase, start, stop) of a fault: down until the node starts (a kill only), recovering
    until it rejoins, catching up until its query service is synced (a query node only). A
    phase still open at the timeout, else at `end`, runs to there."""
    stop = _first(row.timeout, end)
    segments: list[tuple[Phase, float, float]] = []
    if row.started is not None:
        segments.append(("down", row.fault, row.started))
    recovering = _first(row.started, row.fault)
    segments.append(("recovering", recovering, _first(row.rejoined, stop)))
    if row.rejoined is not None and row.caught_up is not None:
        segments.append(("catching up", row.rejoined, row.caught_up))
    elif row.rejoined is not None and row.timeout is not None:
        segments.append(("catching up", row.rejoined, stop))
    return segments


def _first(*times: float | None) -> float:
    """The first time that is not `None`; the last one never is."""
    return next(t for t in times if t is not None)


class StepChaos(TypedDict):
    """What the faults did to one load step."""

    start_s: float
    # Faults active during the step, as "node kind".
    faults: str
    # Nodes whose process was stopped or started during the step: their CPU counter reset.
    restarted: frozenset[str]


class ChaosView(TypedDict):
    """The chaos report of `render`: `lead` replaces the headline, `faults` follows the chart,
    `steps` has one entry per step of the result."""

    lead: list[str]
    faults: list[str]
    steps: list[StepChaos]


def step_chaos(
    spans: Iterable[tuple[float, float]], rows: list[FaultRow], t0: float, end: float
) -> list[StepChaos]:
    """Per step, given as its (start, end): the faults active in it, and the nodes whose process
    stopped or started. `start_s` counts from `t0`."""
    windows = fault_windows(rows, end)
    restarts = [
        (r.node, ts) for r in rows for ts in (r.fault, r.started) if ts is not None
    ]
    return [
        {
            "start_s": start - t0,
            "faults": ", ".join(
                f"{node} {kind}"
                for node, kind, a, b in windows
                if a < stop and b > start
            ),
            "restarted": frozenset(node for node, ts in restarts if start <= ts < stop),
        }
        for start, stop in spans
    ]


def chaos_verdict(
    rows: list[FaultRow],
    events: Sequence[ChaosEvent],
    error: str | None,
    failed: bool,
) -> str:
    reasons = [
        f"{e['node']} ({e['kind']}) not recovered after {e['after_s']:.0f} s"
        for e in events
        if e["event"] == "timeout"
    ]
    reasons += [
        f"{r.node} ({r.kind}) did not rejoin"
        for r in rows
        if r.rejoined is None and r.timeout is None
    ]
    if not reasons and failed:
        reasons.append(error or "the run produced no result")
    if reasons:
        return f"- Verdict: **fail**: {'; '.join(reasons)}"
    return "- Verdict: **pass**: every fault recovered, no timeout"


def recovery_table(rows: list[FaultRow], kinds: Sequence[str]) -> list[str]:
    """Seconds from the fault to the `rejoined` and `caught_up` events, per kind; only query
    nodes catch up."""

    def spread(took: list[float]) -> tuple[str, str]:
        if not took:
            return "-", "-"
        return f"{statistics.median(took):.0f}", f"{max(took):.0f}"

    lines = [
        "| kind | faults | rejoin p50 (s) | rejoin max (s) | caught up p50 (s) | caught up max (s) |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for kind in kinds:
        of_kind = [r for r in rows if r.kind == kind]
        rejoin = spread(
            [r.rejoined - r.fault for r in of_kind if r.rejoined is not None]
        )
        caught = spread(
            [r.caught_up - r.fault for r in of_kind if r.caught_up is not None]
        )
        lines.append(f"| {kind} | {len(of_kind)} | {' | '.join((*rejoin, *caught))} |")
    return lines


def fault_table(rows: list[FaultRow], t0: float) -> list[str]:
    """One row per fault with the seconds since `t0` of each step of its recovery; the counts
    still missing at a timeout, when there was one."""

    def cell(ts: float | None) -> str:
        return "-" if ts is None else f"{ts - t0:.0f}"

    def missing(row: FaultRow) -> str:
        if row.missing is None:
            return "-"
        counts = zip(MISSING_NAMES, row.missing, strict=True)
        return ", ".join(f"{name} {count}" for name, count in counts)

    if not rows:
        return []
    show = any(r.missing is not None for r in rows)
    return [
        "### Faults",
        "",
        "| node | kind | at (s) | started (s) | rejoined (s) | caught up (s) |"
        + (" missing |" if show else ""),
        "|---|---|---|---|---|---|" + ("---|" if show else ""),
        *(
            f"| {r.node} | {r.kind} | {cell(r.fault)} | {cell(r.started)} | "
            f"{cell(r.rejoined)} | {cell(r.caught_up)} |"
            + (f" {missing(r)} |" if show else "")
            for r in rows
        ),
        "",
    ]
