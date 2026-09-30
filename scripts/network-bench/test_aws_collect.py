import argparse
import dataclasses
import json
import logging
import re
import signal
import threading
from collections.abc import Callable, Iterator
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import netbench
import pytest
from fakes import (
    DESCRIBE,
    DONE_STATE,
    FULL_BALANCE,
    NOW,
    FakeClock,
    FakeNode,
    FakePool,
    FakeRunner,
    FakeSystem,
    RunHarness,
    aws_manifest,
    awsb,
    balance_file,
    clean_evidence,
    clean_result,
    completed,
    fake_images,
    fleet,
    host_sample,
    metric_data,
    pg_settings,
    raiser,
    remote,
    shot_estimate,
    two_node_hosts_info,
    write_collected_run,
)


def query_spec() -> dict:
    return {
        "name": "node0",
        "role": "query",
        "instance_type": "x",
        "root_gb": 100,
        "root_iops": 6000,
        "root_mbps": 500,
    }


def two_node_cfg(**extra: Any) -> Any:
    return awsb.RunConfig(
        tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1), **extra
    )


def render_run_dir(run_dir: Path, cfg: Any) -> None:
    hosts = awsb.plan_hosts(cfg)
    for host in hosts:
        (run_dir / "hosts" / host["name"]).mkdir(parents=True)
    manifest = {"hosts": hosts, "images": fake_images()}
    awsb.render_host_files(run_dir, cfg, manifest, two_node_hosts_info())


def container_settings() -> dict[str, str]:
    args = awsb.render_postgres_args()
    assert set(args[0::2]) == {"-c"}
    return dict(setting.split("=", 1) for setting in args[1::2])


def test_sample_pg_writes_lines_and_skips_failed_ticks(tmp_path: Path) -> None:
    stop = threading.Event()
    replies = [
        completed(returncode=1, stderr="no such container"),
        completed(returncode=1, stderr="not ready"),
        completed(stdout='{"xact_commit": 7, "wait_events": {"IO": 1}}\n'),
    ]
    envs = []

    def run(argv: list[str], env: dict | None = None) -> Any:
        assert "password" not in argv
        envs.append(env)
        reply = replies.pop(0)
        if not replies:
            stop.set()
        return reply

    out = tmp_path / "pg-stats.jsonl"
    clock = FakeClock()
    awsb.sample_pg(out, stop, awsb.pg_endpoint(), FakeSystem(run=run, clock=clock))
    assert clock.sleeps == [awsb.PG_SAMPLE_S] * 2
    assert envs == [{"PGPASSWORD": "password"}] * 3
    [line] = netbench.read_jsonl(out)
    assert line["xact_commit"] == 7
    assert "ts" in line


def test_agent_host_role_query_starts_pg_sampler(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    pg_json = tmp_path / "pg.json"
    netbench.write_json(pg_json, awsb.pg_endpoint())
    args = awsb.parse_args(
        ["agent-host", str(tmp_path / "host.jsonl"), "--role", "query"]
        + ["--pg", str(pg_json)]
    )
    probed = threading.Event()

    def run(argv: list[str], env: dict | None = None) -> Any:
        system.fire(signal.SIGINT)
        probed.set()
        return completed(returncode=1)

    def await_probe(now: float) -> None:
        # The sampler thread never advances the clock: its first probe stops the loop.
        assert probed.wait(1), "the pg sampler never probed"

    system = FakeSystem(run=run, clock=FakeClock(on_advance=await_probe))
    monkeypatch.setattr(awsb, "BENCH_DIR", str(tmp_path))
    monkeypatch.setattr(awsb, "host_sample", lambda *a: {"ts": 1})
    assert awsb.cmd_agent_host(args, system) == awsb.EXIT_OK
    assert (tmp_path / "pg-stats.jsonl").exists()


# REQ:querydb-settings-parity
def test_container_args_render_every_tuning_key() -> None:
    settings = container_settings()
    assert {k: settings[k] for k in awsb.PG_TUNING} == {
        k: conf for k, (conf, _) in awsb.PG_TUNING.items()
    }
    assert settings["ssl"] == "on"


def test_bind_parameters_are_not_logged() -> None:
    """Slow payload INSERT statements would otherwise log their ~6 MB bytea binds as hex."""
    assert container_settings()["log_parameter_max_length"] == "0"
    rds = awsb.rds_tfvars({}, "pw", datetime(2026, 1, 1, tzinfo=UTC))["rds"]
    assert rds["parameters"]["log_parameter_max_length"] == "0"


def test_memory_budget_fits_the_query_host() -> None:
    """node0 is a c8g.4xlarge with 32 GiB; the same map runs on the 64 GiB RDS class."""
    assert awsb.NODE_TYPE == "c8g.4xlarge"
    ram = 32 * 1024**3
    pages, kib = 8192, 1024

    def size(key: str, unit: int) -> int:
        return int(awsb.PG_TUNING[key][1]) * unit

    peak = (
        size("shared_buffers", pages)
        + size("wal_buffers", pages)
        + size("autovacuum_max_workers", 1) * size("autovacuum_work_mem", kib)
    )
    assert peak <= ram // 3
    assert size("effective_cache_size", pages) <= ram


def test_rds_values_are_the_conf_values_in_setting_units() -> None:
    sizes = {"kB": 1024, "MB": 1024**2, "GB": 1024**3}
    times = {"s": 1, "min": 60}
    units = {
        "shared_buffers": 8192,
        "effective_cache_size": 8192,
        "wal_buffers": 8192,
        "work_mem": 1024,
        "maintenance_work_mem": 1024,
        "autovacuum_work_mem": 1024,
        "max_wal_size": 1024**2,
        "min_wal_size": 1024**2,
    }
    for key, (conf, setting) in awsb.PG_TUNING.items():
        match = re.fullmatch(r"(\d+)([A-Za-z]+)", conf)
        if match is None:
            assert setting == conf, key
            continue
        number, suffix = int(match[1]), match[2]
        if suffix in times:
            assert int(setting) == number * times[suffix], key
        else:
            assert int(setting) * units[key] == number * sizes[suffix], key


@pytest.mark.parametrize(
    ("overrides", "differing"),
    [
        ({}, []),
        (
            {"shared_buffers": "16384", "max_wal_size": "1024"},
            ["shared_buffers", "max_wal_size"],
        ),
    ],
)
def test_check_pg_tuning(overrides: dict, differing: list[str]) -> None:
    assert awsb.check_pg_tuning(pg_settings(**overrides)) == differing


def test_tls_is_counted_by_the_load_samples_not_the_collect_query() -> None:
    """Freeze stops espresso-node before the settings query, so only the sampler sees it."""
    assert "pg_stat_ssl" not in awsb.PG_SETTINGS_SQL
    sql = awsb.pg_sample_sql()
    ssl = sql[sql.index("'ssl_backends'") :]
    assert "pg_stat_ssl" in ssl
    assert "pid <> pg_backend_pid()" in ssl


def check_aws(
    result: dict | None = None, manifest: dict | None = None, **evidence: Any
) -> dict:
    return awsb.check_validity_aws(
        result or clean_result(),
        manifest or aws_manifest(),
        {**clean_evidence(), **evidence},
    )


def test_evidence_sums_the_backends_of_the_samples_inside_the_load_window(
    tmp_path: Path,
) -> None:
    node0 = tmp_path / "hosts" / "node0"
    node0.mkdir(parents=True)
    samples = [
        {"ts": 5.0, "ssl_backends": {"false": 1}},
        {"ts": 10.0, "ssl_backends": {"true": 2}},
        {"ts": 15.0, "ssl_backends": {"true": 1}},
        {"ts": 25.0, "ssl_backends": {"false": 1}},
    ]
    netbench.write_jsonl(node0 / "pg-stats.jsonl", iter(samples))
    _, evidence = awsb.load_evidence(tmp_path, aws_manifest(), 10.0, 20.0)
    assert evidence["ssl_backends"] == {"true": 3}
    (node0 / "pg-stats.jsonl").unlink()
    _, evidence = awsb.load_evidence(tmp_path, aws_manifest(), 10.0, 20.0)
    assert "ssl_backends" not in evidence


def test_evidence_reads_settings_and_ignores_an_empty_file(tmp_path: Path) -> None:
    node0 = tmp_path / "hosts" / "node0"
    node0.mkdir(parents=True)
    (node0 / "pg-settings.json").write_text(json.dumps(pg_settings()))
    _, evidence = awsb.load_evidence(tmp_path, aws_manifest(), 0.0, 1.0)
    assert evidence["pg_settings"] == pg_settings()
    (node0 / "pg-settings.json").write_text("")
    _, evidence = awsb.load_evidence(tmp_path, aws_manifest(), 0.0, 1.0)
    assert "pg_settings" not in evidence


def test_psql_password_is_in_the_environment_only() -> None:
    argv, env = awsb.psql_argv(awsb.pg_endpoint())
    assert argv[0] == "psql"
    assert env == {"PGPASSWORD": "password"}
    assert "password" not in argv
    assert argv[argv.index("-d") + 1] == "espresso"


# REQ:querydb-colocated-wiring
def test_node_env_reads_the_endpoint() -> None:
    pg = {**awsb.pg_endpoint(), "host": "db.internal", "port": 6432}
    text = awsb.render_node_env(query_spec(), fleet(5), pg)
    env = dict(line.split("=", 1) for line in text.splitlines())
    assert env["ESPRESSO_NODE_POSTGRES_HOST"] == "db.internal"
    assert env["ESPRESSO_NODE_POSTGRES_PORT"] == "6432"


def test_node_env_of_query_role_needs_an_endpoint() -> None:
    with pytest.raises(ValueError, match="PgEndpoint"):
        awsb.render_node_env(query_spec(), fleet(5), None)


def test_pg_json_is_written_0600_for_the_query_host_only(tmp_path: Path) -> None:
    render_run_dir(tmp_path, two_node_cfg(node_env=("A=1",)))
    pg_json = tmp_path / "hosts/node0/pg.json"
    assert netbench.read_json(pg_json) == awsb.pg_endpoint()
    assert pg_json.stat().st_mode & 0o777 == 0o600
    assert not (tmp_path / "hosts/node1/pg.json").exists()
    assert not (tmp_path / "hosts/ctl/pg.json").exists()
    for node in ("node0", "node1"):
        assert "\nA=1\n" in (tmp_path / "hosts" / node / "node.env").read_text()


def test_agent_host_query_role_without_endpoint_is_refused(
    system: FakeSystem,
) -> None:
    """Refused before installing signal handlers, which would outlive the call."""
    args = awsb.parse_args(["agent-host", "host.jsonl", "--role", "query"])
    with pytest.raises(awsb.Refused, match="--pg"):
        awsb.cmd_agent_host(args, system)
    assert system.handlers == {}


@pytest.mark.slow
def test_every_node_waits_concurrently_whatever_the_pool_size(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    """Boundary test: real threads meet at a `Barrier`; a pool of 1 would time out."""
    barrier = threading.Barrier(2, timeout=0.5)

    def runner(argv: list[str]) -> Any:
        if "docker start" in argv[-1]:
            barrier.wait()
        if "inspect" in argv[-1]:
            return completed(stdout="2026-09-29T15:00:00.1Z\n")
        return completed()

    ssh = remote(runner, tmp_path)
    monkeypatch.setattr(awsb, "REMOTE_POOL_SIZE", 1)
    hosts = [ssh.hosts["node0"], ssh.hosts["node1"]]
    assert awsb.start_nodes(ssh, hosts, 0.0) == 0.0


def test_digest_mismatches() -> None:
    images = fake_images()
    good = f"ghcr.io/x/espresso-node@{images['espresso-node']['digest']}"
    assert awsb.digest_mismatches({"espresso-node": good}, images) == []
    bad = {"espresso-node": "ghcr.io/x@sha256:bad"}
    assert awsb.digest_mismatches(bad, images) == ["espresso-node"]


def test_keeps_netbench_findings() -> None:
    result = clean_result()
    result["validity"] = {"valid": False, "noisy": True, "reasons": ["base"]}
    assert check_aws(result) == {"valid": False, "noisy": True, "reasons": ["base"]}


def near_journal_limit() -> int:
    spec = next(h for h in aws_manifest()["hosts"] if h["name"] == "node1")
    return int(awsb._journal_max_bytes(spec) * 0.95)


def cover(node0: float) -> dict:
    return {"ctl": 1.0, "node0": node0, "node1": 1.0}


def ebs(byte: float | None, io: float | None) -> dict:
    return {"EBSByteBalance%": byte, "EBSIOBalance%": io}


QUIET, INVALID, NOISY = (True, False), (False, False), (True, True)


# REQ:awsbench-validity-aws
@pytest.mark.parametrize(
    ("verdict", "reason", "setup"),
    [
        (QUIET, "", {}),
        (QUIET, "", {"ssl_backends": {}}),
        (QUIET, "", {"pg_settings": pg_settings(), "ssl_backends": {"true": 3}}),
        (INVALID, "cover only 50%", {"coverage": cover(0.5)}),
        (INVALID, "cover only 0%", {"coverage": cover(0.0)}),
        (INVALID, "for postgres", {"digest_mismatch": {"node0": ["postgres"]}}),
        (INVALID, "clock offset 1500 ms", {"clock_offset_ms": {"node1": 1500.0}}),
        (NOISY, "clock offset 60 ms", {"clock_offset_ms": {"node1": 60.0}}),
        (NOISY, "nodes started 2.0 s apart", {"start_spread_s": 2.0}),
        (NOISY, "ctl CPU", {"hosts": {"ctl": host_sample(util=0.9)}}),
        (NOISY, "node1 steal", {"hosts": {"node1": host_sample(steal=6.0)}}),
        (NOISY, "node1 journal", {"journal_bytes": {"node1": near_journal_limit()}}),
        (NOISY, "EBSByteBalance% fell to 82%", {"ebs_balance_min": ebs(82.4, 100.0)}),
        (NOISY, "EBSIOBalance% has no datapoint", {"ebs_balance_min": ebs(100, None)}),
        (NOISY, "EBS balance not collected", {"ebs_balance_min": {}}),
        (
            NOISY,
            "shared_buffers=16384",
            {"pg_settings": pg_settings(shared_buffers="16384")},
        ),
        (NOISY, "without TLS", {"ssl_backends": {"true": 4, "false": 1}}),
    ],
)
def test_check_validity_aws(
    verdict: tuple[bool, bool], reason: str, setup: dict
) -> None:
    evidence = dict(setup)
    result = clean_result()
    result["hosts"] |= evidence.pop("hosts", {})
    manifest = aws_manifest() | {"start_spread_s": evidence.pop("start_spread_s", 0.5)}
    got = check_aws(result, manifest, **evidence)
    assert (got["valid"], got["noisy"]) == verdict
    assert len(got["reasons"]) == (verdict != QUIET)
    assert all(reason in r for r in got["reasons"])


def test_parse_clock_offset_ms() -> None:
    text = "Stratum: 4\nSystem time     : 0.000123456 seconds slow of NTP time\n"
    assert awsb.parse_clock_offset_ms(text) == pytest.approx(0.123456)
    with pytest.raises(ValueError):
        awsb.parse_clock_offset_ms("nothing")


def host_tick(ts: float, disk: int, net: int, cpu: list[int]) -> dict:
    io = {"read_bytes": 0, "write_bytes": disk, "io_ticks_ms": 0}
    return {
        "ts": ts,
        "cpu": cpu,
        "mem_avail": 5,
        "disk": {"nvme0n1": io, "nvme0n1p1": io},
        "net": {
            "ens5": {"rx_bytes": net, "tx_bytes": 0},
            "lo": {"rx_bytes": net * 9, "tx_bytes": 0},
        },
    }


def test_host_sample_stats_rates_and_coverage() -> None:
    samples = [
        host_tick(100.0, 0, 0, [0] * 8),
        host_tick(102.0, 4_000_000, 2_000_000, [100, 0, 100, 800, 0, 0, 0, 0]),
        host_tick(104.0, 8_000_000, 4_000_000, [200, 0, 200, 1600, 0, 0, 0, 0]),
    ]
    stats, coverage = awsb.host_sample_stats(samples, 100.0, 104.0)
    assert stats["disk_mb_s"] == pytest.approx(2.0)
    assert stats["net_mb_s"] == pytest.approx(1.0)
    assert coverage == pytest.approx(1.0)
    _, half = awsb.host_sample_stats(samples[:2], 100.0, 108.0)
    assert half == pytest.approx(0.5)


def test_sweep_terminates_then_deletes_group_and_key() -> None:
    runner = FakeRunner(states=[DONE_STATE])
    assert len(awsb.sweep(FakeSystem(run=runner), "run1")) == 3
    assert [c[3:5] for c in runner.calls if c[0] == "aws"] == [
        ["resourcegroupstaggingapi", "get-resources"],
        ["ec2", "terminate-instances"],
        ["ec2", "wait"],
        ["ec2", "delete-security-group"],
        ["ec2", "delete-key-pair"],
    ]
    assert runner.ran("Key=espresso-bench-run,Values=run1")


def test_security_group_delete_retries_on_dependency_violation(
    clock: FakeClock,
) -> None:
    attempts = []

    def runner(argv: list[str]) -> Any:
        attempts.append(argv)
        busy = len(attempts) < 3
        return completed(
            returncode=254 if busy else 0,
            stderr="DependencyViolation: has a dependent object" if busy else "",
        )

    awsb.delete_security_group(FakeSystem(run=runner, clock=clock), "sg-1")
    assert len(attempts) == 3
    assert clock.sleeps == [awsb.SG_DELETE_BACKOFF_S] * 2


@pytest.mark.parametrize(
    ("stderr", "attempts"),
    [("DependencyViolation", awsb.SG_DELETE_RETRIES), ("denied", 1)],
)
def test_security_group_delete_errors(stderr: str, attempts: int) -> None:
    runner = FakeRunner({("aws",): completed(returncode=254, stderr=stderr)})
    with pytest.raises(awsb.Refused, match=stderr):
        awsb.delete_security_group(FakeSystem(run=runner), "sg-1")
    assert len(runner.calls) == attempts


def cost_manifest() -> dict:
    cfg = two_node_cfg()
    hosts = awsb.plan_hosts(cfg)
    return {"name": "run1", "hosts": hosts, "estimate": shot_estimate(hosts, cfg)}


def test_actual_cost_prices_the_observed_duration() -> None:
    manifest = cost_manifest()
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    cost = awsb.actual_cost(runner, manifest, NOW)
    assert cost["duration_s"] == 1800.0
    lines = awsb._cost_lines(manifest["hosts"], 1800.0, None)
    assert cost["actual"] == pytest.approx(sum(line["usd"] for line in lines))
    assert cost["bound"] == manifest["estimate"]["bound_usd"]


def test_actual_cost_skips_rows_without_a_reason() -> None:
    manifest = cost_manifest()
    launch = "2026-09-29T15:00:00+00:00"
    rows = [
        {"launch": launch, "reason": None},
        {"launch": launch, "reason": "User initiated (2026-09-29 15:30:00 GMT)"},
    ]
    runner = FakeRunner(states=[DONE_STATE], describe=json.dumps(rows))
    assert awsb.actual_cost(runner, manifest, NOW)["duration_s"] == 1800.0
    only_none = FakeRunner(states=[DONE_STATE], describe=json.dumps(rows[:1]))
    with pytest.raises(ValueError):
        awsb.actual_cost(only_none, manifest, NOW)


@dataclasses.dataclass
class Agent:
    """`cmd_agent_drive` on a clock and node of its own; `sample_metrics` does nothing."""

    args: argparse.Namespace
    clock: FakeClock
    node: FakeNode
    monkeypatch: pytest.MonkeyPatch

    def run(self, ready: Callable[..., Any], load: Callable[..., Any]) -> int:
        self.monkeypatch.setattr(netbench, "wait_ready", ready)
        self.monkeypatch.setattr(netbench, "drive_load", load)
        system = FakeSystem(clock=self.clock, http_pool=self.node.connect)
        return awsb.cmd_agent_drive(self.args, system)


@pytest.fixture
def agent_out(tmp_path: Path) -> Iterator[Path]:
    root = logging.getLogger()
    level, handlers = root.level, list(root.handlers)
    yield tmp_path / "out"
    root.setLevel(level)
    for h in [h for h in root.handlers if h not in handlers]:
        root.removeHandler(h)
        h.close()
    for h in [h for h in netbench.log.handlers if isinstance(h, awsb.AgentReporter)]:
        netbench.log.removeHandler(h)


@pytest.fixture
def agent(tmp_path: Path, agent_out: Path, monkeypatch: pytest.MonkeyPatch) -> Agent:
    topo = {
        "nodes": {"node0": "http://10.0.0.2:8080"},
        "roles": {"node0": "validator"},
        "query_node": "node0",
    }
    cfg = dataclasses.asdict(netbench.BenchConfig())
    config = tmp_path / "agent.json"
    netbench.write_json(config, {"cfg": cfg, "topology": topo, "ready_timeout_s": 5})
    args = awsb.parse_args(["agent-drive", str(config), str(agent_out)])
    monkeypatch.setattr(netbench, "sample_metrics", lambda *a, **k: None)
    # `logging.basicConfig` sets INFO only on a root logger without handlers.
    logging.getLogger().setLevel(logging.INFO)
    clock = FakeClock()
    return Agent(args, clock, FakeNode(clock, include=False), monkeypatch)


def agent_state(out: Path) -> dict:
    return netbench.read_json(out / "agent-state.json")


def test_the_clock_reaches_sampler_readiness_and_load(agent: Agent) -> None:
    clocks: list[Any] = []
    pools: list[Any] = []

    def sampler(*args: Any) -> None:
        clocks.append(args[-1])

    def ready(*args: Any) -> float:
        clocks.append(args[-1])
        pools.append(args[0])
        return 1.0

    def load(*args: Any) -> tuple[float, float]:
        clocks.append(args[4])
        pools.append(args[5])
        return (1.0, 2.0)

    agent.monkeypatch.setattr(netbench, "sample_metrics", sampler)
    assert agent.run(ready, load) == awsb.EXIT_OK
    assert clocks == [agent.clock] * 3
    ready_pool, load_http = pools
    assert isinstance(ready_pool, FakePool)
    assert ready_pool.node is agent.node
    assert ready_pool.clock is agent.clock
    assert load_http == agent.node.connect


def test_agent_done_state_carries_ready_window_and_progress(
    agent: Agent, agent_out: Path
) -> None:
    def load(*args: Any) -> tuple[float, float]:
        netbench.log.info("height 7: 3 submitted")
        return (100.0, 200.0)

    assert agent.run(lambda *a: 12.0, load) == awsb.EXIT_OK
    state = agent_state(agent_out)
    assert state["phase"] == "done"
    assert (state["ready_s"], state["t0"], state["t1"]) == (12.0, 100.0, 200.0)
    assert state["progress"] == "height 7: 3 submitted"


@pytest.mark.parametrize(
    ("ready", "load", "error"),
    [
        (
            raiser(netbench.NetworkError("network not ready after 5 s: heights {}")),
            lambda *a: (1.0, 2.0),
            "network not ready after 5 s: heights {}",
        ),
        (lambda *a: 1.0, raiser(KeyboardInterrupt()), "interrupted"),
    ],
    ids=["not-ready", "interrupted"],
)
def test_agent_failure_is_an_error_state(
    agent: Agent,
    agent_out: Path,
    ready: Callable[..., Any],
    load: Callable[..., Any],
    error: str,
) -> None:
    assert agent.run(ready, load) == awsb.EXIT_INVALID
    state = agent_state(agent_out)
    assert state["phase"] == "error"
    assert error in state["error"]


def test_render_host_files_writes_env_start_topology_and_agent_config(
    tmp_path: Path,
) -> None:
    cfg = two_node_cfg()
    render_run_dir(tmp_path, cfg)
    assert (tmp_path / "hosts/ctl/ctl.env").exists()
    assert (tmp_path / "hosts/node0/node.env").exists()
    assert (tmp_path / "hosts/node1/start.sh").exists()
    topo = netbench.read_json(tmp_path / "topology.json")
    assert topo["nodes"]["node1"] == "http://10.0.0.3:8080"
    assert topo["query_node"] == "node0"
    config = netbench.read_json(tmp_path / "hosts/ctl/agent.json")
    assert config["topology"] == topo
    assert config["cfg"]["submit_nodes"] == 1
    assert awsb.load_config(config["cfg"]) == cfg.load


def test_parse_hosts_output_takes_role_from_the_spec() -> None:
    info = two_node_hosts_info()
    output = {"hosts": {"value": {h: {**info[h], "role": "x"} for h in info}}}
    parsed = awsb.parse_hosts_output(output, awsb.plan_hosts(two_node_cfg()))
    assert parsed["node0"]["role"] == "query"
    assert parsed["ctl"]["private_ip"] == "10.0.0.1"


def test_genesis_contracts_are_deduplicated(tmp_path: Path) -> None:
    genesis = Path(__file__).with_name("genesis.toml").read_text()
    (tmp_path / "genesis.toml").write_text(genesis)
    assert awsb.genesis_contracts(tmp_path) == [
        "0x8ce361602b935680e8dec218b820ff5056beb7af",
        "0xf7cd8fa9b94db2aa972023b379c7f72c65e4de9d",
    ]


def collect_ebs(runner: FakeRunner, out: Path) -> dict:
    manifest = {"hosts_info": two_node_hosts_info()}
    awsb.collect_ebs_balance(runner, manifest, 100.0, 160.0, out)
    return netbench.read_json(out / awsb.EC2_NODE0_FILE)


def test_collect_ebs_queries_node0_over_the_padded_window(tmp_path: Path) -> None:
    runner = FakeRunner(states=[DONE_STATE], balance=completed(stdout=FULL_BALANCE))
    saved = collect_ebs(runner, tmp_path)
    argv = next(c for c in runner.calls if "get-metric-data" in c)
    assert argv[argv.index("--start-time") + 1] == "1970-01-01T00:00:40+00:00"
    assert argv[argv.index("--end-time") + 1] == "1970-01-01T00:03:40+00:00"
    assert saved["instance_id"] == "i-000000000002"
    assert saved["metrics"]["EBSByteBalance%"] == {
        "timestamps": [60.0, 120.0, 180.0],
        "values": [100.0, 100.0, 100.0],
    }


def test_collect_ebs_metric_without_datapoints_is_saved_empty(tmp_path: Path) -> None:
    data = metric_data([100.0], [None])
    runner = FakeRunner(states=[DONE_STATE], balance=completed(stdout=data))
    assert collect_ebs(runner, tmp_path)["metrics"]["EBSIOBalance%"]["values"] == []


def forbidden_balance() -> str:
    data = json.loads(FULL_BALANCE)
    data["MetricDataResults"][0]["StatusCode"] = "Forbidden"
    return json.dumps(data)


@pytest.mark.parametrize(
    ("balance", "error", "match"),
    [
        (
            completed(stdout=forbidden_balance()),
            "RemoteError",
            "EBSByteBalance%: Forbidden",
        ),
        (completed(returncode=254, stderr="denied"), "Refused", "denied"),
    ],
)
def test_collect_ebs_failure_raises(
    tmp_path: Path, balance: Any, error: str, match: str
) -> None:
    runner = FakeRunner(states=[DONE_STATE], balance=balance)
    with pytest.raises(getattr(awsb, error), match=match):
        collect_ebs(runner, tmp_path)


def test_metric_min_counts_only_periods_overlapping_the_load(tmp_path: Path) -> None:
    path = tmp_path / "ec2.json"
    netbench.write_json(path, balance_file([50.0, 97.0, 99.0, 30.0], [100.0] * 4, 0.0))
    # Periods start at 0, 60, 120, 180; the load is 100..160: periods 60 and 120.
    low = awsb.metric_min(path, 100.0, 160.0)
    assert low == {"EBSByteBalance%": 97.0, "EBSIOBalance%": 100.0}


@pytest.mark.usefixtures("valid")
def test_finish_saves_node0_balance(run_harness: RunHarness) -> None:
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_OK
    assert runner.count("get-metric-data") == 1
    assert (run_harness.run_dir / awsb.EC2_NODE0_FILE).exists()


@pytest.mark.usefixtures("valid")
def test_balance_fetch_failure_does_not_stop_the_teardown(
    run_harness: RunHarness,
) -> None:
    runner = FakeRunner(
        states=[DONE_STATE],
        describe=DESCRIBE,
        balance=completed(returncode=254, stderr="denied"),
    )
    assert run_harness.run(runner) == awsb.EXIT_OK
    assert not (run_harness.run_dir / awsb.EC2_NODE0_FILE).exists()
    assert runner.ran("tofu", "destroy")


@pytest.mark.usefixtures("valid")
def test_no_load_window_fetches_no_balance(run_harness: RunHarness) -> None:
    error = {"phase": "error", "detail": "x", "error": "network not ready"}
    runner = FakeRunner(states=[error], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_FAILED
    assert runner.count("get-metric-data") == 0


def lagging_run(system: FakeSystem, tmp_path: Path) -> Any:
    fleet = awsb.FleetState(
        system,
        awsb.RunConfig(tag="x"),
        tmp_path / "fleet",
        None,
        awsb.Interrupts(system.clock),
    )
    agent = DONE_STATE | {"t1": system.clock.time()}
    return awsb.Run(fleet, tmp_path / "run", fleet.cfg, 0.0, agent=agent)


def test_balance_skipped_on_third_signal_during_the_lag(tmp_path: Path) -> None:
    runner = FakeRunner(states=[DONE_STATE])
    run = lagging_run(FakeSystem(run=runner), tmp_path)
    run.fleet.interrupts.skip_collect.set()
    awsb.collect_node0_ebs_balance(run)
    assert runner.count("get-metric-data") == 0


def test_published_window_waits_out_the_lag_on_the_clock(
    system: FakeSystem, clock: FakeClock, tmp_path: Path
) -> None:
    run = lagging_run(system, tmp_path)
    assert awsb.published_window(run, "x") == run.agent
    assert clock.sleeps == [awsb.CLOUDWATCH_LAG_S]


@pytest.fixture
def collected_run(tmp_path: Path) -> Path:
    write_collected_run(tmp_path)
    return tmp_path


def test_report_has_hosts_deployment_and_summary_blocks(collected_run: Path) -> None:
    result = awsb.write_report(collected_run)
    assert sorted(result["hosts"]) == ["ctl", "node0", "node1", "node2"]
    assert result["hosts"]["node0"]["disk_mb_s"] == pytest.approx(1.0)
    assert result["hosts"]["node0"]["net_mb_s"] == pytest.approx(1.0)
    deployment = result["deployment"]
    assert deployment["az"] == "eu-west-1b"
    assert deployment["clock_offset_ms_max"] == pytest.approx(0.01)
    assert deployment["cost_usd"] == {"expected": 1.0, "bound": 2.0}
    assert result["runner"]["cpu_model"] == "Neoverse-V2"
    saved = netbench.read_json(collected_run / "result.json")
    assert saved["deployment"] == deployment
    summary = (collected_run / "summary.md").read_text()
    assert "Deployment" in summary
    assert "| node1 |" in summary


def test_render_is_repeatable(collected_run: Path) -> None:
    assert awsb.write_report(collected_run) == awsb.write_report(collected_run)


def test_uncollected_host_makes_the_run_invalid(collected_run: Path) -> None:
    (collected_run / "hosts" / "node2" / "host.jsonl").unlink()
    result = awsb.write_report(collected_run)
    assert "node2" not in result["hosts"]
    assert not result["validity"]["valid"]
    reason = "node2 host samples cover only 0% of the window"
    assert reason in result["validity"]["reasons"]


def empty_io_balance() -> dict:
    data = balance_file([100.0, 100.0, 100.0], [], first=60.0)
    data["metrics"]["EBSIOBalance%"] = {"timestamps": [], "values": []}
    return data


# REQ:ebs-balance-evidence
# TEST:ebs-balance-noisy-ok
@pytest.mark.parametrize(
    ("balance", "reason"),
    [
        (
            balance_file([100.0, 91.0, 100.0], [100.0] * 3),
            "node0 EBSByteBalance% fell to 91% during the load",
        ),
        (balance_file([100.0, 100.0, 40.0], [100.0, 100.0, 40.0]), None),
        (empty_io_balance(), "node0 EBSIOBalance% has no datapoint in the load window"),
        (None, "node0 EBS balance not collected"),
    ],
    ids=["drop-inside-load", "drop-after-load", "no-datapoint", "not-collected"],
)
def test_ebs_balance_validity(
    collected_run: Path, balance: dict | None, reason: str | None
) -> None:
    path = collected_run / awsb.EC2_NODE0_FILE
    if balance is None:
        path.unlink()
    else:
        netbench.write_json(path, balance)
    validity = awsb.write_report(collected_run)["validity"]
    assert validity["valid"]
    assert validity["reasons"] == ([] if reason is None else [reason])
    assert validity["noisy"] == (reason is not None)
