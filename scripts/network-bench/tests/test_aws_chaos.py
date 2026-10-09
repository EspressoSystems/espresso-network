import dataclasses
import json
import signal
from pathlib import Path
from typing import Any

import chaos as ch
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
    make_result,
    write_collected_run,
)
from test_chaos import peers_of, topo_of, with_health


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


LOADING = {"phase": "loading", "detail": "x"}


def parse(*argv: str) -> Any:
    return awsb.parse_args(["run", "--tag", "t", *argv])


# REQ:awsbench-chaos-shape
# TEST:chaos-shape-defaults-ok
def test_chaos_shape_defaults():
    cfg = awsb.config_from_args(parse("--chaos"))
    assert (cfg.nodes, cfg.query_nodes) == (22, 22)
    assert (cfg.node_type, cfg.ctl_type) == ("c8g.xlarge", "c8g.xlarge")
    assert cfg.query_engine == "sqlite"
    assert cfg.chaos == ch.ChaosConfig()
    assert cfg.load.steps == (4.0,) * 10
    assert cfg.load.step_s == awsb.CHAOS_STEP_S
    assert cfg.load.keep_going
    assert cfg.load.submit_nodes == 22
    assert awsb.fleet_config_from_args(parse("--chaos")).query_engine == "sqlite"


def test_chaos_defaults_the_latency_profile():
    assert awsb.config_from_args(parse("--chaos")).latency == "decaf-2025"
    assert awsb.fleet_config_from_args(parse("--chaos")).latency == "decaf-2025"


@pytest.mark.parametrize("profile", ("off", "mainnet"))
def test_explicit_latency_wins_under_chaos(profile):
    assert (
        awsb.config_from_args(parse("--chaos", "--latency", profile)).latency == profile
    )
    cfg = awsb.config_from_args(parse("--chaos", f"--latency={profile}"))
    assert cfg.latency == profile


def test_chaos_accepts_latency_flags_valid_for_the_resolved_profile():
    cfg = awsb.config_from_args(
        parse("--chaos", "--tcp-cc", "cubic", "--no-intra-latency", "--mtu", "9000")
    )
    assert cfg.latency == "decaf-2025"


def test_chaos_latency_off_refuses_latency_flags():
    with pytest.raises(awsb.Refused, match="--mtu"):
        awsb.config_from_args(parse("--chaos", "--latency", "off", "--mtu", "9000"))


def test_latency_stays_off_without_chaos():
    assert awsb.config_from_args(parse()).latency == "off"


def test_chaos_summary_has_the_latency_profile():
    cfg = awsb.config_from_args(parse("--chaos"))
    assert "latency decaf-2025;" in awsb.format_node_summary(cfg)[-1]


def test_chaos_nodes_without_query_nodes_makes_every_node_a_query_node():
    cfg = awsb.config_from_args(parse("--chaos", "--nodes", "9"))
    assert (cfg.nodes, cfg.query_nodes) == (9, 9)
    cfg = awsb.config_from_args(parse("--chaos", "--nodes", "9", "--query-nodes", "4"))
    assert cfg.query_nodes == 4


def test_chaos_rejoin_and_node_env_set_the_sync_status_ttl(tmp_path: Path):
    render(tmp_path, chaos_cfg())
    for file in ("node.env", "node-rejoin.env"):
        text = (tmp_path / "hosts/node0" / file).read_text()
        assert "ESPRESSO_NODE_SYNC_STATUS_TTL=5s\n" in text
        assert "ESPRESSO_NODE_API_PEERS=" in text
        text = (tmp_path / "hosts/node5" / file).read_text()
        assert "SYNC_STATUS_TTL" not in text
    plain = tmp_path / "plain"
    render(plain, dataclasses.replace(chaos_cfg(), chaos=None))
    assert "SYNC_STATUS_TTL" not in (plain / "hosts/node0/node.env").read_text()


def test_chaos_wipe_keeps_live_peers_with_every_node_a_query_node():
    hosts = awsb.plan_hosts(dataclasses.replace(chaos_cfg(), nodes=7, query_nodes=7))
    peers = awsb.plan_peers(hosts)
    assert all(len(p) == 3 for p in peers.values())


def test_chaos_log_omits_the_capacity_verdict(caplog):
    result = make_result()
    with caplog.at_level("INFO"):
        awsb.log_result(result, chaos=True)
    assert "Capacity" not in caplog.text
    with caplog.at_level("INFO"):
        awsb.log_result(result, chaos=False)
    assert "Capacity" in caplog.text


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
    assert cfg.query_nodes == 7 and cfg.node_type == "c8g.xlarge"


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
    chaos = ch.ChaosConfig(minutes=3, seed=7, kinds=("kill", "wipe"))
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
        chaos=ch.ChaosConfig(),
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
    state = ch.chaos_init(ch.ChaosConfig(), list(ctrl.topo["nodes"]), 0.0, 60.0, 600.0)
    ctrl.state = with_health(state, node4="down", node5="down", node6="recovering")
    ctrl.restore()
    starts = [c[-1] for c in runner.calls if "docker start" in c[-1]]
    assert starts == ["sudo docker start espresso-node"] * 2
    events = ch.read_events(isolated)
    assert [(e["event"], e["node"]) for e in events] == [
        ("restored", "node4"),
        ("restored", "node5"),
        ("interrupted", "node4"),
        ("interrupted", "node5"),
        ("interrupted", "node6"),
    ]


def test_restore_leaves_stuck_nodes_alone(isolated: Path):
    ctrl, runner = scripted_controller(isolated)
    state = ch.chaos_init(ch.ChaosConfig(), list(ctrl.topo["nodes"]), 0.0, 60.0, 600.0)
    ctrl.state = with_health(state, node4="stuck")
    ctrl.restore()
    assert not [c for c in runner.calls if "docker start" in c[-1]]
    assert ch.read_events(isolated) == []


# TEST:tick-waiting-noop-ok
@pytest.mark.parametrize("agent", [None, {"phase": "waiting", "detail": "x"}])
def test_tick_before_loading_does_nothing(isolated: Path, agent):
    ctrl, runner = scripted_controller(isolated)
    calls = len(runner.calls)
    ctrl.tick(agent)
    assert ctrl.state is None and ctrl.expected_down() == frozenset()
    assert len(runner.calls) == calls
    assert ch.read_events(isolated) == []


class FlakyRunner(ClusterRunner):
    """The first `probes` chaos probes and `restarts` docker restarts fail over ssh."""

    probes = 0
    restarts = 0

    def default(self, argv: list[str]) -> Any:
        command = argv[-1] if argv[0] == "ssh" else ""
        if self.probes and "consensus_last_voted_view" in command:
            self.probes -= 1
            return completed(returncode=awsb.SSH_FAILED_RC, stderr="Connection reset")
        if self.restarts and "docker restart" in command:
            self.restarts -= 1
            return completed(returncode=awsb.SSH_FAILED_RC, stderr="Connection reset")
        return super().default(argv)


def cluster_controller(
    isolated: Path, probes: int = 0, restarts: int = 0, **chaos: Any
) -> tuple[Any, Any, Any]:
    clock = FakeClock(start=FAKE_EPOCH, limit_s=7200)
    names = [f"node{i}" for i in range(7)]
    cluster = FakeCluster(clock, names, {"node0", "node1", "node2"})
    runner = FlakyRunner(cluster, 3, [LOADING])
    runner.probes, runner.restarts = probes, restarts
    cfg = dataclasses.replace(chaos_cfg(), chaos=ch.ChaosConfig(**chaos))
    ctrl = make_controller(isolated, runner, runner.hosts, clock, cfg)
    ctrl.state = ch.chaos_init(
        cfg.chaos, list(ctrl.topo["nodes"]), clock.time(), 60.0, 0.0
    )
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
    assert [e["event"] for e in ch.read_events(isolated)] == ["started", "rejoined"]
    assert clock.time() - FAKE_EPOCH >= ch.ChaosConfig().kill_down_s


# TEST:drain-timeout-fails
def test_drain_returns_past_a_stuck_node(isolated: Path):
    ctrl, _, clock = cluster_controller(isolated, kill_down_s=1e9, recover_timeout_s=60)
    ctrl.drain(interrupts_of(clock))
    assert [e["event"] for e in ch.read_events(isolated)] == ["timeout"]
    assert ctrl.expected_down() == frozenset({"node4"})
    assert ctrl.stuck() == ["node4"]


def test_failed_probes_skip_ticks_until_the_limit(isolated: Path):
    ctrl, _, clock = cluster_controller(isolated, probes=awsb.CHAOS_SSH_FAILURES - 1)
    ctrl.drain(interrupts_of(clock))
    assert [e["event"] for e in ch.read_events(isolated)] == ["started", "rejoined"]
    ctrl, _, clock = cluster_controller(isolated, probes=awsb.CHAOS_SSH_FAILURES)
    with pytest.raises(awsb.RemoteError, match="consecutive"):
        ctrl.drain(interrupts_of(clock))


def test_failed_fault_is_retried_on_the_next_tick(isolated: Path):
    ctrl, cluster, clock = cluster_controller(isolated, restarts=1, kinds=("restart",))
    cluster.start("node4")
    ctrl.state = ch.chaos_init(
        ctrl.chaos, list(ctrl.topo["nodes"]), clock.time(), 0.0, 3600.0
    )
    ctrl.tick(LOADING)
    assert ctrl.remote.run.restarts == 0
    assert ch.read_events(isolated) == []
    assert ctrl.expected_down() == frozenset()
    clock.sleep(awsb.AGENT_POLL_S)
    ctrl.tick(LOADING)
    assert [e["event"] for e in ch.read_events(isolated)] == ["fault"]


# TEST:timeout-event-invalid-ok
def test_timeout_event_makes_the_run_invalid():
    event = ch.chaos_event(300.0, "timeout", "node4", "kill", None, 300.0)
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
        "chaos": dataclasses.asdict(ch.ChaosConfig()),
    }
    netbench.write_json(tmp_path / "manifest.json", manifest)
    meta = netbench.read_json(tmp_path / "load-meta.json")
    netbench.write_json(tmp_path / "load-meta.json", {**meta, "submit_failovers": 5})
    run = netbench.read_json(tmp_path / "run.json")
    ev = ch.chaos_event
    events = [
        ev(run["t0"] + 5, "fault", "node1", "restart", 9, None),
        ev(run["t0"] + 20, "rejoined", "node1", "restart", None, 15.0),
    ]
    (tmp_path / "chaos.jsonl").write_text("".join(json.dumps(e) + "\n" for e in events))
    result = awsb.write_report(tmp_path)
    summary = (tmp_path / "summary.md").read_text()
    assert "Capacity" not in summary
    assert summary.startswith("## Chaos test")
    assert "| node1 | restart | 5 | - | 20 | - |" in summary
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
        digest(dataclasses.replace(base, chaos=ch.ChaosConfig(seed=7))),
        digest(dataclasses.replace(base, chaos=ch.ChaosConfig(rate_mb_s=2.0))),
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
        "--latency",
        "off",
        "--chaos-min",
        "6",
        "--chaos-seed",
        str(seed),
        "--query-nodes",
        "3",
        nodes=7,
    )
    assert awsb.cmd_run(args, FakeSystem(run=runner, clock=clock)) == awsb.EXIT_OK
    events = ch.read_events(run_harness.run_dir)
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
        order = ch.chaos_init(
            ch.ChaosConfig(seed=seed), list(topo["nodes"]), 0.0, 0.0, 0.0
        )["order"]
        if order[0] in queries:
            return seed
    raise AssertionError("no seed")


class NoStart(ClusterRunner):
    """`docker start` answers 0 without starting the node: a killed node never comes back."""

    def default(self, argv: list[str]) -> Any:
        if argv[0] == "ssh" and argv[-1] == awsb.DOCKER_NODE_COMMANDS["start"]:
            return completed()
        return super().default(argv)


@pytest.mark.usefixtures("valid")
def test_stuck_node_lets_the_load_finish_and_fails_the_run(run_harness: RunHarness):
    clock = FakeClock(start=FAKE_EPOCH, limit_s=20000)
    cluster = FakeCluster(
        clock, [f"node{i}" for i in range(7)], {"node0", "node1", "node2"}
    )
    runner = NoStart(cluster, 3, [LOADING] * 80 + [DONE_STATE], describe=DESCRIBE)
    args = run_harness.args(
        "--chaos",
        "--latency",
        "off",
        "--chaos-min",
        "6",
        "--chaos-kinds",
        "kill",
        "--query-nodes",
        "3",
        nodes=7,
    )
    assert awsb.cmd_run(args, FakeSystem(run=runner, clock=clock)) == awsb.EXIT_FAILED
    events = ch.read_events(run_harness.run_dir)
    assert [e["event"] for e in events] == ["fault", "started", "timeout"]
    # Written from the agent's final state: the load ran to its end.
    assert (run_harness.run_dir / "run.json").exists()
    assert "not recovered within 300 s" in run_harness.log()


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
        "--chaos", "--latency", "off", "--chaos-min", "6", "--query-nodes", "3", nodes=7
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
        "--chaos", "--latency", "off", "--chaos-min", "6", "--query-nodes", "3", nodes=7
    )
    assert awsb.cmd_run(args, system) == awsb.EXIT_FAILED
    assert runner.ran("tofu", "destroy")
    assert all(m.running for m in cluster.members.values())
    assert "restored" in {e["event"] for e in ch.read_events(run_harness.run_dir)}
