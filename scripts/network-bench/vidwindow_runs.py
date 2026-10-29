"""Real runs next to `vidwindow`: the compute of every step fitted from the traces, and the
window, view time and finality of each load step against the model at the same bandwidth,
block size and compute.

For every view of a run's leader traces (`<run>/hosts/*/trace/leader_trace_node*.csv`): the
decode time, each node's window from its k-th share of block V to its validation of proposal
V+1, whether its decode outran that window, the model-equivalent critical window built from
the real timestamps, each node's region and the latency matrix (votes placed at validation, as
if reconstruction were instant), the proposal's transit to the next leader and the time to a
quorum of deciders. Views are grouped by the steps of `<run>/steps.json`; the block of a view
comes from `load.jsonl` (1 MB transactions per height) and the node logs (view -> height).

`fit` prints each step's compute per load step and the line fitted through them, which is what
a `Compute` preset is. `compare` (the default) runs the model with that fit, a named preset
(`--compute c8g.4xlarge`) or none (`--compute off`); `--model formula` uses the closed forms of
`vidformula` instead of the simulation.

    vidwindow_runs.py bench-state/aws/<fleet>/runs/<run> ... [--gbps 15] [--out table.json]
    vidwindow_runs.py fit bench-state/aws/<fleet>/runs/<run>

Without `--gbps` the model runs at the nominal rate of the fleet's instance type.
"""

import argparse
import collections
import csv
import glob
import gzip
import json
import math
import re
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
import latency
import vidformula
import vidwindow
from vidwindow import Compute, Cost

MODELS = {"sim": vidwindow.point, "formula": vidformula.point}

NOMINAL_GBPS = {
    "c8g.8xlarge": 15.0,
    "c8g.4xlarge": 15.0,
    "c8g.2xlarge": 15.0,
    "c8g.xlarge": 12.5,
    "c8g.large": 12.5,
}
MODEL_VIEWS = 200
MS = 1e6  # ns per ms

# (segment, start event, end event), on one node and view unless noted. The leader's events
# of view V sit under V; a node's reconstruction of block V too (it gates its vote1 on V+1).
LEADER_SEGMENTS = (
    ("build", "request_block_header_queued", "header_created_applied"),
    ("encode", "ns_disperse_start", "ns_disperse_end"),
    ("propose", "leaf2_commit_computed", "proposal_signed"),
    ("persist_proposal", "proposal_queued", "proposal_broadcast_start"),
    ("verify_proposal", "proposal_broadcast_end", "proposal_validated_v_minus_1"),
)
NODE_SEGMENTS = (
    ("decode", "recover_v_minus_1_start", "recover_v_minus_1_end"),
    ("vote2", "vote2_v_minus_1_queued", "vote2_v_minus_1_broadcast_start"),
    ("decide", "cert2_v_minus_1_input_dispatched", "leaf_decided"),
)
SEGMENTS = [s[0] for s in LEADER_SEGMENTS + NODE_SEGMENTS] + ["vote1", "cert"]


def compare(
    run: Path, gbps: float | None, compute_name: str, model: str = "sim"
) -> list[dict]:
    r = Run.load(run)
    steps = [s for s in (r.step_stats(st) for st in r.steps) if s is not None]
    if compute_name == "fit":
        compute = fit_compute(steps)
    else:
        compute = vidwindow.COMPUTE[compute_name]
    rows = []
    for s in steps:
        row = {
            "run": str(run),
            "node_type": r.node_type,
            "profile": r.profile,
            "compute": compute_name,
            "engine": model,
            **s,
        }
        g = gbps or NOMINAL_GBPS.get(r.node_type, 15.0)
        if s["mb_p50"] > 0:
            m = MODELS[model](
                r.profile, r.n, g, s["mb_p50"], r.k, MODEL_VIEWS, compute=compute
            )
            row["model"] = {
                key: m[key]
                for key in (
                    "gbps",
                    "win_p50",
                    "crit_p10",
                    "crit_p50",
                    "late_pct",
                    "view_p50",
                    "transit_p50",
                    "fin_p50",
                )
            }
        rows.append(row)
    return rows


def fit_compute(steps: list[dict]) -> Compute:
    """A `Compute` from the per-step medians: each segment as a line in the block size."""
    costs = {}
    for name in SEGMENTS:
        pts = [(s["mb_p50"], s["seg"][name]) for s in steps if name in s["seg"]]
        costs[name] = line(pts)
    return Compute(**costs)


def line(pts: list[tuple[float, float]]) -> Cost:
    """Least squares `fixed + per_mb × mb` through (mb, ms) points; one point is a constant,
    and a negative intercept is folded into the slope at the mean block."""
    pts = [(x, y) for x, y in pts if not (math.isnan(x) or math.isnan(y))]
    if not pts:
        return Cost()
    xs, ys = np.array([p[0] for p in pts]), np.array([p[1] for p in pts])
    if len(pts) < 2 or np.ptp(xs) == 0:
        return Cost(round(float(ys.mean()), 2))
    slope, fixed = np.polyfit(xs, ys, 1)
    if fixed < 0:
        fixed, slope = 0.0, float(ys.mean() / xs.mean())
    return Cost(round(float(fixed), 2), round(float(slope), 3))


class Run:
    """A run's traces, block sizes, steps and geography."""

    def __init__(self, node_type, trace, v2h, by_h, steps, region):
        self.node_type, self.trace, self.steps = node_type, trace, steps
        self.profile = "decaf-2025" if region else "off"
        self.n = len(trace)
        self.k, self.q = -(-self.n // 3), -(-2 * self.n // 3)
        self.d = delay_matrix(self.profile, region, self.n)
        self.leader_of = {
            v: nid
            for nid, views in trace.items()
            for v, ev in views.items()
            if "proposal_queued" in ev
        }
        # The logs name a height every few views; without timeouts height - view is constant.
        offs = sorted(h - v for v, h in v2h.items())
        off = offs[len(offs) // 2] if offs else 0
        self.block_mb = lambda v: by_h.get(v2h.get(v, v + off), 0)

    @classmethod
    def load(cls, run: Path) -> "Run":
        man = json.loads((run / "manifest.json").read_text())
        argv = man.get("fleet_argv") or []
        node_type = (
            man["config"].get("node_type") or argv[argv.index("--node-type") + 1]
        )
        region_of_host = (man.get("latency") or {}).get("assignment", {})
        trace, host_of = {}, {}
        for p in glob.glob(str(run / "hosts/node*/trace/leader_trace_node*.csv")):
            host = p.split("/hosts/")[1].split("/")[0]
            found = re.search(r"leader_trace_node(\d+)", p)
            if found is None:
                continue
            nid = int(found.group(1))
            host_of[nid] = host
            with open(p, newline="") as f:
                r = csv.reader(f)
                next(r)
                for view, node, ev, ts in r:
                    trace.setdefault(int(node), {}).setdefault(
                        int(view), {}
                    ).setdefault(ev, []).append(int(ts))
        v2h = {}
        for p in sorted(glob.glob(str(run / "hosts/node*/espresso-node.log.gz")))[:3]:
            with gzip.open(p, "rt") as f:
                for line_ in f:
                    if "validated and applied new header" not in line_:
                        continue
                    try:
                        span = json.loads(line_)["span"]
                        v2h[int(span["view"][11:-1])] = int(span["height"])
                    except (ValueError, KeyError):
                        pass
        by_h = collections.Counter()
        for line_ in (run / "load.jsonl").read_text().splitlines():
            t = json.loads(line_)
            if t.get("status") == "included":
                by_h[t["height"]] += 1
        steps = json.loads((run / "steps.json").read_text())
        region = {
            nid: region_of_host[h] for nid, h in host_of.items() if h in region_of_host
        }
        return cls(node_type, trace, v2h, by_h, steps, region)

    def events(self, node: int, view: int) -> dict[str, list[int]]:
        return self.trace.get(node, {}).get(view, {})

    def step_stats(self, st: dict) -> dict | None:
        """Medians over the views whose proposal left during the step."""
        t0, t1 = st["t_start"] * 1e9, st["t_end"] * 1e9
        seg: dict[str, list[float]] = collections.defaultdict(list)
        dec, win, late, crit, mbs, vts, fins, transit = [], [], [], [], [], [], [], []
        prev = None
        for v in sorted(self.leader_of):
            leader = self.leader_of[v]
            ev = self.events(leader, v)
            pq = ev["proposal_queued"][0]
            if not (t0 <= pq <= t1):
                continue
            if prev is not None and v - 1 in self.leader_of:
                vts.append((pq - prev) / MS)
            prev = pq
            mbs.append(self.block_mb(v))
            self.leader_segments(v, leader, seg)
            nxt = self.events(self.leader_of.get(v + 1, -1), v + 1)
            if "request_block_header_queued" in nxt:
                transit.append((nxt["request_block_header_queued"][0] - pq) / MS)
            ws, pvs = {}, {}
            for i in range(self.n):
                e, nx = self.events(i, v), self.events(i, v + 1)
                self.node_segments(e, seg)
                if "recover_v_minus_1_start" in e and "recover_v_minus_1_end" in e:
                    dec.append(
                        (
                            e["recover_v_minus_1_end"][0]
                            - e["recover_v_minus_1_start"][0]
                        )
                        / MS
                    )
                if i == leader or "threshold_share_reached_v_minus_1" not in e:
                    continue
                if "proposal_validated_v_minus_1" not in nx:
                    continue
                pvs[i] = nx["proposal_validated_v_minus_1"][0]
                ws[i] = (pvs[i] - e["threshold_share_reached_v_minus_1"][0]) / MS
                win.append(ws[i])
                if "recover_v_minus_1_end" in e:
                    late.append(e["recover_v_minus_1_end"][0] > pvs[i])
            c = self.critical(v, leader, ws, pvs)
            if c is not None:
                crit.append(c)
            f = self.finality(v, pq)
            if f is not None:
                fins.append(f)
        if not win:
            return None
        pct = vidwindow.pct
        return {
            "step_mb_s": st["rate_mb_s"],
            "passed": not st.get("consensus_fails") and not st.get("query_fails"),
            "decided_mb_s": round(st.get("decided_mb_s", 0.0), 1),
            "views": len(vts),
            "mb_p50": pct(mbs, 0.5),
            "view_p50": pct(vts, 0.5),
            "fin_p50": pct(fins, 0.5),
            "transit_p50": pct(transit, 0.5),
            "decode_p50": pct(dec, 0.5),
            "decode_p90": pct(dec, 0.9),
            "win_p10": pct(win, 0.1),
            "win_p50": pct(win, 0.5),
            "late_pct": round(100.0 * sum(late) / max(1, len(late)), 1),
            "crit_p10": pct(crit, 0.1),
            "crit_p50": pct(crit, 0.5),
            "seg": {name: pct(xs, 0.5) for name, xs in seg.items()},
        }

    def leader_segments(self, v: int, leader: int, seg: dict[str, list[float]]):
        ev = self.events(leader, v)
        for name, a, b in LEADER_SEGMENTS:
            if a in ev and b in ev:
                seg[name].append((ev[b][0] - ev[a][0]) / MS)
        # The vote1 compute of a node that had the parent already: from pairing to the vote
        # leaving, with the parent's recovery (under V-1) done before pairing.
        for i in range(self.n):
            e, prev = self.events(i, v), self.events(i, v - 1)
            if (
                "proposal_validated_v_minus_1" in e
                and "vote1_broadcast_start" in e
                and "recover_v_minus_1_end" in prev
                and prev["recover_v_minus_1_end"][0]
                <= e["proposal_validated_v_minus_1"][0]
            ):
                seg["vote1"].append(
                    (
                        e["vote1_broadcast_start"][0]
                        - e["proposal_validated_v_minus_1"][0]
                    )
                    / MS
                )
            # Cert1 from the q-th vote1 to arrive.
            arrivals = sorted(e.get("vote1_v_minus_1_arrived", []))
            if len(arrivals) >= self.q and "cert1_v_minus_1_input_dispatched" in e:
                seg["cert"].append(
                    (e["cert1_v_minus_1_input_dispatched"][0] - arrivals[self.q - 1])
                    / MS
                )

    @staticmethod
    def node_segments(e: dict[str, list[int]], seg: dict[str, list[float]]):
        for name, a, b in NODE_SEGMENTS:
            if a in e and b in e:
                seg[name].append((e[b][0] - e[a][0]) / MS)

    def critical(self, v, leader, ws, pvs):
        """The model's critical window from real timestamps: every vote1(V+1) placed at its
        node's validation of proposal V+1 and delayed to the leader of V+2 by the matrix."""
        nxt = self.leader_of.get(v + 2)
        if nxt is None:
            return None
        arrivals = {i: pvs[i] / 1e9 + self.d[i, nxt] for i in pvs}
        own = self.events(leader, v + 1).get("proposal_validated_v_minus_1")
        if own is not None:
            arrivals[leader] = own[0] / 1e9 + self.d[leader, nxt]
        if len(arrivals) < self.q:
            return None
        cert = sorted(arrivals.values())[self.q - 1]
        cs = []
        for i in range(self.n):
            if i == leader:
                cs.append(float("inf"))
            elif i not in arrivals or arrivals[i] > cert + 1e-9:
                cs.append(float("-inf"))
            else:
                cs.append(max(ws[i], 0.0) / 1000.0 + (cert - arrivals[i]))
        cs.sort(reverse=True)
        return cs[self.q - 1] * 1000.0

    def finality(self, v: int, pq: int) -> float | None:
        """From the proposal leaving to the q-th node's decide of it."""
        decided = sorted(
            self.events(i, v)["leaf_decided"][0]
            for i in range(self.n)
            if "leaf_decided" in self.events(i, v)
        )
        if len(decided) < self.q:
            return None
        return (decided[self.q - 1] - pq) / MS


def delay_matrix(profile, region, n):
    d = np.full((n, n), vidwindow.FLAT_MS / 1000.0)
    np.fill_diagonal(d, 0.0)
    if profile == "off":
        return d
    prof = latency.load_profile(profile)
    for i in range(n):
        for j in range(n):
            if i != j:
                a, b = region[i], region[j]
                d[i, j] = (
                    prof.cross_ms[(a, b)] if a != b else prof.intra_ms[a]
                ) / 2000.0
    return d


def print_rows(rows):
    for r in rows:
        m = r.get("model", {})
        print(
            f"{r['node_type']:12} {r['profile']:10} step {r['step_mb_s']:5.0f} {'ok ' if r['passed'] else 'F  '}"
            f" blk {r['mb_p50']:5.0f} MB view {r['view_p50']:5.0f} fin {r['fin_p50']:5.0f} transit {r['transit_p50']:4.0f}"
            f" decode {r['decode_p50']:5.1f}/{r['decode_p90']:5.1f}"
            f" window p10/p50 {r['win_p10']:5.0f}/{r['win_p50']:5.0f} late {r['late_pct']:4.0f}%"
            f" critical p10/p50 {r['crit_p10']:5.0f}/{r['crit_p50']:5.0f}"
            + (
                f" | {r['engine']} {m['gbps']:g} Gbps {r['compute']}: view {m['view_p50']:5.0f} fin {m['fin_p50']:5.0f} transit {m['transit_p50']:4.0f}"
                f" window {m['win_p50']:5.0f} late {m['late_pct']:3.0f}% critical {m['crit_p10']:4.0f}/{m['crit_p50']:4.0f}"
                if m
                else ""
            )
        )


def print_fit(run: Path):
    r = Run.load(run)
    steps = [s for s in (r.step_stats(st) for st in r.steps) if s is not None]
    print(f"{r.node_type} {r.profile}: compute per load step, median ms")
    print(
        f"{'segment':18}"
        + "".join(f"{s['step_mb_s']:>8.0f}" for s in steps)
        + "   fixed ms  ms/MB"
    )
    print(f"{'block MB':18}" + "".join(f"{s['mb_p50']:>8.0f}" for s in steps))
    compute = fit_compute(steps)
    for name in SEGMENTS:
        cost = getattr(compute, name)
        print(
            f"{name:18}"
            + "".join(f"{s['seg'].get(name, float('nan')):>8.1f}" for s in steps)
            + f"   {cost.fixed_ms:8.2f} {cost.per_mb:6.3f}"
        )


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("args", nargs="+", help="[fit] run directories")
    ap.add_argument("--gbps", type=float)
    ap.add_argument(
        "--compute",
        default="fit",
        choices=["fit", *sorted(vidwindow.COMPUTE)],
        help="the model's compute: fitted from the run's traces, a preset, or none",
    )
    ap.add_argument(
        "--model",
        default="sim",
        choices=sorted(MODELS),
        help="the fluid simulation of vidwindow, or the formulas of vidformula",
    )
    ap.add_argument("--out", type=Path)
    args = ap.parse_args(argv)
    if args.args[0] == "fit":
        for run in args.args[1:]:
            print_fit(Path(run))
        return
    rows = [
        r
        for run in args.args
        for r in compare(Path(run), args.gbps, args.compute, args.model)
    ]
    print_rows(rows)
    if args.out:
        args.out.write_text(json.dumps(rows, indent=1))


if __name__ == "__main__":
    main()
