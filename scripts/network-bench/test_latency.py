import math
import re

import latency
import pytest
from latency import Probe, Profile

COORDS = {
    "eu-central-1": (50.11, 8.68),
    "ap-southeast-1": (1.35, 103.82),
    "us-east-1": (38.95, -77.45),
    "ap-southeast-2": (-33.87, 151.21),
    "sa-east-1": (-23.55, -46.63),
    "us-east-2": (39.96, -82.99),
    "eu-west-1": (53.35, -6.26),
    "us-west-1": (37.34, -121.89),
}

DECAF5_SHA256 = "1104f1e848507c501bdf3dcb9775e2af41a03de8eb06bbbe34d49d7964065206"

PING_OK = """PING 10.0.1.5 (10.0.1.5) 56(84) bytes of data.
64 bytes from 10.0.1.5: icmp_seq=1 ttl=64 time=158.4 ms

--- 10.0.1.5 ping statistics ---
3 packets transmitted, 3 received, 0% packet loss, time 2003ms
rtt min/avg/max/mdev = 158.100/158.412/158.900/0.321 ms
"""

PING_LOSS = """PING 10.0.1.5 (10.0.1.5) 56(84) bytes of data.

--- 10.0.1.5 ping statistics ---
3 packets transmitted, 0 received, 100% packet loss, time 2050ms
"""


def decaf() -> Profile:
    return latency.load_profile("decaf-2025")


def two_cities() -> dict:
    return {
        "measured_at": "2026-10-05",
        "epoch": 1,
        "validators": 3,
        "locations": [
            {
                "label": "frankfurt-de",
                "city": "Frankfurt",
                "country": "DE",
                "lat": 50.11,
                "lon": 8.68,
                "nodes": 2,
                "stake_share": 0.6,
            },
            {
                "label": "dublin-ie",
                "city": "Dublin",
                "country": "IE",
                "lat": 53.35,
                "lon": -6.26,
                "nodes": 1,
                "stake_share": 0.4,
            },
        ],
    }


# TEST:apportion-five-nodes-ok
def test_apportion_five_nodes():
    profile = decaf()
    counts = latency.apportion(5, profile.labels, profile.weights)
    assert counts == {
        "eu-central-1": 2,
        "ap-southeast-1": 2,
        "us-east-1": 1,
        "ap-southeast-2": 0,
        "sa-east-1": 0,
    }
    assert latency.assign(1, profile) == ["eu-central-1"]


# TEST:apportion-bad-n-fails
@pytest.mark.parametrize("n", [0, -3])
def test_apportion_bad_n_fails(n):
    profile = decaf()
    with pytest.raises(ValueError, match="positive"):
        latency.apportion(n, profile.labels, profile.weights)


def test_apportion_small_n():
    profile = decaf()
    assert latency.assign(2, profile) == ["eu-central-1", "ap-southeast-1"]
    meta = latency.shaping("decaf-2025", 2, True, "cubic").meta()
    assert meta["nodes"] == {"eu-central-1": 1, "ap-southeast-1": 1}


def test_apportion_ties_by_label_order():
    counts = latency.apportion(1, ("a", "b"), {"a": 1, "b": 1})
    assert counts == {"a": 1, "b": 0}


# TEST:assign-hundred-ok
def test_assign_hundred():
    profile = decaf()
    assignment = latency.assign(100, profile)
    assert [assignment.count(label) for label in profile.labels] == [38, 28, 22, 8, 4]
    assert assignment[:38] == ["eu-central-1"] * 38


# TEST:decaf-profile-pairs-ok
def test_decaf_profile_pairs():
    profile = decaf()
    assert len(profile.cross_ms) == 20
    assert profile.cross_ms[("eu-central-1", "sa-east-1")] == 202.0
    assert profile.intra_ms == dict.fromkeys(profile.labels, 10.0)


def test_matrix_missing_pair_fails():
    text = "src,dst,rtt_p50_ms\neu-central-1,ap-southeast-1,158.0\n"
    with pytest.raises(KeyError, match="eu-central-1.*us-east-1"):
        latency.decaf_profile(latency.read_matrix(text))


def test_matrix_self_pair_fails():
    with pytest.raises(ValueError, match="src == dst"):
        latency.read_matrix("src,dst,rtt_p50_ms\neu-west-1,eu-west-1,0.0\n")


def test_matrix_duplicate_pair_fails():
    text = "src,dst,rtt_p50_ms\na,b,1.0\na,b,2.0\n"
    with pytest.raises(ValueError, match="duplicate"):
        latency.read_matrix(text)


def test_load_profile_unknown_fails():
    for name in ("x", "off"):
        with pytest.raises(ValueError, match="no latency profile"):
            latency.load_profile(name)


# TEST:mainnet-profile-estimate-ok
def test_mainnet_profile_estimate():
    profile = latency.mainnet_profile(two_cities())
    km = latency.great_circle_km(COORDS["eu-central-1"], COORDS["eu-west-1"])
    assert 1080 < km < 1100
    expected = 0.0157 * km
    assert profile.cross_ms[("frankfurt-de", "dublin-ie")] == pytest.approx(expected)
    assert profile.cross_ms[("dublin-ie", "frankfurt-de")] == pytest.approx(expected)
    assert profile.labels == ("frankfurt-de", "dublin-ie")
    assert profile.weights == {"frankfurt-de": 2, "dublin-ie": 1}
    assert profile.intra_ms == {"frankfurt-de": 1.0, "dublin-ie": 1.0}


def test_estimate_rtt_floor():
    assert latency.estimate_rtt_ms(0.0) == 1.0
    assert latency.estimate_rtt_ms(10.0) == 1.0
    assert latency.estimate_rtt_ms(1000.0) == pytest.approx(15.7)


def test_great_circle_zero_and_antipode():
    assert latency.great_circle_km((10.0, 20.0), (10.0, 20.0)) == 0.0
    assert latency.great_circle_km((0.0, 0.0), (0.0, 180.0)) == pytest.approx(
        math.pi * 6371.0
    )


def test_mainnet_json_missing_key_fails():
    data = two_cities()
    del data["locations"][1]["lat"]
    with pytest.raises(KeyError, match="lat"):
        latency.mainnet_profile(data)


def test_mainnet_duplicate_label_fails():
    data = two_cities()
    data["locations"][1]["label"] = "frankfurt-de"
    with pytest.raises(ValueError, match="duplicate"):
        latency.mainnet_profile(data)


# TEST:fit-constant-ok
def test_fit_constant():
    matrix = latency.read_matrix(latency.MATRIX_CSV.read_text())
    assert len(matrix) == 56
    num = den = 0.0
    for (src, dst), rtt in matrix.items():
        km = latency.great_circle_km(COORDS[src], COORDS[dst])
        num += km * rtt
        den += km * km
    assert num / den == pytest.approx(latency.MS_PER_KM, abs=0.0005)


# TEST:delays-half-ok
def test_delays_half():
    profile = decaf()
    assignment = latency.assign(5, profile)
    on = latency.delays_ms(assignment, profile, intra=True)
    assert on[(0, 2)] == 79.0
    assert on[(0, 1)] == 5.0
    assert (0, 0) not in on
    off = latency.delays_ms(assignment, profile, intra=False)
    assert (0, 1) not in off
    assert off[(0, 2)] == 79.0


# TEST:tc-script-ok
def test_tc_script_four_peers():
    profile = decaf()
    delays = latency.delays_ms(latency.assign(5, profile), profile, intra=True)
    peers = {i: f"10.0.0.{i + 10}" for i in range(5)}
    script = latency.tc_script(0, peers, delays, "bbr")
    assert script.startswith("set -eu\n")
    assert script.endswith("\n")
    assert script.count(" netem ") == 4
    assert script.count(" match ip dst ") == 4
    assert "10.0.0.10/32" not in script
    assert "delay 5ms limit 300000" in script
    assert "match ip dst 10.0.0.12/32 flowid 1:4" in script
    assert 'sysctl -q -w net.ipv4.tcp_rmem="4096 131072 268435456"' in script
    assert script.index("sysctl") < script.index("tc qdisc add")
    assert 'ip link set dev "$IFACE" mtu 1500' in script
    assert "tcp_adv_win_scale" not in script


@pytest.mark.parametrize(
    ("tcp_cc", "qdisc", "modprobe"), [("cubic", "fq_codel", False), ("bbr", "fq", True)]
)
def test_tc_script_sets_the_congestion_control(tcp_cc, qdisc, modprobe):
    profile = decaf()
    delays = latency.delays_ms(latency.assign(5, profile), profile, intra=True)
    peers = {i: f"10.0.0.{i + 10}" for i in range(5)}
    script = latency.tc_script(0, peers, delays, tcp_cc)
    assert f'net.ipv4.tcp_congestion_control="{tcp_cc}"' in script
    assert f'net.core.default_qdisc="{qdisc}"' in script
    assert ("modprobe tcp_bbr" in script) == modprobe


def test_tc_script_skips_peers_without_delay():
    profile = decaf()
    delays = latency.delays_ms(latency.assign(5, profile), profile, intra=False)
    peers = {i: f"10.0.0.{i + 10}" for i in range(5)}
    script = latency.tc_script(0, peers, delays, "bbr")
    assert script.count(" netem ") == 3
    assert "10.0.0.11/32" not in script


# TEST:tc-script-ok
def test_tc_script_hundred_peers_unique_minors():
    profile = decaf()
    delays = latency.delays_ms(latency.assign(100, profile), profile, intra=True)
    peers = {i: f"10.0.1.{i}" for i in range(100)}
    script = latency.tc_script(0, peers, delays, "bbr")
    minors = re.findall(r"classid 1:([0-9a-f]+) htb", script)
    assert len(minors) == 100
    assert len(set(minors)) == 100
    assert minors.count("1") == 1


# TEST:probe-pairs-ok
def test_probe_pairs():
    profile = decaf()
    assignment = latency.assign(5, profile)
    delays = latency.delays_ms(assignment, profile, intra=True)
    cross, intra = latency.probe_pairs(assignment, delays, intra=True)
    assert cross == Probe(0, 2, "cross", 158.0)
    assert intra == Probe(0, 1, "intra", 10.0)
    only_cross = latency.probe_pairs(
        assignment, latency.delays_ms(assignment, profile, False), intra=False
    )
    assert only_cross == [cross]


def test_probe_pairs_single_node():
    assert latency.probe_pairs(["eu-central-1"], {}, intra=True) == []


# TEST:ping-parse-ok
def test_ping_parse():
    assert latency.parse_ping_avg_ms(PING_OK) == 158.412


# TEST:ping-parse-fails
@pytest.mark.parametrize("text", ["", PING_LOSS])
def test_ping_parse_fails(text):
    with pytest.raises(ValueError, match="rtt summary"):
        latency.parse_ping_avg_ms(text)


def test_probe_error_tolerance():
    probe = Probe(0, 2, "cross", 158.0)
    assert latency.probe_error(probe, 158.4) is None
    assert latency.probe_error(probe, 158.0 + 15.0) is None
    error = latency.probe_error(probe, 3.0)
    assert error is not None
    assert "expected 158.0 ms, measured 3.0 ms" in error
    small = Probe(0, 1, "intra", 10.0)
    assert latency.probe_error(small, 12.0) is None
    assert latency.probe_error(small, 12.5) is not None


def test_matrix_sha256_stable():
    profile = decaf()
    assignment = latency.assign(5, profile)
    on = latency.delays_ms(assignment, profile, intra=True)
    off = latency.delays_ms(assignment, profile, intra=False)
    assert latency.matrix_sha256(on) == DECAF5_SHA256
    assert latency.matrix_sha256(on) != latency.matrix_sha256(off)


def test_shaping_meta_shape():
    shaping = latency.shaping("decaf-2025", 5, intra=True, tcp_cc="cubic")
    assert shaping.meta() == {
        "profile": "decaf-2025",
        "intra": True,
        "nodes": {"eu-central-1": 2, "ap-southeast-1": 2, "us-east-1": 1},
        "assignment": {
            "node0": "eu-central-1",
            "node1": "eu-central-1",
            "node2": "ap-southeast-1",
            "node3": "ap-southeast-1",
            "node4": "us-east-1",
        },
        "matrix_sha256": DECAF5_SHA256,
        "sysctls": dict(latency.tcp_sysctls("cubic")),
        "mtu": 1500,
        "probes": [],
    }
