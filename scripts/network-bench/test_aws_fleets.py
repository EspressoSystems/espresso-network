from datetime import UTC, datetime
from pathlib import Path

import pytest
from fakes import (
    EXPIRES_LATER,
    EXPIRES_PAST,
    STATUS_DESCRIBE,
    STS_CALL,
    FakeClock,
    FakeRunner,
    FakeSystem,
    FleetHarness,
    awsb,
    completed,
    netbench,
    run_cmd,
    tag_mapping,
    tag_runner,
    valid_result,
    write_fleet,
)


@pytest.fixture
def harness(isolated: Path, monkeypatch: pytest.MonkeyPatch) -> FleetHarness:
    return FleetHarness(monkeypatch, isolated)


@pytest.fixture
def runner(harness: FleetHarness) -> FakeRunner:
    return harness.up_fleet()


def down(harness: FleetHarness, runner: FakeRunner, *argv: str) -> FakeSystem:
    system = FakeSystem(run=runner, clock=FakeClock(), answer=True)
    assert awsb.cmd_down(harness.parse("down", *argv), system) == awsb.EXIT_OK
    return system


# TEST:run-fleet-bare-picks-ok
def test_run_fleet_without_a_dir_picks_the_only_fleet(harness, runner):
    args = harness.parse("run", "--fleet", "--yes")
    assert awsb.cmd_run(args, FakeSystem(run=runner)) == awsb.EXIT_OK
    assert (harness.fleet_dir / "runs" / "01-colocated" / "manifest.json").exists()
    assert "fleet fleet1 (idle)" in harness.driver_log()


def test_fleet_run_manifest_records_both_commands(harness, runner):
    args = harness.parse("run", "--fleet", "--yes")
    assert awsb.cmd_run(args, FakeSystem(run=runner)) == awsb.EXIT_OK
    manifest = netbench.read_json(harness.fleet_dir / "runs" / "01-colocated" / "manifest.json")
    assert manifest["argv"] == ["run", "--fleet"]
    assert manifest["fleet_argv"] == awsb.sanitize_argv(harness.fleet()["argv"])
    assert manifest["fleet_argv"][0] == "up"
    assert "--yes" not in manifest["fleet_argv"]


# TEST:down-no-dir-picks-ok
def test_down_without_a_dir_prompts_with_phase_and_expiry(harness, runner):
    system = down(harness, runner)
    assert runner.ran("tofu", "destroy")
    expires = harness.fleet()["expires_at"][:16]
    assert "fleet1" in system.prompts[0]
    assert "idle" in system.prompts[0]
    assert f"expires {expires}" in system.prompts[0]


# TEST:status-no-dir-picks-ok
def test_status_without_a_dir_shows_the_only_fleet(harness, runner, capsys):
    runner.describe = STATUS_DESCRIBE
    capsys.readouterr()
    awsb.cmd_status(harness.parse("status"), FakeSystem(run=runner))
    assert "- fleet fleet1: phase idle" in capsys.readouterr().out


# TEST:collect-no-dir-last-run-ok
def test_collect_without_a_dir_takes_the_last_run(harness, runner, monkeypatch):
    harness.run(runner)
    harness.run(runner)
    collected: list[Path] = []

    def fake_collect(remote, hosts, run_dir: Path, subdir: str) -> None:
        collected.append(run_dir)
        for host in hosts:
            out = run_dir / "hosts" / host["name"] / subdir
            out.mkdir(parents=True)
            (out / "df.txt").write_text("x")

    monkeypatch.setattr(awsb, "collect_hosts", fake_collect)
    code = awsb.cmd_collect(harness.parse("collect"), FakeSystem(run=runner))
    assert code == awsb.EXIT_OK
    assert collected == [harness.fleet_dir / "runs" / "02-colocated"]


# TEST:fleet-by-name-ok
def test_fleet_arg_resolves_a_bare_name_under_the_out_root(tmp_path: Path):
    fleet_dir = write_fleet(tmp_path, "f1", "idle")
    assert awsb.fleet_arg("f1", tmp_path) == fleet_dir
    assert awsb.fleet_arg(str(fleet_dir), Path("elsewhere")) == fleet_dir
    with pytest.raises(awsb.Refused, match="not a fleet dir"):
        awsb.fleet_arg("f2", tmp_path)
    with pytest.raises(awsb.Refused, match="empty fleet name"):
        awsb.fleet_arg("", tmp_path)


def test_run_fleet_empty_value_does_not_pick(harness, runner):
    args = harness.parse("run", "--fleet", "")
    with pytest.raises(awsb.Refused, match="empty fleet name"):
        awsb.cmd_run(args, FakeSystem(run=runner))


def test_collect_without_runs_names_the_fleet(harness, runner):
    with pytest.raises(awsb.Refused, match=f"{harness.fleet_dir.name} has no runs"):
        awsb.cmd_collect(harness.parse("collect"), FakeSystem(run=runner))


def test_down_by_name(harness, runner):
    down(harness, runner, "fleet1")
    assert harness.fleet()["phase"] == "done"


# TEST:down-implicit-yes-fails
def test_down_yes_without_a_fleet_is_refused(harness, runner):
    mark = len(runner.calls)
    with pytest.raises(awsb.Refused, match="--yes needs FLEET"):
        awsb.cmd_down(harness.parse("down", "--yes"), FakeSystem(run=runner))
    assert len(runner.calls) == mark


# TEST:select-zero-fails
def test_pick_refuses_without_a_selectable_fleet(tmp_path: Path):
    with pytest.raises(awsb.Refused, match="no selectable fleet"):
        awsb.pick_fleet(tmp_path / "missing")
    write_fleet(tmp_path, "old", "done")
    (tmp_path / "INDEX.md").write_text("")
    with pytest.raises(awsb.Refused, match=r"no selectable fleet.*old \(done"):
        awsb.pick_fleet(tmp_path)


# TEST:select-many-fails
def test_pick_refuses_several_and_lists_them(tmp_path: Path):
    write_fleet(tmp_path, "a", "idle")
    write_fleet(tmp_path, "b", "left-running", expires_at=EXPIRES_PAST)
    write_fleet(tmp_path, "c", "done")
    with pytest.raises(awsb.Refused) as refused:
        awsb.pick_fleet(tmp_path)
    message = str(refused.value)
    assert f"a (idle, expires {EXPIRES_LATER[:16]})" in message
    assert f"b (left-running, expires {EXPIRES_PAST[:16]})" in message
    assert "c (" not in message


# TEST:select-applying-only-fails
def test_pick_skips_a_fleet_that_is_still_coming_up(tmp_path: Path):
    write_fleet(tmp_path, "a", "applying")
    with pytest.raises(awsb.Refused, match="no selectable fleet"):
        awsb.pick_fleet(tmp_path)


@pytest.mark.parametrize(
    "phase", ["idle", "running", "dirty", "left-running", "destroying", "destroyed"]
)
def test_pick_takes_the_only_selectable_fleet(tmp_path: Path, phase: str):
    write_fleet(tmp_path, "old", "swept")
    fleet_dir = write_fleet(tmp_path, "a", phase, expires_at=EXPIRES_PAST)
    write_fleet(tmp_path, "new", "planned")
    (tmp_path / "INDEX.md").write_text("")
    (tmp_path / "stray").mkdir()
    assert awsb.pick_fleet(tmp_path) == fleet_dir


# TEST:run-fleet-expired-fails
def test_run_fleet_without_a_dir_refuses_an_expired_fleet(harness, runner):
    harness.set_fleet(expires_at=EXPIRES_PAST)
    with pytest.raises(awsb.Refused, match="expired"):
        awsb.cmd_run(harness.parse("run", "--fleet", "--yes"), FakeSystem(run=runner))


# TEST:down-expired-ok
def test_down_without_a_dir_takes_an_expired_fleet(harness, runner):
    harness.set_fleet(expires_at=EXPIRES_PAST)
    down(harness, runner)
    assert harness.fleet()["phase"] == "done"


# TEST:down-left-running-ok
def test_down_without_a_dir_takes_a_left_running_fleet(harness, runner):
    harness.set_fleet(phase="left-running")
    down(harness, runner)
    assert harness.fleet()["phase"] == "done"


LIST_NOW = datetime(2026, 9, 29, 16, 0, tzinfo=UTC)


def write_run(fleet_dir: Path, name: str, phase: str, result: dict | None = None):
    run_dir = fleet_dir / "runs" / name
    run_dir.mkdir(parents=True)
    netbench.write_json(run_dir / "manifest.json", {"phase": phase})
    if result is not None:
        netbench.write_json(run_dir / "result.json", result)


def list_lines(out_root: Path) -> list[str]:
    return awsb.format_fleets(awsb.fleet_rows(out_root), LIST_NOW)


# TEST:format-fleets-columns-ok
def test_list_shows_local_fleets_newest_first(tmp_path: Path):
    live = write_fleet(tmp_path, "live", "idle", created_at="2026-09-29T14:00:00Z")
    write_run(live, "01-colocated", "done", valid_result())
    write_run(live, "02-colocated", "done", valid_result(valid=False))
    done = write_fleet(tmp_path, "gone", "done", created_at="2026-09-29T13:00:00Z")
    fleet = netbench.read_json(done / "fleet.json")
    netbench.write_json(done / "fleet.json", {**fleet, "cost_usd": {"actual": 1.234}})
    write_fleet(
        tmp_path,
        "stale",
        "left-running",
        expires_at=EXPIRES_PAST,
        created_at="2026-09-29T15:00:00Z",
    )
    assert list_lines(tmp_path) == [
        "| fleet | phase | created | expires | left | cost | runs | last |",
        "|---|---|---|---|---|---|---|---|",
        (
            "| stale | left-running | 2026-09-29T15:00 | 2026-09-29T15:00 | expired "
            "| <=$2.50 | 0 | - |"
        ),
        (
            "| live | idle | 2026-09-29T14:00 | 2026-09-29T17:00 | 60m | <=$2.50 | 2 "
            "| invalid 8 MB/s |"
        ),
        "| gone | done | 2026-09-29T13:00 | 2026-09-29T17:00 | - | $1.23 | 0 | - |",
        "AWS view: aws-bench status --all",
    ]


# TEST:list-empty-ok
def test_list_without_a_state_dir(tmp_path: Path):
    assert list_lines(tmp_path / "missing") == [
        "no local fleets",
        "AWS view: aws-bench status --all",
    ]


# TEST:list-no-fleet-json-ok
def test_list_shows_dirs_without_fleet_json_last(tmp_path: Path):
    (tmp_path / "broken").mkdir()
    write_fleet(tmp_path, "a", "done")
    lines = list_lines(tmp_path)
    assert lines[2].startswith("| a | done |")
    assert lines[3] == "| broken | ? | ? | ? | ? | ? | ? | ? |"


# TEST:list-skips-index-ok
def test_list_skips_index_md(tmp_path: Path):
    write_fleet(tmp_path, "a", "done")
    (tmp_path / "INDEX.md").write_text("")
    assert len(list_lines(tmp_path)) == 4


# TEST:list-last-run-phase-ok
def test_list_shows_the_last_run_phase_without_a_result(tmp_path: Path):
    fleet_dir = write_fleet(tmp_path, "a", "running")
    write_run(fleet_dir, "01-colocated", "done", valid_result())
    write_run(fleet_dir, "02-colocated", "running")
    assert list_lines(tmp_path)[2].endswith("| 2 | running |")


def test_list_command_reads_the_state_dir(harness, runner, capsys):
    harness.run(runner)
    capsys.readouterr()
    mark = len(runner.calls)
    code = awsb.cmd_list(harness.parse("list"), FakeSystem(run=runner))
    assert code == awsb.EXIT_OK
    assert len(runner.calls) == mark
    out = capsys.readouterr().out
    assert "| fleet1 | idle |" in out
    assert "| 1 | valid 8 MB/s |" in out


OLD = "2026-09-01T00:00:00+00:00"
CUTOFF = datetime(2026, 9, 28, 16, 0, tzinfo=UTC)


YOUNG = "2026-09-29T14:00:00+00:00"
DIR = Path("a")


def manifest(phase: str, created_at: str = OLD, **config) -> dict:
    cfg = awsb.config_to_json(awsb.RunConfig(tag="t", **config))
    return {"phase": phase, "created_at": created_at, "config": cfg}


def prune_plan(fleet, tagged=(), locked=(), region="eu-west-1"):
    return awsb.prune_plan([(DIR, fleet)], set(tagged), set(locked), CUTOFF, region)


@pytest.mark.parametrize("phase", ["planned", "done", "swept"])
def test_prune_plan_deletes_old_dirs_in_a_safe_phase(phase: str):
    old = manifest(phase)
    assert prune_plan(old) == ([(DIR, old)], [])
    assert prune_plan(manifest(phase, YOUNG)) == ([], [])


# TEST:prune-live-kept-ok
@pytest.mark.parametrize(
    "phase", ["applying", "idle", "running", "dirty", "left-running", "destroyed"]
)
def test_prune_plan_keeps_an_old_fleet_in_another_phase(phase: str):
    assert prune_plan(manifest(phase)) == ([], [(DIR, f"phase {phase}")])


# TEST:prune-locked-kept-ok
def test_prune_plan_keeps_a_locked_fleet():
    assert prune_plan(manifest("done"), locked=("a",)) == ([], [(DIR, "locked")])


# TEST:prune-tagged-kept-ok
def test_prune_plan_keeps_a_fleet_with_tagged_resources():
    assert prune_plan(manifest("done"), tagged=("a",)) == (
        [],
        [(DIR, "tagged resources remain")],
    )


# TEST:prune-no-fleet-json-kept-ok
def test_prune_plan_keeps_a_dir_without_fleet_json():
    assert prune_plan(None) == ([], [(DIR, "no fleet.json")])


def prune(capsys, runner: FakeRunner, *flags: str, answer: bool = False):
    system = FakeSystem(run=runner, answer=answer)
    code, out = run_cmd(capsys, awsb.cmd_prune, ["prune", *flags], system)
    return code, out, system


def tagged_runner(*names: str) -> FakeRunner:
    return tag_runner(
        [tag_mapping("security-group", f"sg-{n}", n, "me", None) for n in names], []
    )


# TEST:prune-done-deleted-ok
# TEST:prune-index-kept-ok
def test_prune_deletes_safe_fleet_dirs_after_confirmation(isolated: Path, capsys):
    out_root = isolated / awsb.OUT_ROOT
    gone = write_fleet(out_root, "gone", "done", created_at=OLD)
    write_run(gone, "01-colocated", "done", valid_result())
    write_run(gone, "02-colocated", "done", valid_result())
    swept = write_fleet(out_root, "swept", "swept", created_at=OLD)
    live = write_fleet(out_root, "live", "idle", created_at=OLD)
    tagged = write_fleet(out_root, "tagged", "done", created_at=OLD)
    young = write_fleet(out_root, "young", "done")
    locked = write_fleet(out_root, "locked", "done", created_at=OLD)
    (locked / "fleet.lock").write_text("{}")
    (out_root / "INDEX.md").write_text("history")
    code, out, system = prune(
        capsys, tagged_runner("tagged"), "--older-than", "1", answer=True
    )
    assert code == awsb.EXIT_OK
    assert system.prompts == ["Delete 2 fleet dirs (2 runs, results included)?"]
    assert "| gone | done | 2026-09-01T00:00 | 2 | <=$2.50 |" in out
    assert "| live | phase idle |" in out
    assert "| tagged | tagged resources remain |" in out
    assert "| locked | locked |" in out
    assert not gone.exists() and not swept.exists()
    assert all(p.exists() for p in (live, tagged, young, locked))
    assert (out_root / "INDEX.md").read_text() == "history"


# TEST:prune-unconfirmed-fails
def test_prune_unconfirmed_deletes_nothing(isolated: Path, capsys):
    gone = write_fleet(isolated / awsb.OUT_ROOT, "gone", "done", created_at=OLD)
    with pytest.raises(awsb.Refused, match="not confirmed"):
        prune(capsys, tagged_runner(), "--older-than", "1")
    assert gone.exists()


def test_prune_yes_skips_the_prompt(isolated: Path, capsys):
    gone = write_fleet(isolated / awsb.OUT_ROOT, "gone", "done", created_at=OLD)
    _, _, system = prune(capsys, tagged_runner(), "--older-than", "1", "--yes")
    assert system.prompts == []
    assert not gone.exists()


def test_prune_with_nothing_to_delete_does_not_ask(isolated: Path, capsys):
    write_fleet(isolated / awsb.OUT_ROOT, "young", "done")
    code, out, system = prune(capsys, tagged_runner(), "--older-than", "1")
    assert code == awsb.EXIT_OK
    assert system.prompts == []
    assert "nothing to prune" in out


# TEST:prune-no-creds-fails
def test_prune_without_credentials_deletes_nothing(isolated: Path, capsys):
    gone = write_fleet(isolated / awsb.OUT_ROOT, "gone", "done", created_at=OLD)
    runner = FakeRunner({STS_CALL: completed(returncode=255, stderr="no creds")})
    with pytest.raises(awsb.Refused, match="no creds"):
        prune(capsys, runner, "--older-than", "1", "--yes")
    assert len(runner.calls) == 1
    assert gone.exists()


# TEST:prune-days-zero-fails
@pytest.mark.parametrize("days", ["0", "-1", "x"])
def test_prune_needs_at_least_one_day(days: str):
    assert awsb.parse_args(["prune", "--older-than", "1"]).older_than == 1
    with pytest.raises(SystemExit):
        awsb.parse_args(["prune", "--older-than", days])
    with pytest.raises(SystemExit):
        awsb.parse_args(["prune"])


def test_prune_plan_keeps_a_fleet_of_another_region():
    old = manifest("done", region="eu-central-1")
    assert prune_plan(old) == ([], [(DIR, "region eu-central-1")])
    assert prune_plan(old, region="eu-central-1") == ([(DIR, old)], [])


def test_prune_plan_takes_a_fleet_without_a_recorded_region_as_eu_west_1():
    old = manifest("done")
    del old["config"]["region"]
    assert prune_plan(old) == ([(DIR, old)], [])
