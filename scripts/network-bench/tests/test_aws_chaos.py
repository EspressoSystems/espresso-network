import json
import shlex
import subprocess
from itertools import pairwise
from typing import Any

import pytest
from fakes import awsb, fake_images, fleet

CFG = awsb.ChaosConfig()


def topo_of(n: int = 10, k: int = 3) -> Any:
    return awsb.topology(fleet(n, k), "sqlite")


def peers_of(n: int = 10, k: int = 3) -> dict[str, list[str]]:
    return {f"node{i}": [f"node{j}" for j in awsb.peers(i, n, k)] for i in range(n)}


def init(cfg=CFG, topo=None, load_s: float = 6000.0) -> Any:
    return awsb.chaos_init(cfg, topo or topo_of(), 0.0, 60.0, load_s)


def observe(
    topo, state=None, tip=100, view=100, query_tip=None, synced=True, **override
) -> Any:
    """Every node shows the tip unless its health in `state` is not `up` or `override[name]`
    is `None` (a failed probe) or a `(height, view, query height, synced)` tuple."""
    query_tip = tip if query_tip is None else query_tip
    queries = set(awsb.nb.query_nodes(topo))
    obs = {"height": {}, "voted_view": {}, "query_height": {}, "synced": {}}
    for name in topo["nodes"]:
        h, v, q, s = tip, view, query_tip, synced
        if name in override:
            h, v, q, s = override[name] or (None, None, None, None)
        obs["height"][name] = h
        obs["voted_view"][name] = v
        if name in queries:
            obs["query_height"][name] = q
            obs["synced"][name] = s
    return obs


def with_health(state, **health) -> Any:
    nodes = {n: s.copy() for n, s in state["nodes"].items()}
    for name, h in health.items():
        nodes[name].update(health=h)
    return {**state, "nodes": nodes}


def step(state, obs, now, cfg=CFG, topo=None, peers=None) -> Any:
    topo = topo or topo_of()
    return awsb.chaos_step(state, cfg, topo, peers or peers_of(), obs, now)


def run(cfg=CFG, n=10, k=3, seconds=2000.0, tick=5.0, recover=True, load_s=6000.0):
    """Ticks the controller; observations follow the state: a not up node shows the tip when
    `recover`, else answers nothing."""
    topo, peers = topo_of(n, k), peers_of(n, k)
    state = awsb.chaos_init(cfg, topo, 0.0, 60.0, load_s)
    now, steps = 0.0, []
    while now < seconds:
        now += tick
        override = (
            {}
            if recover
            else {m: None for m, s in state["nodes"].items() if s["health"] != "up"}
        )
        result = awsb.chaos_step(
            state, cfg, topo, peers, observe(topo, **override), now
        )
        state = result["state"]
        steps.append(result)
    return steps


def faults(steps) -> list[dict]:
    return [e for s in steps for e in s["events"] if e["event"] == "fault"]


# REQ:awsbench-chaos-budget
# TEST:fault-budget-table-ok
def test_fault_budget_table():
    for n in range(4, 26):
        assert awsb.fault_budget(n) == max(0, (n - 1) // 3 - 1)
    assert awsb.fault_budget(22) == 6
    assert awsb.fault_budget(awsb.CHAOS_MIN_NODES) == 1


# REQ:awsbench-chaos-invariants
# TEST:budget-never-exceeded-ok
@pytest.mark.parametrize("n", range(7, 23))
def test_budget_never_exceeded(n):
    k = min(3, n - 4)
    for seed in range(3):
        cfg = awsb.ChaosConfig(seed=seed, recover_timeout_s=1e9)
        for result in run(cfg, n, k, seconds=1500, recover=False):
            faulty = [
                s for s in result["state"]["nodes"].values() if s["health"] != "up"
            ]
            assert len(faulty) <= awsb.fault_budget(n)


# TEST:query-floor-kept-ok
def test_query_floor_kept():
    topo = topo_of(13, 4)
    state = with_health(init(topo=topo), node0="catching_up", node1="catching_up")
    lagging = observe(topo, node0=(100, 100, 0, False), node1=(100, 100, 0, False))
    state["order"] = ["node2", "node3", "node4", "node5"]
    state["next_fault_at"] = 0.0
    result = step(state, lagging, 100.0, topo=topo, peers=peers_of(13, 4))
    assert [e["node"] for e in faults([result])] == ["node4"]
    assert [result["state"]["nodes"][n]["health"] for n in ("node2", "node3")] == [
        "up",
        "up",
    ]


# TEST:every-node-is-a-target-ok
def test_every_node_is_a_target():
    steps = run(seconds=2000, tick=5.0)
    targets = {e["node"] for e in faults(steps)}
    assert len(faults(steps)) > 30
    assert targets == {f"node{i}" for i in range(10)}


# TEST:peers-down-defers-ok
def test_peers_down_defers():
    topo, peers = topo_of(13, 3), peers_of(13, 3)
    target = "node5"
    down = peers[target][:2]
    state = with_health(init(topo=topo), **dict.fromkeys(down, "recovering"))
    state["order"] = [target, "node9"]
    other = "node9"
    state["next_fault_at"] = 0.0
    result = step(state, observe(topo), 100.0, topo=topo, peers=peers)
    assert [e["node"] for e in faults([result])] == [other]
    state = with_health(state, **{down[0]: "up"})
    result = step(state, observe(topo), 100.0, topo=topo, peers=peers)
    assert [e["node"] for e in faults([result])] == [target]


# REQ:awsbench-chaos-gates
# TEST:rejoin-needs-streak-ok
def test_rejoin_needs_streak():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    state["next_fault_at"] = 1e9
    good = observe(topo)
    bad = observe(topo, node5=(10, 10, 0, True))
    first = step(state, good, 100.0)
    assert first["state"]["nodes"]["node5"]["health"] == "recovering"
    assert first["state"]["nodes"]["node5"]["streak"] == 1
    reset = step(first["state"], bad, 105.0)
    assert reset["state"]["nodes"]["node5"]["streak"] == 0
    again = step(reset["state"], good, 110.0)
    second = step(again["state"], good, 115.0)
    assert second["state"]["nodes"]["node5"]["health"] == "up"
    assert [e["event"] for e in second["events"]] == ["rejoined"]


# TEST:rejoin-needs-vote-ok
def test_rejoin_needs_vote():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    obs = observe(topo, node5=(100, 50, 0, True))
    for now in (100.0, 105.0, 110.0):
        result = step(state, obs, now)
        state = result["state"]
    assert state["nodes"]["node5"]["health"] == "recovering"


# TEST:query-catchup-needs-sync-ok
def test_query_catchup_needs_sync():
    topo = topo_of()
    state = with_health(init(topo=topo), node1="catching_up")
    obs = observe(topo, node1=(100, 100, 100, False))
    for now in (100.0, 105.0, 110.0):
        state = step(state, obs, now)["state"]
    assert state["nodes"]["node1"]["health"] == "catching_up"
    obs = observe(topo, node1=(100, 100, 100, True))
    state = step(state, obs, 115.0)["state"]
    result = step(state, obs, 120.0)
    assert result["state"]["nodes"]["node1"]["health"] == "up"
    assert [e["event"] for e in result["events"]] == ["caught_up"]


# TEST:rejoining-query-node-catches-up-ok
def test_rejoined_query_node_catches_up_before_up():
    topo = topo_of()
    state = with_health(init(topo=topo), node1="recovering")
    state = step(state, observe(topo), 100.0)["state"]
    result = step(state, observe(topo), 105.0)
    assert result["state"]["nodes"]["node1"]["health"] == "catching_up"


# TEST:tips-without-node0-ok
def test_tips_without_node0():
    topo = topo_of()
    state = with_health(
        init(topo=topo), node0="down", node1="catching_up", node5="recovering"
    )
    state["next_fault_at"] = 1e9
    state["nodes"]["node0"].update(since=100.0, kind="kill")
    obs = observe(topo, tip=500, node0=None)
    state = step(state, obs, 100.0)["state"]
    result = step(state, obs, 105.0)
    assert result["state"]["nodes"]["node5"]["health"] == "up"
    assert result["state"]["nodes"]["node1"]["health"] == "up"
    assert result["state"]["nodes"]["node0"]["health"] == "down"


# TEST:kill-starts-after-delay-ok
def test_kill_starts_after_delay():
    topo = topo_of()
    state = init(awsb.ChaosConfig(kinds=("kill",)), topo)
    result = step(state, observe(topo), 60.0, awsb.ChaosConfig(kinds=("kill",)))
    (fault,) = result["actions"]
    assert fault["op"] == "kill"
    state = result["state"]
    obs = observe(topo, **{fault["node"]: None})
    early = step(state, obs, 60.0 + 59.0, awsb.ChaosConfig(kinds=("kill",)))
    assert [a for a in early["actions"] if a["op"] == "start"] == []
    late = step(state, obs, 60.0 + 60.0, awsb.ChaosConfig(kinds=("kill",)))
    assert {"node": fault["node"], "op": "start"} in late["actions"]
    assert late["state"]["nodes"][fault["node"]]["health"] == "recovering"
    started = [e for e in late["events"] if e["event"] == "started"]
    assert started[0]["after_s"] == 60.0


# REQ:awsbench-chaos-timeout
# TEST:recover-timeout-fails
# TEST:crashed-recovering-times-out-fails
def test_recover_timeout_fails():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    state["nodes"]["node5"].update(since=100.0, kind="restart")
    obs = observe(topo, node5=None)
    ok = step(state, obs, 100.0 + CFG.recover_timeout_s - 1)
    assert ok["error"] is None
    result = step(state, obs, 100.0 + CFG.recover_timeout_s)
    assert "node5" in result["error"]
    (event,) = [e for e in result["events"] if e["event"] == "timeout"]
    assert event["node"] == "node5"
    assert event["after_s"] == CFG.recover_timeout_s
    assert faults([result]) == []


# TEST:timeout-counts-from-fault-ok
def test_timeout_counts_from_fault():
    topo = topo_of()
    state = with_health(init(topo=topo), node1="recovering")
    state["nodes"]["node1"].update(since=100.0, kind="wipe")
    state["next_fault_at"] = 1e9
    good = observe(topo)
    state = step(state, good, 200.0)["state"]
    state = step(state, good, 205.0)["state"]
    assert state["nodes"]["node1"]["health"] == "catching_up"
    lagging = observe(topo, node1=(100, 100, 0, False))
    ok = step(state, lagging, 100.0 + CFG.recover_timeout_s - 1)
    assert ok["error"] is None
    result = step(state, lagging, 100.0 + CFG.recover_timeout_s)
    assert "node1" in result["error"]


# EDGE:tip-from-up-nodes
# TEST:all-probes-fail-skips-ok
def test_all_probes_fail_skips():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    state["next_fault_at"] = 0.0
    obs = observe(topo, **{n: None for n in topo["nodes"]})
    result = step(state, obs, 100.0)
    assert result["actions"] == [] and result["events"] == []
    assert result["error"] is None
    assert result["state"] == state


def test_timeout_runs_without_tip():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    obs = observe(topo, **{n: None for n in topo["nodes"]})
    result = step(state, obs, CFG.recover_timeout_s)
    assert result["error"]


# TEST:tip-ignores-faulty-ok
def test_tip_ignores_faulty():
    topo = topo_of()
    state = with_health(init(topo=topo), node6="down", node5="recovering")
    state["next_fault_at"] = 1e9
    obs = observe(topo, node6=(10_000, 10_000, 0, True))
    state = step(state, obs, 100.0)["state"]
    result = step(state, obs, 105.0)
    assert result["state"]["nodes"]["node5"]["health"] == "up"


# TEST:no-up-query-node-skips-query-gates-ok
def test_no_up_query_node_skips_query_gates():
    topo = topo_of()
    state = with_health(
        init(topo=topo),
        node0="catching_up",
        node1="down",
        node2="down",
        node5="recovering",
    )
    state["next_fault_at"] = 1e9
    obs = observe(topo, node1=None, node2=None)
    state = step(state, obs, 100.0)["state"]
    result = step(state, obs, 105.0)
    nodes = result["state"]["nodes"]
    assert nodes["node0"]["health"] == "catching_up"
    assert nodes["node0"]["streak"] == 0
    assert nodes["node5"]["health"] == "up"


# EDGE:restart-blocks-tick
# TEST:late-tick-gates-by-now-ok
def test_late_tick_gates_by_now():
    topo = topo_of()
    cfg = awsb.ChaosConfig(kinds=("kill",))
    state = with_health(init(cfg, topo), node5="down")
    state["nodes"]["node5"].update(since=100.0, kind="kill")
    state["next_fault_at"] = 1e9
    result = step(state, observe(topo, node5=None), 400.0 - 1, cfg)
    assert result["actions"] == [{"node": "node5", "op": "start"}]
    assert result["error"] is None


# REQ:awsbench-chaos-schedule
# TEST:kinds-cycle-ok
def test_kinds_cycle():
    steps = run(seconds=400)
    assert [e["kind"] for e in faults(steps)][:3] == ["restart", "kill", "wipe"]
    ops = [a["op"] for s in steps for a in s["actions"] if a["op"] != "start"]
    assert ops[:3] == ["restart", "kill", "wipe"]


# TEST:gap-respected-ok
def test_gap_respected():
    times = [e["ts"] for e in faults(run(seconds=1500))]
    assert times[0] == 60.0
    assert all(b - a >= CFG.gap_s for a, b in pairwise(times))


# TEST:stops-before-tail-ok
def test_stops_before_tail():
    steps = run(seconds=1500, load_s=600.0)
    stop_at = 60.0 + 600.0 - awsb.CHAOS_TAIL_S
    times = [e["ts"] for e in faults(steps)]
    assert times and max(times) < stop_at
    state = init(load_s=600.0)
    assert state["stop_at"] == stop_at


# TEST:same-seed-same-order-ok
def test_same_seed_same_order():
    def targets(seed):
        cfg = awsb.ChaosConfig(seed=seed)
        return [e["node"] for e in faults(run(cfg, seconds=1000))]

    assert targets(7) == targets(7)
    assert targets(7) != targets(8)


# TEST:round-robin-order-ok
def test_round_robin_order():
    cfg = awsb.ChaosConfig(kinds=("restart",))
    targets = [e["node"] for e in faults(run(cfg, seconds=1500))]
    order = init(cfg)["order"]
    assert len(targets) > 20
    assert targets == [order[i % len(order)] for i in range(len(targets))]


# EDGE:deferred-fault-budget-full
# TEST:deferred-fault-issued-later-ok
def test_deferred_fault_issued_later():
    n = 7
    topo, peers = topo_of(n, 3), peers_of(n, 3)
    state = init(topo=topo)
    state["next_fault_at"] = 60.0
    first = step(state, observe(topo), 60.0, topo=topo, peers=peers)
    (fault,) = faults([first])
    full = first["state"]
    assert full["next_fault_at"] == 60.0 + CFG.gap_s
    blocked = step(full, observe(topo), 200.0, topo=topo, peers=peers)
    assert faults([blocked]) == []
    assert blocked["state"]["next_fault_at"] == full["next_fault_at"]
    healed = with_health(blocked["state"], **{fault["node"]: "up"})
    later = step(healed, observe(topo), 205.0, topo=topo, peers=peers)
    assert len(faults([later])) == 1


# REQ:awsbench-chaos-events
# TEST:event-fields-ok
def test_event_fields():
    events = [e for s in run(seconds=800) for e in s["events"]]
    kinds = {e["event"] for e in events}
    assert {"fault", "rejoined"} <= kinds
    for e in events:
        assert {"ts", "iso", "event", "node"} <= e.keys()
        assert e["iso"].startswith("1970-01-01T")
        if e["event"] in ("rejoined", "caught_up"):
            assert e["after_s"] is not None
        if e["event"] == "fault":
            assert e["height"] == 100


# EDGE:probe-partial-output
# TEST:probe-dash-is-none-ok
def test_probe_dash_is_none():
    topo = topo_of(4, 2)
    out = "node0 10 20 9 true\nnode1 - - - -\nnode2 5 6\nnode3 - 7\n"
    obs = awsb.parse_probe(out, topo)
    assert obs["height"] == {"node0": 10, "node1": None, "node2": 5, "node3": None}
    assert obs["voted_view"]["node3"] == 7
    assert obs["query_height"] == {"node0": 9, "node1": None}
    assert obs["synced"] == {"node0": True, "node1": None}
    assert (
        awsb.parse_probe(out.replace("true", "false"), topo)["synced"]["node0"] is False
    )


# TEST:probe-missing-line-fails
def test_probe_missing_line_fails():
    topo = topo_of(4, 2)
    with pytest.raises(ValueError, match="node3"):
        awsb.parse_probe("node0 1 2 3 true\nnode1 1 2 3 true\nnode2 1 2\n", topo)
    with pytest.raises(ValueError, match="node1"):
        awsb.parse_probe("node0 1 2 3 true\nnode1 1 2\nnode2 1 2\nnode3 1 2\n", topo)


def test_probe_command_covers_every_node():
    topo = topo_of(4, 2)
    *lines, last = awsb.probe_command(topo).splitlines()
    assert last == "wait"
    assert len(lines) == 4
    assert "/v1/node/sync-status" in lines[0] and "/v1/node/sync-status" not in lines[2]
    assert "consensus_last_voted_view" in lines[2]
    assert all(topo["nodes"][f"node{i}"] in lines[i] for i in range(4))
    for line in lines:
        shlex.split(line)


# REQ:awsbench-chaos-wipe
# TEST:wipe-script-steps-ok
def test_wipe_script_steps():
    lines = awsb.wipe_script(True).splitlines()
    assert lines[0] == "set -eu"
    expected = [
        "docker logs espresso-node 2>&1 | gzip >> /opt/bench/espresso-node.wiped.log.gz",
        "docker rm -f espresso-node",
        "find /data/journal -mindepth 1 -delete",
        f"find {awsb.PAYLOAD_HOST_DIR} -mindepth 1 -delete",
        "bash /opt/bench/recreate.sh",
        "docker start espresso-node",
    ]
    assert lines[1:] == expected
    assert awsb.wipe_script(False).splitlines()[1:] == [
        c for c in expected if awsb.PAYLOAD_HOST_DIR not in c
    ]


# TEST:rejoin-env-config-peers-ok
def test_rejoin_env_config_peers():
    hosts = fleet(8, 2)
    spec = {"name": "node4", "role": "validator"}

    def env(**kw):
        text = awsb.render_node_env(spec, hosts, None, query_engine="sqlite", **kw)
        return dict(line.split("=", 1) for line in text.splitlines())

    assert "ESPRESSO_NODE_CONFIG_PEERS" not in env()
    rejoin = env(config_peers=True)
    assert rejoin["ESPRESSO_NODE_CONFIG_PEERS"] == rejoin["ESPRESSO_NODE_STATE_PEERS"]
    assert {
        k: v for k, v in rejoin.items() if k != "ESPRESSO_NODE_CONFIG_PEERS"
    } == env()


# TEST:recreate-sh-same-argv-ok
@pytest.mark.parametrize("role", ["query", "validator"])
def test_recreate_sh_same_argv(role):
    spec: Any = {"name": "node1", "role": role}
    args = (spec, fake_images())
    start = awsb.render_start_sh(*args, 16384, query_engine="sqlite")
    recreate = awsb.render_recreate_sh(*args, query_engine="sqlite")

    def create(script: str) -> str:
        (line,) = [
            ln
            for ln in script.splitlines()
            if "docker create --name espresso-node" in ln
        ]
        return line

    assert create(recreate) == create(start).replace("node.env", "node-rejoin.env")
    assert recreate.startswith("#!/usr/bin/env bash\nset -eEu")
    assert "containers.json" in recreate


def test_synced_filter_matches_rust_method():
    def synced(missing: tuple[int, int, int]) -> str:
        blocks, leaves, vid = ({"missing": m, "ranges": []} for m in missing)
        doc = {
            "blocks": blocks,
            "leaves": leaves,
            "vid_common": vid,
            "pruned_height": 0,
        }
        out = subprocess.run(
            ["jq", "-r", awsb.SYNCED_JQ],
            input=json.dumps(doc),
            text=True,
            capture_output=True,
            check=True,
        )
        return out.stdout.strip()

    assert synced((0, 0, 0)) == "true"
    assert synced((0, 1, 0)) == "false"
    assert shlex.quote(awsb.SYNCED_JQ) in awsb.probe_command(topo_of(4, 2))
