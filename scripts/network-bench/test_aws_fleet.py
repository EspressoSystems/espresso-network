"""Tests for the fleet lifecycle of `aws-bench`: `up`, `run --fleet`, `extend` and `down`.

Reuses the fake runner and fixtures of `test_aws_bench`.

    just py::test
"""

import contextlib
import io
import json
import os
import shutil
import stat
import subprocess
import tempfile
import unittest
import unittest.mock
from datetime import UTC, datetime, timedelta
from pathlib import Path

import netbench
from test_aws_bench import (
    DESCRIBE,
    DONE_STATE,
    STATUS_DESCRIBE,
    FakeRunner,
    FleetRunner,
    aws_manifest,
    awsb,
    completed,
    fake_images,
    fake_preflight,
    price_response,
    two_node_hosts_info,
    valid_result,
    write_collected_run,
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
        awsb.reset_chain(fleet.remote, fleet.interrupts)
        commands = ssh_calls(runner, mark)
        self.assertEqual(len(commands), 3)
        self.assertEqual(sum("find /data/pg" in c for c in commands), 1)
        self.assertTrue(all("sudo timeout" in c for c in commands))


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


RDS_OUTPUT = {
    "identifier": "espresso-bench-fleet1",
    "endpoint": "espresso-bench-fleet1.abc.eu-west-1.rds.amazonaws.com",
    "port": 5432,
    "resource_id": "db-ABCDEF",
    "arn": "arn:aws:rds:eu-west-1:1:db:espresso-bench-fleet1",
    "engine_version": "18.2",
    "schedule_group": "espresso-bench-fleet1",
    "schedule_name": "rds-delete",
}
RDS_CREATED = "2026-09-30T11:05:00+00:00"
SCHEDULE_TARGET = {
    "Arn": "arn:aws:scheduler:::aws-sdk:rds:deleteDBInstance",
    "RoleArn": "arn:aws:iam::1:role/espresso-bench/espresso-bench-fleet1",
    "Input": "{}",
}


def db_instance(state: str = "available", applied: str = "in-sync") -> dict:
    return {
        "DBInstanceStatus": state,
        "DBParameterGroups": [
            {"DBParameterGroupName": "g", "ParameterApplyStatus": applied}
        ],
        "InstanceCreateTime": RDS_CREATED,
    }


def rds_metric_data(missing: tuple[str, ...] = (), value: float = 100.0) -> str:
    def result(query_id: str) -> dict:
        points = [] if query_id in missing else [(60.0 * i, value) for i in (1, 2, 3)]
        return {
            "Id": query_id,
            "Label": query_id,
            "Timestamps": [
                datetime.fromtimestamp(ts, UTC).isoformat() for ts, _ in points
            ],
            "Values": [v for _, v in points],
            "StatusCode": "Complete",
        }

    return json.dumps(
        {"MetricDataResults": [result(i) for i in awsb.RDS_METRICS], "Messages": []}
    )


def rds_spec(**overrides) -> dict:
    return {
        "instance_class": "db.m8g.4xlarge",
        "engine_version": "18.2",
        "gb": 400,
        "iops": 12000,
        "mbps": 500,
        **overrides,
    }


def mode(path: Path) -> int:
    return stat.S_IMODE(path.stat().st_mode)


class RdsRunner(TaggingRunner):
    """`TaggingRunner` with the rds output of `tofu`, and the `aws rds`, `scheduler`, `pi`
    and CloudWatch calls an rds fleet makes. `instances` are the successive
    describe-db-instances answers; the last one repeats."""

    def __init__(
        self,
        *args,
        instances: list[dict] | None = None,
        schedule: dict | None = None,
        events: list[dict] | None = None,
        logs: list[dict] | None = None,
        **kwargs,
    ):
        super().__init__(*args, **kwargs)
        self.instances = instances or [db_instance()]
        self.schedule = schedule or {"State": "ENABLED", "Target": SCHEDULE_TARGET}
        self.events = events or []
        self.logs = logs or []
        self.metrics = rds_metric_data()

    def tofu(self, verb: str) -> subprocess.CompletedProcess:
        if verb == "output":
            output = {
                "hosts": {"value": two_node_hosts_info()},
                "rds": {"value": RDS_OUTPUT},
            }
            return completed(stdout=json.dumps(output))
        return super().tofu(verb)

    def aws(self, argv: list[str]) -> subprocess.CompletedProcess:
        if "describe-db-instances" in argv:
            with self.lock:
                if len(self.instances) > 1:
                    instance = self.instances.pop(0)
                else:
                    instance = self.instances[0]
            return completed(stdout=json.dumps({"DBInstances": [instance]}))
        if "get-schedule" in argv:
            return completed(stdout=json.dumps(self.schedule))
        if "describe-events" in argv:
            return completed(stdout=json.dumps({"Events": self.events}))
        if "get-metric-data" in argv and "AWS/RDS" in " ".join(argv):
            return completed(stdout=self.metrics)
        if "get-resource-metrics" in argv:
            body = {"MetricList": [{"Key": "db.load.avg"}]}
            return completed(stdout=json.dumps(body))
        if "describe-db-log-files" in argv:
            return completed(stdout=json.dumps({"DescribeDBLogFiles": self.logs}))
        if "download-db-log-file-portion" in argv:
            name = argv[argv.index("--log-file-name") + 1]
            return completed(stdout=f"log of {name}\n")
        if "reboot-db-instance" in argv or "update-schedule" in argv:
            return completed()
        return super().aws(argv)


class RdsHarness(FleetHarness):
    """`FleetHarness` for an rds fleet: preflight resolved the engine minor 18.2."""

    def __init__(self, test: unittest.TestCase, modes: str = "rds"):
        super().__init__(test)
        self.modes = modes
        patch = unittest.mock.patch.object(
            awsb,
            "preflight",
            return_value={**fake_preflight(), "rds_engine_version": "18.2"},
        )
        patch.start()
        test.addCleanup(patch.stop)

    def fleet_flags(self) -> list[str]:
        return [*super().fleet_flags(), "--price", "db.m8g.4xlarge=1.82"]

    def up_args(self, *extra: str) -> "awsb.argparse.Namespace":
        return super().up_args("--db-modes", self.modes, *extra)

    def up_rds(self, test: unittest.TestCase, **kwargs) -> RdsRunner:
        runner = RdsRunner([DONE_STATE], describe=DESCRIBE, **kwargs)
        test.assertEqual(self.up(runner), awsb.EXIT_OK)
        return runner

    def tfvars(self) -> dict:
        path = self.fleet_dir / "terraform" / "terraform.tfvars.json"
        return json.loads(path.read_text())

    def run_dir(self, name: str) -> Path:
        return self.fleet_dir / "runs" / name


# REQ:querydb-rds-guard
class RdsGuardTest(unittest.TestCase):
    def refused(self, pattern: str, *extra: str) -> None:
        harness = RdsHarness(self)
        runner = FakeRunner()
        with self.assertRaisesRegex(awsb.Refused, pattern):
            harness.up(runner, *extra)
        self.assertEqual(runner.calls, [])

    def test_small_volume_is_refused_before_any_call(self):
        self.refused("--pg-gb 100", "--pg-gb", "100")

    def test_other_iops_or_throughput_are_refused(self):
        self.refused("--pg-iops 12000", "--pg-iops", "6000")
        self.refused("--pg-mbps 500", "--pg-mbps", "250")

    def test_a_class_below_4xlarge_is_refused(self):
        self.refused("EBS baseline", "--rds-class", "db.r7g.xlarge")
        self.refused("db.<family>", "--rds-class", "m8g.4xlarge")

    # TEST:querydb-rds-name-fails
    def test_names_rds_rejects_are_refused(self):
        for name in ("a--b", "trailing-", "-leading"):
            harness = RdsHarness(self)
            runner = FakeRunner()
            args = harness.up_args()
            args.name = name
            with self.assertRaisesRegex(awsb.Refused, "--name"):
                awsb.cmd_up(args, run=runner, interrupts=awsb.Interrupts())
            self.assertEqual(runner.calls, [], name)

    def test_the_default_name_passes_the_name_check(self):
        awsb.check_rds_config(awsb.RunConfig(tag="x", db_modes=("rds",)))
        self.assertTrue(awsb.RDS_NAME_RE.match(awsb.default_run_name()))

    def test_other_modes_skip_the_guards(self):
        awsb.check_rds_config(awsb.RunConfig(tag="x", pg_gb=1, rds_class="db.t3.micro"))


def orderable(**overrides) -> dict:
    option = {
        "Engine": "postgres",
        "EngineVersion": "18.2",
        "DBInstanceClass": "db.m8g.4xlarge",
        "StorageType": "gp3",
        "AvailabilityZones": [{"Name": "eu-west-1a"}, {"Name": "eu-west-1b"}],
        "SupportsPerformanceInsights": True,
        "MinStorageSize": 400,
        "MaxStorageSize": 65536,
        "MinIopsPerDbInstance": 12000,
        "MaxIopsPerDbInstance": 64000,
        "MinStorageThroughputPerDbInstance": 500,
        "MaxStorageThroughputPerDbInstance": 4000,
    }
    return {**option, **overrides}


def orderable_runner(*options: dict) -> FakeRunner:
    response = completed(stdout=json.dumps({"OrderableDBInstanceOptions": options}))
    return FakeRunner({("aws", "--profile", "timeboost-dev", "rds"): response})


def image_info(ref: str) -> dict:
    return {"ref": ref, "digest": "sha256:0", "revision": None, "platforms": []}


# REQ:querydb-rds-guard
class RdsOrderableTest(unittest.TestCase):
    cfg = awsb.RunConfig(tag="x", db_modes=("rds",))

    def test_a_major_resolves_to_the_highest_minor_that_fits(self):
        runner = orderable_runner(
            orderable(EngineVersion="18.1"),
            orderable(EngineVersion="18.10"),
            orderable(EngineVersion="18.9"),
            orderable(EngineVersion="17.6"),
            orderable(EngineVersion="18.11", StorageType="io2"),
            orderable(
                EngineVersion="18.12", AvailabilityZones=[{"Name": "eu-west-1c"}]
            ),
        )
        self.assertEqual(awsb.rds_orderable(runner, self.cfg, "eu-west-1b"), "18.10")
        argv = runner.calls[0]
        self.assertEqual(argv[argv.index("--db-instance-class") + 1], "db.m8g.4xlarge")
        self.assertEqual(argv[argv.index("--engine") + 1], "postgres")

    def test_a_minor_matches_exactly(self):
        runner = orderable_runner(
            orderable(EngineVersion="18.1"), orderable(EngineVersion="18.10")
        )
        cfg = awsb.RunConfig(tag="x", db_modes=("rds",), rds_engine_version="18.1")
        self.assertEqual(awsb.rds_orderable(runner, cfg, "eu-west-1a"), "18.1")

    def test_an_unorderable_class_names_the_alternative(self):
        runner = orderable_runner(orderable(AvailabilityZones=[{"Name": "eu-west-1a"}]))
        with self.assertRaisesRegex(awsb.Refused, "--rds-class db.m7g.4xlarge"):
            awsb.rds_orderable(runner, self.cfg, "eu-west-1b")

    def test_storage_iops_and_throughput_ranges_must_cover_the_request(self):
        for override in (
            {"MinStorageSize": 500},
            {"MaxIopsPerDbInstance": 3000},
            {"MinStorageThroughputPerDbInstance": 1000},
            {"SupportsPerformanceInsights": False},
        ):
            runner = orderable_runner(orderable(**override))
            with self.assertRaises(awsb.Refused, msg=override):
                awsb.rds_orderable(runner, self.cfg, "eu-west-1b")

    def test_a_failed_call_is_a_refusal(self):
        runner = FakeRunner(
            {("aws",): completed(returncode=254, stderr="AccessDenied")}
        )
        with self.assertRaisesRegex(awsb.Refused, "AccessDenied"):
            awsb.rds_orderable(runner, self.cfg, "eu-west-1b")

    def test_preflight_pins_the_container_to_the_resolved_minor(self):
        cfg = awsb.RunConfig(
            tag="x",
            nodes=2,
            db_modes=("rds",),
            load=netbench.BenchConfig(submit_nodes=1),
        )
        runner = orderable_runner(orderable(EngineVersion="18.4"))
        resolved: list[str] = []

        def resolve(ref: str) -> dict:
            resolved.append(ref)
            return image_info(ref)

        stubs = {
            "caller_account": cfg.account,
            "default_vpc": "vpc-1",
            "capable_az": "eu-west-1b",
            "_running_instance_types": [],
            "describe_instance_types": {},
            "vcpu_headroom": (40, 0, 256.0),
            "resolve_ami_arch": "arm64",
            "resolve_ami": "ami-1",
            "check_git_clean": None,
        }
        with contextlib.ExitStack() as stack:
            for name, value in stubs.items():
                stack.enter_context(
                    unittest.mock.patch.object(awsb, name, return_value=value)
                )
            stack.enter_context(
                unittest.mock.patch.object(awsb, "resolve_image", resolve)
            )
            pre = awsb.preflight(
                runner, cfg, awsb.plan_hosts(cfg), which=lambda name: "/usr/bin/x"
            )
        self.assertEqual(pre["rds_engine_version"], "18.4")
        self.assertIn("docker.io/library/postgres:18.4", resolved)
        self.assertEqual(
            pre["images"]["postgres"]["ref"], "docker.io/library/postgres:18.4"
        )

    def test_a_missing_docker_tag_names_the_older_minor_flag(self):
        cfg = awsb.RunConfig(tag="x", db_modes=("rds",))

        def resolve(ref: str) -> dict:
            if ref.endswith("postgres:18.4"):
                raise awsb.Refused(f"image not found: {ref}")
            return image_info(ref)

        with (
            unittest.mock.patch.object(awsb, "resolve_image", resolve),
            self.assertRaisesRegex(awsb.Refused, "--rds-engine-version <older minor>"),
        ):
            awsb.resolve_images(cfg, "18.4")

    def test_without_rds_the_container_keeps_its_fixed_tag(self):
        cfg = awsb.RunConfig(tag="x")
        self.assertEqual(
            awsb.image_refs(cfg)["postgres"], awsb.SUPPORT_IMAGES["postgres"]
        )


# REQ:fleet-cost
class RdsCostTest(unittest.TestCase):
    def setUp(self):
        self.prices = {
            "instances": {
                "c8g.4xlarge": {"usd_hour": 0.78, "source": "test"},
                "c8g.2xlarge": {"usd_hour": 0.39, "source": "test"},
                "db.m8g.4xlarge": {"usd_hour": 1.82, "source": "test"},
            }
        }
        self.minor = awsb.MINOR_PRICES["eu-west-1"]

    def test_rds_lines_are_the_instance_and_the_storage(self):
        instance, storage = awsb.rds_cost_lines(
            rds_spec(), self.prices, self.minor, 7200.0
        )
        self.assertEqual(instance["item"], "rds instance db.m8g.4xlarge")
        self.assertAlmostEqual(instance["usd"], 2 * 1.82)
        self.assertEqual(storage["item"], "rds gp3 storage")
        self.assertAlmostEqual(
            storage["usd"], 400 * 2 * self.minor["rds_gp3_gb_month_usd"] / 730.0
        )

    def test_the_fleet_bound_bills_rds_until_its_delete_finishes(self):
        cfg = awsb.RunConfig(
            tag="x",
            nodes=2,
            ttl_min="150",
            db_modes=("rds",),
            load=netbench.BenchConfig(submit_nodes=1),
        )
        hosts = awsb.plan_hosts(cfg)
        without = awsb.estimate_fleet(hosts, cfg, self.prices, self.minor)
        rds = rds_spec()
        with_rds = awsb.estimate_fleet(hosts, cfg, self.prices, self.minor, rds)
        ttl_s = 150 * 60.0
        expected = awsb.rds_cost_lines(rds, self.prices, self.minor, ttl_s)
        self.assertAlmostEqual(
            with_rds["expected_usd"] - without["expected_usd"],
            sum(line["usd"] for line in expected),
        )
        bound = awsb.rds_cost_lines(
            rds, self.prices, self.minor, ttl_s + awsb.RDS_DELETE_S
        )
        self.assertAlmostEqual(
            with_rds["bound_usd"] - without["bound_usd"],
            sum(line["usd"] for line in bound),
        )
        self.assertGreater(
            awsb.estimate_rate(with_rds), awsb.estimate_rate(without) + 1.82
        )

    def test_a_single_shot_provisions_for_the_rds_create(self):
        plain = awsb.phase_seconds(awsb.RunConfig(tag="x"))
        rds = awsb.phase_seconds(awsb.RunConfig(tag="x", db_modes=("rds",)))
        self.assertEqual(
            rds["provision"],
            (
                plain["provision"][0] + awsb.RDS_CREATE_S,
                plain["provision"][1] + awsb.RDS_CREATE_MAX_S,
            ),
        )
        self.assertEqual(rds["load"], plain["load"])

    def test_the_price_comes_from_the_rds_pricing_service(self):
        cfg = awsb.RunConfig(tag="x", db_modes=("rds",))
        runner = FakeRunner({("aws",): price_response(1.82)})
        self.assertEqual(awsb.fetch_rds_price(runner, cfg, "db.m8g.4xlarge"), 1.82)
        argv = runner.calls[0]
        self.assertEqual(argv[argv.index("--service-code") + 1], "AmazonRDS")
        filters = {
            f["Field"]: f["Value"]
            for f in json.loads(argv[argv.index("--filters") + 1])
        }
        self.assertEqual(
            filters,
            {
                "instanceType": "db.m8g.4xlarge",
                "regionCode": "eu-west-1",
                "databaseEngine": "PostgreSQL",
                "deploymentOption": "Single-AZ",
            },
        )

    def test_the_class_price_is_cached_with_the_instance_prices(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp) / "prices.json"
            cfg = awsb.RunConfig(
                tag="x", db_modes=("rds",), price=("c8g.4xlarge=1", "c8g.2xlarge=1")
            )
            runner = FakeRunner({("aws",): price_response(1.82)})
            prices = awsb.resolve_prices(runner, cfg, cache, 0.0)
            self.assertEqual(prices["instances"]["db.m8g.4xlarge"]["usd_hour"], 1.82)
            again = FakeRunner()
            awsb.resolve_prices(again, cfg, cache, 60.0)
            self.assertEqual(again.calls, [])

    # TEST:cost-rds-price-missing-fails
    def test_a_missing_price_names_the_flag(self):
        cfg = awsb.RunConfig(
            tag="x", db_modes=("rds",), price=("c8g.4xlarge=1", "c8g.2xlarge=1")
        )
        runner = FakeRunner({("aws",): completed(returncode=254, stderr="denied")})
        with (
            tempfile.TemporaryDirectory() as tmp,
            self.assertRaisesRegex(awsb.Refused, "--price db.m8g.4xlarge="),
        ):
            awsb.resolve_prices(runner, cfg, Path(tmp) / "prices.json", 0.0)

    def test_offline_needs_the_class_price_too(self):
        cfg = awsb.RunConfig(
            tag="x",
            db_modes=("rds",),
            offline=True,
            price=("c8g.4xlarge=1", "c8g.2xlarge=1"),
        )
        with self.assertRaisesRegex(awsb.Refused, "db.m8g.4xlarge"):
            awsb.resolve_prices(FakeRunner(), cfg, Path("unused.json"), 0.0)

    def test_the_bound_after_an_extend_bills_the_delete_window(self):
        harness = RdsHarness(self)
        harness.up_rds(self)
        manifest = harness.fleet()
        expires = datetime.fromisoformat(manifest["expires_at"])
        created = datetime.fromisoformat(manifest["created_at"])
        lifetime_s = (expires - created).total_seconds()
        prices = {
            "instances": {
                "c8g.4xlarge": {"usd_hour": 0.71, "source": "t"},
                "c8g.2xlarge": {"usd_hour": 0.355, "source": "t"},
                "db.m8g.4xlarge": {"usd_hour": 1.82, "source": "t"},
            }
        }
        rds = awsb.rds_cost_lines(
            manifest["rds_spec"], prices, self.minor, lifetime_s + awsb.RDS_DELETE_S
        )
        hosts = awsb._cost_lines(
            manifest["hosts"], prices, self.minor, lifetime_s + awsb.BOOT_ALLOWANCE_S
        )
        self.assertAlmostEqual(
            awsb.fleet_bound_usd(manifest, expires),
            sum(line["usd"] for line in [*rds, *hosts]),
        )

    def test_a_run_needs_time_for_the_rds_delete(self):
        harness = RdsHarness(self)
        harness.up_rds(self)
        manifest = harness.fleet()
        cfg = awsb.fleet_run_config(harness.run_args("--query-db", "rds"), manifest)
        worst_s = awsb.estimate_run(manifest, cfg)["worst_s"]
        floor = worst_s + awsb.NOLOGIN_LEAD_S + awsb.DESTROY_S
        expires = datetime.fromisoformat(manifest["expires_at"])
        short = expires - timedelta(seconds=floor + awsb.RDS_DELETE_S - 60)
        with self.assertRaisesRegex(awsb.Refused, "extend"):
            awsb.check_run_allowed(manifest, cfg, short)
        enough = expires - timedelta(seconds=floor + awsb.RDS_DELETE_S + 60)
        awsb.check_run_allowed(manifest, cfg, enough)


# REQ:querydb-rds-wiring
class RdsTfvarsTest(unittest.TestCase):
    def test_the_variables_carry_the_spec_the_parameters_and_the_delete_time(self):
        harness = RdsHarness(self)
        harness.up_rds(self)
        rds = harness.tfvars()["rds"]
        self.assertEqual(rds["instance_class"], "db.m8g.4xlarge")
        self.assertEqual(rds["engine_version"], "18.2")
        self.assertEqual((rds["gb"], rds["iops"], rds["mbps"]), (400, 12000, 500))
        self.assertEqual(rds["username"], awsb.RDS_USER)
        self.assertEqual(rds["timeout"], "25m")
        expires = datetime.fromisoformat(harness.fleet()["expires_at"])
        reaper = expires - timedelta(seconds=awsb.RDS_REAPER_LEAD_S)
        self.assertEqual(rds["delete_at"], reaper.strftime("%Y-%m-%dT%H:%M:%S"))
        self.assertEqual(harness.fleet()["rds_spec"], rds_spec())

    # TEST:querydb-settings-parity-ok
    def test_the_parameter_group_renders_from_the_container_settings(self):
        harness = RdsHarness(self)
        harness.up_rds(self)
        parameters = harness.tfvars()["rds"]["parameters"]
        for key, value in awsb.PG_TUNING.items():
            self.assertEqual(parameters[key], awsb.rds_parameter(key, value), key)
        self.assertEqual(parameters["shared_buffers"], "2097152")
        self.assertEqual(parameters["shared_preload_libraries"], "pg_stat_statements")
        self.assertEqual(parameters["log_min_duration_statement"], "200")
        self.assertEqual(parameters["pg_stat_statements.track"], "all")
        self.assertNotIn("ssl", parameters)
        self.assertNotIn("rds.force_ssl", parameters)

    def test_the_password_is_generated_private_and_rds_safe(self):
        harness = RdsHarness(self)
        harness.up_rds(self)
        password = harness.tfvars()["rds_password"]
        self.assertGreaterEqual(len(password), 24)
        self.assertRegex(password, r"^[A-Za-z0-9_-]+$")
        terraform = harness.fleet_dir / "terraform"
        self.assertEqual(mode(terraform), 0o700)
        self.assertEqual(mode(terraform / "terraform.tfvars.json"), 0o600)
        self.assertNotIn(password, (harness.fleet_dir / "fleet.json").read_text())
        self.assertNotIn(password, harness.driver_log())

    def test_the_delete_time_follows_the_confirmed_expiry(self):
        harness = RdsHarness(self)
        before = datetime.now(UTC)
        harness.up_rds(self)
        delete_at = datetime.fromisoformat(
            harness.tfvars()["rds"]["delete_at"] + "+00:00"
        )
        expected = before + timedelta(minutes=180, seconds=-awsb.RDS_REAPER_LEAD_S)
        self.assertLess(abs((delete_at - expected).total_seconds()), 60)

    def test_a_fleet_without_rds_has_no_rds_variables(self):
        harness = FleetHarness(self)
        harness.up_fleet(self)
        path = harness.fleet_dir / "terraform" / "terraform.tfvars.json"
        tfvars = json.loads(path.read_text())
        self.assertNotIn("rds", tfvars)
        self.assertNotIn("rds_password", tfvars)
        self.assertNotIn("rds_spec", harness.fleet())

    def test_plan_offline_renders_the_rds_variables_and_prices_them(self):
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        argv = [
            "plan",
            "--offline",
            "--nodes",
            "2",
            "--tag",
            "x",
            "--genesis",
            str(GENESIS),
            "--db-modes",
            "colocated,rds",
            "--out-root",
            str(tmp),
            "--name",
            "planned",
            "--price",
            "c8g.4xlarge=0.78",
            "--price",
            "c8g.2xlarge=0.39",
            "--price",
            "db.m8g.4xlarge=1.82",
            "--operator-cidr",
            "203.0.113.5/32",
        ]
        args = awsb.parse_args(argv)
        args.argv = argv
        runner = FakeRunner({("git",): completed("a" * 40)})
        self.assertEqual(awsb.cmd_plan(args, run=runner), awsb.EXIT_OK)
        tfvars = json.loads(
            (tmp / "planned/terraform/terraform.tfvars.json").read_text()
        )
        self.assertEqual(tfvars["rds"]["engine_version"], "18")
        manifest = json.loads((tmp / "planned/fleet.json").read_text())
        self.assertEqual(manifest["db_modes"], ["colocated", "rds"])
        items = [line["item"] for line in manifest["estimate"]["lines"]]
        self.assertIn("rds instance db.m8g.4xlarge", items)
        self.assertIn("rds gp3 storage", items)

    def test_terraform_access_denied_names_the_rds_actions(self):
        message = awsb.classify_tf_error(
            "Error: creating IAM Role: AccessDenied: not authorized: iam:CreateRole"
        )
        self.assertIn("iam:PassRole", message)
        self.assertIn("scheduler:CreateSchedule", message)
        self.assertNotIn("iam:", awsb.classify_tf_error("AccessDenied: s3:GetObject"))


# REQ:querydb-rds-wiring
class RdsRenderTest(unittest.TestCase):
    def setUp(self):
        self.images = fake_images()
        self.query = {
            "name": "node0",
            "role": "query",
            "instance_type": "x",
            "root_gb": 100,
            "root_iops": 12000,
            "root_mbps": 500,
        }

    def test_rds_has_no_postgres_container(self):
        script = awsb.render_start_sh(self.query, self.images, query_db="rds")
        self.assertNotIn("--name postgres", script)
        self.assertIn("--name espresso-node", script)
        self.assertIn("--name postgres", awsb.render_start_sh(self.query, self.images))

    def test_node0_env_points_at_the_endpoint(self):
        harness = RdsHarness(self)
        harness.up_rds(self)
        rds = json.loads((harness.fleet_dir / "rds.json").read_text())
        run_dir = harness.run_dir("01-rds")
        run_dir.mkdir(parents=True)
        cfg = awsb.dataclasses.replace(
            awsb.config_from_manifest(harness.fleet()["config"]), query_db="rds"
        )
        awsb.render_host_files(run_dir, cfg, harness.fleet(), two_node_hosts_info())
        env = dict(
            line.split("=", 1)
            for line in (run_dir / "hosts/node0/node.env").read_text().splitlines()
        )
        self.assertEqual(env["ESPRESSO_NODE_POSTGRES_HOST"], RDS_OUTPUT["endpoint"])
        self.assertEqual(env["ESPRESSO_NODE_POSTGRES_USER"], awsb.RDS_USER)
        self.assertEqual(env["ESPRESSO_NODE_POSTGRES_PASSWORD"], rds["password"])
        self.assertEqual(env["ESPRESSO_NODE_POSTGRES_DATABASE"], "espresso")
        pg = run_dir / "hosts/node0/pg.json"
        self.assertEqual(json.loads(pg.read_text())["host"], RDS_OUTPUT["endpoint"])
        self.assertEqual(mode(pg), 0o600)
        self.assertEqual(mode(run_dir / "hosts/node0/node.env"), 0o600)
        script = (run_dir / "hosts/node0/start.sh").read_text()
        self.assertNotIn("--name postgres", script)


# REQ:querydb-rds-wiring
class RdsUpTest(unittest.TestCase):
    def test_up_records_the_instance_and_gates_node0_on_it(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        manifest = harness.fleet()
        self.assertEqual(manifest["phase"], "idle")
        self.assertEqual(manifest["rds"], {**RDS_OUTPUT, "created_at": RDS_CREATED})
        rds = json.loads((harness.fleet_dir / "rds.json").read_text())
        self.assertEqual(rds["endpoint"], RDS_OUTPUT["endpoint"])
        self.assertEqual(rds["username"], awsb.RDS_USER)
        self.assertEqual(rds["password"], harness.tfvars()["rds_password"])
        self.assertEqual(mode(harness.fleet_dir / "rds.json"), 0o600)
        pg = json.loads((harness.fleet_dir / "hosts/node0/pg.json").read_text())
        self.assertEqual(pg["host"], RDS_OUTPUT["endpoint"])
        self.assertTrue(runner.ran("rsync", f"{awsb.BENCH_DIR}/pg.json"))
        commands = ssh_calls(runner)
        self.assertTrue(any("pg_isready" in c for c in commands))
        self.assertFalse(any("docker start" in c for c in commands))
        self.assertFalse(runner.ran("reboot-db-instance"))
        self.assertTrue(runner.ran("scheduler", "get-schedule", "rds-delete"))

    def test_the_password_never_reaches_a_command_line(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        password = harness.tfvars()["rds_password"]
        self.assertFalse(any(password in " ".join(call) for call in runner.calls))

    def test_the_prompt_lists_the_rds_resources(self):
        harness = RdsHarness(self)
        args = harness.up_args()
        args.yes = False
        with unittest.mock.patch.object(awsb, "confirm", return_value=True) as confirm:
            awsb.cmd_up(args, run=RdsRunner([DONE_STATE]), interrupts=awsb.Interrupts())
        self.assertIn(
            "1 rds instance, 1 schedule, 1 iam role", confirm.call_args.args[0]
        )

    # TEST:querydb-rds-pending-reboot-ok
    def test_pending_reboot_reboots_once_and_waits(self):
        harness = RdsHarness(self)
        instances = [
            db_instance(applied="pending-reboot"),
            db_instance(state="rebooting", applied="pending-reboot"),
            db_instance(applied="in-sync"),
        ]
        with unittest.mock.patch.object(awsb, "RDS_POLL_S", 0.0):
            runner = harness.up_rds(self, instances=instances)
        self.assertEqual(runner.count("reboot-db-instance"), 1)
        self.assertEqual(harness.fleet()["phase"], "idle")

    def test_parameters_that_stay_pending_destroy_the_fleet(self):
        harness = RdsHarness(self)
        runner = RdsRunner(
            [DONE_STATE], instances=[db_instance(applied="pending-reboot")]
        )
        with unittest.mock.patch.multiple(
            awsb, RDS_POLL_S=0.0, RDS_SYNC_TIMEOUT_S=0.05
        ):
            self.assertEqual(harness.up(runner), awsb.EXIT_FAILED)
        self.assertEqual(runner.count("reboot-db-instance"), 1)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertIn("pending-reboot", harness.driver_log())

    # TEST:querydb-rds-create-timeout-fails
    def test_a_create_timeout_destroys_and_exits_3(self):
        harness = RdsHarness(self)
        runner = RdsRunner(
            [DONE_STATE],
            apply=completed(returncode=1, stderr="Error: waiting for RDS DB Instance"),
        )
        self.assertEqual(harness.up(runner), awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertFalse(runner.ran("describe-db-instances"))

    def test_a_disabled_schedule_destroys_the_fleet(self):
        harness = RdsHarness(self)
        runner = RdsRunner(
            [DONE_STATE], schedule={"State": "DISABLED", "Target": SCHEDULE_TARGET}
        )
        self.assertEqual(harness.up(runner), awsb.EXIT_FAILED)
        self.assertIn("would outlive the fleet", harness.driver_log())
        self.assertTrue(runner.ran("tofu", "destroy"))

    def test_a_missing_schedule_destroys_the_fleet(self):
        harness = RdsHarness(self)
        runner = RdsRunner([DONE_STATE])
        original = runner.aws

        def aws(argv):
            if "get-schedule" in argv:
                return completed(returncode=254, stderr="ResourceNotFoundException")
            return original(argv)

        with unittest.mock.patch.object(runner, "aws", side_effect=aws):
            self.assertEqual(harness.up(runner), awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "destroy"))

    def test_a_null_rds_output_is_an_error(self):
        with self.assertRaisesRegex(awsb.RemoteError, "null"):
            awsb.parse_rds_output({"rds": {"value": None}})


# REQ:querydb-rds-wiring
class RdsRunTest(unittest.TestCase):
    def test_reset_drops_and_recreates_the_database_before_the_services(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        mark = len(runner.calls)
        self.assertEqual(harness.run(runner, "--query-db", "rds"), awsb.EXIT_OK)
        commands = ssh_calls(runner, mark)
        reset = next(i for i, c in enumerate(commands) if "find /data/journal" in c)
        drop = next(i for i, c in enumerate(commands) if "DROP DATABASE" in c)
        anvil = next(i for i, c in enumerate(commands) if "docker start anvil" in c)
        self.assertLess(reset, drop)
        self.assertLess(drop, anvil)
        # TEST:querydb-rds-drop-force-ok
        statement = commands[drop]
        self.assertIn("DROP DATABASE IF EXISTS espresso WITH (FORCE)", statement)
        self.assertIn("CREATE DATABASE espresso", statement)
        self.assertIn("PGDATABASE=postgres", statement)
        self.assertIn("ON_ERROR_STOP=1", statement)
        self.assertNotIn(harness.tfvars()["rds_password"], statement)

    def test_the_endpoint_is_shipped_before_the_drop(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        mark = len(runner.calls)
        harness.run(runner, "--query-db", "rds")
        calls = [" ".join(call) for call in runner.calls[mark:]]
        push = next(
            i
            for i, c in enumerate(calls)
            if c.startswith("rsync") and f"{awsb.BENCH_DIR}/pg.json" in c
        )
        drop = next(i for i, c in enumerate(calls) if "DROP DATABASE" in c)
        self.assertLess(push, drop)

    def test_stats_reset_and_extension_run_and_no_container_starts(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        mark = len(runner.calls)
        harness.run(runner, "--query-db", "rds")
        commands = ssh_calls(runner, mark)
        self.assertFalse(any("docker start postgres" in c for c in commands))
        self.assertTrue(any("pg_stat_reset_shared" in c for c in commands))
        self.assertTrue(any("CREATE EXTENSION IF NOT EXISTS" in c for c in commands))
        self.assertTrue(any("pg_isready" in c for c in commands))

    def test_the_run_manifest_and_start_script_are_rds(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        harness.run(runner, "--query-db", "rds")
        run_dir = harness.run_dir("01-rds")
        manifest = json.loads((run_dir / "manifest.json").read_text())
        self.assertEqual(manifest["query_db"], "rds")
        self.assertEqual(manifest["rds_spec"], rds_spec())
        script = (run_dir / "hosts/node0/start.sh").read_text()
        self.assertNotIn("--name postgres", script)

    # EDGE:querydb-mode-switch
    def test_a_colocated_run_after_an_rds_run_starts_its_own_postgres(self):
        harness = RdsHarness(self, modes="colocated,rds")
        runner = harness.up_rds(self)
        harness.run(runner, "--query-db", "rds")
        mark = len(runner.calls)
        harness.run(runner, "--query-db", "colocated")
        commands = ssh_calls(runner, mark)
        self.assertFalse(any("DROP DATABASE" in c for c in commands))
        self.assertTrue(any("docker start postgres" in c for c in commands))
        host_dir = harness.run_dir("02-colocated") / "hosts/node0"
        self.assertIn("--name postgres", (host_dir / "start.sh").read_text())
        self.assertEqual(
            json.loads((host_dir / "pg.json").read_text())["host"], "127.0.0.1"
        )

    def test_an_rds_run_needs_the_mode_in_the_fleet(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        with self.assertRaisesRegex(awsb.Refused, "--query-db rds was not provisioned"):
            harness.run(runner, "--query-db", "rds")

    # EDGE:fleet-reset-fails
    def test_a_failed_database_drop_leaves_the_fleet_dirty(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        original = runner.ssh

        def failing(command: str) -> subprocess.CompletedProcess:
            if "DROP DATABASE" in command:
                return completed(returncode=1, stderr="permission denied")
            return original(command)

        with unittest.mock.patch.object(runner, "ssh", side_effect=failing):
            self.assertEqual(harness.run(runner, "--query-db", "rds"), awsb.EXIT_FAILED)
        self.assertEqual(harness.fleet()["phase"], "dirty")
        self.assertTrue(harness.lock().exists())
        self.assertFalse(runner.ran("docker start anvil"))

    def test_a_single_shot_creates_the_instance_measures_and_destroys(self):
        harness = RdsHarness(self)
        runner = RdsRunner([DONE_STATE], describe=DESCRIBE)
        args = harness.parse(
            "run",
            "--genesis",
            str(GENESIS),
            *harness.fleet_flags(),
            "--query-db",
            "rds",
        )
        code = awsb.cmd_run(args, run=runner, interrupts=awsb.Interrupts())
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertEqual(harness.fleet()["db_modes"], ["rds"])
        self.assertIn("rds", harness.tfvars())
        self.assertFalse(runner.ran("DROP DATABASE"))
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertTrue((harness.run_dir("01-run") / "cloudwatch/rds.json").exists())


# REQ:querydb-rds-wiring
class CollectRdsTest(unittest.TestCase):
    def collect(self, runner: RdsRunner, t0=100.0, t1=160.0) -> Path:
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        cfg = awsb.RunConfig(tag="x", db_modes=("rds",))
        for collect in (
            awsb.collect_rds_metrics,
            awsb.collect_rds_insights,
            awsb.collect_rds_logs,
        ):
            collect(runner, cfg, RDS_OUTPUT, t0, t1, tmp)
        return tmp

    def test_metrics_cover_the_padded_window_for_the_instance(self):
        runner = RdsRunner([DONE_STATE])
        out = self.collect(runner)
        argv = next(c for c in runner.calls if "get-metric-data" in c)
        self.assertEqual(
            argv[argv.index("--start-time") + 1], "1970-01-01T00:00:40+00:00"
        )
        self.assertEqual(
            argv[argv.index("--end-time") + 1], "1970-01-01T00:03:40+00:00"
        )
        queries = json.loads(argv[argv.index("--metric-data-queries") + 1])
        names = [q["MetricStat"]["Metric"]["MetricName"] for q in queries]
        self.assertEqual(names, [metric for metric, _ in awsb.RDS_METRICS.values()])
        for name in ("ReadIOPS", "WriteIOPS", "ReadLatency", "DiskQueueDepth"):
            self.assertIn(name, names)
        for query in queries:
            metric = query["MetricStat"]["Metric"]
            self.assertEqual(metric["Namespace"], "AWS/RDS")
            self.assertEqual(
                metric["Dimensions"],
                [{"Name": "DBInstanceIdentifier", "Value": RDS_OUTPUT["identifier"]}],
            )
            self.assertEqual(query["MetricStat"]["Period"], 60)
        saved = json.loads((out / awsb.RDS_CLOUDWATCH_FILE).read_text())
        self.assertEqual(saved["identifier"], RDS_OUTPUT["identifier"])
        self.assertEqual(saved["metrics"]["CPUUtilization"]["values"], [100.0] * 3)

    def test_insights_ask_for_load_by_wait_event(self):
        runner = RdsRunner([DONE_STATE])
        out = self.collect(runner)
        argv = next(c for c in runner.calls if "get-resource-metrics" in c)
        self.assertEqual(argv[argv.index("--identifier") + 1], "db-ABCDEF")
        self.assertEqual(argv[argv.index("--service-type") + 1], "RDS")
        query = json.loads(argv[argv.index("--metric-queries") + 1])[0]
        self.assertEqual(query["Metric"], "db.load.avg")
        self.assertEqual(query["GroupBy"]["Group"], "db.wait_event")
        saved = json.loads((out / awsb.PI_FILE).read_text())
        self.assertEqual(saved["MetricList"][0]["Key"], "db.load.avg")

    def test_every_log_written_since_the_load_started_is_downloaded(self):
        logs = [
            {"LogFileName": "error/postgresql.log.10", "LastWritten": 50_000},
            {"LogFileName": "error/postgresql.log.11", "LastWritten": 100_000},
            {"LogFileName": "error/postgresql.log.12", "LastWritten": 9_000_000},
        ]
        runner = RdsRunner([DONE_STATE], logs=logs)
        out = self.collect(runner)
        argv = next(c for c in runner.calls if "describe-db-log-files" in c)
        self.assertEqual(argv[argv.index("--file-last-written") + 1], "100000")
        saved = sorted(p.name for p in (out / awsb.RDS_LOGS_DIR).iterdir())
        self.assertEqual(saved, ["postgresql.log.11", "postgresql.log.12"])
        text = (out / awsb.RDS_LOGS_DIR / "postgresql.log.11").read_text()
        self.assertEqual(text, "log of error/postgresql.log.11\n")
        download = next(c for c in runner.calls if "download-db-log-file-portion" in c)
        self.assertEqual(download[download.index("--starting-token") + 1], "0")

    def test_a_run_collects_the_rds_files_and_the_ec2_balance(self):
        harness = RdsHarness(self)
        logs = [{"LogFileName": "error/postgresql.log.x", "LastWritten": 1_000_000}]
        runner = harness.up_rds(self, logs=logs)
        harness.run(runner, "--query-db", "rds")
        run_dir = harness.run_dir("01-rds")
        for name in (awsb.RDS_CLOUDWATCH_FILE, awsb.PI_FILE, awsb.EC2_NODE0_FILE):
            self.assertTrue((run_dir / name).exists(), name)
        self.assertTrue((run_dir / awsb.RDS_LOGS_DIR / "postgresql.log.x").exists())

    def test_a_colocated_run_collects_no_rds_files(self):
        harness = RdsHarness(self, modes="colocated,rds")
        runner = harness.up_rds(self)
        harness.run(runner, "--query-db", "colocated")
        run_dir = harness.run_dir("01-colocated")
        self.assertFalse((run_dir / awsb.RDS_CLOUDWATCH_FILE).exists())
        self.assertTrue((run_dir / awsb.EC2_NODE0_FILE).exists())
        self.assertFalse(runner.ran("get-resource-metrics"))

    def test_a_failed_rds_call_does_not_stop_the_other_collections(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        original = runner.aws

        def aws(argv):
            if "get-resource-metrics" in argv:
                return completed(returncode=254, stderr="pi denied")
            return original(argv)

        with unittest.mock.patch.object(runner, "aws", side_effect=aws):
            self.assertEqual(harness.run(runner, "--query-db", "rds"), awsb.EXIT_OK)
        run_dir = harness.run_dir("01-rds")
        self.assertTrue((run_dir / awsb.RDS_CLOUDWATCH_FILE).exists())
        self.assertFalse((run_dir / awsb.PI_FILE).exists())


# REQ:querydb-rds-wiring
class RdsValidityTest(unittest.TestCase):
    def evidence(self, **rds_min) -> dict:
        return {
            "coverage": {"ctl": 1.0, "node0": 1.0, "node1": 1.0},
            "journal_bytes": {},
            "clock_offset_ms": {},
            "digest_mismatch": {},
            "ebs_balance_min": {"EBSByteBalance%": 100.0, "EBSIOBalance%": 100.0},
            "rds_min": rds_min,
        }

    def check(self, evidence: dict) -> dict:
        result = valid_result()
        result["hosts"] = {}
        return awsb.check_validity_aws(
            result, {**aws_manifest(), "query_db": "rds"}, evidence
        )

    def test_full_metrics_are_quiet(self):
        verdict = self.check(self.evidence(CPUUtilization=12.0))
        self.assertFalse(verdict["noisy"])

    # TEST:cloudwatch-datapoint-missing-noisy
    def test_a_metric_without_datapoints_is_noisy_not_invalid(self):
        verdict = self.check(self.evidence(ReadIOPS=None))
        self.assertTrue(verdict["valid"])
        self.assertTrue(verdict["noisy"])
        self.assertIn("rds ReadIOPS has no datapoint", verdict["reasons"][0])

    def test_a_balance_below_100_is_noisy(self):
        verdict = self.check(self.evidence(**{"EBSByteBalance%": 80.0}))
        self.assertTrue(verdict["valid"])
        self.assertIn("rds EBSByteBalance% fell to 80%", verdict["reasons"][0])

    def test_uncollected_metrics_are_noisy(self):
        verdict = self.check(self.evidence())
        self.assertTrue(verdict["valid"])
        self.assertIn("rds metrics not collected", verdict["reasons"])

    def test_a_colocated_run_has_no_rds_reasons(self):
        evidence = self.evidence()
        del evidence["rds_min"]
        result = valid_result()
        result["hosts"] = {}
        verdict = awsb.check_validity_aws(result, aws_manifest(), evidence)
        self.assertFalse(verdict["noisy"])

    def test_evidence_reads_the_rds_file_of_an_rds_run_only(self):
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        manifest = {**aws_manifest(), "query_db": "rds"}
        _, evidence = awsb.load_evidence(tmp, manifest, 100.0, 160.0)
        self.assertEqual(evidence["rds_min"], {})
        path = tmp / awsb.RDS_CLOUDWATCH_FILE
        path.parent.mkdir()
        metrics = {
            "ReadIOPS": {"timestamps": [60.0, 120.0], "values": [5.0, 3.0]},
            "WriteIOPS": {"timestamps": [], "values": []},
        }
        path.write_text(json.dumps({"period_s": 60, "metrics": metrics}))
        _, evidence = awsb.load_evidence(tmp, manifest, 100.0, 160.0)
        self.assertEqual(evidence["rds_min"], {"ReadIOPS": 3.0, "WriteIOPS": None})
        _, evidence = awsb.load_evidence(tmp, aws_manifest(), 100.0, 160.0)
        self.assertNotIn("rds_min", evidence)


# REQ:querydb-result-block
class QueryDbMetaTest(unittest.TestCase):
    def setUp(self):
        self.evidence = {"pg_settings": {"pg_stat_ssl": {"true": 2}}}

    def test_rds_records_the_class_engine_store_tls_and_tuning(self):
        manifest = {**aws_manifest(), "query_db": "rds", "rds_spec": rds_spec()}
        meta = awsb.query_db_meta(manifest, self.evidence)
        self.assertEqual(meta["mode"], "rds")
        self.assertEqual(meta["engine"], "postgres 18.2")
        self.assertEqual(meta["instance_class"], "db.m8g.4xlarge")
        self.assertEqual(
            meta["store"],
            {
                "type": "rds-gp3",
                "identifier": "espresso-bench-run1",
                "gb": 400,
                "iops": 12000,
                "mbps": 500,
            },
        )
        self.assertTrue(meta["tls"])
        self.assertEqual(meta["tuning"], awsb.PG_TUNING)

    def test_colocated_records_the_root_volume(self):
        manifest = aws_manifest()
        meta = awsb.query_db_meta(manifest, self.evidence)
        node0 = next(h for h in manifest["hosts"] if h["name"] == "node0")
        self.assertEqual(meta["store"]["type"], "root")
        self.assertEqual(meta["store"]["iops"], node0["root_iops"])
        self.assertNotIn("instance_class", meta)

    def test_tls_is_off_without_settings_or_with_a_plain_backend(self):
        manifest = aws_manifest()
        self.assertFalse(awsb.query_db_meta(manifest, {})["tls"])
        plain = {"pg_settings": {"pg_stat_ssl": {"false": 1}}}
        self.assertFalse(awsb.query_db_meta(manifest, plain)["tls"])

    def test_the_result_and_summary_carry_the_block(self):
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        manifest = write_collected_run(tmp)
        manifest = {**manifest, "query_db": "rds", "rds_spec": rds_spec()}
        netbench.write_json(tmp / "manifest.json", manifest)
        path = tmp / awsb.RDS_CLOUDWATCH_FILE
        series = {"timestamps": [60.0, 120.0, 180.0], "values": [1.0, 1.0, 1.0]}
        metrics = {metric: series for metric, _ in awsb.RDS_METRICS.values()}
        for balance in ("EBSIOBalance%", "EBSByteBalance%"):
            metrics[balance] = {**series, "values": [100.0] * 3}
        path.write_text(json.dumps({"period_s": 60, "metrics": metrics}))
        result = awsb.write_report(tmp)
        query_db = result["deployment"]["query_db"]
        self.assertEqual(query_db["mode"], "rds")
        self.assertEqual(query_db["instance_class"], "db.m8g.4xlarge")
        self.assertFalse(result["validity"]["noisy"], result["validity"]["reasons"])
        summary = (tmp / "summary.md").read_text()
        self.assertIn(
            "- Query DB: rds, postgres 18.2 on db.m8g.4xlarge, rds-gp3", summary
        )


# REQ:fleet-extend-rearm
class RdsExtendTest(unittest.TestCase):
    def extend(self, harness, runner, now=None) -> int:
        args = harness.parse("extend", str(harness.fleet_dir), "--ttl-min", "240")
        return awsb.cmd_extend(args, run=runner, now=now)

    def test_the_schedule_moves_with_the_expiry_and_keeps_its_target(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        now = datetime.now(UTC)
        mark = len(runner.calls)
        self.assertEqual(self.extend(harness, runner, now), awsb.EXIT_OK)
        update = next(c for c in runner.calls[mark:] if "update-schedule" in c)
        new = (now + timedelta(minutes=240)).replace(microsecond=0)
        reaper = new - timedelta(seconds=awsb.RDS_REAPER_LEAD_S)
        self.assertEqual(
            update[update.index("--schedule-expression") + 1],
            f"at({reaper:%Y-%m-%dT%H:%M:%S})",
        )
        target = json.loads(update[update.index("--target") + 1])
        self.assertEqual(target, SCHEDULE_TARGET)
        group = update[update.index("--group-name") + 1]
        self.assertEqual(group, RDS_OUTPUT["schedule_group"])
        commands = [" ".join(c) for c in runner.calls[mark:]]
        rearm = next(i for i, c in enumerate(commands) if "shutdown -c" in c)
        moved = next(i for i, c in enumerate(commands) if "update-schedule" in c)
        tagged = next(i for i, c in enumerate(commands) if "tag-resources" in c)
        self.assertLess(rearm, moved)
        self.assertLess(moved, tagged)

    def test_an_instance_the_schedule_already_deleted_fails_the_extend_unchanged(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        original = runner.aws

        def aws(argv):
            if "describe-db-instances" in argv:
                return completed(returncode=254, stderr="DBInstanceNotFound")
            return original(argv)

        mark = len(runner.calls)
        before = harness.fleet()
        with (
            unittest.mock.patch.object(runner, "aws", side_effect=aws),
            self.assertRaisesRegex(awsb.RemoteError, "DBInstanceNotFound"),
        ):
            self.extend(harness, runner)
        self.assertEqual(ssh_calls(runner, mark), [])
        self.assertFalse(runner.ran("update-schedule"))
        self.assertEqual(harness.fleet(), before)

    def test_the_bound_counts_the_rds_instance(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        before = harness.fleet()["estimate"]["bound_usd"]
        self.extend(harness, runner)
        manifest = harness.fleet()
        expires = datetime.fromisoformat(manifest["expires_at"])
        self.assertGreater(manifest["estimate"]["bound_usd"], before)
        self.assertAlmostEqual(
            manifest["estimate"]["bound_usd"], awsb.fleet_bound_usd(manifest, expires)
        )


# REQ:fleet-down
class RdsDownTest(unittest.TestCase):
    def down(self, harness, runner) -> int:
        args = harness.parse("down", str(harness.fleet_dir), "--yes")
        return awsb.cmd_down(args, run=runner)

    def cost(self, harness) -> dict:
        return json.loads((harness.fleet_dir / "cost.json").read_text())

    def test_the_actual_cost_bills_the_rds_from_create_to_the_logged_delete(self):
        harness = RdsHarness(self)
        deleted = datetime(2026, 9, 30, 13, 0, tzinfo=UTC)
        runner = harness.up_rds(
            self,
            events=[
                {
                    "Message": "DB instance shutdown",
                    "Date": "2026-09-30T11:40:00+00:00",
                },
                {"Message": "DB instance deleted", "Date": deleted.isoformat()},
            ],
        )
        self.assertEqual(self.down(harness, runner), awsb.EXIT_OK)
        cost = self.cost(harness)
        rds_s = (deleted - datetime.fromisoformat(RDS_CREATED)).total_seconds()
        manifest = harness.fleet()
        self.assertAlmostEqual(
            cost["actual"], awsb.manifest_cost(manifest, cost["duration_s"], rds_s)
        )
        self.assertGreater(
            cost["actual"], awsb.manifest_cost(manifest, cost["duration_s"], 0.0)
        )
        self.assertTrue(runner.ran("describe-events", "espresso-bench-fleet1"))

    # EDGE:reaper-fires-during-down
    def test_a_reaper_delete_before_down_still_prices_to_the_event(self):
        harness = RdsHarness(self)
        deleted = datetime(2026, 9, 30, 12, 55, tzinfo=UTC)
        runner = harness.up_rds(
            self,
            events=[{"Message": "DB instance deleted", "Date": deleted.isoformat()}],
        )
        self.assertEqual(self.down(harness, runner), awsb.EXIT_OK)
        cost = self.cost(harness)
        rds_s = (deleted - datetime.fromisoformat(RDS_CREATED)).total_seconds()
        self.assertAlmostEqual(
            cost["actual"],
            awsb.manifest_cost(harness.fleet(), cost["duration_s"], rds_s),
        )

    def test_without_a_delete_event_the_driver_clock_ends_the_bill(self):
        cfg = awsb.RunConfig(tag="x")
        fallback = datetime(2026, 9, 30, 13, 30, tzinfo=UTC)
        runner = RdsRunner([DONE_STATE])
        got = awsb.rds_delete_time(runner, cfg, "espresso-bench-fleet1", fallback)
        self.assertEqual(got, fallback)
        argv = next(c for c in runner.calls if "describe-events" in c)
        self.assertEqual(argv[argv.index("--source-type") + 1], "db-instance")
        self.assertEqual(argv[argv.index("--event-categories") + 1], "deletion")

    def test_only_deletion_messages_end_the_bill(self):
        cfg = awsb.RunConfig(tag="x")
        runner = RdsRunner(
            [DONE_STATE],
            events=[
                {
                    "Message": "DB instance shutdown",
                    "Date": "2026-09-30T13:10:00+00:00",
                },
                {"Message": "DB instance deleted", "Date": "2026-09-30T13:00:00+00:00"},
            ],
        )
        fallback = datetime(2026, 9, 30, 14, 0, tzinfo=UTC)
        self.assertEqual(
            awsb.rds_delete_time(runner, cfg, "x", fallback),
            datetime(2026, 9, 30, 13, 0, tzinfo=UTC),
        )

    def test_a_fleet_that_failed_before_the_instance_was_seen_counts_from_launch(self):
        harness = RdsHarness(self)
        runner = RdsRunner(
            [DONE_STATE],
            describe=DESCRIBE,
            instances=[db_instance(applied="pending-reboot")],
            events=[
                {"Message": "DB instance deleted", "Date": "2026-09-30T15:00:00+00:00"}
            ],
        )
        with unittest.mock.patch.multiple(
            awsb, RDS_POLL_S=0.0, RDS_SYNC_TIMEOUT_S=0.05
        ):
            self.assertEqual(harness.up(runner), awsb.EXIT_FAILED)
        self.assertGreater(self.cost(harness)["actual"], 0)


# REQ:querydb-rds-wiring
class TofuRdsValidateTest(unittest.TestCase):
    """TEST:tofu-validate-modes-ok: `tofu validate` and an offline plan with the rds variable
    set and null. Skips without tofu or when the providers cannot be installed."""

    def run_tofu(self, tfvars_extra: dict) -> None:
        tofu = shutil.which("tofu")
        if tofu is None:
            self.skipTest("tofu/opentofu not on PATH")
        with tempfile.TemporaryDirectory() as tmp:
            terraform = Path(tmp) / "terraform"
            shutil.copytree(awsb.TERRAFORM_SRC, terraform)
            user_data = terraform / "ctl-user-data.sh"
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
                "hosts": {
                    "ctl": {
                        "role": "ctl",
                        "instance_type": "c8g.2xlarge",
                        "root_gb": 40,
                        "root_iops": 3000,
                        "root_mbps": 125,
                        "user_data_path": str(user_data),
                    }
                },
                **tfvars_extra,
            }
            (terraform / "terraform.tfvars.json").write_text(json.dumps(tfvars))
            env = {
                **os.environ,
                "AWS_ACCESS_KEY_ID": "test",
                "AWS_SECRET_ACCESS_KEY": "test",
                "AWS_REGION": "eu-west-1",
            }

            def tofu_run(*args: str) -> subprocess.CompletedProcess:
                return subprocess.run(
                    [tofu, f"-chdir={terraform}", *args],
                    capture_output=True,
                    text=True,
                    env=env,
                    check=False,
                )

            init = tofu_run("init", "-input=false")
            if init.returncode != 0:
                self.skipTest(f"tofu init cannot install the providers: {init.stderr}")
            validate = tofu_run("validate")
            self.assertEqual(validate.returncode, 0, validate.stderr)
            plan = tofu_run("plan", "-input=false", "-refresh=false")
            self.assertEqual(plan.returncode, 0, plan.stderr)

    def test_rds_set(self):
        expires = datetime(2026, 1, 1, tzinfo=UTC)
        self.run_tofu(awsb.rds_tfvars(rds_spec(), "secret-pass", expires))

    def test_rds_null(self):
        self.run_tofu({"rds": None})

    def test_a_volume_below_the_baseline_fails_validation(self):
        expires = datetime(2026, 1, 1, tzinfo=UTC)
        tfvars = awsb.rds_tfvars(rds_spec(gb=100), "secret-pass", expires)
        with self.assertRaisesRegex(AssertionError, "400 GiB"):
            self.run_tofu(tfvars)
