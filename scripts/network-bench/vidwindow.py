"""One fast-finality view as a fluid network model with the protocol's compute steps.

`window` is the time a node has to reconstruct block V before its vote1 on V+1 waits: from its
k-th share of V to its pairing of proposal V+1 with its share, as the traces measure it (the
vote1 compute after pairing is slack on top). `critical` is the per-view
version that matters to consensus: the largest reconstruction time, applied at every node, that
leaves Cert1(V+1) where it is at the leader of V+2. Only the q-th vote1 to reach that leader sets
Cert1, so a node with a short window delays nothing unless it is that voter. `finality` is the
time from a proposal to a quorum of nodes deciding it.

Network: every node has one interface of capacity B in each direction; a flow runs at
B / max(active flows out of its sender, active flows into its receiver); the messages on one
(sender, receiver) pair arrive in the order they were sent; a message arrives one-way-delay after
its last byte leaves. Delays come from `latency.py` (`decaf-2025`, `mainnet`, ...) or are a flat
0.1 ms for profile `off`.

Protocol, as `crates/hotshot/new-protocol` does it (file references are to its `src/`):

- The next leader builds its block as soon as proposal V arrives, before its own share of V
  (`consensus.rs` `may_build_on_parked`), erasure-codes it and unicasts share j to every node
  (`vid/disperse.rs`). It proposes V+1 once it holds Cert1(V) and the header, without waiting
  for the parent payload (`maybe_propose`), after the proposal is persisted (`release_proposal`).
- A node pairs a proposal with its own share once both are verified (`pair_vid_share`), starts
  state validation, and votes1 once that is done, the Vote action is persisted and block V-1 is
  reconstructed (`maybe_vote_1`, `release_vote1`). Vote1s are broadcast to every node, and the
  node's share follows them (`SendVote1`, `BroadcastVidShare`).
- Every node forms Cert1 from the vote1 broadcasts (`vote.rs` `run_tally`: the votes are
  aggregated once q have arrived, then the aggregate is checked). A node votes2 once it holds
  Cert1 and the reconstructed block, after the lock is persisted (`maybe_vote_2_and_update_lock`,
  `release_vote2`); Cert2 decides (`maybe_decide`).
- Reconstruction starts at the k-th share (`vid/reconstruct.rs` `try_reconstruct`), with
  k = ceil(n / 3) the f+1 threshold of today (`init_avidm_gf2_param`); k = q = ceil(2n / 3) is
  the 2f+1 variant (half the bytes, a quorum of shares to reconstruct).

- One connection per peer carries everything in order (`crates/cliquenet` `Peer::start`
  writes one message to the end before taking the next, lowest slot first). A proposal or vote
  queued behind a share waits for it. `lane=True` models a change: control messages pass the
  shares.

Compute is a `Compute`: one `fixed + per_mb × block MB` cost per step above, zero by default,
with a preset fitted to a run (`COMPUTE`). Each step sits where the code spends it. The view
is the longer of two chains, which `closed_form` states:

    latency chain  = Q + verify_proposal + vote1 + cert + propose + persist_proposal
    transfer chain = 2 T + propose + persist_proposal + delta        (shares and control in order)
                   = T + decode + delta                               (control on its own lane)

with T the share transfer (n - 1) / k × P / B, Q the quorum round trip (the q-th smallest
d(L, j) + d(j, L') over voters j) and delta the latency left once transfers dominate. In order,
the next leader's rebroadcast of share V and its dispersal of V+1 leave its interface back to
back and the proposal of V+1 behind both, two transfers a view; the other compute hides under
them. On a lane the dispersal overlaps the previous view and the k-th share plus its decode
gates the vote. The window is the latency chain less one transfer, or `T + delta + vote1` once
transfers dominate, both as measured from pairing, so less the vote1 compute; `best_case` is
its minimum over bandwidth, (Q + delta) / 2 without compute,
at B = 2 (n - 1) / k × P / (Q - delta).

    vidwindow.py point --profile decaf-2025 --nodes 100 --gbps 15 --block 50
    vidwindow.py point --profile off --nodes 10 --gbps 15 --block 50 --compute c8g.4xlarge
    vidwindow.py point --profile off --nodes 10 --gbps 15 --block 50 --compute c8g.4xlarge --lane
    vidwindow.py grid --profile decaf-2025 --nodes 100 --threshold 2f+1 --out grid.json
"""

import argparse
import heapq
import json
import random
import sys
import time
from collections import defaultdict, deque
from dataclasses import dataclass, fields
from pathlib import Path
from typing import NamedTuple

import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
import latency

FLAT_MS = 0.1
BLOCKS_MB = (5, 20, 50, 100, 200)
GBPS = (1, 1.5, 2, 3, 5, 7.5, 10, 15, 20, 25, 30, 40, 50, 75, 100)


class Cost(NamedTuple):
    """A compute step in ms: `fixed + per_mb × block MB`."""

    fixed_ms: float = 0.0
    per_mb: float = 0.0

    def ms(self, block_mb: float) -> float:
        return self.fixed_ms + self.per_mb * block_mb


NO_COST = Cost()


@dataclass(frozen=True)
class Compute:
    """Compute on a view's path, where `crates/hotshot/new-protocol` spends it.

    - build: next leader, from the proposal's arrival: the payload from the pool and its
      commitments (`block.rs` `request_block`), then the header (`state.rs` `request_header`).
    - encode: erasure coding, before the first share leaves (`vid/disperse.rs`).
    - propose: leaf commitment and signature (`maybe_propose`).
    - persist_proposal: the proposal and Propose action reach storage before the broadcast.
    - verify_proposal: signatures and the justify QC at every receiver, before pairing.
    - verify_share: the own share's proof, before pairing (`VidShareValidator`); untraced.
    - vote1: from pairing, in parallel with waiting for the parent block: state validation, the
      Vote action persisted, the vote signed.
    - decode: from the k-th share: erasure decoding, `from_bytes`, transaction commitments.
    - cert: from the q-th vote: aggregation and the aggregate check (`CheckedAccumulator`).
    - vote2: from Cert1 with the block in hand: sign, persist the lock.
    - decide: from Cert2.
    """

    build: Cost = NO_COST
    encode: Cost = NO_COST
    propose: Cost = NO_COST
    persist_proposal: Cost = NO_COST
    verify_proposal: Cost = NO_COST
    verify_share: Cost = NO_COST
    vote1: Cost = NO_COST
    decode: Cost = NO_COST
    cert: Cost = NO_COST
    vote2: Cost = NO_COST
    decide: Cost = NO_COST

    def table(self, block_mb: float) -> dict[str, float]:
        return {
            f.name: round(getattr(self, f.name).ms(block_mb), 2) for f in fields(self)
        }


ZERO = Compute()
# Per-step medians of run brendon-20261028-180812/01-volume (c8g.4xlarge, 16 vCPU Neoverse-V2,
# postgres on the node, 9 to 92 MB blocks), fitted as a line in the block size; see
# `vidwindow_runs.py fit`. The leader's own proposal took 5 ms from broadcast to pairing, which
# stands in for every receiver's verification.
C8G_4XLARGE = Compute(
    build=Cost(0.0, 0.85),
    encode=Cost(0.6, 0.27),
    propose=Cost(1.0),
    persist_proposal=Cost(7.9),
    verify_proposal=Cost(5.0),
    vote1=Cost(7.3, 0.05),
    decode=Cost(0.0, 0.73),
    cert=Cost(3.1),
    vote2=Cost(4.7),
    decide=Cost(0.6),
)
# Per-step medians of run brendon-20261029-193318/01-volume (c8g.8xlarge, 32 vCPU, no latency),
# the same fit; the explorer page carries the same numbers.
C8G_8XLARGE = Compute(
    build=Cost(0.0, 1.2),
    encode=Cost(0.35, 0.16),
    propose=Cost(0.5),
    persist_proposal=Cost(7.6, 0.02),
    verify_proposal=Cost(3.7, 0.07),
    vote1=Cost(7.1, 0.06),
    decode=Cost(0.0, 0.61),
    cert=Cost(3.1),
    vote2=Cost(4.0, 0.01),
    decide=Cost(0.5, 0.01),
)
COMPUTE = {"off": ZERO, "c8g.4xlarge": C8G_4XLARGE, "c8g.8xlarge": C8G_8XLARGE}


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
    compute: Compute = ZERO,
    lane: bool = False,
) -> dict:
    """Window, critical window, view time, finality and throughput of one setting."""
    d, assignment = delays(profile, n)
    sim = Sim(
        n,
        gbps * 1e9 / 8,
        block_mb * 1e6,
        views,
        d,
        k=k,
        seed=seed,
        compute=compute,
        lane=lane,
    )
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
        "lane": lane,
        "share_ms": round((n - 1) / k * block_mb * 8 / gbps, 1),
        "views": len(crit),
        "win_p10": pct(win, 0.1),
        "win_p50": pct(win, 0.5),
        "crit_p10": pct([c for _, c in crit], 0.1),
        "crit_p50": pct([c for _, c in crit], 0.5),
        "crit_p90": pct([c for _, c in crit], 0.9),
        "late_pct": sim.late_pct(lo, hi),
        "view_p50": view_p50,
        "transit_p50": pct(sim.transit(lo, hi), 0.5),
        "fin_p50": pct(sim.finality(lo, hi), 0.5),
        "mbps": round(block_mb / view_p50 * 1000, 1) if view_p50 else float("nan"),
        "compute": compute.table(block_mb),
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
    compute: Compute = ZERO,
    lane: bool = False,
) -> list[dict]:
    rows = []
    t0 = time.time()
    for mb in blocks:
        for g in gbps:
            row = point(profile, n, g, mb, k, views, compute=compute, lane=lane)
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


def closed_form(
    n: int,
    k: int,
    block_mb: float,
    gbps: float,
    q_ms: float,
    delta_ms: float,
    compute: Compute = ZERO,
    lane: bool = False,
) -> dict[str, float]:
    """The view time and window of one leader pair in ms, from the two chains of the docstring.

    `q_ms` is the quorum round trip of the pair and `delta_ms` the latency left once transfers
    dominate; both are 0 without latency. The view is the mean over consecutive distinct
    leaders: in order, views alternate around it. With a lane the window given holds while
    latency binds; once transfers do, every vote waits on the decode."""
    c = compute
    t_ms = (n - 1) / k * block_mb * 8 / gbps
    proposal = c.propose.ms(block_mb) + c.persist_proposal.ms(block_mb)
    lat = (
        q_ms
        + c.verify_proposal.ms(block_mb)
        + c.vote1.ms(block_mb)
        + c.cert.ms(block_mb)
        + proposal
    )
    slack = lat - c.vote1.ms(block_mb) - t_ms  # from pairing, while latency binds
    if lane:
        xfer = t_ms + c.decode.ms(block_mb) + delta_ms
        window = slack
    else:
        xfer = 2 * t_ms + proposal + delta_ms
        window = max(slack, t_ms + delta_ms)
    return {
        "share_ms": round(t_ms, 1),
        "view_ms": round(max(lat, xfer), 1),
        "window_ms": round(window, 1),
        "latency_chain_ms": round(lat, 1),
        "transfer_chain_ms": round(xfer, 1),
    }


def best_case(q_ms: float, delta_ms: float, n: int, k: int) -> tuple[float, float]:
    """(minimum window in ms, Gbps per MB of block at which it occurs) for a leader with
    quorum round trip `q_ms` and transfer-bound remainder `delta_ms`, without compute."""
    ms_per_mb_gbps = (n - 1) / k * 8
    return (q_ms + delta_ms) / 2, 2 * ms_per_mb_gbps / (q_ms - delta_ms)


def pct(xs, p: float) -> float:
    xs = sorted(xs)
    return (
        round(float(xs[min(len(xs) - 1, int(p * len(xs)))]), 1) if xs else float("nan")
    )


class Sim:
    """One run of the fluid model; `run()` then read `windows`, `critical`, `view_times`,
    `finality`, `late_pct`. `lane` puts control messages ahead of the shares."""

    def __init__(
        self,
        n,
        bytes_per_s,
        block_bytes,
        views,
        d,
        k=None,
        seed=1,
        leaders=None,
        compute: Compute = ZERO,
        lane: bool = False,
    ):
        self.n, self.b, self.views, self.d = n, bytes_per_s, views, d
        self.lane = lane
        self.k = k or -(-n // 3)
        self.q = -(-2 * n // 3)
        self.share = block_bytes / self.k
        self.mb = block_bytes / 1e6
        self.c = compute
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
        self.nvotes1: dict[tuple, int] = defaultdict(int)
        self.nvotes2: dict[tuple, int] = defaultdict(int)
        self.kth: dict[tuple, float] = {}  # (node, view): k-th share in hand
        self.recovered: dict[tuple, float] = {}  # (node, view): block in hand
        self.prop_arr: dict[tuple, float] = {}  # (node, view): proposal arrived
        self.pv: dict[tuple, float] = {}  # (node, view): proposal paired with the share
        self.ready: dict[tuple, float] = {}  # (node, view): vote1 compute done
        self.left1: dict[tuple, float] = {}  # (node, view): vote1 left
        self.cert1: dict[tuple, float] = {}
        self.cert2: dict[tuple, float] = {}
        self.decided: dict[tuple, float] = {}
        self.voted1: set = set()
        self.voted2: set = set()
        self.building: set = set()
        self.built: dict[int, float] = {}
        self.proposing: set = set()
        self.proposed: dict[int, float] = {}
        self.vote_arr: dict[tuple, dict] = defaultdict(dict)

    # ---- results ----

    def windows(self, lo, hi):
        """Per node and view, from the k-th share of block V to the pairing of proposal V+1."""
        return [
            (self.pv[(i, v + 1)] - self.kth[(i, v)]) * 1000.0
            for v in range(lo, hi)
            for i in range(self.n)
            if i != self.leader[v] and (i, v) in self.kth and (i, v + 1) in self.pv
        ]

    def transit(self, lo, hi):
        """Per view, from the proposal leaving to its arrival at the next leader."""
        return [
            (self.prop_arr[(self.leader[v + 1], v)] - self.proposed[v]) * 1000.0
            for v in range(lo, hi)
            if (self.leader[v + 1], v) in self.prop_arr
        ]

    def critical(self, lo, hi):
        """[(V, ms)]: the largest reconstruction time, the same at every node, that leaves the
        q-th vote1(V+1) at the leader of V+2 where it is. Vote arrivals are taken as if the
        decode of the run had been instant, so the answer is the slack and not the margin
        left after the decode modelled; like `windows` it counts from pairing."""
        out = []
        for v in range(lo, hi):
            leader = self.leader[v + 2]
            arrivals = self.vote_arr.get((leader, v + 1), {})
            if len(arrivals) < self.q or (v + 2) not in self.proposed:
                continue
            instant = {
                i: a - max(0.0, self.left1[(i, v + 1)] - self.ready[(i, v + 1)])
                for i, a in arrivals.items()
            }
            cert = sorted(instant.values())[self.q - 1]
            cs = []
            for i in range(self.n):
                if i == self.leader[v]:
                    cs.append(float("inf"))  # built block V, nothing to reconstruct
                    continue
                a = instant.get(i)
                if a is None or a > cert + 1e-9:
                    cs.append(float("-inf"))
                    continue
                # the vote leaves at max(ready, kth + D): D moves it once D exceeds the window
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

    def finality(self, lo, hi):
        """Per view, the time from the proposal to a quorum of nodes deciding it."""
        out = []
        for v in range(lo, hi):
            ts = sorted(
                self.decided[(i, v)] for i in range(self.n) if (i, v) in self.decided
            )
            if len(ts) >= self.q and v in self.proposed:
                out.append((ts[self.q - 1] - self.proposed[v]) * 1000.0)
        return out

    def late_pct(self, lo, hi) -> float:
        """Share of vote1s whose parent block was reconstructed after the proposal paired, as
        the traces count it; some of those still left on time, under the vote1 compute."""
        waited = total = 0
        for v in range(lo, hi):
            for i in range(self.n):
                if i == self.leader[v] or (i, v + 1) not in self.pv:
                    continue
                total += 1
                waited += self.recovered[(i, v)] > self.pv[(i, v + 1)] + 1e-9
        return round(100.0 * waited / total, 1) if total else float("nan")

    # ---- the event loop ----

    def run(self):
        n = self.n
        for i in range(n):
            self.recovered[(i, 0)] = self.kth[(i, 0)] = 0.0
            self.cert1[(i, 0)] = self.ready[(i, 0)] = 0.0
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
        if self.lane and size <= 0:  # control messages pass the shares
            for j in dsts:
                self.schedule(self.t + self.d[s, j], j, msg)
            return
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

    def after(self, cost: Cost, dst, msg):
        """Deliver `msg` to `dst` once `cost` of compute has passed."""
        self.schedule(self.t + cost.ms(self.mb) / 1000.0, dst, msg)

    # ---- the protocol ----

    def deliver(self, i, msg):
        kind, v, src = msg
        if kind == "prop":
            self.prop_arr[(i, v)] = self.t
            self.after(self.c.verify_proposal, i, ("prop_ok", v, src))
        elif kind == "prop_ok":
            self.got_prop[i].add(v)
            if self.leader[v + 1] == i and v + 1 <= self.views + 1:
                self.build(i, v + 1)  # on the parked proposal, before the own share
            self.try_pair(i, v)
        elif kind == "share":
            self.after(self.c.verify_share, i, ("share_ok", v, src))
        elif kind == "share_ok":
            self.got_share[i].add(v)
            self.add_share(i, v)
            self.try_pair(i, v)
        elif kind == "rebroadcast":
            self.add_share(i, v)
        elif kind == "recovered":
            self.on_recovered(i, v)
        elif kind == "built":
            self.on_built(i, v)
        elif kind == "disperse":
            self.enqueue(i, self.peers(i), self.share, ("share", v, i))
            self.on_recovered(i, v)  # `VidDisperseCreated`
        elif kind == "proposed":
            self.proposed[v] = self.t
            self.enqueue(i, self.peers(i), 0.0, ("prop", v, i))
            self.deliver(i, ("prop", v, i))
        elif kind == "vote1_out":
            self.left1[(i, v)] = self.t
            self.enqueue(i, self.peers(i), 0.0, ("vote1", v, i))
            self.enqueue(i, self.peers(i), self.share, ("rebroadcast", v, i))
            self.deliver(i, ("vote1", v, i))
        elif kind == "vote1":
            self.nvotes1[(i, v)] += 1
            self.vote_arr[(i, v)][src] = self.t
            if self.nvotes1[(i, v)] == self.q:
                self.after(self.c.cert, i, ("cert1", v, i))
        elif kind == "cert1":
            self.cert1[(i, v)] = self.t
            if self.leader[v + 1] == i:
                self.try_propose(i, v + 1)
            self.try_vote2(i, v)
        elif kind == "vote2_out":
            self.enqueue(i, self.peers(i), 0.0, ("vote2", v, i))
            self.deliver(i, ("vote2", v, i))
        elif kind == "vote2":
            self.nvotes2[(i, v)] += 1
            if self.nvotes2[(i, v)] == self.q:
                self.after(self.c.cert, i, ("cert2", v, i))
        elif kind == "cert2":
            self.cert2[(i, v)] = self.t
            self.after(self.c.decide, i, ("decided", v, i))
        elif kind == "decided":
            self.decided[(i, v)] = self.t

    def build(self, leader, v):
        if v in self.building:
            return
        self.building.add(v)
        self.after(self.c.build, leader, ("built", v, leader))

    def on_built(self, leader, v):
        self.built[v] = self.t
        self.kth[(leader, v)] = self.t
        self.got_share[leader].add(v)
        self.nshares[(leader, v)] += 1
        self.after(self.c.encode, leader, ("disperse", v, leader))
        self.try_pair(leader, v)
        self.try_propose(leader, v)

    def try_propose(self, leader, v):
        if (
            v in self.proposing
            or v not in self.built
            or (leader, v - 1) not in self.cert1
        ):
            return
        self.proposing.add(v)
        cost = Cost(self.c.propose.ms(self.mb) + self.c.persist_proposal.ms(self.mb))
        self.after(cost, leader, ("proposed", v, leader))

    def try_pair(self, i, v):
        if (i, v) in self.pv or v not in self.got_prop[i] or v not in self.got_share[i]:
            return
        self.pv[(i, v)] = self.t
        self.ready[(i, v)] = self.t + self.c.vote1.ms(self.mb) / 1000.0
        self.try_vote1(i, v)

    def on_recovered(self, i, v):
        self.recovered[(i, v)] = self.t
        self.try_vote1(i, v + 1)
        self.try_vote2(i, v)

    def add_share(self, i, v):
        self.nshares[(i, v)] += 1
        if self.nshares[(i, v)] >= self.k and (i, v) not in self.kth:
            self.kth[(i, v)] = self.t
            self.after(self.c.decode, i, ("recovered", v, i))

    def try_vote1(self, i, v):
        if (
            (i, v) in self.voted1
            or (i, v) not in self.ready
            or (i, v - 1) not in self.recovered
        ):
            return
        self.voted1.add((i, v))
        self.schedule(max(self.t, self.ready[(i, v)]), i, ("vote1_out", v, i))

    def try_vote2(self, i, v):
        if (
            (i, v) in self.voted2
            or (i, v) not in self.cert1
            or (i, v) not in self.recovered
        ):
            return
        self.voted2.add((i, v))
        self.after(self.c.vote2, i, ("vote2_out", v, i))

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
        p.add_argument("--compute", default="off", choices=sorted(COMPUTE))
        p.add_argument(
            "--lane",
            action="store_true",
            help="control messages on their own lane, ahead of the shares",
        )
    sub.choices["point"].add_argument("--gbps", type=float, required=True)
    sub.choices["point"].add_argument("--block", type=float, required=True, help="MB")
    sub.choices["grid"].add_argument("--blocks", default=",".join(map(str, BLOCKS_MB)))
    sub.choices["grid"].add_argument("--gbps", default=",".join(map(str, GBPS)))
    sub.choices["grid"].add_argument("--out", type=Path, required=True)
    args = ap.parse_args(argv)
    k = threshold(args.nodes, args.threshold)
    compute = COMPUTE[args.compute]
    if args.cmd == "point":
        row = point(
            args.profile,
            args.nodes,
            args.gbps,
            args.block,
            k,
            args.views,
            compute=compute,
            lane=args.lane,
        )
        print(json.dumps(row, indent=1))
        return
    blocks = tuple(float(x) for x in args.blocks.split(","))
    gbps = tuple(float(x) for x in args.gbps.split(","))
    rows = grid(
        args.profile,
        args.nodes,
        k,
        args.views,
        blocks,
        gbps,
        sys.stderr,
        compute,
        args.lane,
    )
    args.out.write_text(json.dumps(rows, indent=1))


if __name__ == "__main__":
    main()
