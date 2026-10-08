import dataclasses
import json
import shlex
import signal
import subprocess
from itertools import pairwise
from pathlib import Path
from typing import Any

import netbench
import pytest
from fakes import (
    DESCRIBE,
    DONE_STATE,
    FAKE_EPOCH,
    MEMORY_MIB,
    ClusterRunner,
    FakeClock,
    FakeCluster,
    FakeRunner,
    FakeSystem,
    FleetHarness,
    RunHarness,
    Scripted,
    aws_manifest,
    awsb,
    clean_evidence,
    clean_result,
    completed,
    fake_images,
    fleet,
    write_collected_run,
)

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


LOADING = {"phase": "loading", "detail": "x"}


def parse(*argv: str) -> Any:
    return awsb.parse_args(["run", "--tag", "t", *argv])


# REQ:awsbench-chaos-shape
# TEST:chaos-shape-defaults-ok
def test_chaos_shape_defaults():
    cfg = awsb.config_from_args(parse("--chaos"))
    assert (cfg.nodes, cfg.query_nodes) == (22, 4)
    assert (cfg.node_type, cfg.ctl_type) == ("c8g.xlarge", "c8g.xlarge")
    assert cfg.query_engine == "sqlite"
    assert cfg.chaos == awsb.ChaosConfig()
    assert cfg.load.steps == (4.0,) * 10
    assert cfg.load.step_s == awsb.CHAOS_STEP_S
    assert cfg.load.keep_going
    assert cfg.load.submit_nodes == 22
    assert awsb.fleet_config_from_args(parse("--chaos")).query_engine == "sqlite"


def test_plain_run_shape_is_unchanged():
    cfg = awsb.config_from_args(parse())
    assert cfg.chaos is None
    assert (cfg.nodes, cfg.query_nodes, cfg.node_type) == (5, 1, awsb.NODE_TYPE)
    assert not cfg.load.keep_going


# TEST:chaos-overrides-kept-ok
def test_chaos_overrides_kept():
    args = parse(
        "--chaos",
        "--nodes",
        "7",
        "--ctl-type=c8g.2xlarge",
        "--chaos-rate",
        "2",
        "--submit-nodes",
        "3",
    )
    cfg = awsb.config_from_args(args)
    assert cfg.nodes == 7 and cfg.ctl_type == "c8g.2xlarge"
    assert cfg.load.submit_nodes == 3 and cfg.load.steps == (2.0,) * 10
    assert cfg.query_nodes == 4 and cfg.node_type == "c8g.xlarge"


# REQ:awsbench-chaos-refusals
# TEST:chaos-steps-refused-fails
@pytest.mark.parametrize(
    "flags",
    [
        ["--steps", "8"],
        ["--step-s", "30"],
        ["--search"],
        ["--keep-going"],
        ["--warmup-s", "10"],
    ],
)
def test_chaos_load_flags_refused(flags):
    with pytest.raises(awsb.Refused, match=flags[0]):
        awsb.config_from_args(parse("--chaos", *flags))


def test_chaos_flags_need_chaos():
    with pytest.raises(awsb.Refused, match="--chaos-rate needs --chaos"):
        awsb.config_from_args(parse("--chaos-rate", "2"))


# TEST:chaos-postgres-refused-fails
@pytest.mark.parametrize(
    ("change", "reason"),
    [
        ({"query_engine": "postgres"}, "--query-engine sqlite"),
        ({"query_db": "tmpfs"}, "--query-db colocated"),
        ({"query_db": "volume"}, "--query-db colocated"),
        ({"query_db": "rds"}, "--query-db colocated"),
    ],
)
def test_chaos_store_refused(change, reason):
    cfg = dataclasses.replace(chaos_cfg(), **change)
    with pytest.raises(awsb.Refused, match=reason):
        awsb.check_chaos(cfg)


# TEST:chaos-small-fleet-refused-fails
@pytest.mark.parametrize(
    ("flags", "reason"),
    [(["--nodes", "5"], "at least 7"), (["--query-nodes", "2"], "query nodes")],
)
def test_chaos_small_fleet_refused(flags, reason):
    cfg = awsb.fleet_config_from_args(parse("--chaos", *flags))
    with pytest.raises(awsb.Refused, match=reason):
        awsb.plan_hosts(cfg)


def test_chaos_refusal_reaches_no_aws_call(run_harness: RunHarness):
    runner = FakeRunner(states=[DONE_STATE])
    with pytest.raises(awsb.Refused):
        run_harness.run(runner, "--chaos", nodes=5)
    assert not runner.ran("tofu", "apply")


def test_chaos_on_a_small_saved_fleet_refused(
    isolated: Path, monkeypatch: pytest.MonkeyPatch
):
    harness = FleetHarness(monkeypatch, isolated)
    harness.up_fleet()
    with pytest.raises(awsb.Refused, match="at least 7"):
        harness.run(FakeRunner(states=[DONE_STATE]), "--chaos")


# TEST:fleet-query-nodes-flag-fails
def test_fleet_run_refuses_query_nodes(isolated: Path, monkeypatch: pytest.MonkeyPatch):
    harness = FleetHarness(monkeypatch, isolated)
    harness.up_fleet()
    with pytest.raises(awsb.Refused, match="--query-nodes"):
        harness.run(FakeRunner(states=[DONE_STATE]), "--chaos", "--query-nodes", "4")


# TEST:old-manifest-defaults-ok
def test_old_manifest_defaults():
    saved = awsb.config_to_json(awsb.RunConfig(tag="x"))
    del saved["query_nodes"], saved["chaos"]
    cfg = awsb.config_from_manifest(saved)
    assert (cfg.query_nodes, cfg.chaos) == (1, None)


def test_manifest_config_round_trips_chaos():
    chaos = awsb.ChaosConfig(minutes=3, seed=7, kinds=("kill", "wipe"))
    cfg = awsb.RunConfig(tag="x", nodes=9, query_nodes=3, chaos=chaos)
    saved = json.loads(json.dumps(awsb.config_to_json(cfg)))
    assert awsb.config_from_manifest(saved).chaos == chaos


# REQ:awsbench-chaos-wipe
def chaos_cfg(**kw: Any) -> Any:
    return awsb.RunConfig(
        tag="x",
        nodes=7,
        query_nodes=3,
        query_engine="sqlite",
        chaos=awsb.ChaosConfig(),
        load=netbench.BenchConfig(submit_nodes=7),
        **kw,
    )


def render(run_dir: Path, cfg: Any) -> None:
    hosts = awsb.plan_hosts(cfg)
    for host in hosts:
        (run_dir / "hosts" / host["name"]).mkdir(parents=True)
    manifest = {"hosts": hosts, "images": fake_images(), "memory_mib": MEMORY_MIB}
    awsb.render_host_files(run_dir, cfg, manifest, fleet(7, 3))


def test_render_host_files_writes_rejoin_files_only_for_chaos(tmp_path: Path):
    render(tmp_path, chaos_cfg())
    for name in ("node0", "node5"):
        assert (tmp_path / "hosts" / name / "node-rejoin.env").exists()
        assert (
            "node-rejoin.env" in (tmp_path / "hosts" / name / "recreate.sh").read_text()
        )
    assert not (tmp_path / "hosts/ctl/recreate.sh").exists()
    plain = tmp_path / "plain"
    render(plain, dataclasses.replace(chaos_cfg(), chaos=None))
    assert not (plain / "hosts/node0/recreate.sh").exists()
    assert not (plain / "hosts/node0/node-rejoin.env").exists()


# REQ:awsbench-chaos-check-nodes
# TEST:check-nodes-expected-down-ok
def test_check_nodes_ignores_expected_down(isolated: Path):
    hosts = fleet(6, 3)
    runner = Scripted({".State.Status": [completed("exited 137")]})
    rem = awsb.Remote(
        runner, isolated.relative_to(Path.cwd()), Path("~/.ssh/id"), hosts
    )
    names = frozenset(f"node{i}" for i in range(6))
    with pytest.raises(awsb.RemoteError, match="node5 espresso-node exited 137"):
        awsb.check_nodes(rem, names - {"node5"})
    awsb.check_nodes(rem, names)


def make_controller(
    isolated: Path, runner: Any, hosts: dict, clock: FakeClock, cfg: Any = None
) -> Any:
    n = len(hosts) - 1
    k = sum(h["role"] == "query" for h in hosts.values())
    rem = awsb.Remote(
        runner, isolated.relative_to(Path.cwd()), Path("~/.ssh/id"), hosts
    )
    cfg = cfg or chaos_cfg()
    return awsb.ChaosRunner(
        rem, cfg.chaos, cfg.load, topo_of(n, k), peers_of(n, k), clock, isolated
    )


def scripted_controller(isolated: Path) -> tuple[Any, Any]:
    runner = Scripted({"date +%s.%N": [completed("1000.5")]})
    clock = FakeClock(start=1000.0)
    return make_controller(isolated, runner, fleet(7, 3), clock), runner


# REQ:awsbench-chaos-restore
# TEST:restore-starts-down-ok
def test_restore_starts_down_nodes(isolated: Path):
    ctrl, runner = scripted_controller(isolated)
    state = awsb.chaos_init(awsb.ChaosConfig(), ctrl.topo, 0.0, 60.0, 600.0)
    ctrl.state = with_health(state, node4="down", node5="down", node6="recovering")
    ctrl.restore()
    starts = [c[-1] for c in runner.calls if "docker start" in c[-1]]
    assert starts == ["sudo docker start espresso-node"] * 2
    events = awsb.chaos_events(isolated)
    assert [(e["event"], e["node"]) for e in events] == [
        ("restored", "node4"),
        ("restored", "node5"),
    ]


# TEST:tick-waiting-noop-ok
@pytest.mark.parametrize("agent", [None, {"phase": "waiting", "detail": "x"}])
def test_tick_before_loading_does_nothing(isolated: Path, agent):
    ctrl, runner = scripted_controller(isolated)
    calls = len(runner.calls)
    ctrl.tick(agent)
    assert ctrl.state is None and ctrl.expected_down() == frozenset()
    assert len(runner.calls) == calls
    assert awsb.chaos_events(isolated) == []


def cluster_controller(isolated: Path, **chaos: Any) -> tuple[Any, Any, Any]:
    clock = FakeClock(start=FAKE_EPOCH, limit_s=7200)
    names = [f"node{i}" for i in range(7)]
    cluster = FakeCluster(clock, names, {"node0", "node1", "node2"})
    runner = ClusterRunner(cluster, 3, [LOADING])
    cfg = dataclasses.replace(chaos_cfg(), chaos=awsb.ChaosConfig(**chaos))
    ctrl = make_controller(isolated, runner, runner.hosts, clock, cfg)
    ctrl.state = awsb.chaos_init(cfg.chaos, ctrl.topo, clock.time(), 60.0, 0.0)
    cluster.kill("node4")
    ctrl.state = with_health(ctrl.state, node4="down")
    ctrl.state["nodes"]["node4"].update(kind="kill", since=clock.time())
    return ctrl, cluster, clock


def interrupts_of(clock: FakeClock) -> Any:
    return awsb.Interrupts(clock, lambda signum: [])


# TEST:drain-waits-then-returns-ok
def test_drain_waits_for_the_recovery(isolated: Path):
    ctrl, cluster, clock = cluster_controller(isolated)
    ctrl.drain(interrupts_of(clock))
    assert ctrl.expected_down() == frozenset()
    assert cluster.members["node4"].running
    assert [e["event"] for e in awsb.chaos_events(isolated)] == ["started", "rejoined"]
    assert clock.time() - FAKE_EPOCH >= awsb.ChaosConfig().kill_down_s


# TEST:drain-timeout-fails
def test_drain_timeout_fails(isolated: Path):
    ctrl, _, clock = cluster_controller(isolated, kill_down_s=1e9, recover_timeout_s=60)
    with pytest.raises(awsb.RemoteError, match="node4"):
        ctrl.drain(interrupts_of(clock))
    assert [e["event"] for e in awsb.chaos_events(isolated)] == ["timeout"]


# REQ:awsbench-chaos-report
# TEST:chaos-section-rows-ok
def test_chaos_section_rows():
    def event(ts, name, node, kind):
        return awsb._chaos_event(ts, name, node, kind, 10, None)

    events = [
        event(100, "fault", "node0", "restart"),
        event(112, "rejoined", "node0", "restart"),
        event(130, "caught_up", "node0", "restart"),
        event(150, "fault", "node5", "kill"),
        event(210, "started", "node5", "kill"),
        event(222, "rejoined", "node5", "kill"),
        event(200, "fault", "node6", "wipe"),
    ]
    text = awsb.chaos_section(events, 90.0, {"submit_failovers": 17})
    assert "### Chaos" in text and "rejoined (s)" in text
    rows = [ln for ln in text.splitlines() if ln.startswith("| node")]
    assert rows[1:] == [
        "| node0 | restart | 10 | - | 22 | 40 |",
        "| node5 | kill | 60 | 120 | 132 | - |",
        "| node6 | wipe | 110 | - | - | - |",
    ]
    assert "faults 3, max concurrent faulty 2, submit failovers 17, timeouts 0" in text


def test_chaos_section_counts_timeouts():
    event = awsb._chaos_event(300.0, "timeout", "node4", "kill", None, 300.0)
    text = awsb.chaos_section([event], 0.0, {"submit_failovers": 0})
    assert "faults 0" in text and "timeouts 1" in text


# TEST:timeout-event-invalid-ok
def test_timeout_event_makes_the_run_invalid():
    event = awsb._chaos_event(300.0, "timeout", "node4", "kill", None, 300.0)
    args = (clean_result(), aws_manifest(), clean_evidence())
    assert awsb.check_validity_aws(*args)["valid"]
    verdict = awsb.check_validity_aws(*args, [event])
    assert not verdict["valid"]
    assert "chaos: node4 not recovered after 300 s" in verdict["reasons"]


def test_write_report_has_the_chaos_section_and_exempts_faulted_nodes(tmp_path: Path):
    manifest = write_collected_run(tmp_path)
    degraded = (
        {**r, "ok": False} if r["node"] == "node1" else r
        for r in list(netbench.read_jsonl(tmp_path / "metrics.jsonl"))
    )
    netbench.write_jsonl(tmp_path / "metrics.jsonl", degraded)
    manifest["config"] = {
        **manifest["config"],
        "chaos": dataclasses.asdict(awsb.ChaosConfig()),
    }
    netbench.write_json(tmp_path / "manifest.json", manifest)
    meta = netbench.read_json(tmp_path / "load-meta.json")
    netbench.write_json(tmp_path / "load-meta.json", {**meta, "submit_failovers": 5})
    run = netbench.read_json(tmp_path / "run.json")
    ev = awsb._chaos_event
    events = [
        ev(run["t0"] + 5, "fault", "node1", "restart", 9, None),
        ev(run["t0"] + 20, "rejoined", "node1", "restart", None, 15.0),
    ]
    (tmp_path / "chaos.jsonl").write_text("".join(json.dumps(e) + "\n" for e in events))
    result = awsb.write_report(tmp_path)
    summary = (tmp_path / "summary.md").read_text()
    assert "### Chaos" in summary and "| node1 | restart | 5 | - | 20 | - |" in summary
    assert "submit failovers 5" in summary
    assert result["validity"]["valid"]
    (tmp_path / "chaos.jsonl").unlink()
    assert not awsb.write_report(tmp_path)["validity"]["valid"]


# REQ:awsbench-chaos-estimate
# TEST:measure-seconds-chaos-ok
def test_measure_seconds_grow_under_chaos():
    plain = chaos_cfg()
    plain = dataclasses.replace(plain, chaos=None)
    chaos = chaos_cfg()
    (e0, w0), (e1, w1) = awsb._measure_seconds(plain), awsb._measure_seconds(chaos)
    assert e1 - e0 == awsb.CHAOS_DRAIN_EXPECTED_S
    assert w1 - w0 == chaos.chaos.recover_timeout_s


# TEST:hash-includes-chaos-ok
def test_hash_includes_chaos_and_query_nodes():
    base = chaos_cfg()
    hosts = awsb.plan_hosts(base)

    def digest(cfg):
        return awsb.run_config_hash(cfg, hosts, {}, b"genesis")

    hashes = {
        digest(base),
        digest(dataclasses.replace(base, chaos=awsb.ChaosConfig(seed=7))),
        digest(dataclasses.replace(base, chaos=awsb.ChaosConfig(rate_mb_s=2.0))),
        digest(dataclasses.replace(base, query_nodes=4)),
        digest(dataclasses.replace(base, chaos=None)),
    }
    assert len(hashes) == 5
    assert digest(base) == digest(chaos_cfg())


# REQ:awsbench-chaos-run
# TEST:chaos-run-end-to-end-ok
@pytest.mark.usefixtures("valid")
def test_chaos_run_end_to_end(run_harness: RunHarness):
    seed = seed_faulting_a_query_node()
    clock = FakeClock(start=FAKE_EPOCH, limit_s=20000)
    cluster = FakeCluster(
        clock, [f"node{i}" for i in range(7)], {"node0", "node1", "node2"}
    )
    states = [LOADING] * 80 + [DONE_STATE]
    runner = ClusterRunner(cluster, 3, states, describe=DESCRIBE)
    args = run_harness.args(
        "--chaos",
        "--chaos-min",
        "6",
        "--chaos-seed",
        str(seed),
        "--query-nodes",
        "3",
        nodes=7,
    )
    assert awsb.cmd_run(args, FakeSystem(run=runner, clock=clock)) == awsb.EXIT_OK
    events = awsb.chaos_events(run_harness.run_dir)
    kinds = {e["event"] for e in events}
    assert {"fault", "started", "rejoined", "caught_up"} <= kinds
    fault_kinds = [e["kind"] for e in events if e["event"] == "fault"]
    assert fault_kinds[:3] == ["restart", "kill", "wipe"]
    assert "timeout" not in kinds
    for needle in ("docker kill", "docker restart", "docker start", "recreate.sh"):
        assert runner.ran(needle), needle
    assert runner.ran("tofu", "destroy")
    assert all(m.running for m in cluster.members.values())
    log = run_harness.log()
    assert "chaos: kill node" in log and "chaos: restart node" in log
    assert "caught up after" in log and "rejoined after" in log
    assert (run_harness.run_dir / "hosts/node0/recreate.sh").exists()


def seed_faulting_a_query_node() -> int:
    """First seed whose first fault of the run order hits a query node, so that the run has a
    `caught_up` event."""
    topo = topo_of(7, 3)
    queries = set(awsb.nb.query_nodes(topo))
    for seed in range(100):
        order = awsb.chaos_init(awsb.ChaosConfig(seed=seed), topo, 0.0, 0.0, 0.0)[
            "order"
        ]
        if order[0] in queries:
            return seed
    raise AssertionError("no seed")


class CtlDropsOnce(ClusterRunner):
    """The fifth agent poll fails over ssh, and ctl stays unreachable for the rest of that
    poll."""

    unreachable = False
    dropped = False
    polls = 0

    def default(self, argv: list[str]) -> Any:
        command = argv[-1] if argv[0] == "ssh" else ""
        if "is-active" in command:
            self.polls += 1
            self.unreachable = self.polls == 5
            self.dropped = self.dropped or self.unreachable
        elif self.unreachable and "consensus_last_voted_view" in command:
            return completed(returncode=awsb.SSH_FAILED_RC, stderr="Connection reset")
        if self.unreachable and "is-active" in command:
            return completed(returncode=awsb.SSH_FAILED_RC, stderr="Connection reset")
        return super().default(argv)


@pytest.mark.usefixtures("valid")
def test_ctl_ssh_failure_while_loading_is_retried_under_chaos(
    run_harness: RunHarness,
):
    clock = FakeClock(start=FAKE_EPOCH, limit_s=20000)
    cluster = FakeCluster(
        clock, [f"node{i}" for i in range(7)], {"node0", "node1", "node2"}
    )
    runner = CtlDropsOnce(cluster, 3, [LOADING] * 80 + [DONE_STATE], describe=DESCRIBE)
    args = run_harness.args(
        "--chaos", "--chaos-min", "6", "--query-nodes", "3", nodes=7
    )
    assert awsb.cmd_run(args, FakeSystem(run=runner, clock=clock)) == awsb.EXIT_OK
    assert runner.dropped


# TEST:interrupt-mid-chaos-restores-ok
@pytest.mark.usefixtures("valid")
def test_interrupt_mid_chaos_restores_killed_nodes(run_harness: RunHarness):
    clock = FakeClock(start=FAKE_EPOCH, limit_s=20000)
    cluster = FakeCluster(
        clock, [f"node{i}" for i in range(7)], {"node0", "node1", "node2"}
    )
    system = FakeSystem(run=FakeRunner(), clock=clock)

    def interrupt_while_killed(polls: int) -> None:
        if polls == 25:
            assert any(not m.running for m in cluster.members.values())
            system.fire(signal.SIGINT)

    runner = ClusterRunner(
        cluster,
        3,
        [LOADING] * 80 + [DONE_STATE],
        describe=DESCRIBE,
        on_poll=interrupt_while_killed,
    )
    system.run = runner
    args = run_harness.args(
        "--chaos", "--chaos-min", "6", "--query-nodes", "3", nodes=7
    )
    assert awsb.cmd_run(args, system) == awsb.EXIT_FAILED
    assert runner.ran("tofu", "destroy")
    assert all(m.running for m in cluster.members.values())
    assert "restored" in {e["event"] for e in awsb.chaos_events(run_harness.run_dir)}
