"""One fast-finality view as formulas over the latency matrix, the bandwidth and the compute.

No event simulation: every quantity is an order statistic over the matrix plus a few sums. The
protocol is the one `vidwindow.py` states; this module is its closed form, and `vidwindow` the
oracle it is checked against (`tests/test_vidformula.py`).

Per leader pair (L of V, L' of V+1), with T the share transfer (n - 1) / k × P / B and d the
one-way delays:

    Q          = q-th smallest over voters x of d(L, x) + d(x, L')        quorum round trip
    delta_j    = d(L, L') + d(L', j) - k-th smallest over x of d(L, x) + d(x, j)
    window_j   = T + delta_j + verify_proposal                             where transfers bind
               = Q + cert + propose + persist_proposal + verify_proposal
                 + d(L', j) - kth_j                                        where latency binds
    view       = max(Q + verify_proposal + vote1 + cert + propose + persist_proposal,
                     verify_proposal + max(vote1 + T, build + encode) + T + d(L, L'))
    finality   = q-th_i [ q-th_j [ cert1_j + vote2 + d(j, i) ] + cert + decide ]
    cert1_j    = q-th_x [ d(L, x) + verify_proposal + vote1 + d(x, j) ] + cert

delta_j is the detour the proposal of V+1 takes through L' against the k shares of V that
reach j directly or by rebroadcast; `vidwindow` found it by simulation, here it is read off the
matrix. It is the window's floor: three quarters of the node-views sit on it, the rest above
it by whole hops, when the proposal crossed behind more of the leader's queue. Consecutive views alternate around the pair's view, because the next leader gets the
proposal behind the previous rebroadcast and builds late, so the pair formulas hold for the
mean and `run_views` evaluates the actual sequence: a recurrence over views, each view's
timestamps from the previous view's, every node at once. It takes every flow of a transfer to
finish together, T after it starts; the fluid model of `vidwindow` lets flows to idle receivers
finish first, so where bandwidth binds its k-th shares arrive earlier and its windows and
finality come out shorter, by up to a third at large blocks, and its proposals cross while the shares still
drain (its `transit` is shorter). The view's mean agrees within a few percent.

    vidformula.py point --profile decaf-2025 --nodes 100 --gbps 15 --block 50 --compute c8g.4xlarge
    vidformula.py point --profile off --nodes 10 --gbps 15 --rate 300 --compute c8g.4xlarge
    vidformula.py sweep --profile off --nodes 10 --gbps 15 --rates 200,250,300,350 --compute c8g.4xlarge
    vidformula.py point --profile off --nodes 10 --gbps 15 --block 92 --check     # next to the sim
    vidformula.py pairs --profile decaf-2025 --nodes 100 --gbps 15 --block 50
"""

import argparse
import json
import random
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
import vidwindow
from vidwindow import COMPUTE, ZERO, Compute, closed_form, delays, pct, threshold

# ---- per leader pair ----


def quorum_rtt(d: np.ndarray, lead: int, nxt: int, q: int) -> float:
    """Q in ms: the q-th smallest d(L, x) + d(x, L') over voters x."""
    return float(np.sort(d[lead] + d[:, nxt])[q - 1] * 1000.0)


def detour(d: np.ndarray, lead: int, nxt: int, k: int) -> np.ndarray:
    """delta_j in ms for every node j: the proposal's detour through L' against the k-th
    share path d(L, x) + d(x, j) over x != j."""
    n = len(d)
    paths = d[lead][:, None] + d  # [x, j]
    paths[np.arange(n), np.arange(n)] = np.inf
    kth = np.sort(paths, axis=0)[k - 1]
    out = (d[lead, nxt] + d[nxt] - kth) * 1000.0
    out[lead] = np.nan
    return out


def finality_latency_bound(
    d: np.ndarray, lead: int, q: int, mb: float, c: Compute
) -> float:
    """Finality in ms of a proposal from L while latency binds: every node's Cert1 from the
    q-th vote1 to reach it, its vote2 at once, the q-th node's decide."""
    vote1 = d[lead] * 1000.0 + c.verify_proposal.ms(mb) + c.vote1.ms(mb)  # leaves x
    cert1 = np.sort(vote1[:, None] + d * 1000.0, axis=0)[q - 1] + c.cert.ms(mb)  # at j
    vote2 = cert1 + c.vote2.ms(mb)
    cert2 = np.sort(vote2[:, None] + d * 1000.0, axis=0)[q - 1] + c.cert.ms(mb)  # at i
    return float(np.sort(cert2)[q - 1] + c.decide.ms(mb))


def pair(
    d: np.ndarray,
    lead: int,
    nxt: int,
    n: int,
    k: int,
    mb: float,
    gbps: float,
    c: Compute = ZERO,
    lane: bool = False,
) -> dict[str, float]:
    """The closed forms of one leader pair: Q, delta's median, view, window, finality, ms."""
    q = -(-2 * n // 3)
    q_ms = quorum_rtt(d, lead, nxt, q)
    deltas = detour(d, lead, nxt, k)
    delta_ms = float(np.nanmedian(deltas))
    form = closed_form(n, k, mb, gbps, q_ms, delta_ms, c, lane)
    t_ms = form["share_ms"]
    xfer_view = (
        t_ms + c.decode.ms(mb)
        if lane
        else c.verify_proposal.ms(mb)
        + max(c.vote1.ms(mb) + t_ms, c.build.ms(mb) + c.encode.ms(mb))
        + t_ms
    ) + d[lead, nxt] * 1000.0
    return {
        "q_ms": round(q_ms, 1),
        "delta_p10": round(float(np.nanpercentile(deltas, 10)), 1),
        "delta_p50": round(delta_ms, 1),
        "latency_chain_ms": form["latency_chain_ms"],
        "transfer_chain_ms": round(xfer_view, 1),
        "view_ms": round(max(form["latency_chain_ms"], xfer_view), 1),
        "window_ms": round(form["window_ms"] + c.verify_proposal.ms(mb), 1),
        "finality_ms": round(finality_latency_bound(d, lead, q, mb, c), 1),
    }


def pairs(
    profile: str, n: int, k: int, mb: float, gbps: float, c: Compute, lane: bool
) -> list[dict]:
    """`pair` per region pair, medians over the leaders of each region."""
    d, assignment = delays(profile, n)
    by: dict[tuple[str, str], list[dict]] = defaultdict(list)
    for lead in range(n):
        for nxt in range(n):
            if lead != nxt:
                by[(assignment[lead], assignment[nxt])].append(
                    pair(d, lead, nxt, n, k, mb, gbps, c, lane)
                )
    rows = []
    for (a, b), items in sorted(by.items()):
        row: dict = {"leader": a, "next": b, "n": len(items)}
        for key in items[0]:
            row[key] = round(float(np.median([it[key] for it in items])), 1)
        rows.append(row)
    return rows


# ---- the recurrence over a leader sequence ----


@dataclass(frozen=True)
class Load:
    """Transactions arriving at `rate` MB/s. A leader's block is what arrived since the previous
    leader pulled the pool, at most `max_mb`; the view then sets the next block and the block the
    next view. `seed_mb` is the first block, forgotten by the measured views."""

    rate: float
    max_mb: float = 400.0
    seed_mb: float = 10.0


@dataclass
class Views:
    """Timestamps in seconds, [view, node] unless scalar per view; view 0 is the seed."""

    leader: list[int]
    proposed: np.ndarray  # the proposal leaves the leader
    transit: np.ndarray  # ... and reaches the next leader
    pair: np.ndarray  # proposal and own share in hand, verified
    kth: np.ndarray  # k-th share of the view's block
    recovered: np.ndarray  # block in hand
    cert1: np.ndarray
    decided: np.ndarray
    critical: np.ndarray  # per view, the largest decode that leaves Cert1 where it is
    mb: np.ndarray  # per view, the block's size
    q: int

    def view_times(self, lo, hi):
        return list((self.proposed[lo + 1 : hi + 1] - self.proposed[lo:hi]) * 1000.0)

    def blocks(self, lo, hi):
        return list(self.mb[lo:hi])

    def decided_rate(self, lo, hi) -> float:
        """MB/s over the views, the blocks against the time their proposals span."""
        return float(self.mb[lo:hi].sum() / (self.proposed[hi] - self.proposed[lo]))

    def windows(self, lo, hi):
        out = []
        for v in range(lo, hi):
            w = (self.pair[v + 1] - self.kth[v]) * 1000.0
            out.extend(np.delete(w, self.leader[v]))
        return out

    def finality(self, lo, hi):
        return [
            (np.sort(self.decided[v])[self.q - 1] - self.proposed[v]) * 1000.0
            for v in range(lo, hi)
        ]

    def late_pct(self, lo, hi) -> float:
        waited = total = 0
        for v in range(lo, hi):
            late = np.delete(
                self.recovered[v] > self.pair[v + 1] + 1e-9, self.leader[v]
            )
            waited += int(late.sum())
            total += len(late)
        return round(100.0 * waited / total, 1) if total else float("nan")

    def transits(self, lo, hi):
        return list((self.transit[lo:hi] - self.proposed[lo:hi]) * 1000.0)


def run_views(
    n: int,
    k: int,
    d: np.ndarray,
    mb: float,
    gbps: float,
    leaders: list[int],
    views: int,
    c: Compute = ZERO,
    lane: bool = False,
    load: Load | None = None,
) -> Views:
    """Every node's timestamps for `views` views, each from the previous view's. Blocks are
    `mb` each, or with `load` what arrived since the previous leader pulled the pool."""
    q = -(-2 * n // 3)
    t_share = lambda mb: (n - 1) / k * mb * 8 / gbps / 1000.0
    ms = lambda cost, mb: cost.ms(mb) / 1000.0
    proposal = lambda mb: ms(c.propose, mb) + ms(c.persist_proposal, mb)
    build = lambda mb: ms(c.build, mb) + ms(c.encode, mb)
    shape = (views + 2, n)
    out = Views(
        leaders,
        np.zeros(views + 2),
        np.zeros(views + 2),
        np.zeros(shape),
        np.zeros(shape),
        np.zeros(shape),
        np.zeros(shape),
        np.zeros(shape),
        np.full(views + 2, np.nan),
        np.full(views + 2, load.seed_mb if load else mb),
        q,
    )
    disperse = np.zeros(views + 2)  # the leader's shares start leaving
    pull = np.zeros(views + 2)  # the leader takes the pool into its block
    vote1_out = np.zeros(shape)
    ready = np.zeros(shape)
    rb_done = np.zeros(shape)  # a node's rebroadcast of the view's share has left
    free = np.zeros(n)  # each egress is busy with shares until then
    eye = np.arange(n)
    out.proposed[1] = proposal(out.mb[1])
    disperse[1] = build(out.mb[1])
    free[leaders[1]] = build(out.mb[1]) + t_share(out.mb[1])
    for v in range(1, views + 1):
        lead, nxt = leaders[v], leaders[v + 1]
        mbv = out.mb[v]
        p, disp = out.proposed[v], disperse[v]
        # The proposal leaves behind the shares still leaving L, unless on a lane.
        prop_arr = (p if lane else max(p, free[lead])) + d[lead]
        out.transit[v] = prop_arr[nxt]
        verify = ms(c.verify_proposal, mbv)
        paired = np.maximum(prop_arr, disp + t_share(mbv) + d[lead]) + verify
        paired[lead] = max(p, disp) + verify
        out.pair[v] = paired
        ready[v] = paired + ms(c.vote1, mbv)
        vote1_out[v] = np.maximum(ready[v], out.recovered[v - 1])
        # The next leader builds on the proposal's arrival, from what reached the pool since
        # the last pull; its dispersal of V+1 goes out before its rebroadcast of V if it was
        # queued first.
        pull[v + 1] = prop_arr[nxt] + verify
        if load:
            out.mb[v + 1] = min(load.max_mb, load.rate * (pull[v + 1] - pull[v]))
        mbn = out.mb[v + 1]
        built = pull[v + 1] + build(mbn)
        disperse_first = built < vote1_out[v][nxt]
        if disperse_first:
            disperse[v + 1] = max(built, free[nxt])
            free[nxt] = disperse[v + 1] + t_share(mbn)
        vote1_leave = vote1_out[v] if lane else np.maximum(vote1_out[v], free)
        rb_done[v] = np.maximum(vote1_out[v], free) + t_share(mbv)
        free = rb_done[v].copy()
        if not disperse_first:
            disperse[v + 1] = max(built, free[nxt])
            free[nxt] = disperse[v + 1] + t_share(mbn)
        arrivals = vote1_leave[:, None] + d  # [x, i]
        out.cert1[v] = np.sort(arrivals, axis=0)[q - 1] + ms(c.cert, mbv)
        out.proposed[v + 1] = out.cert1[v][nxt] + proposal(mbn)
        # The k-th share of block V at j: the dispersal from L or a rebroadcast.
        shares = rb_done[v][:, None] + d  # [x, j]
        shares[lead] = np.minimum(shares[lead], disp + t_share(mbv) + d[lead])
        shares[eye, eye] = np.inf
        out.kth[v] = np.sort(shares, axis=0)[k - 1]
        out.kth[v][lead] = disp
        out.recovered[v] = out.kth[v] + ms(c.decode, mbv)
        out.recovered[v][lead] = disp
        # Vote2 behind the node's rebroadcast of V, Cert2, decide.
        vote2 = np.maximum(out.cert1[v], out.recovered[v]) + ms(c.vote2, mbv)
        vote2_leave = vote2 if lane else np.maximum(vote2, rb_done[v])
        cert2 = np.sort(vote2_leave[:, None] + d, axis=0)[q - 1] + ms(c.cert, mbv)
        out.decided[v] = cert2 + ms(c.decide, mbv)
        if v >= 2:
            out.critical[v - 1] = critical(
                out, ready[v], vote1_leave, vote1_out[v], d, v - 1, leaders[v + 1], q
            )
    return out


def critical(out, ready, leave, vote1_out, d, v, nxt2, q) -> float:
    """`vidwindow.Sim.critical` for view `v`: the vote1(V+1) arrivals at the leader of V+2
    with the decode taken as instant."""
    instant = leave - np.maximum(0.0, vote1_out - ready)
    arrivals = instant + d[:, nxt2]
    cert = np.sort(arrivals)[q - 1]
    w = np.maximum(0.0, out.pair[v + 1] - out.kth[v])
    cs = w + (cert - arrivals)
    cs[arrivals > cert + 1e-9] = -np.inf
    cs[out.leader[v]] = np.inf
    return float(np.sort(cs)[::-1][q - 1] * 1000.0)


def point(
    profile: str,
    n: int,
    gbps: float,
    block_mb: float,
    k: int,
    views: int,
    seed: int = 1,
    compute: Compute = ZERO,
    lane: bool = False,
    load: Load | None = None,
) -> dict:
    """`vidwindow.point` from the recurrence: the same keys, the same leader sequence. With
    `load` the blocks follow the offered rate (`block_mb` is ignored) and the result adds the
    block sizes, the decided rate and whether the blocks still grew over the measured views."""
    d, assignment = delays(profile, n)
    rng = random.Random(seed)
    leaders = [0] + [rng.randrange(n) for _ in range(views + 2)]
    r = run_views(n, k, d, block_mb, gbps, leaders, views, compute, lane, load)
    lo, hi = views // 4, views - 5
    crit = [(v, r.critical[v]) for v in range(lo, hi) if not np.isnan(r.critical[v])]
    vt = r.view_times(lo, hi)
    win = r.windows(lo, hi)
    by_region: dict[str, list[float]] = defaultdict(list)
    for v, c in crit:
        by_region[assignment[leaders[v + 1]]].append(c)
    view_p50 = pct(vt, 0.5)
    mbs = r.blocks(lo, hi)
    mb_p50 = pct(mbs, 0.5)
    if load:
        mid = (lo + hi) // 2
        growth = pct(r.blocks(mid, hi), 0.5) / max(pct(r.blocks(lo, mid), 0.5), 1e-9) - 1
        rate = {
            "rate": load.rate,
            "mb_p50": mb_p50,
            "mb_p90": pct(mbs, 0.9),
            "capped_pct": round(100.0 * np.mean(np.array(mbs) >= load.max_mb - 1e-9), 1),
            "growth_pct": round(100.0 * growth, 1),
            "mbps": round(r.decided_rate(lo, hi), 1),
        }
    else:
        rate = {"mbps": round(block_mb / view_p50 * 1000, 1) if view_p50 else float("nan")}
    return {
        "profile": profile,
        "nodes": n,
        "k": k,
        "gbps": gbps,
        "mb": mb_p50,
        "lane": lane,
        "share_ms": round((n - 1) / k * mb_p50 * 8 / gbps, 1),
        "views": len(crit),
        "win_p10": pct(win, 0.1),
        "win_p50": pct(win, 0.5),
        "crit_p10": pct([c for _, c in crit], 0.1),
        "crit_p50": pct([c for _, c in crit], 0.5),
        "crit_p90": pct([c for _, c in crit], 0.9),
        "late_pct": r.late_pct(lo, hi),
        "view_p50": view_p50,
        "view_mean": round(float(np.mean(vt)), 1),
        "transit_p50": pct(r.transits(lo, hi), 0.5),
        "fin_p50": pct(r.finality(lo, hi), 0.5),
        **rate,
        "compute": compute.table(mb_p50),
        "regions": {
            reg: {
                "n": len(v),
                "p10": pct(v, 0.1),
                "p50": pct(v, 0.5),
                "p90": pct(v, 0.9),
            }
            for reg, v in sorted(by_region.items())
        },
    }


def capacity(
    profile: str, n: int, gbps: float, k: int, views: int, c: Compute, lane: bool, max_mb: float
) -> tuple[float, float]:
    """(MB/s, block MB) at the peak of throughput by block size up to `max_mb`: the largest
    offered rate whose blocks settle. Below the peak the loop mb -> rate × view(mb) has a stable
    fixed point, above it none; the growth of measured blocks cannot tell, since the leader
    sequence alone moves their medians by a fifth under geography."""
    best = (0.0, 0.0)
    for i in range(1, 31):
        mb = max_mb * i / 30
        r = point(profile, n, gbps, mb, k, views, 1, c, lane)
        best = max(best, (r["mbps"], mb))
    return best


def print_sweep(args, k: int, compute: Compute) -> None:
    """Blocks, view, finality and the decided rate at each offered rate, and the capacity."""
    cap_mbps, cap_mb = capacity(
        args.profile, args.nodes, args.gbps, k, args.views, compute, args.lane, args.max_block
    )
    print(
        f"{'rate':>6}{'block p50':>10}{'p90':>6}{'view':>7}{'fin':>7}{'late%':>7}"
        f"{'decided':>9}{'capped%':>9}{'growth%':>9}  verdict"
    )
    for rate in (float(x) for x in args.rates.split(",")):
        r = point(
            args.profile,
            args.nodes,
            args.gbps,
            0.0,
            k,
            args.views,
            1,
            compute,
            args.lane,
            Load(rate, args.max_block),
        )
        stable = rate <= cap_mbps and r["capped_pct"] == 0
        print(
            f"{rate:6g}{r['mb_p50']:10.1f}{r['mb_p90']:6.0f}{r['view_mean']:7.0f}{r['fin_p50']:7.0f}"
            f"{r['late_pct']:7.1f}{r['mbps']:9.0f}{r['capped_pct']:9.0f}{r['growth_pct']:9.1f}"
            f"  {'steady' if stable else 'blocks grow'}"
        )
    bound = " (the max block binds)" if cap_mb >= args.max_block - 1e-9 else ""
    print(f"capacity: {cap_mbps:.0f} MB/s at {cap_mb:.0f} MB blocks{bound}")


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("point", "pairs", "sweep"):
        p = sub.add_parser(name)
        p.add_argument(
            "--profile", default="decaf-2025", help="latency profile, or off"
        )
        p.add_argument("--nodes", type=int, default=100)
        p.add_argument("--threshold", default="f+1", choices=("f+1", "2f+1"))
        p.add_argument("--compute", default="off", choices=sorted(COMPUTE))
        p.add_argument("--lane", action="store_true")
        p.add_argument("--gbps", type=float, required=True)
        p.add_argument("--max-block", type=float, default=400.0, help="MB")
    for name in ("point", "pairs"):
        sub.choices[name].add_argument("--block", type=float, help="MB, fixed blocks")
    sub.choices["point"].add_argument(
        "--rate", type=float, help="MB/s offered; blocks follow the rate"
    )
    sub.choices["sweep"].add_argument(
        "--rates", required=True, help="MB/s offered, comma separated"
    )
    for name in ("point", "sweep"):
        sub.choices[name].add_argument("--views", type=int, default=400)
    sub.choices["point"].add_argument(
        "--check", action="store_true", help="also run the simulation and print both"
    )
    args = ap.parse_args(argv)
    k = threshold(args.nodes, args.threshold)
    compute = COMPUTE[args.compute]
    if args.cmd == "sweep":
        print_sweep(args, k, compute)
        return
    if args.cmd == "pairs":
        rows = pairs(
            args.profile, args.nodes, k, args.block, args.gbps, compute, args.lane
        )
        keys = [key for key in rows[0] if key not in ("leader", "next")]
        print(f"{'leader':15} {'next':15}" + "".join(f"{key:>18}" for key in keys))
        for row in rows:
            print(
                f"{row['leader']:15} {row['next']:15}"
                + "".join(f"{row[key]:18g}" for key in keys)
            )
        return
    if (args.block is None) == (args.rate is None):
        ap.error("point takes --block or --rate")
    load = Load(args.rate, args.max_block) if args.rate else None
    formula = point(
        args.profile,
        args.nodes,
        args.gbps,
        args.block or 0.0,
        k,
        args.views,
        1,
        compute,
        args.lane,
        load,
    )
    print(json.dumps(formula, indent=1))
    if args.check and args.block is not None:
        sim = vidwindow.point(
            args.profile,
            args.nodes,
            args.gbps,
            args.block,
            k,
            args.views,
            1,
            compute,
            args.lane,
        )
        print(f"{'':12}{'formula':>10}{'sim':>10}")
        for key in (
            "view_p50",
            "fin_p50",
            "win_p10",
            "win_p50",
            "crit_p10",
            "crit_p50",
            "late_pct",
            "transit_p50",
            "mbps",
        ):
            print(f"{key:12}{formula[key]:10g}{sim[key]:10g}")


if __name__ == "__main__":
    main()
