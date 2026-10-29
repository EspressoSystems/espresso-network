"""The VID reconstruction window with every computation instant: a fluid model of one view.

`window` is the time a node has to reconstruct block V before its vote1 on V+1 waits: from its
k-th share of V to its validation of proposal V+1. `critical` is the per-view version that
matters to consensus: the largest reconstruction time, applied at every node, that leaves
Cert1(V+1) where it is at the leader of V+2. Only the q-th vote1 to reach that leader sets Cert1,
so a node with a short window delays nothing unless it is that voter.

Network: every node has one interface of capacity B in each direction; a flow runs at
B / max(active flows out of its sender, active flows into its receiver); the messages on one
(sender, receiver) pair arrive in the order they were sent; a message arrives one-way-delay after
its last byte leaves. Delays come from `latency.py` (`decaf-2025`, `mainnet`, ...) or are a flat
0.1 ms for profile `off`.

Protocol, as `crates/hotshot/new-protocol` does it: the next leader builds its block and unicasts
share j (P / k bytes) to every node as soon as it validates the current proposal; it proposes
once it holds Cert1 (q vote1s), without waiting for the parent payload. A node validates a
proposal once the proposal and its own share are in hand, votes once it holds k shares of the
parent block, and rebroadcasts its share to every peer right after the vote. k = ceil(n / 3) is
the f+1 threshold of today; k = q = ceil(2n / 3) is the 2f+1 variant (half the bytes, a quorum
of shares to reconstruct).

Closed form under latency, per leader: window ~ max(Q - T, T + delta) with T the share transfer
(n - 1) / k * P / B, Q the quorum round trip (the q-th smallest d(L, j) + d(j, L') over voters j)
and delta the latency left once transfers dominate. The minimum over bandwidth is (Q + delta) / 2
at B = 2 (n - 1) / k * P / (Q - delta), whatever the block size.

    vidwindow.py point --profile decaf-2025 --nodes 100 --gbps 15 --block 50
    vidwindow.py grid --profile decaf-2025 --nodes 100 --threshold 2f+1 --out grid.json
"""

import argparse
import heapq
import json
import random
import sys
import time
from collections import defaultdict, deque
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
import latency

FLAT_MS = 0.1
BLOCKS_MB = (5, 20, 50, 100, 200)
GBPS = (1, 1.5, 2, 3, 5, 7.5, 10, 15, 20, 25, 30, 40, 50, 75, 100)


def threshold(n: int, name: str) -> int:
    if name == "f+1":
        return -(-n // 3)
    if name == "2f+1":
        return -(-2 * n // 3)
    raise ValueError(f"threshold {name!r}; expected f+1 or 2f+1")


def delays(profile: str, n: int) -> tuple[np.ndarray, list[str]]:
    """One-way delay in seconds between node indices, and each node's region label."""
    d = np.zeros((n, n))
    if profile == "off":
        d[:] = FLAT_MS / 1000.0
        np.fill_diagonal(d, 0.0)
        return d, ["one-az"] * n
    shaping = latency.shaping(profile, n, True, "cubic", 1500)
    for (i, j), ms in shaping.delays.items():
        d[i, j] = ms / 1000.0
    return d, list(shaping.assignment)


def point(
    profile: str,
    n: int,
    gbps: float,
    block_mb: float,
    k: int,
    views: int,
    seed: int = 1,
) -> dict:
    """Window, critical window, view time and throughput of one setting, medians and tails."""
    d, assignment = delays(profile, n)
    sim = Sim(n, gbps * 1e9 / 8, block_mb * 1e6, views, d, k=k, seed=seed)
    sim.run()
    lo, hi = views // 4, views - 5
    crit = sim.critical(lo, hi)
    vt = sim.view_times(lo, hi)
    win = sim.windows(lo, hi)
    by_region: dict[str, list[float]] = defaultdict(list)
    for v, c in crit:
        by_region[assignment[sim.leader[v + 1]]].append(c)
    view_p50 = pct(vt, 0.5)
    return {
        "profile": profile,
        "nodes": n,
        "k": k,
        "gbps": gbps,
        "mb": block_mb,
        "share_ms": round((n - 1) / k * block_mb * 8 / gbps, 1),
        "views": len(crit),
        "win_p10": pct(win, 0.1),
        "win_p50": pct(win, 0.5),
        "crit_p10": pct([c for _, c in crit], 0.1),
        "crit_p50": pct([c for _, c in crit], 0.5),
        "crit_p90": pct([c for _, c in crit], 0.9),
        "view_p50": view_p50,
        "mbps": round(block_mb / view_p50 * 1000, 1) if view_p50 else float("nan"),
        "regions": {
            r: {"n": len(v), "p10": pct(v, 0.1), "p50": pct(v, 0.5), "p90": pct(v, 0.9)}
            for r, v in sorted(by_region.items())
        },
    }


def grid(
    profile: str,
    n: int,
    k: int,
    views: int,
    blocks: tuple[float, ...] = BLOCKS_MB,
    gbps: tuple[float, ...] = GBPS,
    out=None,
) -> list[dict]:
    rows = []
    t0 = time.time()
    for mb in blocks:
        for g in gbps:
            row = point(profile, n, g, mb, k, views)
            rows.append(row)
            if out is not None:
                print(
                    f"{time.time() - t0:6.0f}s {mb:6g} MB {g:6g} Gbps  window p10/p50"
                    f" {row['crit_p10']:7.1f} {row['crit_p50']:7.1f}  view {row['view_p50']:7.1f} ms"
                    f"  {row['mbps']:6.0f} MB/s",
                    file=out,
                    flush=True,
                )
    return rows


def best_case(q_ms: float, delta_ms: float, n: int, k: int) -> tuple[float, float]:
    """(minimum window in ms, Gbps per MB of block at which it occurs) for a leader with
    quorum round trip `q_ms` and transfer-bound remainder `delta_ms`."""
    ms_per_mb_gbps = (n - 1) / k * 8
    return (q_ms + delta_ms) / 2, 2 * ms_per_mb_gbps / (q_ms - delta_ms)


def pct(xs, p: float) -> float:
    xs = sorted(xs)
    return (
        round(float(xs[min(len(xs) - 1, int(p * len(xs)))]), 1) if xs else float("nan")
    )


class Sim:
    """One run of the fluid model; `run()` then read `windows`, `critical`, `view_times`."""

    def __init__(
        self, n, bytes_per_s, block_bytes, views, d, k=None, seed=1, leaders=None
    ):
        self.n, self.b, self.views, self.d = n, bytes_per_s, views, d
        self.k = k or -(-n // 3)
        self.q = -(-2 * n // 3)
        self.share = block_bytes / self.k
        rng = random.Random(seed)
        self.leader = leaders or [0] + [rng.randrange(n) for _ in range(views + 2)]
        self.t = 0.0
        self.queue: list[deque] = [deque() for _ in range(n * n)]
        self.left = np.full(n * n, -1.0)  # bytes left in the head message of each pair
        self.src = np.repeat(np.arange(n), n)
        self.dst = np.tile(np.arange(n), n)
        self.events: list = []
        self.seq = 0
        self.got_prop: dict[int, set] = defaultdict(set)
        self.got_share: dict[int, set] = defaultdict(set)
        self.nshares: dict[tuple, int] = defaultdict(int)
        self.nvotes: dict[tuple, int] = defaultdict(int)
        self.kth: dict[tuple, float] = {}
        self.pv: dict[tuple, float] = {}
        self.cert1: dict[tuple, float] = {}
        self.voted: set = set()
        self.built: dict[int, float] = {}
        self.proposed: dict[int, float] = {}
        self.vote_arr: dict[tuple, dict] = defaultdict(dict)

    # ---- results ----

    def windows(self, lo, hi):
        return [
            (self.pv[(i, v + 1)] - self.kth[(i, v)]) * 1000.0
            for v in range(lo, hi)
            for i in range(self.n)
            if i != self.leader[v] and (i, v) in self.kth and (i, v + 1) in self.pv
        ]

    def critical(self, lo, hi):
        """[(V, ms)]: the largest reconstruction time, the same at every node, that leaves the
        q-th vote1(V+1) at the leader of V+2 where it is."""
        out = []
        for v in range(lo, hi):
            leader = self.leader[v + 2]
            arrivals = self.vote_arr.get((leader, v + 1), {})
            if len(arrivals) < self.q or (v + 2) not in self.proposed:
                continue
            cert = sorted(arrivals.values())[self.q - 1]
            cs = []
            for i in range(self.n):
                if i == self.leader[v]:
                    cs.append(float("inf"))  # built block V, nothing to reconstruct
                    continue
                a = arrivals.get(i)
                if a is None or a > cert + 1e-9:
                    cs.append(float("-inf"))
                    continue
                # the vote leaves at max(pv, kth + D): D moves it once D exceeds the window
                w = max(0.0, self.pv[(i, v + 1)] - self.kth[(i, v)])
                cs.append(w + (cert - a))
            cs.sort(reverse=True)
            out.append((v, cs[self.q - 1] * 1000.0))
        return out

    def view_times(self, lo, hi):
        return [
            (self.proposed[v + 1] - self.proposed[v]) * 1000.0
            for v in range(lo, hi)
            if v + 1 in self.proposed
        ]

    # ---- the event loop ----

    def run(self):
        n = self.n
        for i in range(n):
            self.kth[(i, 0)] = self.cert1[(i, 0)] = self.pv[(i, 0)] = 0.0
        self.build(self.leader[1], 1)
        while True:
            active = np.flatnonzero(self.left > 0)
            if active.size:
                s, r = self.src[active], self.dst[active]
                out_cnt = np.bincount(s, minlength=n)[s]
                in_cnt = np.bincount(r, minlength=n)[r]
                rate = self.b / np.maximum(out_cnt, in_cnt)
                dt_flow = float(np.min(self.left[active] / rate))
            else:
                dt_flow = float("inf")
            t_ev = self.events[0][0] if self.events else float("inf")
            dt = min(dt_flow, t_ev - self.t)
            if dt == float("inf"):
                break
            if active.size:
                self.left[active] -= rate * dt
            self.t += dt
            if active.size and dt_flow <= t_ev - (self.t - dt) + 1e-15:
                for p in active[self.left[active] <= 1e-6 * self.share]:
                    _, msg = self.queue[p].popleft()
                    self.schedule(self.t + self.d[p // n, p % n], p % n, msg)
                    self.set_head(p)
            while self.events and self.events[0][0] <= self.t + 1e-12:
                _, _, dst, msg = heapq.heappop(self.events)
                self.deliver(dst, msg)
            if self.proposed.get(self.views + 1):
                break

    def enqueue(self, s, dsts, size, msg):
        for j in dsts:
            p = s * self.n + j
            self.queue[p].append((size, msg))
            if len(self.queue[p]) == 1:
                self.set_head(p)

    def set_head(self, p):
        q = self.queue[p]
        while q and q[0][0] <= 0:  # a message without bytes leaves at once
            _, msg = q.popleft()
            self.schedule(self.t + self.d[p // self.n, p % self.n], p % self.n, msg)
        self.left[p] = q[0][0] if q else -1.0

    def schedule(self, t, dst, msg):
        self.seq += 1
        heapq.heappush(self.events, (t, self.seq, dst, msg))

    # ---- the protocol ----

    def deliver(self, i, msg):
        kind, v, src = msg
        if kind == "share":
            self.got_share[i].add(v)
            self.add_share(i, v)
            self.try_validate(i, v)
        elif kind == "prop":
            self.got_prop[i].add(v)
            self.try_validate(i, v)
        elif kind == "vote1":
            self.nvotes[(i, v)] += 1
            self.vote_arr[(i, v)][src] = self.t
            if self.nvotes[(i, v)] >= self.q and (i, v) not in self.cert1:
                self.cert1[(i, v)] = self.t
                if self.leader[v + 1] == i:
                    self.try_propose(i, v + 1)
        elif kind == "rebroadcast":
            self.add_share(i, v)

    def add_share(self, i, v):
        self.nshares[(i, v)] += 1
        if self.nshares[(i, v)] >= self.k and (i, v) not in self.kth:
            self.kth[(i, v)] = self.t
            self.try_vote1(i, v + 1)

    def try_validate(self, i, v):
        if (i, v) in self.pv or v not in self.got_prop[i] or v not in self.got_share[i]:
            return
        self.pv[(i, v)] = self.t
        self.try_vote1(i, v)
        if self.leader[v + 1] == i and v + 1 <= self.views + 1:
            self.build(i, v + 1)

    def build(self, leader, v):
        self.built[v] = self.t
        self.kth[(leader, v)] = self.t
        self.enqueue(leader, self.peers(leader), self.share, ("share", v, leader))
        self.got_share[leader].add(v)
        self.nshares[(leader, v)] += 1
        self.try_validate(leader, v)
        self.try_propose(leader, v)

    def try_propose(self, leader, v):
        if (
            v in self.proposed
            or v not in self.built
            or (leader, v - 1) not in self.cert1
        ):
            return
        self.proposed[v] = self.t
        self.enqueue(leader, self.peers(leader), 0.0, ("prop", v, leader))
        self.got_prop[leader].add(v)
        self.try_validate(leader, v)

    def try_vote1(self, i, v):
        if (i, v) in self.voted or (i, v) not in self.pv or (i, v - 1) not in self.kth:
            return
        self.voted.add((i, v))
        self.enqueue(i, self.peers(i), 0.0, ("vote1", v, i))
        self.enqueue(i, self.peers(i), self.share, ("rebroadcast", v, i))
        self.deliver(i, ("vote1", v, i))

    def peers(self, i):
        return [j for j in range(self.n) if j != i]


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("point", "grid"):
        p = sub.add_parser(name)
        p.add_argument(
            "--profile", default="decaf-2025", help="latency profile, or off"
        )
        p.add_argument("--nodes", type=int, default=100)
        p.add_argument("--threshold", default="f+1", choices=("f+1", "2f+1"))
        p.add_argument("--views", type=int, default=400)
    sub.choices["point"].add_argument("--gbps", type=float, required=True)
    sub.choices["point"].add_argument("--block", type=float, required=True, help="MB")
    sub.choices["grid"].add_argument("--blocks", default=",".join(map(str, BLOCKS_MB)))
    sub.choices["grid"].add_argument("--gbps", default=",".join(map(str, GBPS)))
    sub.choices["grid"].add_argument("--out", type=Path, required=True)
    args = ap.parse_args(argv)
    k = threshold(args.nodes, args.threshold)
    if args.cmd == "point":
        print(
            json.dumps(
                point(args.profile, args.nodes, args.gbps, args.block, k, args.views),
                indent=1,
            )
        )
        return
    blocks = tuple(float(x) for x in args.blocks.split(","))
    gbps = tuple(float(x) for x in args.gbps.split(","))
    rows = grid(args.profile, args.nodes, k, args.views, blocks, gbps, out=sys.stderr)
    args.out.write_text(json.dumps(rows, indent=1))


if __name__ == "__main__":
    main()
