import ast
import contextlib
import json
import os
import re
from collections.abc import Callable
from pathlib import Path

import netbench
import pytest
from fakes import (
    DESCRIBE,
    DONE_STATE,
    EXPIRES_LATER,
    EXPIRES_PAST,
    NOW,
    SCRIPT,
    STATUS_DESCRIBE,
    STS_CALL,
    FakeClock,
    FakeRunner,
    FakeSystem,
    RunHarness,
    awsb,
    completed,
    instance,
    run_cmd,
    shot_estimate,
    sts_response,
    tag_mapping,
    tag_runner,
    valid_result,
    write_collected_run,
)

TERMINATE = ("aws", "--profile", "timeboost-dev", "ec2", "terminate-instances")
DELETE_VOLUME = ("aws", "--profile", "timeboost-dev", "ec2", "delete-volume")
EMPTY_REGION = "no espresso-bench resources in eu-west-1\n"

pytestmark = pytest.mark.usefixtures("isolated")


@pytest.mark.parametrize(
    ("phase", "owner", "expires", "orphan"),
    [
        ("measuring", "me", EXPIRES_LATER, None),
        ("left-running", "me", EXPIRES_LATER, None),
        ("measuring", "me", EXPIRES_PAST, "past expiry"),
        (None, "bob", EXPIRES_PAST, "past expiry"),
        ("done", "me", EXPIRES_LATER, "run is done"),
        ("swept", "me", EXPIRES_LATER, "run is swept"),
        (None, "me", EXPIRES_LATER, "no local state"),
        (None, "bob", EXPIRES_LATER, None),
        ("measuring", "me", None, None),
    ],
)
def test_classify_orphan(phase, owner, expires, orphan):
    tagged = {"name": "r", "owner": owner, "expires": expires}
    assert awsb.classify_orphan(tagged, phase, NOW, "me") == orphan


def test_terminal_phase_needs_terraform_state(tmp_path: Path):
    (tmp_path / "r" / "terraform").mkdir(parents=True)
    netbench.write_json(tmp_path / "r" / "fleet.json", {"phase": "done"})
    assert awsb.local_phase(tmp_path, "r") is None
    (tmp_path / "r" / "terraform" / "terraform.tfstate").write_text("{}")
    assert awsb.local_phase(tmp_path, "r") == "done"


def test_accrued_usd_is_instance_hours_at_the_price():
    two = [instance("i-1"), instance("i-2", launch="2026-09-29T15:00:00+00:00")]
    expected = 1.5 * awsb.PRICES["c8g.4xlarge"]
    assert awsb.accrued_usd(two, NOW) == pytest.approx(expected)


def test_accrued_usd_of_an_unpriced_type_raises():
    with pytest.raises(KeyError):
        awsb.accrued_usd([{**instance("i-1"), "type": "t4g.nano"}], NOW)


@pytest.mark.parametrize(
    ("mappings", "instances", "stale"),
    [
        ([], [], []),
        (
            [
                tag_mapping("instance", "i-1", "done", "bob", EXPIRES_LATER),
                tag_mapping("volume", "vol-1", "done", None, None),
                tag_mapping("volume", "vol-2", "done", None, None),
            ],
            [instance("i-1", "terminated")],
            ["vol-1", "vol-2"],
        ),
    ],
    ids=["empty", "stale_volumes_of_a_finished_fleet"],
)
def test_status_all_shows_nothing(mappings, instances, stale):
    runner = tag_runner(mappings, instances, stale_volumes=stale)
    code, out = run_cmd(awsb.cmd_status, ["status", "--all"], FakeSystem(run=runner))
    assert code == awsb.EXIT_OK
    assert out == EMPTY_REGION


def test_status_all_marks_orphans():
    mappings = [
        tag_mapping("instance", "i-1", "amy", "bob", EXPIRES_PAST),
        tag_mapping("instance", "i-2", "ok", "bob", EXPIRES_LATER),
    ]
    runner = tag_runner(mappings, [instance("i-1"), instance("i-2")])
    code, out = run_cmd(awsb.cmd_status, ["status", "--all"], FakeSystem(run=runner))
    assert code == awsb.EXIT_OK
    amy, ok = out.splitlines()[2:4]
    assert "| amy | bob |" in amy and amy.endswith("| past expiry |")
    assert ok.endswith("| 0.34 |  |")


def test_status_all_refuses_a_wrong_account_before_listing():
    runner = FakeRunner({STS_CALL: sts_response("999")})
    with pytest.raises(awsb.Refused):
        run_cmd(awsb.cmd_status, ["status", "--all"], FakeSystem(run=runner))
    assert len(runner.calls) == 1


def _orphans_runner() -> FakeRunner:
    mappings = [
        tag_mapping("instance", "i-1", "amy", "bob", EXPIRES_PAST),
        tag_mapping("volume", "vol-1", "amy", None, None),
        tag_mapping("security-group", "sg-1", "amy", "bob", EXPIRES_PAST),
        tag_mapping("key-pair", "key-1", "amy", "bob", EXPIRES_PAST),
        tag_mapping("instance", "i-2", "live", "bob", EXPIRES_LATER),
    ]
    return tag_runner(mappings, [instance("i-1"), instance("i-2")])


def _destroy_orphans(runner: FakeRunner, *flags: str) -> tuple[int, str]:
    argv = ["destroy", "--orphans", *flags]
    return run_cmd(awsb.cmd_destroy, argv, FakeSystem(run=runner))


def test_destroy_orphans_sweeps_only_orphans_in_dependency_order():
    runner = _orphans_runner()
    code, out = _destroy_orphans(runner, "--yes")
    assert code == awsb.EXIT_OK
    assert "| amy | bob |" in out
    assert "| live |" not in out
    ec2 = [c[4] for c in runner.calls if c[3:4] == ["ec2"]]
    order = (
        "terminate-instances wait delete-volume delete-security-group delete-key-pair"
    )
    assert [c for c in ec2 if not c.startswith("describe-")] == order.split()
    terminate = next(c for c in runner.calls if "terminate-instances" in c)
    assert "i-2" not in terminate


def test_destroy_orphans_declined_deletes_nothing():
    runner = _orphans_runner()
    with pytest.raises(awsb.Refused, match="not confirmed"):
        _destroy_orphans(runner)
    assert not runner.ran("terminate-instances")


def test_destroy_orphans_failed_sweep_exits_4():
    runner = _orphans_runner()
    runner.responses = {
        TERMINATE: completed(returncode=1, stderr="denied"),
        **runner.responses,
    }
    code, _ = _destroy_orphans(runner, "--yes")
    assert code == awsb.EXIT_LEFTOVER


def test_destroy_orphans_marks_local_state_swept(isolated: Path):
    fleet_dir = isolated / awsb.OUT_ROOT / "amy"
    fleet_dir.mkdir(parents=True)
    netbench.write_json(fleet_dir / "fleet.json", {"phase": "left-running"})
    key = fleet_dir / "ssh" / "id_ed25519"
    key.parent.mkdir()
    key.write_text("private")
    _destroy_orphans(_orphans_runner(), "--yes")
    assert not key.exists()
    assert json.loads((fleet_dir / "fleet.json").read_text())["phase"] == "swept"


@pytest.mark.parametrize(
    ("stderr", "expect"),
    [
        ("InvalidVolume.NotFound: gone", contextlib.nullcontext()),
        ("VolumeInUse", pytest.raises(awsb.Refused, match="VolumeInUse")),
    ],
)
def test_sweep_tolerates_only_an_already_deleted_volume(stderr, expect):
    runner = tag_runner([tag_mapping("volume", "vol-1", "r", None, None)], [])
    runner.responses = {
        DELETE_VOLUME: completed(returncode=255, stderr=stderr),
        **runner.responses,
    }
    with expect:
        awsb.sweep(FakeSystem(run=runner), "r")


def test_manifest_cost_is_linear_and_covers_instance_hours():
    cfg = awsb.RunConfig(tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1))
    hosts = awsb.plan_hosts(cfg)
    manifest = {"hosts": hosts, "estimate": shot_estimate(hosts, cfg)}
    cost = [awsb.manifest_cost(manifest, s) for s in (0, 3600, 7200)]
    hour = cost[1] - cost[0]
    assert cost[2] - cost[1] == pytest.approx(hour)
    assert hour >= awsb.PRICES["c8g.2xlarge"] + 2 * awsb.PRICES["c8g.4xlarge"]


def test_append_index_keeps_existing_rows(tmp_path: Path):
    manifest = {
        "name": "run1",
        "created_at": "2026-09-29T15:00:00+00:00",
        "git_rev": "a" * 40,
        "config": {"tag": "release-x", "nodes": 5},
        "fleet": "run1",
        "query_db": "colocated",
        "images": {"espresso-node": {"revision": "bd2ad6e1dc7abc"}},
    }
    path = tmp_path / "INDEX.md"
    path.write_text(awsb.INDEX_HEADER + "| older | row |\n")
    awsb.append_index(tmp_path, awsb.index_row(manifest, "01-run", None, 4, None))
    awsb.append_index(
        tmp_path, awsb.index_row(manifest, "02-run", None, 0, {"usd": 1.0})
    )
    lines = path.read_text().splitlines()
    assert len(lines) == 5
    assert lines[2] == "| older | row |"
    assert sum(line.startswith("| fleet/run |") for line in lines) == 1
    assert lines[3].endswith("| failed | 4 | - |")
    assert lines[4].endswith("| failed | 0 | 1.00 |")


@pytest.fixture
def kept(run_harness: RunHarness, monkeypatch: pytest.MonkeyPatch) -> FakeRunner:
    """A run whose destroy failed, left running with a working destroy for later verbs."""
    runner = FakeRunner(
        states=[DONE_STATE],
        describe=STATUS_DESCRIBE,
        destroys=[completed(returncode=1, stderr="locked")],
    )
    with monkeypatch.context() as m:
        m.setattr(awsb, "write_report", lambda *_: valid_result())
        assert run_harness.run(runner) == awsb.EXIT_LEFTOVER
    runner.destroys = [completed()]
    (run_harness.fleet_dir / "terraform").mkdir(exist_ok=True)
    return runner


def _verb(run_harness: RunHarness, verb: str, *extra: str):
    target = run_harness.run_dir if verb == "collect" else run_harness.fleet_dir
    return awsb.parse_args([verb, str(target), *extra])


def _status(run_harness: RunHarness, runner: FakeRunner) -> tuple[int, str]:
    argv = ["status", str(run_harness.fleet_dir)]
    return run_cmd(awsb.cmd_status, argv, FakeSystem(run=runner))


def _down(run_harness: RunHarness, runner: FakeRunner, *extra: str) -> int:
    system = FakeSystem(run=runner, clock=FakeClock())
    return awsb.cmd_down(_verb(run_harness, "down", *extra), system)


def test_status_shows_phase_instances_agent_and_cost(run_harness, kept):
    code, text = _status(run_harness, kept)
    assert code == awsb.EXIT_OK
    assert "- fleet run1: phase left-running" in text
    assert "- 01-run: phase left-running" in text
    assert "- ctl: running, c8g.4xlarge, launched 2026-09-29T15:00" in text
    assert "- agent: done, load finished" in text
    assert re.search(r"- cost: \$\d+\.\d\d so far, bound \$\d+\.\d\d", text)


def test_status_of_a_finished_run_reads_cost_json_without_aws(run_harness, monkeypatch):
    monkeypatch.setattr(awsb, "write_report", lambda *_: valid_result())
    assert run_harness.run(FakeRunner(states=[DONE_STATE], describe=DESCRIBE)) == 0
    offline = FakeRunner(states=[DONE_STATE])
    code, text = _status(run_harness, offline)
    assert code == awsb.EXIT_OK
    assert offline.calls == []
    assert re.search(r"- cost: \$\d+\.\d\d actual, bound \$\d+", text)


def test_down_destroys_prices_appends_index_and_refuses_a_second(run_harness, kept):
    kept.describe = DESCRIBE
    assert _down(run_harness, kept, "--yes") == awsb.EXIT_OK
    assert kept.ran("tofu", "destroy")
    manifest = json.loads((run_harness.fleet_dir / "fleet.json").read_text())
    assert manifest["phase"] == "done"
    assert manifest["cost_usd"]["actual"] > 0
    run_manifest = json.loads((run_harness.run_dir / "manifest.json").read_text())
    assert run_manifest["cost_usd"] == manifest["cost_usd"]
    rows = run_harness.index().splitlines()
    assert len(rows) == 3
    assert "| valid | 4 |" in rows[2]
    with pytest.raises(awsb.Refused, match="already done"):
        _down(run_harness, kept, "--yes")


def test_down_removes_the_private_key(run_harness, kept):
    key = run_harness.fleet_dir / "ssh" / "id_ed25519"
    _status(run_harness, kept)
    assert kept.ran("-i", str(key))
    kept.describe = DESCRIBE
    _down(run_harness, kept, "--yes")
    assert not key.exists()
    assert key.with_name("id_ed25519.pub").exists()


def test_down_declined_destroys_nothing(run_harness, kept):
    destroys = kept.count("tofu", "destroy")
    with pytest.raises(awsb.Refused, match="not confirmed"):
        _down(run_harness, kept)
    assert kept.count("tofu", "destroy") == destroys


def test_failed_down_sweeps_and_exits_4(run_harness, kept):
    kept.destroys = [completed(returncode=1, stderr="locked")]
    assert _down(run_harness, kept, "--yes") == awsb.EXIT_LEFTOVER
    assert kept.ran("terminate-instances", "i-1")
    manifest = json.loads((run_harness.fleet_dir / "fleet.json").read_text())
    assert manifest["phase"] == "left-running"


def _copy_df(remote, host, source, local: Path, *flags) -> None:
    local.mkdir(parents=True, exist_ok=True)
    (local / "df.txt").write_text("x")


def test_collect_writes_numbered_subdirs(run_harness, kept, monkeypatch):
    monkeypatch.setattr(awsb.Remote, "rsync_from", _copy_df)
    for index in (1, 2):
        code = awsb.cmd_collect(_verb(run_harness, "collect"), FakeSystem(run=kept))
        assert code == awsb.EXIT_OK
        host = run_harness.run_dir / "hosts" / "node0"
        assert (host / f"collect-{index}" / "df.txt").exists()
    assert kept.ran("psql -At -c")


def _copies_nothing(m: pytest.MonkeyPatch) -> None:
    m.setattr(awsb.Remote, "rsync_from", lambda *_: None)


def _script_fails(m: pytest.MonkeyPatch) -> None:
    _copies_nothing(m)
    real_ssh = awsb.Remote.ssh

    def failing(self, host, command, check=True):
        if "docker logs" in command:
            raise awsb.RemoteError(f"{host}: exited 255")
        return real_ssh(self, host, command, check)

    m.setattr(awsb.Remote, "ssh", failing)


@pytest.mark.parametrize("breaks", [_copies_nothing, _script_fails])
def test_collect_fails_for_a_host_that_yielded_nothing(
    run_harness, kept, monkeypatch, breaks: Callable[[pytest.MonkeyPatch], None]
):
    breaks(monkeypatch)
    code = awsb.cmd_collect(_verb(run_harness, "collect"), FakeSystem(run=kept))
    assert code == awsb.EXIT_FAILED


def test_collect_of_an_older_run_is_refused_without_calls(run_harness, kept):
    (run_harness.fleet_dir / "runs" / "02-later").mkdir()
    mark = len(kept.calls)
    with pytest.raises(awsb.Refused, match="not the last run"):
        awsb.cmd_collect(_verb(run_harness, "collect"), FakeSystem(run=kept))
    assert len(kept.calls) == mark


def test_collect_while_a_run_is_in_progress_is_refused(run_harness, kept):
    fleet_json = run_harness.fleet_dir / "fleet.json"
    manifest = json.loads(fleet_json.read_text())
    netbench.write_json(fleet_json, {**manifest, "phase": "running"})
    with pytest.raises(awsb.Refused, match="is running"):
        awsb.cmd_collect(_verb(run_harness, "collect"), FakeSystem(run=kept))


def test_render_rewrites_summary_and_compares_a_baseline(tmp_path: Path):
    write_collected_run(tmp_path)
    (tmp_path / "summary.md").write_text("stale")
    code = awsb.cmd_render(awsb.parse_args(["render", str(tmp_path)]), FakeSystem())
    assert code in (awsb.EXIT_OK, awsb.EXIT_INVALID)
    assert "Deployment" in (tmp_path / "summary.md").read_text()
    result = json.loads((tmp_path / "result.json").read_text())
    assert (code == awsb.EXIT_OK) == result["validity"]["valid"]
    baseline = tmp_path / "baseline.json"
    baseline.write_text((tmp_path / "result.json").read_text())
    argv = ["render", str(tmp_path), "--baseline", str(baseline)]
    awsb.cmd_render(awsb.parse_args(argv), FakeSystem())
    assert "baseline" in (tmp_path / "summary.md").read_text().lower()


def test_config_round_trips_through_the_manifest():
    cfg = awsb.RunConfig(
        tag="x", nodes=3, load=netbench.BenchConfig(submit_nodes=2), node_env=("A=1",)
    )
    saved = json.loads(json.dumps(awsb.config_to_json(cfg)))
    assert awsb.config_from_manifest(saved) == cfg
    del saved["node_env"]
    assert awsb.config_from_manifest(saved).node_env == ()


# TEST:system-cmd-requires-system-fails
@pytest.mark.parametrize(
    ("cmd", "argv"),
    [("cmd_render", ["render", "run"]), ("cmd_status", ["status", "--all"])],
)
def test_command_without_system_fails(cmd: str, argv: list[str]):
    with pytest.raises(TypeError):
        getattr(awsb, cmd)(awsb.parse_args(argv))


def test_pid_alive_for_this_process_and_not_for_an_impossible_pid():
    assert awsb._pid_alive(os.getpid())
    assert not awsb._pid_alive(2**22 + 1)


BOUNDARY = {"_run", "_ask", "_http_get", "_trap", "_pid_alive", "host_system", "stamp"}
EFFECTS = """
subprocess.run subprocess.Popen subprocess.check_output shutil.which sys.stdin input
datetime.now time.time time.sleep time.monotonic signal.signal getpass.getuser
socket.gethostname os.getpid os.kill urllib.request.urlopen nb.SYSTEM_CLOCK nb.HttpPool
"""
SIDE_EFFECTS = set(EFFECTS.split())


def side_effect_uses(tree: ast.AST) -> list[tuple[str, str]]:
    """(enclosing top-level function, side effect) for every use in `tree`."""
    found = []
    for top in ast.iter_child_nodes(tree):
        owner = top.name if isinstance(top, ast.FunctionDef) else "<module>"
        for node in ast.walk(top):
            if isinstance(node, ast.Attribute):
                name = ast.unparse(node)
            elif isinstance(node, ast.Name):
                name = node.id
            else:
                continue
            if name in SIDE_EFFECTS:
                found.append((owner, name))
    return found


def test_side_effects_only_in_boundary_helpers():
    tree = ast.parse(SCRIPT.read_text())
    assert [u for u in side_effect_uses(tree) if u[0] not in BOUNDARY] == []


def test_scan_flags_a_stray_call():
    tree = ast.parse("def f():\n    return time.time() + len(input())\n")
    assert side_effect_uses(tree) == [("f", "time.time"), ("f", "input")]
