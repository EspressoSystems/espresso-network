"""Tests for the fleet lifecycle of `aws-bench`: `up`, `run --fleet`, `extend` and `down`.

Reuses the fake runner and fixtures of `test_aws_bench`.

    just py::test
"""

import contextlib
import io
import json
import os
import shutil
import subprocess
import tempfile
import unittest
import unittest.mock
from dataclasses import replace
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import ClassVar

import netbench
from test_aws_bench import (
    DESCRIBE,
    DONE_STATE,
    STATUS_DESCRIBE,
    FleetRunner,
    awsb,
    completed,
    fake_images,
    fake_preflight,
    valid_result,
)

GENESIS = Path(__file__).with_name("genesis.toml")


class TaggingRunner(FleetRunner):
    """Adds `arns` more tagged resources than the three of `FleetRunner`, and `tag-resources`."""

    def __init__(self, *args, arns: int = 0, **kwargs):
        super().__init__(*args, **kwargs)
        self.arns = arns

    def aws(self, argv: list[str]) -> subprocess.CompletedProcess:
        if "get-resources" in argv:
            extra = [
                f"arn:aws:ec2:eu-west-1:1:volume/vol-{i:03d}" for i in range(self.arns)
            ]
            return completed(
                stdout=json.dumps([*json.loads(super().aws(argv).stdout), *extra])
            )
        if "tag-resources" in argv:
            return completed(stdout=json.dumps({"FailedResourcesMap": {}}))
        return super().aws(argv)


def fake_report(run_dir: Path, baseline=None) -> dict:
    result = valid_result()
    netbench.write_json(run_dir / "result.json", result)
    return result


class FleetHarness:
    """A temp out-root and ssh key, and the argv of `up` and `run --fleet` for one fleet."""

    def __init__(self, test: unittest.TestCase, name: str = "fleet1"):
        self.tmp = Path(tempfile.mkdtemp())
        test.addCleanup(shutil.rmtree, self.tmp)
        (self.tmp / "key").write_text("private")
        (self.tmp / "key.pub").write_text("ssh-ed25519 AAAA test")
        self.name = name
        self.out = self.tmp / "out"
        self.fleet_dir = self.out / name
        patches = [
            unittest.mock.patch.multiple(
                awsb,
                SSH_RETRY_S=0.0,
                AGENT_POLL_S=0.0,
                GATE_RETRY_S=0.0,
                DESTROY_BACKOFF_S=0.0,
                DOCKER_START_LEAD_S=0.0,
            ),
            unittest.mock.patch.object(
                awsb, "preflight", return_value=fake_preflight()
            ),
            unittest.mock.patch.object(awsb, "write_report", fake_report),
        ]
        for patch in patches:
            patch.start()
            test.addCleanup(patch.stop)

    def parse(self, *argv: str) -> "awsb.argparse.Namespace":
        args = awsb.parse_args(list(argv))
        args.argv = list(argv)
        return args

    def fleet_flags(self) -> list[str]:
        return [
            "--tag",
            "t",
            "--nodes",
            "2",
            "--name",
            self.name,
            "--out-root",
            str(self.out),
            "--ssh-key",
            str(self.tmp / "key"),
            "--operator-cidr",
            "203.0.113.5/32",
            "--price",
            "c8g.4xlarge=0.71",
            "--price",
            "c8g.2xlarge=0.355",
            "--yes",
        ]

    def up_args(self, *extra: str) -> "awsb.argparse.Namespace":
        return self.parse("up", *self.fleet_flags(), *extra)

    def single_shot_args(self) -> "awsb.argparse.Namespace":
        return self.parse("run", "--genesis", str(GENESIS), *self.fleet_flags())

    def run_args(self, *extra: str) -> "awsb.argparse.Namespace":
        return self.parse(
            "run",
            "--fleet",
            str(self.fleet_dir),
            "--genesis",
            str(GENESIS),
            "--yes",
            *extra,
        )

    def up(self, runner, *extra: str) -> int:
        return awsb.cmd_up(
            self.up_args(*extra), run=runner, interrupts=awsb.Interrupts()
        )

    def run(self, runner, *extra: str, interrupts: "awsb.Interrupts | None" = None):
        return awsb.cmd_run(
            self.run_args(*extra),
            run=runner,
            interrupts=interrupts or awsb.Interrupts(),
        )

    def fleet(self) -> dict:
        return json.loads((self.fleet_dir / "fleet.json").read_text())

    def set_fleet(self, **changes) -> None:
        netbench.write_json(self.fleet_dir / "fleet.json", {**self.fleet(), **changes})

    def index(self) -> list[str]:
        return (self.out / "INDEX.md").read_text().splitlines()[2:]

    def lock(self) -> Path:
        return self.fleet_dir / awsb.FLEET_LOCK

    def driver_log(self) -> str:
        return (self.fleet_dir / "driver.log").read_text()

    def up_fleet(self, test: unittest.TestCase) -> TaggingRunner:
        runner = TaggingRunner([DONE_STATE], describe=DESCRIBE)
        test.assertEqual(self.up(runner), awsb.EXIT_OK)
        return runner


def ssh_calls(runner: FleetRunner, since: int = 0) -> list[str]:
    return [" ".join(call) for call in runner.calls[since:] if call[0] == "ssh"]


# REQ:fleet-up-idle
class UpTest(unittest.TestCase):
    def test_up_ends_idle_without_starting_anything(self):
        harness = FleetHarness(self)
        runner = TaggingRunner([DONE_STATE])
        self.assertEqual(harness.up(runner), awsb.EXIT_OK)
        manifest = harness.fleet()
        self.assertEqual(manifest["phase"], "idle")
        self.assertEqual(manifest["db_modes"], ["colocated"])
        for name in ("hosts.json", "terraform/plan.txt", "hosts/ctl/ready.json"):
            self.assertTrue((harness.fleet_dir / name).exists(), name)
        self.assertTrue((harness.fleet_dir / "ssh").is_dir())
        self.assertFalse((harness.fleet_dir / "runs").exists())
        commands = ssh_calls(runner)
        self.assertFalse(any("docker start" in c or "start.sh" in c for c in commands))
        self.assertFalse(runner.ran("agent-drive"))
        self.assertFalse(runner.ran("tofu", "destroy"))
        self.assertFalse(harness.lock().exists())

    def test_up_prints_cost_and_the_run_command(self):
        harness = FleetHarness(self)
        harness.up(TaggingRunner([DONE_STATE]))
        log = harness.driver_log()
        self.assertRegex(
            log,
            r"cost: fleet \$\d+\.\d\d/h, bound \$\d+\.\d\d at TTL 180 min "
            r"\(self-terminate \d\d:\d\d UTC\), limit \$60\.00",
        )
        self.assertIn(f"idle: fleet {harness.fleet_dir}; ", log)
        self.assertIn(
            f"run: aws-bench run --fleet {harness.fleet_dir} --query-db colocated", log
        )

    def test_expiry_counts_from_the_confirm_without_a_provision_margin(self):
        harness = FleetHarness(self)
        before = datetime.now(UTC).replace(microsecond=0)
        harness.up(TaggingRunner([DONE_STATE]))
        manifest = harness.fleet()
        expires = datetime.fromisoformat(manifest["expires_at"])
        self.assertGreaterEqual(expires, before + timedelta(minutes=180))
        self.assertLess(expires, before + timedelta(minutes=181))
        tfvars = json.loads(
            (harness.fleet_dir / "terraform" / "terraform.tfvars.json").read_text()
        )
        self.assertEqual(tfvars["expires_at"], manifest["expires_at"])
        user_data = (harness.fleet_dir / "hosts" / "ctl" / "user-data.sh").read_text()
        self.assertIn("shutdown -P +180", user_data)

    def test_fleet_bound_is_the_rate_over_the_ttl_plus_boot(self):
        harness = FleetHarness(self)
        harness.up(TaggingRunner([DONE_STATE]))
        estimate = harness.fleet()["estimate"]
        self.assertEqual(estimate["ttl_s"], 180 * 60)
        rate = awsb.estimate_rate(estimate)
        self.assertGreater(rate, 0.355 + 2 * 0.71)
        egress = next(l["usd"] for l in estimate["lines"] if l["item"] == "egress")
        expected = rate * (180 * 60 + awsb.BOOT_ALLOWANCE_S) / 3600 + egress
        self.assertAlmostEqual(estimate["bound_usd"], expected)

    def test_apply_failure_destroys_and_exits_3(self):
        harness = FleetHarness(self)
        runner = TaggingRunner(
            [DONE_STATE],
            apply=completed(returncode=1, stderr="Error: InsufficientInstanceCapacity"),
        )
        self.assertEqual(harness.up(runner), awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertIn("InsufficientInstanceCapacity", harness.driver_log())
        self.assertRegex(harness.index()[-1], r"^\| fleet1 \| .* \| 0 \| .* \| 3 \|$")

    def test_apply_failure_with_a_failed_destroy_exits_4(self):
        harness = FleetHarness(self)
        runner = TaggingRunner(
            [DONE_STATE],
            apply=completed(returncode=1, stderr="Error: boom"),
            destroys=[completed(returncode=1, stderr="locked")],
        )
        self.assertEqual(harness.up(runner), awsb.EXIT_LEFTOVER)
        last = harness.driver_log().splitlines()[-1]
        self.assertTrue(last.endswith(f"aws-bench destroy {harness.fleet_dir}"))

    def test_declined_prompt_creates_nothing(self):
        harness = FleetHarness(self)
        runner = TaggingRunner([DONE_STATE])
        args = harness.up_args()
        args.yes = False
        with self.assertRaisesRegex(awsb.Refused, "not confirmed"):
            awsb.cmd_up(args, run=runner, interrupts=awsb.Interrupts())
        self.assertFalse(runner.ran("tofu", "apply"))

    def test_up_needs_explicit_minutes(self):
        harness = FleetHarness(self)
        runner = TaggingRunner([DONE_STATE])
        with self.assertRaisesRegex(awsb.Refused, "--ttl-min"):
            harness.up(runner, "--ttl-min", "auto")
        self.assertFalse(runner.ran("tofu", "apply"))

    def test_bound_above_max_usd_is_refused(self):
        harness = FleetHarness(self)
        runner = TaggingRunner([DONE_STATE])
        with self.assertRaisesRegex(awsb.Refused, "exceeds --max-usd"):
            harness.up(runner, "--max-usd", "1")
        self.assertFalse(runner.ran("tofu", "apply"))


# REQ:fleet-run-on-fleet
class RunOnFleetTest(unittest.TestCase):
    def test_two_runs_reset_between_and_leave_the_fleet_idle(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        for verb, name in (("first", "01-first"), ("second", "02-second")):
            mark = len(runner.calls)
            self.assertEqual(harness.run(runner, "--name", verb), awsb.EXIT_OK)
            commands = ssh_calls(runner, mark)
            reset = next(i for i, c in enumerate(commands) if "find /data/journal" in c)
            start = next(i for i, c in enumerate(commands) if "docker start anvil" in c)
            self.assertLess(reset, start)
            run_dir = harness.fleet_dir / "runs" / name
            self.assertTrue((run_dir / "manifest.json").exists(), name)
            self.assertTrue((run_dir / "cost.json").exists(), name)
            self.assertFalse(harness.lock().exists())
            self.assertEqual(harness.fleet()["phase"], "idle")
            manifest = json.loads((run_dir / "manifest.json").read_text())
            self.assertEqual(manifest["phase"], "done")
            self.assertEqual(manifest["fleet"], "fleet1")
            self.assertEqual(manifest["query_db"], "colocated")
            self.assertEqual(manifest["index"], int(name[:2]))
            self.assertIn("run_estimate", manifest)
        rows = harness.index()
        self.assertEqual(len(rows), 2)
        self.assertTrue(rows[0].startswith("| fleet1/01-first |"))
        self.assertTrue(rows[1].startswith("| fleet1/02-second |"))
        self.assertFalse(runner.ran("tofu", "destroy"))

    def test_the_run_name_defaults_to_the_database_mode(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.run(runner)
        harness.run(runner)
        runs = harness.fleet_dir / "runs"
        self.assertEqual(
            [p.name for p in sorted(runs.glob("*"))], ["01-colocated", "02-colocated"]
        )

    def test_run_prints_its_incremental_cost_and_the_time_left(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.run(runner)
        log = harness.driver_log()
        self.assertRegex(
            log,
            r"run 01-colocated: incremental cost \$\d+\.\d\d \(\d+ min worst\), "
            r"\d+ min left on the fleet",
        )
        self.assertRegex(log, r"run dir .*01-colocated; fleet idle, \d+ min left")

    def test_invalid_result_exits_1_and_the_fleet_stays_up(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result(valid=False)
        ):
            self.assertEqual(harness.run(runner), awsb.EXIT_INVALID)
        self.assertEqual(harness.fleet()["phase"], "idle")
        self.assertFalse(harness.lock().exists())
        self.assertFalse(runner.ran("tofu", "destroy"))

    def test_agent_error_exits_3_collects_and_leaves_the_fleet_idle(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        runner.states = [{"phase": "error", "detail": "x", "error": "boom"}]
        runner.polls = 0
        self.assertEqual(harness.run(runner), awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("systemctl stop bench-agent"))
        self.assertTrue(runner.ran("rsync", "/opt/bench/out/"))
        self.assertFalse(runner.ran("tofu", "destroy"))
        self.assertEqual(harness.fleet()["phase"], "idle")
        self.assertFalse(harness.lock().exists())
        summary = harness.fleet_dir / "runs" / "01-colocated" / "summary.md"
        self.assertIn("boom", summary.read_text())

    # EDGE:fleet-run-interrupted
    def test_interrupt_collects_without_destroy_and_releases_the_lock(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        interrupts = awsb.Interrupts()

        def sigint(polls: int) -> None:
            if polls == 2:
                interrupts.event.set()

        runner.states = [{"phase": "loading", "detail": "x"}]
        runner.polls = 0
        runner.on_poll = sigint
        self.assertEqual(harness.run(runner, interrupts=interrupts), awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("systemctl stop bench-agent"))
        self.assertTrue(runner.ran("rsync", "/opt/bench/out/"))
        self.assertFalse(runner.ran("tofu", "destroy"))
        self.assertEqual(harness.fleet()["phase"], "idle")
        self.assertFalse(harness.lock().exists())

    # EDGE:fleet-reset-fails
    def test_failed_reset_leaves_the_fleet_dirty_and_locked(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        original = runner.ssh

        def failing(command: str) -> subprocess.CompletedProcess:
            if "find /data/journal" in command:
                return completed(returncode=255, stderr="unreachable")
            return original(command)

        with unittest.mock.patch.object(runner, "ssh", side_effect=failing):
            self.assertEqual(harness.run(runner), awsb.EXIT_FAILED)
        self.assertEqual(harness.fleet()["phase"], "dirty")
        self.assertTrue(harness.lock().exists())
        self.assertFalse(runner.ran("docker start"))
        self.assertFalse(runner.ran("tofu", "destroy"))
        self.assertRegex(harness.index()[-1], r"\| failed \| 3 \|")
        mark = len(runner.calls)
        dead = unittest.mock.patch.object(
            awsb.os, "kill", side_effect=ProcessLookupError
        )
        with dead, self.assertRaisesRegex(awsb.Refused, r"--force.*down"):
            harness.run(runner)
        self.assertEqual(ssh_calls(runner, mark), [])
        with dead:
            self.assertEqual(harness.run(runner, "--force"), awsb.EXIT_OK)
        self.assertEqual(harness.fleet()["phase"], "idle")
        self.assertFalse(harness.lock().exists())

    # EDGE:fleet-stale-running
    def test_stale_running_fleet_needs_force(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.set_fleet(phase="running")
        harness.lock().write_text(
            json.dumps(
                {
                    "pid": 999999,
                    "hostname": awsb.socket.gethostname(),
                    "run": "old",
                    "taken_at": "2026-09-30T10:00:00+00:00",
                }
            )
        )
        mark = len(runner.calls)
        dead = unittest.mock.patch.object(
            awsb.os, "kill", side_effect=ProcessLookupError
        )
        with (
            dead,
            self.assertRaisesRegex(awsb.Refused, r"pid 999999 \(dead\).*--force"),
        ):
            harness.run(runner)
        self.assertEqual(ssh_calls(runner, mark), [])
        with dead:
            code = harness.run(runner, "--force")
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertTrue(any("find /data/journal" in c for c in ssh_calls(runner, mark)))
        self.assertEqual(harness.fleet()["phase"], "idle")
        self.assertFalse(harness.lock().exists())

    def test_force_does_not_replace_a_live_holder(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.set_fleet(phase="running")
        awsb.take_fleet_lock(harness.fleet_dir, "other", False)
        with self.assertRaisesRegex(awsb.Refused, r"cannot --force.*\(alive\)"):
            harness.run(runner, "--force")

    # REQ:fleet-single-shot-unchanged
    def test_single_shot_does_not_reset_and_writes_both_index_rows(self):
        harness = FleetHarness(self)
        runner = TaggingRunner([DONE_STATE], describe=DESCRIBE)
        args = harness.single_shot_args()
        code = awsb.cmd_run(args, run=runner, interrupts=awsb.Interrupts())
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertFalse(runner.ran("find /data/journal"))
        self.assertEqual(harness.fleet()["db_modes"], ["colocated"])
        run_row, fleet_row = harness.index()
        self.assertIn("| colocated |", run_row)
        self.assertTrue(fleet_row.startswith("| fleet1 |"))
        self.assertFalse(harness.lock().exists())


# REQ:fleet-run-refusals
class RunRefusalTest(unittest.TestCase):
    def refused(self, harness: FleetHarness, runner, pattern: str, *extra: str) -> None:
        mark = len(runner.calls)
        with self.assertRaisesRegex(awsb.Refused, pattern):
            harness.run(runner, *extra)
        self.assertEqual(ssh_calls(runner, mark), [])
        self.assertFalse(harness.lock().exists())

    def test_phase_not_idle(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.set_fleet(phase="destroying")
        self.refused(harness, runner, "is destroying, not idle")

    def test_lock_held(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        awsb.take_fleet_lock(harness.fleet_dir, "other", False)
        mark = len(runner.calls)
        with self.assertRaisesRegex(
            awsb.Refused, r"locked: run other, pid \d+ \(alive\)"
        ):
            harness.run(runner)
        self.assertEqual(ssh_calls(runner, mark), [])
        self.assertTrue(harness.lock().exists())

    def test_mode_missing(self):
        harness = FleetHarness(self)
        harness.up_fleet(self)
        manifest = harness.fleet()
        cfg = awsb.fleet_run_config(harness.run_args(), manifest)
        cfg = awsb.dataclasses.replace(cfg, query_db="volume")
        with self.assertRaisesRegex(awsb.Refused, "was not provisioned"):
            awsb.check_run_allowed(manifest, cfg, datetime.now(UTC))

    def test_ttl_too_short_names_extend_with_the_minutes(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        expires = datetime.now(UTC) + timedelta(minutes=10)
        harness.set_fleet(expires_at=awsb.expiry_stamp(expires))
        self.refused(harness, runner, rf"extend {harness.fleet_dir} --ttl-min \d\d")

    def test_the_lockout_window_counts_against_the_ttl(self):
        harness = FleetHarness(self)
        harness.up_fleet(self)
        manifest = harness.fleet()
        cfg = awsb.fleet_run_config(harness.run_args(), manifest)
        worst = awsb.estimate_run(manifest, cfg)["worst_s"]
        now = datetime.now(UTC)
        without_lockout = now + timedelta(seconds=worst + awsb.DESTROY_S + 60)
        with self.assertRaisesRegex(awsb.Refused, "extend"):
            awsb.check_run_allowed(
                {**manifest, "expires_at": awsb.expiry_stamp(without_lockout)},
                cfg,
                now,
            )
        enough = without_lockout + timedelta(seconds=awsb.NOLOGIN_LEAD_S)
        awsb.check_run_allowed(
            {**manifest, "expires_at": awsb.expiry_stamp(enough)}, cfg, now
        )

    # EDGE:fleet-ttl-expired
    def test_expired_fleet_names_status_and_orphans(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        past = datetime.now(UTC) - timedelta(minutes=5)
        harness.set_fleet(expires_at=awsb.expiry_stamp(past))
        self.refused(harness, runner, r"expired at .*status.*destroy --orphans")

    def test_fleet_flags_are_refused(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        for flags in (
            ("--nodes", "3"),
            ("--max-usd=5",),
            ("--ttl-min", "10"),
            ("--keep",),
        ):
            self.refused(harness, runner, r"shape the fleet", *flags)

    # EDGE:fleet-run-name-collision
    def test_run_name_collision(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        self.assertEqual(harness.run(runner, "--name", "same"), awsb.EXIT_OK)
        self.refused(harness, runner, "already exists", "--name", "same")
        self.assertEqual(len(list((harness.fleet_dir / "runs").glob("*"))), 1)

    def test_tag_must_match_the_fleet(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        self.refused(harness, runner, "up a new fleet", "--tag", "other")
        self.assertEqual(harness.run(runner, "--tag", "t"), awsb.EXIT_OK)

    def test_unknown_dir_is_not_a_fleet(self):
        harness = FleetHarness(self)
        with self.assertRaisesRegex(awsb.Refused, "not a fleet dir"):
            harness.run(TaggingRunner([DONE_STATE]))

    def test_single_shot_needs_a_tag(self):
        harness = FleetHarness(self)
        args = harness.parse("run", "--nodes", "2")
        with self.assertRaisesRegex(awsb.Refused, "--tag"):
            awsb.cmd_run(args, run=TaggingRunner([DONE_STATE]))


class FleetLockTest(unittest.TestCase):
    def test_lock_is_exclusive_and_names_the_holder(self):
        with tempfile.TemporaryDirectory() as tmp:
            fleet_dir = Path(tmp)
            lock = awsb.take_fleet_lock(fleet_dir, "a", False)
            self.assertEqual(lock["pid"], os.getpid())
            self.assertEqual(json.loads((fleet_dir / "fleet.lock").read_text()), lock)
            with self.assertRaisesRegex(awsb.Refused, "run a, pid"):
                awsb.take_fleet_lock(fleet_dir, "b", False)
            awsb.release_fleet_lock(fleet_dir)
            awsb.take_fleet_lock(fleet_dir, "b", False)

    def test_release_without_a_lock_is_fine(self):
        with tempfile.TemporaryDirectory() as tmp:
            awsb.release_fleet_lock(Path(tmp))

    def test_a_holder_on_another_host_counts_as_alive(self):
        lock = {"pid": 1, "hostname": "elsewhere", "run": "r", "taken_at": "t"}
        self.assertTrue(awsb.lock_holder_alive(lock))


class ResetScriptTest(unittest.TestCase):
    def test_every_host_stops_units_removes_containers_and_wipes_the_journal(self):
        script = awsb.reset_script("validator")
        self.assertTrue(script.startswith("set -eu\n"))
        for needle in (
            "systemctl stop $unit",
            "docker stop -t 30 $ids",
            "docker rm -f $ids",
            "find /data/journal -mindepth 1 -delete",
            "find /opt/bench -mindepth 1 -maxdepth 1",
        ):
            self.assertIn(needle, script)
        for kept in awsb.RESET_KEEP:
            self.assertIn(f"! -name {kept}", script)
        self.assertNotIn("/data/pg", script)

    def test_the_query_host_also_wipes_postgres(self):
        self.assertIn("find /data/pg -mindepth 1 -delete", awsb.reset_script("query"))

    def test_reset_chain_runs_the_script_on_every_host(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        fleet = awsb.open_fleet(runner, harness.fleet_dir, awsb.Interrupts())
        assert fleet.remote is not None
        mark = len(runner.calls)
        awsb.reset_chain(fleet.remote, "colocated", fleet.manifest, fleet.interrupts)
        commands = ssh_calls(runner, mark)
        self.assertEqual(len(commands), 3)
        self.assertEqual(sum("find /data/pg" in c for c in commands), 1)
        self.assertTrue(all("sudo timeout" in c for c in commands))


VOLUME_ID = "vol-0abc123def456"
BY_ID = "/dev/disk/by-id/nvme-Amazon_Elastic_Block_Store_vol0abc123def456"
NODE0_IP = "203.0.113.2"


class VolumeRunner(TaggingRunner):
    """Adds the `pg_volume_id` output of a fleet with a Postgres volume."""

    def __init__(self, *args, volume_id: str | None = VOLUME_ID, **kwargs):
        super().__init__(*args, **kwargs)
        self.volume_id = volume_id

    def tofu(self, verb: str) -> subprocess.CompletedProcess:
        result = super().tofu(verb)
        if verb != "output":
            return result
        return completed(
            stdout=json.dumps(
                {**json.loads(result.stdout), "pg_volume_id": {"value": self.volume_id}}
            )
        )


class MissingDeviceRunner(VolumeRunner):
    def ssh(self, command: str) -> subprocess.CompletedProcess:
        if "mkfs.ext4" in command:
            return completed(returncode=1, stderr=f"{BY_ID} missing after 60 s")
        return super().ssh(command)


def node0_calls(runner: FleetRunner, since: int = 0) -> list[str]:
    return [c for c in ssh_calls(runner, since) if NODE0_IP in c]


# REQ:querydb-volume-wiring
class VolumeWiringTest(unittest.TestCase):
    def test_by_id_path_drops_the_dash_of_the_volume_id(self):
        self.assertEqual(awsb.volume_device(VOLUME_ID), BY_ID)

    def test_tfvars_carry_the_volume_only_when_the_mode_is_provisioned(self):
        def tfvars(cfg: "awsb.RunConfig") -> dict:
            return awsb.render_tfvars(
                cfg,
                Path("/tmp/f"),
                "f",
                "alice",
                awsb.plan_hosts(cfg),
                "k",
                "203.0.113.5/32",
                "2026-01-01T00:00:00Z",
                "abc1234",
                "eu-west-1a",
                "ami-0abc",
            )

        volume = awsb.RunConfig(
            tag="x",
            nodes=2,
            db_modes=("colocated", "volume"),
            pg_gb=300,
            load=netbench.BenchConfig(submit_nodes=1),
        )
        self.assertEqual(
            tfvars(volume)["pg_volume"], {"gb": 300, "iops": 12000, "mbps": 500}
        )
        self.assertIsNone(tfvars(replace(volume, db_modes=("colocated",)))["pg_volume"])

    def test_up_makes_and_mounts_the_volume_on_the_query_host_only(self):
        harness = FleetHarness(self)
        runner = VolumeRunner([DONE_STATE])
        self.assertEqual(harness.up(runner, "--db-modes", "volume"), awsb.EXIT_OK)
        manifest = harness.fleet()
        self.assertEqual(manifest["phase"], "idle")
        self.assertEqual(manifest["pg_volume_id"], VOLUME_ID)
        self.assertEqual(manifest["pg_volume"], {"gb": 400, "iops": 12000, "mbps": 500})
        formats = [c for c in ssh_calls(runner) if "mkfs.ext4" in c]
        self.assertEqual(len(formats), 1)
        self.assertIn(NODE0_IP, formats[0])
        for needle in (
            f"mkfs.ext4 -q -F -E lazy_itable_init=0,lazy_journal_init=0 {BY_ID}",
            "seq 60",
            "udevadm settle",
            "findmnt -no OPTIONS /",
            f'mount -t ext4 -o "$(findmnt -no OPTIONS /)" {BY_ID} /data/pg',
        ):
            self.assertIn(needle, formats[0])
        self.assertLess(formats[0].index("mkfs.ext4"), formats[0].index("mount -t"))

    def test_a_single_shot_volume_run_formats_once_and_does_not_reset(self):
        harness = FleetHarness(self)
        runner = VolumeRunner([DONE_STATE], describe=DESCRIBE)
        args = harness.parse(
            "run",
            "--genesis",
            str(GENESIS),
            *harness.fleet_flags(),
            "--query-db",
            "volume",
        )
        code = awsb.cmd_run(args, run=runner, interrupts=awsb.Interrupts())
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertEqual(runner.count("mkfs.ext4"), 1)
        self.assertFalse(runner.ran("find /data/journal"))
        self.assertEqual(harness.fleet()["db_modes"], ["volume"])

    def test_up_without_the_volume_mode_issues_no_mkfs(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        self.assertFalse(runner.ran("mkfs"))
        self.assertNotIn("pg_volume", harness.fleet())

    def test_missing_device_destroys_and_exits_3(self):
        harness = FleetHarness(self)
        runner = MissingDeviceRunner([DONE_STATE], describe=DESCRIBE)
        self.assertEqual(harness.up(runner, "--db-modes", "volume"), awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertIn("missing after 60 s", harness.driver_log())

    def test_a_null_volume_output_fails_up(self):
        harness = FleetHarness(self)
        runner = VolumeRunner([DONE_STATE], volume_id=None, describe=DESCRIBE)
        self.assertEqual(harness.up(runner, "--db-modes", "volume"), awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertFalse(runner.ran("mkfs"))

    def test_the_run_resets_by_mounting_and_wiping_without_reformatting(self):
        harness = FleetHarness(self)
        runner = VolumeRunner([DONE_STATE], describe=DESCRIBE)
        harness.up(runner, "--db-modes", "volume")
        mark = len(runner.calls)
        self.assertEqual(harness.run(runner, "--query-db", "volume"), awsb.EXIT_OK)
        commands = ssh_calls(runner, mark)
        self.assertFalse(any("mkfs" in c for c in commands))
        reset = next(c for c in node0_calls(runner, mark) if "find /data/journal" in c)
        self.assertIn(f'mount -t ext4 -o "$(findmnt -no OPTIONS /)" {BY_ID}', reset)
        self.assertLess(reset.index("docker rm -f"), reset.index("mount -t"))
        self.assertLess(reset.index("mount -t"), reset.index("find /data/pg"))
        others = [c for c in ssh_calls(runner, mark) if "find /data/journal" in c]
        self.assertEqual(sum("mount -t" in c or "umount" in c for c in others), 1)
        run_dir = harness.fleet_dir / "runs" / "01-volume"
        manifest = json.loads((run_dir / "manifest.json").read_text())
        self.assertEqual(manifest["query_db"], "volume")
        self.assertEqual(manifest["pg_volume_id"], VOLUME_ID)

    def test_a_volume_run_needs_the_mode_provisioned(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        mark = len(runner.calls)
        with self.assertRaisesRegex(awsb.Refused, "--query-db volume was not"):
            harness.run(runner, "--query-db", "volume")
        self.assertEqual(ssh_calls(runner, mark), [])

    def test_the_pg_size_shapes_the_fleet(self):
        args = awsb.parse_args(["up", "--tag", "t", "--pg-gb", "500"])
        self.assertEqual(args.pg_gb, 500)
        with self.assertRaisesRegex(awsb.Refused, "--pg-gb"):
            awsb.reject_fleet_flags(["run", "--fleet", "d", "--pg-gb", "500"])

    def test_db_modes_accept_volume(self):
        self.assertEqual(
            awsb.parse_db_modes("colocated,volume"), ("colocated", "volume")
        )


# REQ:querydb-volume-wiring
class PgStoreScriptTest(unittest.TestCase):
    manifest: ClassVar = {"name": "f", "pg_volume_id": VOLUME_ID}

    def reset(self, mode: str) -> str:
        store = awsb.pg_store_script(mode, self.manifest)
        return awsb.reset_script("query", store)

    # TEST:querydb-mode-switch-ok
    def test_colocated_unmounts_after_the_containers_and_before_the_wipe(self):
        script = self.reset("colocated")
        self.assertNotIn("mount -t", script)
        unmount = script.index("umount /data/pg")
        self.assertIn("mountpoint -q /data/pg", script)
        self.assertLess(script.index("docker rm -f"), unmount)
        self.assertLess(unmount, script.index("find /data/pg"))

    def test_volume_mounts_only_when_not_mounted(self):
        script = self.reset("volume")
        self.assertIn(
            f"if ! mountpoint -q /data/pg; then\n  mount -t ext4 -o "
            f'"$(findmnt -no OPTIONS /)" {BY_ID} /data/pg\nfi\n',
            script,
        )
        self.assertNotIn("umount", script)

    def test_validators_never_touch_the_store(self):
        script = awsb.reset_script(
            "validator", awsb.pg_store_script("volume", self.manifest)
        )
        self.assertNotIn("/data/pg", script)

    def test_volume_without_a_volume_id_is_refused(self):
        with self.assertRaisesRegex(awsb.Refused, "no pg volume"):
            awsb.pg_store_script("volume", {"name": "f"})


# REQ:fleet-cost
class PgVolumeCostTest(unittest.TestCase):
    MINOR = awsb.MINOR_PRICES["eu-west-1"]

    def rate(self, volume: "awsb.VolumeSpec | None") -> float:
        hosts = awsb.plan_hosts(
            awsb.RunConfig(tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1))
        )
        prices = {
            "instances": {
                t: {"usd_hour": 1.0, "source": "test"}
                for t in ("c8g.4xlarge", "c8g.2xlarge")
            }
        }
        lines = awsb._cost_lines(hosts, prices, self.MINOR, 3600.0, volume)
        return sum(l["usd"] for l in lines if l["item"] != "egress")

    def test_the_volume_adds_size_iops_and_throughput_above_the_baseline(self):
        volume: awsb.VolumeSpec = {"gb": 400, "iops": 12000, "mbps": 500}
        expected = (
            400 * self.MINOR["gp3_gb_month_usd"]
            + 9000 * self.MINOR["gp3_iops_month_usd"]
            + 375 * self.MINOR["gp3_mbps_month_usd"]
        ) / awsb.HOURS_PER_MONTH
        self.assertAlmostEqual(self.rate(volume) - self.rate(None), expected)

    def test_a_baseline_volume_has_a_storage_line_only(self):
        lines = awsb.pg_volume_cost_lines(
            {"gb": 100, "iops": 3000, "mbps": 125}, self.MINOR, 1.0
        )
        self.assertEqual([l["item"] for l in lines], ["ebs pg volume storage"])

    def test_the_fleet_estimate_prices_the_volume_only_when_provisioned(self):
        prices = {
            "instances": {
                t: {"usd_hour": 1.0, "source": "test"}
                for t in ("c8g.4xlarge", "c8g.2xlarge")
            }
        }

        def items(modes: tuple) -> set[str]:
            cfg = awsb.RunConfig(
                tag="x",
                nodes=2,
                db_modes=modes,
                ttl_min="60",
                load=netbench.BenchConfig(submit_nodes=1),
            )
            estimate = awsb.estimate_fleet(
                awsb.plan_hosts(cfg), cfg, prices, self.MINOR
            )
            return {l["item"] for l in estimate["lines"]}

        self.assertIn("ebs pg volume storage", items(("colocated", "volume")))
        self.assertNotIn("ebs pg volume storage", items(("colocated",)))

    def test_actual_cost_of_a_fleet_includes_the_volume(self):
        harness = FleetHarness(self)
        runner = VolumeRunner([DONE_STATE], describe=DESCRIBE)
        harness.up(runner, "--db-modes", "volume")
        manifest = harness.fleet()
        with_volume = awsb.manifest_cost(manifest, 3600.0)
        without = awsb.manifest_cost(
            {k: v for k, v in manifest.items() if k != "pg_volume"}, 3600.0
        )
        self.assertGreater(with_volume, without)


@unittest.skipUnless(shutil.which("tofu"), "tofu/opentofu not on PATH")
class TofuPgVolumeTest(unittest.TestCase):
    """TEST:tofu-validate-modes-ok for `pg_volume`: validates the module and plans it offline
    with the volume set and null; skipped when the providers cannot be installed."""

    def node0_after(self, pg_volume: dict | None) -> dict:
        """The planned `aws_instance.host["node0"]`: its `after` values and `after_unknown`."""
        tofu = shutil.which("tofu")
        assert tofu is not None
        with tempfile.TemporaryDirectory() as tmp:
            module = Path(tmp) / "terraform"
            shutil.copytree(Path(__file__).parent / "aws" / "terraform", module)
            user_data = module / "user-data.sh"
            user_data.write_text("#!/bin/sh\necho ok\n")
            tfvars = {
                "name": "test-run",
                "owner": "tester",
                "git_rev": "abc1234",
                "account_id": "000000000000",
                "region": "eu-west-1",
                "profile": "test",
                "az": "",
                "ami_id": "",
                "ssh_public_key": "ssh-ed25519 AAAAtest test@example.com",
                "operator_cidr": "203.0.113.5/32",
                "expires_at": "2026-01-01T00:00:00Z",
                "offline": True,
                "pg_volume": pg_volume,
                "hosts": {
                    name: {
                        "role": role,
                        "instance_type": "c8g.2xlarge",
                        "root_gb": 40,
                        "root_iops": 3000,
                        "root_mbps": 125,
                        "user_data_path": str(user_data),
                    }
                    for name, role in (("ctl", "ctl"), ("node0", "query"))
                },
            }
            (module / "terraform.tfvars.json").write_text(json.dumps(tfvars))
            env = {
                **os.environ,
                "AWS_ACCESS_KEY_ID": "test",
                "AWS_SECRET_ACCESS_KEY": "test",
                "AWS_REGION": "eu-west-1",
            }

            def tofu_run(*args: str) -> subprocess.CompletedProcess:
                return subprocess.run(
                    [tofu, f"-chdir={module}", *args],
                    capture_output=True,
                    text=True,
                    env=env,
                    check=False,
                )

            init = tofu_run("init", "-input=false")
            if init.returncode != 0:
                self.skipTest(f"providers unavailable: {init.stderr.strip()[-200:]}")
            validate = tofu_run("validate")
            self.assertEqual(validate.returncode, 0, validate.stderr)
            plan = tofu_run("plan", "-input=false", "-refresh=false", "-out=plan.bin")
            self.assertEqual(plan.returncode, 0, plan.stderr)
            shown = tofu_run("show", "-json", "plan.bin")
            self.assertEqual(shown.returncode, 0, shown.stderr)
            changes = json.loads(shown.stdout)["resource_changes"]
            return next(
                c["change"]
                for c in changes
                if c["address"] == 'aws_instance.host["node0"]'
            )

    def test_the_volume_is_an_inline_block_on_the_query_host(self):
        change = self.node0_after({"gb": 400, "iops": 12000, "mbps": 500})
        (block,) = change["after"]["ebs_block_device"]
        self.assertEqual(block["device_name"], "/dev/sdf")
        self.assertEqual(block["volume_type"], "gp3")
        self.assertEqual(
            (block["volume_size"], block["iops"], block["throughput"]),
            (400, 12000, 500),
        )
        self.assertTrue(block["delete_on_termination"])
        self.assertEqual(block["tags"]["espresso-bench-run"], "test-run")
        self.assertEqual(block["tags"]["espresso-bench-role"], "pg")

    def test_without_the_volume_the_block_is_absent(self):
        change = self.node0_after(None)
        self.assertNotIn("ebs_block_device", change["after"])


# REQ:fleet-cost
class RunEstimateTest(unittest.TestCase):
    def test_run_phases_have_no_provision_or_destroy(self):
        phases = awsb.run_phase_seconds(awsb.RunConfig(tag="x"))
        self.assertEqual(
            list(phases), ["reset", "services", "ready", "load", "collect"]
        )
        self.assertEqual(phases["reset"], (awsb.RESET_EXPECTED_S, awsb.RESET_MAX_S))

    def test_estimate_is_the_fleet_rate_over_the_phases(self):
        harness = FleetHarness(self)
        harness.up_fleet(self)
        manifest = harness.fleet()
        cfg = awsb.fleet_run_config(harness.run_args(), manifest)
        estimate = awsb.estimate_run(manifest, cfg)
        phases = awsb.run_phase_seconds(cfg)
        self.assertEqual(estimate["worst_s"], sum(w for _, w in phases.values()))
        self.assertAlmostEqual(
            estimate["worst_usd"], estimate["usd_per_hour"] * estimate["worst_s"] / 3600
        )
        self.assertLess(estimate["expected_usd"], estimate["worst_usd"])

    def test_run_cost_is_the_rate_over_the_wall_time(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.run(runner)
        path = harness.fleet_dir / "runs" / "01-colocated" / "cost.json"
        cost = json.loads(path.read_text())
        rate = awsb.estimate_rate(harness.fleet()["estimate"])
        self.assertAlmostEqual(cost["usd_per_hour"], rate)
        self.assertAlmostEqual(cost["usd"], rate * cost["duration_s"] / 3600)

    def test_explicit_ttl_below_the_worst_case_is_refused_for_a_single_shot(self):
        with self.assertRaisesRegex(awsb.Refused, "below the worst case"):
            awsb.single_shot_ttl_s(awsb.RunConfig(tag="x", ttl_min="5"), 3600.0)
        self.assertEqual(
            awsb.single_shot_ttl_s(awsb.RunConfig(tag="x", ttl_min="90"), 3600.0),
            5400.0,
        )
        self.assertEqual(
            awsb.single_shot_ttl_s(awsb.RunConfig(tag="x"), 3600.0),
            3600.0 + awsb.TTL_MARGIN_S,
        )

    def test_fleet_bound_grows_with_the_expiry(self):
        harness = FleetHarness(self)
        harness.up_fleet(self)
        manifest = harness.fleet()
        expires = datetime.fromisoformat(manifest["expires_at"])
        base = awsb.fleet_bound_usd(manifest, expires)
        later = awsb.fleet_bound_usd(manifest, expires + timedelta(hours=1))
        self.assertAlmostEqual(
            later - base, awsb.estimate_rate(manifest["estimate"]), places=6
        )


# REQ:fleet-extend-rearm
class ExtendTest(unittest.TestCase):
    def extend(self, harness, runner, minutes: int, now: datetime | None = None):
        args = harness.parse(
            "extend", str(harness.fleet_dir), "--ttl-min", str(minutes)
        )
        return awsb.cmd_extend(args, run=runner, now=now)

    def test_rearms_every_host_and_moves_the_expiry(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        now = datetime.now(UTC)
        mark = len(runner.calls)
        self.assertEqual(self.extend(harness, runner, 240, now), awsb.EXIT_OK)
        commands = ssh_calls(runner, mark)
        rearm = [c for c in commands if "shutdown -c; shutdown -P +240" in c]
        self.assertEqual(len(rearm), 3)
        stamp = awsb.expiry_stamp(now + timedelta(minutes=240))
        self.assertEqual(harness.fleet()["expires_at"], stamp)
        self.assertEqual(harness.fleet()["phase"], "idle")

    def test_retags_every_arn_of_the_fleet_in_batches(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        runner.arns = 42
        now = datetime.now(UTC)
        self.extend(harness, runner, 240, now)
        tagged = [c for c in runner.calls if "tag-resources" in c]
        self.assertEqual(len(tagged), 3)
        per_call = [[a for a in c if a.startswith("arn:")] for c in tagged]
        self.assertEqual(sum(map(len, per_call)), 45)
        self.assertTrue(all(len(arns) <= 20 for arns in per_call))
        stamp = awsb.expiry_stamp(now + timedelta(minutes=240))
        self.assertTrue(all(f"{awsb.TAG_EXPIRES}={stamp}" in c for c in tagged))

    def test_failed_tagging_is_an_error(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        failed = completed(stdout=json.dumps({"FailedResourcesMap": {"arn:x": {}}}))
        with (
            unittest.mock.patch.object(
                awsb,
                "tagged_resources",
                return_value=["arn:x"],
            ),
            unittest.mock.patch.object(runner, "aws", return_value=failed),
            self.assertRaisesRegex(awsb.RemoteError, "arn:x"),
        ):
            self.extend(harness, runner, 240)

    def test_the_new_bound_is_recorded(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        before = harness.fleet()["estimate"]["bound_usd"]
        self.extend(harness, runner, 240)
        manifest = harness.fleet()
        self.assertGreater(manifest["estimate"]["bound_usd"], before)
        expires = datetime.fromisoformat(manifest["expires_at"])
        self.assertAlmostEqual(
            manifest["estimate"]["bound_usd"], awsb.fleet_bound_usd(manifest, expires)
        )
        self.assertRegex(
            harness.driver_log(), r"expires \d\d:\d\d UTC, bound \$\d+\.\d\d"
        )

    def test_over_budget_is_refused_before_any_change(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        mark = len(runner.calls)
        before = harness.fleet()
        with self.assertRaisesRegex(awsb.Refused, r"exceeds the fleet's --max-usd"):
            self.extend(harness, runner, 72 * 60)
        self.assertEqual(ssh_calls(runner, mark), [])
        self.assertEqual(harness.fleet(), before)

    def test_inside_the_nologin_window_is_refused(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        expires = datetime.fromisoformat(harness.fleet()["expires_at"])
        now = expires - timedelta(seconds=awsb.NOLOGIN_LEAD_S - 30)
        mark = len(runner.calls)
        with self.assertRaisesRegex(awsb.Refused, "locked out"):
            self.extend(harness, runner, 240, now)
        self.assertEqual(ssh_calls(runner, mark), [])

    def test_an_earlier_expiry_is_refused(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        with self.assertRaisesRegex(awsb.Refused, "not after the expiry"):
            self.extend(harness, runner, 30)

    def test_a_destroyed_fleet_cannot_be_extended(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.set_fleet(phase="done")
        with self.assertRaisesRegex(awsb.Refused, "not live"):
            self.extend(harness, runner, 240)

    def test_minutes_must_be_positive(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            awsb.parse_args(["extend", "x", "--ttl-min", "0"])


# REQ:fleet-down
class DownTest(unittest.TestCase):
    def down(self, harness, runner, *extra: str) -> int:
        args = harness.parse("down", str(harness.fleet_dir), "--yes", *extra)
        return awsb.cmd_down(args, run=runner)

    def test_down_destroys_prices_and_appends_the_fleet_row(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.run(runner)
        harness.run(runner, "--name", "again")
        self.assertEqual(self.down(harness, runner), awsb.EXIT_OK)
        self.assertTrue(runner.ran("tofu", "destroy"))
        cost = json.loads((harness.fleet_dir / "cost.json").read_text())
        self.assertGreater(cost["actual"], 0)
        self.assertEqual(harness.fleet()["phase"], "done")
        rows = harness.index()
        self.assertEqual(len(rows), 3)
        self.assertRegex(
            rows[2],
            rf"^\| fleet1 \| \S+ \| a{{10}} \| 2 \| [\d.]+ \| {cost['actual']:.2f} \| 0 \|$",
        )
        log = harness.driver_log()
        self.assertRegex(
            log, r"destroyed; actual cost \$\d+\.\d\d \(bound \$\d+\.\d\d\); 2 runs"
        )
        self.assertIn("- 01-colocated: valid, $", log)
        self.assertIn("- 02-again: valid, $", log)
        self.assertFalse(harness.lock().exists())

    def test_leftover_after_failed_destroys_exits_4_without_a_fleet_row(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        runner.destroys = [completed(returncode=1, stderr="locked")]
        self.assertEqual(self.down(harness, runner), awsb.EXIT_LEFTOVER)
        self.assertEqual(runner.count("tofu", "destroy"), awsb.DESTROY_RETRIES)
        self.assertTrue(runner.ran("terminate-instances", "i-1"))
        self.assertEqual(harness.fleet()["phase"], "left-running")
        self.assertFalse((harness.out / "INDEX.md").exists())

    # EDGE:fleet-down-while-running
    def test_down_while_running_stops_the_agent_first(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.set_fleet(phase="running")
        awsb.take_fleet_lock(harness.fleet_dir, "gone", False)
        mark = len(runner.calls)
        self.assertEqual(self.down(harness, runner), awsb.EXIT_OK)
        calls = [" ".join(call) for call in runner.calls[mark:]]
        stop = next(i for i, c in enumerate(calls) if "systemctl stop bench-agent" in c)
        destroy = next(
            i for i, c in enumerate(calls) if c.startswith("tofu") and "destroy" in c
        )
        self.assertLess(stop, destroy)
        self.assertFalse(harness.lock().exists())

    def test_declined_down_destroys_nothing(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        args = harness.parse("down", str(harness.fleet_dir))
        with self.assertRaisesRegex(awsb.Refused, "not confirmed"):
            awsb.cmd_down(args, run=runner)
        self.assertFalse(runner.ran("tofu", "destroy"))

    def test_destroy_dir_is_the_same_command(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        args = harness.parse("destroy", str(harness.fleet_dir), "--yes")
        self.assertEqual(awsb.cmd_destroy(args, run=runner), awsb.EXIT_OK)
        self.assertEqual(harness.fleet()["phase"], "done")


class StatusTest(unittest.TestCase):
    def test_status_shows_the_time_left_and_the_lock_holder(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        runner.describe = STATUS_DESCRIBE
        awsb.take_fleet_lock(harness.fleet_dir, "measuring", False)
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            awsb.cmd_status(harness.parse("status", str(harness.fleet_dir)), runner)
        text = out.getvalue()
        self.assertIn("- run fleet1: phase idle", text)
        self.assertRegex(text, r"- expires \S+ UTC, 1[78]\d min left")
        self.assertRegex(text, r"- lock: run measuring, pid \d+ \(alive\)")


class ParseArgsTest(unittest.TestCase):
    def test_db_modes(self):
        self.assertEqual(awsb.parse_db_modes("colocated"), ("colocated",))
        self.assertEqual(awsb.parse_db_modes("colocated,colocated"), ("colocated",))
        with self.assertRaisesRegex(awsb.argparse.ArgumentTypeError, "unknown db mode"):
            awsb.parse_db_modes("colocated,nonsense")

    def test_ttl_min(self):
        self.assertEqual(awsb.ttl_min_arg("auto"), "auto")
        self.assertEqual(awsb.ttl_min_arg("90"), "90")
        for bad in ("0", "-5", "1.5", "soon"):
            with self.assertRaises(awsb.argparse.ArgumentTypeError):
                awsb.ttl_min_arg(bad)

    def test_up_defaults_to_a_long_ttl_and_budget(self):
        args = awsb.parse_args(["up", "--tag", "t"])
        self.assertEqual(args.ttl_min, awsb.UP_TTL_MIN)
        self.assertEqual(args.max_usd, awsb.UP_MAX_USD)
        self.assertEqual(args.db_modes, ("colocated",))
        run = awsb.parse_args(["run", "--tag", "t"])
        self.assertEqual(run.ttl_min, "auto")
        self.assertEqual(run.max_usd, 10.0)
        self.assertIsNone(run.fleet)

    def test_up_needs_a_tag_but_run_fleet_does_not(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            awsb.parse_args(["up"])
        args = awsb.parse_args(["run", "--fleet", "some/dir"])
        self.assertIsNone(args.tag)
        self.assertEqual(args.fleet, Path("some/dir"))

    def test_run_does_not_abbreviate_flags(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            awsb.parse_args(["run", "--fleet", "d", "--node", "3"])

    def test_config_round_trips_through_the_fleet_manifest(self):
        cfg = awsb.RunConfig(tag="x", ttl_min="90", fleet=Path("a/b"))
        saved = json.loads(json.dumps(awsb.config_to_json(cfg)))
        self.assertEqual(awsb.config_from_manifest(saved), cfg)


class RunManifestTest(unittest.TestCase):
    def test_the_run_manifest_copies_the_fleet(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.run(runner)
        path = harness.fleet_dir / "runs" / "01-colocated" / "manifest.json"
        run_manifest = json.loads(path.read_text())
        fleet = harness.fleet()
        for key in ("hosts", "images", "az", "ami_id", "hosts_info", "git_rev"):
            self.assertEqual(run_manifest[key], fleet[key], key)
        self.assertEqual(set(run_manifest["images"]), set(fake_images()))
