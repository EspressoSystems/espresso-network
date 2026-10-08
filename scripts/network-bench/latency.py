"""Geographic latency profiles for aws-bench, rendered as per-node tc scripts.

`latency-matrix.csv` is copied verbatim from EspressoSystems/espresso-deploy commit 34b35f6
(branch new-protocol), measured with t3.micro probes, `ping -c 200`, 2026-08. Only `src`, `dst`
and `rtt_p50_ms` are read.

Delays are one-way: half the directed RTT on each egress. Node indices are 0-based positions in
the assignment.
"""

import csv
import hashlib
import io
import json
import math
import re
from dataclasses import dataclass
from pathlib import Path

MS_PER_KM = 0.0157
SAME_CITY_RTT_MS = 1.0
INTRA_REGION_RTT_MS = 10.0
NETEM_LIMIT = 300000
TC_IFACE = "IFACE=$(ip -o -4 route show to default | awk '{print $5}' | head -1)"
# Congestion control of the shaped nodes. cubic, the Linux default, sends a burst as fast as its
# window allows. bbr paces at its bandwidth estimate: bursty validator traffic keeps it in
# STARTUP (pacing gain 2.89) until the first overload, after which cross-region sockets pace
# 3-5x slower for the rest of the run (run lulu-20261008-110417).
# bbr_hold: BBR held in STARTUP, built on the node by aws/bbr-hold.sh; an experiment, not a
# stock congestion control.
TCP_CCS = ("cubic", "bbr", "bbr_hold")
# On the node next to the shipped scripts.
BBR_HOLD_SCRIPT = "/opt/bench/bbr-hold.sh"
# Host TCP settings an operator can set. 256 MB buffers give a 128 MB window, enough for a
# 100 MB proposal or the 5 Gbps x 330 ms bandwidth-delay product; Ubuntu's 4 MB cap holds one
# flow near 25 MB/s at 158 ms. The qdisc matters only where no HTB root is installed.
BASE_TCP_SYSCTLS: tuple[tuple[str, str], ...] = (
    ("net.ipv4.tcp_rmem", "4096 131072 268435456"),
    ("net.ipv4.tcp_wmem", "4096 16384 268435456"),
    ("net.ipv4.tcp_notsent_lowat", "131072"),
    ("net.ipv4.tcp_slow_start_after_idle", "0"),
    ("net.ipv4.tcp_mtu_probing", "1"),
)


def tcp_sysctls(tcp_cc: str) -> tuple[tuple[str, str], ...]:
    qdisc = "fq_codel" if tcp_cc == "cubic" else "fq"
    return (
        ("net.ipv4.tcp_congestion_control", tcp_cc),
        ("net.core.default_qdisc", qdisc),
        *BASE_TCP_SYSCTLS,
    )


# Traffic between AWS regions, over inter-region peering or an internet gateway carries at most
# 1500 bytes; only traffic inside one VPC gets jumbo frames (9001). `--mtu` sets the interface MTU.
# https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/network_mtu.html
INTERNET_MTU = 1500
TC_CLEAR = 'if tc qdisc show dev "$IFACE" | grep -q "htb 1:"; then tc qdisc del dev "$IFACE" root; fi'
# Sysctls and MTU outlive the qdisc until a reboot, which a fleet never gets between runs. The
# first shaping after a reset saves the host's values here, and the reset replays them.
BASELINE = "/var/lib/aws-bench-net-baseline.sh"
SAVE_BASELINE = "\n".join(
    [
        f"if [ ! -e {BASELINE} ]; then",
        "{",
        *(
            f'echo "sysctl -q -w {key}=\\"$(sysctl -n {key})\\""'
            # Every --tcp-cc sets the same keys.
            for key, _ in tcp_sysctls("cubic")
        ),
        (
            'echo "ip link set dev $IFACE mtu'
            ' $(ip -o link show dev "$IFACE" | sed -n \'s/.* mtu \\([0-9]*\\).*/\\1/p\')"'
        ),
        f"}} > {BASELINE}.tmp",
        f"mv {BASELINE}.tmp {BASELINE}",
        "fi",
    ]
)
# networkd re-applies a drop-in MTU on every link reset, so it goes before the baseline MTU.
NETWORKD_DIR = "/etc/systemd/network"
MTU_DROPIN = "bench-mtu.conf"
RESTORE_BASELINE = (
    f"if [ -e {BASELINE} ]; then rm -f {NETWORKD_DIR}/*.d/{MTU_DROPIN}; networkctl reload; "
    f"bash {BASELINE}; rm {BASELINE}; fi"
)
PROFILES = ("off", "decaf-2025", "mainnet")
DECAF_SPLIT: tuple[tuple[str, int], ...] = (
    ("eu-central-1", 38),
    ("ap-southeast-1", 28),
    ("us-east-1", 22),
    ("ap-southeast-2", 8),
    ("sa-east-1", 4),
)
MATRIX_CSV = Path(__file__).with_name("latency-matrix.csv")
MAINNET_JSON = Path(__file__).with_name("mainnet-locations.json")

EARTH_RADIUS_KM = 6371.0
PROBE_ABS_TOL_MS = 2.0
PROBE_REL_TOL = 0.1
PING_SUMMARY = re.compile(r"rtt min/avg/max/mdev = [\d.]+/([\d.]+)/[\d.]+/[\d.]+ ms")


@dataclass(frozen=True)
class Profile:
    name: str
    labels: tuple[str, ...]
    weights: dict[str, int]
    cross_ms: dict[tuple[str, str], float]
    intra_ms: dict[str, float]


@dataclass(frozen=True)
class Shaping:
    """Everything derived from a profile name and node count: built once per run."""

    profile: Profile
    assignment: list[str]
    delays: dict[tuple[int, int], float]
    intra: bool
    tcp_cc: str
    mtu: int

    def meta(self) -> dict:
        return {
            "profile": self.profile.name,
            "intra": self.intra,
            "nodes": {
                label: self.assignment.count(label)
                for label in self.profile.labels
                if label in self.assignment
            },
            "assignment": {
                f"node{i}": label for i, label in enumerate(self.assignment)
            },
            "matrix_sha256": matrix_sha256(self.delays),
            "sysctls": dict(tcp_sysctls(self.tcp_cc)),
            "mtu": self.mtu,
            "probes": [],
        }


@dataclass(frozen=True)
class Probe:
    src: int
    dst: int
    tier: str
    expected_ms: float


def load_profile(name: str) -> Profile:
    if name == "decaf-2025":
        return decaf_profile(read_matrix(MATRIX_CSV.read_text()))
    if name == "mainnet":
        return mainnet_profile(json.loads(MAINNET_JSON.read_text()))
    raise ValueError(f"no latency profile {name!r}; expected one of {PROFILES[1:]}")


def shaping(name: str, n: int, intra: bool, tcp_cc: str, mtu: int) -> Shaping:
    profile = load_profile(name)
    assignment = assign(n, profile)
    delays = delays_ms(assignment, profile, intra)
    return Shaping(profile, assignment, delays, intra, tcp_cc, mtu)


def read_matrix(text: str) -> dict[tuple[str, str], float]:
    matrix: dict[tuple[str, str], float] = {}
    for row in csv.DictReader(io.StringIO(text)):
        pair = (row["src"], row["dst"])
        if pair[0] == pair[1]:
            raise ValueError(f"matrix row {pair} has src == dst")
        if pair in matrix:
            raise ValueError(f"matrix has duplicate row {pair}")
        matrix[pair] = float(row["rtt_p50_ms"])
    return matrix


def decaf_profile(matrix: dict[tuple[str, str], float]) -> Profile:
    labels = tuple(label for label, _ in DECAF_SPLIT)
    cross = {}
    for src in labels:
        for dst in labels:
            if src != dst:
                cross[(src, dst)] = matrix[(src, dst)]
    return Profile(
        name="decaf-2025",
        labels=labels,
        weights=dict(DECAF_SPLIT),
        cross_ms=cross,
        intra_ms={label: INTRA_REGION_RTT_MS for label in labels},
    )


def great_circle_km(a: tuple[float, float], b: tuple[float, float]) -> float:
    lat1, lon1, lat2, lon2 = map(math.radians, (*a, *b))
    h = (
        math.sin((lat2 - lat1) / 2) ** 2
        + math.cos(lat1) * math.cos(lat2) * math.sin((lon2 - lon1) / 2) ** 2
    )
    return 2 * EARTH_RADIUS_KM * math.asin(math.sqrt(h))


def estimate_rtt_ms(km: float) -> float:
    return max(SAME_CITY_RTT_MS, MS_PER_KM * km)


def mainnet_profile(data: dict) -> Profile:
    places = data["locations"]
    labels = tuple(p["label"] for p in places)
    if len(set(labels)) != len(labels):
        raise ValueError(f"duplicate location labels in {labels}")
    coords = {p["label"]: (p["lat"], p["lon"]) for p in places}
    cross = {
        (src, dst): estimate_rtt_ms(great_circle_km(coords[src], coords[dst]))
        for src in labels
        for dst in labels
        if src != dst
    }
    return Profile(
        name="mainnet",
        labels=labels,
        weights={p["label"]: p["nodes"] for p in places},
        cross_ms=cross,
        intra_ms={label: SAME_CITY_RTT_MS for label in labels},
    )


def apportion(
    n: int, labels: tuple[str, ...], weights: dict[str, int]
) -> dict[str, int]:
    """Largest remainder in exact integer arithmetic; ties go to the earlier label."""
    if n <= 0:
        raise ValueError(f"node count must be positive, got {n}")
    total = sum(weights[label] for label in labels)
    shares = {label: divmod(n * weights[label], total) for label in labels}
    counts = {label: quotient for label, (quotient, _) in shares.items()}
    by_remainder = sorted(labels, key=lambda label: -shares[label][1])
    for label in by_remainder[: n - sum(counts.values())]:
        counts[label] += 1
    return counts


def assign(n: int, profile: Profile) -> list[str]:
    counts = apportion(n, profile.labels, profile.weights)
    return [label for label in profile.labels for _ in range(counts[label])]


def delays_ms(
    assignment: list[str], profile: Profile, intra: bool
) -> dict[tuple[int, int], float]:
    delays = {}
    for i, src in enumerate(assignment):
        for j, dst in enumerate(assignment):
            if i == j:
                continue
            if src != dst:
                delays[(i, j)] = profile.cross_ms[(src, dst)] / 2
            elif intra:
                delays[(i, j)] = profile.intra_ms[src] / 2
    return delays


def matrix_sha256(delays: dict[tuple[int, int], float]) -> str:
    triples = [[i, j, round(ms, 1)] for (i, j), ms in sorted(delays.items())]
    return hashlib.sha256(
        json.dumps(triples, separators=(",", ":")).encode()
    ).hexdigest()


def mtu_lines(mtu: int) -> list[str]:
    """Sets the MTU where systemd-networkd keeps it: an MTU change resets the ENA link, and on
    link-up networkd re-applies the 9001 of the DHCP lease (seen in run lulu-20261008-121303).
    Waits up to 10 s for the link to report it."""
    return [
        "unit=$(networkctl status \"$IFACE\" | awk '/Network File:/ {print $3}')",
        'test -n "$unit"',
        f'dropin="{NETWORKD_DIR}/$(basename "$unit").d"',
        'mkdir -p "$dropin"',
        f"printf '[Link]\\nMTUBytes={mtu}\\n[DHCPv4]\\nUseMTU=no\\n' > \"$dropin/{MTU_DROPIN}\"",
        "networkctl reload",
        f'ip link set dev "$IFACE" mtu {mtu}',
        f'for _ in $(seq 50); do [ "$(cat /sys/class/net/$IFACE/mtu)" = {mtu} ] && break; sleep 0.2; done',
    ]


def tc_script(
    node: int,
    peer_ips: dict[int, str],
    delays: dict[tuple[int, int], float],
    tcp_cc: str,
    mtu: int,
) -> str:
    lines = [
        "set -eu",
        TC_IFACE,
        'test -n "$IFACE"',
        SAVE_BASELINE,
        *(
            {"bbr": ["modprobe tcp_bbr"], "bbr_hold": [f"bash {BBR_HOLD_SCRIPT}"]}.get(
                tcp_cc, []
            )
        ),
        *(f'sysctl -q -w {key}="{value}"' for key, value in tcp_sysctls(tcp_cc)),
        *mtu_lines(mtu),
        TC_CLEAR,
        'tc qdisc add dev "$IFACE" root handle 1: htb default 1',
        'tc class add dev "$IFACE" parent 1: classid 1:1 htb rate 100gbit',
    ]
    for peer, ip in sorted(peer_ips.items()):
        if peer == node or (node, peer) not in delays:
            continue
        minor = f"{peer + 2:x}"
        lines += [
            f'tc class add dev "$IFACE" parent 1: classid 1:{minor} htb rate 100gbit',
            (
                f'tc qdisc add dev "$IFACE" parent 1:{minor} netem'
                f" delay {delays[(node, peer)]:g}ms limit {NETEM_LIMIT}"
            ),
            (
                'tc filter add dev "$IFACE" protocol ip parent 1: prio 1 u32'
                f" match ip dst {ip}/32 flowid 1:{minor}"
            ),
        ]
    return "\n".join(lines) + "\n"


def probe_pairs(
    assignment: list[str], delays: dict[tuple[int, int], float], intra: bool
) -> list[Probe]:
    def expected(j: int) -> float:
        return delays[(0, j)] + delays[(j, 0)]

    peers = range(1, len(assignment))
    cross = [j for j in peers if assignment[j] != assignment[0]]
    probes = []
    if cross:
        far = max(cross, key=expected)
        probes.append(Probe(0, far, "cross", expected(far)))
    same = [j for j in peers if assignment[j] == assignment[0]]
    if intra and same:
        probes.append(Probe(0, same[0], "intra", expected(same[0])))
    return probes


def parse_ping_avg_ms(stdout: str) -> float:
    match = PING_SUMMARY.search(stdout)
    if match is None:
        raise ValueError(f"no iputils rtt summary line in ping output: {stdout!r}")
    return float(match.group(1))


def probe_error(probe: Probe, measured_ms: float) -> str | None:
    tolerance = max(PROBE_ABS_TOL_MS, PROBE_REL_TOL * probe.expected_ms)
    if abs(measured_ms - probe.expected_ms) <= tolerance:
        return None
    return (
        f"probe node{probe.src}->node{probe.dst} {probe.tier}: expected"
        f" {probe.expected_ms:.1f} ms, measured {measured_ms:.1f} ms,"
        f" tolerance {tolerance:.1f} ms"
    )
