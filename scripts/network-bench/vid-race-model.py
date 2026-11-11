#!/usr/bin/env python3
"""When does the systematic VID path beat erasure decoding?

A timing model of one view's dispersal, parameterised on the latency profile,
the stake distribution, where the systematic shard range lands, and the
bandwidth per node. It answers: at each replica, when is a threshold of shares
in hand (decode path), when is the specific systematic set in hand (concat
path), and which finishes first once reconstruction cost is added.

Flow modelled (crates/hotshot/new-protocol/src/vid.rs):

1. The leader unicasts every node its share, all flows at once, sharing the
   leader's egress fairly. Node `i` holds `w_i` shards of `P / t` bytes, with
   `t` the recovery threshold in shards.
2. A node rebroadcasts its share to every peer as soon as it arrives, sharing
   its own egress fairly across the peers.
3. A replica's ingress is also bounded: shares that arrive together are drained
   at the node's bandwidth, in arrival order.
4. Decode is ready at a replica once received weight covers `t`; concat is
   ready once every node whose range touches `[0, t)` has delivered. Each path
   then pays its reconstruction cost (recover plus re-commit, from the
   `vid_reconstruct_*_duration` histograms of the 5 November runs).

The view's consensus-relevant number is the time by which two thirds of the
stake has the payload, since vote1 waits for the parent's reconstruction.
That is reported per path, averaged over every node taking a turn as leader.

Examples:

    ./vid-race-model.py --profile decaf-2025 -n 10 --stake equal --policy registration nearest
    ./vid-race-model.py --profile decaf-2025 -n 10 --stake top:1:0.2 top:2:0.15 --policy stake nearest
    ./vid-race-model.py --profile mainnet -n 92 --stake profile --threshold 1/3 2/3
"""

from __future__ import annotations

import argparse
import json
import math
import statistics
import sys
from dataclasses import dataclass
from fractions import Fraction
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import latency  # noqa: E402

VID_TARGET_TOTAL_STAKE = 1000
POLICIES = ("registration", "stake", "inverse-stake", "nearest", "farthest")


@dataclass(frozen=True)
class Scenario:
    profile: str
    n: int
    stake: str
    policy: str
    threshold: Fraction
    parity: str
    payload_mb: float
    bw_mb_s: float
    concat_ms_per_mb: float
    decode_ms_per_mb: float
    intra: bool
    relay: str = "stake"
    relay_threshold: Fraction = Fraction(1, 3)
    pipelined: bool = False
    fanout: int = 8
    relay_cap: float = 2.0


@dataclass
class Outcome:
    """Per (leader, replica) finish times in ms, and per-leader quorum times. The leader's
    egress and the mean node egress are means over the leaders; the max is over all of them."""

    decode_finish: list[float]
    concat_finish: list[float]
    decode_quorum: list[float]
    concat_quorum: list[float]
    race_quorum: list[float]
    leader_egress_ms: float
    holders: list[int]
    leader_egress_mb: float = 0.0
    max_node_egress_mb: float = 0.0
    mean_node_egress_mb: float = 0.0


def stake_fractions(spec: str, n: int, profile_name: str, assignment: list[str]) -> list[float]:
    """Stake share per node from a spec: `equal`, `profile` (the locations
    file's `stake_share` split evenly across a location's nodes) or
    `top:K:F` (K nodes each with fraction F, the rest equal)."""
    if spec == "equal":
        return [1.0 / n] * n
    if spec == "profile":
        path = latency.LOCATIONS_JSON.get(profile_name)
        if path is None:
            raise ValueError(f"profile {profile_name!r} carries no stake shares; use equal or top:K:F")
        data = json.loads(path.read_text())
        share = {p["label"]: p["stake_share"] for p in data["locations"]}
        counts = {label: assignment.count(label) for label in set(assignment)}
        fractions = [share[label] / counts[label] for label in assignment]
        total = sum(fractions)
        return [f / total for f in fractions]
    if spec.startswith("top:"):
        _, k, f = spec.split(":")
        k, f = int(k), float(f)
        if k * f >= 1 or k >= n:
            raise ValueError(f"top:{k}:{f} leaves no stake for the other nodes")
        rest = (1 - k * f) / (n - k)
        return [f] * k + [rest] * (n - k)
    raise ValueError(f"unknown stake spec {spec!r}")


def vid_weights(fractions: list[float]) -> list[int]:
    """`approximate_weights` in hotshot-types for a stake table above the target."""
    return [int(f * VID_TARGET_TOTAL_STAKE) + 1 for f in fractions]


def fair_share_finish(sizes: list[float], bw: float) -> list[float]:
    """Finish time of each flow when they start together and share `bw`
    equally (processor sharing): smaller flows finish first."""
    order = sorted(range(len(sizes)), key=lambda i: sizes[i])
    finish = [0.0] * len(sizes)
    sent = 0.0
    t = 0.0
    remaining = len(sizes)
    for i in order:
        chunk = sizes[i] - sent
        t += chunk * remaining / bw
        finish[i] = t
        sent = sizes[i]
        remaining -= 1
    return finish


def systematic_order(policy: str, leader: int, stake: list[float], delays: dict) -> list[int]:
    n = len(stake)
    nodes = list(range(n))
    if policy == "registration":
        return nodes
    if policy == "stake":
        return sorted(nodes, key=lambda i: -stake[i])
    if policy == "inverse-stake":
        return sorted(nodes, key=lambda i: stake[i])
    if policy in ("nearest", "farthest"):
        sign = 1 if policy == "nearest" else -1
        return sorted(nodes, key=lambda i: sign * delays.get((leader, i), 0.0))
    raise ValueError(policy)


def simulate(sc: Scenario) -> Outcome:
    profile = latency.load_profile(sc.profile)
    assignment = latency.assign(sc.n, profile)
    delays = latency.delays_ms(assignment, profile, sc.intra)
    stake = stake_fractions(sc.stake, sc.n, sc.profile, assignment)
    if sc.relay == "equal":
        # Every node relays an equal slice of the coded data; a replica needs
        # any `relay_threshold` of the slices by count. Its own stake-weighted
        # share falls out of the re-commit once it holds the payload.
        weights = [1] * sc.n
        total = sc.n
        t = math.ceil(total * sc.relay_threshold)
    else:
        weights = vid_weights(stake)
        total = sum(weights)
        t = math.ceil(total * sc.threshold)
    shard_mb = sc.payload_mb / t
    bw = sc.bw_mb_s / 1000.0  # MB per ms
    n = sc.n

    out = Outcome([], [], [], [], [], 0.0, [])
    for leader in range(n):
        order = systematic_order(sc.policy, leader, stake, delays)
        # Contiguous ranges in committee order; the systematic range is [0, t).
        start = 0
        holders, size, shards = [], [0.0] * n, [0] * n
        for i in order:
            end = start + weights[i]
            if start < t:
                holders.append(i)
            shards[i] = weights[i] if sc.parity == "full" else max(0, min(end, t) - start)
            size[i] = shards[i] * shard_mb
            start = end
        if leader == 0:
            out.holders = holders

        # 1. Leader unicast, every flow at once over the leader's egress.
        flows = [size[i] for i in range(n) if i != leader]
        finish = fair_share_finish(flows, bw)
        arrive = [0.0] * n
        k = 0
        for i in range(n):
            if i == leader:
                continue
            arrive[i] = finish[k] + delays.get((leader, i), 0.0)
            k += 1
        out.leader_egress_ms += (max(finish) if finish else 0.0) / n
        out.leader_egress_mb += sum(flows) / n

        # 2. Rebroadcast: node i sends its share to n-1 peers at once. With
        # `--relay tree`, a share above `relay_cap` times the mean goes to
        # `fanout` helpers (the least-loaded nodes), each forwarding it on to
        # an equal slice of the remaining peers.
        received = {j: [] for j in range(n) if j != leader}
        egress = [(n - 1) * size[i] if i != leader else 0.0 for i in range(n)]
        cap = sc.relay_cap * sum(size) / n
        for i in range(n):
            if size[i] == 0:
                continue
            if sc.relay == "tree" and size[i] > cap and i != leader:
                d = sc.fanout
                peers = [j for j in received if j != i]
                egress[i] = d * size[i]
                helpers = sorted((j for j in peers), key=lambda j: egress[j])[:d]
                rest = [j for j in peers if j not in helpers]
                first_hop = (delays.get((leader, i), 0.0) if sc.pipelined else arrive[i]) + d * size[i] / bw
                per_helper = math.ceil(len(rest) / d)
                for k, h in enumerate(helpers):
                    received[h].append((first_hop + delays.get((i, h), 0.0), i))
                    fanned = rest[k * per_helper:(k + 1) * per_helper]
                    egress[h] += len(fanned) * size[i]
                    hop_done = first_hop + delays.get((i, h), 0.0) + len(fanned) * size[i] / bw
                    for j in fanned:
                        received[j].append((hop_done + delays.get((h, j), 0.0), i))
                continue
            if sc.pipelined:
                # Forward fragments as they arrive: the relay's own egress is the
                # bound, since the leader's fair share delivers no faster.
                done = delays.get((leader, i), 0.0) + (n - 1) * size[i] / bw
            else:
                done = arrive[i] + (n - 1) * size[i] / bw
            for j in received:
                if j != i:
                    received[j].append((done + delays.get((i, j), 0.0), i))

        node_egress = [egress[i] for i in range(n) if i != leader]
        out.max_node_egress_mb = max(out.max_node_egress_mb, max(node_egress))
        out.mean_node_egress_mb += sum(node_egress) / len(node_egress) / n

        # 3. Ingress drain, then 4. readiness per path.
        d_fin, c_fin = {}, {}
        for j, arrivals in received.items():
            arrivals.sort()
            drained = 0.0
            got_w = shards[j]  # a replica's own share is in hand
            got_holders = {j} if j in holders else set()
            d_ready = 0.0 if got_w >= t else None
            c_ready = 0.0 if got_holders >= set(holders) else None
            for at, i in arrivals:
                drained = max(at, drained + size[i] / bw)
                got_w += shards[i]
                if i in holders:
                    got_holders.add(i)
                if d_ready is None and got_w >= t:
                    d_ready = drained
                if c_ready is None and got_holders >= set(holders):
                    c_ready = drained
            d_fin[j] = (d_ready if d_ready is not None else math.inf) + sc.decode_ms_per_mb * sc.payload_mb
            c_fin[j] = (c_ready if c_ready is not None else math.inf) + sc.concat_ms_per_mb * sc.payload_mb
            out.decode_finish.append(d_fin[j])
            out.concat_finish.append(c_fin[j])

        def quorum(fin: dict) -> float:
            # Time by which nodes holding >= 2/3 of stake have the payload; the
            # leader has it at time 0.
            have = stake[leader]
            last = 0.0
            for j in sorted(fin, key=fin.get):
                if have >= 2 / 3:
                    break
                have += stake[j]
                last = fin[j]
            return last if have >= 2 / 3 else math.inf

        out.decode_quorum.append(quorum(d_fin))
        out.concat_quorum.append(quorum(c_fin))
        out.race_quorum.append(quorum({j: min(d_fin[j], c_fin[j]) for j in d_fin}))
    return out


def fmt(x: float) -> str:
    return "inf" if math.isinf(x) else f"{x:.0f}"


def summarize(sc: Scenario, out: Outcome) -> str:
    pairs = list(zip(out.decode_finish, out.concat_finish))
    finite = [(d, c) for d, c in pairs if not math.isinf(d) and not math.isinf(c)]
    wins = sum(1 for d, c in finite if c < d)
    saving = [d - c for d, c in finite]
    race = statistics.mean(out.race_quorum)
    decode = statistics.mean(out.decode_quorum)
    concat = statistics.mean(out.concat_quorum)
    return " | ".join([
        sc.profile,
        str(sc.n),
        sc.stake,
        f"{sc.threshold}",
        sc.parity,
        sc.policy,
        {"stake": "stake", "equal": f"equal {sc.relay_threshold}", "tree": f"tree x{sc.fanout} cap {sc.relay_cap}"}[sc.relay]
        + (" piped" if sc.pipelined else ""),
        f"{len(out.holders)}",
        fmt(out.leader_egress_mb),
        fmt(out.max_node_egress_mb),
        fmt(out.mean_node_egress_mb),
        fmt(out.leader_egress_ms),
        fmt(decode),
        fmt(concat),
        fmt(race),
        fmt(decode - race),
        f"{100 * wins / len(finite):.0f}%" if finite else "-",
        fmt(statistics.median(saving)) if saving else "-",
    ])


HEADER = (
    "| profile | n | stake | thr | parity | systematic on | relay | holders | leader MB | max node MB | mean node MB | leader egress ms | "
    "decode 2/3 ms | concat 2/3 ms | race 2/3 ms | race saves ms | concat wins | median save ms |"
)


def parse_fraction(s: str) -> Fraction:
    return Fraction(s)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--profile", nargs="+", default=["decaf-2025"], choices=latency.PROFILES[1:])
    ap.add_argument("-n", "--nodes", nargs="+", type=int, default=[10])
    ap.add_argument("--stake", nargs="+", default=["equal"], help="equal | profile | top:K:F")
    ap.add_argument("--policy", nargs="+", default=["stake", "nearest"], choices=POLICIES)
    ap.add_argument("--threshold", nargs="+", type=parse_fraction, default=[Fraction(1, 3)])
    ap.add_argument("--parity", nargs="+", default=["full"], choices=("full", "none"),
                    help="none models ESPRESSO_BENCH_VID_SYSTEMATIC_ONLY: no parity shards are sent")
    ap.add_argument("--payload-mb", type=float, default=100.0)
    ap.add_argument("--bw", type=float, default=1800.0, help="MB/s per node, egress and ingress")
    ap.add_argument("--concat-ms-per-mb", type=float, default=0.37,
                    help="recover + re-commit on the systematic path (sysonly run, node1, per-step mean at 80-120 MB blocks)")
    ap.add_argument("--decode-ms-per-mb", type=float, default=0.51,
                    help="recover + re-commit on the decode path (q2base run, node1, per-step mean at 80-120 MB blocks)")
    ap.add_argument("--no-intra", action="store_true", help="no delay between nodes of one location")
    ap.add_argument("--pipelined", nargs="+", type=int, default=[0], choices=(0, 1),
                    help="1: a node forwards fragments as they arrive instead of after its whole share")
    ap.add_argument("--relay", nargs="+", default=["stake"], choices=("stake", "equal", "tree"),
                    help="stake: each node rebroadcasts its own stake-weighted share (today); "
                         "equal: each node relays an equal slice of the coded data; "
                         "tree: as stake, but a share over --relay-cap times the mean goes "
                         "through --fanout helpers")
    ap.add_argument("--fanout", type=int, default=8)
    ap.add_argument("--relay-cap", type=float, default=2.0, help="multiple of the mean share size")
    ap.add_argument("--relay-threshold", nargs="+", type=parse_fraction, default=[Fraction(1, 3)],
                    help="with --relay equal, the fraction of slices by count a replica needs")
    args = ap.parse_args()

    print(HEADER)
    print("|" + "---|" * (HEADER.count("|") - 1))
    for profile in args.profile:
        for n in args.nodes:
            for stake in args.stake:
                for thr in args.threshold:
                    for parity in args.parity:
                        for policy in args.policy:
                            for relay in args.relay:
                                for rthr in (args.relay_threshold if relay == "equal" else [Fraction(1, 3)]):
                                    for piped in args.pipelined:
                                        sc = Scenario(profile, n, stake, policy, thr, parity, args.payload_mb,
                                                      args.bw, args.concat_ms_per_mb, args.decode_ms_per_mb,
                                                      not args.no_intra, relay, rthr, bool(piped),
                                                      args.fanout, args.relay_cap)
                                        print("| " + summarize(sc, simulate(sc)) + " |")


if __name__ == "__main__":
    main()
