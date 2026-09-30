"""Tests for the fleet lifecycle of `aws-bench`: `up`, `run --fleet`, `extend` and `down`.

Reuses the fake runners of `fakes` and the fixtures of `test_aws_bench`.

    just py::test
"""

import contextlib
import getpass
import io
import json
import os
import re
import shlex
import shutil
import stat
import subprocess
import tempfile
import unittest
import unittest.mock
from dataclasses import replace
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import ClassVar

import netbench
from fakes import (
    FakeClock,
    FakeRunner,
    FleetRunner,
    completed,
    two_node_hosts_info,
)
from test_aws_bench import (
    DESCRIBE,
    DONE_STATE,
    DOTENV_TEXT,
    STATUS_DESCRIBE,
    aws_manifest,
    awsb,
    fake_image,
    fake_images,
    fake_preflight,
    isolated_env,
    price_response,
    setUpModule,  # noqa: F401  (unittest runs it for this module too)
    tag_runner,
    temp_dir,
    valid_result,
    write_collected_run,
)


def fake_report(run_dir: Path, baseline=None) -> dict:
    # The real report reads these keys from the run manifest a driver command wrote.
    awsb.deployment_meta(
        netbench.read_json(run_dir / "manifest.json"), {"clock_offset_ms": {}}
    )
    result = valid_result()
    netbench.write_json(run_dir / "result.json", result)
    return result


class FleetHarness:
    """A temp working dir with its own out root, and the argv of `up` and `run --fleet` for one
    fleet."""

    def __init__(self, test: unittest.TestCase, name: str = "fleet1"):
        self.tmp = temp_dir(test)
        isolated_env(test, self.tmp, name)
        self.name = name
        self.out = awsb.OUT_ROOT
        self.fleet_dir = self.out / name
        patches = [
            unittest.mock.patch.object(
                awsb, "preflight", return_value=fake_preflight()
            ),
            unittest.mock.patch.object(awsb, "write_report", fake_report),
            unittest.mock.patch.object(awsb, "read_dotenv", return_value=DOTENV_TEXT),
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
            "--yes",
        ]

    def up_args(self, *extra: str) -> "awsb.argparse.Namespace":
        return self.parse("up", *self.fleet_flags(), *extra)

    def single_shot_args(self) -> "awsb.argparse.Namespace":
        return self.parse("run", *self.fleet_flags())

    def run_args(self, *extra: str) -> "awsb.argparse.Namespace":
        return self.parse(
            "run",
            "--fleet",
            str(self.fleet_dir),
            "--yes",
            *extra,
        )

    def up(self, runner, *extra: str) -> int:
        return awsb.cmd_up(
            self.up_args(*extra), run=runner, interrupts=awsb.Interrupts(FakeClock())
        )

    def run(self, runner, *extra: str, interrupts: "awsb.Interrupts | None" = None):
        return awsb.cmd_run(
            self.run_args(*extra),
            run=runner,
            interrupts=interrupts or awsb.Interrupts(FakeClock()),
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

    def up_fleet(self, test: unittest.TestCase) -> FleetRunner:
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        test.assertEqual(self.up(runner), awsb.EXIT_OK)
        return runner


def ssh_calls(runner: FleetRunner, since: int = 0) -> list[str]:
    return [" ".join(call) for call in runner.calls[since:] if call[0] == "ssh"]


# REQ:fleet-up-idle
class UpTest(unittest.TestCase):
    def test_up_ends_idle_without_starting_anything(self):
        harness = FleetHarness(self)
        runner = FleetRunner([DONE_STATE])
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
        harness.up(FleetRunner([DONE_STATE]))
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
        harness.up(FleetRunner([DONE_STATE]))
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
        harness.up(FleetRunner([DONE_STATE]))
        estimate = harness.fleet()["estimate"]
        self.assertEqual(estimate["ttl_s"], 180 * 60)
        rate = awsb.estimate_rate(estimate)
        self.assertGreater(rate, 0.355 + 2 * 0.71)
        egress = next(l["usd"] for l in estimate["lines"] if l["item"] == "egress")
        expected = rate * (180 * 60 + awsb.BOOT_ALLOWANCE_S) / 3600 + egress
        self.assertAlmostEqual(estimate["bound_usd"], expected)

    def test_apply_failure_destroys_and_exits_3(self):
        harness = FleetHarness(self)
        runner = FleetRunner(
            [DONE_STATE],
            apply=completed(returncode=1, stderr="Error: InsufficientInstanceCapacity"),
        )
        self.assertEqual(harness.up(runner), awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertIn("InsufficientInstanceCapacity", harness.driver_log())

    def test_apply_failure_with_a_failed_destroy_exits_4(self):
        harness = FleetHarness(self)
        runner = FleetRunner(
            [DONE_STATE],
            apply=completed(returncode=1, stderr="Error: boom"),
            destroys=[completed(returncode=1, stderr="locked")],
        )
        self.assertEqual(harness.up(runner), awsb.EXIT_LEFTOVER)
        last = harness.driver_log().splitlines()[-1]
        self.assertTrue(last.endswith(f"aws-bench down {harness.fleet_dir}"))

    def test_declined_prompt_creates_nothing(self):
        harness = FleetHarness(self)
        runner = FleetRunner([DONE_STATE])
        args = harness.up_args()
        args.yes = False
        with self.assertRaisesRegex(awsb.Refused, "not confirmed"):
            awsb.cmd_up(args, run=runner, interrupts=awsb.Interrupts(FakeClock()))
        self.assertFalse(runner.ran("tofu", "apply"))

    def test_up_needs_explicit_minutes(self):
        harness = FleetHarness(self)
        runner = FleetRunner([DONE_STATE])
        with self.assertRaisesRegex(awsb.Refused, "--ttl-min"):
            harness.up(runner, "--ttl-min", "auto")
        self.assertFalse(runner.ran("tofu", "apply"))

    def test_bound_above_max_usd_is_refused(self):
        harness = FleetHarness(self)
        runner = FleetRunner([DONE_STATE])
        with self.assertRaisesRegex(awsb.Refused, "exceeds --max-usd"):
            harness.up(runner, "--max-usd", "1")
        self.assertFalse(runner.ran("tofu", "apply"))


# REQ:fleet-run-on-fleet
class RunOnFleetTest(unittest.TestCase):
    def test_two_runs_reset_between_and_leave_the_fleet_idle(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        for name in ("01-colocated", "02-colocated"):
            mark = len(runner.calls)
            self.assertEqual(harness.run(runner), awsb.EXIT_OK)
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
        self.assertTrue(rows[0].startswith("| fleet1/01-colocated |"))
        self.assertTrue(rows[1].startswith("| fleet1/02-colocated |"))
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
        interrupts = awsb.Interrupts(FakeClock())

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
    def test_single_shot_does_not_reset_and_writes_its_index_row(self):
        harness = FleetHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        args = harness.single_shot_args()
        code = awsb.cmd_run(args, run=runner, interrupts=awsb.Interrupts(FakeClock()))
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertFalse(runner.ran("find /data/journal"))
        self.assertEqual(harness.fleet()["db_modes"], ["colocated"])
        (run_row,) = harness.index()
        self.assertIn("| colocated |", run_row)
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

    def test_node_env_belongs_to_one_run(self):
        harness = FleetHarness(self)
        harness.up_fleet(self)
        manifest = harness.fleet()
        self.assertEqual(
            awsb.fleet_run_config(harness.run_args(), manifest).node_env, ()
        )
        cfg = awsb.fleet_run_config(harness.run_args("--node-env", "A=1"), manifest)
        self.assertEqual(cfg.node_env, ("A=1",))
        with self.assertRaisesRegex(awsb.Refused, "run --fleet"):
            awsb.cmd_up(
                harness.up_args("--node-env", "A=1"), run=FleetRunner([DONE_STATE])
            )

    def test_mode_missing(self):
        harness = FleetHarness(self)
        harness.up_fleet(self)
        manifest = harness.fleet()
        cfg = awsb.fleet_run_config(harness.run_args(), manifest)
        cfg = awsb.dataclasses.replace(cfg, query_db="volume")
        with self.assertRaisesRegex(awsb.Refused, "was not provisioned"):
            awsb.check_run_allowed(manifest, cfg, datetime.now(UTC))

    def test_ttl_too_short_names_the_minutes(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        expires = datetime.now(UTC) + timedelta(minutes=10)
        harness.set_fleet(expires_at=awsb.expiry_stamp(expires))
        self.refused(
            harness, runner, r"\d\d min left.*needs up to \d\d: up a new fleet"
        )

    def test_the_lockout_window_counts_against_the_ttl(self):
        harness = FleetHarness(self)
        harness.up_fleet(self)
        manifest = harness.fleet()
        cfg = awsb.fleet_run_config(harness.run_args(), manifest)
        worst = awsb.estimate_run(manifest, cfg)["worst_s"]
        now = datetime.now(UTC)
        without_lockout = now + timedelta(seconds=worst + awsb.DESTROY_S + 60)
        with self.assertRaisesRegex(awsb.Refused, "up a new fleet"):
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
        ):
            self.refused(harness, runner, r"shape the fleet", *flags)

    def test_unknown_dir_is_not_a_fleet(self):
        harness = FleetHarness(self)
        with self.assertRaisesRegex(awsb.Refused, "not a fleet dir"):
            harness.run(FleetRunner([DONE_STATE]))

    def test_single_shot_needs_a_tag(self):
        harness = FleetHarness(self)
        args = harness.parse("run", "--nodes", "2")
        with self.assertRaisesRegex(awsb.Refused, "--tag"):
            awsb.cmd_run(args, run=FleetRunner([DONE_STATE]))


NEW_DIGEST = f"sha256:{'1' * 64}"


def resolve_tag(ref: str) -> dict:
    """A registry that knows `:other` at `NEW_DIGEST` and everything else at the fake digest."""
    image = fake_image(ref)
    return {**image, "digest": NEW_DIGEST} if ref.endswith(":other") else image


def respond_to_pulls(runner: FleetRunner, wrong: dict[str, str] | None = None) -> None:
    """Answers the pull script with what docker would report: the requested digests, or the
    ones in `wrong`. The record script echoes a `ready.json` holding the digests it was given."""
    replaced = wrong or {}

    def pull(argv: list[str]) -> subprocess.CompletedProcess:
        if not argv[-1].startswith("sudo timeout"):
            return runner.default(argv)
        script = shlex.split(argv[-1])[-1]
        pulls = re.findall(r"docker pull (\S+)@(\S+) >&2", script)
        names = re.findall(r"--arg name (\S+) ", script)
        digests = {
            name: f"{ref}@{replaced.get(name, digest)}"
            for name, (ref, digest) in zip(names, pulls, strict=True)
        }
        return completed(stdout=json.dumps(digests))

    def record(argv: list[str]) -> subprocess.CompletedProcess:
        if not argv[-1].startswith("sudo timeout"):
            return runner.default(argv)
        script = shlex.split(argv[-1])[-1]
        printf = next(l for l in script.splitlines() if l.startswith("printf"))
        digests = json.loads(shlex.split(printf)[2])
        return completed(stdout=json.dumps({"digests": digests}))

    runner.respond("docker pull", pull)
    runner.respond("digests.json.tmp", record)


# REQ:fleet-tag-pull
class TagPullTest(unittest.TestCase):
    def setUp(self):
        patch = unittest.mock.patch.object(
            awsb, "resolve_image", side_effect=resolve_tag
        )
        self.resolve = patch.start()
        self.addCleanup(patch.stop)

    def up_fleet(
        self, harness: FleetHarness, wrong: dict[str, str] | None = None
    ) -> FleetRunner:
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        respond_to_pulls(runner, wrong)
        self.assertEqual(harness.up(runner), awsb.EXIT_OK)
        self.resolve.reset_mock()
        return runner

    def test_a_different_tag_pulls_every_role_image_by_digest_on_every_host(self):
        harness = FleetHarness(self)
        runner = self.up_fleet(harness)
        mark = len(runner.calls)
        self.assertEqual(harness.run(runner, "--tag", "other"), awsb.EXIT_OK)
        commands = ssh_calls(runner, mark)
        pulls = [c for c in commands if "docker pull" in c]
        self.assertEqual(len(pulls), 3)
        for name in awsb.IMAGE_COMPONENTS:
            ref = f"{awsb.GHCR_ORG}/{name}:other@{NEW_DIGEST}"
            per_host = 2 if name == "espresso-node" else 1
            self.assertEqual(
                sum(c.count(f"docker pull {ref}") for c in pulls), per_host, name
            )
        for ref in awsb.SUPPORT_IMAGES.values():
            self.assertEqual(sum(c.count(f"docker pull {ref}@") for c in pulls), 1, ref)
        pull_at = min(i for i, c in enumerate(commands) if "docker pull" in c)
        record_at = max(i for i, c in enumerate(commands) if "digests.json.tmp" in c)
        reset_at = next(i for i, c in enumerate(commands) if "find /data/journal" in c)
        self.assertLess(pull_at, record_at)
        self.assertLess(record_at, reset_at)

    def test_digests_and_ready_json_are_rewritten_on_the_hosts_and_locally(self):
        harness = FleetHarness(self)
        runner = self.up_fleet(harness)
        mark = len(runner.calls)
        harness.run(runner, "--tag", "other")
        records = [c for c in ssh_calls(runner, mark) if "digests.json.tmp" in c]
        self.assertEqual(len(records), 3)
        for command in records:
            script = shlex.split(command)[-1]
            self.assertIn("ready.json.tmp", script)
            self.assertIn(NEW_DIGEST, script)
        ready = json.loads(
            (harness.fleet_dir / "hosts" / "node1" / "ready.json").read_text()
        )
        self.assertEqual(set(ready["digests"]), {"espresso-node"})
        self.assertTrue(ready["digests"]["espresso-node"].endswith(NEW_DIGEST))

    def test_the_new_images_become_the_fleets_and_the_runs(self):
        harness = FleetHarness(self)
        runner = self.up_fleet(harness)
        harness.run(runner, "--tag", "other")
        fleet = harness.fleet()
        self.assertEqual(fleet["config"]["tag"], "other")
        self.assertEqual(fleet["phase"], "idle")
        for name in awsb.IMAGE_COMPONENTS:
            self.assertEqual(fleet["images"][name]["digest"], NEW_DIGEST)
        run_dir = harness.fleet_dir / "runs" / "01-colocated"
        run = json.loads((run_dir / "manifest.json").read_text())
        self.assertEqual(run["images"], fleet["images"])
        self.assertEqual(run["config"]["tag"], "other")
        self.assertEqual(
            run["phase_seconds"]["pull"], [awsb.PULL_EXPECTED_S, awsb.PULL_MAX_S]
        )

    def test_the_same_tag_pulls_nothing(self):
        harness = FleetHarness(self)
        runner = self.up_fleet(harness)
        mark = len(runner.calls)
        self.assertEqual(harness.run(runner, "--tag", "t"), awsb.EXIT_OK)
        self.assertEqual(harness.run(runner), awsb.EXIT_OK)
        self.assertFalse(any("docker pull" in c for c in ssh_calls(runner, mark)))
        self.resolve.assert_not_called()

    def test_the_pulled_tag_is_not_pulled_again(self):
        harness = FleetHarness(self)
        runner = self.up_fleet(harness)
        harness.run(runner, "--tag", "other")
        mark = len(runner.calls)
        self.assertEqual(harness.run(runner, "--tag", "other"), awsb.EXIT_OK)
        self.assertFalse(any("docker pull" in c for c in ssh_calls(runner, mark)))
        self.assertEqual(harness.run(runner, "--tag", "t"), awsb.EXIT_OK)
        self.assertEqual(harness.fleet()["config"]["tag"], "t")

    # EDGE:fleet-pull-digest-mismatch
    def test_a_pulled_digest_that_differs_fails_before_the_reset(self):
        harness = FleetHarness(self)
        runner = self.up_fleet(harness, wrong={"espresso-node": f"sha256:{'2' * 64}"})
        before = harness.fleet()["images"]
        mark = len(runner.calls)
        self.assertEqual(harness.run(runner, "--tag", "other"), awsb.EXIT_FAILED)
        commands = ssh_calls(runner, mark)
        self.assertFalse(any("find /data/journal" in c for c in commands))
        self.assertFalse(any("digests.json.tmp" in c for c in commands))
        self.assertIn("pulled digests differ", harness.driver_log())
        self.assertEqual(harness.fleet()["images"], before)
        self.assertEqual(harness.fleet()["config"]["tag"], "t")
        self.assertEqual(harness.fleet()["phase"], "dirty")
        self.assertTrue(harness.lock().exists())
        self.assertFalse(runner.ran("tofu", "destroy"))

    def test_an_unknown_tag_is_refused_before_any_ssh(self):
        harness = FleetHarness(self)
        runner = self.up_fleet(harness)
        self.resolve.side_effect = awsb.Refused("image not found: x")
        mark = len(runner.calls)
        with self.assertRaisesRegex(awsb.Refused, "image not found"):
            harness.run(runner, "--tag", "nope")
        self.assertEqual(ssh_calls(runner, mark), [])
        self.assertFalse(harness.lock().exists())
        self.assertEqual(len(list((harness.fleet_dir / "runs").glob("*"))), 0)

    def test_a_tag_change_needs_the_ttl_for_the_pull(self):
        harness = FleetHarness(self)
        runner = self.up_fleet(harness)
        manifest = harness.fleet()
        cfg = awsb.fleet_run_config(harness.run_args(), manifest)
        needed = awsb.estimate_run(manifest, cfg)["worst_s"] + awsb.NOLOGIN_LEAD_S
        expires = datetime.now(UTC) + timedelta(
            seconds=needed + awsb.DESTROY_S + awsb.PULL_MAX_S - 30
        )
        harness.set_fleet(expires_at=awsb.expiry_stamp(expires))
        self.assertEqual(harness.run(runner), awsb.EXIT_OK)
        mark = len(runner.calls)
        harness.set_fleet(expires_at=awsb.expiry_stamp(expires))
        with self.assertRaisesRegex(awsb.Refused, "up a new fleet"):
            harness.run(runner, "--tag", "other")
        self.assertEqual(ssh_calls(runner, mark), [])


class ShipAgentsTest(unittest.TestCase):
    def test_both_agents_go_to_every_host_and_the_git_revision_is_recorded(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        mark = len(runner.calls)
        harness.run(runner)
        rsyncs = [c for c in runner.calls[mark:] if c[0] == "rsync"]
        sends = [c for c in rsyncs if not any(a.startswith("--exclude") for a in c)]
        agents = [c for c in sends if any(a.endswith("/netbench.py") for a in c)]
        self.assertEqual(len(agents), 3)
        for call in agents:
            self.assertTrue(any(a.endswith("/aws-bench") for a in call))
        per_run = [c for c in sends if any(a.endswith("/genesis.toml") for a in c)]
        self.assertEqual(len(per_run), 3)
        self.assertFalse(
            any(a.endswith(("/netbench.py", "/aws-bench")) for c in per_run for a in c)
        )
        run_dir = harness.fleet_dir / "runs" / "01-colocated"
        manifest = json.loads((run_dir / "manifest.json").read_text())
        self.assertEqual(manifest["git_rev"], "a" * 40)

    def test_the_run_records_the_revision_shipped_for_it(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        with unittest.mock.patch.object(awsb, "git_head", return_value="b" * 40):
            harness.run(runner)
        run_dir = harness.fleet_dir / "runs" / "01-colocated"
        manifest = json.loads((run_dir / "manifest.json").read_text())
        self.assertEqual(manifest["git_rev"], "b" * 40)
        self.assertEqual(harness.fleet()["git_rev"], "a" * 40)


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
        fleet = awsb.open_fleet(runner, harness.fleet_dir, awsb.Interrupts(FakeClock()))
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


def volume_runner(
    states: list[dict], volume_id: str | None = VOLUME_ID, **kwargs
) -> FleetRunner:
    """A `FleetRunner` whose `tofu output` has the `pg_volume_id` of a fleet with a Postgres
    volume."""
    runner = FleetRunner(states, **kwargs)

    def output(argv: list[str]) -> subprocess.CompletedProcess:
        outputs = json.loads(runner.default(argv).stdout)
        return completed(
            stdout=json.dumps({**outputs, "pg_volume_id": {"value": volume_id}})
        )

    runner.respond("output -json", output)
    return runner


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
            load=netbench.BenchConfig(submit_nodes=1),
        )
        self.assertEqual(
            tfvars(volume)["pg_volume"], {"gb": 400, "iops": 12000, "mbps": 500}
        )
        self.assertIsNone(tfvars(replace(volume, db_modes=("colocated",)))["pg_volume"])

    def test_up_makes_and_mounts_the_volume_on_the_query_host_only(self):
        harness = FleetHarness(self)
        runner = volume_runner([DONE_STATE])
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
        runner = volume_runner([DONE_STATE], describe=DESCRIBE)
        args = harness.parse(
            "run",
            *harness.fleet_flags(),
            "--query-db",
            "volume",
        )
        code = awsb.cmd_run(args, run=runner, interrupts=awsb.Interrupts(FakeClock()))
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
        runner = volume_runner([DONE_STATE], describe=DESCRIBE)
        runner.respond(
            "mkfs.ext4",
            lambda argv: completed(returncode=1, stderr=f"{BY_ID} missing after 60 s"),
        )
        self.assertEqual(harness.up(runner, "--db-modes", "volume"), awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertIn("missing after 60 s", harness.driver_log())

    def test_a_null_volume_output_fails_up(self):
        harness = FleetHarness(self)
        runner = volume_runner([DONE_STATE], volume_id=None, describe=DESCRIBE)
        self.assertEqual(harness.up(runner, "--db-modes", "volume"), awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertFalse(runner.ran("mkfs"))

    def test_the_run_resets_by_mounting_and_wiping_without_reformatting(self):
        harness = FleetHarness(self)
        runner = volume_runner([DONE_STATE], describe=DESCRIBE)
        harness.up(runner, "--db-modes", "volume")
        mark = len(runner.calls)
        self.assertEqual(harness.run(runner, "--query-db", "volume"), awsb.EXIT_OK)
        commands = ssh_calls(runner, mark)
        self.assertFalse(any("mkfs" in c for c in commands))
        reset = next(c for c in node0_calls(runner, mark) if "find /data/journal" in c)
        self.assertIn(f'mount -t ext4 -o "$(findmnt -no OPTIONS /)" {BY_ID}', reset)
        self.assertLess(reset.index("docker rm -f"), reset.index("mount -t"))
        self.assertLess(reset.index("mount -t"), reset.rindex("find /data/pg"))
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
            f"if ! mountpoint -q /data/pg; then\n"
            f"  find /data/pg -mindepth 1 -delete\n"
            f'  mount -t ext4 -o "$(findmnt -no OPTIONS /)" {BY_ID} /data/pg\nfi\n',
            script,
        )
        self.assertNotIn("umount", script)

    def test_volume_after_colocated_empties_the_root_copy_before_it_is_hidden(self):
        script = awsb.pg_store_script("volume", self.manifest)
        guard, _, mounted = script.partition("then\n")
        self.assertIn("! mountpoint -q /data/pg", guard)
        self.assertLess(
            mounted.index("find /data/pg -mindepth 1 -delete"),
            mounted.index("mount -t ext4"),
        )

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
        runner = volume_runner([DONE_STATE], describe=DESCRIBE)
        harness.up(runner, "--db-modes", "volume")
        manifest = harness.fleet()
        with_volume = awsb.manifest_cost(manifest, 3600.0)
        without = awsb.manifest_cost(
            {k: v for k, v in manifest.items() if k != "pg_volume"}, 3600.0
        )
        self.assertGreater(with_volume, without)


# REQ:fleet-cost
class RunEstimateTest(unittest.TestCase):
    def test_run_phases_have_no_provision_or_destroy(self):
        phases = awsb.run_phase_seconds(awsb.RunConfig(tag="x"))
        self.assertEqual(
            list(phases), ["pull", "reset", "services", "ready", "load", "collect"]
        )
        self.assertEqual(phases["reset"], (awsb.RESET_EXPECTED_S, awsb.RESET_MAX_S))
        self.assertEqual(phases["pull"], (0.0, 0.0))

    def test_a_tag_change_adds_the_pull_to_the_estimate(self):
        harness = FleetHarness(self)
        harness.up_fleet(self)
        manifest = harness.fleet()
        same = awsb.fleet_run_config(harness.run_args(), manifest)
        other = awsb.fleet_run_config(harness.run_args("--tag", "other"), manifest)
        self.assertEqual(
            awsb.run_phase_seconds(other, pull=True)["pull"],
            (awsb.PULL_EXPECTED_S, awsb.PULL_MAX_S),
        )
        gap = (
            awsb.estimate_run(manifest, other)["worst_s"]
            - awsb.estimate_run(manifest, same)["worst_s"]
        )
        self.assertEqual(gap, awsb.PULL_MAX_S)

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


# REQ:fleet-down
class DownTest(unittest.TestCase):
    def down(self, harness, runner, *extra: str) -> int:
        args = harness.parse("down", str(harness.fleet_dir), "--yes", *extra)
        return awsb.cmd_down(args, run=runner, clock=FakeClock())

    def test_down_destroys_and_prices(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        harness.run(runner)
        harness.run(runner)
        self.assertEqual(self.down(harness, runner), awsb.EXIT_OK)
        self.assertTrue(runner.ran("tofu", "destroy"))
        cost = json.loads((harness.fleet_dir / "cost.json").read_text())
        self.assertGreater(cost["actual"], 0)
        self.assertEqual(harness.fleet()["phase"], "done")
        log = harness.driver_log()
        self.assertRegex(
            log, r"destroyed; actual cost \$\d+\.\d\d \(bound \$\d+\.\d\d\); 2 runs"
        )
        self.assertIn("- 01-colocated: valid, $", log)
        self.assertIn("- 02-colocated: valid, $", log)
        self.assertFalse(harness.lock().exists())

    def test_leftover_after_failed_destroys_exits_4(self):
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
            awsb.cmd_down(args, run=runner, clock=FakeClock())
        self.assertFalse(runner.ran("tofu", "destroy"))


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
        self.assertIn("- fleet fleet1: phase idle", text)
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
    "engine_version": "18.2",
}
RDS_CREATED = "2026-09-30T11:05:00+00:00"


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


class RdsRunner(FleetRunner):
    """`FleetRunner` with the rds output of `tofu`, and the `aws rds`
    and CloudWatch calls an rds fleet makes. `instances` are the successive
    describe-db-instances answers; the last one repeats."""

    def __init__(
        self,
        *args,
        instances: list[dict] | None = None,
        events: list[dict] | None = None,
        logs: list[dict] | None = None,
        **kwargs,
    ):
        super().__init__(*args, **kwargs)
        self.instances = instances or [db_instance()]
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
        if "describe-events" in argv:
            return completed(stdout=json.dumps({"Events": self.events}))
        if "get-metric-data" in argv and "AWS/RDS" in " ".join(argv):
            return completed(stdout=self.metrics)
        if "describe-db-log-files" in argv:
            return completed(stdout=json.dumps({"DescribeDBLogFiles": self.logs}))
        if "download-db-log-file-portion" in argv:
            name = argv[argv.index("--log-file-name") + 1]
            return completed(stdout=f"log of {name}\n")
        if "reboot-db-instance" in argv:
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

    def test_other_iops_or_throughput_are_refused(self):
        self.refused("--pg-iops 12000", "--pg-iops", "6000")
        self.refused("--pg-mbps 500", "--pg-mbps", "250")

    def test_other_modes_skip_the_guards(self):
        awsb.check_rds_config(awsb.RunConfig(tag="x", pg_iops=1))


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
        with unittest.mock.patch.object(awsb, "RDS_ENGINE_VERSION", "18.1"):
            self.assertEqual(awsb.rds_orderable(runner, self.cfg, "eu-west-1a"), "18.1")

    def test_an_unorderable_class_is_refused(self):
        runner = orderable_runner(orderable(AvailabilityZones=[{"Name": "eu-west-1a"}]))
        with self.assertRaisesRegex(
            awsb.Refused, "db.m8g.4xlarge cannot run postgres 18"
        ):
            awsb.rds_orderable(runner, self.cfg, "eu-west-1b")

    def test_storage_iops_and_throughput_ranges_must_cover_the_request(self):
        for override in (
            {"MinStorageSize": 500},
            {"MaxIopsPerDbInstance": 3000},
            {"MinStorageThroughputPerDbInstance": 1000},
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
            "caller_account": awsb.ACCOUNT,
            "default_vpc": "vpc-1",
            "capable_az": "eu-west-1b",
            "_running_instance_types": [],
            "describe_instance_types": {},
            "vcpu_headroom": (40, 0, 256.0),
            "resolve_ami_arch": "arm64",
            "resolve_ami": "ami-1",
        }
        with contextlib.ExitStack() as stack:
            for name, value in stubs.items():
                stack.enter_context(
                    unittest.mock.patch.object(awsb, name, return_value=value)
                )
            stack.enter_context(
                unittest.mock.patch.object(awsb, "resolve_image", resolve)
            )
            stack.enter_context(
                unittest.mock.patch("shutil.which", return_value="/usr/bin/x")
            )
            pre = awsb.preflight(runner, cfg, awsb.plan_hosts(cfg))
        self.assertEqual(pre["rds_engine_version"], "18.4")
        self.assertIn("docker.io/library/postgres:18.4", resolved)
        self.assertEqual(
            pre["images"]["postgres"]["ref"], "docker.io/library/postgres:18.4"
        )

    def test_a_missing_docker_tag_names_the_minor(self):
        cfg = awsb.RunConfig(tag="x", db_modes=("rds",))

        def resolve(ref: str) -> dict:
            if ref.endswith("postgres:18.4"):
                raise awsb.Refused(f"image not found: {ref}")
            return image_info(ref)

        with (
            unittest.mock.patch.object(awsb, "resolve_image", resolve),
            self.assertRaisesRegex(
                awsb.Refused, "RDS_ENGINE_VERSION must name an older minor"
            ),
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
        runner = FakeRunner({("aws",): price_response(1.82)})
        self.assertEqual(awsb.fetch_rds_price(runner, "db.m8g.4xlarge"), 1.82)
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
                "productFamily": "Database Instance",
                "locationType": "AWS Region",
            },
        )

    def test_the_class_price_is_cached_with_the_instance_prices(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp) / "prices.json"
            cfg = awsb.RunConfig(tag="x", db_modes=("rds",))
            runner = FakeRunner({("aws",): price_response(1.82)})
            prices = awsb.resolve_prices(runner, cfg, cache, 0.0)
            self.assertEqual(prices["instances"]["db.m8g.4xlarge"]["usd_hour"], 1.82)
            again = FakeRunner()
            awsb.resolve_prices(again, cfg, cache, 60.0)
            self.assertEqual(again.calls, [])

    def test_a_run_needs_time_for_the_rds_delete(self):
        harness = RdsHarness(self)
        harness.up_rds(self)
        manifest = harness.fleet()
        cfg = awsb.fleet_run_config(harness.run_args("--query-db", "rds"), manifest)
        worst_s = awsb.estimate_run(manifest, cfg)["worst_s"]
        floor = worst_s + awsb.NOLOGIN_LEAD_S + awsb.DESTROY_S
        expires = datetime.fromisoformat(manifest["expires_at"])
        short = expires - timedelta(seconds=floor + awsb.RDS_DELETE_S - 60)
        with self.assertRaisesRegex(awsb.Refused, "up a new fleet"):
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
        self.assertEqual(parameters["shared_buffers"], "1048576")
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

    def test_a_single_shot_deletes_the_rds_before_its_hosts_end(self):
        """The tag expiry counts the provisioning time, the hosts' own timers do not."""
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        (tmp / "terraform").mkdir()
        netbench.write_json(
            tmp / "fleet.json", {"phase": "planned", "estimate": {"ttl_s": 3600.0}}
        )
        tfvars = {"expires_at": "x", "rds": {"delete_at": "x"}}
        netbench.write_json(tmp / "terraform" / "terraform.tfvars.json", tfvars)
        confirmed = datetime(2026, 9, 29, 16, 0, tzinfo=UTC)
        tf = unittest.mock.Mock()
        awsb.stamp_expiry(tmp, tf, confirmed)
        written = json.loads((tmp / "terraform" / "terraform.tfvars.json").read_text())
        self.assertEqual(written["expires_at"], "2026-09-29T17:08:00Z")
        self.assertEqual(written["rds"]["delete_at"], "2026-09-29T16:55:00")

    def test_a_fleet_without_rds_has_no_rds_variables(self):
        harness = FleetHarness(self)
        harness.up_fleet(self)
        path = harness.fleet_dir / "terraform" / "terraform.tfvars.json"
        tfvars = json.loads(path.read_text())
        self.assertNotIn("rds", tfvars)
        self.assertNotIn("rds_password", tfvars)
        self.assertNotIn("rds_spec", harness.fleet())

    def test_plan_renders_the_rds_variables_and_prices_them(self):
        out = isolated_env(self, temp_dir(self), "planned")
        argv = [
            "plan",
            "--nodes",
            "2",
            "--tag",
            "x",
            "--db-modes",
            "colocated,rds",
        ]
        args = awsb.parse_args(argv)
        args.argv = argv
        pre = {**fake_preflight(), "rds_engine_version": "18"}
        with unittest.mock.patch.object(awsb, "preflight", return_value=pre):
            self.assertEqual(awsb.cmd_plan(args, run=FleetRunner([])), awsb.EXIT_OK)
        tfvars = json.loads(
            (out / "planned/terraform/terraform.tfvars.json").read_text()
        )
        self.assertEqual(tfvars["rds"]["engine_version"], "18")
        manifest = json.loads((out / "planned/fleet.json").read_text())
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
        awsb.render_host_files(
            run_dir, cfg, harness.fleet(), two_node_hosts_info(), DOTENV_TEXT
        )
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

    def test_the_password_files_are_private_before_they_are_written(self):
        harness = RdsHarness(self)
        harness.up_rds(self)
        run_dir = harness.run_dir("01-rds")
        run_dir.mkdir(parents=True)
        cfg = awsb.dataclasses.replace(
            awsb.config_from_manifest(harness.fleet()["config"]), query_db="rds"
        )
        modes: dict[str, int | None] = {}
        write_text, write_json = Path.write_text, netbench.write_json

        def spy_text(path, *args, **kwargs):
            modes[f"{path.parent.name}/{path.name}"] = (
                mode(path) if path.exists() else None
            )
            return write_text(path, *args, **kwargs)

        def spy_json(path, data):
            modes[f"{path.parent.name}/{path.name}"] = (
                mode(path) if path.exists() else None
            )
            return write_json(path, data)

        with (
            unittest.mock.patch.object(Path, "write_text", spy_text),
            unittest.mock.patch.object(netbench, "write_json", spy_json),
        ):
            awsb.render_host_files(
                run_dir, cfg, harness.fleet(), two_node_hosts_info(), DOTENV_TEXT
            )
        self.assertEqual(modes["node0/node.env"], 0o600)
        self.assertEqual(modes["node0/pg.json"], 0o600)


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
            awsb.cmd_up(
                args,
                run=RdsRunner([DONE_STATE]),
                interrupts=awsb.Interrupts(FakeClock()),
            )
        self.assertIn(
            "1 rds instance, 1 subnet group, 1 parameter group, 1 schedule, 1 iam role",
            confirm.call_args.args[0],
        )

    def test_the_prompt_lists_the_extra_volume(self):
        harness = FleetHarness(self)
        args = harness.up_args("--db-modes", "colocated,volume")
        args.yes = False
        with unittest.mock.patch.object(awsb, "confirm", return_value=True) as confirm:
            awsb.cmd_up(
                args,
                run=volume_runner([DONE_STATE]),
                interrupts=awsb.Interrupts(FakeClock()),
            )
        self.assertIn("1 extra volume", confirm.call_args.args[0])

    # TEST:querydb-rds-pending-reboot-ok
    def test_pending_reboot_reboots_once_and_waits(self):
        harness = RdsHarness(self)
        instances = [
            db_instance(applied="pending-reboot"),
            db_instance(state="rebooting", applied="pending-reboot"),
            db_instance(applied="in-sync"),
        ]
        runner = harness.up_rds(self, instances=instances)
        self.assertEqual(runner.count("reboot-db-instance"), 1)
        self.assertEqual(harness.fleet()["phase"], "idle")

    def test_parameters_that_stay_pending_destroy_the_fleet(self):
        harness = RdsHarness(self)
        runner = RdsRunner(
            [DONE_STATE], instances=[db_instance(applied="pending-reboot")]
        )
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
            *harness.fleet_flags(),
            "--query-db",
            "rds",
        )
        code = awsb.cmd_run(args, run=runner, interrupts=awsb.Interrupts(FakeClock()))
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
        for collect in (
            awsb.collect_rds_metrics,
            awsb.collect_rds_logs,
        ):
            collect(runner, RDS_OUTPUT, t0, t1, tmp)
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
        for name in (awsb.RDS_CLOUDWATCH_FILE, awsb.EC2_NODE0_FILE):
            self.assertTrue((run_dir / name).exists(), name)
        self.assertTrue((run_dir / awsb.RDS_LOGS_DIR / "postgresql.log.x").exists())

    def test_a_colocated_run_collects_no_rds_files(self):
        harness = RdsHarness(self, modes="colocated,rds")
        runner = harness.up_rds(self)
        harness.run(runner, "--query-db", "colocated")
        run_dir = harness.run_dir("01-colocated")
        self.assertFalse((run_dir / awsb.RDS_CLOUDWATCH_FILE).exists())
        self.assertTrue((run_dir / awsb.EC2_NODE0_FILE).exists())
        self.assertFalse(runner.ran("describe-db-log-files"))

    def test_a_failed_rds_call_does_not_stop_the_other_collections(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        original = runner.aws

        def aws(argv):
            if "describe-db-log-files" in argv:
                return completed(returncode=254, stderr="logs denied")
            return original(argv)

        with unittest.mock.patch.object(runner, "aws", side_effect=aws):
            self.assertEqual(harness.run(runner, "--query-db", "rds"), awsb.EXIT_OK)
        run_dir = harness.run_dir("01-rds")
        self.assertTrue((run_dir / awsb.RDS_CLOUDWATCH_FILE).exists())
        self.assertFalse((run_dir / awsb.RDS_LOGS_DIR).exists())


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
        self.evidence = {"ssl_backends": {"true": 2}}

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

    def test_volume_records_the_ebs_volume(self):
        manifest = {
            **aws_manifest(),
            "query_db": "volume",
            "pg_volume": {"gb": 400, "iops": 12000, "mbps": 500},
            "pg_volume_id": VOLUME_ID,
        }
        meta = awsb.query_db_meta(manifest, self.evidence)
        self.assertEqual(meta["mode"], "volume")
        self.assertEqual(
            meta["store"],
            {
                "type": "ebs",
                "volume_id": VOLUME_ID,
                "gb": 400,
                "iops": 12000,
                "mbps": 500,
            },
        )
        self.assertNotIn("instance_class", meta)

    def test_write_report_succeeds_in_every_mode(self):
        for query_db in netbench.DbMode.__args__:
            with self.subTest(query_db):
                tmp = Path(tempfile.mkdtemp())
                self.addCleanup(shutil.rmtree, tmp)
                manifest = write_collected_run(tmp)
                manifest |= {
                    "query_db": query_db,
                    "pg_volume": {"gb": 400, "iops": 12000, "mbps": 500},
                    "pg_volume_id": VOLUME_ID,
                    "rds_spec": rds_spec(),
                }
                netbench.write_json(tmp / "manifest.json", manifest)
                self.assertEqual(
                    awsb.write_report(tmp)["deployment"]["query_db"]["mode"], query_db
                )

    def test_node_env_is_in_the_deployment_block(self):
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        manifest = write_collected_run(tmp)
        self.assertNotIn("node_env", awsb.write_report(tmp)["deployment"])
        del manifest["config"]["node_env"]
        netbench.write_json(tmp / "manifest.json", manifest)
        self.assertNotIn("node_env", awsb.write_report(tmp)["deployment"])
        manifest["config"]["node_env"] = ["A=1"]
        netbench.write_json(tmp / "manifest.json", manifest)
        self.assertEqual(awsb.write_report(tmp)["deployment"]["node_env"], ["A=1"])

    def test_tls_is_off_without_settings_or_with_a_plain_backend(self):
        manifest = aws_manifest()
        self.assertFalse(awsb.query_db_meta(manifest, {})["tls"])
        plain = {"ssl_backends": {"false": 1}}
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


# REQ:fleet-down
class RdsDownTest(unittest.TestCase):
    def down(self, harness, runner) -> int:
        args = harness.parse("down", str(harness.fleet_dir), "--yes")
        return awsb.cmd_down(args, run=runner, clock=FakeClock())

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
        fallback = datetime(2026, 9, 30, 13, 30, tzinfo=UTC)
        runner = RdsRunner([DONE_STATE])
        got = awsb.rds_delete_time(runner, "espresso-bench-fleet1", fallback)
        self.assertEqual(got, fallback)
        argv = next(c for c in runner.calls if "describe-events" in c)
        self.assertEqual(argv[argv.index("--source-type") + 1], "db-instance")
        self.assertEqual(argv[argv.index("--event-categories") + 1], "deletion")

    def test_only_deletion_messages_end_the_bill(self):
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
            awsb.rds_delete_time(runner, "x", fallback),
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
        self.assertEqual(harness.up(runner), awsb.EXIT_FAILED)
        self.assertGreater(self.cost(harness)["actual"], 0)


def tf_resource(kind: str, name: str) -> str:
    """The text of one resource block of main.tf, up to its closing brace at column 0."""
    source = (awsb.TERRAFORM_SRC / "main.tf").read_text()
    start = source.index(f'resource "{kind}" "{name}" {{')
    return source[start : source.index("\n}\n", start)]


# REQ:querydb-rds-wiring
class TerraformSourceTest(unittest.TestCase):
    """Properties of main.tf checked on its source, without `tofu plan`."""

    def test_the_delete_schedule_does_not_wait_for_the_instance(self):
        for kind, name in (
            ("aws_iam_role_policy", "scheduler"),
            ("aws_scheduler_schedule", "rds_delete"),
        ):
            self.assertNotIn("aws_db_instance", tf_resource(kind, name), name)
        self.assertIn(
            "depends_on = [aws_scheduler_schedule.rds_delete]",
            tf_resource("aws_db_instance", "this"),
        )

    def test_the_rds_arn_and_identifier_are_built_from_the_name(self):
        policy = tf_resource("aws_iam_role_policy", "scheduler")
        self.assertIn(
            "arn:aws:rds:${local.region}:${local.account_id}:db:${local.rds_name}",
            policy,
        )
        self.assertIn(
            "DbInstanceIdentifier   = local.rds_name",
            tf_resource("aws_scheduler_schedule", "rds_delete"),
        )

    def test_hosts_report_ec2_metrics_every_minute(self):
        self.assertIn("monitoring", tf_resource("aws_instance", "host"))


LIST_ROLES = ("aws", "--profile", "timeboost-dev", "iam", "list-roles")
ROLE_ARN = "arn:aws:iam::1:role/espresso-bench/espresso-bench-fleet1"
EXPIRES_LATER = "2026-09-29T17:00:00Z"
EXPIRES_PAST = "2026-09-29T15:00:00Z"
SWEEP_NOW = datetime(2026, 9, 29, 16, 0, tzinfo=UTC)


def resource_arn(service: str, resource: str) -> str:
    return f"arn:aws:{service}:eu-west-1:1:{resource}"


def mapping(arn: str, run: str, owner: str | None, expires: str | None) -> dict:
    tags = {awsb.TAG_RUN: run}
    if owner:
        tags[awsb.TAG_OWNER] = owner
    if expires:
        tags[awsb.TAG_EXPIRES] = expires
    return {"arn": arn, "tags": [{"Key": k, "Value": v} for k, v in tags.items()]}


def rds_fleet_mappings(
    run: str, owner: str, expires: str, instance_id: str = "i-1"
) -> list[dict]:
    name = f"espresso-bench-{run}"
    arns = [
        resource_arn("ec2", f"instance/{instance_id}"),
        resource_arn("ec2", "security-group/sg-1"),
        resource_arn("ec2", "key-pair/key-1"),
        resource_arn("ec2", "volume/vol-1"),
        resource_arn("rds", f"db:{name}"),
        resource_arn("rds", f"subgrp:{name}"),
        resource_arn("rds", f"pg:{name}"),
        resource_arn("scheduler", f"schedule-group/{name}"),
    ]
    return [mapping(arn, run, owner, expires) for arn in arns]


def instance_row(instance_id: str) -> dict:
    return {
        "id": instance_id,
        "type": "c8g.4xlarge",
        "state": "running",
        "launch": "2026-09-29T15:30:00+00:00",
        "reason": "",
    }


def rds_tag_runner(
    mappings: list[dict],
    instances: list[dict],
    roles: list[str] | None = None,
    rds_status: str = "available",
    role_missing: bool = False,
) -> FakeRunner:
    """The tag API, describe-instances, the rds and scheduler calls of a sweep, and the role
    listing: `roles` are the ARNs under the scheduler path."""
    runner = tag_runner(mappings, instances)
    prefix = ("aws", "--profile", "timeboost-dev")
    gone = completed(returncode=254, stderr="NoSuchEntity: gone")
    runner.responses = {
        LIST_ROLES: completed(stdout=json.dumps(roles or [])),
        (*prefix, "iam", "list-role-policies"): (
            gone if role_missing else completed(stdout='["delete-rds"]')
        ),
        (*prefix, "iam", "delete-role-policy"): completed(),
        (*prefix, "iam", "delete-role"): gone if role_missing else completed(),
        (*prefix, "rds", "describe-db-instances"): (
            completed(stdout=f"{rds_status}\n")
            if rds_status
            else completed(returncode=254, stderr="DBInstanceNotFound")
        ),
        **runner.responses,
    }
    return runner


def aws_verbs(runner: FakeRunner) -> list[tuple[str, str]]:
    return [(c[3], c[4]) for c in runner.calls if c[0] == "aws"]


# REQ:sweep-rds-volume
class ArnPartsTest(unittest.TestCase):
    def test_ec2_arns_split_on_the_slash(self):
        self.assertEqual(
            awsb.arn_parts(resource_arn("ec2", "instance/i-0abc")),
            ("instance", "i-0abc"),
        )

    def test_rds_arns_split_on_the_colon(self):
        for kind in ("db", "subgrp", "pg"):
            self.assertEqual(
                awsb.arn_parts(resource_arn("rds", f"{kind}:espresso-bench-f")),
                (kind, "espresso-bench-f"),
            )

    def test_scheduler_and_iam_arns(self):
        self.assertEqual(
            awsb.arn_parts(
                resource_arn("scheduler", "schedule-group/espresso-bench-f")
            ),
            ("schedule-group", "espresso-bench-f"),
        )
        self.assertEqual(
            awsb.arn_parts(ROLE_ARN), ("role", "espresso-bench/espresso-bench-fleet1")
        )
        self.assertEqual(awsb.role_fleet(ROLE_ARN), "fleet1")


# REQ:sweep-rds-volume
class SweepRdsTest(unittest.TestCase):
    def sweep(self, **kwargs) -> tuple[FakeRunner, list[str]]:
        mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_LATER)
        runner = rds_tag_runner(mappings, [], roles=[ROLE_ARN], **kwargs)
        return runner, awsb.sweep(runner, "eu-west-1", "fleet1")

    # TEST:sweep-rds-volume-ok
    def test_deletes_in_dependency_order_and_waits_for_the_instance(self):
        runner, arns = self.sweep()
        self.assertEqual(len(arns), 8)
        self.assertEqual(
            aws_verbs(runner),
            [
                ("resourcegroupstaggingapi", "get-resources"),
                ("scheduler", "delete-schedule-group"),
                ("ec2", "terminate-instances"),
                ("ec2", "wait"),
                ("rds", "describe-db-instances"),
                ("rds", "delete-db-instance"),
                ("rds", "wait"),
                ("rds", "delete-db-subnet-group"),
                ("rds", "delete-db-parameter-group"),
                ("ec2", "delete-volume"),
                ("ec2", "delete-security-group"),
                ("ec2", "delete-key-pair"),
                ("iam", "list-roles"),
                ("iam", "list-role-policies"),
                ("iam", "delete-role-policy"),
                ("iam", "delete-role"),
            ],
        )
        self.assertTrue(runner.ran("aws", "--profile", "timeboost-dev", "rds", "wait"))
        wait = next(c for c in runner.calls if c[4:5] == ["wait"] and c[3] == "rds")
        self.assertEqual(
            wait[wait.index("--db-instance-identifier") + 1], "espresso-bench-fleet1"
        )

    def test_the_role_policy_goes_before_the_role(self):
        runner, _ = self.sweep()
        policy = next(c for c in runner.calls if "delete-role-policy" in c)
        self.assertEqual(policy[policy.index("--policy-name") + 1], "delete-rds")
        self.assertEqual(
            policy[policy.index("--role-name") + 1], "espresso-bench-fleet1"
        )

    def test_an_instance_already_deleting_is_only_waited_for(self):
        runner, _ = self.sweep(rds_status="deleting")
        verbs = aws_verbs(runner)
        self.assertNotIn(("rds", "delete-db-instance"), verbs)
        self.assertIn(("rds", "wait"), verbs)

    def test_an_instance_the_schedule_deleted_needs_no_wait(self):
        runner, _ = self.sweep(rds_status="")
        verbs = aws_verbs(runner)
        self.assertNotIn(("rds", "delete-db-instance"), verbs)
        self.assertNotIn(("rds", "wait"), verbs)
        self.assertIn(("rds", "delete-db-subnet-group"), verbs)

    # TEST:sweep-iam-absent-ok
    def test_a_role_that_is_already_gone_is_tolerated(self):
        runner, _ = self.sweep(role_missing=True)
        self.assertIn(("iam", "delete-role"), aws_verbs(runner))

    def test_a_group_the_destroy_already_deleted_is_tolerated(self):
        mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_LATER)
        runner = rds_tag_runner(mappings, [], roles=[])
        runner.responses = {
            ("aws", "--profile", "timeboost-dev", "rds", "delete-db-subnet-group"): (
                completed(returncode=254, stderr="DBSubnetGroupNotFoundFault")
            ),
            ("aws", "--profile", "timeboost-dev", "rds", "delete-db-parameter-group"): (
                completed(returncode=254, stderr="DBParameterGroupNotFound")
            ),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "scheduler",
                "delete-schedule-group",
            ): (completed(returncode=254, stderr="ResourceNotFoundException")),
            **runner.responses,
        }
        awsb.sweep(runner, "eu-west-1", "fleet1")

    def test_another_fleets_role_is_left_alone(self):
        mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_LATER)
        other = "arn:aws:iam::1:role/espresso-bench/espresso-bench-other"
        runner = rds_tag_runner(mappings, [], roles=[other])
        awsb.sweep(runner, "eu-west-1", "fleet1")
        self.assertNotIn(("iam", "delete-role"), aws_verbs(runner))

    def test_a_fleet_without_rds_touches_no_iam(self):
        mappings = [
            mapping(resource_arn("ec2", "security-group/sg-1"), "fleet1", None, None)
        ]
        runner = rds_tag_runner(mappings, [])
        awsb.sweep(runner, "eu-west-1", "fleet1")
        self.assertFalse(any(service == "iam" for service, _ in aws_verbs(runner)))

    def test_no_iam_permission_lists_no_roles(self):
        runner = rds_tag_runner([], [])
        runner.responses[LIST_ROLES] = completed(
            returncode=254, stderr="AccessDenied: iam:ListRoles"
        )
        self.assertEqual(awsb.list_scheduler_roles(runner, "timeboost-dev"), [])

    def test_other_iam_errors_raise(self):
        runner = rds_tag_runner([], [])
        runner.responses[LIST_ROLES] = completed(returncode=254, stderr="Throttling")
        with self.assertRaisesRegex(awsb.RemoteError, "Throttling"):
            awsb.list_scheduler_roles(runner, "timeboost-dev")


# REQ:orphans-rds-expiry
class GroupRdsRunsTest(unittest.TestCase):
    def test_the_latest_expiry_of_a_partly_retagged_fleet_counts(self):
        mappings = [
            mapping(
                resource_arn("ec2", "security-group/sg-1"), "f", "bob", EXPIRES_PAST
            ),
            mapping(
                resource_arn("rds", "db:espresso-bench-f"), "f", "bob", EXPIRES_LATER
            ),
            mapping(resource_arn("ec2", "key-pair/key-1"), "f", "bob", EXPIRES_PAST),
        ]
        (tagged,) = awsb.group_runs(mappings, [], [])
        self.assertEqual(tagged["expires"], EXPIRES_LATER)

    def test_expiries_compare_as_times_not_strings(self):
        self.assertEqual(
            awsb.later("2026-09-29T09:00:00+00:00", "2026-09-29T10:00:00+01:00"),
            "2026-09-29T09:00:00+00:00",
        )
        self.assertIsNone(awsb.later(None, None))
        self.assertEqual(awsb.later(None, EXPIRES_PAST), EXPIRES_PAST)

    def test_a_role_joins_its_fleet_and_stands_alone_without_one(self):
        mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_LATER)
        lone = "arn:aws:iam::1:role/espresso-bench/espresso-bench-lost"
        runs = awsb.group_runs(
            mappings, [instance_row("i-1")], ["vol-1"], [ROLE_ARN, lone]
        )
        self.assertEqual([r["name"] for r in runs], ["lost", "fleet1"])
        self.assertEqual(runs[0]["arns"], [lone])
        self.assertIn(ROLE_ARN, runs[1]["arns"])

    def test_resource_counts_name_each_kind(self):
        mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_LATER)
        (tagged,) = awsb.group_runs(
            mappings, [instance_row("i-1")], ["vol-1"], [ROLE_ARN]
        )
        self.assertEqual(
            awsb.format_runs([tagged], {}, SWEEP_NOW)[2].split(" | ")[4],
            "1 db, 1 instance, 1 key-pair, 1 pg, 1 role, 1 schedule-group, "
            "1 security-group, 1 subgrp, 1 volume",
        )


# REQ:orphans-rds-expiry
class DestroyRdsOrphansTest(unittest.TestCase):
    def setUp(self):
        self.out = isolated_env(self, temp_dir(self))

    def destroy(self, runner, *flags: str) -> tuple[int, str]:
        parsed = awsb.parse_args(["destroy", "--orphans", *flags])
        text = io.StringIO()
        with contextlib.redirect_stdout(text):
            code = awsb.cmd_destroy(parsed, runner, SWEEP_NOW, FakeClock())
        return code, text.getvalue()

    def write_fleet(self, name: str, phase: str) -> None:
        (self.out / name).mkdir(parents=True)
        netbench.write_json(self.out / name / "fleet.json", {"phase": phase})

    # TEST:orphans-rds-expiry-ok
    def test_an_rds_fleet_past_expiry_is_listed_and_swept(self):
        mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_PAST)
        runner = rds_tag_runner(mappings, [instance_row("i-1")], roles=[ROLE_ARN])
        code, text = self.destroy(runner, "--yes")
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertIn("| fleet1 | bob |", text)
        self.assertIn(
            "1 db, 1 instance, 1 key-pair, 1 pg, 1 role, 1 schedule-group, 1 security-group, 1 subgrp, 1 volume",
            text,
        )
        self.assertTrue(text.splitlines()[2].endswith("| past expiry |"))
        verbs = aws_verbs(runner)
        self.assertIn(("rds", "delete-db-instance"), verbs)
        self.assertIn(("iam", "delete-role"), verbs)

    def test_the_roles_are_listed_once_to_find_and_once_to_sweep(self):
        mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_PAST)
        runner = rds_tag_runner(mappings, [instance_row("i-1")], roles=[ROLE_ARN])
        self.destroy(runner, "--yes")
        self.assertEqual(sum("list-roles" in call for call in runner.calls), 2)

    # TEST:orphans-live-fleet-kept-ok
    def test_a_live_fleet_of_this_user_with_state_is_kept(self):
        user = getpass.getuser()
        mappings = rds_fleet_mappings("fleet1", user, EXPIRES_LATER)
        runner = rds_tag_runner(mappings, [instance_row("i-1")], roles=[ROLE_ARN])
        self.write_fleet("fleet1", "idle")
        code, text = self.destroy(runner, "--yes")
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertIn("no orphaned", text)
        self.assertNotIn(("ec2", "terminate-instances"), aws_verbs(runner))
        self.assertNotIn(("iam", "delete-role"), aws_verbs(runner))

    def test_a_fleet_that_lost_its_state_is_swept_for_its_owner(self):
        mappings = rds_fleet_mappings("fleet1", getpass.getuser(), EXPIRES_LATER)
        runner = rds_tag_runner(mappings, [instance_row("i-1")], roles=[ROLE_ARN])
        code, text = self.destroy(runner, "--yes")
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertTrue(text.splitlines()[2].endswith("| no local state |"))

    def test_a_role_left_by_a_fleet_with_no_other_resources_is_swept(self):
        runner = rds_tag_runner([], [], roles=[ROLE_ARN])
        code, text = self.destroy(runner, "--yes")
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertTrue(text.splitlines()[2].endswith("| role without resources |"))
        self.assertIn("1 role", text)
        self.assertIn(("iam", "delete-role"), aws_verbs(runner))

    def test_a_role_of_a_fleet_being_provisioned_here_is_kept(self):
        runner = rds_tag_runner([], [], roles=[ROLE_ARN])
        self.write_fleet("fleet1", "applying")
        code, text = self.destroy(runner, "--yes")
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertIn("no orphaned", text)
        self.assertNotIn(("iam", "delete-role"), aws_verbs(runner))

    def test_a_failed_rds_delete_exits_4(self):
        mappings = rds_fleet_mappings("fleet1", "bob", EXPIRES_PAST)
        runner = rds_tag_runner(mappings, [], roles=[ROLE_ARN])
        runner.responses = {
            ("aws", "--profile", "timeboost-dev", "rds", "delete-db-instance"): (
                completed(returncode=254, stderr="InvalidDBInstanceState")
            ),
            **runner.responses,
        }
        code, _ = self.destroy(runner, "--yes")
        self.assertEqual(code, awsb.EXIT_LEFTOVER)


# REQ:sweep-rds-volume
class StatusStoresTest(unittest.TestCase):
    def status(self, harness, runner) -> str:
        runner.describe = STATUS_DESCRIBE
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            awsb.cmd_status(harness.parse("status", str(harness.fleet_dir)), runner)
        return out.getvalue()

    def test_an_rds_fleet_shows_the_instance_and_parameters(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        self.assertIn(
            "- rds espresso-bench-fleet1: available, parameters in-sync",
            self.status(harness, runner),
        )

    def test_an_instance_the_schedule_deleted_is_gone(self):
        harness = RdsHarness(self)
        runner = harness.up_rds(self)
        aws = runner.aws

        def deleted(argv: list[str]) -> subprocess.CompletedProcess:
            if "describe-db-instances" in argv:
                return completed(returncode=254, stderr="DBInstanceNotFound")
            return aws(argv)

        with unittest.mock.patch.object(runner, "aws", side_effect=deleted):
            text = self.status(harness, runner)
        self.assertIn("- rds espresso-bench-fleet1: gone", text)

    def test_a_pg_volume_shows_its_state(self):
        harness = FleetHarness(self)
        runner = volume_runner([DONE_STATE], describe=DESCRIBE)
        self.assertEqual(harness.up(runner, "--db-modes", "volume"), awsb.EXIT_OK)
        aws = runner.aws

        def volumes(argv: list[str]) -> subprocess.CompletedProcess:
            if "describe-volumes" in argv:
                return completed(stdout="in-use\n")
            return aws(argv)

        with unittest.mock.patch.object(runner, "aws", side_effect=volumes):
            text = self.status(harness, runner)
        self.assertIn(f"- pg volume {VOLUME_ID}: in-use", text)

    def test_a_fleet_without_stores_prints_neither(self):
        harness = FleetHarness(self)
        runner = harness.up_fleet(self)
        text = self.status(harness, runner)
        self.assertNotIn("- rds", text)
        self.assertNotIn("- pg volume", text)
