import argparse
import json
import re
import shlex
import signal
import subprocess
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any

import netbench
import pytest
from fakes import (
    BY_ID,
    DESCRIBE,
    DONE_STATE,
    NOW,
    STATUS_DESCRIBE,
    VOLUME_ID,
    FakeRunner,
    FakeSystem,
    FleetHarness,
    awsb,
    completed,
    fake_image,
    ssh_calls,
    valid_result,
    volume_runner,
)

NEW_DIGEST = f"sha256:{'1' * 64}"
NODE0_IP = "203.0.113.2"
PG_VOLUME = {"gb": 400, "iops": 12000, "mbps": 500}


@pytest.fixture
def harness(isolated: Path, monkeypatch: pytest.MonkeyPatch) -> FleetHarness:
    return FleetHarness(monkeypatch, isolated)


@pytest.fixture
def runner(harness: FleetHarness) -> FakeRunner:
    return harness.up_fleet()


def run_manifest(harness: FleetHarness, name: str = "01-colocated") -> dict:
    return netbench.read_json(harness.fleet_dir / "runs" / name / "manifest.json")


def first(commands: list[str], needle: str) -> int:
    return next(i for i, c in enumerate(commands) if needle in c)


def assert_idle(harness: FleetHarness) -> None:
    assert harness.fleet()["phase"] == "idle"
    assert not harness.lock().exists()


# REQ:fleet-up-idle
def test_up_ends_idle_without_starting_anything(harness):
    runner = FakeRunner(states=[DONE_STATE])
    assert harness.up(runner) == awsb.EXIT_OK
    assert harness.fleet()["db_modes"] == ["colocated"]
    for name in ("hosts.json", "terraform/plan.txt", "hosts/ctl/ready.json"):
        assert (harness.fleet_dir / name).exists(), name
    assert (harness.fleet_dir / "ssh").is_dir()
    assert not (harness.fleet_dir / "runs").exists()
    assert not any("docker start" in c or "start.sh" in c for c in ssh_calls(runner))
    assert not runner.ran("agent-drive")
    assert not runner.ran("tofu", "destroy")
    assert_idle(harness)


# REQ:state-dir-path
def test_up_keeps_fleet_state_in_bench_state(harness, isolated):
    assert harness.up(FakeRunner(states=[DONE_STATE])) == awsb.EXIT_OK
    assert (isolated / "bench-state" / "aws" / "fleet1" / "fleet.json").is_file()


def test_up_logs_the_cost_and_the_run_command(harness):
    harness.up(FakeRunner(states=[DONE_STATE]))
    log = harness.driver_log()
    assert re.search(
        r"cost: fleet \$\d+\.\d\d/h, bound \$\d+\.\d\d at TTL 180 min "
        r"\(self-terminate \d\d:\d\d UTC\), limit \$60\.00",
        log,
    )
    assert f"idle: fleet {harness.fleet_dir}; " in log
    assert f"run: aws-bench run --fleet {harness.fleet_dir} --query-db colocated" in log


def test_fleet_bound_is_the_rate_over_the_ttl_plus_boot(harness):
    harness.up(FakeRunner(states=[DONE_STATE]))
    estimate = harness.fleet()["estimate"]
    assert estimate["ttl_s"] == 180 * 60
    rate = awsb.estimate_rate(estimate)
    assert rate > awsb.PRICES["c8g.2xlarge"] + 2 * awsb.PRICES["c8g.4xlarge"]
    egress = next(l["usd"] for l in estimate["lines"] if l["item"] == "egress")
    expected = rate * (180 * 60 + awsb.BOOT_ALLOWANCE_S) / 3600 + egress
    assert estimate["bound_usd"] == pytest.approx(expected)


def test_expiry_counts_from_the_confirm_without_a_provision_margin(harness):
    harness.up(FakeRunner(states=[DONE_STATE]))
    expires_at = harness.fleet()["expires_at"]
    assert datetime.fromisoformat(expires_at) == NOW + timedelta(minutes=180)
    tfvars = harness.fleet_dir / "terraform" / "terraform.tfvars.json"
    assert netbench.read_json(tfvars)["expires_at"] == expires_at
    user_data = harness.fleet_dir / "hosts" / "ctl" / "user-data.sh"
    assert "shutdown -P +180" in user_data.read_text()


def test_apply_failure_destroys_and_exits_3(harness):
    runner = FakeRunner(
        states=[DONE_STATE],
        apply=completed(returncode=1, stderr="Error: InsufficientInstanceCapacity"),
    )
    assert harness.up(runner) == awsb.EXIT_FAILED
    assert runner.ran("tofu", "destroy")
    assert "InsufficientInstanceCapacity" in harness.driver_log()


def test_apply_failure_with_a_failed_destroy_exits_4(harness):
    runner = FakeRunner(
        states=[DONE_STATE],
        apply=completed(returncode=1, stderr="Error: boom"),
        destroys=[completed(returncode=1, stderr="locked")],
    )
    assert harness.up(runner) == awsb.EXIT_LEFTOVER
    last = harness.driver_log().splitlines()[-1]
    assert last.endswith(f"aws-bench down {harness.fleet_dir}")


def test_declined_prompt_creates_nothing(harness):
    runner = FakeRunner(states=[DONE_STATE])
    args = harness.up_args()
    args.yes = False
    with pytest.raises(awsb.Refused, match="not confirmed"):
        awsb.cmd_up(args, FakeSystem(run=runner))
    assert not runner.ran("tofu", "apply")


@pytest.mark.parametrize(
    ("flags", "pattern"),
    [(("--ttl-min", "auto"), "--ttl-min"), (("--max-usd", "1"), "exceeds --max-usd")],
)
def test_up_refusals(harness, flags, pattern):
    runner = FakeRunner(states=[DONE_STATE])
    with pytest.raises(awsb.Refused, match=pattern):
        harness.up(runner, *flags)
    assert not runner.ran("tofu", "apply")


# REQ:fleet-run-on-fleet
def test_two_runs_reset_between_and_leave_the_fleet_idle(harness, runner):
    for name in ("01-colocated", "02-colocated"):
        mark = len(runner.calls)
        assert harness.run(runner) == awsb.EXIT_OK
        cmds = ssh_calls(runner, mark)
        assert first(cmds, "find /data/journal") < first(cmds, "docker start anvil")
        assert (harness.fleet_dir / "runs" / name / "cost.json").exists()
        assert_idle(harness)
        manifest = run_manifest(harness, name)
        assert manifest["phase"] == "done"
        assert manifest["fleet"] == "fleet1"
        assert manifest["query_db"] == "colocated"
        assert manifest["index"] == int(name[:2])
        assert "run_estimate" in manifest
    rows = [r.split(" |")[0] for r in harness.index()]
    assert rows == ["| fleet1/01-colocated", "| fleet1/02-colocated"]
    assert not runner.ran("tofu", "destroy")


def test_run_logs_its_incremental_cost_and_the_time_left(harness, runner):
    harness.run(runner)
    log = harness.driver_log()
    assert re.search(
        r"run 01-colocated: incremental cost \$\d+\.\d\d \(\d+ min worst\), "
        r"\d+ min left on the fleet",
        log,
    )
    assert re.search(r"run dir .*01-colocated; fleet idle, \d+ min left", log)


def test_invalid_result_exits_1_and_the_fleet_stays_up(harness, runner, monkeypatch):
    monkeypatch.setattr(awsb, "write_report", lambda *_: valid_result(valid=False))
    assert harness.run(runner) == awsb.EXIT_INVALID
    assert_idle(harness)
    assert not runner.ran("tofu", "destroy")


def test_agent_error_exits_3_collects_and_leaves_the_fleet_idle(harness, runner):
    runner.states = [{"phase": "error", "detail": "x", "error": "boom"}]
    runner.polls = 0
    assert harness.run(runner) == awsb.EXIT_FAILED
    assert runner.ran("systemctl stop bench-agent")
    assert runner.ran("rsync", "/opt/bench/out/")
    assert not runner.ran("tofu", "destroy")
    assert_idle(harness)
    summary = harness.fleet_dir / "runs" / "01-colocated" / "summary.md"
    assert "boom" in summary.read_text()


# EDGE:fleet-run-interrupted
def test_interrupt_collects_without_destroy_and_releases_the_lock(
    harness, runner, system
):
    system.run = runner

    def sigint(polls: int) -> None:
        if polls == 2:
            system.fire(signal.SIGINT)

    runner.states = [{"phase": "loading", "detail": "x"}]
    runner.polls = 0
    runner.on_poll = sigint
    assert harness.run_with(system) == awsb.EXIT_FAILED
    assert runner.ran("systemctl stop bench-agent")
    assert runner.ran("rsync", "/opt/bench/out/")
    assert not runner.ran("tofu", "destroy")
    assert_idle(harness)


def test_failed_reset_leaves_the_fleet_dirty_and_locked(harness, runner, monkeypatch):
    original = runner.ssh

    def failing(command: str) -> subprocess.CompletedProcess:
        if "find /data/journal" in command:
            return completed(returncode=255, stderr="unreachable")
        return original(command)

    with monkeypatch.context() as patch:
        patch.setattr(runner, "ssh", failing)
        assert harness.run(runner) == awsb.EXIT_FAILED
    assert harness.fleet()["phase"] == "dirty"
    assert harness.lock().exists()
    assert not runner.ran("docker start")
    assert not runner.ran("tofu", "destroy")
    assert re.search(r"\| failed \| 3 \|", harness.index()[-1])
    mark = len(runner.calls)
    dead = FakeSystem(run=runner, dead_pids={FakeSystem().pid})
    with pytest.raises(awsb.Refused, match=r"--force.*down"):
        harness.run_with(dead)
    assert ssh_calls(runner, mark) == []
    assert harness.run_with(dead, "--force") == awsb.EXIT_OK
    assert_idle(harness)


# EDGE:fleet-stale-running
def test_stale_running_fleet_needs_force(harness, runner):
    harness.set_fleet(phase="running")
    dead = FakeSystem(run=runner, dead_pids={999999})
    lock = {"pid": 999999, "hostname": dead.hostname(), "run": "old", "taken_at": "t"}
    harness.lock().write_text(json.dumps(lock))
    mark = len(runner.calls)
    with pytest.raises(awsb.Refused, match=r"pid 999999 \(dead\).*--force"):
        harness.run_with(dead)
    assert ssh_calls(runner, mark) == []
    assert harness.run_with(dead, "--force") == awsb.EXIT_OK
    assert any("find /data/journal" in c for c in ssh_calls(runner, mark))
    assert_idle(harness)


def test_force_does_not_replace_a_live_holder(harness, runner):
    harness.set_fleet(phase="running")
    awsb.take_fleet_lock(FakeSystem(), harness.fleet_dir, "other", False)
    with pytest.raises(awsb.Refused, match=r"cannot --force.*\(alive\)"):
        harness.run(runner, "--force")


# REQ:fleet-single-shot-unchanged
def test_single_shot_does_not_reset_and_writes_its_index_row(harness):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    code = awsb.cmd_run(harness.single_shot_args(), FakeSystem(run=runner))
    assert code == awsb.EXIT_OK
    assert not runner.ran("find /data/journal")
    assert harness.fleet()["db_modes"] == ["colocated"]
    (row,) = harness.index()
    assert "| colocated |" in row
    assert not harness.lock().exists()


def expires_in(minutes: int) -> dict:
    return {"expires_at": awsb.expiry_stamp(NOW + timedelta(minutes=minutes))}


# REQ:fleet-run-refusals
@pytest.mark.parametrize(
    ("changes", "flags", "pattern"),
    [
        ({"phase": "destroying"}, (), "is destroying, not idle"),
        (expires_in(10), (), r"\d\d min left.*needs up to \d\d: up a new fleet"),
        # EDGE:fleet-ttl-expired
        (expires_in(-5), (), r"expired at .*status.*destroy --orphans"),
        ({}, ("--nodes", "3"), "shape the fleet"),
        ({}, ("--max-usd=5",), "shape the fleet"),
        ({}, ("--ttl-min", "10"), "shape the fleet"),
        ({}, ("--query-db", "volume"), "--query-db volume was not"),
    ],
)
def test_fleet_run_refusals(harness, runner, changes, flags, pattern):
    harness.set_fleet(**changes)
    mark = len(runner.calls)
    with pytest.raises(awsb.Refused, match=pattern):
        harness.run(runner, *flags)
    assert ssh_calls(runner, mark) == []
    assert not harness.lock().exists()


@pytest.mark.parametrize(
    ("argv", "pattern"),
    [
        (("run", "--fleet", str(awsb.OUT_ROOT / "fleet1"), "--yes"), "not a fleet dir"),
        (("run", "--nodes", "2"), "--tag"),
    ],
    ids=["unknown-dir", "single-shot-without-tag"],
)
def test_run_refusals_before_any_call(harness, argv, pattern):
    runner = FakeRunner(states=[DONE_STATE])
    with pytest.raises(awsb.Refused, match=pattern):
        awsb.cmd_run(harness.parse(*argv), FakeSystem(run=runner))
    assert runner.calls == []


def test_node_env_belongs_to_one_run(harness, runner):
    manifest = harness.fleet()
    assert awsb.fleet_run_config(harness.run_args(), manifest).node_env == ()
    cfg = awsb.fleet_run_config(harness.run_args("--node-env", "A=1"), manifest)
    assert cfg.node_env == ("A=1",)
    with pytest.raises(SystemExit):
        harness.up_args("--node-env", "A=1")


def test_lock_held(harness, runner):
    awsb.take_fleet_lock(FakeSystem(), harness.fleet_dir, "other", False)
    mark = len(runner.calls)
    with pytest.raises(awsb.Refused, match=r"locked: run other, pid \d+ \(alive\)"):
        harness.run(runner)
    assert ssh_calls(runner, mark) == []
    assert harness.lock().exists()


def test_the_lockout_window_counts_against_the_ttl(harness, runner):
    manifest = harness.fleet()
    cfg = awsb.fleet_run_config(harness.run_args(), manifest)
    worst = awsb.estimate_run(manifest, cfg)["worst_s"]
    short = NOW + timedelta(seconds=worst + awsb.DESTROY_S + 60)
    with pytest.raises(awsb.Refused, match="up a new fleet"):
        awsb.check_run_allowed(
            {**manifest, "expires_at": awsb.expiry_stamp(short)}, cfg, NOW
        )
    enough = short + timedelta(seconds=awsb.NOLOGIN_LEAD_S)
    awsb.check_run_allowed(
        {**manifest, "expires_at": awsb.expiry_stamp(enough)}, cfg, NOW
    )


def respond_to_pulls(runner: FakeRunner, wrong: dict[str, str]) -> None:
    """Pulls report the requested digests except those in `wrong`; the record script echoes
    the digests it was given."""

    def pull(argv: list[str]) -> subprocess.CompletedProcess:
        if not argv[-1].startswith("sudo timeout"):
            return runner.default(argv)
        script = shlex.split(argv[-1])[-1]
        pulls = re.findall(r"docker pull (\S+)@(\S+) >&2", script)
        names = re.findall(r"--arg name (\S+) ", script)
        digests = {
            name: f"{ref}@{wrong.get(name, digest)}"
            for name, (ref, digest) in zip(names, pulls, strict=True)
        }
        return completed(stdout=json.dumps(digests))

    def record(argv: list[str]) -> subprocess.CompletedProcess:
        if not argv[-1].startswith("sudo timeout"):
            return runner.default(argv)
        script = shlex.split(argv[-1])[-1]
        printf = next(l for l in script.splitlines() if l.startswith("printf"))
        return completed(
            stdout=json.dumps({"digests": json.loads(shlex.split(printf)[2])})
        )

    runner.respond("docker pull", pull)
    runner.respond("digests.json.tmp", record)


@pytest.fixture
def resolved(monkeypatch: pytest.MonkeyPatch) -> list[str]:
    """Refs looked up in a registry that has `:other` at `NEW_DIGEST`."""
    refs: list[str] = []

    def resolve(_http_get, ref: str) -> dict:
        refs.append(ref)
        image = fake_image(ref)
        return {**image, "digest": NEW_DIGEST} if ref.endswith(":other") else image

    monkeypatch.setattr(awsb, "resolve_image", resolve)
    return refs


def pull_fleet(harness: FleetHarness, resolved: list[str], wrong=None) -> FakeRunner:
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    respond_to_pulls(runner, wrong or {})
    assert harness.up(runner) == awsb.EXIT_OK
    resolved.clear()
    return runner


# REQ:fleet-tag-pull
def test_a_different_tag_pulls_every_role_image_by_digest_on_every_host(
    harness, resolved
):
    runner = pull_fleet(harness, resolved)
    mark = len(runner.calls)
    assert harness.run(runner, "--tag", "other") == awsb.EXIT_OK
    commands = ssh_calls(runner, mark)
    pulls = [c for c in commands if "docker pull" in c]
    assert len(pulls) == 3
    for name in awsb.IMAGE_COMPONENTS:
        ref = f"{awsb.GHCR_ORG}/{name}:other@{NEW_DIGEST}"
        per_host = 2 if name == "espresso-node" else 1
        assert sum(c.count(f"docker pull {ref}") for c in pulls) == per_host, name
    for ref in awsb.SUPPORT_IMAGES.values():
        assert sum(c.count(f"docker pull {ref}@") for c in pulls) == 1, ref
    record_at = max(i for i, c in enumerate(commands) if "digests.json.tmp" in c)
    assert first(commands, "docker pull") < record_at
    assert record_at < first(commands, "find /data/journal")


def test_the_new_images_become_the_fleets_and_the_runs(harness, resolved):
    runner = pull_fleet(harness, resolved)
    harness.run(runner, "--tag", "other")
    fleet = harness.fleet()
    assert fleet["config"]["tag"] == "other"
    assert fleet["phase"] == "idle"
    for name in awsb.IMAGE_COMPONENTS:
        assert fleet["images"][name]["digest"] == NEW_DIGEST
    run = run_manifest(harness)
    assert run["images"] == fleet["images"]
    assert run["config"]["tag"] == "other"


def test_the_same_tag_pulls_nothing(harness, resolved):
    runner = pull_fleet(harness, resolved)
    mark = len(runner.calls)
    assert harness.run(runner, "--tag", "t") == awsb.EXIT_OK
    assert harness.run(runner) == awsb.EXIT_OK
    assert not any("docker pull" in c for c in ssh_calls(runner, mark))
    assert resolved == []


# EDGE:fleet-pull-digest-mismatch
def test_a_pulled_digest_that_differs_fails_before_the_reset(harness, resolved):
    runner = pull_fleet(harness, resolved, {"espresso-node": f"sha256:{'2' * 64}"})
    before = harness.fleet()["images"]
    mark = len(runner.calls)
    assert harness.run(runner, "--tag", "other") == awsb.EXIT_FAILED
    commands = ssh_calls(runner, mark)
    assert not any("find /data/journal" in c for c in commands)
    assert not any("digests.json.tmp" in c for c in commands)
    fleet = harness.fleet()
    assert fleet["images"] == before
    assert fleet["config"]["tag"] == "t"
    assert fleet["phase"] == "dirty"
    assert harness.lock().exists()
    assert not runner.ran("tofu", "destroy")


def test_a_tag_change_needs_the_ttl_for_the_pull(harness, resolved):
    runner = pull_fleet(harness, resolved)
    manifest = harness.fleet()
    cfg = awsb.fleet_run_config(harness.run_args(), manifest)
    needed = awsb.estimate_run(manifest, cfg)["worst_s"] + awsb.NOLOGIN_LEAD_S
    expires = awsb.expiry_stamp(
        NOW + timedelta(seconds=needed + awsb.DESTROY_S + awsb.PULL_MAX_S - 30)
    )
    harness.set_fleet(expires_at=expires)
    assert harness.run(runner) == awsb.EXIT_OK
    harness.set_fleet(expires_at=expires)
    mark = len(runner.calls)
    with pytest.raises(awsb.Refused, match="up a new fleet"):
        harness.run(runner, "--tag", "other")
    assert ssh_calls(runner, mark) == []


def test_the_run_records_the_revision_shipped_for_it(harness, runner, monkeypatch):
    monkeypatch.setattr(awsb, "git_head", lambda *_: "b" * 40)
    harness.run(runner)
    assert run_manifest(harness)["git_rev"] == "b" * 40
    assert harness.fleet()["git_rev"] == "a" * 40


def test_a_lock_holder_on_another_host_counts_as_alive():
    lock = {"pid": 1, "hostname": "elsewhere", "run": "r", "taken_at": "t"}
    assert awsb.lock_holder_alive(FakeSystem(dead_pids={1}), lock)


def test_lock_is_exclusive_and_names_the_holder(tmp_path: Path):
    lock = awsb.take_fleet_lock(FakeSystem(), tmp_path, "a", False)
    assert lock["pid"] == FakeSystem().pid
    assert netbench.read_json(tmp_path / awsb.FLEET_LOCK) == lock
    with pytest.raises(awsb.Refused, match="run a, pid"):
        awsb.take_fleet_lock(FakeSystem(), tmp_path, "b", False)
    awsb.release_fleet_lock(tmp_path)
    awsb.take_fleet_lock(FakeSystem(), tmp_path, "b", False)


def test_release_without_a_lock_is_fine(tmp_path: Path):
    awsb.release_fleet_lock(tmp_path)
    assert list(tmp_path.iterdir()) == []


@pytest.mark.parametrize(
    ("modes", "volume"), [(("colocated", "volume"), PG_VOLUME), (("colocated",), None)]
)
# REQ:querydb-volume-wiring
def test_tfvars_carry_the_volume_only_when_the_mode_is_provisioned(modes, volume):
    cfg = awsb.RunConfig(
        tag="x", nodes=2, db_modes=modes, load=netbench.BenchConfig(submit_nodes=1)
    )
    rest = (
        "k",
        "203.0.113.5/32",
        "2026-01-01T00:00:00Z",
        "abc1234",
        "eu-west-1a",
        "ami",
    )
    tfvars = awsb.render_tfvars(
        cfg, Path("f"), "f", "alice", awsb.plan_hosts(cfg), *rest
    )
    assert tfvars["pg_volume"] == volume


def test_up_makes_and_mounts_the_volume_on_the_query_host_only(harness):
    runner = volume_runner([DONE_STATE])
    assert harness.up(runner, "--db-modes", "volume") == awsb.EXIT_OK
    manifest = harness.fleet()
    assert manifest["phase"] == "idle"
    assert manifest["pg_volume_id"] == VOLUME_ID
    assert manifest["pg_volume"] == PG_VOLUME
    (script,) = [c for c in ssh_calls(runner) if "mkfs.ext4" in c]
    assert NODE0_IP in script
    assert BY_ID in script
    assert script.index("mkfs.ext4") < script.index("mount -t")


@pytest.mark.parametrize("volume_id", [VOLUME_ID, None], ids=["no-device", "no-id"])
def test_a_volume_failure_during_up_destroys_and_exits_3(harness, volume_id):
    runner = volume_runner([DONE_STATE], volume_id=volume_id, describe=DESCRIBE)
    runner.respond(
        "mkfs.ext4",
        lambda _: completed(returncode=1, stderr=f"{BY_ID} missing after 60 s"),
    )
    assert harness.up(runner, "--db-modes", "volume") == awsb.EXIT_FAILED
    assert runner.ran("tofu", "destroy")
    assert runner.ran("mkfs") == (volume_id is not None)


def test_a_volume_run_records_its_store(harness):
    runner = volume_runner([DONE_STATE], describe=DESCRIBE)
    harness.up(runner, "--db-modes", "volume")
    assert harness.run(runner, "--query-db", "volume") == awsb.EXIT_OK
    manifest = run_manifest(harness, "01-volume")
    assert manifest["phase"] == "done"
    assert manifest["query_db"] == "volume"
    assert manifest["pg_volume_id"] == VOLUME_ID


PG_MANIFEST = {"name": "f", "pg_volume_id": VOLUME_ID}


def test_the_volume_reset_mounts_and_wipes_without_reformatting():
    store = awsb.pg_store_script("volume", PG_MANIFEST)
    script = awsb.reset_script("query", store)
    assert "mkfs" not in script
    assert f'mount -t ext4 -o "$(findmnt -no OPTIONS /)" {BY_ID}' in script
    assert script.index("docker rm -f") < script.index("mount -t")
    assert script.index("mount -t") < script.rindex("find /data/pg")
    assert "mount -t" not in awsb.reset_script("validator", store)


# TEST:querydb-mode-switch-ok
def test_colocated_unmounts_after_the_containers_and_before_the_wipe():
    store = awsb.pg_store_script("colocated", PG_MANIFEST)
    script = awsb.reset_script("query", store)
    assert "mount -t" not in script
    assert "mountpoint -q /data/pg" in script
    unmount = script.index("umount /data/pg")
    assert script.index("docker rm -f") < unmount < script.index("find /data/pg")


def test_volume_after_colocated_empties_the_root_copy_before_it_is_hidden():
    script = awsb.pg_store_script("volume", PG_MANIFEST)
    guard, _, mounted = script.partition("then\n")
    assert "! mountpoint -q /data/pg" in guard
    wipe = mounted.index("find /data/pg -mindepth 1 -delete")
    assert wipe < mounted.index("mount -t ext4")


def volume_rate(volume: Any) -> float:
    cfg = awsb.RunConfig(tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1))
    lines = awsb._cost_lines(awsb.plan_hosts(cfg), 3600.0, volume)
    return sum(l["usd"] for l in lines if l["item"] != "egress")


def test_the_volume_adds_size_iops_and_throughput_above_the_baseline():
    expected = (
        400 * awsb.PRICES["gp3_gb_month_usd"]
        + 9000 * awsb.PRICES["gp3_iops_month_usd"]
        + 375 * awsb.PRICES["gp3_mbps_month_usd"]
    ) / awsb.HOURS_PER_MONTH
    assert volume_rate(PG_VOLUME) - volume_rate(None) == pytest.approx(expected)


def test_the_fleet_estimate_prices_the_volume_only_when_provisioned():
    def gb_hours(modes: tuple[str, ...]) -> float:
        cfg = awsb.RunConfig(
            tag="x",
            nodes=2,
            db_modes=modes,
            ttl_min="60",
            load=netbench.BenchConfig(submit_nodes=1),
        )
        estimate = awsb.cost_estimate(awsb.plan_hosts(cfg), cfg, None, 3600, 3600)
        return next(l["qty"] for l in estimate["lines"] if l["item"] == "gp3 storage")

    extra = gb_hours(("colocated", "volume")) - gb_hours(("colocated",))
    assert extra == awsb.PG_GB


# REQ:fleet-cost
def test_run_cost_is_the_rate_over_the_wall_time(harness, runner):
    harness.run(runner)
    cost = netbench.read_json(harness.fleet_dir / "runs" / "01-colocated" / "cost.json")
    rate = awsb.estimate_rate(harness.fleet()["estimate"])
    assert cost["usd_per_hour"] == pytest.approx(rate)
    assert cost["usd"] == pytest.approx(rate * cost["duration_s"] / 3600)


def test_explicit_ttl_below_the_worst_case_is_refused_for_a_single_shot():
    with pytest.raises(awsb.Refused, match="below the worst case"):
        awsb.ttl_seconds(awsb.RunConfig(tag="x", ttl_min="5"), 3600.0)
    assert awsb.ttl_seconds(awsb.RunConfig(tag="x", ttl_min="90"), 3600.0) == 5400.0
    auto = awsb.ttl_seconds(awsb.RunConfig(tag="x"), 3600.0)
    assert auto == 3600.0 + awsb.TTL_MARGIN_S


# REQ:fleet-down
def test_down_destroys_and_prices_every_run(harness, runner):
    harness.run(runner)
    harness.run(runner)
    assert harness.down(runner, "--yes") == awsb.EXIT_OK
    assert runner.ran("tofu", "destroy")
    assert netbench.read_json(harness.fleet_dir / "cost.json")["actual"] > 0
    assert harness.fleet()["phase"] == "done"
    assert not harness.lock().exists()
    log = harness.driver_log()
    assert re.search(
        r"destroyed; actual cost \$\d+\.\d\d \(bound \$\d+\.\d\d\); 2 runs", log
    )
    assert "- 01-colocated: valid, $" in log
    assert "- 02-colocated: valid, $" in log


def test_leftover_after_failed_destroys_exits_4(harness, runner):
    runner.destroys = [completed(returncode=1, stderr="locked")]
    assert harness.down(runner, "--yes") == awsb.EXIT_LEFTOVER
    assert runner.count("tofu", "destroy") == awsb.DESTROY_RETRIES
    assert runner.ran("terminate-instances", "i-1")
    assert harness.fleet()["phase"] == "left-running"
    assert not (harness.out / "INDEX.md").exists()


# EDGE:fleet-down-while-running
def test_down_while_running_stops_the_agent_first(harness, runner):
    harness.set_fleet(phase="running")
    awsb.take_fleet_lock(FakeSystem(), harness.fleet_dir, "gone", False)
    mark = len(runner.calls)
    assert harness.down(runner, "--yes") == awsb.EXIT_OK
    calls = [" ".join(call) for call in runner.calls[mark:]]
    destroy = next(
        i for i, c in enumerate(calls) if c.startswith("tofu") and "destroy" in c
    )
    assert first(calls, "systemctl stop bench-agent") < destroy
    assert not harness.lock().exists()


def test_declined_down_destroys_nothing(harness, runner):
    with pytest.raises(awsb.Refused, match="not confirmed"):
        harness.down(runner)
    assert not runner.ran("tofu", "destroy")


def test_status_shows_the_time_left_and_the_lock_holder(harness, runner, capsys):
    runner.describe = STATUS_DESCRIBE
    awsb.take_fleet_lock(FakeSystem(), harness.fleet_dir, "measuring", False)
    args = harness.parse("status", str(harness.fleet_dir))
    awsb.cmd_status(args, FakeSystem(run=runner))
    text = capsys.readouterr().out
    assert "- fleet fleet1: phase idle" in text
    assert re.search(r"- expires \S+ UTC, 1[78]\d min left", text)
    assert re.search(r"- lock: run measuring, pid \d+ \(alive\)", text)


def test_db_modes_dedupe_and_refuse_an_unknown_mode():
    assert awsb.parse_db_modes("colocated") == ("colocated",)
    assert awsb.parse_db_modes("colocated,colocated") == ("colocated",)
    with pytest.raises(argparse.ArgumentTypeError, match="unknown db mode"):
        awsb.parse_db_modes("colocated,nonsense")


@pytest.mark.parametrize("text", ["auto", "90"])
def test_ttl_min_accepts_auto_and_minutes(text: str):
    assert awsb.ttl_min_arg(text) == text


@pytest.mark.parametrize("text", ["0", "-5", "1.5", "soon"])
def test_ttl_min_refuses_other_values(text: str):
    with pytest.raises(argparse.ArgumentTypeError):
        awsb.ttl_min_arg(text)


def test_up_defaults_to_a_long_ttl_and_budget_and_run_to_a_short_one():
    up = awsb.parse_args(["up", "--tag", "t"])
    assert (up.ttl_min, up.max_usd) == (awsb.UP_TTL_MIN, awsb.UP_MAX_USD)
    assert up.db_modes == ("colocated",)
    run = awsb.parse_args(["run", "--tag", "t"])
    assert (run.ttl_min, run.max_usd, run.fleet) == ("auto", 10.0, None)


def test_up_needs_a_tag_but_run_fleet_does_not():
    with pytest.raises(SystemExit):
        awsb.parse_args(["up"])
    args = awsb.parse_args(["run", "--fleet", "some/dir"])
    assert args.tag is None
    assert args.fleet == Path("some/dir")


def test_run_does_not_abbreviate_flags():
    with pytest.raises(SystemExit):
        awsb.parse_args(["run", "--fleet", "d", "--node", "3"])


def test_config_round_trips_through_the_fleet_manifest():
    cfg = awsb.RunConfig(tag="x", ttl_min="90", fleet=Path("a/b"))
    saved = json.loads(json.dumps(awsb.config_to_json(cfg)))
    assert awsb.config_from_manifest(saved) == cfg
