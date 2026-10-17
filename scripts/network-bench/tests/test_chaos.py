import gzip
import json
import random
import shlex
import subprocess
from datetime import UTC, datetime
from itertools import pairwise
from pathlib import Path
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


def init(cfg=CFG, topo=None, load_s: float = 6000.0, stake=None) -> Any:
    nodes = (topo or topo_of())["nodes"]
    stake = stake or dict.fromkeys(nodes, 1)
    return ch.chaos_init(cfg, list(nodes), stake, 0.0, 60.0, load_s)


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


def run(
    cfg=CFG,
    n=10,
    k=3,
    seconds=2000.0,
    tick=5.0,
    recover=True,
    load_s=6000.0,
    stake=None,
):
    """Ticks the controller; observations follow the state: a not up node shows the tip when
    `recover`, else answers nothing."""
    topo, peers = topo_of(n, k), peers_of(n, k)
    state = init(cfg, topo, load_s, stake)
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
def test_halts_is_the_exact_third_of_the_stake():
    stake = dict.fromkeys("abcdefghij", 1)
    assert not ch.halts(stake, list("abc"))
    assert ch.halts(stake, list("abcd"))
    assert ch.halts({"a": 1, "b": 2}, ["b"])
    assert not ch.halts({"a": 1, "b": 3, "c": 3}, ["a"])


# REQ:awsbench-chaos-invariants
# REQ:sched-stake-budget
# TEST:sched-stake-budget-ok
@pytest.mark.parametrize("n", range(7, 33))
def test_budget_never_exceeded(n):
    k = min(3, n - 4)
    for seed in range(3):
        rng = random.Random(seed)
        stake = {f"node{i}": rng.randint(1, 20) for i in range(n)}
        cfg = ch.ChaosConfig(seed=seed, recover_timeout_s=1e9)
        for result in run(cfg, n, k, seconds=1500, recover=False, stake=stake):
            faulty = [
                m for m, s in result["state"]["nodes"].items() if s["health"] != "up"
            ]
            assert not ch.halts(stake, faulty)


# REQ:sched-stake-budget
# TEST:sched-heavy-node-ok
def test_a_node_holding_a_third_of_the_stake_is_never_a_target():
    n = 10
    topo = topo_of(n, 3)
    stake = {**dict.fromkeys(topo["nodes"], 1), "node0": 5}
    for result in run(CFG, n, 3, seconds=1500, stake=stake):
        assert "node0" not in [e["node"] for e in faults([result])]


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


# TEST:sched-query-floor-ok
def test_query_floor_sets_the_up_query_nodes_kept():
    topo = topo_of(13, 3)
    state = with_health(init(topo=topo), node0="catching_up")
    state["order"] = ["node1", "node5"]
    state["next_fault_at"] = 0.0
    obs = observe(topo, node0=None)
    for floor, target in ((1, "node1"), (2, "node5")):
        cfg = ch.ChaosConfig(query_floor=floor)
        result = step(state, obs, 100.0, cfg=cfg, topo=topo, peers=peers_of(13, 3))
        assert [e["node"] for e in faults([result])] == [target]


def test_a_lagging_query_node_does_not_count_toward_the_query_floor():
    topo = topo_of(13, 4)
    state = with_health(init(topo=topo), node0="catching_up")
    state["order"] = ["node2", "node4"]
    state["next_fault_at"] = 0.0
    result = step(
        state, observe(topo, node1=None), 100.0, topo=topo, peers=peers_of(13, 4)
    )
    assert [e["node"] for e in faults([result])] == ["node4"]


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
    state = init(topo=topo)
    state["order"] = ["node3", "node4"]
    state["next_fault_at"] = 0.0
    silent = step(state, observe(topo, node3=None), 100.0)
    assert [e["node"] for e in faults([silent])] == ["node4"]
    lagging = (100 - ch.LAG_BLOCKS - 1, 100, 0, (0, 0, 0))
    full = step(state, observe(topo, node3=None, node5=lagging, node7=lagging), 100.0)
    assert faults([full]) == []
    near = (100 - ch.LAG_BLOCKS, 100, 0, (0, 0, 0))
    room = step(state, observe(topo, node3=None, node5=near, node7=lagging), 100.0)
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


# TEST:sched-no-tip-kill-ok
def test_kill_starts_when_no_node_answers():
    topo = topo_of()
    cfg = ch.ChaosConfig(kinds=("kill",))
    state = with_health(init(cfg, topo), node5="down")
    state["nodes"]["node5"].update(since=100.0, kind="kill")
    state["next_fault_at"] = 1e9
    obs = observe(topo, **dict.fromkeys(topo["nodes"]))
    result = step(state, obs, 100.0 + cfg.kill_down_s, cfg)
    assert {"node": "node5", "op": "start"} in result["actions"]
    assert result["state"]["nodes"]["node5"]["health"] == "recovering"
    assert timeouts(result) == []


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
    state = with_health(init(topo=topo), node5="stuck", node6="stuck")
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


def test_a_recover_timeout_within_the_kill_down_time_is_refused():
    with pytest.raises(ValueError, match="recover_timeout_s"):
        ch.ChaosConfig(kill_down_s=100.0, recover_timeout_s=100.0)
    ch.ChaosConfig(kill_down_s=100.0, recover_timeout_s=101.0)


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


def test_share_wipe_can_target_query_nodes():
    cfg = ch.ChaosConfig(kinds=("share-wipe",))
    steps = run(cfg, seconds=1000)
    queries = queries_of(topo_of())
    targets = [e["node"] for e in faults(steps)]
    assert queries.intersection(targets)
    assert {a["op"] for s in steps for a in s["actions"] if a["op"] != "start"} == {
        "share-wipe"
    }


# TEST:gap-respected-ok
def test_gap_respected():
    times = [e["ts"] for e in faults(run(seconds=1500))]
    assert times[0] == 60.0
    assert all(b - a >= CFG.gap_s for a, b in pairwise(times))


# TEST:sched-gap-density-ok
def test_gap_sets_the_fault_density():
    cfg = ch.ChaosConfig(gap_s=10.0)
    window = 600.0
    load_s = window + cfg.kill_down_s + ch.RECOVERY_ALLOWANCE_S
    steps = run(cfg, n=32, k=3, seconds=60.0 + window, load_s=load_s)
    times = [e["ts"] for e in faults(steps)]
    assert len(times) >= 40
    assert all(b - a >= cfg.gap_s for a, b in pairwise(times))


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


@pytest.mark.parametrize(
    ("kind", "health"),
    [("restart", "recovering"), ("kill", "down"), ("wipe", "recovering")],
)
def test_a_failed_fault_keeps_the_schedule_and_is_issued_again_at_its_time(
    kind: ch.ChaosKind, health: ch.NodeHealth
):
    topo = topo_of()
    cfg = ch.ChaosConfig(kinds=(kind,))
    state = init(cfg, topo)
    first = step(state, observe(topo), 60.0, cfg, topo)
    ((action,),) = [first["actions"]]
    node = action["node"]
    undone = ch.revert_failed(state, first, {node})
    assert undone["state"]["nodes"][node] == state["nodes"][node]
    schedule = ("cursor", "faults", "next_fault_at")
    assert [undone["state"][k] for k in schedule] == [
        first["state"][k] for k in schedule
    ]
    assert undone["state"]["fault_retry"] == {node: (kind, 60.0)}
    assert undone["actions"] == [] and undone["events"] == []
    retry = step(undone["state"], observe(topo), 65.0, cfg, topo)
    assert retry["actions"] == [action]
    (fault,) = [e for e in retry["events"] if e["event"] == "fault"]
    assert fault["ts"] == 60.0
    assert retry["state"]["nodes"][node]["health"] == health
    assert retry["state"]["nodes"][node]["since"] == 60.0
    assert retry["state"]["fault_retry"] == {}


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
    full = with_health(first["state"], node6="stuck")
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


def test_verdict_of_a_run_without_faults_says_none_issued():
    assert ch.chaos_verdict([], [], None, False) == (
        "- Verdict: **pass**: no fault issued, no timeout"
    )


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


def test_a_kill_that_timed_out_while_down_restarts_at_its_timeout():
    events = [ev(10, "fault", "node5", "kill"), ev(70, "timeout", "node5", "kill")]
    spans = [(T0, T0 + 60), (T0 + 60, T0 + 120)]
    chaos = ch.step_chaos(spans, ch.fault_rows(events), T0, T0 + 120)
    assert [sorted(c["restarted"]) for c in chaos] == [["node5"], ["node5"]]


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


def test_floor_span_ends_a_tail_after_rejoin_not_at_catch_up():
    rows = ch.fault_rows(EVENTS)
    assert ch.floor_span(rows[0], T0 + 240) == (T0 + 10, T0 + 25 + ch.FLOOR_TAIL_S)
    assert ch.fault_span(rows[0], T0 + 240) == (T0 + 10, T0 + 40)


def test_floor_span_of_stuck_and_interrupted_faults():
    stuck = ch.fault_rows(
        [ev(10, "fault", "node5", "kill"), ev(60, "rejoined", "node5", "kill")]
        + [ev(70, "timeout", "node5", "kill")]
    )
    assert ch.floor_span(stuck[0], T0 + 240) == (T0 + 10, T0 + 240)
    open_ = ch.fault_rows([ev(10, "fault", "node5", "kill")])
    assert ch.floor_span(open_[0], T0 + 240) == (T0 + 10, T0 + 240)
    cut = ch.fault_rows(
        [ev(10, "fault", "node5", "kill"), ev(30, "interrupted", "node5", "kill")]
    )
    assert ch.floor_span(cut[0], T0 + 240) == (T0 + 10, T0 + 30 + ch.FLOOR_TAIL_S)


def test_counter_split_counts_the_rise_after_every_node_restarted():
    node = [(T0 + t, v) for t, v in [(0, 0), (10, 100), (20, 200), (30, 0), (40, 50)]]
    series = {"node0": node, "node1": node}
    split = ch.counter_split(series, [], T0, T0 + 40)
    # The restart interval (20, 30] adds nothing; (30, 40] adds the 50 after it.
    assert split == ch.CounterSplit(0.0, 0.0, 250.0, 40.0)


def test_a_block_two_nodes_see_in_different_intervals_counts_once():
    series = {
        "node0": [(0.0, 0.0), (1.0, 10.0), (2.0, 10.0), (3.0, 20.0)],
        "node1": [(0.0, 0.0), (1.0, 0.0), (2.0, 10.0), (3.0, 20.0)],
    }
    assert ch.counter_increases(series) == [
        (0.0, 1.0, 10.0),
        (1.0, 2.0, 0.0),
        (2.0, 3.0, 10.0),
    ]


def test_a_restarted_node_continues_from_its_count_before_the_restart():
    series = {
        "node0": [(T0 + t, v) for t, v in [(0, 10), (10, 20), (20, 5), (30, 9)]],
        "node1": [(T0 + t, v) for t, v in [(0, 10), (10, 13), (20, 14), (30, 15)]],
    }
    split = ch.counter_split(series, [(T0 + 15, T0 + 25)], T0, T0 + 30)
    # node0 stitched: 10, 20, 20, 24; the restart interval adds nothing.
    assert split == ch.CounterSplit(
        during=0.0 + 4.0, during_s=20.0, outside=10.0, outside_s=10.0
    )
    assert ch.counter_split(series, [], T0 + 10, T0 + 20).outside == 0.0


def test_counter_split_diffs_bytes_by_fault_overlap():
    counters = {"node0": [(float(t), 2e6 * t) for t in range(11)]}
    split = ch.counter_split(counters, [(2.5, 4.0)], 1.0, 9.0)
    # Intervals 2-3 and 3-4 overlap the fault; 1-2 and 4-9 do not.
    assert (split.during / 1e6, split.during_s) == (4.0, 2.0)
    assert (split.outside / 1e6, split.outside_s) == (12.0, 6.0)
    assert ch.counter_split(counters, [], 0.0, 10.0) == ch.CounterSplit(
        0.0, 0.0, 20e6, 10.0
    )


def test_metrics_fallback_does_not_count_a_wiped_node_starting_from_zero(tmp_path):
    scrapes = [
        (1.1, "node0", 100),
        (1.2, "node1", 101),
        (2.1, "node0", 102),
        (2.2, "node1", 103),
        (2.3, "node5", 0),
        (3.1, "node0", 103),
        (3.2, "node1", 104),
        (3.3, "node5", 1),
    ]
    (tmp_path / "metrics.jsonl").write_text(
        "".join(
            f'{{"ts": {ts}, "node": "{node}", "ok": true, "m": {{"consensus_current_view": {v}.0}}}}\n'
            for ts, node, v in scrapes
        )
    )
    series = ch.view_samples(tmp_path, [])
    assert series["node5"] == [(2.0, 0.0), (3.0, 1.0)]
    assert ch.counter_split(series, [], 0.0, 10.0).outside == 3.0


def test_running_max_ignores_a_node_catching_up_from_zero():
    series = {
        "a": [(0.0, 120.0), (3.0, 130.0)],
        "b": [(0.0, 0.0), (3.0, 125.0)],
    }
    split = ch.counter_split(ch.running_max(series), [(0.0, 3.0)], 0.0, 3.0)
    assert split.during == 10.0


def test_legacy_samples_do_not_count_the_rise_back_after_a_dip():
    samples = [{"ts": t, "k": v} for t, v in [(0, 100), (1, 50), (2, 100), (3, 110)]]
    split = ch.counter_split(ch.counter_series(samples, "k"), [], 0.0, 10.0)
    assert split.outside == 10.0


def test_mass_halt_counts_stuck_nodes_as_faulty():
    cfg = ch.ChaosConfig(kinds=("mass-kill",), mass_nodes=3)
    topo = topo_of()
    state = init(cfg, topo)
    stuck = state["order"][0]
    state = with_health(state, **{stuck: "stuck"})
    state["next_fault_at"] = 0.0
    out = step(state, observe(topo, **{stuck: None}), 10.0, cfg, topo)
    members = [a["node"] for a in out["actions"]]
    state = out["state"]
    events = []
    for now in range(15, 120, 5):
        overrides = {stuck: None}
        if now <= 65:
            overrides.update(silent(members))
        elif now <= 80:
            overrides.update({m: (100, 100, 100, (0, 0, 0)) for m in members})
        out = step(state, observe(topo, tip=101, **overrides), float(now), cfg, topo)
        state = out["state"]
        events += out["events"]
    assert not [e for e in events if e["event"] == "rejoined"]


def test_common_window_clips_to_the_shared_samples():
    a = {"n0": [(5.0, 0.0), (30.0, 1.0)], "n1": [(20.0, 0.0), (50.0, 1.0)]}
    b = {"n0": [(1.0, 0.0), (40.0, 1.0)]}
    assert ch.common_window([a, b], 0.0, 100.0) == (5.0, 40.0)
    assert ch.common_window([a, {}, {"n0": []}], 10.0, 100.0) == (10.0, 50.0)


def test_floor_lines_show_rates_and_dash_without_views():
    zero = ch.CounterSplit(0, 0, 0, 0)
    split = ch.CounterSplit(4, 100, 2, 200)
    views = ch.CounterSplit(900, 100, 100, 200)
    assert ch.floor_lines(split, views) == [
        (
            "- View timeouts: outside faults 2 timeouts in 100 views (2.00 %) over 200 s; "
            "during faults 4 timeouts in 900 views (0.44 %) over 100 s"
        )
    ]
    assert "(-)" in ch.floor_lines(zero, zero)[0]


def log_line(message: str, at: float, **fields: str) -> str:
    stamp = datetime.fromtimestamp(T0 + at, UTC).isoformat()
    record = {
        "timestamp": stamp,
        "level": "WARN",
        "fields": {"message": message, **fields},
        "target": "x",
    }
    return json.dumps(record)


def test_log_label_matches_prefix_and_breaks_down_by_field():
    stub = log_line(
        "parent state unavailable; queued on parent for retry (from_header stub inserted). If",
        5,
    )
    assert ch.log_label(stub) == ("stub", T0 + 5)
    propose = log_line(
        "timeout: we were the leader but did not propose", 6, missing="parent_proposal"
    )
    assert ch.log_label(propose) == ("no-propose[parent_proposal]", T0 + 6)
    did = log_line("timeout: we were the leader and did propose for this view", 6)
    assert ch.log_label(did) is None
    assert ch.log_label("Starting Espresso node with sqlite...") is None
    assert ch.log_label(log_line("unrelated", 1)) is None


def test_count_log_splits_by_the_nodes_windows_and_clips_to_the_load():
    lines = [log_line("header creation failed", at) for at in (-5, 5, 15, 25, 500)] + [
        log_line("timeout: we did not vote1 for this view", 15, missing="proposal")
    ]
    during, outside = ch.count_log(lines, [(T0 + 10, T0 + 20)], T0, T0 + 100)
    assert during == {"header-failed": 1, "no-vote1[proposal]": 1}
    assert outside == {"header-failed": 2}


def test_node_counts_read_every_wipe_appended_to_the_wiped_log(tmp_path):
    """The wipe script appends one gzip member per wipe to one file."""
    host = tmp_path / "hosts" / "node0"
    host.mkdir(parents=True)
    line = log_line("header creation failed", 15) + "\n"
    (host / "espresso-node.wiped.log.gz").write_bytes(
        gzip.compress(line.encode()) + gzip.compress(line.encode())
    )
    counts = ch.node_counts(tmp_path, {"node0": "eu"}, [], T0, T0 + 100, T0 + 100)
    assert counts["node0"]["outside"] == {"header-failed": 2}


def test_node_regions_follow_the_host_dirs_in_node_order(tmp_path):
    for node in ("node10", "node0", "node2"):
        (tmp_path / "hosts" / node).mkdir(parents=True)
    manifest = {"latency": {"assignment": {"node0": "eu", "node2": "us"}}}
    assert ch.node_regions(tmp_path, manifest) == {
        "node0": "eu",
        "node2": "us",
        "node10": "-",
    }
    assert ch.node_regions(tmp_path, {}) == dict.fromkeys(
        ("node0", "node2", "node10"), "-"
    )


def test_node_counts_read_current_and_wiped_logs_by_region(tmp_path):
    for node, name in [
        ("node0", "espresso-node.log.gz"),
        ("node0", "espresso-node.wiped.log.gz"),
        ("node1", "espresso-node.log.gz"),
    ]:
        host = tmp_path / "hosts" / node
        host.mkdir(parents=True, exist_ok=True)
        with gzip.open(host / name, "wt") as f:
            f.write("plain line\n" + log_line("header creation failed", 15) + "\n")
    (tmp_path / "hosts" / "node2").mkdir()
    rows = ch.fault_rows(
        [ev(10, "fault", "node0", "restart"), ev(20, "rejoined", "node0", "restart")]
    )
    counts = ch.node_counts(
        tmp_path,
        {"node0": "eu", "node1": "us", "node2": "us"},
        rows,
        T0,
        T0 + 100,
        T0 + 100,
    )
    assert counts == {
        "node0": {"region": "eu", "during": {"header-failed": 2}, "outside": {}},
        "node1": {"region": "us", "during": {}, "outside": {"header-failed": 1}},
    }
    assert ch.log_table(counts) == [
        "| log event | during fault | outside fault | eu (1) | us (1) |",
        "|---|---:|---:|---:|---:|",
        "| header-failed | 2 | 1 | 0 | 1 |",
    ]
    assert ch.log_table({}) == []


def test_views_come_from_consensus_else_metrics(tmp_path):
    assert ch.view_samples(tmp_path, ch.read_consensus(tmp_path)) == {}
    (tmp_path / "consensus.jsonl").write_text(
        '{"ts": 1.5, "decided_bytes": 0, "timeouts": 2}\n'
    )
    (tmp_path / "metrics.jsonl").write_text(
        '{"ts": 1.1, "node": "node0", "ok": true, "m": {"a": 1, "consensus_current_view": 5.0}}\n'
        '{"ts": 1.7, "node": "node1", "ok": true, "m": {"consensus_current_view": 7.0}}\n'
        '{"ts": 2.1, "node": "node1", "ok": false}\n'
    )
    consensus = ch.read_consensus(tmp_path)
    assert ch.view_samples(tmp_path, consensus) == {
        "node0": [(1.0, 5.0)],
        "node1": [(1.0, 7.0)],
    }
    assert ch.counter_series(consensus, "timeouts") == {"": [(1.5, 2)]}
    (tmp_path / "consensus.jsonl").write_text(
        '{"ts": 1.5, "decided_bytes": 0, "timeouts": 2, "views": 9}\n'
    )
    assert ch.view_samples(tmp_path, ch.read_consensus(tmp_path)) == {"": [(1.5, 9)]}


def test_no_kinds_issues_no_fault():
    cfg = ch.ChaosConfig(kinds=())
    assert faults(run(CFG, seconds=600.0))
    assert faults(run(cfg, seconds=600.0)) == []


def test_log_events_carry_timestamps_within_the_window():
    def line(ts: str, message: str) -> str:
        return json.dumps({"timestamp": ts, "fields": {"message": message}})

    stub = "parent state unavailable; queued on parent for retry (from_header stub inserted)"
    lines = [
        line("1970-01-01T00:00:05+00:00", stub),
        line("1970-01-01T00:00:50+00:00", stub),
        line("1970-01-01T00:00:06+00:00", "unrelated"),
        "container output",
    ]
    assert ch.log_events(lines, 0.0, 10.0) == [(5.0, "stub")]


MASS = ch.ChaosConfig(kinds=("mass-restart",))
ROTATION = ch.ChaosConfig(kinds=("restart", "mass-restart"))


def rising(topo, now, **override) -> Any:
    """Every node answers a tip that grows with `now`."""
    return observe(topo, tip=100 + int(now), view=100 + int(now), **override)


def run_mass(cfg, seconds=1500.0, load_s=600.0, tick=5.0, n=10, k=3):
    topo, peers = topo_of(n, k), peers_of(n, k)
    state = init(cfg, topo, load_s)
    now, steps = 0.0, []
    while now < seconds:
        now += tick
        result = step(state, rising(topo, now), now, cfg, topo, peers)
        state = result["state"]
        steps.append(result)
    return steps


def mass_events(steps) -> list[dict]:
    return [e for e in faults(steps) if e["kind"] == "mass-restart"]


# TEST:mass-restart-alone-once-midway
def test_a_lone_mass_restart_is_issued_once_midway():
    steps = run_mass(MASS)
    first, stop_at = 60.0, 60.0 + 600.0 - CFG.kill_down_s - ch.RECOVERY_ALLOWANCE_S
    (at,) = {e["ts"] for e in mass_events(steps)}
    assert at == pytest.approx((first + stop_at) / 2, abs=5.0)
    assert len(mass_events(steps)) == 10
    (issued,) = [s for s in steps if s["actions"]]
    assert {a["node"]: a["op"] for a in issued["actions"]} == {
        f"node{i}": "restart" for i in range(10)
    }


def test_a_mass_restart_ignores_budget_floor_and_peers():
    topo = topo_of()
    state = init(MASS, topo)
    state["next_fault_at"] = 0.0
    out = step(state, observe(topo), 10.0, MASS, topo)
    assert len(out["actions"]) == 10
    assert all(s["health"] == "recovering" for s in out["state"]["nodes"].values())
    assert out["state"]["mass_height"] == 100


def test_a_mass_restart_restarts_a_stuck_node_and_keeps_it_stuck():
    topo = topo_of()
    state = init(MASS, topo)
    state["next_fault_at"] = 0.0
    state = with_health(state, node1="stuck")
    out = step(state, observe(topo), 10.0, MASS, topo)
    assert {a["node"] for a in out["actions"]} == set(topo["nodes"])
    assert "node1" not in {e["node"] for e in out["events"]}
    assert len(out["events"]) == 9
    state = out["state"]
    for now, tip in ((15.0, 100), (20.0, 101), (25.0, 101), (30.0, 101)):
        state = step(state, observe(topo, tip=tip), now, MASS, topo)["state"]
    assert state["nodes"]["node1"]["health"] == "stuck"
    assert state["nodes"]["node0"]["health"] == "catching_up"


def test_a_mass_restart_waits_for_open_faults():
    topo = topo_of()
    state = with_health(init(MASS, topo), node2="recovering")
    state["next_fault_at"] = 0.0
    waiting = step(state, observe(topo), 10.0, MASS, topo)
    assert waiting["actions"] == []
    state = with_health(waiting["state"], node2="up")
    assert len(step(state, observe(topo), 15.0, MASS, topo)["actions"]) == 10


def test_a_rotation_waits_on_the_mass_restart_and_blocks_others_until_recovered():
    topo = topo_of()
    state = init(ROTATION, topo)
    state["faults"], state["next_fault_at"] = 1, 0.0
    flat = observe(topo)
    issued = step(state, flat, 10.0, ROTATION, topo)
    assert len(issued["actions"]) == 10
    state = issued["state"]
    state["next_fault_at"] = 0.0
    state["faults"] = 2  # rotation back at `restart`
    for now in (15.0, 20.0, 25.0):
        # no progress past the restart height: nobody rejoins, nothing else is issued
        blocked = step(state, flat, now, ROTATION, topo)
        assert blocked["actions"] == [] and blocked["events"] == []
        state = blocked["state"]
    state = with_health(state, **{f"node{i}": "up" for i in range(10)})
    assert len(step(state, flat, 30.0, ROTATION, topo)["actions"]) == 1


# TEST:mass-restart-rejoin-needs-progress
def test_mass_rejoin_needs_progress_past_the_restart_height():
    topo = topo_of()
    state = init(MASS, topo)
    state["next_fault_at"] = 0.0
    state = step(state, observe(topo), 10.0, MASS, topo)["state"]
    for now in (15.0, 20.0, 25.0):
        out = step(state, observe(topo, tip=100), now, MASS, topo)
        assert out["events"] == []
        state = out["state"]
    first = step(state, observe(topo, tip=101), 30.0, MASS, topo)
    done = step(first["state"], observe(topo, tip=101), 35.0, MASS, topo)
    rejoined = [e for e in done["events"] if e["event"] == "rejoined"]
    assert len(rejoined) == 10
    assert {e["ts"] for e in rejoined} == {30.0}
    assert all(
        s["health"] == ("catching_up" if name in queries_of(topo) else "up")
        for name, s in done["state"]["nodes"].items()
    )


def test_mass_rejoin_compares_to_the_highest_answer_and_needs_an_answer():
    topo = topo_of()
    state = init(MASS, topo)
    state["next_fault_at"] = 0.0
    state = step(state, observe(topo), 10.0, MASS, topo)["state"]
    state = step(state, observe(topo), 12.0, MASS, topo)["state"]
    obs = observe(
        topo,
        tip=110,
        view=110,
        node1=None,
        node2=(105, 110, 105, (0, 0, 0)),
        node3=(110, 100, 110, (0, 0, 0)),
    )
    first = step(state, obs, 15.0, MASS, topo)
    out = step(first["state"], obs, 20.0, MASS, topo)
    health = {n: s["health"] for n, s in out["state"]["nodes"].items()}
    assert health["node1"] == health["node2"] == health["node3"] == "recovering"
    assert health["node0"] == "catching_up" and health["node4"] == "up"


def test_mass_restart_query_nodes_need_caught_up():
    topo = topo_of()
    queries = queries_of(topo)
    state = init(MASS, topo)
    state["next_fault_at"] = 0.0
    state = step(state, observe(topo), 10.0, MASS, topo)["state"]
    state = step(state, observe(topo), 12.0, MASS, topo)["state"]
    unsynced = {q: (110, 110, 110, (3, 0, 0)) for q in queries}
    now = 15.0
    for _ in range(8):
        out = step(state, observe(topo, tip=110, view=110, **unsynced), now, MASS, topo)
        state, now = out["state"], now + 5.0
    assert {
        n for n, s in state["nodes"].items() if s["health"] == "catching_up"
    } == queries
    for _ in range(3):
        out = step(state, observe(topo, tip=110, view=110), now, MASS, topo)
        state, now = out["state"], now + 5.0
    assert all(s["health"] == "up" for s in state["nodes"].values())


def test_mass_restart_times_out_per_node():
    topo = topo_of()
    state = init(MASS, topo)
    state["next_fault_at"] = 0.0
    state = step(state, observe(topo), 10.0, MASS, topo)["state"]
    silent = {"node4": None}
    out = step(
        state, observe(topo, tip=110, view=110, **silent), 10.0 + 300.0, MASS, topo
    )
    timeouts = [e for e in out["events"] if e["event"] == "timeout"]
    assert {e["node"] for e in timeouts} == {f"node{i}" for i in range(10)}


def mass_issued(topo):
    state = init(MASS, topo)
    state["next_fault_at"] = 0.0
    return state, step(state, observe(topo), 10.0, MASS, topo)


def test_a_rejoin_needs_a_height_above_the_first_answers_after_the_restart():
    # Blocks decided between the probe and the stop are stored and reported again.
    topo = topo_of()
    _, issued = mass_issued(topo)
    state = issued["state"]
    for now in (15.0, 20.0, 25.0):
        out = step(state, observe(topo, tip=103), now, MASS, topo)
        assert out["events"] == []
        state = out["state"]
    assert state["mass_height"] == 103
    state = step(state, observe(topo, tip=104), 30.0, MASS, topo)["state"]
    done = step(state, observe(topo, tip=104), 35.0, MASS, topo)
    assert len([e for e in done["events"] if e["event"] == "rejoined"]) == 10


def test_a_mass_restart_rejoins_after_its_started_event():
    topo = topo_of()
    _, issued = mass_issued(topo)
    started = ch.restarted_events(issued, 40.0)
    state = ch.with_started(issued["state"], started)
    for now, tip in ((45.0, 105), (50.0, 106), (55.0, 106)):
        out = step(state, observe(topo, tip=tip), now, MASS, topo)
        state = out["state"]
    rejoined = [e for e in out["events"] if e["event"] == "rejoined"]
    assert len(rejoined) == 10
    assert {e["after_s"] for e in rejoined} == {10.0}


def test_a_lone_node_up_does_not_rejoin():
    topo = topo_of()
    _, issued = mass_issued(topo)
    state = issued["state"]
    alone = {f"node{i}": None for i in range(1, 10)}
    for now in (15.0, 20.0, 25.0, 30.0):
        out = step(state, observe(topo, tip=105, **alone), now, MASS, topo)
        assert out["events"] == []
        state = out["state"]


def test_a_partly_failed_mass_restart_retries_only_the_failed_nodes():
    topo = topo_of()
    state, issued = mass_issued(topo)
    reverted = ch.revert_failed(state, issued, {"node3"})
    assert reverted["state"]["fault_retry"] == {"node3": ("mass-restart", 10.0)}
    assert reverted["state"]["nodes"]["node3"] == state["nodes"]["node3"]
    assert reverted["state"]["faults"] == 1
    assert len(reverted["actions"]) == 9 and len(reverted["events"]) == 9
    retry = step(reverted["state"], observe(topo), 15.0, MASS, topo)
    assert retry["actions"] == [{"node": "node3", "op": "restart"}]
    (fault,) = [e for e in retry["events"] if e["event"] == "fault"]
    assert (fault["node"], fault["ts"]) == ("node3", 10.0)
    assert retry["state"]["fault_retry"] == {}
    assert retry["state"]["nodes"]["node3"]["health"] == "recovering"


def test_a_failed_retry_keeps_the_rejoined_events_of_the_same_tick():
    topo = topo_of()
    state, issued = mass_issued(topo)
    state = ch.revert_failed(state, issued, {"node3"})["state"]
    for now, tip in ((15.0, 100), (20.0, 101)):
        out = step(state, observe(topo, tip=tip), now, MASS, topo)
        state = ch.revert_failed(state, out, {"node3"})["state"]
    out = step(state, observe(topo, tip=101), 25.0, MASS, topo)
    assert {e["event"] for e in out["events"]} == {"rejoined", "fault"}
    kept = ch.revert_failed(state, out, {"node3"})
    assert len(kept["events"]) == 9
    assert kept["state"]["fault_retry"] == {"node3": ("mass-restart", 10.0)}
    assert kept["state"]["nodes"]["node0"]["health"] == "catching_up"


def test_restarted_events_follow_the_restart_that_finished():
    topo = topo_of()
    state, issued = mass_issued(topo)
    kept = ch.revert_failed(state, issued, {"node3"})
    started = ch.restarted_events(kept, 40.0)
    assert {e["node"] for e in started} == set(topo["nodes"]) - {"node3"}
    assert {(e["event"], e["ts"], e["after_s"]) for e in started} == {
        ("started", 40.0, 30.0)
    }


def test_a_mass_restart_counts_as_during_fault_for_the_floor():
    events = [
        ch.chaos_event(100.0, "fault", f"node{i}", False, "mass-restart", 50, None)
        for i in range(3)
    ] + [
        ch.chaos_event(
            110.0 + i, "rejoined", f"node{i}", False, "mass-restart", 60, 10.0
        )
        for i in range(3)
    ]
    spans = [ch.floor_span(r, 500.0) for r in ch.fault_rows(events)]
    assert spans == [(100.0, 123.0 + i) for i in range(3)]


def test_mass_restart_line_from_synthetic_data():
    ev = ch.chaos_event
    events = [
        *(
            ev(1000.0, "fault", f"node{i}", i < 2, "mass-restart", 500 + i, None)
            for i in range(4)
        ),
        *(
            ev(1004.0 + i, "started", f"node{i}", i < 2, "mass-restart", None, 4.0)
            for i in range(4)
        ),
        *(
            ev(1010.0 + i, "rejoined", f"node{i}", i < 2, "mass-restart", 600, 10.0)
            for i in range(4)
        ),
        ev(1025.0, "caught_up", "node0", True, "mass-restart", 600, 25.0),
        ev(1030.0, "caught_up", "node1", True, "mass-restart", 600, 30.0),
    ]
    # 505 shown before the restarts finished (1007) is the baseline, not a decide
    heights = [(999.0, 504), (1003.0, 505), (1007.2, 505), (1007.9, 506), (1008.5, 507)]
    lines = ch.mass_lines(ch.fault_rows(events), heights, 900.0, 4, None)
    assert lines == [
        (
            "- Mass restart: all 4 nodes at 100 s, first decide after 1 s, "
            "all rejoined after 13 s (all caught up after 30 s)"
        )
    ]


def test_mass_restart_line_names_what_did_not_happen():
    ev = ch.chaos_event
    events = [ev(1000.0, "fault", "node0", True, "mass-restart", 500, None)]
    (line,) = ch.mass_lines(ch.fault_rows(events), [], 1000.0, 1, None)
    assert line == (
        "- Mass restart: all 1 nodes at 0 s, first decide after never, "
        "all rejoined after never (all caught up after never)"
    )
    assert ch.mass_lines([], [], 0.0, 1, None) == []


MASS_KILL = ch.ChaosConfig(kinds=("mass-kill",), mass_nodes=4)


def mass_kill_issued(cfg=MASS_KILL, topo=None):
    topo = topo or topo_of()
    state = init(cfg, topo)
    state["next_fault_at"] = 0.0
    return state, step(state, observe(topo), 10.0, cfg, topo)


def silent(names) -> dict:
    return dict.fromkeys(names)


# REQ:sched-mass-group-size
# TEST:sched-mass-group-size-ok
def test_a_mass_group_is_the_next_nodes_from_the_cursor():
    state = init(CFG)
    order, nodes = state["order"], state["nodes"]
    assert ch._mass_group(order, 0, nodes, 3) == (order[:3], 3)
    assert ch._mass_group(order, 8, nodes, 3) == ([*order[8:], order[0]], 1)
    assert ch._mass_group(order, 4, nodes, None) == (list(nodes), 4)
    assert ch._mass_group(order, 0, nodes, 20) == (order, 0)


# TEST:sched-stuck-in-group-ok
def test_a_mass_group_skips_stuck_nodes():
    state = init(CFG)
    order = state["order"]
    nodes = with_health(state, **{order[1]: "stuck"})["nodes"]
    assert ch._mass_group(order, 0, nodes, 3) == ([order[0], *order[2:4]], 4)


def test_mass_nodes_faults_a_group_and_moves_the_cursor():
    cfg = ch.ChaosConfig(kinds=("mass-restart",), mass_nodes=3)
    topo = topo_of()
    state = init(cfg, topo)
    state["next_fault_at"] = 0.0
    out = step(state, observe(topo), 10.0, cfg, topo)
    group = state["order"][:3]
    assert [a["node"] for a in out["actions"]] == group
    assert out["state"]["cursor"] == 3
    assert {n for n, s in out["state"]["nodes"].items() if s["health"] != "up"} == set(
        group
    )


def test_a_mass_kill_leaves_a_stuck_node_alone():
    cfg = ch.ChaosConfig(kinds=("mass-kill",))
    topo = topo_of()
    state = with_health(init(cfg, topo), node1="stuck")
    state["next_fault_at"] = 0.0
    out = step(state, observe(topo), 10.0, cfg, topo)
    assert {a["node"] for a in out["actions"]} == set(topo["nodes"]) - {"node1"}
    assert {a["op"] for a in out["actions"]} == {"kill"}
    assert {e["kind"] for e in out["events"]} == {"mass-kill"}


# REQ:sched-mass-kill-halt
# TEST:sched-mass-kill-halt-ok
def test_a_mass_kill_beyond_the_bound_rejoins_only_after_a_new_decide():
    topo = topo_of()
    state, issued = mass_kill_issued(topo=topo)
    members = [a["node"] for a in issued["actions"]]
    assert len(members) == 4 and ch.halts(state["stake"], members)
    assert {a["op"] for a in issued["actions"]} == {"kill"}
    state = issued["state"]
    assert {state["nodes"][m]["health"] for m in members} == {"down"}
    down = silent(members)
    out = step(state, observe(topo, **down), 65.0, MASS_KILL, topo)
    assert out["actions"] == []
    out = step(out["state"], observe(topo, **down), 70.0, MASS_KILL, topo)
    assert {(a["node"], a["op"]) for a in out["actions"]} == {
        (m, "start") for m in members
    }
    state = out["state"]
    for now in (75.0, 80.0, 85.0, 90.0):
        out = step(state, observe(topo, tip=100), now, MASS_KILL, topo)
        assert [e for e in out["events"] if e["event"] == "rejoined"] == []
        state = out["state"]
    assert state["mass_height"] == 100
    for now in (95.0, 100.0):
        out = step(state, observe(topo, tip=101), now, MASS_KILL, topo)
        state = out["state"]
    assert {e["node"] for e in out["events"] if e["event"] == "rejoined"} == set(
        members
    )


# TEST:sched-silent-member-ok
def test_a_silent_member_times_out_alone():
    topo = topo_of()
    state, issued = mass_kill_issued(topo=topo)
    members = [a["node"] for a in issued["actions"]]
    mute = members[0]
    state, events = issued["state"], []
    for now in range(15, 316, 5):
        tip = 100 if now <= 75 else 101
        out = step(
            state, observe(topo, tip=tip, **{mute: None}), float(now), MASS_KILL, topo
        )
        state, events = out["state"], [*events, *out["events"]]
    assert {e["node"] for e in events if e["event"] == "timeout"} == {mute}
    assert {e["node"] for e in events if e["event"] == "rejoined"} == set(members) - {
        mute
    }
    assert state["nodes"][mute]["health"] == "stuck"


# TEST:sched-small-group-ok
def test_a_mass_group_below_the_bound_needs_a_decide_after_its_own_answers():
    cfg = ch.ChaosConfig(kinds=("mass-restart",), mass_nodes=3)
    topo = topo_of()
    state = init(cfg, topo)
    state["next_fault_at"] = 0.0
    out = step(state, observe(topo), 10.0, cfg, topo)
    group = {a["node"] for a in out["actions"]}
    assert not ch.halts(state["stake"], group)
    state = out["state"]
    for now, tip in ((15.0, 100), (20.0, 101), (25.0, 101)):
        out = step(state, observe(topo, tip=tip), now, cfg, topo)
        state = out["state"]
    assert state["mass_height"] == 100
    assert {e["node"] for e in out["events"] if e["event"] == "rejoined"} == group
    assert {n for n, s in state["nodes"].items() if s["kind"] is None} >= (
        set(topo["nodes"]) - group
    )


# REQ:sched-mass-retry-kind
# TEST:sched-mass-retry-kind-ok
def test_a_failed_mass_kill_member_is_killed_again_at_the_fault_time():
    topo = topo_of()
    state, issued = mass_kill_issued(topo=topo)
    failed = issued["actions"][0]["node"]
    reverted = ch.revert_failed(state, issued, {failed})
    assert reverted["state"]["fault_retry"] == {failed: ("mass-kill", 10.0)}
    assert reverted["state"]["nodes"][failed]["health"] == "up"
    retry = step(reverted["state"], observe(topo), 15.0, MASS_KILL, topo)
    assert retry["actions"] == [{"node": failed, "op": "kill"}]
    (fault,) = [e for e in retry["events"] if e["event"] == "fault"]
    assert (fault["node"], fault["kind"], fault["ts"]) == (failed, "mass-kill", 10.0)
    assert retry["state"]["nodes"][failed]["health"] == "down"
    assert retry["state"]["nodes"][failed]["since"] == 10.0


# REQ:report-mass-lines
# TEST:report-mass-lines-ok
def test_mass_lines_name_the_kind_and_the_group_size():
    events = [
        *(ev(0, "fault", f"node{i}", "mass-kill") for i in range(4)),
        *(ev(60, "started", f"node{i}", "mass-kill") for i in range(4)),
        *(ev(75, "rejoined", f"node{i}", "mass-kill") for i in range(4)),
    ]
    heights = [(T0, 10), (T0 + 62, 10), (T0 + 70, 11)]
    (line,) = ch.mass_lines(ch.fault_rows(events), heights, T0, 10, 4)
    assert line == (
        "- Mass kill: 4 of 10 nodes at 0 s, first decide after 10 s, "
        "all rejoined after 75 s (all caught up after never)"
    )


def test_mass_lines_say_all_nodes_when_a_stuck_node_got_no_fault_event():
    events = [
        *(ev(0, "fault", f"node{i}", "mass-restart") for i in range(9)),
        *(ev(75, "rejoined", f"node{i}", "mass-restart") for i in range(9)),
    ]
    (line,) = ch.mass_lines(ch.fault_rows(events), [], T0, 10, None)
    assert line.startswith("- Mass restart: all 10 nodes at 0 s")


def test_mass_lines_leave_out_catch_up_without_query_nodes():
    events = [
        *(ev(0, "fault", f"node{i}", "mass-kill") for i in range(4, 8)),
        *(ev(60, "started", f"node{i}", "mass-kill") for i in range(4, 8)),
        *(ev(75, "rejoined", f"node{i}", "mass-kill") for i in range(4, 8)),
    ]
    (line,) = ch.mass_lines(ch.fault_rows(events), [], T0, 10, 4)
    assert line.endswith("all rejoined after 75 s")


# REQ:report-kind-dedupe
# TEST:report-kind-dedupe-ok
def test_recovery_table_prints_one_row_per_kind():
    kinds = ["kill", "restart", "kill", "mass-kill", "mass-restart"]
    table = ch.recovery_table(ch.fault_rows(EVENTS), kinds)
    assert [row.split(" | ")[0:3] for row in table[2:]] == [
        ["| kill", "1", "start"],
        ["| restart", "1", "fault"],
        ["| mass-kill", "0", "start"],
        ["| mass-restart", "0", "restart"],
    ]


def heal_scrape(
    ts: float, targets: float, done: float, fetched: float = 0, inflight: float = 0
) -> ch.HealSample:
    p = ch.HEAL
    return ts, {
        p + "targets": targets,
        p + "done_total": done,
        p + "fetch_bytes_total": fetched,
        p + "inflight": inflight,
        p + "unreachable": 0,
        p + "verify_fail_total": 0,
        p + 'height_seconds_sum{phase="fetch"}': done * 0.5,
        p + 'height_seconds_count{phase="fetch"}': done,
    }


def share_wipe_rows() -> list[ch.FaultRow]:
    return ch.fault_rows(
        [
            ev(0, "fault", "node5", "share-wipe"),
            ev(10, "rejoined", "node5", "share-wipe"),
        ]
    )


def test_heal_rows_healed() -> None:
    series = {
        "node5": [
            heal_scrape(T0 + 10, 100, 0),
            heal_scrape(T0 + 30, 40, 60, 90e6, inflight=8),
            heal_scrape(T0 + 50, 0, 100, 150e6, inflight=3),
            heal_scrape(T0 + 60, 0, 100, 150e6),
        ]
    }
    [heal] = ch.heal_rows(share_wipe_rows(), series)
    assert heal.heal_s == pytest.approx(40)
    assert (heal.heights, heal.fetched_mb, heal.max_inflight) == (100, 150, 8)
    assert heal.payload_mb_s == pytest.approx(150 / 1.5 / 40)
    assert heal.phase_ms == {"fetch": pytest.approx(500)}
    assert "| node5 | 40 s | 100 | 150 |" in "\n".join(ch.heal_table([heal]))


def test_heal_rows_still_healing() -> None:
    series = {
        "node5": [heal_scrape(T0 + 10, 100, 0), heal_scrape(T0 + 30, 70, 30, 30e6)]
    }
    [heal] = ch.heal_rows(share_wipe_rows(), series)
    assert heal.heal_s is None
    assert "still healing, 70 left after 20 s" in "\n".join(ch.heal_table([heal]))


def test_heal_rows_ignores_idle_targets_before_any_height_healed() -> None:
    series = {"node5": [heal_scrape(T0 + 10, 0, 0), heal_scrape(T0 + 20, 0, 0)]}
    [heal] = ch.heal_rows(share_wipe_rows(), series)
    assert heal.heal_s is None


def test_heal_rows_omitted_without_metrics_or_share_wipe() -> None:
    assert ch.heal_rows(share_wipe_rows(), {"node5": []}) == []
    assert ch.heal_table([]) == []
    restart = ch.fault_rows(
        [ev(0, "fault", "node5", "restart"), ev(10, "rejoined", "node5", "restart")]
    )
    assert ch.heal_rows(restart, {"node5": [heal_scrape(T0 + 10, 0, 5)]}) == []


def test_read_heal_series(tmp_path: Path) -> None:
    lines = [
        {"ts": 1, "node": "node5", "ok": True, "m": {"x": 1.0}},
        {
            "ts": 2,
            "node": "node5",
            "ok": True,
            "m": {
                ch.HEAL + "targets": 3.0,
                ch.HEAL + 'height_seconds_bucket{le="1"}': 2.0,
            },
        },
        {"ts": 2, "node": "node6", "ok": True, "m": {ch.HEAL + "targets": 4.0}},
    ]
    (tmp_path / "metrics.jsonl").write_text("\n".join(map(json.dumps, lines)))
    assert ch.read_heal_series(tmp_path, {"node5"}) == {
        "node5": [(2, {ch.HEAL + "targets": 3.0})]
    }
    assert ch.read_heal_series(tmp_path / "none", {"node5"}) == {"node5": []}
