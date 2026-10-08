import json
import os
import subprocess
from pathlib import Path

import latency
import netbench
import pytest
from fakes import (
    DESCRIBE,
    DONE_STATE,
    FakeRunner,
    FakeSystem,
    FleetHarness,
    RunHarness,
    Scripted,
    awsb,
    completed,
    host_info,
    plan_args,
    remote,
    ssh_calls,
    write_collected_run,
)

NODE_PUBLIC_IPS = ("203.0.113.2", "203.0.113.3")
NODE1_PRIVATE_IP = "10.0.0.3"


def cross_rtt_ms() -> float:
    profile = latency.load_profile("decaf-2025")
    pair = ("eu-central-1", "ap-southeast-1")
    return profile.cross_ms[pair] / 2 + profile.cross_ms[pair[::-1]] / 2


def phases(run_dir: Path) -> list[str]:
    lines = (run_dir / "events.jsonl").read_text().splitlines()
    return [json.loads(line)["phase"] for line in lines]


def first(commands: list[str], needle: str) -> int:
    return next(i for i, c in enumerate(commands) if needle in c)


@pytest.fixture
def run_harness(isolated: Path, monkeypatch: pytest.MonkeyPatch) -> RunHarness:
    return RunHarness(monkeypatch, isolated)


@pytest.fixture
def harness(isolated: Path, monkeypatch: pytest.MonkeyPatch) -> FleetHarness:
    return FleetHarness(monkeypatch, isolated)


def shaping_runner(ms: float | None = None) -> FakeRunner:
    ms = cross_rtt_ms() if ms is None else ms
    return FakeRunner(
        states=[DONE_STATE], describe=DESCRIBE, pings={NODE1_PRIVATE_IP: ms}
    )


# TEST:flags-roundtrip-ok
def test_flags_round_trip_through_the_manifest_config():
    cfg = awsb.config_from_args(plan_args("--latency", "mainnet", "--no-intra-latency"))
    assert (cfg.latency, cfg.intra_latency) == ("mainnet", False)
    saved = json.loads(json.dumps(awsb.config_to_json(cfg)))
    assert awsb.config_from_manifest(saved) == cfg
    old = {k: v for k, v in saved.items() if k not in ("latency", "intra_latency")}
    restored = awsb.config_from_manifest(old)
    assert (restored.latency, restored.intra_latency) == ("off", True)


def test_up_defaults_the_latency_flags():
    args = awsb.parse_args(["up", "--tag", "x"])
    assert (args.latency, args.intra_latency) == ("off", True)
    with pytest.raises(SystemExit):
        awsb.parse_args(["up", "--tag", "x", "--latency", "mainnet"])


# TEST:flags-off-no-intra-fails
def test_no_intra_latency_needs_a_profile():
    with pytest.raises(awsb.Refused, match="--no-intra-latency"):
        awsb.config_from_args(plan_args("--no-intra-latency"))


def test_tcp_cc_and_mtu_need_a_profile_and_default_to_bbr_and_1500():
    for flag, value in (("--tcp-cc", "cubic"), ("--mtu", "9001")):
        with pytest.raises(awsb.Refused, match=flag):
            awsb.config_from_args(plan_args(flag, value))
    cfg = awsb.config_from_args(plan_args("--latency", "decaf-2025"))
    assert (cfg.tcp_cc, cfg.mtu) == ("bbr", 1500)
    meta = awsb.latency_meta(cfg, awsb.plan_hosts(cfg))
    assert meta["sysctls"]["net.ipv4.tcp_congestion_control"] == "bbr"
    assert meta["mtu"] == 1500
    flags = ("--latency", "decaf-2025", "--tcp-cc", "cubic", "--mtu", "9001")
    assert awsb.config_from_args(plan_args(*flags)).mtu == 9001


# TEST:hash-four-distinct-ok
def test_hash_differs_per_profile_and_matches_the_old_value_for_off():
    base = awsb.RunConfig(
        tag="x",
        nodes=2,
        load=netbench.BenchConfig(submit_nodes=1),
        consensus_storage="journal",
    )
    hosts = awsb.plan_hosts(base)
    images = {}
    variants = [
        base,
        awsb.RunConfig(**{**vars(base), "latency": "decaf-2025"}),
        awsb.RunConfig(**{**vars(base), "latency": "mainnet"}),
        awsb.RunConfig(
            **{**vars(base), "latency": "decaf-2025", "intra_latency": False}
        ),
    ]
    hashes = [
        awsb.run_config_hash(
            cfg,
            hosts,
            images,
            b"g",
            awsb.latency_meta(cfg, hosts) if cfg.latency != "off" else None,
        )
        for cfg in variants
    ]
    assert len(set(hashes)) == 4
    old = netbench.config_hash(
        base.load,
        [
            b"g",
            json.dumps({}).encode(),
            json.dumps(hosts, sort_keys=True).encode(),
            base.query_db.encode(),
        ],
    )
    assert hashes[0] == old


def test_hash_follows_the_matrix(monkeypatch: pytest.MonkeyPatch):
    cfg = awsb.RunConfig(
        tag="x",
        nodes=2,
        load=netbench.BenchConfig(submit_nodes=1),
        latency="decaf-2025",
    )
    hosts = awsb.plan_hosts(cfg)
    meta = awsb.latency_meta(cfg, hosts)
    other = {**meta, "matrix_sha256": "0" * 64}
    assert awsb.run_config_hash(cfg, hosts, {}, b"g", meta) != awsb.run_config_hash(
        cfg, hosts, {}, b"g", other
    )


def test_unreadable_profile_data_is_refused(monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setattr(latency, "MATRIX_CSV", Path("missing.csv"))
    load = netbench.BenchConfig(submit_nodes=1)
    cfg = awsb.RunConfig(tag="x", nodes=2, load=load, latency="decaf-2025")
    with pytest.raises(awsb.Refused, match="decaf-2025"):
        awsb.latency_meta(cfg, awsb.plan_hosts(cfg))


# TEST:run-shapes-nodes-ok
@pytest.mark.usefixtures("valid")
def test_run_shapes_node_hosts_before_the_services(run_harness: RunHarness):
    runner = shaping_runner()
    assert run_harness.run(runner, "--latency", "decaf-2025") == awsb.EXIT_OK
    commands = ssh_calls(runner)
    shaping = [c for c in commands if "tc qdisc add" in c]
    assert len(shaping) == 2
    assert all(any(ip in c for ip in NODE_PUBLIC_IPS) for c in shaping)
    assert not any("203.0.113.1" in c for c in commands if "tc " in c or "ping" in c)
    assert max(map(commands.index, shaping)) < first(commands, "docker start anvil")
    manifest = netbench.read_json(run_harness.run_dir / "manifest.json")
    (probe,) = manifest["latency"]["probes"]
    assert probe["tier"] == "cross"
    assert probe["ok"]
    assert "shaping" in phases(run_harness.run_dir)
    assert (
        "latency decaf-2025: eu-central-1 1, ap-southeast-1 1; intra 10 ms on"
        in run_harness.log()
    )
    assert f"probe node0->node1 cross: expected {cross_rtt_ms():.1f} ms" in (
        run_harness.log()
    )


@pytest.mark.usefixtures("valid")
def test_run_index_row_names_the_latency_profile(run_harness: RunHarness):
    assert run_harness.run(shaping_runner(), "--latency", "decaf-2025") == awsb.EXIT_OK
    row = netbench.read_json(run_harness.run_dir / "index-row.json")
    assert row["latency"] == "decaf-2025"


@pytest.mark.parametrize(
    ("config", "cell"),
    [
        ({"latency": "off", "intra_latency": True}, "off"),
        ({"latency": "decaf-2025", "intra_latency": True}, "decaf-2025"),
        ({"latency": "mainnet", "intra_latency": False}, "mainnet no-intra"),
        ({}, "off"),
    ],
)
def test_latency_cell(config: dict, cell: str):
    assert awsb.latency_cell(config) == cell


# TEST:run-off-no-tc-ok
@pytest.mark.usefixtures("valid")
def test_off_issues_no_tc_or_ping(run_harness: RunHarness):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_OK
    assert not runner.ran("tc qdisc")
    assert not runner.ran("ping -c")
    manifest = netbench.read_json(run_harness.run_dir / "manifest.json")
    assert "latency" not in manifest
    assert "shaping" not in phases(run_harness.run_dir)


# TEST:verify-mismatch-fails
def test_probe_mismatch_fails_before_the_nodes_start(run_harness: RunHarness):
    runner = shaping_runner(ms=3.0)
    assert run_harness.run(runner, "--latency", "decaf-2025") == awsb.EXIT_FAILED
    assert not runner.ran("docker start espresso-node")
    assert not runner.ran("docker start anvil")
    manifest = netbench.read_json(run_harness.run_dir / "manifest.json")
    (probe,) = manifest["latency"]["probes"]
    assert probe["ok"] is False
    assert probe["measured_ms"] == 3.0
    assert runner.ran("tofu", "destroy")
    assert "probe node0->node1 cross" in run_harness.log()


def test_ping_without_summary_is_recorded_and_fails(run_harness: RunHarness):
    runner = shaping_runner()
    runner.respond("ping -c", lambda _: completed(returncode=1, stdout="100% loss"))
    assert run_harness.run(runner, "--latency", "decaf-2025") == awsb.EXIT_FAILED
    manifest = netbench.read_json(run_harness.run_dir / "manifest.json")
    (probe,) = manifest["latency"]["probes"]
    assert probe["measured_ms"] is None
    assert probe["ok"] is False
    assert probe["rc"] == 1
    assert not runner.ran("docker start espresso-node")


# TEST:tc-failure-collects-ok
def test_tc_failure_collects_and_destroys(run_harness: RunHarness):
    runner = shaping_runner()
    runner.respond(
        "tc qdisc add",
        lambda argv: completed(
            returncode=1 if any(NODE_PUBLIC_IPS[1] in a for a in argv) else 0,
            stderr="boom",
        ),
    )
    assert run_harness.run(runner, "--latency", "decaf-2025") == awsb.EXIT_FAILED
    assert not runner.ran("docker start anvil")
    assert not runner.ran("agent-drive")
    assert runner.ran("tofu", "destroy")
    assert "boom" in run_harness.log()


# TEST:single-node-profile-ok
def test_single_node_shapes_nothing_but_records_the_block(isolated: Path):
    cfg = awsb.RunConfig(tag="x", nodes=1, latency="decaf-2025")
    ctl_and_node0 = awsb.plan_hosts(
        awsb.RunConfig(tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1))
    )[:2]
    meta = awsb.latency_meta(cfg, ctl_and_node0)
    assert meta["nodes"] == {"eu-central-1": 1}
    hosts = {
        n: host_info(n, r, i)
        for i, (n, r) in enumerate((("ctl", "ctl"), ("node0", "query")), 1)
    }
    runner = FakeRunner(states=[DONE_STATE])
    remote = awsb.Remote(
        runner, isolated.relative_to(Path.cwd()), Path("~/.ssh/id"), hosts
    )
    manifest = isolated / "manifest.json"
    netbench.write_json(manifest, {"phase": "shipping"})
    shaping = awsb.latency_shaping(cfg, 1)
    awsb.shape_nodes(remote, shaping)
    awsb.verify_shaping(remote, shaping, manifest)
    assert not runner.ran("ping -c")
    saved = netbench.read_json(manifest)["latency"]
    assert saved["probes"] == []
    assert saved["profile"] == "decaf-2025"


# TEST:reset-clears-ok
def test_reset_deletes_an_htb_root_only_when_present():
    script = awsb.reset_script("validator")
    assert latency.TC_IFACE in script
    assert latency.TC_CLEAR in script
    assert 'grep -q "htb 1:"' in script
    subprocess.run(["bash", "-n"], input=script, text=True, check=True)


# Stand-ins for the host commands shaping calls; settings live as files under $FAKE_HOST.
FAKE_COMMANDS = {
    "sysctl": """if [ "$1" = -n ]; then cat "$FAKE_HOST/$2"; exit; fi
printf '%s' "${3#*=}" > "$FAKE_HOST/${3%%=*}"
""",
    "ip": """case "$*" in
"-o -4 route show to default") echo "default via 10.0.0.1 dev eth0 proto dhcp" ;;
"link set dev eth0 mtu "*) printf '%s' "$6" > "$FAKE_HOST/mtu" ;;
"-o link show dev eth0") echo "2: eth0: <UP> mtu $(cat "$FAKE_HOST/mtu") qdisc mq" ;;
*) echo "unexpected: ip $*" >&2; exit 1 ;;
esac
""",
    "tc": "",
    "modprobe": "",
    "networkctl": """case "$*" in
"status eth0") echo "Network File: /run/systemd/network/10-netplan-eth0.network" ;;
reload) ;;
*) echo "unexpected: networkctl $*" >&2; exit 1 ;;
esac
""",
}
UBUNTU_DEFAULTS = {
    "net.ipv4.tcp_congestion_control": "cubic",
    "net.core.default_qdisc": "fq_codel",
    "net.ipv4.tcp_rmem": "4096\t131072\t6291456",
    "net.ipv4.tcp_wmem": "4096\t16384\t4194304",
    "net.ipv4.tcp_notsent_lowat": "4294967295",
    "net.ipv4.tcp_slow_start_after_idle": "1",
    "net.ipv4.tcp_mtu_probing": "0",
    "mtu": "9001",
}


def fake_host(tmp_path: Path) -> tuple[Path, dict[str, str]]:
    host, bin_dir = tmp_path / "host", tmp_path / "bin"
    host.mkdir()
    bin_dir.mkdir()
    for name, value in UBUNTU_DEFAULTS.items():
        (host / name).write_text(value)
    for name, body in FAKE_COMMANDS.items():
        (bin_dir / name).write_text("#!/usr/bin/env bash\n" + body)
        (bin_dir / name).chmod(0o755)
    env = {
        **os.environ,
        "PATH": f"{bin_dir}:{os.environ['PATH']}",
        "FAKE_HOST": str(host),
    }
    return host, env


def run_on(script: str, tmp_path: Path, env: dict[str, str]) -> None:
    script = (
        script.replace(latency.BASELINE, str(tmp_path / "baseline.sh"))
        .replace(latency.NETWORKD_DIR, str(tmp_path / "networkd"))
        .replace("/sys/class/net/$IFACE/mtu", "$FAKE_HOST/mtu")
    )
    subprocess.run(["bash", "-c", script], env=env, check=True)


def settings(host: Path) -> dict[str, str]:
    return {path.name: path.read_text() for path in host.iterdir()}


def test_reset_restores_the_tcp_settings_and_mtu_shaping_changed(tmp_path: Path):
    host, env = fake_host(tmp_path)
    shaping = latency.shaping("decaf-2025", 2, intra=False, tcp_cc="bbr", mtu=1500)
    shape = latency.tc_script(
        0, {0: "10.0.0.2", 1: "10.0.0.3"}, shaping.delays, shaping.tcp_cc, shaping.mtu
    )
    restore = f"{latency.TC_IFACE}\n{latency.RESTORE_BASELINE}\n"
    assert latency.RESTORE_BASELINE in awsb.reset_script("validator")
    for _ in range(2):
        run_on(shape, tmp_path, env)
        run_on(shape, tmp_path, env)
        assert settings(host)["mtu"] == "1500"
        assert settings(host)["net.ipv4.tcp_congestion_control"] == "bbr"
        run_on(restore, tmp_path, env)
        assert settings(host) == UBUNTU_DEFAULTS
        assert not list((tmp_path / "networkd").glob(f"*.d/{latency.MTU_DROPIN}"))
        assert not (tmp_path / "baseline.sh").exists()


def test_tc_script_is_valid_bash():
    shaping = latency.shaping("decaf-2025", 5, intra=True, tcp_cc="cubic", mtu=1500)
    peers = {i: f"10.0.0.{i + 10}" for i in range(5)}
    script = latency.tc_script(0, peers, shaping.delays, shaping.tcp_cc, shaping.mtu)
    subprocess.run(["bash", "-n"], input=script, text=True, check=True)


# TEST:ship-latency-ok
def test_ship_agents_sends_the_latency_module(isolated: Path):
    runner = FakeRunner(states=[DONE_STATE])
    remote = awsb.Remote(
        runner,
        isolated.relative_to(Path.cwd()),
        Path("~/.ssh/id"),
        {"ctl": host_info("ctl", "ctl", 1)},
    )
    awsb.ship_agents(remote)
    assert runner.ran("rsync", "latency.py", "netbench.py", "aws-bench")
    assert "latency.py" in awsb.RESET_KEEP


# TEST:report-bullet-ok
def test_report_carries_the_latency_block(tmp_path: Path):
    write_collected_run(tmp_path)
    cfg = awsb.RunConfig(tag="x", nodes=3, latency="decaf-2025")
    meta = awsb.latency_meta(cfg, awsb.plan_hosts(cfg))
    path = tmp_path / "manifest.json"
    netbench.write_json(path, {**netbench.read_json(path), "latency": meta})
    result = awsb.write_report(tmp_path)
    assert result["deployment"]["latency"]["matrix_sha256"] == meta["matrix_sha256"]
    saved = netbench.read_json(tmp_path / "result.json")
    assert saved["deployment"]["latency"]["profile"] == "decaf-2025"
    summary = (tmp_path / "summary.md").read_text()
    assert (
        f"- latency: decaf-2025, intra on, matrix {meta['matrix_sha256'][:8]}"
        in summary
    )


def test_report_without_the_block_has_no_bullet(tmp_path: Path):
    write_collected_run(tmp_path)
    result = awsb.write_report(tmp_path)
    assert "latency" not in result["deployment"]
    assert "- latency:" not in (tmp_path / "summary.md").read_text()


# TEST:fleet-run-latency-ok
def test_fleet_run_takes_the_latency_flags_and_a_later_reset_clears_them(
    harness: FleetHarness,
):
    runner = harness.up_fleet()
    runner.pings = {NODE1_PRIVATE_IP: cross_rtt_ms()}
    mark = len(runner.calls)
    assert harness.run(runner, "--latency", "decaf-2025") == awsb.EXIT_OK
    commands = ssh_calls(runner, mark)
    assert sum("tc qdisc add" in c for c in commands) == 2
    assert first(commands, latency.TC_CLEAR) < first(commands, "tc qdisc add")
    manifest = netbench.read_json(
        harness.fleet_dir / "runs" / "01-colocated" / "manifest.json"
    )
    assert manifest["latency"]["profile"] == "decaf-2025"
    mark = len(runner.calls)
    assert harness.run(runner) == awsb.EXIT_OK
    assert not any("tc qdisc add" in c for c in ssh_calls(runner, mark))


def test_fleet_run_refuses_no_intra_latency_without_a_profile(harness: FleetHarness):
    runner = harness.up_fleet()
    with pytest.raises(awsb.Refused, match="--no-intra-latency"):
        harness.run(runner, "--no-intra-latency")


def capacity(query: tuple[float | None, bool], consensus: tuple[float | None, bool]):
    return {
        "overall": {"mb_s": None, "bounded": True},
        "consensus": {"mb_s": consensus[0], "bounded": consensus[1]},
        "query_node": {"mb_s": query[0], "bounded": query[1]},
        "failed_at_mb_s": None,
        "fail_rule": None,
    }


@pytest.mark.parametrize(
    ("limits_", "bound"),
    [
        (capacity((180, False), (140, True)), "consensus"),
        (capacity((100, True), (140, False)), "query"),
        (capacity((100, True), (140, True)), "query"),
        (capacity((140, True), (100, True)), "consensus"),
        (capacity((140, True), (140, True)), "both"),
        (capacity((None, True), (140, True)), "query"),
        (capacity((140, True), (None, True)), "consensus"),
        (capacity((None, True), (None, True)), "both"),
        (capacity((180, False), (180, False)), "-"),
    ],
)
def test_capacity_bound(limits_: dict, bound: str):
    assert awsb.capacity_bound(limits_) == bound


def test_render_refreshes_the_index_row(tmp_path: Path):
    write_collected_run(tmp_path)
    manifest_path = tmp_path / awsb.MANIFEST_JSON
    netbench.write_json(
        manifest_path,
        {
            **netbench.read_json(manifest_path),
            "created_at": "2026-10-05T10:00:00+00:00",
            "git_rev": "abcdef0123456789",
        },
    )
    netbench.write_json(
        tmp_path / awsb.INDEX_ROW_JSON,
        {**{key: "-" for key in awsb.INDEX_COLUMNS}, "exit": "1", "user": "lulu"},
    )
    result = awsb.write_report(tmp_path)
    awsb.refresh_index_row(tmp_path, result)
    row = netbench.read_json(tmp_path / awsb.INDEX_ROW_JSON)
    assert row["rate"] == netbench.fmt_num(result["capacity"]["overall"]["mb_s"])
    assert (row["exit"], row["user"], row["cost"]) == ("1", "lulu", "-")
    assert list(row) == [*awsb.INDEX_COLUMNS, "user"]


def test_render_refreshes_the_index_row_of_a_run_without_consensus_storage(
    tmp_path: Path,
):
    write_collected_run(tmp_path)
    manifest_path = tmp_path / awsb.MANIFEST_JSON
    manifest = netbench.read_json(manifest_path)
    manifest["config"].pop("consensus_storage")
    manifest |= {"created_at": "2026-10-05T10:00:00+00:00", "git_rev": "abcdef0123"}
    netbench.write_json(manifest_path, manifest)
    netbench.write_json(
        tmp_path / awsb.INDEX_ROW_JSON,
        {**{key: "-" for key in awsb.INDEX_COLUMNS}, "exit": "1", "user": "lulu"},
    )
    awsb.refresh_index_row(tmp_path, awsb.write_report(tmp_path))
    assert netbench.read_json(tmp_path / awsb.INDEX_ROW_JSON)["storage"] == "journal"


def test_latency_line_before_host_tuning():
    meta: netbench.LatencyMeta = {
        "profile": "decaf-2025",
        "intra": True,
        "nodes": {},
        "assignment": {},
        "matrix_sha256": "ab" * 32,
        "probes": [],
    }
    assert (
        netbench.latency_line(meta)
        == "- latency: decaf-2025, intra on, matrix abababab"
    )


def test_publish_does_not_sign(tmp_path: Path):
    runner = FakeRunner()
    runner.respond("commit", lambda argv: completed())
    awsb.git_run(FakeSystem(run=runner), tmp_path, "commit", "-m", "x")
    assert runner.ran("commit.gpgsign=false", "commit -m x")


def test_mtu_mismatches_name_each_node_whose_interface_differs(isolated: Path):
    runner = Scripted(
        {"/sys/class/net/": [completed(stdout="9001\n"), completed(stdout="1500\n")]}
    )
    found = awsb.mtu_mismatches(remote(runner, isolated), ["node0", "node1"], 1500)
    assert found == ["node0: mtu 9001, expected 1500"]
