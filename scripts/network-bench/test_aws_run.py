import dataclasses
import gzip
import json
import logging
import os
import signal
import subprocess
from collections.abc import Callable
from datetime import datetime
from pathlib import Path
from typing import Any

import netbench
import pytest
from fakes import (
    DESCRIBE,
    DONE_STATE,
    FAKE_EPOCH,
    FakeClock,
    FakeRunner,
    FakeSystem,
    RunHarness,
    Scripted,
    awsb,
    completed,
    fake_image,
    fake_images,
    raiser,
    remote,
    two_node_hosts_info,
    valid_result,
)

LOADING = {"phase": "loading", "detail": "x"}
LOCKED = completed(returncode=1, stderr="locked")


def ran_ssh(runner: FakeRunner) -> bool:
    return any(call[0] == "ssh" for call in runner.calls)


# REQ:awsbench-apply-failure
def test_apply_failure_logs_the_error_destroys_and_exits_3(run_harness: RunHarness):
    apply = completed(returncode=1, stderr="Error: InsufficientInstanceCapacity")
    runner = FakeRunner(states=[DONE_STATE], apply=apply)
    assert run_harness.run(runner) == awsb.EXIT_FAILED
    assert runner.ran("tofu", "apply") and runner.ran("tofu", "destroy")
    assert not ran_ssh(runner)
    assert "InsufficientInstanceCapacity" in run_harness.log()
    assert "| 3 |" in run_harness.index()


def test_declined_prompt_refuses_before_apply(run_harness: RunHarness):
    runner = FakeRunner(states=[DONE_STATE])
    args = run_harness.args()
    args.yes = False
    with pytest.raises(awsb.Refused):
        awsb.cmd_run(args, FakeSystem(run=runner))
    assert not runner.ran("tofu", "apply")
    assert not ran_ssh(runner)


@pytest.mark.usefixtures("valid")
def test_valid_run_exits_0_with_cost_and_index(run_harness: RunHarness):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_OK
    cost = netbench.read_json(run_harness.fleet_dir / "cost.json")
    assert cost["duration_s"] == pytest.approx(1800.0)
    assert 0 < cost["actual"] < cost["bound"]
    assert netbench.read_json(run_harness.fleet_dir / "fleet.json")["phase"] == "done"
    manifest = netbench.read_json(run_harness.run_dir / "manifest.json")
    assert manifest["phase"] == "done"
    assert manifest["fleet"] == "run1"
    assert manifest["cost_usd"]["actual"] == cost["actual"]
    assert manifest["start_spread_s"] == 0.0
    *_, row = run_harness.index().splitlines()
    assert row.startswith("| run1/01-run |")
    assert "| colocated |" in row
    assert "| valid | 0 |" in row
    assert netbench.read_json(run_harness.run_dir / "cost.json")["usd"] > 0


# TEST:system-expiry-exact-ok
@pytest.mark.usefixtures("valid")
def test_expiry_is_stamped_after_confirm_and_matches_the_manifest(
    run_harness: RunHarness,
):
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    run_harness.run(runner)
    manifest = netbench.read_json(run_harness.fleet_dir / "fleet.json")
    tfvars = netbench.read_json(
        run_harness.fleet_dir / "terraform" / "terraform.tfvars.json"
    )
    assert tfvars["expires_at"] == manifest["expires_at"]
    exact = FAKE_EPOCH + manifest["estimate"]["ttl_s"] + awsb.PROVISION_S
    assert datetime.fromisoformat(tfvars["expires_at"]).timestamp() == exact
    assert sum(c[0] == "tofu" and c[2] == "plan" for c in runner.calls) == 2


def test_invalid_result_exits_1(
    run_harness: RunHarness, monkeypatch: pytest.MonkeyPatch
):
    monkeypatch.setattr(awsb, "write_report", lambda *a, **k: valid_result(False))
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_INVALID
    assert runner.ran("tofu", "destroy")


@pytest.mark.usefixtures("valid")
@pytest.mark.parametrize(
    ("destroy", "code", "key_stays"),
    [(completed(), awsb.EXIT_OK, False), (LOCKED, awsb.EXIT_LEFTOVER, True)],
    ids=["destroyed", "leftover"],
)
def test_ssh_key_is_removed_only_after_a_destroy(
    run_harness: RunHarness,
    destroy: subprocess.CompletedProcess,
    code: int,
    key_stays: bool,
):
    runner = FakeRunner(states=[DONE_STATE], destroys=[destroy], describe=DESCRIBE)
    assert run_harness.run(runner) == code
    key = run_harness.fleet_dir / "ssh" / "id_ed25519"
    assert runner.ran("-i", str(key))
    assert key.exists() == key_stays
    assert key.with_suffix(".pub").exists()


def test_agent_error_collects_reports_failure_and_exits_3(run_harness: RunHarness):
    error = {"phase": "error", "detail": "x", "error": "network not ready: heights"}
    runner = FakeRunner(states=[error], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_FAILED
    assert "network not ready" in (run_harness.run_dir / "summary.md").read_text()
    assert runner.ran("systemctl stop bench-agent")
    assert runner.ran("docker stop")
    assert runner.ran("rsync", "/opt/bench/out/")
    assert runner.ran("tofu", "destroy")


def run_signalled(run_harness: RunHarness, signals: int) -> tuple[FakeRunner, int]:
    """`cmd_run` receiving `signals` SIGINTs on the second agent poll."""

    def on_poll(polls: int) -> None:
        if polls == 2:
            for _ in range(signals):
                system.fire(signal.SIGINT)

    runner = FakeRunner(states=[LOADING], on_poll=on_poll)
    system = FakeSystem(run=runner)
    return runner, awsb.cmd_run(run_harness.args(), system)


# REQ:awsbench-interrupt
def test_interrupt_stops_agent_collects_destroys_and_exits_3(run_harness: RunHarness):
    runner, code = run_signalled(run_harness, 1)
    assert code == awsb.EXIT_FAILED
    assert runner.ran("systemctl stop bench-agent")
    assert runner.ran("rsync", "/opt/bench/out/")
    assert runner.ran("tofu", "destroy")
    assert "interrupted" in (run_harness.run_dir / "summary.md").read_text()


def test_skipped_collection_still_destroys(run_harness: RunHarness):
    runner, code = run_signalled(run_harness, awsb.SKIP_COLLECT_SIGNALS)
    assert code == awsb.EXIT_FAILED
    assert not runner.ran("rsync", "/opt/bench/out/")
    assert runner.ran("tofu", "destroy")


def test_sighup_is_handled_like_sigint(system: FakeSystem, clock: FakeClock):
    awsb.Interrupts(clock).install(system.trap)
    assert set(system.handlers) == {signal.SIGINT, signal.SIGTERM, signal.SIGHUP}


@pytest.fixture
def interrupts(system: FakeSystem, clock: FakeClock) -> Any:
    interrupts = awsb.Interrupts(clock)
    interrupts.install(system.trap)
    return interrupts


def test_first_signal_logs_the_phase_and_later_ones_remind(
    system: FakeSystem, interrupts: Any, caplog: pytest.LogCaptureFixture
):
    interrupts.phase = "services"
    with caplog.at_level(logging.WARNING, awsb.log.name):
        system.fire(signal.SIGINT)
        system.fire(signal.SIGINT)
    assert interrupts.event.is_set()
    assert caplog.messages == [
        "interrupt received; stopping after the current step (services)",
        "still waiting for the current step (services)",
    ]


def test_interrupts_are_ignored_after_disarm(system: FakeSystem, interrupts: Any):
    interrupts.disarm()
    system.fire(signal.SIGINT)
    interrupts.check()
    interrupts.sleep(0)


def test_third_signal_skips_the_collection_even_after_disarm(
    system: FakeSystem, interrupts: Any
):
    interrupts.disarm()
    system.fire(signal.SIGINT)
    system.fire(signal.SIGINT)
    assert not interrupts.skip_collect.is_set()
    system.fire(signal.SIGINT)
    assert interrupts.skip_collect.is_set()


# TEST:interrupt-during-fake-sleep-fails
def test_a_signal_during_the_poll_sleep_raises_after_the_wait(
    system: FakeSystem, clock: FakeClock, interrupts: Any
):
    clock.on_advance = lambda now: system.fire(signal.SIGINT)
    with pytest.raises(awsb.Interrupted):
        interrupts.sleep(awsb.AGENT_POLL_S)
    assert clock.sleeps == [awsb.AGENT_POLL_S]


# TEST:destroy-verdict-ok
@pytest.mark.parametrize(
    ("attempt", "ok", "verdict"),
    [
        *((n, True, "done") for n in range(1, awsb.DESTROY_RETRIES + 1)),
        *((n, False, "retry") for n in range(1, awsb.DESTROY_RETRIES)),
        (awsb.DESTROY_RETRIES, False, "sweep"),
    ],
)
def test_destroy_verdict(attempt: int, ok: bool, verdict: str):
    assert awsb.destroy_verdict(attempt, ok) == verdict


class FailingTerraform:
    """`Terraform` whose first `failures` destroys fail."""

    def __init__(self, failures: int) -> None:
        self.failures = failures

    def destroy(self) -> None:
        if self.failures:
            self.failures -= 1
            raise awsb.TfFailed("destroy", "locked")


@pytest.mark.parametrize(
    ("failures", "destroyed", "sleeps", "sweeps"),
    [(awsb.DESTROY_RETRIES, False, 2, 1), (1, True, 1, 0)],
    ids=["sweep-after-the-last", "late-success"],
)
def test_destroy_backs_off_between_attempts(
    tmp_path: Path,
    system: FakeSystem,
    clock: FakeClock,
    monkeypatch: pytest.MonkeyPatch,
    failures: int,
    destroyed: bool,
    sleeps: int,
    sweeps: int,
):
    swept: list[str] = []
    monkeypatch.setattr(awsb, "sweep", lambda _system, name: swept.append(name) or [])
    fleet = awsb.FleetState(
        system,
        awsb.RunConfig(tag="x"),
        tmp_path / "fleet1",
        FailingTerraform(failures),
        awsb.Interrupts(clock),
    )
    assert awsb.destroy_fleet(fleet) == destroyed
    assert clock.sleeps == [awsb.DESTROY_BACKOFF_S] * sleeps
    assert swept == ["fleet1"] * sweeps


# REQ:awsbench-destroy-fallback
def test_three_failed_destroys_sweep_and_exit_4(run_harness: RunHarness):
    error = {"phase": "error", "detail": "x", "error": "boom"}
    runner = FakeRunner(states=[error], destroys=[LOCKED])
    assert run_harness.run(runner) == awsb.EXIT_LEFTOVER
    assert runner.count("tofu", "destroy") == awsb.DESTROY_RETRIES
    assert runner.ran("terminate-instances", "i-1")
    assert runner.ran("delete-security-group", "sg-1")
    assert runner.ran("delete-key-pair", "key-1")
    assert run_harness.ends_with_down_command()
    assert "| 4 |" in run_harness.index()


def test_destroy_recovers_on_retry(run_harness: RunHarness):
    error = {"phase": "error", "detail": "x", "error": "boom"}
    runner = FakeRunner(
        states=[error], destroys=[LOCKED, completed()], describe=DESCRIBE
    )
    assert run_harness.run(runner) == awsb.EXIT_FAILED
    assert runner.count("tofu", "destroy") == 2
    assert not runner.ran("terminate-instances")


# REQ:awsbench-teardown-guaranteed
def test_an_exception_in_finish_still_destroys(
    run_harness: RunHarness, monkeypatch: pytest.MonkeyPatch
):
    monkeypatch.setattr(awsb, "finish_run", raiser(IndexError("x")))
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_FAILED
    assert runner.ran("tofu", "destroy")
    assert "ERROR finish failed" in run_harness.log()
    assert "IndexError: x" in run_harness.log()


def test_an_exception_in_the_report_writes_the_failure_summary_and_destroys(
    run_harness: RunHarness, monkeypatch: pytest.MonkeyPatch
):
    monkeypatch.setattr(awsb, "write_report", raiser(ZeroDivisionError("x")))
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_FAILED
    assert runner.ran("tofu", "destroy")
    assert "ZeroDivisionError" in (run_harness.run_dir / "summary.md").read_text()


@pytest.mark.usefixtures("valid")
def test_exception_in_cost_after_destroy_follows_the_result(
    run_harness: RunHarness, monkeypatch: pytest.MonkeyPatch
):
    monkeypatch.setattr(awsb, "actual_cost", raiser(TypeError("reason")))
    runner = FakeRunner(states=[DONE_STATE], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_OK
    assert runner.ran("tofu", "destroy")
    assert "TypeError: reason" in run_harness.log()


@pytest.mark.usefixtures("valid")
@pytest.mark.parametrize(
    ("target", "destroy"),
    [("append_index", LOCKED), ("destroy_fleet", completed())],
    ids=["bookkeeping-after-failed-destroy", "destroy"],
)
def test_an_exception_with_leftovers_exits_4_with_command_last(
    run_harness: RunHarness,
    monkeypatch: pytest.MonkeyPatch,
    target: str,
    destroy: subprocess.CompletedProcess,
):
    monkeypatch.setattr(awsb, target, raiser(RuntimeError("boom")))
    runner = FakeRunner(states=[DONE_STATE], destroys=[destroy], describe=DESCRIBE)
    assert run_harness.run(runner) == awsb.EXIT_LEFTOVER
    assert run_harness.ends_with_down_command()


def test_truncated_node_log_does_not_break_the_failure_summary(tmp_path: Path):
    host_dir = tmp_path / "hosts" / "node0"
    host_dir.mkdir(parents=True)
    good = gzip.compress(b"line\n" * 1000)
    (host_dir / "espresso-node.log.gz").write_bytes(good[: len(good) // 2])
    awsb.write_failure_summary(tmp_path, {"hosts": [{"name": "node0"}]}, "boom")
    assert "log unreadable" in (tmp_path / "summary.md").read_text()


def test_interrupt_before_the_fleet_exists_is_refused(
    run_harness: RunHarness, monkeypatch: pytest.MonkeyPatch
):
    monkeypatch.setattr(awsb, "preflight", raiser(KeyboardInterrupt()))
    runner = FakeRunner(states=[DONE_STATE])
    with pytest.raises(awsb.Refused, match="interrupted"):
        run_harness.run(runner)
    assert not runner.ran("tofu", "apply")


# REQ:awsbench-manifest-repro
@pytest.mark.usefixtures("run_harness")
def test_fleet_name_collision_refuses():
    awsb.new_fleet_dir(FakeSystem(), awsb.OUT_ROOT)
    with pytest.raises(awsb.Refused, match="already exists"):
        awsb.new_fleet_dir(FakeSystem(), awsb.OUT_ROOT)


CFG = awsb.RunConfig(tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1))
HOSTS = awsb.plan_hosts(CFG)
IMAGES = fake_images()


@pytest.mark.parametrize(
    ("cfg", "hosts", "images", "genesis"),
    [
        (
            CFG,
            HOSTS,
            {**IMAGES, "espresso-node": fake_image("r") | {"digest": "sha256:1"}},
            b"genesis",
        ),
        (
            CFG,
            [dict(h, instance_type="c8g.8xlarge") for h in HOSTS],
            IMAGES,
            b"genesis",
        ),
        (CFG, HOSTS, IMAGES, b"other"),
        (
            awsb.RunConfig(
                tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1, step_s=31)
            ),
            HOSTS,
            IMAGES,
            b"genesis",
        ),
        (dataclasses.replace(CFG, node_env=("A=1",)), HOSTS, IMAGES, b"genesis"),
        (dataclasses.replace(CFG, query_db="rds"), HOSTS, IMAGES, b"genesis"),
    ],
    ids=["digest", "instance-type", "genesis", "load", "node-env", "query-db"],
)
def test_config_hash_changes_with_the_config(cfg, hosts, images, genesis):
    base = awsb.run_config_hash(CFG, HOSTS, IMAGES, b"genesis")
    assert base == awsb.run_config_hash(CFG, HOSTS, IMAGES, b"genesis")
    assert base != awsb.run_config_hash(cfg, hosts, images, genesis)


# TEST:remote-control-path-h-ok, at the longest name `NAME_RE` allows
def test_control_path_is_keyed_by_host_and_fits_the_socket_limit(isolated: Path):
    fleet_dir = awsb.OUT_ROOT / ("a" * 40)
    fleet_dir.mkdir(parents=True)
    ssh = awsb.Remote(Scripted({}), fleet_dir, Path("id"), two_node_hosts_info())
    assert f"ControlPath={fleet_dir}/ssh/%h" in ssh.ssh_opts
    assert awsb.control_path_peak(fleet_dir) <= awsb.SOCKET_PATH_MAX


# TEST:remote-absolute-dir-fails
def test_an_absolute_fleet_dir_that_overflows_the_socket_path_is_refused(
    isolated: Path,
):
    fleet_dir = isolated.resolve() / ("d" * 64) / awsb.OUT_ROOT / ("a" * 40)
    with pytest.raises(awsb.Refused, match="relative"):
        awsb.Remote(Scripted({}), fleet_dir, Path("id"), two_node_hosts_info())
    assert not fleet_dir.exists()


# EDGE:awsbench-ssh-not-ready
def test_wait_ssh_retries_until_reachable(isolated: Path, clock: FakeClock):
    results = iter([completed(returncode=255, stderr="refused")] * 2 + [completed()])
    ssh = remote(lambda argv, env=None: next(results), isolated)
    awsb.wait_ssh(ssh, "ctl", awsb.Interrupts(clock))
    assert clock.sleeps == [awsb.SSH_RETRY_S, awsb.SSH_RETRY_S * 1.5]


def test_wait_ssh_gives_up_after_the_timeout(isolated: Path):
    runner = FakeRunner({("ssh",): completed(returncode=255, stderr="refused")})
    clock = FakeClock()
    with pytest.raises(awsb.RemoteError, match="not reachable"):
        awsb.wait_ssh(remote(runner, isolated), "ctl", awsb.Interrupts(clock))
    assert clock.time() >= awsb.SSH_READY_TIMEOUT_S


# TEST:fakeclock-limit-fails
def test_a_gate_that_never_passes_hits_the_clock_limit(isolated: Path):
    runner = FakeRunner({("ssh",): completed(returncode=1, stderr="no")})
    clock = FakeClock(limit_s=awsb.GATE_TIMEOUT_S / 2)
    with pytest.raises(RuntimeError, match="FakeClock: advanced past"):
        awsb.gate(
            remote(runner, isolated), "ctl", "anvil", "curl x", awsb.Interrupts(clock)
        )


# TEST:gate-timeout-fails
def test_gate_retries_until_its_timeout_on_the_clock(isolated: Path):
    runner = FakeRunner({("ssh",): completed(returncode=1, stderr="no")})
    clock = FakeClock(limit_s=10 * awsb.GATE_TIMEOUT_S)
    with pytest.raises(awsb.RemoteError, match="gate `anvil`"):
        awsb.gate(
            remote(runner, isolated), "ctl", "anvil", "curl x", awsb.Interrupts(clock)
        )
    assert clock.time() >= awsb.GATE_TIMEOUT_S


def test_start_nodes_spread_from_started_at(isolated: Path):
    stamps = {
        "203.0.113.2": "2026-09-29T15:00:00.100000000Z",
        "203.0.113.3": "2026-09-29T15:00:00.350000000Z",
    }

    def runner(argv, env=None):
        if "inspect" in argv[-1]:
            return completed(stdout=stamps[argv[-2].split("@")[1]] + "\n")
        return completed()

    info = two_node_hosts_info()
    spread = awsb.start_nodes(
        remote(runner, isolated), [info["node0"], info["node1"]], 1234.5
    )
    assert spread == pytest.approx(0.25, abs=1e-3)


# REQ:awsbench-collect-bounded
def test_stop_freeze_and_collect_run_under_timeout(isolated: Path):
    runner = Scripted({})
    hosts = remote(runner, isolated)
    awsb.stop_agent(hosts)
    awsb.freeze(hosts)
    awsb.collect_hosts(hosts, list(hosts.hosts.values()), isolated)
    commands = [c[-1] for c in runner.calls if c[0] == "ssh"]
    assert len(commands) == 1 + 3 + 3
    prefix = f"sudo timeout -k 10 {awsb.COLLECT_HOST_TIMEOUT_S:.0f} bash -c "
    assert all(c.startswith(prefix) for c in commands), commands
    rsyncs = [c for c in runner.calls if c[0] == "rsync"]
    assert len(rsyncs) == 3
    assert all("--timeout=60" in c for c in rsyncs)


def test_a_timed_out_collect_script_still_copies_back_what_is_on_the_host(
    isolated: Path,
):
    runner = Scripted({"docker logs": [completed(returncode=124)]})
    hosts = remote(runner, isolated)
    awsb.collect_hosts(hosts, [hosts.hosts["node0"]], isolated)
    assert len([c for c in runner.calls if c[0] == "rsync"]) == 1


@pytest.mark.slow
@pytest.mark.parametrize(
    ("stderr", "rc", "expected"),
    [("Error: No such container: a", 1, 0), ("permission denied", 1, 1), ("", 0, 0)],
)
def test_freeze_command_ignores_only_a_missing_container(
    tmp_path: Path, stderr: str, rc: int, expected: int
):
    docker = tmp_path / "docker"
    docker.write_text(f"#!/bin/sh\necho '{stderr}' >&2\nexit {rc}\n")
    docker.chmod(0o755)
    env = {"PATH": f"{tmp_path}:{os.environ['PATH']}"}
    command = awsb.freeze_command(("a", "b"))
    result = subprocess.run(["bash", "-c", command], env=env, check=False)
    assert result.returncode == expected


def poll(runner: Scripted, tmp: Path, clock: FakeClock) -> Any:
    cfg = awsb.RunConfig(tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1))
    return awsb.poll_agent(remote(runner, tmp), tmp, cfg, awsb.Interrupts(clock))


def agent_state(state: dict) -> subprocess.CompletedProcess:
    return completed(stdout=json.dumps(state))


# REQ:awsbench-poll-tolerant
def test_ssh_blips_are_retried(isolated: Path, clock: FakeClock):
    blip = completed(returncode=255, stderr="timed out")
    runner = Scripted(
        {
            "is-active": [blip, blip, completed()],
            "agent-state.json": [agent_state(DONE_STATE)],
        }
    )
    assert poll(runner, isolated, clock)["phase"] == "done"
    assert clock.sleeps == [awsb.AGENT_POLL_S] * 2


# TEST:poll-agent-deadline-fails
def test_an_agent_that_never_finishes_hits_the_deadline(isolated: Path):
    runner = Scripted({"agent-state.json": [agent_state(LOADING)]})
    with pytest.raises(awsb.RemoteError, match="did not finish within"):
        poll(runner, isolated, FakeClock(limit_s=1e6))


# TEST:poll-step-unreachable-fails
def test_persistent_ssh_failure_raises(
    isolated: Path, clock: FakeClock, monkeypatch: pytest.MonkeyPatch
):
    monkeypatch.setattr(awsb, "AGENT_POLL_SSH_FAILURES_MAX", 2)
    runner = Scripted({"is-active": [completed(returncode=255, stderr="down")]})
    with pytest.raises(awsb.RemoteError, match="unreachable for 3 polls"):
        poll(runner, isolated, clock)


@pytest.mark.parametrize("rc", [awsb.SYSTEMCTL_INACTIVE_RC, awsb.SYSTEMCTL_NO_UNIT_RC])
def test_an_agent_exit_without_done_raises(isolated: Path, clock: FakeClock, rc: int):
    runner = Scripted(
        {
            "is-active": [completed(returncode=rc)],
            "agent-state.json": [agent_state(LOADING)],
        }
    )
    with pytest.raises(awsb.RemoteError, match="exited in phase loading without"):
        poll(runner, isolated, clock)


def test_collected_agent_unit_with_done_state_returns(isolated: Path, clock: FakeClock):
    runner = Scripted(
        {
            "is-active": [completed(returncode=awsb.SYSTEMCTL_NO_UNIT_RC)],
            "agent-state.json": [agent_state(DONE_STATE)],
        }
    )
    assert poll(runner, isolated, clock)["phase"] == "done"


def test_a_failed_partial_rsync_does_not_fail_the_poll(
    isolated: Path, clock: FakeClock
):
    polls = int(awsb.OUT_RSYNC_S / awsb.AGENT_POLL_S) + 1
    states = [agent_state(LOADING)] * polls + [agent_state(DONE_STATE)]
    runner = Scripted({"agent-state.json": states}, rsync_rc=23)
    assert poll(runner, isolated, clock)["phase"] == "done"
    assert any(c[0] == "rsync" for c in runner.calls)


BEGIN = 100.0


def poll_state(**overrides) -> Any:
    state = awsb.PollState(
        begin=BEGIN,
        deadline=BEGIN + 1000,
        next_log=BEGIN + awsb.AGENT_LOG_S,
        next_sync=BEGIN + awsb.OUT_RSYNC_S,
        unreachable=0,
        state=None,
    )
    return {**state, **overrides}


# TEST:poll-step-done-ok
def test_done_wins_over_an_inactive_unit_and_the_deadline():
    step = awsb.poll_step(
        poll_state(), awsb.SYSTEMCTL_NO_UNIT_RC, "", DONE_STATE, BEGIN + 5000
    )
    assert step["done"]


def test_a_reachable_probe_resets_the_unreachable_count():
    step = awsb.poll_step(poll_state(unreachable=3), 0, "", LOADING, BEGIN)
    assert step["state"]["unreachable"] == 0
    assert not step["retry"]


def test_unexpected_probe_rc_is_an_error():
    step = awsb.poll_step(poll_state(), 1, "odd\n", None, BEGIN)
    assert step["error"] == "systemctl is-active exited 1: odd"


def test_no_state_file_after_the_grace_is_an_error():
    grace = BEGIN + awsb.AGENT_START_GRACE_S
    assert awsb.poll_step(poll_state(), 0, "", None, grace)["error"] is None
    after = awsb.poll_step(poll_state(), 0, "", None, grace + 1)
    assert after["error"] == "agent wrote no state file"


# TEST:retry-verdict-ok
# TEST:retry-verdict-timeout-fails
@pytest.mark.parametrize(
    ("ok", "now", "verdict"),
    [
        (True, 0.0, "ok"),
        (True, 99.0, "ok"),
        (False, 0.0, "retry"),
        (False, 9.9, "retry"),
        (False, 10.0, "timeout"),
        (False, 11.0, "timeout"),
    ],
)
def test_retry_verdict(ok: bool, now: float, verdict: str):
    assert awsb.retry_verdict(ok, now, 10.0) == verdict


def wait_cloud_init(tmp: Path, rc: int, status: str) -> None:
    runner = Scripted(
        {
            "--wait": [completed(returncode=rc, stderr="err")],
            "--format json": [completed(stdout=json.dumps({"status": status}))],
        }
    )
    awsb.wait_cloud_init(remote(runner, tmp), "ctl")


@pytest.mark.parametrize("rc", [0, 2], ids=["clean", "recoverable"])
def test_cloud_init_done_is_accepted(isolated: Path, rc: int):
    wait_cloud_init(isolated, rc, "done")


@pytest.mark.parametrize(
    ("rc", "status", "match"), [(1, "error", "exited 1"), (2, "degraded", "'degraded'")]
)
def test_cloud_init_failure_raises(isolated: Path, rc: int, status: str, match: str):
    with pytest.raises(awsb.RemoteError, match=match):
        wait_cloud_init(isolated, rc, status)


@pytest.mark.parametrize(
    ("result", "match"),
    [
        (completed(returncode=124), f"still running after {awsb.DEPLOY_TIMEOUT_S} s"),
        (completed(stdout="1\n"), "deploy exited with status 1"),
    ],
    ids=["timeout", "status"],
)
def test_a_failed_deploy_raises(
    isolated: Path, clock: FakeClock, result: subprocess.CompletedProcess, match: str
):
    runner = Scripted({"docker wait deploy": [result]})
    with pytest.raises(awsb.RemoteError, match=match):
        awsb.start_support(remote(runner, isolated), [], awsb.Interrupts(clock))


# TEST:support-plan-order-ok
@pytest.mark.parametrize(
    ("query_db", "postgres"), [("colocated", "postgres"), ("rds", None)]
)
def test_support_containers_start_in_order_and_postgres_last(
    query_db: str, postgres: str | None
):
    containers = [step["container"] for step in awsb.support_plan(query_db, ["0xabc"])]
    assert containers == [
        "anvil",
        "deploy",
        None,
        "orchestrator",
        "state-relay-server",
        postgres,
        None,
        None,
    ]


def test_postgres_is_ready_before_its_extension_and_the_stats_reset():
    plan = awsb.support_plan("colocated", [])
    assert [step["what"] for step in plan][-3:] == [
        "pg_isready",
        "pg_stat_statements",
        "pg stats reset",
    ]
    gate = plan[-3]["gate_cmd"]
    assert gate.startswith("sudo bash -c ")
    assert awsb.PG_JSON in gate
    assert "PGPASSWORD=password" not in gate


def test_every_contract_is_checked_between_deploy_and_orchestrator():
    plan = awsb.support_plan("colocated", ["0xabc", "0xdef"])
    assert [step["what"] for step in plan][1:6] == [
        "deploy",
        "eth_getCode 0xabc",
        "eth_getCode 0xdef",
        "orchestrator healthcheck",
        "relay healthcheck",
    ]
    check = plan[3]
    assert check["kind"] == "gate"
    assert "0xdef" in check["gate_cmd"]


def test_start_support_starts_the_planned_containers_in_order(
    isolated: Path, clock: FakeClock
):
    runner = Scripted({"docker wait deploy": [completed(stdout="0\n")]})
    hosts = remote(runner, isolated)
    awsb.start_support(hosts, ["0xabc"], awsb.Interrupts(clock), "rds")
    commands = [c[-1] for c in runner.calls if c[0] == "ssh"]
    started = [
        c.rpartition("docker start ")[2] for c in commands if "docker start" in c
    ]
    assert started == ["anvil", "deploy", "orchestrator", "state-relay-server"]


# TEST:collect-plan-skip-ok
@pytest.mark.parametrize(
    ("error", "agent_started", "query_db", "plan"),
    [
        (
            None,
            True,
            "colocated",
            ["freeze", "collect_hosts", "rsync_out", "ebs_balance"],
        ),
        (None, False, "colocated", ["freeze", "collect_hosts", "ebs_balance"]),
        (
            "boom",
            True,
            "volume",
            ["stop_agent", "freeze", "collect_hosts", "rsync_out", "ebs_balance"],
        ),
        (
            None,
            True,
            "rds",
            [
                "freeze",
                "collect_hosts",
                "rsync_out",
                "ebs_balance",
                "rds_metrics",
                "rds_logs",
            ],
        ),
    ],
    ids=["clean", "no-agent", "error", "rds"],
)
def test_collect_plan(
    error: str | None, agent_started: bool, query_db: str, plan: list[str]
):
    assert awsb.collect_plan(error, agent_started, query_db) == plan


def skip_collect(interrupts: Any) -> None:
    interrupts.skip_collect.set()


def no_signal(interrupts: Any) -> None:
    pass


@pytest.mark.parametrize(
    ("on_freeze", "ran", "skipped"),
    [
        (skip_collect, ["freeze"], ["collect", "final rsync", "node0 EBS balance"]),
        (no_signal, ["freeze", "collect", "final rsync", "ebs balance"], []),
    ],
    ids=["signal", "no-signal"],
)
def test_skip_collect_is_checked_before_every_collect_step(
    isolated: Path,
    clock: FakeClock,
    monkeypatch: pytest.MonkeyPatch,
    caplog: pytest.LogCaptureFixture,
    on_freeze: Callable[[Any], None],
    ran: list[str],
    skipped: list[str],
):
    interrupts = awsb.Interrupts(clock)
    fleet = awsb.FleetState(
        FakeSystem(),
        awsb.RunConfig(tag="x"),
        isolated,
        None,
        interrupts,
        remote(Scripted({}), isolated),
    )
    steps: list[str] = []

    def step(name: str, after: Callable[[], None] = lambda: None):
        def action(*args, **kwargs):
            steps.append(name)
            after()

        return action

    monkeypatch.setattr(awsb, "update_manifest", step("manifest"))
    monkeypatch.setattr(awsb, "report", step("report"))
    monkeypatch.setattr(awsb, "freeze", step("freeze", lambda: on_freeze(interrupts)))
    monkeypatch.setattr(awsb, "collect_hosts", step("collect"))
    monkeypatch.setattr(awsb.Remote, "rsync_from", step("final rsync"))
    monkeypatch.setattr(awsb, "collect_node0_ebs_balance", step("ebs balance"))
    with caplog.at_level(logging.WARNING, awsb.log.name):
        awsb.finish_run(awsb.Run(fleet, isolated, fleet.cfg, 0.0, agent_started=True))
    assert steps == ["manifest", *ran, "report"]
    assert caplog.messages == [f"{what} skipped" for what in skipped]


# TEST:sweep-plan-order-ok
def test_instances_terminate_before_volumes_security_group_and_key():
    arns = [
        "arn:aws:ec2:eu-west-1:1:key-pair/key-1",
        "arn:aws:ec2:eu-west-1:1:security-group/sg-1",
        "arn:aws:ec2:eu-west-1:1:volume/vol-1",
        "arn:aws:rds:eu-west-1:1:pg:espresso-bench-f",
        "arn:aws:rds:eu-west-1:1:subgrp:espresso-bench-f",
        "arn:aws:rds:eu-west-1:1:db:espresso-bench-f",
        "arn:aws:ec2:eu-west-1:1:instance/i-1",
        "arn:aws:scheduler:eu-west-1:1:schedule-group/espresso-bench-f",
    ]
    verbs = [
        " ".join(step["args"][:2]) if step["action"] == "aws" else step["action"]
        for step in awsb.sweep_plan(arns)
    ]
    assert verbs == [
        "scheduler delete-schedule-group",
        "ec2 terminate-instances",
        "ec2 wait",
        "delete_rds",
        "rds delete-db-subnet-group",
        "rds delete-db-parameter-group",
        "ec2 delete-volume",
        "delete_security_group",
        "ec2 delete-key-pair",
    ]
