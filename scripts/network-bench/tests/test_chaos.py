import json
import shlex
import subprocess
from itertools import pairwise
from typing import Any

import chaos as ch
import netbench
import pytest
from fakes import awsb, fleet

CFG = ch.ChaosConfig()


def topo_of(n: int = 10, k: int = 3) -> Any:
    return awsb.topology(fleet(n, k), "sqlite")


def peers_of(n: int = 10, k: int = 3) -> dict[str, list[str]]:
    return {f"node{i}": [f"node{j}" for j in awsb.peers(i, n, k)] for i in range(n)}


def queries_of(topo) -> frozenset[str]:
    return frozenset(netbench.query_nodes(topo))


def init(cfg=CFG, topo=None, load_s: float = 6000.0) -> Any:
    return ch.chaos_init(cfg, list((topo or topo_of())["nodes"]), 0.0, 60.0, load_s)


def observe(
    topo, state=None, tip=100, view=100, query_tip=None, missing=(0, 0, 0), **override
) -> Any:
    """Every node shows the tip unless its health in `state` is not `up` or `override[name]`
    is `None` (a failed probe) or a `(height, view, query height, missing)` tuple."""
    query_tip = tip if query_tip is None else query_tip
    queries = queries_of(topo)
    obs = {"height": {}, "voted_view": {}, "query_height": {}, "missing": {}}
    for name in topo["nodes"]:
        h, v, q, s = tip, view, query_tip, missing
        if name in override:
            h, v, q, s = override[name] or (None, None, None, None)
        obs["height"][name] = h
        obs["voted_view"][name] = v
        if name in queries:
            obs["query_height"][name] = q
            obs["missing"][name] = s
    return obs


def with_health(state, **health) -> Any:
    nodes = {n: s.copy() for n, s in state["nodes"].items()}
    for name, h in health.items():
        nodes[name].update(health=h)
        if h == "catching_up":
            nodes[name].update(up_at=0.0)
    return {**state, "nodes": nodes}


def step(state, obs, now, cfg=CFG, topo=None, peers=None) -> Any:
    topo = topo or topo_of()
    return ch.chaos_step(state, cfg, queries_of(topo), peers or peers_of(), obs, now)


def run(cfg=CFG, n=10, k=3, seconds=2000.0, tick=5.0, recover=True, load_s=6000.0):
    """Ticks the controller; observations follow the state: a not up node shows the tip when
    `recover`, else answers nothing."""
    topo, peers = topo_of(n, k), peers_of(n, k)
    state = init(cfg, topo, load_s)
    now, steps = 0.0, []
    while now < seconds:
        now += tick
        override = (
            {}
            if recover
            else {m: None for m, s in state["nodes"].items() if s["health"] != "up"}
        )
        result = step(state, observe(topo, **override), now, cfg, topo, peers)
        state = result["state"]
        steps.append(result)
    return steps


def faults(steps) -> list[dict]:
    return [e for s in steps for e in s["events"] if e["event"] == "fault"]


# REQ:awsbench-chaos-budget
# TEST:fault-budget-table-ok
def test_fault_budget_table():
    for n in range(4, 26):
        assert ch.fault_budget(n) == max(0, (n - 1) // 3 - 1)
    assert ch.fault_budget(22) == 6
    assert ch.fault_budget(awsb.CHAOS_MIN_NODES) == 1


# REQ:awsbench-chaos-invariants
# TEST:budget-never-exceeded-ok
@pytest.mark.parametrize("n", range(7, 23))
def test_budget_never_exceeded(n):
    k = min(3, n - 4)
    for seed in range(3):
        cfg = ch.ChaosConfig(seed=seed, recover_timeout_s=1e9)
        for result in run(cfg, n, k, seconds=1500, recover=False):
            faulty = [
                s for s in result["state"]["nodes"].values() if s["health"] != "up"
            ]
            assert len(faulty) <= ch.fault_budget(n)


# TEST:query-floor-kept-ok
def test_query_floor_kept():
    topo = topo_of(13, 4)
    state = with_health(init(topo=topo), node0="catching_up", node1="catching_up")
    lagging = observe(
        topo, node0=(100, 100, 0, (5, 0, 7)), node1=(100, 100, 0, (5, 0, 7))
    )
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
    # Not a peer of the down nodes: node9 would leave node6 one up peer.
    other = "node11"
    state["order"] = [target, other]
    state["next_fault_at"] = 0.0
    result = step(state, observe(topo), 100.0, topo=topo, peers=peers)
    assert [e["node"] for e in faults([result])] == [other]
    state = with_health(state, **{down[0]: "up"})
    result = step(state, observe(topo), 100.0, topo=topo, peers=peers)
    assert [e["node"] for e in faults([result])] == [target]


def test_fault_keeps_two_up_peers_for_every_faulty_node():
    topo, peers = topo_of(13, 3), peers_of(13, 3)
    assert peers["node9"] == ["node10", "node11", "node12"]
    state = with_health(init(topo=topo), node9="recovering", node12="recovering")
    state["order"] = ["node10", "node6"]
    state["next_fault_at"] = 0.0
    result = step(state, observe(topo), 100.0, topo=topo, peers=peers)
    assert [e["node"] for e in faults([result])] == ["node6"]


def test_up_node_without_answer_or_behind_the_tip_counts_as_faulty():
    topo = topo_of()
    assert ch.fault_budget(10) == 2
    state = init(topo=topo)
    state["order"] = ["node3", "node4"]
    state["next_fault_at"] = 0.0
    silent = step(state, observe(topo, node3=None), 100.0)
    assert [e["node"] for e in faults([silent])] == ["node4"]
    lagging = (100 - ch.LAG_BLOCKS - 1, 100, 0, (0, 0, 0))
    full = step(state, observe(topo, node3=None, node5=lagging), 100.0)
    assert faults([full]) == []
    near = (100 - ch.LAG_BLOCKS, 100, 0, (0, 0, 0))
    room = step(state, observe(topo, node3=None, node5=near), 100.0)
    assert [e["node"] for e in faults([room])] == ["node4"]


# REQ:awsbench-chaos-gates
# TEST:rejoin-needs-streak-ok
def test_rejoin_needs_streak():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    state["next_fault_at"] = 1e9
    good = observe(topo)
    bad = observe(topo, node5=(10, 10, 0, (0, 0, 0)))
    first = step(state, good, 100.0)
    assert first["state"]["nodes"]["node5"]["health"] == "recovering"
    assert first["state"]["nodes"]["node5"]["streak"] == 1
    reset = step(first["state"], bad, 105.0)
    assert reset["state"]["nodes"]["node5"]["streak"] == 0
    again = step(reset["state"], good, 110.0)
    second = step(again["state"], good, 115.0)
    assert second["state"]["nodes"]["node5"]["health"] == "up"
    (event,) = second["events"]
    assert (event["event"], event["ts"], event["after_s"]) == ("rejoined", 110.0, 110.0)


# TEST:rejoin-needs-vote-ok
def test_rejoin_needs_vote():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    obs = observe(topo, node5=(100, 50, 0, (0, 0, 0)))
    for now in (100.0, 105.0, 110.0):
        result = step(state, obs, now)
        state = result["state"]
    assert state["nodes"]["node5"]["health"] == "recovering"


# TEST:query-catchup-needs-sync-ok
def test_query_catchup_needs_sync():
    topo = topo_of()
    state = with_health(init(topo=topo), node1="catching_up")
    obs = observe(topo, node1=(100, 100, 100, (5, 0, 7)))
    for now in (100.0, 105.0, 110.0):
        state = step(state, obs, now)["state"]
    assert state["nodes"]["node1"]["health"] == "catching_up"
    obs = observe(topo, node1=(100, 100, 100, (0, 0, 0)))
    state = step(state, obs, 115.0)["state"]
    result = step(state, obs, 120.0)
    assert result["state"]["nodes"]["node1"]["health"] == "up"
    (event,) = result["events"]
    assert (event["event"], event["ts"], event["after_s"]) == (
        "caught_up",
        115.0,
        115.0,
    )


def wiped(at: float = 100.0) -> Any:
    """node1 wiped at `at`."""
    state = with_health(init(), node1="recovering")
    state["next_fault_at"] = 1e9
    state["nodes"]["node1"].update(kind="wipe", since=at, started=at)
    return state


# TEST:stale-synced-after-rejoin-fails
def test_catch_up_ignores_a_synced_answer_before_the_settle_time():
    topo = topo_of()
    state = wiped()
    obs = observe(topo, node1=(100, 100, 100, (0, 0, 0)))
    events = []
    for now in (105.0, 110.0, 114.9):
        result = step(state, obs, now)
        state = result["state"]
        events += result["events"]
    assert state["nodes"]["node1"]["up_at"] == 105.0
    assert [e["event"] for e in events] == ["rejoined"]
    assert state["nodes"]["node1"]["health"] == "catching_up"
    assert state["nodes"]["node1"]["streak"] == 0


def test_settle_counts_from_the_first_answer_after_the_fault():
    topo = topo_of()
    state = wiped()
    silent = observe(topo, node1=None)
    state = step(state, silent, 105.0)["state"]
    assert state["nodes"]["node1"]["up_at"] is None
    synced = observe(topo, node1=(100, 100, 100, (0, 0, 0)))
    events = []
    for now in (108.0, 113.0, 118.0, 123.0):
        result = step(state, synced, now)
        state = result["state"]
        events += result["events"]
    assert [(e["event"], e["ts"], e["after_s"]) for e in events] == [
        ("rejoined", 108.0, 8.0),
        ("caught_up", 118.0, 18.0),
    ]


def test_caught_up_event_has_zero_counts_and_timeout_the_last_counts():
    topo = topo_of()
    state = with_health(init(topo=topo), node1="catching_up")
    state["next_fault_at"] = 1e9
    state["nodes"]["node1"].update(since=100.0, kind="wipe")
    lagging = observe(topo, node1=(100, 100, 100, (12, 0, 340)))
    state = step(state, lagging, 200.0)["state"]
    assert state["nodes"]["node1"]["last_missing"] == (12, 0, 340)
    # a failed probe keeps the last counts
    state = step(state, observe(topo, node1=None), 250.0)["state"]
    assert state["nodes"]["node1"]["last_missing"] == (12, 0, 340)
    result = step(state, lagging, 100.0 + CFG.recover_timeout_s)
    (event,) = [e for e in result["events"] if e["event"] == "timeout"]
    assert event["missing"] == (12, 0, 340)
    assert ch.chaos_log_line(event).endswith(
        "(missing blocks 12, leaves 0, vid_common 340)"
    )
    good = observe(topo, node1=(100, 100, 100, (0, 0, 0)))
    events = []
    for now in (300.0, 305.0, 310.0):
        result = step(state, good, now)
        state = result["state"]
        events += result["events"]
    (event,) = [e for e in events if e["event"] == "caught_up"]
    assert event["missing"] == (0, 0, 0)


def test_timeout_of_a_validator_has_no_counts():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    state["nodes"]["node5"].update(since=100.0, kind="kill")
    result = step(state, observe(topo, node5=None), 100.0 + CFG.recover_timeout_s)
    (event,) = [e for e in result["events"] if e["event"] == "timeout"]
    assert "missing" not in event
    assert ch.chaos_log_line(event).endswith("s")


# TEST:synced-after-settle-passes-ok
def test_catch_up_passes_on_synced_ticks_after_the_settle_time():
    topo = topo_of()
    state = wiped()
    obs = observe(topo, node1=(100, 100, 100, (0, 0, 0)))
    state = step(state, obs, 105.0)["state"]
    state = step(state, obs, 110.0)["state"]
    settle = 105.0 + ch.SYNC_SETTLE_S
    state = step(state, obs, settle)["state"]
    assert state["nodes"]["node1"]["health"] == "catching_up"
    result = step(state, obs, settle + 5.0)
    assert result["state"]["nodes"]["node1"]["health"] == "up"
    (event,) = result["events"]
    assert (event["event"], event["ts"]) == ("caught_up", settle)


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
    state = init(ch.ChaosConfig(kinds=("kill",)), topo)
    result = step(state, observe(topo), 60.0, ch.ChaosConfig(kinds=("kill",)))
    (fault,) = result["actions"]
    assert fault["op"] == "kill"
    state = result["state"]
    obs = observe(topo, **{fault["node"]: None})
    early = step(state, obs, 60.0 + 59.0, ch.ChaosConfig(kinds=("kill",)))
    assert [a for a in early["actions"] if a["op"] == "start"] == []
    late = step(state, obs, 60.0 + 60.0, ch.ChaosConfig(kinds=("kill",)))
    assert {"node": fault["node"], "op": "start"} in late["actions"]
    assert late["state"]["nodes"][fault["node"]]["health"] == "recovering"
    started = [e for e in late["events"] if e["event"] == "started"]
    assert started[0]["after_s"] == 60.0
    assert late["state"]["nodes"][fault["node"]]["started"] == 120.0


def test_kill_recovery_counts_from_the_start():
    topo = topo_of()
    cfg = ch.ChaosConfig(kinds=("kill",))
    state = with_health(init(cfg, topo), node5="recovering")
    state["nodes"]["node5"].update(kind="kill", since=100.0, started=160.0)
    state["next_fault_at"] = 1e9
    state = step(state, observe(topo), 170.0, cfg)["state"]
    (event,) = step(state, observe(topo), 175.0, cfg)["events"]
    assert (event["event"], event["ts"], event["after_s"]) == ("rejoined", 170.0, 10.0)


def timeouts(result) -> list[str]:
    return [e["node"] for e in result["events"] if e["event"] == "timeout"]


# REQ:awsbench-chaos-timeout
# TEST:recover-timeout-fails
# TEST:crashed-recovering-times-out-fails
def test_recover_timeout_marks_the_node_stuck_once():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    state["nodes"]["node5"].update(since=100.0, kind="restart")
    state["next_fault_at"] = 1e9
    obs = observe(topo, node5=None)
    ok = step(state, obs, 100.0 + CFG.recover_timeout_s - 1)
    assert timeouts(ok) == []
    assert "error" not in ok
    result = step(state, obs, 100.0 + CFG.recover_timeout_s)
    (event,) = [e for e in result["events"] if e["event"] == "timeout"]
    assert event["node"] == "node5"
    assert event["after_s"] == CFG.recover_timeout_s
    state = result["state"]
    assert state["nodes"]["node5"]["health"] == "stuck"
    for now in (500.0, 505.0, 510.0):
        result = step(state, observe(topo), now)
        state = result["state"]
        assert result["events"] == []
    assert state["nodes"]["node5"]["health"] == "stuck"
    assert ch.open_faults(state) == []


def test_stuck_node_keeps_its_budget_slot_and_is_never_targeted():
    n = 7
    topo, peers = topo_of(n, 3), peers_of(n, 3)
    assert ch.fault_budget(n) == 1
    state = with_health(init(topo=topo), node5="stuck")
    state["next_fault_at"] = 0.0
    state["order"] = ["node5", "node4"]
    result = step(state, observe(topo), 100.0, topo=topo, peers=peers)
    assert faults([result]) == []
    roomy = with_health(init(), node5="stuck")
    roomy["next_fault_at"] = 0.0
    roomy["order"] = ["node5", "node4"]
    assert [e["node"] for e in faults([step(roomy, observe(topo_of()), 100.0)])] == [
        "node4"
    ]


def test_killed_node_past_its_timeout_is_started_and_stuck():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="down")
    state["nodes"]["node5"].update(since=100.0, kind="kill")
    obs = observe(topo, **{n: None for n in topo["nodes"]})
    result = step(state, obs, 100.0 + CFG.recover_timeout_s)
    assert result["actions"] == [{"node": "node5", "op": "start"}]
    assert result["state"]["nodes"]["node5"]["health"] == "stuck"


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
    lagging = observe(topo, node1=(100, 100, 0, (5, 0, 7)))
    ok = step(state, lagging, 100.0 + CFG.recover_timeout_s - 1)
    assert timeouts(ok) == []
    result = step(state, lagging, 100.0 + CFG.recover_timeout_s)
    assert timeouts(result) == ["node1"]


# EDGE:tip-from-up-nodes
# TEST:all-probes-fail-skips-ok
def test_all_probes_fail_skips():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    state["next_fault_at"] = 0.0
    obs = observe(topo, **{n: None for n in topo["nodes"]})
    result = step(state, obs, 100.0)
    assert result["actions"] == [] and result["events"] == []
    assert result["state"] == state


def test_timeout_runs_without_tip():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="recovering")
    obs = observe(topo, **{n: None for n in topo["nodes"]})
    result = step(state, obs, CFG.recover_timeout_s)
    assert timeouts(result) == ["node5"]


# TEST:tip-ignores-faulty-ok
def test_tip_ignores_faulty():
    topo = topo_of()
    state = with_health(init(topo=topo), node6="down", node5="recovering")
    state["next_fault_at"] = 1e9
    obs = observe(topo, node6=(10_000, 10_000, 0, (0, 0, 0)))
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
    cfg = ch.ChaosConfig(kinds=("kill",))
    state = with_health(init(cfg, topo), node5="down")
    state["nodes"]["node5"].update(since=100.0, kind="kill")
    state["next_fault_at"] = 1e9
    result = step(state, observe(topo, node5=None), 400.0 - 1, cfg)
    assert result["actions"] == [{"node": "node5", "op": "start"}]
    assert timeouts(result) == []


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
def test_stops_so_that_a_late_kill_recovers_under_load():
    steps = run(seconds=1500, load_s=600.0)
    stop_at = 60.0 + 600.0 - CFG.kill_down_s - ch.RECOVERY_ALLOWANCE_S
    times = [e["ts"] for e in faults(steps)]
    assert len(times) >= 8 and max(times) < stop_at
    assert init(load_s=600.0)["stop_at"] == stop_at


@pytest.mark.parametrize("load_s", [60.0, 120.0, 180.0])
def test_short_load_has_no_faults(load_s):
    assert faults(run(seconds=600, load_s=load_s)) == []


def test_failed_fault_is_not_applied_and_retried_next_tick():
    topo = topo_of()
    state = init(topo=topo)
    first = step(state, observe(topo), 60.0)
    ((fault,),) = [first["actions"]]
    undone = ch.revert_failed(state, first, {fault["node"]})
    after = undone["state"]
    assert after["nodes"][fault["node"]] == state["nodes"][fault["node"]]
    schedule = ("cursor", "faults", "next_fault_at")
    assert [after[k] for k in schedule] == [state[k] for k in schedule]
    assert undone["actions"] == [] and undone["events"] == []
    retry = step(undone["state"], observe(topo), 65.0)
    assert retry["actions"] == [fault]


def test_failed_start_keeps_the_node_down_until_a_start_works():
    topo = topo_of()
    state = with_health(init(topo=topo), node5="down", node6="recovering")
    state["nodes"]["node5"].update(kind="kill", since=100.0)
    state["next_fault_at"] = 1e9
    good = observe(topo, node5=None)
    result = step(state, good, 160.0)
    undone = ch.revert_failed(state, result, {"node5"})
    assert undone["state"]["nodes"]["node5"] == state["nodes"]["node5"]
    assert undone["state"]["nodes"]["node6"]["streak"] == 1
    assert [e["event"] for e in undone["events"]] == []
    again = step(undone["state"], good, 165.0)
    assert {"node": "node5", "op": "start"} in again["actions"]


# TEST:same-seed-same-order-ok
def test_same_seed_same_order():
    def targets(seed):
        cfg = ch.ChaosConfig(seed=seed)
        return [e["node"] for e in faults(run(cfg, seconds=1000))]

    assert targets(7) == targets(7)
    assert targets(7) != targets(8)


# TEST:round-robin-order-ok
def test_round_robin_order():
    cfg = ch.ChaosConfig(kinds=("restart",))
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
    queries = queries_of(topo_of())
    assert {e["query"] for e in events} == {True, False}
    for e in events:
        assert {"ts", "iso", "event", "node"} <= e.keys()
        assert e["query"] == (e["node"] in queries)
        assert e["iso"].startswith("1970-01-01T")
        if e["event"] in ("rejoined", "caught_up"):
            assert e["after_s"] is not None
        if e["event"] == "fault":
            assert e["height"] == 100


# EDGE:probe-partial-output
# TEST:probe-dash-is-none-ok
def test_probe_dash_is_none():
    topo = topo_of(4, 2)
    out = "node0 10 20 9 0,1,2\nnode1 - - - -\nnode2 5 6\nnode3 - 7\n"
    obs = ch.parse_probe(out, topo["nodes"], queries_of(topo))
    assert obs["height"] == {"node0": 10, "node1": None, "node2": 5, "node3": None}
    assert obs["voted_view"]["node3"] == 7
    assert obs["query_height"] == {"node0": 9, "node1": None}
    assert obs["missing"] == {"node0": (0, 1, 2), "node1": None}


# TEST:probe-missing-line-fails
def test_probe_missing_line_fails():
    topo = topo_of(4, 2)
    with pytest.raises(ValueError, match="node3"):
        ch.parse_probe(
            "node0 1 2 3 0,0,0\nnode1 1 2 3 0,0,0\nnode2 1 2\n",
            topo["nodes"],
            queries_of(topo),
        )
    with pytest.raises(ValueError, match="node1"):
        ch.parse_probe(
            "node0 1 2 3 0,0,0\nnode1 1 2\nnode2 1 2\nnode3 1 2\n",
            topo["nodes"],
            queries_of(topo),
        )


def test_probe_command_covers_every_node():
    topo = topo_of(4, 2)
    *lines, last = ch.probe_command(topo["nodes"], queries_of(topo)).splitlines()
    assert last == "wait"
    assert len(lines) == 4
    assert "/v1/node/sync-status" in lines[0] and "/v1/node/sync-status" not in lines[2]
    assert "consensus_last_voted_view" in lines[2]
    assert all(topo["nodes"][f"node{i}"] in lines[i] for i in range(4))
    for line in lines:
        shlex.split(line)


def test_missing_filter_prints_the_three_counts():
    def counts(missing: tuple[int, int, int]) -> str:
        blocks, leaves, vid = ({"missing": m, "ranges": []} for m in missing)
        doc = {
            "blocks": blocks,
            "leaves": leaves,
            "vid_common": vid,
            "pruned_height": 0,
        }
        out = subprocess.run(
            ["jq", "-r", ch.MISSING_JQ],
            input=json.dumps(doc),
            text=True,
            capture_output=True,
            check=True,
        )
        return out.stdout.strip()

    assert counts((0, 0, 0)) == "0,0,0"
    assert counts((12, 0, 340)) == "12,0,340"
    topo = topo_of(4, 2)
    command = ch.probe_command(topo["nodes"], queries_of(topo))
    assert shlex.quote(ch.MISSING_JQ) in command


T0 = 1000.0
KINDS = ["restart", "kill", "wipe"]
QUERIES = frozenset(f"node{i}" for i in range(4))


def ev(
    at: float,
    event: ch.ChaosEventKind,
    node: str,
    kind: ch.ChaosKind,
    missing: ch.Missing | None = None,
) -> ch.ChaosEvent:
    return ch.chaos_event(T0 + at, event, node, node in QUERIES, kind, 10, 5.0, missing)


# node0, a query node, restarts at 10 s and is synced at 40 s; node5, a validator, is killed at
# 70 s, starts at 130 s and rejoins at 150 s, so it is faulty for the second and third step.
EVENTS = [
    ev(10, "fault", "node0", "restart"),
    ev(25, "rejoined", "node0", "restart"),
    ev(40, "caught_up", "node0", "restart"),
    ev(70, "fault", "node5", "kill"),
    ev(130, "started", "node5", "kill"),
    ev(150, "rejoined", "node5", "kill"),
]


def test_fault_rows_follow_each_fault_to_its_events():
    rows = ch.fault_rows(EVENTS)
    assert [(r.node, r.kind) for r in rows] == [
        ("node0", "restart"),
        ("node5", "kill"),
    ]
    assert rows[0].caught_up == T0 + 40
    assert rows[1].caught_up is None and rows[1].started == T0 + 130


def test_fault_table_adds_the_missing_column_only_for_a_timeout_with_counts():
    plain = ch.fault_table(ch.fault_rows(EVENTS), T0)
    assert "missing" not in plain[2]
    assert "| node5 | kill | 70 | 130 | 150 | - |" in plain
    timeout = [*EVENTS[:2], ev(330, "timeout", "node0", "restart", (3, 0, 2))]
    table = ch.fault_table(ch.fault_rows(timeout), T0)
    assert table[2].endswith("| missing |")
    assert (
        table[4]
        == "| node0 | restart | 10 | - | 25 | - | blocks 3, leaves 0, vid_common 2 |"
    )
    assert ch.fault_table([], T0) == []


def test_verdict_fails_only_on_timeouts():
    rows = ch.fault_rows(EVENTS)
    assert ch.chaos_verdict(rows, EVENTS, None, False).startswith("- Verdict: **pass**")
    open_ = EVENTS[:-1]
    timed_out = [*open_, ev(400, "timeout", "node5", "kill")]
    verdict = ch.chaos_verdict(ch.fault_rows(timed_out), timed_out, None, False)
    assert verdict == "- Verdict: **fail**: node5 (kill) not recovered after 5 s"


def test_verdict_names_interrupted_faults_without_blaming_them():
    aborted = [
        *EVENTS[:-1],
        ev(155, "interrupted", "node5", "kill"),
        ev(160, "fault", "node1", "wipe"),
        ev(170, "rejoined", "node1", "wipe"),
        ev(175, "interrupted", "node1", "wipe"),
    ]
    rows = ch.fault_rows(aborted)
    assert [r.interrupted for r in rows] == [None, T0 + 155, T0 + 175]
    verdict = ch.chaos_verdict(rows, aborted, "interrupted", True)
    assert verdict == (
        "- Verdict: **fail**: interrupted; interrupted faults: node5 (kill), node1 (wipe)"
    )
    verdict = ch.chaos_verdict(rows, aborted, None, False)
    assert verdict == (
        "- Verdict: **pass**: no timeout; interrupted faults: node5 (kill), node1 (wipe)"
    )
    line = ch.chaos_log_line(aborted[-1])
    assert line == "node1 interrupted"


@pytest.mark.parametrize(("error", "reason"), [("boom", "boom"), (None, "no result")])
def test_verdict_of_a_run_without_result_fails_even_with_clean_faults(error, reason):
    rows = ch.fault_rows(EVENTS)
    verdict = ch.chaos_verdict(rows, EVENTS, error, True)
    assert verdict.startswith("- Verdict: **fail**") and reason in verdict


def test_recovery_table_has_median_and_max_per_kind():
    events = [
        *EVENTS,
        ev(200, "fault", "node1", "restart"),
        ev(230, "rejoined", "node1", "restart"),
    ]
    table = ch.recovery_table(ch.fault_rows(events), KINDS)
    assert "| from |" in table[0]
    assert table[2:] == [
        "| restart | 2 | fault | 22 | 30 | 30 | 30 |",
        "| kill | 1 | start | 20 | 20 | - | - |",
        "| wipe | 0 | fault | - | - | - | - |",
    ]


def test_steps_name_their_faults_and_restarted_nodes():
    spans = [(T0 + 60 * i, T0 + 60 * i + 60) for i in range(4)]
    chaos = ch.step_chaos(spans, ch.fault_rows(EVENTS), T0, T0 + 240)
    assert [c["start_s"] for c in chaos] == [0, 60, 120, 180]
    # A fault is active from its start to its rejoin.
    assert [c["faults"] for c in chaos] == [
        "node0 restart",
        "node5 kill",
        "node5 kill",
        "",
    ]
    assert [sorted(c["restarted"]) for c in chaos] == [
        ["node0"],
        ["node5"],
        ["node5"],
        [],
    ]


def span_of(*events: ch.ChaosEvent, end: float = T0 + 300) -> tuple[float, float]:
    (row,) = ch.fault_rows(events)
    start, stop = ch.fault_span(row, end)
    return start - T0, stop - T0


def test_a_fault_lasts_until_the_node_recovered():
    assert span_of(*EVENTS[:3]) == (10, 40)
    assert span_of(*EVENTS[3:]) == (70, 150)
    # A query node that rejoined is still faulty while it catches up.
    assert span_of(*EVENTS[:2]) == (10, 300)


def test_a_stuck_fault_lasts_until_the_end_and_an_interrupted_one_until_the_abort():
    stuck = [ev(5, "fault", "node1", "wipe"), ev(9, "rejoined", "node1", "wipe")]
    assert span_of(*stuck, ev(305, "timeout", "node1", "wipe")) == (5, 300)
    killed = [ev(5, "fault", "node7", "kill"), ev(9, "timeout", "node7", "kill")]
    assert span_of(*killed) == (5, 300)
    aborted = [*stuck, ev(20, "interrupted", "node1", "wipe")]
    assert span_of(*aborted) == (5, 20)


def test_max_concurrent_faulty_counts_query_nodes_until_caught_up():
    events = [
        ev(0, "fault", "node1", "wipe"),
        ev(10, "rejoined", "node1", "wipe"),
        ev(20, "fault", "node5", "restart"),
        ev(30, "rejoined", "node5", "restart"),
        ev(50, "caught_up", "node1", "wipe"),
    ]
    assert ch.max_concurrent_faulty(ch.fault_rows(events), T0 + 60) == 2
    assert ch.max_concurrent_faulty(ch.fault_rows(EVENTS), T0 + 300) == 1


def test_a_step_counts_a_query_node_faulty_while_it_catches_up():
    events = [ev(50, "fault", "node1", "wipe"), ev(55, "rejoined", "node1", "wipe")]
    events.append(ev(70, "caught_up", "node1", "wipe"))
    spans = [(T0, T0 + 60), (T0 + 60, T0 + 120)]
    chaos = ch.step_chaos(spans, ch.fault_rows(events), T0, T0 + 120)
    assert [c["faults"] for c in chaos] == ["node1 wipe", "node1 wipe"]


def test_decided_splits_counter_intervals_by_fault_overlap():
    counters = [(float(t), 2e6 * t) for t in range(11)]
    faulty, quiet = ch.decided_by_fault(counters, [(2.5, 4.0)], start=1.0, stop=9.0)
    # Intervals 2-3 and 3-4 overlap the fault; 1-2 and 4-9 do not.
    assert faulty == (4.0, 2.0)
    assert quiet == (12.0, 6.0)
    assert ch.decided_by_fault(counters, [], 0.0, 10.0) == ((0.0, 0.0), (20.0, 10.0))


def test_events_without_query_get_it_from_the_topology(tmp_path):
    old = [{k: v for k, v in e.items() if k != "query"} for e in EVENTS]
    lines = "".join(json.dumps(e) + "\n" for e in old)
    (tmp_path / "chaos.jsonl").write_text(lines)
    roles = {"node0": "validator, query, sqlite", "node5": "validator"}
    (tmp_path / "topology.json").write_text(json.dumps({"roles": roles}))
    assert ch.read_events(tmp_path) == EVENTS
    (tmp_path / "topology.json").unlink()
    (tmp_path / "chaos.jsonl").write_text("".join(json.dumps(e) + "\n" for e in EVENTS))
    assert ch.read_events(tmp_path) == EVENTS


def fault_event(
    ts: float, event: ch.ChaosEventKind, node: str, kind: ch.ChaosKind = "kill"
) -> ch.ChaosEvent:
    return ch.chaos_event(ts, event, node, True, kind, None, None)


CHAOS_EVENTS = [
    fault_event(1005.0, "fault", "node5"),
    fault_event(1015.0, "started", "node5"),
    fault_event(1020.0, "rejoined", "node5"),
    fault_event(1030.0, "caught_up", "node5"),
    fault_event(1008.0, "fault", "node2", "wipe"),
    fault_event(1018.0, "rejoined", "node2"),
    fault_event(1040.0, "timeout", "node2"),
    fault_event(1050.0, "fault", "node7", "restart"),
]


def test_phases_of_a_killed_query_node_run_from_fault_to_catch_up():
    row = ch.fault_rows(CHAOS_EVENTS)[0]
    assert ch.phase_segments(row, end=1100.0) == [
        ("down", 1005.0, 1015.0),
        ("recovering", 1015.0, 1020.0),
        ("catching up", 1020.0, 1030.0),
    ]


def test_a_node_without_a_start_event_has_no_down_phase():
    row = ch.FaultRow("node1", False, "restart", 10.0, rejoined=25.0)
    assert ch.phase_segments(row, end=100.0) == [("recovering", 10.0, 25.0)]


def test_a_stuck_node_keeps_its_open_phase_and_a_run_end_ends_an_unfinished_one():
    wiped = ch.fault_rows(CHAOS_EVENTS)[1]
    assert ch.phase_segments(wiped, end=1100.0) == [
        ("recovering", 1008.0, 1018.0),
        ("catching up", 1018.0, 1100.0),
    ]
    open_ = ch.fault_rows(CHAOS_EVENTS)[2]
    assert ch.phase_segments(open_, end=1100.0) == [("recovering", 1050.0, 1100.0)]


def test_a_kill_aborted_while_down_stays_down():
    restored = ch.fault_rows(
        [
            fault_event(1005.0, "fault", "node5"),
            fault_event(1030.0, "restored", "node5"),
            fault_event(1030.0, "interrupted", "node5"),
        ]
    )[0]
    assert ch.phase_segments(restored, end=1100.0) == [("down", 1005.0, 1030.0)]


def test_a_kill_stuck_while_down_recovers_from_its_timeout():
    stuck = ch.fault_rows(
        [fault_event(1005.0, "fault", "node5"), fault_event(1305.0, "timeout", "node5")]
    )[0]
    assert ch.phase_segments(stuck, end=1400.0) == [
        ("down", 1005.0, 1305.0),
        ("recovering", 1305.0, 1400.0),
    ]
