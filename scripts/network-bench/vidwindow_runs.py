"""The reconstruction window of real runs, per load step, next to `vidwindow` at the same
bandwidth and block size.

For every view of a run's leader traces (`<run>/hosts/*/trace/leader_trace_node*.csv`): the
decode time, each node's window from its k-th share of block V to its validation of proposal
V+1, whether its decode outran that window, and the model-equivalent critical window built from
the real timestamps, each node's region and the latency matrix (votes placed at validation, as
if reconstruction were instant). Views are grouped by the steps of `<run>/steps.json`; the
median block of a step comes from the node logs (view -> height) and `load.jsonl` (1 MB
transactions per height).

    vidwindow_runs.py bench-state/aws/<fleet>/runs/<run> ... [--gbps 15] [--out table.json]

Without `--gbps` the model runs at the nominal rate of the fleet's instance type.
"""

import argparse
import collections
import csv
import glob
import gzip
import json
import re
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
import latency
import vidwindow

NOMINAL_GBPS = {
    "c8g.8xlarge": 15.0,
    "c8g.4xlarge": 15.0,
    "c8g.2xlarge": 15.0,
    "c8g.xlarge": 12.5,
    "c8g.large": 12.5,
}
MODEL_VIEWS = 200


def compare(run: Path, gbps: float | None) -> list[dict]:
    node_type, trace, v2h, by_h, steps, region = load_run(run)
    profile = "decaf-2025" if region else "off"
    n = len(trace)
    k, q = -(-n // 3), -(-2 * n // 3)
    d = delay_matrix(profile, region, n)
    leader_of = {
        v: nid
        for nid, views in trace.items()
        for v, ev in views.items()
        if "proposal_queued" in ev
    }
    rows = []
    for st in steps:
        s = step_stats(
            trace,
            leader_of,
            v2h,
            by_h,
            d,
            n,
            k,
            q,
            st["t_start"] * 1e9,
            st["t_end"] * 1e9,
        )
        if s is None:
            continue
        row = {
            "run": str(run),
            "node_type": node_type,
            "profile": profile,
            "step_mb_s": st["rate_mb_s"],
            "passed": not st.get("consensus_fails") and not st.get("query_fails"),
            "decided_mb_s": round(st.get("decided_mb_s", 0.0), 1),
            **s,
        }
        g = gbps or NOMINAL_GBPS.get(node_type, 15.0)
        if s["mb_p50"] > 0:
            m = vidwindow.point(profile, n, g, s["mb_p50"], k, MODEL_VIEWS)
            row["model"] = {
                key: m[key]
                for key in ("gbps", "win_p50", "crit_p10", "crit_p50", "view_p50")
            }
        rows.append(row)
    return rows


def step_stats(
    trace, leader_of, v2h, by_h, d, n, k, q, t0, t1
) -> dict[str, float] | None:
    dec, win, late, crit, mbs, vts = [], [], [], [], [], []
    prev = None
    for v in sorted(leader_of):
        leader = leader_of[v]
        pq = trace[leader][v]["proposal_queued"]
        if not (t0 <= pq <= t1):
            continue
        if prev is not None and v - 1 in leader_of:
            vts.append((pq - prev) / 1e6)
        prev = pq
        if v in v2h:
            mbs.append(by_h.get(v2h[v], 0))
        ws, pvs = {}, {}
        for i in range(n):
            ev, nx = trace.get(i, {}).get(v, {}), trace.get(i, {}).get(v + 1, {})
            if "recover_v_minus_1_start" in ev and "recover_v_minus_1_end" in ev:
                dec.append(
                    (ev["recover_v_minus_1_end"] - ev["recover_v_minus_1_start"]) / 1e6
                )
            if i == leader or "threshold_share_reached_v_minus_1" not in ev:
                continue
            if "proposal_validated_v_minus_1" not in nx:
                continue
            pvs[i] = nx["proposal_validated_v_minus_1"]
            ws[i] = (pvs[i] - ev["threshold_share_reached_v_minus_1"]) / 1e6
            win.append(ws[i])
            if "recover_v_minus_1_end" in ev:
                late.append(ev["recover_v_minus_1_end"] > pvs[i])
        c = critical(trace, leader_of, d, n, q, v, leader, ws, pvs)
        if c is not None:
            crit.append(c)
    if not win:
        return None
    pct = vidwindow.pct
    return {
        "views": len(vts),
        "mb_p50": pct(mbs, 0.5),
        "view_p50": pct(vts, 0.5),
        "decode_p50": pct(dec, 0.5),
        "decode_p90": pct(dec, 0.9),
        "win_p10": pct(win, 0.1),
        "win_p50": pct(win, 0.5),
        "late_pct": round(100.0 * sum(late) / max(1, len(late)), 1),
        "crit_p10": pct(crit, 0.1),
        "crit_p50": pct(crit, 0.5),
    }


def critical(trace, leader_of, d, n, q, v, leader, ws, pvs):
    """The model's critical window from real timestamps: every vote1(V+1) placed at its node's
    validation of proposal V+1 and delayed to the leader of V+2 by the matrix."""
    nxt = leader_of.get(v + 2)
    if nxt is None:
        return None
    arrivals = {i: pvs[i] / 1e9 + d[i, nxt] for i in pvs}
    own = trace.get(leader, {}).get(v + 1, {}).get("proposal_validated_v_minus_1")
    if own is not None:
        arrivals[leader] = own / 1e9 + d[leader, nxt]
    if len(arrivals) < q:
        return None
    cert = sorted(arrivals.values())[q - 1]
    cs = []
    for i in range(n):
        if i == leader:
            cs.append(float("inf"))
        elif i not in arrivals or arrivals[i] > cert + 1e-9:
            cs.append(float("-inf"))
        else:
            cs.append(max(ws[i], 0.0) / 1000.0 + (cert - arrivals[i]))
    cs.sort(reverse=True)
    return cs[q - 1] * 1000.0


def load_run(run: Path):
    man = json.loads((run / "manifest.json").read_text())
    argv = man.get("fleet_argv") or []
    node_type = man["config"].get("node_type") or argv[argv.index("--node-type") + 1]
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
                trace.setdefault(int(node), {}).setdefault(int(view), {}).setdefault(
                    ev, int(ts)
                )
    v2h = {}
    for p in sorted(glob.glob(str(run / "hosts/node*/espresso-node.log.gz")))[:3]:
        with gzip.open(p, "rt") as f:
            for line in f:
                if "validated and applied new header" not in line:
                    continue
                try:
                    span = json.loads(line)["span"]
                    v2h[int(span["view"][11:-1])] = int(span["height"])
                except (ValueError, KeyError):
                    pass
    by_h = collections.Counter()
    for line in (run / "load.jsonl").read_text().splitlines():
        t = json.loads(line)
        if t.get("status") == "included":
            by_h[t["height"]] += 1
    steps = json.loads((run / "steps.json").read_text())
    region = {
        nid: region_of_host[h] for nid, h in host_of.items() if h in region_of_host
    }
    return node_type, trace, v2h, by_h, steps, region


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
            f" blk {r['mb_p50']:5.0f} MB view {r['view_p50']:5.0f} ms decode {r['decode_p50']:6.1f}/{r['decode_p90']:6.1f}"
            f" window p10/p50 {r['win_p10']:5.0f}/{r['win_p50']:5.0f} late {r['late_pct']:4.0f}%"
            f" critical p10/p50 {r['crit_p10']:5.0f}/{r['crit_p50']:5.0f}"
            + (
                f" | model {m['gbps']:g} Gbps: window {m['win_p50']:5.0f} critical {m['crit_p10']:4.0f}/{m['crit_p50']:4.0f} view {m['view_p50']:5.0f}"
                if m
                else ""
            )
        )


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("runs", nargs="+", type=Path)
    ap.add_argument("--gbps", type=float)
    ap.add_argument("--out", type=Path)
    args = ap.parse_args(argv)
    rows = [r for run in args.runs for r in compare(run, args.gbps)]
    print_rows(rows)
    if args.out:
        args.out.write_text(json.dumps(rows, indent=1))


if __name__ == "__main__":
    main()
