from datetime import UTC, datetime
from pathlib import Path

import pytest
from fakes import (
    EXPIRES_LATER,
    EXPIRES_PAST,
    STATUS_DESCRIBE,
    FakeClock,
    FakeRunner,
    FakeSystem,
    FleetHarness,
    awsb,
    netbench,
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
