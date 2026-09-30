"""Tests for `aws-bench`: cost estimate, budget refusal, `plan` rendering, and the
`run` orchestration (apply failure, interrupt, destroy fallback, validity, cost, index).

Side effects go through a `FakeRunner` that maps an argv prefix to a canned
`subprocess.CompletedProcess`, and records every call, so a refused plan can be shown to have
made no `aws` call at all.

    just py::test
"""

import contextlib
import dataclasses
import gzip
import importlib.util
import io
import json
import logging
import os
import re
import shlex
import shutil
import signal
import subprocess
import tempfile
import threading
import unittest
import unittest.mock
from collections.abc import Iterable
from datetime import UTC, datetime
from http.server import BaseHTTPRequestHandler, HTTPServer
from importlib.machinery import SourceFileLoader
from pathlib import Path

import netbench
import test_netbench
from fakes import (
    FAKE_EPOCH,
    FULL_BALANCE,
    SLOW,
    FakeClock,
    FakeRegistry,
    FakeRunner,
    FakeSystem,
    FleetRunner,
    completed,
    host_info,
    metric_data,
    two_node_hosts_info,
)

SCRIPT = Path(__file__).with_name("aws-bench")
_spec = importlib.util.spec_from_loader(
    "aws_bench", SourceFileLoader("aws_bench", str(SCRIPT))
)
assert _spec is not None
awsb = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(awsb)


def parse_plan_args(argv: list[str]) -> "awsb.argparse.Namespace":
    full = ["plan", *argv]
    args = awsb.parse_args(full)
    args.argv = full
    return args


def cmd_plan_exit(args: "awsb.argparse.Namespace", run: "awsb.Runner") -> int:
    """Mirrors `main`'s single `Refused` catch, since these tests call `cmd_plan` directly."""
    try:
        return awsb.cmd_plan(args, FakeSystem(run=run))
    except awsb.Refused:
        return awsb.EXIT_REFUSED


def sts_response(account: str) -> subprocess.CompletedProcess:
    return completed(stdout=json.dumps({"Account": account}))


STS_CALL = ("aws", "--profile", "timeboost-dev", "sts", "get-caller-identity")


def shot_estimate(hosts: list, cfg: "awsb.RunConfig") -> "awsb.Estimate":
    return awsb.cost_estimate(hosts, cfg, None, *awsb.shot_seconds(cfg))


def isolated_env(test: unittest.TestCase, tmp: Path, name: str = "run1") -> Path:
    """Runs in `tmp` so that the relative `OUT_ROOT` lands there, with a fixed fleet name and
    no checkip call. Returns the out root."""
    cwd = os.getcwd()
    os.chdir(tmp)
    test.addCleanup(os.chdir, cwd)
    patches = [
        unittest.mock.patch.object(awsb, "default_run_name", return_value=name),
    ]
    for patch in patches:
        patch.start()
        test.addCleanup(patch.stop)
    return tmp / awsb.OUT_ROOT


def temp_dir(test: unittest.TestCase) -> Path:
    tmp = Path(tempfile.mkdtemp())
    test.addCleanup(shutil.rmtree, tmp)
    return tmp


# REQ:awsbench-topology
class GeometricStepsTest(unittest.TestCase):
    def test_default_ramp_ends_at_the_target(self):
        steps = awsb.geometric_steps(4.0, 1.5, 200.0)
        self.assertEqual(steps[0], 4.0)
        self.assertEqual(steps[-1], 200.0)
        self.assertEqual(list(steps), sorted(set(steps)))
        self.assertEqual(steps[:4], (4.0, 6.0, 9.0, 13.5))
        self.assertLess(steps[-2], 200.0)

    def test_run_config_uses_the_ramp(self):
        load = awsb.RunConfig(tag="x").load
        self.assertEqual(load.steps, awsb.geometric_steps(4.0, 1.5, 200.0))
        self.assertEqual(load.workers, awsb.SUBMIT_WORKERS)
        self.assertEqual(netbench.BenchConfig().workers, 6)


class PeersTest(unittest.TestCase):
    def test_excludes_self_and_node0(self):
        for n in range(3, 9):
            for i in range(1, n):
                result = awsb.peers(i, n)
                self.assertNotIn(i, result)
                self.assertNotIn(0, result)
                self.assertEqual(len(result), min(3, n - 2))

    def test_deterministic(self):
        self.assertEqual(awsb.peers(1, 5), [2, 3, 4])
        self.assertEqual(awsb.peers(4, 5), [1, 2, 3])

    def test_node0_gets_first_validators(self):
        self.assertEqual(awsb.peers(0, 5), [1, 2, 3])
        self.assertEqual(awsb.peers(0, 3), [1, 2])
        self.assertEqual(awsb.peers(0, 2), [1])

    # EDGE:awsbench-two-nodes
    def test_two_nodes_one_validator_empty(self):
        self.assertEqual(awsb.peers(1, 2), [])
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        self.assertEqual(awsb.plan_peers(hosts), {"node0": ["node1"], "node1": []})


class PlanHostsTest(unittest.TestCase):
    def test_roles_and_count(self):
        hosts = awsb.plan_hosts(awsb.RunConfig(tag="x", nodes=5))
        self.assertEqual(
            [h["name"] for h in hosts],
            ["ctl", "node0", "node1", "node2", "node3", "node4"],
        )
        self.assertEqual(hosts[0]["role"], "ctl")
        self.assertEqual(hosts[1]["role"], "query")
        self.assertTrue(all(h["role"] == "validator" for h in hosts[2:]))

    def test_query_node_gets_pg_iops(self):
        hosts = awsb.plan_hosts(
            awsb.RunConfig(
                tag="x",
                nodes=3,
                pg_iops=6000,
                pg_mbps=500,
                load=netbench.BenchConfig(submit_nodes=2),
            )
        )
        node0 = next(h for h in hosts if h["name"] == "node0")
        node1 = next(h for h in hosts if h["name"] == "node1")
        self.assertEqual(node0["root_iops"], 6000)
        self.assertEqual(node0["root_mbps"], 500)
        self.assertEqual(node1["root_iops"], awsb.GP3_BASELINE_IOPS)

    def test_pg_iops_default_is_12000(self):
        self.assertEqual(awsb.RunConfig(tag="x").pg_iops, 12000)
        for verb in ("plan", "run"):
            args = awsb.parse_args([verb, "--tag", "x"])
            self.assertEqual(args.pg_iops, 12000)
            self.assertEqual(args.pg_mbps, 500)
        cfg = awsb.RunConfig(
            tag="x", nodes=3, load=netbench.BenchConfig(submit_nodes=2)
        )
        self.assertEqual(awsb.plan_hosts(cfg)[1]["root_iops"], 12000)

    def test_rejects_single_node(self):
        with self.assertRaises(awsb.Refused):
            awsb.plan_hosts(awsb.RunConfig(tag="x", nodes=1))


class NodeRootGbTest(unittest.TestCase):
    def test_default_ramp_hits_the_minimum(self):
        cfg = awsb.RunConfig(tag="x")
        self.assertEqual(awsb.node_root_gb(cfg, query=False), 100)
        self.assertEqual(awsb.node_root_gb(cfg, query=True), 100)

    def test_scales_with_offered_payload(self):
        load = netbench.BenchConfig(steps=(100.0, 200.0), step_s=300)
        cfg = awsb.RunConfig(tag="x", load=load)
        self.assertEqual(awsb.node_root_gb(cfg, query=False), 20 + 2 * 90)
        self.assertEqual(awsb.node_root_gb(cfg, query=True), 20 + 3 * 90)
        load = netbench.BenchConfig(steps=(1000.0,), step_s=300)
        cfg = awsb.RunConfig(tag="x", load=load)
        self.assertEqual(awsb.node_root_gb(cfg, query=False), 620)
        self.assertEqual(awsb.node_root_gb(cfg, query=True), 920)

    def test_override(self):
        cfg = awsb.RunConfig(tag="x", root_gb="250")
        self.assertEqual(awsb.node_root_gb(cfg, query=True), 250)


class PhaseSecondsTest(unittest.TestCase):
    def test_load_seconds(self):
        load = netbench.BenchConfig(
            steps=(4.0, 8.0), step_s=30, warmup_s=60, tx_timeout_s=30
        )
        self.assertEqual(
            awsb.load_seconds(load), 60 + 3 * 30 + 2 * 30 + netbench.DRAIN_SLACK_S
        )

    def test_keep_going_flag(self):
        argv = ["run", "--tag", "x"]
        self.assertFalse(awsb.config_from_args(awsb.parse_args(argv)).load.keep_going)
        args = awsb.parse_args([*argv, "--keep-going"])
        self.assertTrue(awsb.config_from_args(args).load.keep_going)

    def test_load_seconds_keep_going(self):
        load = netbench.BenchConfig(
            steps=(4.0, 8.0), step_s=30, warmup_s=60, tx_timeout_s=30, keep_going=True
        )
        self.assertEqual(
            awsb.load_seconds(load), 60 + 2 * 30 + netbench.CATCHUP_TIMEOUT_S + 30
        )

    def test_worst_uses_ready_timeout_and_collect_max(self):
        expected_s, worst_s = awsb.shot_seconds(awsb.RunConfig(tag="x"))
        self.assertEqual(
            worst_s - expected_s,
            awsb.READY_TIMEOUT_S
            - awsb.READY_EXPECTED_S
            + awsb.COLLECT_MAX_S
            - awsb.COLLECT_EXPECTED_S,
        )


# REQ:awsbench-cost-bound
class EstimateCostTest(unittest.TestCase):
    def setUp(self):
        self.cfg = awsb.RunConfig(
            tag="x",
            nodes=2,
            pg_iops=6000,
            pg_mbps=500,
            load=netbench.BenchConfig(submit_nodes=1),
        )
        self.hosts = awsb.plan_hosts(self.cfg)

    def test_matches_hand_computed_totals(self):
        # Literal dollar values for this 2-node config at PRICES, hand-computed independently
        # of cost_estimate's formula, so a formula regression trips this test.
        estimate = shot_estimate(self.hosts, self.cfg)
        self.assertEqual(estimate["expected_s"], 1695.0)
        self.assertEqual(estimate["ttl_s"], 3315.0)
        self.assertAlmostEqual(estimate["expected_usd"], 0.858907, places=5)
        self.assertAlmostEqual(estimate["bound_usd"], 1.754221, places=5)

    def test_bound_is_cost_at_ttl(self):
        estimate = shot_estimate(self.hosts, self.cfg)
        ratio = (estimate["ttl_s"] + awsb.BOOT_ALLOWANCE_S) / estimate["expected_s"]
        # every line scales with duration except egress, which is flat per host
        egress_line = next(
            line for line in estimate["lines"] if line["item"] == "egress"
        )
        scaled_expected = estimate["expected_usd"] - egress_line["usd"]
        self.assertAlmostEqual(
            estimate["bound_usd"],
            scaled_expected * ratio + egress_line["usd"],
            places=4,
        )


class FormatEstimateTest(unittest.TestCase):
    def test_contains_expected_and_bound(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        estimate = shot_estimate(awsb.plan_hosts(cfg), cfg)
        text = awsb.format_estimate_summary(estimate, cfg.max_usd)
        self.assertIn(f"expected ${estimate['expected_usd']:.2f}", text)
        self.assertIn(f"hard bound ${estimate['bound_usd']:.2f}", text)
        self.assertIn("limit $10.00", text)


class ConfirmTest(unittest.TestCase):
    def test_yes_flag_always_confirms(self):
        system = FakeSystem()
        self.assertTrue(awsb.confirm(system.ask, "go?", yes=True))
        self.assertEqual(system.prompts, [])

    def test_unanswered_prompt_refuses(self):
        system = FakeSystem()
        self.assertFalse(awsb.confirm(system.ask, "go?", yes=False))
        self.assertEqual(system.prompts, ["go?"])

    def test_answer_decides(self):
        system = FakeSystem(answer=True)
        self.assertTrue(awsb.confirm(system.ask, "go?", yes=False))


def plan_args(*extra: str, nodes: str = "2") -> "awsb.argparse.Namespace":
    return parse_plan_args(["--tag", "x", "--nodes", nodes, *extra])


# REQ:awsbench-render / REQ:awsbench-budget-refusal / EDGE:awsbench-name-collision
class CmdPlanTest(unittest.TestCase):
    def setUp(self):
        self.tmp = temp_dir(self)
        self.out = isolated_env(self, self.tmp)
        self.fleet_dir = self.out / "run1"

    def test_renders_manifest_and_peers(self):
        with unittest.mock.patch.object(
            awsb, "preflight", return_value=fake_preflight()
        ):
            code = awsb.cmd_plan(plan_args(), FakeSystem(run=FleetRunner([])))
        self.assertEqual(code, awsb.EXIT_OK)
        for name in (
            "runs/01-run/genesis.toml",
            "runs/01-run/config.json",
            "hosts/ctl/user-data.sh",
            "hosts/node0/user-data.sh",
            "terraform/main.tf",
            "terraform/terraform.tfvars.json",
        ):
            self.assertTrue((self.fleet_dir / name).exists(), name)
        manifest = json.loads((self.fleet_dir / "fleet.json").read_text())
        self.assertEqual(manifest["phase"], "planned")
        self.assertEqual(manifest["git_rev"], "a" * 40)
        self.assertEqual(manifest["peers"], {"node0": ["node1"], "node1": []})
        self.assertEqual(len(manifest["hosts"]), 3)
        self.assertNotIn("expires_at", manifest)
        self.assertFalse((self.fleet_dir / "manifest.json").exists())
        self.assertFalse((self.fleet_dir / "diff.patch").exists())
        run_manifest = json.loads(
            (self.fleet_dir / "runs/01-run/manifest.json").read_text()
        )
        self.assertEqual(run_manifest["fleet"], "run1")
        self.assertEqual(run_manifest["hosts"], manifest["hosts"])
        tfvars = json.loads(
            (self.fleet_dir / "terraform" / "terraform.tfvars.json").read_text()
        )
        self.assertEqual(tfvars["expires_at"], "")
        for constant in ("account_id", "region", "profile"):
            self.assertNotIn(constant, tfvars)
        self.assertTrue((self.fleet_dir / "driver.log").exists())
        self.assertTrue((self.fleet_dir / "events.jsonl").exists())

    def test_key_is_generated_into_the_fleet_dir(self):
        args = plan_args()
        runner = FleetRunner([])
        with unittest.mock.patch.object(
            awsb, "preflight", return_value=fake_preflight()
        ):
            self.assertEqual(awsb.cmd_plan(args, FakeSystem(run=runner)), awsb.EXIT_OK)
        key = self.fleet_dir / "ssh" / "id_ed25519"
        keygen = next(c for c in runner.calls if c[0] == "ssh-keygen")
        self.assertEqual(
            keygen,
            [
                "ssh-keygen",
                "-q",
                "-t",
                "ed25519",
                "-N",
                "",
                "-C",
                "run1",
                "-f",
                str(awsb.OUT_ROOT / "run1" / "ssh" / "id_ed25519"),
            ],
        )
        self.assertEqual(key.stat().st_mode & 0o777, 0o600)
        tfvars = json.loads(
            (self.fleet_dir / "terraform" / "terraform.tfvars.json").read_text()
        )
        self.assertEqual(tfvars["ssh_public_key"], "ssh-ed25519 GENERATED run")
        manifest = json.loads((self.fleet_dir / "fleet.json").read_text())
        self.assertEqual(manifest["ssh_public_key"], "ssh-ed25519 GENERATED run")

    def test_preflight_requires_ssh_keygen(self):
        system = FakeSystem(tools={"tofu", "aws", "ssh", "scp", "rsync", "git"})
        with self.assertRaisesRegex(awsb.Refused, "ssh-keygen"):
            awsb.preflight(system, awsb.RunConfig(tag="x"), [])

    def test_budget_refusal_makes_no_tofu_apply(self):
        args = plan_args("--max-usd", "0.01", nodes="5")
        runner = FleetRunner([])
        with unittest.mock.patch.object(
            awsb, "preflight", return_value=fake_preflight()
        ):
            code = cmd_plan_exit(args, run=runner)
        self.assertEqual(code, awsb.EXIT_REFUSED)
        self.assertFalse(runner.ran("tofu"))

    def test_name_collision_refuses(self):
        with unittest.mock.patch.object(
            awsb, "preflight", return_value=fake_preflight()
        ):
            first = awsb.cmd_plan(plan_args(), FakeSystem(run=FleetRunner([])))
        self.assertEqual(first, awsb.EXIT_OK)
        second_runner = FakeRunner({})
        second = cmd_plan_exit(plan_args(), run=second_runner)
        self.assertEqual(second, awsb.EXIT_REFUSED)
        # the name collision is caught before any AWS call, including the account guard
        self.assertEqual(second_runner.calls, [])

    def test_default_name_is_valid(self):
        unittest.mock.patch.stopall()
        self.assertRegex(awsb.default_run_name("tester", NOW), awsb.NAME_RE.pattern)

    # REQ:awsbench-account-guard
    def test_online_account_mismatch_makes_exactly_one_call(self):
        runner = FakeRunner({STS_CALL: sts_response("999999999999")})
        code = cmd_plan_exit(plan_args(), run=runner)
        self.assertEqual(code, awsb.EXIT_REFUSED)
        self.assertEqual(len(runner.calls), 1)

    def test_online_runs_preflight_and_tofu_plan_without_apply(self):
        runner = FleetRunner([])
        with unittest.mock.patch.object(
            awsb, "preflight", return_value=fake_preflight()
        ) as preflight:
            code = awsb.cmd_plan(plan_args(), FakeSystem(run=runner))
        self.assertEqual(code, awsb.EXIT_OK)
        preflight.assert_called_once()
        manifest = json.loads((self.fleet_dir / "fleet.json").read_text())
        self.assertEqual(manifest["phase"], "planned")
        self.assertEqual(manifest["az"], "eu-west-1b")
        self.assertEqual(manifest["ami_id"], "ami-0abc")
        self.assertEqual(set(manifest["images"]), set(fake_images()))
        self.assertEqual((self.fleet_dir / "terraform/plan.txt").read_text(), "plan")
        self.assertTrue(runner.ran("tofu", "init"))
        self.assertTrue(runner.ran("tofu", "plan"))
        self.assertFalse(runner.ran("tofu", "apply"))

    def test_online_logs_preflight_before_cost(self):
        with unittest.mock.patch.object(
            awsb, "preflight", return_value=fake_preflight()
        ):
            awsb.cmd_plan(plan_args(), FakeSystem(run=FleetRunner([])))
        lines = (self.fleet_dir / "driver.log").read_text().splitlines()
        account = next(i for i, line in enumerate(lines) if "account 0275" in line)
        cost = next(i for i, line in enumerate(lines) if "expected $" in line)
        self.assertLess(account, cost)


def two_node_hosts() -> list["awsb.HostSpec"]:
    return awsb.plan_hosts(
        awsb.RunConfig(tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1))
    )


# REQ:awsbench-account-guard
class CallerAccountTest(unittest.TestCase):
    def test_mismatch_refuses_after_exactly_one_call(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "sts",
                    "get-caller-identity",
                ): completed(stdout=json.dumps({"Account": "999999999999"}))
            }
        )
        with self.assertRaises(awsb.Refused):
            awsb.caller_account(runner)
        self.assertEqual(len(runner.calls), 1)

    def test_match_returns_account(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "sts",
                    "get-caller-identity",
                ): completed(stdout=json.dumps({"Account": awsb.ACCOUNT}))
            }
        )
        self.assertEqual(awsb.caller_account(runner), awsb.ACCOUNT)


class CapableAzTest(unittest.TestCase):
    def _offerings(self, azs: list[str]) -> subprocess.CompletedProcess:
        return completed(
            stdout=json.dumps(
                {"InstanceTypeOfferings": [{"Location": az} for az in azs]}
            )
        )

    def test_intersects_and_picks_lowest_az(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-instance-type-offerings",
                ): self._offerings(["eu-west-1b", "eu-west-1a"])
            }
        )
        az = awsb.capable_az(runner, {"c8g.4xlarge"})
        self.assertEqual(az, "eu-west-1a")

    def test_no_capable_az_refuses(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-instance-type-offerings",
                ): self._offerings([])
            }
        )
        with self.assertRaises(awsb.Refused):
            awsb.capable_az(runner, {"c8g.4xlarge"})


class PreflightTest(unittest.TestCase):
    def test_resolves_az_ami_and_images(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        prefix = ("aws", "--profile", "timeboost-dev")
        runner = FakeRunner(
            {
                (*prefix, "sts", "get-caller-identity"): completed(
                    stdout=json.dumps({"Account": awsb.ACCOUNT})
                ),
                (*prefix, "ec2", "describe-instance-type-offerings"): completed(
                    stdout=json.dumps(
                        {"InstanceTypeOfferings": [{"Location": "eu-west-1a"}]}
                    )
                ),
                (*prefix, "ec2", "describe-images"): completed(
                    stdout=json.dumps("ami-0abc")
                ),
            }
        )
        with unittest.mock.patch.object(
            awsb, "resolve_image", lambda _http_get, ref: fake_image(ref)
        ):
            result = awsb.preflight(FakeSystem(run=runner), cfg, awsb.plan_hosts(cfg))
        self.assertEqual(result["az"], "eu-west-1a")
        self.assertEqual(result["ami_id"], "ami-0abc")
        self.assertEqual(set(result["images"]), set(awsb.image_refs(cfg)))


class RenderTfvarsTest(unittest.TestCase):
    def test_shape_and_host_count(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        run_dir = Path("/tmp/aws-bench/run1")
        tfvars = awsb.render_tfvars(
            cfg,
            run_dir,
            "run1",
            "alice",
            hosts,
            ssh_pub="ssh-ed25519 AAAA test@example.com",
            operator_cidr="203.0.113.5/32",
            expires_at="2026-01-01T00:00:00Z",
            git_rev="abc1234",
            az="eu-west-1a",
            ami_id="ami-0abc",
        )
        self.assertEqual(tfvars["name"], "run1")
        self.assertEqual(tfvars["owner"], "alice")
        self.assertEqual(tfvars["az"], "eu-west-1a")
        self.assertEqual(tfvars["ami_id"], "ami-0abc")
        self.assertEqual(tfvars["operator_cidr"], "203.0.113.5/32")
        self.assertEqual(len(tfvars["hosts"]), 3)
        self.assertEqual(
            tfvars["hosts"]["node0"]["user_data_path"],
            str(run_dir / "hosts" / "node0" / "user-data.sh"),
        )


class TerraformClassTest(unittest.TestCase):
    def test_init_plan_apply_destroy(self):
        tf_dir = Path("/tmp/aws-bench/run1/terraform")
        runner = FakeRunner(
            {
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "init",
                ): completed(),
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "plan",
                ): completed(stdout="plan ok"),
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "apply",
                ): completed(),
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "output",
                ): completed(stdout=json.dumps({"az": {"value": "eu-west-1a"}})),
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "destroy",
                ): completed(),
            }
        )
        tf = awsb.Terraform(runner, tf_dir, env={})
        tf.init()
        tf.plan()
        tf.apply()
        self.assertEqual(tf.output(), {"az": {"value": "eu-west-1a"}})
        tf.destroy()
        self.assertTrue(runner.ran("tofu", f"-chdir={tf_dir}", "apply"))

    def test_apply_failure_keeps_the_last_stderr_line(self):
        tf_dir = Path("/tmp/aws-bench/run1/terraform")
        runner = FakeRunner(
            {
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "apply",
                ): completed(
                    returncode=1, stderr="Error: creating\nVcpuLimitExceeded: x\n\n"
                )
            }
        )
        tf = awsb.Terraform(runner, tf_dir, env={})
        with self.assertRaises(awsb.TfFailed) as ctx:
            tf.apply()
        self.assertEqual(ctx.exception.stage, "apply")
        self.assertEqual(str(ctx.exception), "tofu apply failed: VcpuLimitExceeded: x")

    def test_destroy_failure_raises_tf_failed(self):
        tf_dir = Path("/tmp/aws-bench/run1/terraform")
        runner = FakeRunner(
            {
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "destroy",
                ): completed(returncode=1, stderr="boom")
            }
        )
        tf = awsb.Terraform(runner, tf_dir, env={})
        with self.assertRaises(awsb.TfFailed) as ctx:
            tf.destroy()
        self.assertEqual(ctx.exception.stage, "destroy")

    def test_env_is_passed_to_the_runner_not_exported(self):
        tf_dir = Path("/tmp/aws-bench/run1/terraform")
        env = {"TF_PLUGIN_CACHE_DIR": "/tmp/plugins"}
        runner = FakeRunner({("tofu",): completed()})
        before = dict(os.environ)
        awsb.Terraform(runner, tf_dir, env=env).init()
        self.assertEqual(dict(os.environ), before)
        self.assertEqual(runner.envs, [env])

    def test_terraform_env_creates_the_plugin_cache_dir(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp) / "a" / "plugins"
            with unittest.mock.patch.object(awsb, "TF_PLUGIN_CACHE_DIR", cache):
                env = awsb.terraform_env()
            self.assertEqual(env, {"TF_PLUGIN_CACHE_DIR": str(cache)})
            self.assertTrue(cache.is_dir())


def resolve_with(registry: FakeRegistry) -> "awsb.ImageInfo":
    return awsb.resolve_image(FakeSystem(http=registry).http_get, registry.ref)


# REQ:awsbench-image-check
class ResolveImageTest(unittest.TestCase):
    def test_resolves_digest_platforms_and_revision(self):
        server = FakeRegistry(
            repository="test/image",
            tag="v1",
            platforms=[("linux", "amd64"), ("linux", "arm64")],
            revision="abc1234",
        )
        info = resolve_with(server)
        self.assertEqual(info["digest"], server.digests[("linux", "arm64")])
        self.assertCountEqual(info["platforms"], ["linux/amd64", "linux/arm64"])
        self.assertEqual(info["revision"], "abc1234")
        self.assertEqual(info["ref"], server.ref)

    def test_no_revision_label_is_none(self):
        server = FakeRegistry(repository="x", tag="v1", platforms=[("linux", "arm64")])
        info = resolve_with(server)
        self.assertIsNone(info["revision"])

    def test_single_manifest_without_index(self):
        server = FakeRegistry(
            repository="x", tag="v1", platforms=[("linux", "arm64")], index=False
        )
        info = resolve_with(server)
        self.assertEqual(info["platforms"], ["linux/arm64"])
        self.assertEqual(info["digest"], server.single_digest)

    def test_attestation_platform_ignored(self):
        # buildx publishes an extra unknown/unknown manifest for SBOM/provenance attestations.
        server = FakeRegistry(
            repository="x",
            tag="v1",
            platforms=[("unknown", "unknown"), ("linux", "arm64")],
        )
        info = resolve_with(server)
        self.assertEqual(info["platforms"], ["linux/arm64"])

    # EDGE:awsbench-image-no-arm64
    def test_missing_arm64_refuses(self):
        server = FakeRegistry(repository="x", tag="v1", platforms=[("linux", "amd64")])
        with self.assertRaises(awsb.Refused) as ctx:
            resolve_with(server)
        self.assertIn("no linux/arm64 platform", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))

    def test_missing_tag_refuses(self):
        server = FakeRegistry(
            repository="x", tag="missing", platforms=[("linux", "arm64")]
        )
        with self.assertRaises(awsb.Refused) as ctx:
            resolve_with(server)
        self.assertIn("image not found", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))

    # EDGE:awsbench-image-private
    def test_private_image_denied_token_refuses(self):
        server = FakeRegistry(
            repository="x", tag="v1", platforms=[("linux", "arm64")], deny_token=True
        )
        with self.assertRaises(awsb.Refused) as ctx:
            resolve_with(server)
        self.assertIn("token request", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))

    def test_private_image_403_refuses(self):
        server = FakeRegistry(
            repository="x",
            tag="v1",
            platforms=[("linux", "arm64")],
            deny_manifest_status=403,
        )
        with self.assertRaises(awsb.Refused) as ctx:
            resolve_with(server)
        self.assertIn("private or inaccessible", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))


class RegistryGetTest(unittest.TestCase):
    """`_http_get` against a loopback server that relays to a `FakeRegistry`."""

    def _serve(self, registry: FakeRegistry) -> str:
        class Handler(BaseHTTPRequestHandler):
            def log_message(self, format, *args):
                pass

            def do_GET(self):
                status, headers, body = registry(
                    f"https://{registry.host}{self.path}", dict(self.headers)
                )
                self.send_response(status)
                for name, value in headers.items():
                    self.send_header(name, value)
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)

        server = HTTPServer(("127.0.0.1", 0), Handler)
        thread = threading.Thread(
            target=server.serve_forever, kwargs={"poll_interval": 0.01}, daemon=True
        )
        thread.start()
        # LIFO: shutdown, join, then server_close.
        self.addCleanup(server.server_close)
        self.addCleanup(thread.join)
        self.addCleanup(server.shutdown)
        return f"http://127.0.0.1:{server.server_address[1]}"

    def test_ok_status_headers_and_body(self):
        registry = FakeRegistry("x", "v1", [("linux", "arm64")])
        base = self._serve(registry)
        status, _, body = awsb._http_get(
            f"{base}/v2/x/blobs/{registry.config_digest}", {}
        )
        self.assertEqual(status, 200)
        self.assertEqual(json.loads(body)["architecture"], "arm64")

    def test_http_error_status_is_returned_not_raised(self):
        registry = FakeRegistry("x", "v1", [("linux", "arm64")])
        base = self._serve(registry)
        status, headers, _ = awsb._http_get(f"{base}/v2/x/manifests/v1", {})
        self.assertEqual(status, 401)
        self.assertIn("realm=", headers["WWW-Authenticate"])


class ParseRefTest(unittest.TestCase):
    def test_ghcr_multi_segment(self):
        self.assertEqual(
            awsb.parse_ref("ghcr.io/espressosystems/espresso-network/espresso-node:x"),
            ("ghcr.io", "espressosystems/espresso-network/espresso-node", "x"),
        )

    def test_docker_io_resolves_to_the_registry_api_host(self):
        # https://docker.io/v2/... 302s to www.docker.com.
        self.assertEqual(
            awsb.parse_ref("docker.io/library/postgres:16"),
            ("registry-1.docker.io", "library/postgres", "16"),
        )


TOFU = shutil.which("tofu")


@SLOW
@unittest.skipUnless(TOFU, "tofu/opentofu not on PATH")
class TofuValidateTest(unittest.TestCase):
    """TEST:awsbench-tofu-validate-ok: `tofu validate` needs no AWS credentials; `init` needs
    the network for the providers."""

    def test_validate(self):
        assert TOFU is not None  # narrows the type; the class is skipped otherwise
        with tempfile.TemporaryDirectory() as tmp:
            module = Path(tmp) / "terraform"
            shutil.copytree(Path(__file__).parent / "aws" / "terraform", module)
            for args in (["init", "-input=false", "-backend=false"], ["validate"]):
                result = subprocess.run(
                    [TOFU, f"-chdir={module}", *args],
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)


def fleet(n: int) -> dict:
    hosts = {"ctl": host_info("ctl", "ctl", 1)}
    for i in range(n):
        role = "query" if i == 0 else "validator"
        hosts[f"node{i}"] = host_info(f"node{i}", role, i + 2)
    return hosts


class RenderGenesisTest(unittest.TestCase):
    def setUp(self):
        self.template = (Path(__file__).parent / "genesis.toml").read_bytes()

    def test_capacity_at_least_ten(self):
        rendered = awsb.render_genesis(self.template, n=3).decode()
        self.assertIn("stake_table_capacity = 10", rendered)
        self.assertIn("capacity = 10", rendered)

    def test_capacity_scales_above_ten(self):
        rendered = awsb.render_genesis(self.template, n=50).decode()
        self.assertIn("stake_table_capacity = 50", rendered)
        self.assertIn("capacity = 50", rendered)

    def test_raises_if_template_shape_changes(self):
        with self.assertRaises(ValueError):
            awsb.render_genesis(b"no capacity fields here", n=5)


class ParseDotenvTest(unittest.TestCase):
    def test_parses_simple_and_quoted_values(self):
        env = awsb.parse_dotenv(
            '# comment\nFOO=bar\nQUOTED="a b c"\n\nESPRESSO_NODE_L1_STAKE_TABLE_UPDATE_INTERVAL=1m\n'
        )
        self.assertEqual(env["FOO"], "bar")
        self.assertEqual(env["QUOTED"], "a b c")
        self.assertEqual(env["ESPRESSO_NODE_L1_STAKE_TABLE_UPDATE_INTERVAL"], "1m")

    def test_expands_reference_to_earlier_key(self):
        env = awsb.parse_dotenv("FOO=bar\nBAZ=${FOO}/baz\n")
        self.assertEqual(env["BAZ"], "bar/baz")

    def test_reference_to_unknown_key_raises(self):
        with self.assertRaises(KeyError):
            awsb.parse_dotenv("BAZ=${UNKNOWN}\n")

    def test_line_without_equals_raises(self):
        with self.assertRaises(ValueError):
            awsb.parse_dotenv("FOO=bar\nnot a line\n")

    @SLOW
    def test_real_env_file_parses(self):
        env = awsb.parse_dotenv((Path(__file__).parents[2] / ".env").read_text())
        self.assertIn("ESPRESSO_ETH_MNEMONIC", env)
        self.assertIn("ESPRESSO_STAKE_TABLE_PROXY_ADDRESS", env)
        self.assertEqual(
            env["ESPRESSO_OPS_TIMELOCK_ADMIN"], env["ESPRESSO_ETH_MULTISIG_ADDRESS"]
        )


class RenderNodeEnvTest(unittest.TestCase):
    def setUp(self):
        self.cfg = awsb.RunConfig(
            tag="x", nodes=5, load=netbench.BenchConfig(submit_nodes=4)
        )
        self.hosts = fleet(5)

    def env(self, name: str) -> dict:
        host = next(h for h in awsb.plan_hosts(self.cfg) if h["name"] == name)
        text = awsb.render_node_env(host, self.hosts, awsb.pg_endpoint())
        return dict(line.split("=", 1) for line in text.splitlines())

    def test_node_env_goes_to_every_node_and_overrides(self):
        extra = ("ESPRESSO_QUERY_PAYLOAD_DIR=/payload", "RUST_LOG=debug,a=b")
        for host in awsb.plan_hosts(self.cfg):
            if host["role"] == "ctl":
                continue
            pg = awsb.pg_endpoint() if host["role"] == "query" else None
            text = awsb.render_node_env(host, self.hosts, pg, extra)
            env = dict(line.split("=", 1) for line in text.splitlines())
            self.assertEqual(env["ESPRESSO_QUERY_PAYLOAD_DIR"], "/payload")
            self.assertEqual(env["RUST_LOG"], "debug,a=b")
            self.assertEqual(text.count("RUST_LOG="), 1)

    def test_node_env_flag(self):
        argv = ["run", "--tag", "x"]
        self.assertEqual(awsb.config_from_args(awsb.parse_args(argv)).node_env, ())
        args = awsb.parse_args([*argv, "--node-env", "A=1", "--node-env", "B="])
        self.assertEqual(awsb.config_from_args(args).node_env, ("A=1", "B="))
        repeated = awsb.parse_args([*argv, "--node-env", "A=1", "--node-env", "A=2"])
        with self.assertRaisesRegex(awsb.Refused, "repeats A"):
            awsb.config_from_args(repeated)
        for bad in ("A", "=1", "1A=2", "A B=1"):
            with (
                self.subTest(bad),
                unittest.mock.patch("sys.stderr"),
                self.assertRaises(SystemExit),
            ):
                awsb.parse_args([*argv, "--node-env", bad])

    def test_key_index_offset_by_twenty(self):
        self.assertEqual(self.env("node0")["ESPRESSO_NODE_KEY_INDEX"], "20")
        self.assertEqual(self.env("node3")["ESPRESSO_NODE_KEY_INDEX"], "23")

    def test_validator_has_no_postgres_vars(self):
        env = self.env("node1")
        self.assertNotIn("ESPRESSO_NODE_POSTGRES_HOST", env)
        self.assertEqual(env["ESPRESSO_NODE_STORAGE_PATH"], "/store/espresso")

    def test_query_node_has_postgres_vars(self):
        env = self.env("node0")
        self.assertEqual(env["ESPRESSO_NODE_POSTGRES_HOST"], "127.0.0.1")
        self.assertEqual(env["ESPRESSO_NODE_POSTGRES_DATABASE"], "espresso")

    def test_state_peers_via_peers_function(self):
        env = self.env("node1")
        expected = [
            f"http://{self.hosts[f'node{j}']['private_ip']}:{awsb.NODE_API_PORT}"
            for j in awsb.peers(1, 5)
        ]
        self.assertEqual(env["ESPRESSO_NODE_STATE_PEERS"], ",".join(expected))

    def test_node0_has_state_peers(self):
        env = self.env("node0")
        expected = [
            f"http://{self.hosts[f'node{j}']['private_ip']}:{awsb.NODE_API_PORT}"
            for j in awsb.peers(0, 5)
        ]
        self.assertEqual(env["ESPRESSO_NODE_STATE_PEERS"], ",".join(expected))

    def test_state_peers_omitted_when_empty(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = fleet(2)
        host = next(h for h in awsb.plan_hosts(cfg) if h["name"] == "node1")
        env = dict(
            line.split("=", 1)
            for line in awsb.render_node_env(
                host, hosts, awsb.pg_endpoint()
            ).splitlines()
        )
        self.assertNotIn("ESPRESSO_NODE_STATE_PEERS", env)

    def test_journal_max_bytes_reserves_disk_and_halves_for_query(self):
        validator_gb = self.env("node1")["ESPRESSO_NODE_JOURNAL_MAX_BYTES"]
        query_gb = self.env("node0")["ESPRESSO_NODE_JOURNAL_MAX_BYTES"]
        usable = (awsb.NODE_ROOT_GB_MIN - awsb.RESERVED_GB) * 1_000_000_000
        self.assertEqual(
            int(validator_gb), int(usable * awsb.JOURNAL_MAX_BYTES_FRACTION)
        )
        self.assertEqual(
            int(query_gb),
            int(usable * awsb.JOURNAL_MAX_BYTES_FRACTION * awsb.QUERY_JOURNAL_FRACTION),
        )

    def test_advertise_addresses_use_private_ip_and_dns(self):
        env = self.env("node2")
        self.assertEqual(
            env["ESPRESSO_NODE_CLIQUENET_ADVERTISE_ADDRESS"],
            f"10.0.0.4:{awsb.CLIQUENET_PORT}",
        )
        self.assertEqual(
            env["ESPRESSO_NODE_LIBP2P_ADVERTISE_ADDRESS"],
            f"ip-10-0-0-4.eu-west-1.compute.internal:{awsb.LIBP2P_PORT}",
        )


DOTENV_TEXT = f"""# fixture in the syntax of the repo's .env
ESPRESSO_ETH_MNEMONIC="{awsb.BENCH_MNEMONIC}"
ESPRESSO_ORCHESTRATOR_PORT={awsb.ORCHESTRATOR_PORT}
ESPRESSO_L1_PORT={awsb.L1_PORT}
ESPRESSO_STATE_RELAY_SERVER_PORT={awsb.RELAY_PORT}
ESPRESSO_FEE_CONTRACT_PROXY_ADDRESS=0x0000000000000000000000000000000000000001
ESP_TOKEN_PROXY_ADDRESS=0x0000000000000000000000000000000000000002
ESPRESSO_STAKE_TABLE_PROXY_ADDRESS=0x0000000000000000000000000000000000000003
ESPRESSO_LIGHT_CLIENT_PROXY_ADDRESS=0x0000000000000000000000000000000000000004
ESPRESSO_ETH_MULTISIG_ADDRESS=a0Ee7A142d267C1f36714E4a8F75612F20a79720
ESPRESSO_OPS_TIMELOCK_ADMIN=${{ESPRESSO_ETH_MULTISIG_ADDRESS}}
"""


class RenderCtlEnvTest(unittest.TestCase):
    def setUp(self):
        self.cfg = awsb.RunConfig(
            tag="x", nodes=5, load=netbench.BenchConfig(submit_nodes=4)
        )
        self.hosts = fleet(5)
        self.dotenv = awsb.parse_dotenv(DOTENV_TEXT)

    def env(self) -> dict:
        text = awsb.render_ctl_env(self.hosts, self.cfg, self.dotenv)
        return dict(line.split("=", 1) for line in text.splitlines())

    def test_proxy_addresses_unset(self):
        env = self.env()
        for key in awsb.PROXY_ADDRESS_KEYS:
            self.assertNotIn(key, env)
        # Not one of the three unset proxy addresses; stays.
        self.assertIn("ESPRESSO_LIGHT_CLIENT_PROXY_ADDRESS", env)

    def test_orchestrator_num_nodes_and_builder_disabled(self):
        env = self.env()
        self.assertEqual(env["ESPRESSO_ORCHESTRATOR_NUM_NODES"], "5")
        self.assertEqual(
            env["ESPRESSO_ORCHESTRATOR_BUILDER_URLS"], "http://localhost:1"
        )
        self.assertEqual(env["ESPRESSO_ORCHESTRATOR_BUILDER_TIMEOUT"], "100ms")

    def test_relay_reads_node0(self):
        env = self.env()
        self.assertEqual(
            env["ESPRESSO_API_NODE_URL"], f"http://10.0.0.2:{awsb.NODE_API_PORT}"
        )

    def test_no_unsubstituted_dollar_reference(self):
        env = self.env()
        for value in env.values():
            self.assertNotIn("$", value)


def fake_image(ref: str) -> dict:
    return {
        "ref": ref,
        "digest": f"sha256:{'0' * 64}",
        "revision": "abc1234",
        "platforms": [],
    }


def fake_images() -> dict:
    return {
        name: fake_image(f"ghcr.io/x/{name}:t") for name in awsb.IMAGE_COMPONENTS
    } | {name: fake_image(ref) for name, ref in awsb.SUPPORT_IMAGES.items()}


FleetRunner.ready_digests = {
    name: f"{image['ref']}@{image['digest']}" for name, image in fake_images().items()
}


# REQ:awsbench-user-data
class RenderUserDataTest(unittest.TestCase):
    def setUp(self):
        self.images = fake_images()

    def test_ttl_line_is_first(self):
        host = {
            "name": "ctl",
            "role": "ctl",
            "instance_type": "x",
            "root_gb": 40,
            "root_iops": 3000,
            "root_mbps": 125,
        }
        text = awsb.render_user_data(host, self.images, ttl_s=930)
        lines = [
            line
            for line in text.splitlines()
            if line.strip() and not line.startswith(("#", "set "))
        ]
        self.assertTrue(lines[0].startswith("shutdown -P +"))
        self.assertEqual(lines[0], "shutdown -P +16")

    def test_uv_and_python_installed_after_ttl(self):
        host = {
            "name": "ctl",
            "role": "ctl",
            "instance_type": "x",
            "root_gb": 40,
            "root_iops": 3000,
            "root_mbps": 125,
        }
        text = awsb.render_user_data(host, self.images, ttl_s=60)
        ttl = text.index("shutdown -P +")
        uv = text.index(
            "curl -LsSf https://astral.sh/uv/install.sh"
            " | env UV_INSTALL_DIR=/usr/local/bin UV_NO_MODIFY_PATH=1 sh"
        )
        python = text.index("uv python install 3.14")
        self.assertLess(ttl, uv)
        self.assertLess(uv, python)
        self.assertIn("UV_PYTHON_INSTALL_DIR=/opt/uv/python", text)

    def test_every_image_pulled_by_digest(self):
        host = {
            "name": "node0",
            "role": "query",
            "instance_type": "x",
            "root_gb": 100,
            "root_iops": 6000,
            "root_mbps": 500,
        }
        text = awsb.render_user_data(host, self.images, ttl_s=60)
        for name in awsb.ROLE_IMAGES["query"]:
            image = self.images[name]
            self.assertIn(f"docker pull {image['ref']}@{image['digest']}", text)

    def test_no_unsubstituted_placeholder(self):
        host = {
            "name": "ctl",
            "role": "ctl",
            "instance_type": "x",
            "root_gb": 40,
            "root_iops": 3000,
            "root_mbps": 125,
        }
        text = awsb.render_user_data(host, self.images, ttl_s=60)
        found = set(re.findall(r"\$\{?[A-Za-z_]\w*", text))
        # $digests/$chrony: ready.json's jq filter. $name/$digest: the per-image jq merge that
        # records what was actually pulled. $pulled: the bash variable holding it. None are
        # leftover Template placeholders.
        self.assertLessEqual(
            found,
            {"$digests", "$chrony", "$uv", "$pythons", "$name", "$digest", "$pulled"},
        )

    def test_digest_recorded_from_docker_inspect_not_requested_digest(self):
        host = {
            "name": "node0",
            "role": "query",
            "instance_type": "x",
            "root_gb": 100,
            "root_iops": 6000,
            "root_mbps": 500,
        }
        text = awsb.render_user_data(host, self.images, ttl_s=60)
        self.assertIn("docker image inspect", text)
        self.assertIn("RepoDigests", text)
        self.assertIn("/opt/bench/digests.json", text)

    def test_query_role_mounts_pg_volume(self):
        query = {
            "name": "node0",
            "role": "query",
            "instance_type": "x",
            "root_gb": 100,
            "root_iops": 6000,
            "root_mbps": 500,
        }
        validator = {
            "name": "node1",
            "role": "validator",
            "instance_type": "x",
            "root_gb": 100,
            "root_iops": 3000,
            "root_mbps": 125,
        }
        self.assertIn(
            "mkdir -p /data/pg", awsb.render_user_data(query, self.images, ttl_s=60)
        )
        self.assertNotIn(
            "mkdir -p /data/pg", awsb.render_user_data(validator, self.images, ttl_s=60)
        )


class RenderStartShTest(unittest.TestCase):
    def setUp(self):
        self.images = fake_images()

    def test_ctl_creates_support_containers(self):
        ctl = {
            "name": "ctl",
            "role": "ctl",
            "instance_type": "x",
            "root_gb": 40,
            "root_iops": 3000,
            "root_mbps": 125,
        }
        script = awsb.render_start_sh(ctl, self.images)
        for name in ("anvil", "deploy", "orchestrator", "state-relay-server"):
            self.assertIn(f"--name {name}", script)
        self.assertNotIn("--num-nodes", script)
        self.assertNotIn("docker start", script)

    def test_anvil_uses_entrypoint_and_binds_every_interface(self):
        ctl = {
            "name": "ctl",
            "role": "ctl",
            "instance_type": "x",
            "root_gb": 40,
            "root_iops": 3000,
            "root_mbps": 125,
        }
        script = awsb.render_start_sh(ctl, self.images)
        self.assertIn("--entrypoint anvil", script)
        self.assertIn("--host 0.0.0.0", script)

    def test_deploy_has_deploy_flags(self):
        ctl = {
            "name": "ctl",
            "role": "ctl",
            "instance_type": "x",
            "root_gb": 40,
            "root_iops": 3000,
            "root_mbps": 125,
        }
        script = awsb.render_start_sh(ctl, self.images)
        for flag in awsb.DEPLOY_FLAGS:
            self.assertIn(flag, script)

    def test_validator_uses_storage_journal_only(self):
        validator = {
            "name": "node1",
            "role": "validator",
            "instance_type": "x",
            "root_gb": 100,
            "root_iops": 3000,
            "root_mbps": 125,
        }
        script = awsb.render_start_sh(validator, self.images)
        self.assertIn("-- storage-journal -- http", script)
        self.assertNotIn("storage-sql", script)
        self.assertNotIn("--name postgres", script)
        self.assertIn("/opt/bench/genesis.toml:/opt/bench/genesis.toml:ro", script)

    def test_query_node_adds_storage_sql_and_postgres(self):
        query = {
            "name": "node0",
            "role": "query",
            "instance_type": "x",
            "root_gb": 100,
            "root_iops": 6000,
            "root_mbps": 500,
        }
        script = awsb.render_start_sh(query, self.images)
        self.assertIn("-- storage-journal -- storage-sql", script)
        self.assertIn("--name postgres", script)
        for setting in (
            "shared_preload_libraries=pg_stat_statements",
            "pg_stat_statements.track=all",
            "log_min_duration_statement=200",
            "log_checkpoints=on",
            "log_lock_waits=on",
        ):
            self.assertIn(setting, script)
        image_at = script.index("@sha256")
        self.assertGreater(script.index("shared_preload_libraries"), image_at)


# REQ:awsbench-hostmon-parsers
class DiskstatsTest(unittest.TestCase):
    def test_parses_known_columns(self):
        line = "   8       0 nvme0n1 100 0 2000 5 200 0 4000 10 0 20 20\n"
        result = awsb.diskstats(line)
        self.assertEqual(
            result["nvme0n1"],
            {
                "read_bytes": 2000 * 512,
                "write_bytes": 4000 * 512,
                "io_ticks_ms": 20,
                "reads": 100,
                "writes": 200,
                "read_ms": 5,
                "write_ms": 10,
                "in_flight": 0,
                "weighted_ms": 20,
            },
        )

    def test_raises_on_short_line(self):
        with self.assertRaises(ValueError):
            awsb.diskstats("garbage\n")


class NetdevTest(unittest.TestCase):
    FIXTURE = (
        "Inter-|   Receive                                                |  Transmit\n"
        " face |bytes    packets errs drop fifo frame compressed multicast|"
        "bytes    packets errs drop fifo colls carrier compressed\n"
        "    lo:  100    1    0    0    0     0          0         0  "
        "  100    1    0    0    0     0       0          0\n"
        "  eth0: 5000    5    0    0    0     0          0         0 "
        " 7000    7    0    0    0     0       0          0\n"
    )

    def test_parses_rx_and_tx_bytes(self):
        result = awsb.netdev(self.FIXTURE)
        self.assertEqual(result["eth0"], {"rx_bytes": 5000, "tx_bytes": 7000})
        self.assertEqual(result["lo"], {"rx_bytes": 100, "tx_bytes": 100})

    def test_skips_header_lines(self):
        result = awsb.netdev(self.FIXTURE)
        self.assertNotIn("Inter-", result)
        self.assertNotIn("face ", result)

    def test_raises_on_short_device_line(self):
        with self.assertRaises(ValueError):
            awsb.netdev("eth0: 100 1\n")


class CgroupParsersTest(unittest.TestCase):
    def test_parse_cpu_stat(self):
        text = "usage_usec 123456\nuser_usec 100000\nsystem_usec 23456\n"
        self.assertEqual(awsb.parse_cpu_stat(text), 123456)

    def test_parse_cpu_stat_missing_raises(self):
        with self.assertRaises(ValueError):
            awsb.parse_cpu_stat("user_usec 1\n")

    def test_parse_memory_stat_anon(self):
        text = "anon 104857600\nfile 500000000\nkernel_stack 8192\n"
        self.assertEqual(awsb.parse_memory_stat_anon(text), 104857600)

    def test_parse_memory_stat_anon_missing_raises(self):
        with self.assertRaises(ValueError):
            awsb.parse_memory_stat_anon("file 500000000\n")

    def test_container_stats_reads_scope_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            scope = root / "system.slice" / "docker-abc123.scope"
            scope.mkdir(parents=True)
            (scope / "cpu.stat").write_text("usage_usec 2000000\n")
            # file (page cache) dwarfs anon here; container_stats must report anon, not the sum.
            (scope / "memory.stat").write_text("anon 104857600\nfile 900000000\n")
            stats = awsb.container_stats({"espresso-node": "abc123"}, root)
            self.assertEqual(stats, {"espresso-node": {"cpu_s": 2.0, "rss": 104857600}})

    def test_container_stats_skips_missing_scope(self):
        with tempfile.TemporaryDirectory() as tmp:
            stats = awsb.container_stats({"gone": "deadbeef"}, Path(tmp))
            self.assertEqual(stats, {})


def fake_preflight() -> dict:
    return {
        "account": "027574771971",
        "az": "eu-west-1b",
        "ami_id": "ami-0abc",
        "images": fake_images(),
        "git_diff": None,
    }


DONE_STATE = {
    "phase": "done",
    "detail": "load finished",
    "ready_s": 30.0,
    "t0": FAKE_EPOCH + 100.0,
    "t1": FAKE_EPOCH + 200.0,
}
DESCRIBE = json.dumps(
    [
        {
            "launch": "2026-09-29T15:00:00+00:00",
            "reason": "User initiated (2026-09-29 15:30:00 GMT)",
        }
    ]
)


def valid_result(valid: bool = True) -> dict:
    limit = {"mb_s": 8.0, "bounded": False}
    return {
        "validity": {"valid": valid, "noisy": False, "reasons": []},
        "run": {"sha": "a" * 40, "pr": None, "event": "aws", "ref": "x"},
        "config_hash": "h",
        "capacity": {
            "overall": limit,
            "consensus": limit,
            "query_node": limit,
            "failed_at_mb_s": None,
            "fail_rule": None,
        },
        "steps": [
            {"passed": True, "decided_mb_s": 8.0, "query_lag_ms": {"p99": 120.0}}
        ],
    }


class RunHarness:
    """A temp working dir with its own out root and patched waits for driving `cmd_run` end to
    end."""

    def __init__(
        self, test: unittest.TestCase, name: str = "run1", confirmed: bool = False
    ):
        self.yes = not confirmed
        self.answer = confirmed
        self.tmp = temp_dir(test)
        self.out = isolated_env(test, self.tmp, name)
        self.name = name
        self.fleet_dir = awsb.OUT_ROOT / name
        self.run_dir = self.fleet_dir / "runs" / "01-run"
        patches = [
            unittest.mock.patch.object(
                awsb, "preflight", return_value=fake_preflight()
            ),
            unittest.mock.patch.object(awsb, "DOTENV", awsb.parse_dotenv(DOTENV_TEXT)),
        ]
        for patch in patches:
            patch.start()
            test.addCleanup(patch.stop)

    def args(self, *extra: str) -> "awsb.argparse.Namespace":
        argv = [
            "run",
            "--tag",
            "t",
            "--nodes",
            "2",
            *(["--yes"] if self.yes else []),
            *extra,
        ]
        args = awsb.parse_args(argv)
        args.argv = argv
        return args

    def run(self, runner: FleetRunner, extra=()) -> int:
        return awsb.cmd_run(
            self.args(*extra), FakeSystem(run=runner, answer=self.answer)
        )

    def index_log(self) -> str:
        return (self.fleet_dir / "driver.log").read_text()

    def last_log_line(self) -> str:
        return (self.fleet_dir / "driver.log").read_text().splitlines()[-1]

    def index(self) -> str:
        return (self.out / "INDEX.md").read_text()


# REQ:awsbench-apply-failure
class RunApplyFailureTest(unittest.TestCase):
    def test_logs_the_error_destroys_and_exits_3(self):
        harness = RunHarness(self)
        runner = FleetRunner(
            [DONE_STATE],
            apply=completed(returncode=1, stderr="Error: InsufficientInstanceCapacity"),
        )
        code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "apply"))
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertFalse(any(call[0] == "ssh" for call in runner.calls))
        log = (harness.fleet_dir / "driver.log").read_text()
        self.assertIn("InsufficientInstanceCapacity", log)
        self.assertIn("| 3 |", harness.index())

    def test_declined_prompt_refuses_before_apply(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE])
        args = harness.args()
        args.yes = False
        with self.assertRaises(awsb.Refused):
            awsb.cmd_run(args, FakeSystem(run=runner))
        self.assertFalse(runner.ran("tofu", "apply"))
        self.assertFalse(any(call[0] == "ssh" for call in runner.calls))


class RunFlowTest(unittest.TestCase):
    def test_valid_run_exits_0_with_cost_and_index(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result()
        ):
            code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_OK)
        order = (
            "docker start anvil",
            "docker wait deploy",
            "docker start orchestrator",
            "docker start state-relay-server",
            "docker start postgres",
            "docker start espresso-node",
            "agent-drive",
        )
        joined = [" ".join(call) for call in runner.calls if call[0] == "ssh"]
        positions = [
            next(i for i, line in enumerate(joined) if needle in line)
            for needle in order
        ]
        self.assertEqual(positions, sorted(positions))
        cost = json.loads((harness.fleet_dir / "cost.json").read_text())
        self.assertAlmostEqual(cost["duration_s"], 1800.0)
        self.assertGreater(cost["actual"], 0)
        self.assertLess(cost["actual"], cost["bound"])
        fleet = json.loads((harness.fleet_dir / "fleet.json").read_text())
        self.assertEqual(fleet["phase"], "done")
        manifest = json.loads((harness.run_dir / "manifest.json").read_text())
        self.assertEqual(manifest["phase"], "done")
        self.assertEqual(manifest["fleet"], "run1")
        self.assertEqual(manifest["cost_usd"]["actual"], cost["actual"])
        self.assertEqual(manifest["start_spread_s"], 0.0)
        *_, run_row = harness.index().splitlines()
        self.assertTrue(run_row.startswith("| run1/01-run |"))
        self.assertIn("| colocated |", run_row)
        self.assertIn("| valid | 0 |", run_row)
        run_cost = json.loads((harness.run_dir / "cost.json").read_text())
        self.assertGreater(run_cost["usd"], 0)

    def test_fleet_files_and_run_files_live_in_their_own_dirs(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result()
        ):
            harness.run(runner)
        for name in (
            "fleet.json",
            "hosts.json",
            "cost.json",
            "terraform/plan.txt",
            "hosts/ctl/user-data.sh",
            "hosts/ctl/ready.json",
        ):
            self.assertTrue((harness.fleet_dir / name).exists(), name)
        for name in (
            "manifest.json",
            "genesis.toml",
            "topology.json",
            "hosts/ctl/agent.json",
            "hosts/node0/start.sh",
            "hosts/node0/node.env",
        ):
            self.assertTrue((harness.run_dir / name).exists(), name)
        for name in ("manifest.json", "topology.json", "genesis.toml"):
            self.assertFalse((harness.fleet_dir / name).exists(), name)
        self.assertEqual([p.name for p in harness.fleet_dir.glob("runs/*")], ["01-run"])

    # TEST:system-expiry-exact-ok
    def test_expiry_is_stamped_after_confirm_and_matches_the_manifest(self):
        harness = RunHarness(self, confirmed=True)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result()
        ):
            harness.run(runner, extra=("--yes",))
        manifest = json.loads((harness.fleet_dir / "fleet.json").read_text())
        tfvars = json.loads(
            (harness.fleet_dir / "terraform" / "terraform.tfvars.json").read_text()
        )
        self.assertEqual(tfvars["expires_at"], manifest["expires_at"])
        expires = datetime.fromisoformat(tfvars["expires_at"])
        exact = FAKE_EPOCH + manifest["estimate"]["ttl_s"] + awsb.PROVISION_S
        self.assertEqual(expires.timestamp(), exact)
        plans = [c for c in runner.calls if c[0] == "tofu" and c[2] == "plan"]
        self.assertEqual(len(plans), 2)

    def test_invalid_result_exits_1(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result(valid=False)
        ):
            code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_INVALID)
        self.assertTrue(runner.ran("tofu", "destroy"))

    def test_key_is_removed_after_destroy(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result()
        ):
            code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_OK)
        ssh_dir = harness.fleet_dir / "ssh"
        self.assertFalse((ssh_dir / "id_ed25519").exists())
        self.assertTrue((ssh_dir / "id_ed25519.pub").exists())
        self.assertIn("ssh key removed", harness.index_log())
        self.assertTrue(runner.ran("-i", str(ssh_dir / "id_ed25519")))

    def test_key_stays_after_failed_destroy(self):
        harness = RunHarness(self)
        runner = FleetRunner(
            [DONE_STATE], destroys=[completed(returncode=1, stderr="locked")]
        )
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result()
        ):
            code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_LEFTOVER)
        self.assertTrue((harness.fleet_dir / "ssh" / "id_ed25519").exists())

    def test_agent_error_collects_reports_failure_and_exits_3(self):
        harness = RunHarness(self)
        error = {"phase": "error", "detail": "x", "error": "network not ready: heights"}
        runner = FleetRunner([error], describe=DESCRIBE)
        code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_FAILED)
        self.assertIn("network not ready", (harness.run_dir / "summary.md").read_text())
        self.assertTrue(runner.ran("systemctl stop bench-agent"))
        self.assertTrue(runner.ran("docker stop"))
        self.assertTrue(runner.ran("rsync", "/opt/bench/out/"))
        self.assertTrue(runner.ran("tofu", "destroy"))


# REQ:awsbench-interrupt
class RunInterruptTest(unittest.TestCase):
    def run_interrupted(self) -> tuple[RunHarness, FleetRunner, int]:
        harness = RunHarness(self)
        running = {"phase": "loading", "detail": "x"}
        runner = FleetRunner(
            [running], on_poll=lambda n: n == 2 and system.fire(signal.SIGINT)
        )
        system = FakeSystem(run=runner)
        code = awsb.cmd_run(harness.args(), system)
        return harness, runner, code

    def test_stops_agent_collects_destroys_and_exits_3(self):
        harness, runner, code = self.run_interrupted()
        self.assertEqual(code, awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("systemctl stop bench-agent"))
        self.assertTrue(runner.ran("rsync", "/opt/bench/out/"))
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertIn("interrupted", (harness.run_dir / "summary.md").read_text())

    def test_first_signal_logs_the_phase_and_later_ones_remind(self):
        interrupts = awsb.Interrupts(FakeClock())
        interrupts.phase = "services"
        with self.assertLogs("aws-bench", "WARNING") as logs:
            interrupts._handle(2, None)
            interrupts._handle(2, None)
        self.assertTrue(interrupts.event.is_set())
        self.assertIn(
            "interrupt received; stopping after the current step (services)",
            logs.output[0],
        )
        self.assertIn("still waiting for the current step (services)", logs.output[1])

    def test_sighup_is_handled_like_sigint(self):
        interrupts = awsb.Interrupts(FakeClock())
        system = FakeSystem()
        interrupts.install(system.trap)
        self.assertEqual(
            set(system.handlers), {signal.SIGINT, signal.SIGTERM, signal.SIGHUP}
        )

    def test_third_signal_skips_the_collection_even_after_disarm(self):
        interrupts = awsb.Interrupts(FakeClock())
        interrupts.disarm()
        interrupts._handle(1, None)
        interrupts._handle(1, None)
        self.assertFalse(interrupts.skip_collect.is_set())
        interrupts._handle(1, None)
        self.assertTrue(interrupts.skip_collect.is_set())

    def test_skipped_collection_still_destroys(self):
        harness = RunHarness(self)

        def third_signal(polls: int) -> None:
            if polls == 2:
                for _ in range(awsb.SKIP_COLLECT_SIGNALS):
                    system.fire(signal.SIGINT)

        runner = FleetRunner(
            [{"phase": "loading", "detail": "x"}], on_poll=third_signal
        )
        system = FakeSystem(run=runner)
        code = awsb.cmd_run(harness.args(), system)
        self.assertEqual(code, awsb.EXIT_FAILED)
        self.assertFalse(runner.ran("rsync", "/opt/bench/out/"))
        self.assertTrue(runner.ran("tofu", "destroy"))

    # TEST:interrupt-during-fake-sleep-fails
    def test_a_signal_during_the_poll_sleep_raises_after_the_wait(self):
        clock = FakeClock()
        interrupts = awsb.Interrupts(clock)
        clock.on_advance = lambda now: interrupts._handle(signal.SIGINT, None)
        with self.assertRaises(awsb.Interrupted):
            interrupts.sleep(awsb.AGENT_POLL_S)
        self.assertEqual(clock.sleeps, [awsb.AGENT_POLL_S])

    def test_interrupts_are_ignored_after_disarm(self):
        interrupts = awsb.Interrupts(FakeClock())
        interrupts.disarm()
        interrupts._handle(2, None)
        interrupts.check()
        interrupts.sleep(0)


# REQ:awsbench-destroy-fallback
class RunDestroyFallbackTest(unittest.TestCase):
    def test_three_failed_destroys_sweep_and_exit_4(self):
        harness = RunHarness(self)
        error = {"phase": "error", "detail": "x", "error": "boom"}
        runner = FleetRunner(
            [error], destroys=[completed(returncode=1, stderr="locked")]
        )
        code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_LEFTOVER)
        self.assertEqual(runner.count("tofu", "destroy"), awsb.DESTROY_RETRIES)
        self.assertTrue(runner.ran("terminate-instances", "i-1"))
        self.assertTrue(runner.ran("delete-security-group", "sg-1"))
        self.assertTrue(runner.ran("delete-key-pair", "key-1"))
        self.assertTrue(
            harness.last_log_line().endswith(f"aws-bench down {harness.fleet_dir}")
        )
        self.assertIn("| 4 |", harness.index())

    def test_destroy_recovers_on_retry(self):
        harness = RunHarness(self)
        error = {"phase": "error", "detail": "x", "error": "boom"}
        runner = FleetRunner(
            [error],
            destroys=[completed(returncode=1, stderr="locked"), completed()],
            describe=DESCRIBE,
        )
        code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_FAILED)
        self.assertEqual(runner.count("tofu", "destroy"), 2)
        self.assertFalse(runner.ran("terminate-instances"))


# REQ:awsbench-teardown-guaranteed
class RunTeardownGuaranteedTest(unittest.TestCase):
    def test_exception_in_finish_still_destroys(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with unittest.mock.patch.object(
            awsb, "finish_run", side_effect=IndexError("x")
        ):
            code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertIn("ERROR finish failed", harness.index_log())
        self.assertIn("IndexError: x", harness.index_log())

    def test_exception_in_analysis_writes_failure_summary_and_destroys(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with unittest.mock.patch.object(
            awsb, "write_report", side_effect=ZeroDivisionError("x")
        ):
            code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertIn("ZeroDivisionError", (harness.run_dir / "summary.md").read_text())

    def test_exception_in_cost_after_destroy_follows_the_result(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with (
            unittest.mock.patch.object(
                awsb, "write_report", return_value=valid_result()
            ),
            unittest.mock.patch.object(
                awsb, "actual_cost", side_effect=TypeError("reason")
            ),
        ):
            code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertIn("TypeError: reason", harness.index_log())

    def test_exception_in_bookkeeping_after_failed_destroy_exits_4(self):
        harness = RunHarness(self)
        runner = FleetRunner(
            [DONE_STATE], destroys=[completed(returncode=1, stderr="locked")]
        )
        with (
            unittest.mock.patch.object(
                awsb, "write_report", return_value=valid_result()
            ),
            unittest.mock.patch.object(
                awsb, "append_index", side_effect=KeyError("git_rev")
            ),
        ):
            code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_LEFTOVER)
        self.assertTrue(
            harness.last_log_line().endswith(f"aws-bench down {harness.fleet_dir}")
        )

    def test_exception_in_destroy_exits_4_with_command_last(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with (
            unittest.mock.patch.object(
                awsb, "write_report", return_value=valid_result()
            ),
            unittest.mock.patch.object(
                awsb, "destroy_fleet", side_effect=RuntimeError("boom")
            ),
        ):
            code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_LEFTOVER)
        self.assertTrue(
            harness.last_log_line().endswith(f"aws-bench down {harness.fleet_dir}")
        )

    def test_truncated_node_log_does_not_break_the_failure_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp)
            host_dir = run_dir / "hosts" / "node0"
            host_dir.mkdir(parents=True)
            good = gzip.compress(b"line\n" * 1000)
            (host_dir / "espresso-node.log.gz").write_bytes(good[: len(good) // 2])
            awsb.write_failure_summary(run_dir, {"hosts": [{"name": "node0"}]}, "boom")
            self.assertIn("log unreadable", (run_dir / "summary.md").read_text())

    def test_interrupt_before_the_fleet_exists_is_refused(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE])
        with (
            unittest.mock.patch.object(
                awsb, "preflight", side_effect=KeyboardInterrupt
            ),
            self.assertRaisesRegex(awsb.Refused, "interrupted"),
        ):
            harness.run(runner)
        self.assertFalse(runner.ran("tofu", "apply"))


# REQ:awsbench-manifest-repro
class RunDirsTest(unittest.TestCase):
    def test_run_dirs_are_numbered_in_the_fleet_dir(self):
        with tempfile.TemporaryDirectory() as tmp:
            fleet_dir = Path(tmp) / "fleet-a"
            fleet_dir.mkdir()
            first = awsb.new_run_dir(fleet_dir, "run")
            second = awsb.new_run_dir(fleet_dir, "other")
            self.assertEqual(first, fleet_dir / "runs" / "01-run")
            self.assertEqual(second, fleet_dir / "runs" / "02-other")
            self.assertEqual(awsb.fleet_of(second), fleet_dir)

    def test_fleet_name_collision_refuses(self):
        isolated_env(self, temp_dir(self), "fleet-a")
        awsb.new_fleet_dir(FakeSystem(), awsb.OUT_ROOT)
        with self.assertRaisesRegex(awsb.Refused, "already exists"):
            awsb.new_fleet_dir(FakeSystem(), awsb.OUT_ROOT)


class ManifestReproTest(unittest.TestCase):
    def render(self) -> tuple[Path, "awsb.RunConfig"]:
        tmp = temp_dir(self)
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        fleet_dir = tmp / "run1"
        fleet_dir.mkdir()
        run_dir = awsb.new_run_dir(fleet_dir, "run")
        estimate = shot_estimate(hosts, cfg)
        fleet = awsb.write_fleet_manifest(
            fleet_dir, cfg, hosts, awsb.plan_peers(hosts), estimate, "planned", ["run"]
        )
        manifest = awsb.build_run_manifest(fleet, cfg, 1, "hash", None)
        netbench.write_json(run_dir / "manifest.json", manifest)
        return run_dir, cfg

    def test_manifest_round_trips_and_update_keeps_keys(self):
        run_dir, _ = self.render()
        path = run_dir / "manifest.json"
        before = json.loads(path.read_text())
        updated = awsb.update_manifest(path, "measuring", start_spread_s=0.4)
        after = json.loads(path.read_text())
        self.assertEqual(updated, after)
        self.assertEqual(after["phase"], "measuring")
        self.assertEqual(after["start_spread_s"], 0.4)
        self.assertEqual(after["estimate"], before["estimate"])
        self.assertEqual(after["hosts"], before["hosts"])
        events = (run_dir / "events.jsonl").read_text().splitlines()
        self.assertEqual(json.loads(events[-1])["phase"], "measuring")

    def test_config_hash_changes_on_digest_type_and_genesis(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        images = fake_images()
        genesis = b"genesis"
        base = awsb.run_config_hash(cfg, hosts, images, genesis)
        self.assertEqual(base, awsb.run_config_hash(cfg, hosts, images, genesis))
        other_digest = {
            **images,
            "espresso-node": fake_image("r") | {"digest": "sha256:1"},
        }
        self.assertNotEqual(
            base, awsb.run_config_hash(cfg, hosts, other_digest, genesis)
        )
        other_type = [dict(h, instance_type="c8g.8xlarge") for h in hosts]
        self.assertNotEqual(
            base, awsb.run_config_hash(cfg, other_type, images, genesis)
        )
        self.assertNotEqual(base, awsb.run_config_hash(cfg, hosts, images, b"other"))
        other_load = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1, step_s=31)
        )
        self.assertNotEqual(
            base, awsb.run_config_hash(other_load, hosts, images, genesis)
        )
        node_env = dataclasses.replace(cfg, node_env=("A=1",))
        self.assertNotEqual(
            base, awsb.run_config_hash(node_env, hosts, images, genesis)
        )
        other_db = dataclasses.replace(cfg, query_db="rds")
        self.assertNotEqual(
            base, awsb.run_config_hash(other_db, hosts, images, genesis)
        )


class RemoteTest(unittest.TestCase):
    def remote(self, runner) -> "awsb.Remote":
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        return awsb.Remote(runner, tmp, Path("~/.ssh/id"), two_node_hosts_info())

    def test_ssh_argv_uses_run_dir_sockets_and_known_hosts(self):
        runner = FakeRunner({("ssh",): completed(stdout="ok")})
        remote = self.remote(runner)
        remote.ssh("node0", "uptime")
        argv = runner.calls[0]
        self.assertEqual(argv[-2:], ["ubuntu@203.0.113.2", "uptime"])
        joined = " ".join(argv)
        self.assertIn("ControlMaster=auto", joined)
        self.assertRegex(joined, r"ControlPath=\S+/ssh/%C")
        self.assertRegex(joined, r"UserKnownHostsFile=\S+/known_hosts")

    def test_ssh_failure_raises_with_context(self):
        remote = self.remote(
            FakeRunner({("ssh",): completed(returncode=255, stderr="refused")})
        )
        with self.assertRaisesRegex(awsb.RemoteError, "node1.*refused"):
            remote.ssh("node1", "true")

    def test_rsync_runs_remote_side_under_sudo(self):
        runner = FakeRunner({("rsync",): completed()})
        remote = self.remote(runner)
        remote.rsync_to("ctl", [Path("a"), Path("b")], "/opt/bench/")
        argv = runner.calls[0]
        self.assertEqual(argv[argv.index("--rsync-path") + 1], "sudo rsync")
        self.assertEqual(argv[-3:], ["a", "b", "ubuntu@203.0.113.1:/opt/bench/"])

    def test_parallel_returns_results_and_raises_first_failure(self):
        remote = self.remote(FakeRunner())
        self.assertEqual(
            remote.parallel(lambda n: n.upper(), ["a", "b"]), {"a": "A", "b": "B"}
        )

        def fail(name: str) -> str:
            raise awsb.RemoteError(name)

        with self.assertRaisesRegex(awsb.RemoteError, "a"):
            remote.parallel(fail, ["a", "b"])

    # EDGE:awsbench-ssh-not-ready
    def test_wait_ssh_retries_until_reachable(self):
        results = [completed(returncode=255, stderr="refused")] * 2 + [completed()]
        calls = []

        def runner(argv):
            calls.append(argv)
            return results[len(calls) - 1]

        remote = self.remote(runner)
        clock = FakeClock()
        awsb.wait_ssh(remote, "ctl", awsb.Interrupts(clock))
        self.assertEqual(len(calls), 3)
        self.assertEqual(clock.sleeps, [awsb.SSH_RETRY_S, awsb.SSH_RETRY_S * 1.5])

    def test_wait_ssh_gives_up_after_the_timeout(self):
        remote = self.remote(
            FakeRunner({("ssh",): completed(returncode=255, stderr="refused")})
        )
        clock = FakeClock()
        with self.assertRaisesRegex(awsb.RemoteError, "not reachable"):
            awsb.wait_ssh(remote, "ctl", awsb.Interrupts(clock))
        self.assertGreaterEqual(clock.time(), awsb.SSH_READY_TIMEOUT_S)

    def test_gate_times_out_naming_the_gate(self):
        remote = self.remote(
            FakeRunner({("ssh",): completed(returncode=1, stderr="no")})
        )
        clock = FakeClock()
        with self.assertRaisesRegex(awsb.RemoteError, "gate `anvil`"):
            awsb.gate(remote, "ctl", "anvil", "curl x", awsb.Interrupts(clock), 0.05)
        self.assertEqual(clock.sleeps, [awsb.GATE_RETRY_S])

    # TEST:gate-timeout-fails
    def test_gate_retries_until_its_timeout_on_the_clock(self):
        remote = self.remote(
            FakeRunner({("ssh",): completed(returncode=1, stderr="no")})
        )
        clock = FakeClock(limit_s=10 * awsb.GATE_TIMEOUT_S)
        with self.assertRaisesRegex(awsb.RemoteError, "gate `anvil`"):
            awsb.gate(remote, "ctl", "anvil", "curl x", awsb.Interrupts(clock))
        self.assertGreaterEqual(clock.time(), awsb.GATE_TIMEOUT_S)

    # TEST:fakeclock-limit-fails
    def test_a_gate_that_never_passes_hits_the_clock_limit(self):
        remote = self.remote(
            FakeRunner({("ssh",): completed(returncode=1, stderr="no")})
        )
        clock = FakeClock(limit_s=awsb.GATE_TIMEOUT_S / 2)
        with self.assertRaisesRegex(RuntimeError, "FakeClock: advanced past"):
            awsb.gate(remote, "ctl", "anvil", "curl x", awsb.Interrupts(clock))

    def test_start_nodes_spread_from_started_at(self):
        stamps = {
            "203.0.113.2": "2026-09-29T15:00:00.100000000Z",
            "203.0.113.3": "2026-09-29T15:00:00.350000000Z",
        }

        def runner(argv):
            if "inspect" in argv[-1]:
                return completed(stdout=stamps[argv[-2].split("@")[1]] + "\n")
            return completed()

        remote = self.remote(runner)
        hosts = [two_node_hosts_info()["node0"], two_node_hosts_info()["node1"]]
        spread = awsb.start_nodes(remote, hosts, 1234.5)
        self.assertAlmostEqual(spread, 0.25, places=3)

    def test_parse_docker_time(self):
        self.assertAlmostEqual(
            awsb.parse_docker_time("2026-09-29T15:00:00.5Z\n")
            - awsb.parse_docker_time("2026-09-29T15:00:00Z"),
            0.5,
        )


class Scripted:
    """Runner answering ssh by substring of the remote command, and rsync with `rsync_rc`.
    `answers` maps a substring to a list of results; the last result repeats."""

    def __init__(self, answers, rsync_rc: int = 0):
        self.answers = {k: list(v) for k, v in answers.items()}
        self.rsync_rc = rsync_rc
        self.calls: list[list[str]] = []
        self.lock = threading.Lock()

    def __call__(self, argv, env=None):
        with self.lock:
            self.calls.append(argv)
            if argv[0] == "rsync":
                return completed(returncode=self.rsync_rc, stderr="rsync broke")
            for needle, results in self.answers.items():
                if needle in argv[-1]:
                    return results.pop(0) if len(results) > 1 else results[0]
        return completed()


def tmp_dir(test: unittest.TestCase) -> Path:
    tmp = Path(tempfile.mkdtemp())
    test.addCleanup(shutil.rmtree, tmp)
    return tmp


def scripted_remote(
    test: unittest.TestCase, runner, tmp: Path | None = None
) -> "awsb.Remote":
    tmp = tmp or tmp_dir(test)
    return awsb.Remote(runner, tmp, Path("~/.ssh/id"), two_node_hosts_info())


# REQ:awsbench-collect-bounded
class CollectBoundTest(unittest.TestCase):
    def test_stop_freeze_and_collect_run_under_timeout(self):
        runner = Scripted({})
        tmp = tmp_dir(self)
        remote = scripted_remote(self, runner, tmp)
        awsb.stop_agent(remote)
        awsb.freeze(remote)
        awsb.collect_hosts(remote, list(remote.hosts.values()), tmp)
        commands = [c[-1] for c in runner.calls if c[0] == "ssh"]
        self.assertEqual(len(commands), 1 + 3 + 3)
        prefix = f"sudo timeout -k 10 {awsb.COLLECT_HOST_TIMEOUT_S:.0f} bash -c "
        self.assertTrue(all(c.startswith(prefix) for c in commands), commands)
        rsyncs = [c for c in runner.calls if c[0] == "rsync"]
        self.assertEqual(len(rsyncs), 3)
        self.assertTrue(all("--timeout=60" in c for c in rsyncs))

    def test_a_hung_host_fails_only_itself(self):
        runner = Scripted({"docker logs": [completed(returncode=124)]})
        tmp = tmp_dir(self)
        remote = scripted_remote(self, runner, tmp)
        with self.assertLogs("aws-bench", "WARNING"):
            awsb.collect_hosts(remote, list(remote.hosts.values()), tmp)

    def test_a_timed_out_collect_script_still_copies_back_what_is_on_the_host(self):
        runner = Scripted({"docker logs": [completed(returncode=124)]})
        tmp = tmp_dir(self)
        remote = scripted_remote(self, runner, tmp)
        with self.assertLogs("aws-bench", "WARNING") as logs:
            awsb.collect_hosts(remote, [remote.hosts["node0"]], tmp)
        rsyncs = [c for c in runner.calls if c[0] == "rsync"]
        self.assertEqual(len(rsyncs), 1)
        self.assertIn("collect script node0 failed", logs.output[0])
        self.assertIn("exited 124", logs.output[0])

    def test_a_failed_script_and_rsync_are_both_reported(self):
        runner = Scripted({"docker logs": [completed(returncode=124)]}, rsync_rc=23)
        tmp = tmp_dir(self)
        remote = scripted_remote(self, runner, tmp)
        with self.assertLogs("aws-bench", "WARNING") as logs:
            awsb.collect_hosts(remote, [remote.hosts["node0"]], tmp, "sub")
        self.assertEqual(len(logs.output), 2)
        self.assertIn("collect script node0 failed", logs.output[0])
        self.assertIn("collect node0 rsync failed", logs.output[1])

    def test_freeze_names_each_roles_containers(self):
        runner = Scripted({})
        remote = scripted_remote(self, runner)
        awsb.freeze(remote)
        by_host = {c[-2].split("@")[1]: c[-1] for c in runner.calls}
        self.assertIn("anvil orchestrator state-relay-server", by_host["203.0.113.1"])
        self.assertIn("for c in espresso-node;", by_host["203.0.113.2"])
        self.assertNotIn("postgres", " ".join(by_host.values()))

    def run_freeze(self, docker_stderr: str, docker_rc: int) -> int:
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        docker = tmp / "docker"
        docker.write_text(f"#!/bin/sh\necho '{docker_stderr}' >&2\nexit {docker_rc}\n")
        docker.chmod(0o755)
        env = {"PATH": f"{tmp}:{os.environ['PATH']}"}
        command = awsb.freeze_command(("a", "b"))
        return subprocess.run(["bash", "-c", command], env=env, check=False).returncode

    def test_freeze_command_ignores_only_a_missing_container(self):
        self.assertEqual(self.run_freeze("Error: No such container: a", 1), 0)
        self.assertEqual(self.run_freeze("permission denied", 1), 1)
        self.assertEqual(self.run_freeze("", 0), 0)


# REQ:awsbench-poll-tolerant
class PollAgentTest(unittest.TestCase):
    def poll(self, runner, clock: FakeClock | None = None) -> "awsb.AgentState":
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        tmp = tmp_dir(self)
        remote = scripted_remote(self, runner, tmp)
        return awsb.poll_agent(remote, tmp, cfg, awsb.Interrupts(clock or FakeClock()))

    def state(self, state: dict) -> "subprocess.CompletedProcess":
        return completed(stdout=json.dumps(state))

    def test_ssh_blips_are_retried(self):
        blip = completed(returncode=255, stderr="timed out")
        runner = Scripted(
            {
                "is-active": [blip, blip, completed()],
                "agent-state.json": [self.state(DONE_STATE)],
            }
        )
        clock = FakeClock()
        self.assertEqual(self.poll(runner, clock)["phase"], "done")
        self.assertEqual(clock.sleeps, [awsb.AGENT_POLL_S] * 2)

    # TEST:poll-agent-deadline-fails
    def test_an_agent_that_never_finishes_hits_the_deadline(self):
        loading = self.state({"phase": "loading", "detail": "x"})
        runner = Scripted({"agent-state.json": [loading]})
        clock = FakeClock(limit_s=1e6)
        with self.assertRaisesRegex(awsb.RemoteError, "did not finish within"):
            self.poll(runner, clock)
        self.assertGreater(clock.time(), 0)

    def test_persistent_ssh_failure_raises(self):
        runner = Scripted({"is-active": [completed(returncode=255, stderr="down")]})
        with (
            unittest.mock.patch.object(awsb, "AGENT_POLL_SSH_FAILURES_MAX", 2),
            self.assertRaisesRegex(awsb.RemoteError, "unreachable for 3 polls"),
        ):
            self.poll(runner)

    def test_inactive_agent_without_done_raises(self):
        loading = self.state({"phase": "loading", "detail": "x"})
        runner = Scripted(
            {
                "is-active": [completed(returncode=awsb.SYSTEMCTL_INACTIVE_RC)],
                "agent-state.json": [loading],
            }
        )
        with self.assertRaisesRegex(awsb.RemoteError, "agent exited in phase loading"):
            self.poll(runner)

    def test_collected_agent_unit_with_done_state_returns(self):
        runner = Scripted(
            {
                "is-active": [completed(returncode=awsb.SYSTEMCTL_NO_UNIT_RC)],
                "agent-state.json": [self.state(DONE_STATE)],
            }
        )
        self.assertEqual(self.poll(runner)["phase"], "done")

    def test_collected_agent_unit_without_done_raises(self):
        loading = self.state({"phase": "loading", "detail": "x"})
        runner = Scripted(
            {
                "is-active": [completed(returncode=awsb.SYSTEMCTL_NO_UNIT_RC)],
                "agent-state.json": [loading],
            }
        )
        with self.assertRaisesRegex(
            awsb.RemoteError, "agent exited .* without finishing"
        ):
            self.poll(runner)

    def test_unexpected_is_active_status_raises(self):
        runner = Scripted({"is-active": [completed(returncode=1)]})
        with self.assertRaisesRegex(awsb.RemoteError, "is-active exited 1"):
            self.poll(runner)

    def test_failed_partial_rsync_only_warns(self):
        loading = self.state({"phase": "loading", "detail": "x"})
        polls = int(awsb.OUT_RSYNC_S / awsb.AGENT_POLL_S) + 1
        runner = Scripted(
            {"agent-state.json": [loading] * polls + [self.state(DONE_STATE)]},
            rsync_rc=23,
        )
        with self.assertLogs("aws-bench", "WARNING") as logs:
            self.assertEqual(self.poll(runner)["phase"], "done")
        self.assertIn("partial rsync failed", logs.output[0])


class PollStepTest(unittest.TestCase):
    BEGIN = 100.0

    def poll_state(self, **overrides) -> "awsb.PollState":
        state = awsb.PollState(
            begin=self.BEGIN,
            deadline=self.BEGIN + 1000,
            next_log=self.BEGIN + awsb.AGENT_LOG_S,
            next_sync=self.BEGIN + awsb.OUT_RSYNC_S,
            unreachable=0,
            state=None,
        )
        return {**state, **overrides}

    def step(self, state, rc=0, agent=None, now=BEGIN, stderr="") -> "awsb.PollStep":
        return awsb.poll_step(state, rc, stderr, agent, now)

    # TEST:poll-step-done-ok
    def test_done_phase_is_done(self):
        step = self.step(self.poll_state(), agent=DONE_STATE)
        self.assertTrue(step["done"])
        self.assertIsNone(step["error"])
        self.assertEqual(step["state"]["state"], DONE_STATE)

    def test_done_wins_over_an_inactive_unit_and_the_deadline(self):
        step = self.step(
            self.poll_state(),
            rc=awsb.SYSTEMCTL_NO_UNIT_RC,
            agent=DONE_STATE,
            now=self.BEGIN + 5000,
        )
        self.assertTrue(step["done"])

    def test_error_phase_reports_the_agent_error(self):
        agent = {"phase": "error", "detail": "x", "error": "boom"}
        step = self.step(self.poll_state(), agent=agent)
        self.assertEqual(step["error"], "agent failed: boom")
        self.assertFalse(step["done"])

    # TEST:poll-step-unreachable-fails
    def test_unreachable_ctl_fails_after_the_bound(self):
        state = self.poll_state()
        for count in range(1, awsb.AGENT_POLL_SSH_FAILURES_MAX + 1):
            step = self.step(state, rc=awsb.SSH_FAILED_RC, stderr="down")
            self.assertIsNone(step["error"])
            self.assertTrue(step["retry"])
            self.assertEqual(step["state"]["unreachable"], count)
            state = step["state"]
        step = self.step(state, rc=awsb.SSH_FAILED_RC, stderr=" down\n")
        self.assertEqual(
            step["error"],
            f"ctl unreachable for {awsb.AGENT_POLL_SSH_FAILURES_MAX + 1} polls: down",
        )
        self.assertFalse(step["retry"])

    def test_a_reachable_probe_resets_the_unreachable_count(self):
        loading = {"phase": "loading", "detail": "x"}
        step = self.step(self.poll_state(unreachable=3), agent=loading)
        self.assertEqual(step["state"]["unreachable"], 0)
        self.assertFalse(step["retry"])

    def test_unexpected_probe_rc_is_an_error(self):
        step = self.step(self.poll_state(), rc=1, stderr="odd\n")
        self.assertEqual(step["error"], "systemctl is-active exited 1: odd")

    def test_inactive_unit_without_done_is_an_error(self):
        loading = {"phase": "loading", "detail": "x"}
        for rc in (awsb.SYSTEMCTL_INACTIVE_RC, awsb.SYSTEMCTL_NO_UNIT_RC):
            step = self.step(self.poll_state(), rc=rc, agent=loading)
            self.assertEqual(
                step["error"], "agent exited in phase loading without finishing"
            )

    def test_inactive_unit_without_state_names_the_start_phase(self):
        step = self.step(self.poll_state(), rc=awsb.SYSTEMCTL_INACTIVE_RC)
        self.assertEqual(step["error"], "agent exited in phase start without finishing")

    def test_no_state_file_after_the_grace_is_an_error(self):
        state = self.poll_state()
        within = self.step(state, now=self.BEGIN + awsb.AGENT_START_GRACE_S)
        self.assertIsNone(within["error"])
        after = self.step(state, now=self.BEGIN + awsb.AGENT_START_GRACE_S + 1)
        self.assertEqual(after["error"], "agent wrote no state file")

    def test_earlier_state_is_kept_when_the_file_is_unreadable(self):
        loading = {"phase": "loading", "detail": "x"}
        step = self.step(self.poll_state(state=loading), now=self.BEGIN + 5000)
        self.assertEqual(step["state"]["state"], loading)
        self.assertEqual(step["error"], "agent did not finish within the expected time")

    def test_deadline_is_exclusive(self):
        loading = {"phase": "loading", "detail": "x"}
        state = self.poll_state(deadline=self.BEGIN + 10)
        self.assertIsNone(self.step(state, agent=loading, now=self.BEGIN + 10)["error"])
        late = self.step(state, agent=loading, now=self.BEGIN + 10.5)
        self.assertEqual(late["error"], "agent did not finish within the expected time")

    def test_log_and_rsync_come_due_and_reschedule(self):
        loading = {"phase": "loading", "detail": "x"}
        state = self.poll_state()
        idle = self.step(state, agent=loading, now=self.BEGIN)
        self.assertEqual((idle["log"], idle["rsync"]), (False, False))
        now = self.BEGIN + max(awsb.AGENT_LOG_S, awsb.OUT_RSYNC_S)
        due = self.step(state, agent=loading, now=now)
        self.assertEqual((due["log"], due["rsync"]), (True, True))
        self.assertEqual(due["state"]["next_log"], now + awsb.AGENT_LOG_S)
        self.assertEqual(due["state"]["next_sync"], now + awsb.OUT_RSYNC_S)

    def test_log_waits_for_a_state_but_rsync_does_not(self):
        now = self.BEGIN + awsb.AGENT_START_GRACE_S
        state = self.poll_state(next_log=now, next_sync=now)
        step = self.step(state, now=now)
        self.assertEqual((step["log"], step["rsync"]), (False, True))
        self.assertEqual(step["state"]["next_log"], now)

    def test_input_state_is_not_mutated(self):
        state = self.poll_state()
        before = dict(state)
        self.step(state, rc=awsb.SSH_FAILED_RC)
        self.step(state, agent=DONE_STATE, now=self.BEGIN + 5000)
        self.assertEqual(state, before)


class RetryVerdictTest(unittest.TestCase):
    # TEST:retry-verdict-ok
    def test_ok_even_past_the_deadline(self):
        self.assertEqual(awsb.retry_verdict(True, 0.0, 10.0), "ok")
        self.assertEqual(awsb.retry_verdict(True, 99.0, 10.0), "ok")

    def test_other_rc_before_the_deadline_retries(self):
        self.assertEqual(awsb.retry_verdict(False, 9.9, 10.0), "retry")
        self.assertEqual(awsb.retry_verdict(False, 0.0, 10.0), "retry")

    # TEST:retry-verdict-timeout-fails
    def test_other_rc_at_or_after_the_deadline_times_out(self):
        self.assertEqual(awsb.retry_verdict(False, 10.0, 10.0), "timeout")
        self.assertEqual(awsb.retry_verdict(False, 11.0, 10.0), "timeout")


class WaitCloudInitTest(unittest.TestCase):
    def wait(self, rc: int, status: str) -> None:
        runner = Scripted(
            {
                "--wait": [completed(returncode=rc, stderr="err")],
                "--format json": [completed(stdout=json.dumps({"status": status}))],
            }
        )
        awsb.wait_cloud_init(scripted_remote(self, runner), "ctl")

    def test_recoverable_errors_are_accepted(self):
        self.wait(2, "done")

    def test_clean_done_is_accepted(self):
        self.wait(0, "done")

    def test_cloud_init_error_raises(self):
        with self.assertRaisesRegex(awsb.RemoteError, "exited 1"):
            self.wait(1, "error")

    def test_status_other_than_done_raises(self):
        with self.assertRaisesRegex(awsb.RemoteError, "'degraded'"):
            self.wait(2, "degraded")


class StartSupportTest(unittest.TestCase):
    def test_deploy_timeout_names_the_limit(self):
        runner = Scripted({"docker wait deploy": [completed(returncode=124)]})
        remote = scripted_remote(self, runner)
        with self.assertRaisesRegex(
            awsb.RemoteError, f"still running after {awsb.DEPLOY_TIMEOUT_S} s"
        ):
            awsb.start_support(remote, [], awsb.Interrupts(FakeClock()))

    def test_deploy_failure_status_is_reported(self):
        runner = Scripted({"docker wait deploy": [completed(stdout="1\n")]})
        remote = scripted_remote(self, runner)
        with self.assertRaisesRegex(awsb.RemoteError, "deploy exited with status 1"):
            awsb.start_support(remote, [], awsb.Interrupts(FakeClock()))


class SupportPlanTest(unittest.TestCase):
    def containers(self, query_db: awsb.DbMode) -> list[str | None]:
        plan = awsb.support_plan(query_db, ["0xabc"])
        return [step["container"] for step in plan]

    # TEST:support-plan-order-ok
    def test_containers_start_in_order_and_postgres_last(self):
        self.assertEqual(
            self.containers("colocated"),
            [
                "anvil",
                "deploy",
                None,
                "orchestrator",
                "state-relay-server",
                "postgres",
                None,
                None,
            ],
        )

    def test_rds_is_gated_but_not_started(self):
        colocated = awsb.support_plan("colocated", [])
        rds = awsb.support_plan("rds", [])
        self.assertNotIn("postgres", [step["container"] for step in rds])
        self.assertEqual(
            [step["what"] for step in rds], [step["what"] for step in colocated]
        )
        self.assertEqual(rds[-3]["what"], "pg_isready")

    def test_deploy_is_waited_on_not_polled(self):
        deploy = awsb.support_plan("colocated", [])[1]
        self.assertEqual(deploy["container"], "deploy")
        self.assertEqual(deploy["kind"], "deploy")
        self.assertEqual(deploy["timeout_s"], awsb.DEPLOY_TIMEOUT_S)

    def test_every_contract_is_checked_between_deploy_and_orchestrator(self):
        plan = awsb.support_plan("colocated", ["0xabc", "0xdef"])
        whats = [step["what"] for step in plan]
        self.assertEqual(
            whats[1:6],
            [
                "deploy",
                "eth_getCode 0xabc",
                "eth_getCode 0xdef",
                "orchestrator healthcheck",
                "relay healthcheck",
            ],
        )
        check = plan[3]
        assert check["kind"] == "gate"
        self.assertIn("0xdef", check["gate_cmd"])

    def test_gates_use_the_gate_timeout_and_the_right_host(self):
        for step in awsb.support_plan("colocated", ["0xabc"]):
            if step["kind"] == "gate":
                self.assertEqual(step["timeout_s"], awsb.GATE_TIMEOUT_S)
        hosts = [step["host"] for step in awsb.support_plan("colocated", [])]
        self.assertEqual(hosts, ["ctl"] * 4 + ["node0"] * 3)

    def test_start_support_starts_the_planned_containers_in_order(self):
        runner = Scripted({"docker wait deploy": [completed(stdout="0\n")]})
        remote = scripted_remote(self, runner)
        awsb.start_support(remote, ["0xabc"], awsb.Interrupts(FakeClock()), "rds")
        commands = [c[-1] for c in runner.calls if c[0] == "ssh"]
        started = [
            c.rpartition("docker start ")[2] for c in commands if "docker start" in c
        ]
        self.assertEqual(
            started, ["anvil", "deploy", "orchestrator", "state-relay-server"]
        )


class CollectPlanTest(unittest.TestCase):
    # TEST:collect-plan-skip-ok
    def test_a_clean_run_freezes_collects_and_fetches_the_balance(self):
        self.assertEqual(
            awsb.collect_plan(None, True, "colocated"),
            ["freeze", "collect_hosts", "rsync_out", "ebs_balance"],
        )

    def test_no_agent_means_no_final_rsync(self):
        self.assertNotIn("rsync_out", awsb.collect_plan(None, False, "colocated"))

    def test_an_error_stops_the_agent_first(self):
        plan = awsb.collect_plan("boom", True, "colocated")
        self.assertEqual(plan[0], "stop_agent")
        self.assertEqual(plan[1:], awsb.collect_plan(None, True, "colocated"))

    def test_rds_adds_its_collection_last(self):
        self.assertEqual(awsb.collect_plan(None, True, "rds")[-1], "collect_rds")
        self.assertNotIn("collect_rds", awsb.collect_plan(None, True, "volume"))


class FinishRunSkipTest(unittest.TestCase):
    """`skip_collect` is checked before every step, not once when the plan is made."""

    def finish(self, on_freeze) -> tuple[list[str], list[str]]:
        """(steps that ran, warnings) of a `finish_run` whose `freeze` calls `on_freeze`."""
        tmp = tmp_dir(self)
        interrupts = awsb.Interrupts(FakeClock())
        fleet = awsb.FleetState(
            FakeSystem(),
            awsb.RunConfig(tag="x"),
            tmp,
            None,
            interrupts,
            scripted_remote(self, Scripted({}), tmp),
        )
        run = awsb.Run(fleet, tmp, fleet.cfg, 0.0, agent_started=True)
        ran: list[str] = []

        def step(name: str, after=lambda: None):
            def action(*args, **kwargs):
                ran.append(name)
                after()

            return action

        with (
            unittest.mock.patch.object(awsb, "update_manifest"),
            unittest.mock.patch.object(awsb, "report"),
            unittest.mock.patch.object(
                awsb, "freeze", step("freeze", lambda: on_freeze(interrupts))
            ),
            unittest.mock.patch.object(awsb, "collect_hosts", step("collect")),
            unittest.mock.patch.object(awsb.Remote, "rsync_from", step("final rsync")),
            unittest.mock.patch.object(
                awsb, "collect_node0_ebs_balance", step("node0 EBS balance")
            ),
            self.assertLogs(awsb.log, "INFO") as logs,
        ):
            awsb.finish_run(run)
        warnings = [r.getMessage() for r in logs.records if r.levelname == "WARNING"]
        return ran, warnings

    def test_a_signal_during_a_step_skips_the_steps_after_it(self):
        ran, warnings = self.finish(lambda i: i.skip_collect.set())
        self.assertEqual(ran, ["freeze"])
        self.assertEqual(
            warnings,
            ["collect skipped", "final rsync skipped", "node0 EBS balance skipped"],
        )

    def test_without_a_signal_every_step_runs_in_plan_order(self):
        ran, warnings = self.finish(lambda i: None)
        self.assertEqual(ran, ["freeze", "collect", "final rsync", "node0 EBS balance"])
        self.assertEqual(warnings, [])


class DestroyVerdictTest(unittest.TestCase):
    # TEST:destroy-verdict-ok
    def test_success_is_done_at_any_attempt(self):
        for attempt in range(1, awsb.DESTROY_RETRIES + 1):
            self.assertEqual(awsb.destroy_verdict(attempt, True), "done")

    def test_failure_retries_below_the_bound(self):
        for attempt in range(1, awsb.DESTROY_RETRIES):
            self.assertEqual(awsb.destroy_verdict(attempt, False), "retry")

    def test_failure_at_the_bound_sweeps(self):
        self.assertEqual(awsb.destroy_verdict(awsb.DESTROY_RETRIES, False), "sweep")


class DestroyFleetBackoffTest(unittest.TestCase):
    def destroy(self, failures: int) -> tuple[bool, FakeClock, unittest.mock.Mock]:
        clock = FakeClock()
        terraform = unittest.mock.Mock()
        terraform.destroy.side_effect = [
            awsb.TfFailed("destroy", "locked")
        ] * failures + [None]
        fleet = awsb.FleetState(
            FakeSystem(clock=clock),
            awsb.RunConfig(tag="x"),
            tmp_dir(self) / "fleet1",
            terraform,
            awsb.Interrupts(clock),
        )
        with unittest.mock.patch.object(awsb, "sweep", return_value=[]) as sweep:
            return awsb.destroy_fleet(fleet), clock, sweep

    def test_backs_off_between_attempts_and_sweeps_after_the_last(self):
        destroyed, clock, sweep = self.destroy(awsb.DESTROY_RETRIES)
        self.assertFalse(destroyed)
        self.assertEqual(clock.sleeps, [awsb.DESTROY_BACKOFF_S] * 2)
        sweep.assert_called_once()

    def test_a_late_success_does_not_sweep(self):
        destroyed, clock, sweep = self.destroy(1)
        self.assertTrue(destroyed)
        self.assertEqual(clock.sleeps, [awsb.DESTROY_BACKOFF_S])
        sweep.assert_not_called()


SWEEP_ARNS = [
    "arn:aws:ec2:eu-west-1:1:key-pair/key-1",
    "arn:aws:ec2:eu-west-1:1:security-group/sg-1",
    "arn:aws:ec2:eu-west-1:1:volume/vol-1",
    "arn:aws:rds:eu-west-1:1:pg:espresso-bench-f",
    "arn:aws:rds:eu-west-1:1:subgrp:espresso-bench-f",
    "arn:aws:rds:eu-west-1:1:db:espresso-bench-f",
    "arn:aws:ec2:eu-west-1:1:instance/i-1",
    "arn:aws:scheduler:eu-west-1:1:schedule-group/espresso-bench-f",
]


class SweepPlanTest(unittest.TestCase):
    def verbs(self, arns: list[str]) -> list[str]:
        plan = awsb.sweep_plan(arns)
        return [
            " ".join(step["args"][:2]) if step["action"] == "aws" else step["action"]
            for step in plan
        ]

    # TEST:sweep-plan-order-ok
    def test_instances_terminate_before_volumes_security_group_and_key(self):
        self.assertEqual(
            self.verbs(SWEEP_ARNS),
            [
                "scheduler delete-schedule-group",
                "ec2 terminate-instances",
                "ec2 wait",
                "delete_rds",
                "rds delete-db-subnet-group",
                "rds delete-db-parameter-group",
                "ec2 delete-volume",
                "delete_security_group",
                "ec2 delete-key-pair",
            ],
        )

    def test_no_resources_no_steps(self):
        self.assertEqual(awsb.sweep_plan([]), [])

    def test_steps_carry_id_and_tolerated_error(self):
        plan = awsb.sweep_plan(SWEEP_ARNS[2:3])
        self.assertEqual(
            plan,
            [
                {
                    "action": "aws",
                    "args": ["ec2", "delete-volume", "--volume-id", "vol-1"],
                    "tolerate": "InvalidVolume.NotFound",
                }
            ],
        )

    def test_rds_and_security_group_steps_name_their_resource(self):
        plan = awsb.sweep_plan(SWEEP_ARNS)
        resources = {
            step["action"]: step["resource"] for step in plan if step["action"] != "aws"
        }
        self.assertEqual(
            resources,
            {"delete_rds": "espresso-bench-f", "delete_security_group": "sg-1"},
        )

    def test_instance_ids_are_terminated_and_waited_on_together(self):
        arns = [f"arn:aws:ec2:eu-west-1:1:instance/i-{n}" for n in (1, 2)]
        terminate, wait = awsb.sweep_plan(arns)
        assert terminate["action"] == "aws" and wait["action"] == "aws"
        self.assertEqual(terminate["args"][-3:], ["--instance-ids", "i-1", "i-2"])
        self.assertEqual(wait["args"][:2], ["ec2", "wait"])
        self.assertEqual(wait["args"][-3:], ["--instance-ids", "i-1", "i-2"])


class PostgresStatsTest(unittest.TestCase):
    def test_sql_is_one_shell_word_per_command(self):
        sql = awsb.pg_sample_sql()
        self.assertEqual(sql.count("("), sql.count(")"))
        self.assertEqual(sql.count("'") % 2, 0)
        self.assertIn("pg_stat_checkpointer", sql)
        script = awsb.PG_STATS_SCRIPT
        for name in ("pg-stats.json", "pg-statements.json", "pg-settings.json"):
            self.assertIn(f"> {awsb.BENCH_DIR}/{name}", script)
        for line, sql in zip(
            script.splitlines()[-3:],
            (awsb.PG_STATS_SQL, awsb.PG_STATEMENTS_SQL, awsb.PG_SETTINGS_SQL),
        ):
            words = shlex.split(line)
            self.assertEqual(words[:4], ["psql", "-At", "-c", sql])

    def test_collect_script_for_node0_dumps_statements_and_settings(self):
        runner = Scripted({})
        tmp = tmp_dir(self)
        remote = scripted_remote(self, runner, tmp)
        awsb.collect_hosts(remote, [remote.hosts["node0"]], tmp)
        script = runner.calls[0][-1]
        self.assertIn("pg-statements.json", script)
        self.assertIn("pg-settings.json", script)
        self.assertIn("pg_stat_statements", script)

    def test_collect_script_for_validator_has_no_postgres_dump(self):
        runner = Scripted({})
        tmp = tmp_dir(self)
        remote = scripted_remote(self, runner, tmp)
        awsb.collect_hosts(remote, [remote.hosts["node1"]], tmp)
        self.assertNotIn("pg-statements.json", runner.calls[0][-1])

    def test_agent_host_writes_pg_lines_and_skips_failed_ticks(self):
        tmp = tmp_dir(self)
        stop = threading.Event()
        replies = [
            completed(returncode=1, stderr="no such container"),
            completed(returncode=1, stderr="not ready"),
            completed(stdout='{"xact_commit": 7, "wait_events": {"IO": 1}}\n'),
        ]
        commands: list[list[str]] = []

        def run(argv, env=None):
            commands.append(argv)
            reply = replies.pop(0)
            if not replies:
                stop.set()
            return reply

        out = tmp / "pg-stats.jsonl"
        clock = FakeClock()
        awsb.sample_pg(out, stop, awsb.pg_endpoint(), FakeSystem(run=run, clock=clock))
        self.assertEqual(clock.sleeps, [awsb.PG_SAMPLE_S] * 2)
        lines = [json.loads(line) for line in out.read_text().splitlines()]
        self.assertEqual(len(lines), 1)
        self.assertEqual(lines[0]["xact_commit"], 7)
        self.assertIn("ts", lines[0])
        self.assertIn("pg_stat_checkpointer", commands[-1][-1])
        self.assertEqual(len(commands), 3)

    def test_agent_host_role_query_starts_pg_sampler(self):
        tmp = tmp_dir(self)
        pg_json = tmp / "pg.json"
        pg_json.write_text(json.dumps(awsb.pg_endpoint()))
        args = awsb.parse_args(
            [
                "agent-host",
                str(tmp / "host.jsonl"),
                "--role",
                "query",
                "--pg",
                str(pg_json),
            ]
        )
        probed = threading.Event()

        def run(argv, env=None):
            probed.set()
            return completed(returncode=1)

        system = FakeSystem(run=run, clock=netbench.SYSTEM_CLOCK)

        def stop_soon():
            probed.wait(5)
            system.fire(signal.SIGINT)

        with (
            unittest.mock.patch.object(awsb, "BENCH_DIR", str(tmp)),
            unittest.mock.patch.object(awsb, "host_sample", return_value={"ts": 1}),
        ):
            threading.Thread(target=stop_soon).start()
            self.assertEqual(awsb.cmd_agent_host(args, system), awsb.EXIT_OK)
        self.assertTrue((tmp / "pg-stats.jsonl").exists())

    def test_extension_created_after_pg_isready(self):
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result()
        ):
            RunHarness(self).run(runner)
        commands = [" ".join(c) for c in runner.calls]
        ready = next(i for i, c in enumerate(commands) if "pg_isready" in c)
        extension = next(
            i for i, c in enumerate(commands) if "CREATE EXTENSION IF NOT EXISTS" in c
        )
        self.assertGreater(extension, ready)
        self.assertIn(awsb.PG_JSON, commands[ready])


def pg_settings(**overrides) -> dict:
    """pg-settings.json of a node0 whose container took every PG_TUNING value."""
    settings = {key: setting for key, (_, setting) in awsb.PG_TUNING.items()}
    return {**settings, **overrides}


def query_spec() -> dict:
    return {
        "name": "node0",
        "role": "query",
        "instance_type": "x",
        "root_gb": 100,
        "root_iops": 6000,
        "root_mbps": 500,
    }


# REQ:querydb-settings-parity
class PgTuningTest(unittest.TestCase):
    def test_container_args_render_every_tuning_key(self):
        args = awsb.render_postgres_args()
        flags = args[0::2]
        self.assertEqual(set(flags), {"-c"})
        settings = dict(setting.split("=", 1) for setting in args[1::2])
        self.assertEqual(
            {k: settings[k] for k in awsb.PG_TUNING},
            {k: conf for k, (conf, _) in awsb.PG_TUNING.items()},
        )
        self.assertEqual(settings["ssl"], "on")

    def test_bind_parameters_are_not_logged(self):
        """Slow payload INSERT statements would otherwise log their ~6 MB bytea binds as hex."""
        args = awsb.render_postgres_args()
        settings = dict(setting.split("=", 1) for setting in args[1::2])
        self.assertEqual(settings["log_parameter_max_length"], "0")
        rds = awsb.rds_tfvars({}, "pw", datetime(2026, 1, 1, tzinfo=UTC))["rds"][
            "parameters"
        ]
        self.assertEqual(rds["log_parameter_max_length"], "0")

    def test_memory_budget_fits_the_query_host(self):
        """node0 is a c8g.4xlarge with 32 GiB; the same map runs on the 64 GiB RDS class."""
        self.assertEqual(awsb.NODE_TYPE, "c8g.4xlarge")
        ram = 32 * 1024**3
        pages, kib = 8192, 1024
        tuning = awsb.PG_TUNING

        def size(key: str, unit: int) -> int:
            return int(tuning[key][1]) * unit

        peak = (
            size("shared_buffers", pages)
            + size("wal_buffers", pages)
            + size("autovacuum_max_workers", 1) * size("autovacuum_work_mem", kib)
        )
        self.assertLessEqual(peak, ram // 3)
        self.assertLessEqual(size("effective_cache_size", pages), ram)

    def test_settings_whose_rds_default_differs_are_pinned(self):
        for key in (
            "autovacuum_naptime",
            "autovacuum_vacuum_scale_factor",
            "autovacuum_analyze_scale_factor",
            "autovacuum_work_mem",
            "track_io_timing",
            "wal_buffers",
            "max_connections",
        ):
            self.assertIn(key, awsb.PG_TUNING)

    def test_rds_values_are_the_conf_values_in_setting_units(self):
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
                self.assertEqual(setting, conf, key)
                continue
            number, suffix = int(match[1]), match[2]
            if suffix in times:
                self.assertEqual(int(setting), number * times[suffix], key)
            else:
                self.assertEqual(int(setting) * units[key], number * sizes[suffix], key)

    def test_matching_settings_report_nothing(self):
        self.assertEqual(awsb.check_pg_tuning(pg_settings()), [])

    def test_differing_setting_is_named(self):
        settings = pg_settings(shared_buffers="16384", max_wal_size="1024")
        self.assertEqual(
            awsb.check_pg_tuning(settings), ["shared_buffers", "max_wal_size"]
        )

    def test_missing_setting_raises(self):
        settings = pg_settings()
        del settings["work_mem"]
        with self.assertRaises(KeyError):
            awsb.check_pg_tuning(settings)

    def test_settings_query_selects_every_tuning_key(self):
        for key in awsb.PG_TUNING:
            self.assertIn(f"'{key}'", awsb.PG_SETTINGS_SQL)

    def test_tls_is_counted_by_the_load_samples_not_the_collect_query(self):
        """Freeze stops espresso-node before the settings query, so only the sampler sees it."""
        self.assertNotIn("pg_stat_ssl", awsb.PG_SETTINGS_SQL)
        sql = awsb.pg_sample_sql()
        ssl = sql[sql.index("'ssl_backends'") :]
        self.assertIn("pg_stat_ssl", ssl)
        self.assertIn("pid <> pg_backend_pid()", ssl)

    def test_tls_is_none_without_backends(self):
        self.assertIsNone(awsb.pg_tls({}))
        self.assertTrue(awsb.pg_tls({"true": 3}))
        self.assertFalse(awsb.pg_tls({"true": 2, "false": 1}))


# REQ:querydb-settings-parity
class PgValidityTest(unittest.TestCase):
    def check(self, **evidence):
        return awsb.check_validity_aws(
            clean_result(), aws_manifest(), {**clean_evidence(), **evidence}
        )

    def test_matching_settings_and_tls_are_quiet(self):
        verdict = self.check(pg_settings=pg_settings(), ssl_backends={"true": 3})
        self.assertEqual(verdict, {"valid": True, "noisy": False, "reasons": []})

    def test_missing_settings_are_not_judged(self):
        self.assertFalse(self.check()["noisy"])

    def test_differing_setting_is_noisy_with_its_value(self):
        verdict = self.check(pg_settings=pg_settings(shared_buffers="16384"))
        self.assertTrue(verdict["valid"])
        self.assertTrue(verdict["noisy"])
        self.assertEqual(len(verdict["reasons"]), 1)
        self.assertIn("shared_buffers=16384", verdict["reasons"][0])

    def test_backend_without_tls_is_noisy(self):
        verdict = self.check(ssl_backends={"true": 4, "false": 1})
        self.assertTrue(verdict["noisy"])
        self.assertIn("without TLS", verdict["reasons"][0])

    def test_no_backends_during_the_load_is_quiet(self):
        self.assertFalse(self.check(ssl_backends={})["noisy"])

    def test_evidence_sums_the_backends_of_the_samples_inside_the_load_window(self):
        tmp = tmp_dir(self)
        node0 = tmp / "hosts" / "node0"
        node0.mkdir(parents=True)
        samples = [
            {"ts": 5.0, "ssl_backends": {"false": 1}},
            {"ts": 10.0, "ssl_backends": {"true": 2}},
            {"ts": 15.0, "ssl_backends": {"true": 1}},
            {"ts": 25.0, "ssl_backends": {"false": 1}},
        ]
        netbench.write_jsonl(node0 / "pg-stats.jsonl", iter(samples))
        _, evidence = awsb.load_evidence(tmp, aws_manifest(), 10.0, 20.0)
        self.assertEqual(evidence["ssl_backends"], {"true": 3})
        (node0 / "pg-stats.jsonl").unlink()
        _, evidence = awsb.load_evidence(tmp, aws_manifest(), 10.0, 20.0)
        self.assertNotIn("ssl_backends", evidence)

    def test_evidence_reads_settings_and_ignores_an_empty_file(self):
        tmp = tmp_dir(self)
        manifest = aws_manifest()
        node0 = tmp / "hosts" / "node0"
        node0.mkdir(parents=True)
        (node0 / "pg-settings.json").write_text(json.dumps(pg_settings()))
        _, evidence = awsb.load_evidence(tmp, manifest, 0.0, 1.0)
        self.assertEqual(evidence["pg_settings"], pg_settings())
        (node0 / "pg-settings.json").write_text("")
        _, evidence = awsb.load_evidence(tmp, manifest, 0.0, 1.0)
        self.assertNotIn("pg_settings", evidence)


# REQ:querydb-colocated-wiring
class ColocatedPostgresWiringTest(unittest.TestCase):
    def setUp(self):
        self.images = fake_images()

    def test_psql_password_is_in_the_environment_only(self):
        argv, env = awsb.psql_argv(awsb.pg_endpoint())
        self.assertEqual(argv[:1], ["psql"])
        self.assertEqual(env, {"PGPASSWORD": "password"})
        self.assertNotIn("password", argv)
        self.assertEqual(argv[argv.index("-d") + 1], "espresso")

    def test_container_enables_ssl_with_the_mounted_cert(self):
        script = awsb.render_start_sh(query_spec(), self.images)
        self.assertIn(f"-v {awsb.PG_TLS_DIR}:/tls:ro", script)
        for setting in (
            "ssl=on",
            "ssl_cert_file=/tls/server.crt",
            "ssl_key_file=/tls/server.key",
            "shared_buffers=8GB",
            "checkpoint_timeout=15min",
        ):
            self.assertIn(setting, script)

    def test_container_has_the_shared_memory_rds_has(self):
        script = awsb.render_start_sh(query_spec(), self.images)
        create = next(line for line in script.splitlines() if "--name postgres" in line)
        self.assertIn("--shm-size=8g", create)
        self.assertNotIn("--shm-size", script.replace(create, ""))

    def test_query_user_data_installs_client_and_makes_owned_cert(self):
        text = awsb.render_user_data(query_spec(), self.images, ttl_s=60)
        self.assertIn("jq curl postgresql-client openssl", text)
        self.assertIn("openssl req -x509", text)
        self.assertIn(f"chown 999:999 {awsb.PG_TLS_DIR}/server.key", text)
        self.assertIn(f"chmod 0600 {awsb.PG_TLS_DIR}/server.key", text)

    def test_validator_user_data_has_no_client_or_cert(self):
        validator = {**query_spec(), "name": "node1", "role": "validator"}
        text = awsb.render_user_data(validator, self.images, ttl_s=60)
        self.assertNotIn("postgresql-client", text)
        self.assertNotIn("openssl", text)

    def test_node_env_reads_the_endpoint(self):
        pg = {**awsb.pg_endpoint(), "host": "db.internal", "port": 6432}
        hosts = fleet(5)
        env = dict(
            line.split("=", 1)
            for line in awsb.render_node_env(query_spec(), hosts, pg).splitlines()
        )
        self.assertEqual(env["ESPRESSO_NODE_POSTGRES_HOST"], "db.internal")
        self.assertEqual(env["ESPRESSO_NODE_POSTGRES_PORT"], "6432")

    def test_node_env_of_query_role_needs_an_endpoint(self):
        with self.assertRaisesRegex(ValueError, "PgEndpoint"):
            awsb.render_node_env(query_spec(), fleet(5), None)

    def test_pg_json_is_written_0600_for_the_query_host_only(self):
        cfg = awsb.RunConfig(
            tag="x",
            nodes=2,
            load=netbench.BenchConfig(submit_nodes=1),
            node_env=("A=1",),
        )
        hosts = awsb.plan_hosts(cfg)
        manifest = {"hosts": hosts, "images": fake_images()}
        run_dir = tmp_dir(self)
        for host in hosts:
            (run_dir / "hosts" / host["name"]).mkdir(parents=True)
        awsb.render_host_files(run_dir, cfg, manifest, two_node_hosts_info())
        pg_json = run_dir / "hosts/node0/pg.json"
        self.assertEqual(json.loads(pg_json.read_text()), awsb.pg_endpoint())
        self.assertEqual(pg_json.stat().st_mode & 0o777, 0o600)
        self.assertFalse((run_dir / "hosts/node1/pg.json").exists())
        self.assertFalse((run_dir / "hosts/ctl/pg.json").exists())
        for node in ("node0", "node1"):
            node_env = (run_dir / "hosts" / node / "node.env").read_text()
            self.assertIn("\nA=1\n", node_env)

    def test_pg_json_is_shipped_and_not_collected_back(self):
        self.assertIn("pg.json", awsb.SHIPPED_HOST_FILES)
        self.assertIn("--exclude=/pg.json", awsb.COLLECT_EXCLUDES)

    def test_gates_check_readiness_then_extension_then_reset(self):
        runner = Scripted({"docker wait deploy": [completed(stdout="0\n")]})
        remote = scripted_remote(self, runner)
        awsb.start_support(remote, [], awsb.Interrupts(FakeClock()))
        commands = [c[-1] for c in runner.calls if c[0] == "ssh"]
        order = [
            next(i for i, c in enumerate(commands) if needle in c)
            for needle in (
                "docker start postgres",
                "pg_isready",
                "CREATE EXTENSION IF NOT EXISTS",
                "pg_stat_reset_shared",
            )
        ]
        self.assertEqual(order, sorted(order))
        gate = commands[order[1]]
        self.assertTrue(gate.startswith("sudo bash -c "))
        self.assertIn(awsb.PG_JSON, gate)
        self.assertNotIn("PGPASSWORD=password", gate)

    def test_hostmon_gives_only_the_query_host_the_endpoint(self):
        runner = Scripted({})
        remote = scripted_remote(self, runner)
        awsb.start_hostmon(remote)
        commands = [c[-1] for c in runner.calls if c[0] == "ssh"]
        with_pg = [c for c in commands if f"--pg {awsb.PG_JSON}" in c]
        self.assertEqual(len(commands), 3)
        self.assertEqual(len(with_pg), 1)
        self.assertIn("--role query", with_pg[0])

    def test_agent_host_query_role_without_endpoint_is_refused(self):
        """Refused before installing signal handlers, which would outlive the call."""
        args = awsb.parse_args(["agent-host", "host.jsonl", "--role", "query"])
        system = FakeSystem()
        with self.assertRaisesRegex(awsb.Refused, "--pg"):
            awsb.cmd_agent_host(args, system)
        self.assertEqual(system.handlers, {})

    def test_sampler_passes_the_password_in_the_environment(self):
        stop = threading.Event()
        seen = []

        def run(argv, env=None):
            seen.append((argv, env))
            stop.set()
            return completed(returncode=1)

        awsb.sample_pg(
            tmp_dir(self) / "pg-stats.jsonl",
            stop,
            awsb.pg_endpoint(),
            FakeSystem(run=run),
        )
        argv, env = seen[0]
        self.assertEqual(env, {"PGPASSWORD": "password"})
        self.assertNotIn("password", argv)


class StartNodesSyncTest(unittest.TestCase):
    def test_every_node_waits_concurrently_whatever_the_pool_size(self):
        # Both calls must be inside `docker start` at once; a pool of 1 would time out here.
        barrier = threading.Barrier(2, timeout=2)

        def runner(argv):
            if "docker start" in argv[-1]:
                barrier.wait()
            if "inspect" in argv[-1]:
                return completed(stdout="2026-09-29T15:00:00.1Z\n")
            return completed()

        remote = scripted_remote(self, runner)
        hosts = [remote.hosts["node0"], remote.hosts["node1"]]
        with unittest.mock.patch.object(awsb, "REMOTE_POOL_SIZE", 1):
            self.assertEqual(awsb.start_nodes(remote, hosts, 0.0), 0.0)


class WaitHostsTest(unittest.TestCase):
    def test_digest_mismatches(self):
        images = fake_images()
        good = f"ghcr.io/x/espresso-node@{images['espresso-node']['digest']}"
        self.assertEqual(awsb.digest_mismatches({"espresso-node": good}, images), [])
        self.assertEqual(
            awsb.digest_mismatches({"espresso-node": "ghcr.io/x@sha256:bad"}, images),
            ["espresso-node"],
        )


def aws_manifest(steal_hosts: bool = True) -> dict:
    cfg = awsb.RunConfig(tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1))
    return {
        "name": "run1",
        "fleet": "run1",
        "query_db": "colocated",
        "hosts": awsb.plan_hosts(cfg),
        "images": fake_images(),
        "start_spread_s": 0.5,
    }


def host_sample(util: float = 0.3, steal: float = 0.0) -> dict:
    return {
        "util_mean": util,
        "util_max": util,
        "steal_pct": steal,
        "iowait_pct": 0.0,
        "mem_avail_min_bytes": 1,
        "disk_mb_s": 1.0,
        "net_mb_s": 1.0,
    }


def clean_evidence() -> dict:
    return {
        "coverage": {"ctl": 1.0, "node0": 1.0, "node1": 1.0},
        "journal_bytes": {"node0": 1_000_000, "node1": 1_000_000},
        "clock_offset_ms": {"ctl": 0.2, "node0": 0.3, "node1": 0.4},
        "digest_mismatch": {"ctl": [], "node0": [], "node1": []},
        "ebs_balance_min": {"EBSByteBalance%": 100.0, "EBSIOBalance%": 100.0},
    }


def clean_result() -> dict:
    return {
        "validity": {"valid": True, "noisy": False, "reasons": []},
        "hosts": {n: host_sample() for n in ("ctl", "node0", "node1")},
    }


# REQ:awsbench-validity-aws
class CheckValidityAwsTest(unittest.TestCase):
    def check(self, result=None, manifest=None, **evidence):
        return awsb.check_validity_aws(
            result or clean_result(),
            manifest or aws_manifest(),
            {**clean_evidence(), **evidence},
        )

    def test_clean_input_is_valid_and_quiet(self):
        self.assertEqual(self.check(), {"valid": True, "noisy": False, "reasons": []})

    def test_keeps_netbench_findings(self):
        result = clean_result()
        result["validity"] = {"valid": False, "noisy": True, "reasons": ["base"]}
        verdict = self.check(result)
        self.assertFalse(verdict["valid"])
        self.assertTrue(verdict["noisy"])
        self.assertEqual(verdict["reasons"], ["base"])

    def test_low_sample_coverage_is_invalid(self):
        verdict = self.check(coverage={"ctl": 1.0, "node0": 0.5, "node1": 1.0})
        self.assertFalse(verdict["valid"])
        self.assertIn("node0 host samples cover only 50%", verdict["reasons"][0])

    def test_missing_host_samples_are_invalid(self):
        verdict = self.check(coverage={"ctl": 1.0, "node0": 0.0, "node1": 1.0})
        self.assertFalse(verdict["valid"])

    def test_digest_mismatch_is_invalid(self):
        verdict = self.check(
            digest_mismatch={"ctl": [], "node0": ["postgres"], "node1": []}
        )
        self.assertFalse(verdict["valid"])
        self.assertIn("postgres", verdict["reasons"][0])

    def test_clock_offset_over_a_second_is_invalid(self):
        verdict = self.check(clock_offset_ms={"node1": 1500.0})
        self.assertFalse(verdict["valid"])
        self.assertFalse(verdict["noisy"])

    def test_clock_offset_over_50_ms_is_noisy(self):
        verdict = self.check(clock_offset_ms={"node1": 60.0})
        self.assertTrue(verdict["valid"])
        self.assertTrue(verdict["noisy"])

    def test_start_spread_is_noisy_from_2_s(self):
        manifest = aws_manifest() | {"start_spread_s": 2.0}
        verdict = self.check(manifest=manifest)
        self.assertTrue(verdict["valid"])
        self.assertTrue(verdict["noisy"])

    def test_ctl_cpu_over_85_percent_is_noisy(self):
        result = clean_result()
        result["hosts"]["ctl"] = host_sample(util=0.9)
        verdict = self.check(result)
        self.assertTrue(verdict["noisy"])
        self.assertIn("ctl CPU", verdict["reasons"][0])

    def test_steal_on_a_validator_is_noisy(self):
        result = clean_result()
        result["hosts"]["node1"] = host_sample(steal=6.0)
        verdict = self.check(result)
        self.assertTrue(verdict["noisy"])
        self.assertIn("node1 steal", verdict["reasons"][0])

    def test_journal_near_its_limit_is_noisy(self):
        manifest = aws_manifest()
        spec = next(h for h in manifest["hosts"] if h["name"] == "node1")
        near = int(awsb._journal_max_bytes(spec) * 0.95)
        verdict = self.check(journal_bytes={"node1": near})
        self.assertTrue(verdict["valid"])
        self.assertTrue(verdict["noisy"])
        self.assertIn("node1 journal", verdict["reasons"][0])

    # REQ:ebs-balance-evidence
    def test_ebs_balance_below_100_is_noisy_not_invalid(self):
        verdict = self.check(
            ebs_balance_min={"EBSByteBalance%": 82.4, "EBSIOBalance%": 100.0}
        )
        self.assertTrue(verdict["valid"])
        self.assertTrue(verdict["noisy"])
        self.assertEqual(
            verdict["reasons"], ["node0 EBSByteBalance% fell to 82% during the load"]
        )

    # REQ:ebs-balance-evidence
    def test_ebs_balance_metric_without_datapoint_is_noisy_not_invalid(self):
        verdict = self.check(
            ebs_balance_min={"EBSByteBalance%": 100.0, "EBSIOBalance%": None}
        )
        self.assertTrue(verdict["valid"])
        self.assertTrue(verdict["noisy"])
        self.assertEqual(
            verdict["reasons"],
            ["node0 EBSIOBalance% has no datapoint in the load window"],
        )

    def test_ebs_balance_not_collected_is_noisy_not_invalid(self):
        verdict = self.check(ebs_balance_min={})
        self.assertTrue(verdict["valid"])
        self.assertEqual(verdict["reasons"], ["node0 EBS balance not collected"])


class EvidenceParsersTest(unittest.TestCase):
    def test_parse_clock_offset_ms(self):
        text = "Stratum: 4\nSystem time     : 0.000123456 seconds slow of NTP time\n"
        self.assertAlmostEqual(awsb.parse_clock_offset_ms(text), 0.123456)
        with self.assertRaises(ValueError):
            awsb.parse_clock_offset_ms("nothing")

    def test_parse_du_bytes(self):
        self.assertEqual(awsb.parse_du_bytes("12345\t/data/journal\n"), 12345)

    def test_host_sample_stats_rates_and_coverage(self):
        def sample(ts, disk, net, cpu):
            return {
                "ts": ts,
                "cpu": cpu,
                "mem_avail": 5,
                "disk": {
                    "nvme0n1": {"read_bytes": 0, "write_bytes": disk, "io_ticks_ms": 0},
                    "nvme0n1p1": {
                        "read_bytes": 0,
                        "write_bytes": disk,
                        "io_ticks_ms": 0,
                    },
                },
                "net": {
                    "ens5": {"rx_bytes": net, "tx_bytes": 0},
                    "lo": {"rx_bytes": net * 9, "tx_bytes": 0},
                },
            }

        samples = [
            sample(100.0, 0, 0, [0] * 8),
            sample(102.0, 4_000_000, 2_000_000, [100, 0, 100, 800, 0, 0, 0, 0]),
            sample(104.0, 8_000_000, 4_000_000, [200, 0, 200, 1600, 0, 0, 0, 0]),
        ]
        stats, coverage = awsb.host_sample_stats(samples, 100.0, 104.0)
        self.assertAlmostEqual(stats["disk_mb_s"], 2.0)
        self.assertAlmostEqual(stats["net_mb_s"], 1.0)
        self.assertAlmostEqual(coverage, 1.0)
        _, half = awsb.host_sample_stats(samples[:2], 100.0, 108.0)
        self.assertAlmostEqual(half, 0.5)

    def test_runner_info(self):
        lscpu = (
            "CPU(s):                  16\n"
            "Model name:              Neoverse-V2\n"
            "Flags:                   fp asimd sve\n"
        )
        info = awsb.runner_info(
            lscpu, "MemTotal:       32000000 kB\n", "6.8.0-aws\n", "ami-1"
        )
        self.assertEqual(info["cpu_model"], "Neoverse-V2")
        self.assertEqual(info["nproc"], 16)
        self.assertEqual(info["flags"], ["fp", "asimd", "sve"])
        self.assertEqual(info["mem_total_bytes"], 32_000_000 * 1024)
        self.assertEqual(info["mhz"], [])


class SweepAndCostTest(unittest.TestCase):
    def test_sweep_terminates_then_deletes_group_and_key(self):
        runner = FleetRunner([DONE_STATE])
        arns = awsb.sweep(FakeSystem(run=runner), "run1")
        self.assertEqual(len(arns), 3)
        names = [c[3:5] for c in runner.calls if c[0] == "aws"]
        self.assertEqual(
            names,
            [
                ["resourcegroupstaggingapi", "get-resources"],
                ["ec2", "terminate-instances"],
                ["ec2", "wait"],
                ["ec2", "delete-security-group"],
                ["ec2", "delete-key-pair"],
            ],
        )
        self.assertTrue(runner.ran("Key=espresso-bench-run,Values=run1"))

    def test_security_group_delete_retries_on_dependency_violation(self):
        attempts = []

        def runner(argv):
            attempts.append(argv)
            busy = len(attempts) < 3
            return completed(
                returncode=254 if busy else 0,
                stderr="DependencyViolation: has a dependent object" if busy else "",
            )

        clock = FakeClock()
        awsb.delete_security_group(FakeSystem(run=runner, clock=clock), "sg-1")
        self.assertEqual(len(attempts), 3)
        self.assertEqual(clock.sleeps, [awsb.SG_DELETE_BACKOFF_S] * 2)

    def test_security_group_delete_gives_up_after_the_retry_bound(self):
        runner = FakeRunner(
            {("aws",): completed(returncode=254, stderr="DependencyViolation")}
        )
        with self.assertRaisesRegex(awsb.Refused, "DependencyViolation"):
            awsb.delete_security_group(FakeSystem(run=runner), "sg-1")
        self.assertEqual(len(runner.calls), awsb.SG_DELETE_RETRIES)

    def test_security_group_delete_does_not_retry_other_errors(self):
        runner = FakeRunner({("aws",): completed(returncode=254, stderr="denied")})
        with self.assertRaisesRegex(awsb.Refused, "denied"):
            awsb.delete_security_group(FakeSystem(run=runner), "sg-1")
        self.assertEqual(len(runner.calls), 1)

    def test_tagged_resources_without_name_matches_every_run(self):
        runner = FleetRunner([DONE_STATE])
        awsb.tagged_resources(runner)
        self.assertTrue(runner.ran("Key=espresso-bench-run"))
        self.assertFalse(runner.ran("Values="))

    def test_aws_failure_raises(self):
        runner = FakeRunner({("aws",): completed(returncode=1, stderr="denied")})
        with self.assertRaisesRegex(awsb.Refused, "denied"):
            awsb.tagged_resources(runner)

    def test_actual_cost_prices_the_observed_duration(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        estimate = shot_estimate(hosts, cfg)
        manifest = {
            "name": "run1",
            "hosts": hosts,
            "estimate": estimate,
        }
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        cost = awsb.actual_cost(runner, manifest, NOW)
        self.assertEqual(cost["duration_s"], 1800.0)
        expected_lines = awsb._cost_lines(hosts, 1800.0, None)
        self.assertAlmostEqual(
            cost["actual"], sum(line["usd"] for line in expected_lines)
        )
        self.assertEqual(cost["bound"], estimate["bound_usd"])

    def test_actual_cost_skips_rows_without_a_reason(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        estimate = shot_estimate(hosts, cfg)
        manifest = {
            "name": "run1",
            "hosts": hosts,
            "estimate": estimate,
        }
        launch = "2026-09-29T15:00:00+00:00"
        rows = [
            {"launch": launch, "reason": None},
            {"launch": launch, "reason": "User initiated (2026-09-29 15:30:00 GMT)"},
        ]
        runner = FleetRunner([DONE_STATE], describe=json.dumps(rows))
        self.assertEqual(awsb.actual_cost(runner, manifest, NOW)["duration_s"], 1800.0)
        only_none = json.dumps(rows[:1])
        with self.assertRaises(ValueError):
            awsb.actual_cost(
                FleetRunner([DONE_STATE], describe=only_none), manifest, NOW
            )

    def test_actual_cost_without_termination_time_raises(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        estimate = shot_estimate(hosts, cfg)
        manifest = {
            "name": "run1",
            "hosts": hosts,
            "estimate": estimate,
        }
        running = json.dumps([{"launch": "2026-09-29T15:00:00+00:00", "reason": ""}])
        with self.assertRaises(ValueError):
            awsb.actual_cost(FleetRunner([DONE_STATE], describe=running), manifest, NOW)


class IndexTest(unittest.TestCase):
    def manifest(self) -> dict:
        return {
            "name": "run1",
            "created_at": "2026-09-29T15:00:00+00:00",
            "git_rev": "a" * 40,
            "config": {"tag": "release-x", "nodes": 5},
            "fleet": "run1",
            "query_db": "colocated",
            "images": {"espresso-node": {"revision": "bd2ad6e1dc7abc"}},
        }

    def test_row_of_a_result(self):
        row = awsb.index_row(self.manifest(), "01-run", valid_result(), 0, {"usd": 2.4})
        self.assertEqual(
            row,
            "| run1/01-run | 2026-09-29T15:00 | aaaaaaaaaa | release-x@bd2ad6e1dc | 5 "
            "| colocated | 8 | 8 | yes | 120 | valid | 0 | 2.40 |\n",
        )

    def test_row_without_result_or_cost(self):
        row = awsb.index_row(self.manifest(), "01-run", None, 3, None)
        self.assertIn("| colocated | - | - | - | - | failed | 3 | - |", row)

    def test_append_writes_the_header_once(self):
        with tempfile.TemporaryDirectory() as tmp:
            for _ in range(2):
                awsb.append_index(
                    Path(tmp), awsb.index_row(self.manifest(), "01-run", None, 3, None)
                )
            lines = (Path(tmp) / "INDEX.md").read_text().splitlines()
        self.assertEqual(len(lines), 4)
        self.assertTrue(lines[0].startswith("| fleet/run |"))


class AgentDriveTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp)
        topo = {
            "nodes": {"node0": "http://10.0.0.2:8080"},
            "roles": {"node0": "validator"},
            "query_node": "node0",
        }
        nb_cfg = dataclasses.asdict(netbench.BenchConfig())
        (self.tmp / "agent.json").write_text(
            json.dumps({"cfg": nb_cfg, "topology": topo, "ready_timeout_s": 5})
        )
        self.args = awsb.parse_args(
            ["agent-drive", str(self.tmp / "agent.json"), str(self.tmp / "out")]
        )
        handlers = list(logging.getLogger().handlers)
        self.addCleanup(
            lambda: [
                logging.getLogger().removeHandler(h)
                for h in logging.getLogger().handlers
                if h not in handlers
            ]
        )
        self.addCleanup(
            lambda: [
                netbench.log.removeHandler(h)
                for h in list(netbench.log.handlers)
                if isinstance(h, awsb.AgentReporter)
            ]
        )

    def state(self) -> dict:
        return json.loads((self.tmp / "out" / "agent-state.json").read_text())

    def run_agent(self, ready, load, sampler=None, clock=None) -> int:
        with (
            unittest.mock.patch.object(netbench, "wait_ready", ready),
            unittest.mock.patch.object(netbench, "drive_load", load),
            unittest.mock.patch.object(
                netbench, "sample_metrics", sampler or unittest.mock.Mock()
            ),
        ):
            return awsb.cmd_agent_drive(
                self.args, FakeSystem(clock=FakeClock() if clock is None else clock)
            )

    def test_the_clock_reaches_sampler_readiness_and_load(self):
        clock = FakeClock()
        clocks = []
        self.run_agent(
            lambda *a: clocks.append(a[-1]) or 1.0,
            lambda *a: clocks.append(a[-1]) or (1.0, 2.0),
            lambda *a, **k: clocks.append(a[-1]),
            clock,
        )
        self.assertEqual(clocks, [clock, clock, clock])

    def test_done_state_carries_ready_and_window(self):
        code = self.run_agent(lambda *a: 12.0, lambda *a: (100.0, 200.0))
        self.assertEqual(code, awsb.EXIT_OK)
        state = self.state()
        self.assertEqual(state["phase"], "done")
        self.assertEqual(
            (state["ready_s"], state["t0"], state["t1"]), (12.0, 100.0, 200.0)
        )

    def test_progress_follows_netbench_log(self):
        def load(*args):
            netbench.log.info("height 7: 3 submitted")
            return (1.0, 2.0)

        self.run_agent(lambda *a: 1.0, load)
        self.assertEqual(self.state()["progress"], "height 7: 3 submitted")

    def test_not_ready_is_an_error_state(self):
        def not_ready(*args):
            raise netbench.NetworkError("network not ready after 5 s: heights {}")

        code = self.run_agent(not_ready, lambda *a: (1.0, 2.0))
        self.assertEqual(code, awsb.EXIT_INVALID)
        state = self.state()
        self.assertEqual(state["phase"], "error")
        self.assertIn("network not ready", state["error"])

    def test_sigterm_during_load_is_reported_as_interrupted(self):
        def load(*args):
            raise KeyboardInterrupt

        code = self.run_agent(lambda *a: 1.0, load)
        self.assertEqual(code, awsb.EXIT_INVALID)
        self.assertEqual(self.state()["error"], "interrupted")


class RenderHostFilesTest(unittest.TestCase):
    def test_writes_env_start_topology_and_agent_config(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        manifest = {"hosts": hosts, "images": fake_images()}
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp)
            for host in hosts:
                (run_dir / "hosts" / host["name"]).mkdir(parents=True)
            awsb.render_host_files(run_dir, cfg, manifest, two_node_hosts_info())
            self.assertTrue((run_dir / "hosts/ctl/ctl.env").exists())
            self.assertTrue((run_dir / "hosts/node0/node.env").exists())
            self.assertTrue((run_dir / "hosts/node1/start.sh").exists())
            topo = json.loads((run_dir / "topology.json").read_text())
            self.assertEqual(topo["nodes"]["node1"], "http://10.0.0.3:8080")
            self.assertEqual(topo["query_node"], "node0")
            agent = json.loads((run_dir / "hosts/ctl/agent.json").read_text())
            self.assertEqual(agent["topology"], topo)
            self.assertEqual(agent["cfg"]["submit_nodes"], 1)
            config = awsb.load_config(agent["cfg"])
            self.assertEqual(config, cfg.load)

    def test_parse_hosts_output_takes_role_from_the_spec(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        output = {
            "hosts": {
                "value": {
                    h: {**two_node_hosts_info()[h], "role": "x"}
                    for h in two_node_hosts_info()
                }
            }
        }
        parsed = awsb.parse_hosts_output(output, awsb.plan_hosts(cfg))
        self.assertEqual(parsed["node0"]["role"], "query")
        self.assertEqual(parsed["ctl"]["private_ip"], "10.0.0.1")

    def test_genesis_contracts_are_deduplicated(self):
        with tempfile.TemporaryDirectory() as tmp:
            genesis = Path(__file__).with_name("genesis.toml").read_text()
            (Path(tmp) / "genesis.toml").write_text(genesis)
            self.assertEqual(
                awsb.genesis_contracts(Path(tmp)),
                [
                    "0x8ce361602b935680e8dec218b820ff5056beb7af",
                    "0xf7cd8fa9b94db2aa972023b379c7f72c65e4de9d",
                ],
            )


def balance_file(byte: list[float], io: list[float], first: float = 60.0) -> dict:
    """`cloudwatch/ec2-node0.json` with one period per item from `first`."""
    times = [first + 60.0 * i for i in range(len(byte))]
    return {
        "instance_id": "i-1",
        "period_s": 60,
        "start": 40.0,
        "end": 220.0,
        "metrics": {
            "EBSByteBalance%": {"timestamps": times, "values": byte},
            "EBSIOBalance%": {"timestamps": times, "values": io},
        },
    }


def write_collected_run(run_dir: Path) -> dict:
    """A 3-node run dir as `cmd_run` leaves it after collection: netbench's synthetic run plus
    manifest, config, topology and `hosts/<name>/` files."""
    test_netbench.write_run_dir(run_dir)
    cfg = awsb.RunConfig(tag="x", nodes=3, load=netbench.BenchConfig(submit_nodes=2))
    hosts = awsb.plan_hosts(cfg)
    manifest = {
        "name": "run1",
        "fleet": "run1",
        "query_db": "colocated",
        "config": awsb.config_to_json(cfg),
        "hosts": hosts,
        "images": fake_images(),
        "az": "eu-west-1b",
        "ami_id": "ami-0abc",
        "start_spread_s": 0.3,
        "cost_usd": {"expected": 1.0, "bound": 2.0},
    }
    netbench.write_json(run_dir / "manifest.json", manifest)
    netbench.write_json(run_dir / "config.json", dataclasses.asdict(cfg.load))
    netbench.write_json(run_dir / "topology.json", test_netbench.TOPOLOGY)
    tracking = "System time     : 0.000010000 seconds fast of NTP time\n"
    for host in hosts:
        host_dir = run_dir / "hosts" / host["name"]
        host_dir.mkdir(parents=True)
        samples = (
            {
                "ts": float(ts),
                "cpu": [50 * ts, 0, 0, 50 * ts, 0, 0, 0, 0, 0, 0],
                "mem_avail": 1000,
                "procs": {},
                "disk": {"nvme0n1": {"read_bytes": 0, "write_bytes": ts * 1_000_000}},
                "net": {"ens5": {"rx_bytes": ts * 500_000, "tx_bytes": ts * 500_000}},
            }
            for ts in range(90, 172, 2)
        )
        netbench.write_jsonl(host_dir / "host.jsonl", samples)
        (host_dir / "du-journal.txt").write_text("1000000\t/data/journal\n")
        (host_dir / "chrony.txt").write_text(tracking)
        digests = {n: f"{i['ref']}@{i['digest']}" for n, i in fake_images().items()}
        netbench.write_json(
            host_dir / "ready.json",
            {"digests": digests, "chronyc_tracking": tracking},
        )
    (run_dir / "cloudwatch").mkdir()
    (run_dir / awsb.EC2_NODE0_FILE).write_text(
        json.dumps(balance_file([100.0, 100.0, 100.0], [100.0, 100.0, 100.0]))
    )
    node0 = run_dir / "hosts" / "node0"
    (node0 / "lscpu.txt").write_text("CPU(s): 16\nModel name: Neoverse-V2\nFlags: fp\n")
    (node0 / "meminfo.txt").write_text("MemTotal:       32000000 kB\n")
    (node0 / "uname.txt").write_text("6.8.0-aws\n")
    return manifest


class CollectEbsBalanceTest(unittest.TestCase):
    def collect(self, runner, t0=100.0, t1=160.0) -> Path:
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        manifest = {"hosts_info": two_node_hosts_info()}
        awsb.collect_ebs_balance(runner, manifest, t0, t1, tmp)
        return tmp

    def call(self, runner) -> list[str]:
        return next(c for c in runner.calls if "get-metric-data" in c)

    def test_queries_node0_over_the_padded_window_and_saves_the_series(self):
        runner = FleetRunner([DONE_STATE], balance=completed(stdout=FULL_BALANCE))
        out = self.collect(runner)
        argv = self.call(runner)
        self.assertEqual(
            argv[:5],
            ["aws", "--profile", "timeboost-dev", "cloudwatch", "get-metric-data"],
        )
        self.assertEqual(argv[argv.index("--region") + 1], "eu-west-1")
        self.assertEqual(
            argv[argv.index("--start-time") + 1], "1970-01-01T00:00:40+00:00"
        )
        self.assertEqual(
            argv[argv.index("--end-time") + 1], "1970-01-01T00:03:40+00:00"
        )
        queries = json.loads(argv[argv.index("--metric-data-queries") + 1])
        self.assertEqual(
            [q["MetricStat"]["Metric"]["MetricName"] for q in queries],
            ["EBSByteBalance%", "EBSIOBalance%"],
        )
        for query in queries:
            self.assertEqual(query["MetricStat"]["Metric"]["Namespace"], "AWS/EC2")
            self.assertEqual(
                query["MetricStat"]["Metric"]["Dimensions"],
                [{"Name": "InstanceId", "Value": "i-000000000002"}],
            )
            self.assertEqual(query["MetricStat"]["Period"], 60)
        saved = json.loads((out / awsb.EC2_NODE0_FILE).read_text())
        self.assertEqual(saved["instance_id"], "i-000000000002")
        self.assertEqual(
            saved["metrics"]["EBSByteBalance%"],
            {"timestamps": [60.0, 120.0, 180.0], "values": [100.0, 100.0, 100.0]},
        )

    def test_metric_without_datapoints_is_saved_empty(self):
        data = metric_data([100.0], [None])
        out = self.collect(FleetRunner([DONE_STATE], balance=completed(stdout=data)))
        saved = json.loads((out / awsb.EC2_NODE0_FILE).read_text())
        self.assertEqual(saved["metrics"]["EBSIOBalance%"]["values"], [])

    def test_status_other_than_complete_raises(self):
        data = json.loads(FULL_BALANCE)
        data["MetricDataResults"][0]["StatusCode"] = "Forbidden"
        runner = FleetRunner([DONE_STATE], balance=completed(stdout=json.dumps(data)))
        with self.assertRaisesRegex(awsb.RemoteError, "EBSByteBalance%: Forbidden"):
            self.collect(runner)

    def test_failed_aws_call_raises(self):
        runner = FleetRunner(
            [DONE_STATE], balance=completed(returncode=254, stderr="denied")
        )
        with self.assertRaisesRegex(awsb.Refused, "denied"):
            self.collect(runner)

    def test_min_counts_only_periods_overlapping_the_load(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "ec2.json"
            path.write_text(
                json.dumps(balance_file([50.0, 97.0, 99.0, 30.0], [100.0] * 4, 0.0))
            )
            # Periods start at 0, 60, 120, 180; the load is 100..160: periods 60 and 120.
            low = awsb.metric_min(path, 100.0, 160.0)
        self.assertEqual(low, {"EBSByteBalance%": 97.0, "EBSIOBalance%": 100.0})


class FinishCollectsEbsBalanceTest(unittest.TestCase):
    def run_flow(self, runner, harness):
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result()
        ):
            return harness.run(runner)

    def test_valid_run_saves_node0_balance(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        self.assertEqual(self.run_flow(runner, harness), awsb.EXIT_OK)
        self.assertEqual(runner.count("get-metric-data"), 1)
        self.assertTrue((harness.run_dir / awsb.EC2_NODE0_FILE).exists())

    def test_balance_fetch_failure_does_not_stop_the_teardown(self):
        harness = RunHarness(self)
        runner = FleetRunner(
            [DONE_STATE],
            describe=DESCRIBE,
            balance=completed(returncode=254, stderr="denied"),
        )
        self.assertEqual(self.run_flow(runner, harness), awsb.EXIT_OK)
        self.assertFalse((harness.run_dir / awsb.EC2_NODE0_FILE).exists())
        self.assertTrue(runner.ran("tofu", "destroy"))

    def test_no_load_window_fetches_nothing(self):
        harness = RunHarness(self)
        error = {"phase": "error", "detail": "x", "error": "network not ready"}
        runner = FleetRunner([error], describe=DESCRIBE)
        self.assertEqual(harness.run(runner), awsb.EXIT_FAILED)
        self.assertEqual(runner.count("get-metric-data"), 0)

    def test_waits_for_the_lag_and_skips_on_third_signal(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE])
        system = FakeSystem(run=runner)
        interrupts = awsb.Interrupts(system.clock)
        interrupts.skip_collect.set()
        fleet = awsb.FleetState(
            system,
            awsb.RunConfig(tag="x"),
            harness.fleet_dir,
            None,
            interrupts,
        )
        agent = DONE_STATE | {"t1": system.clock.time()}
        run = awsb.Run(fleet, harness.run_dir, fleet.cfg, 0.0, agent=agent)
        awsb.collect_node0_ebs_balance(run)
        self.assertEqual(runner.count("get-metric-data"), 0)

    def test_published_window_waits_out_the_lag_on_the_clock(self):
        harness = RunHarness(self)
        clock = FakeClock()
        fleet = awsb.FleetState(
            FakeSystem(run=FleetRunner([DONE_STATE]), clock=clock),
            awsb.RunConfig(tag="x"),
            harness.fleet_dir,
            None,
            awsb.Interrupts(clock),
        )
        agent = DONE_STATE | {"t1": clock.time()}
        run = awsb.Run(fleet, harness.run_dir, fleet.cfg, 0.0, agent=agent)
        self.assertEqual(awsb.published_window(run, "x"), agent)
        self.assertEqual(clock.sleeps, [awsb.CLOUDWATCH_LAG_S])


class WriteReportTest(unittest.TestCase):
    def report(self, mutate=None) -> tuple[Path, dict]:
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        write_collected_run(tmp)
        if mutate:
            mutate(tmp)
        return tmp, awsb.write_report(tmp)

    def test_result_has_hosts_deployment_and_summary_blocks(self):
        run_dir, result = self.report()
        self.assertEqual(sorted(result["hosts"]), ["ctl", "node0", "node1", "node2"])
        self.assertAlmostEqual(result["hosts"]["node0"]["disk_mb_s"], 1.0)
        self.assertAlmostEqual(result["hosts"]["node0"]["net_mb_s"], 1.0)
        deployment = result["deployment"]
        self.assertEqual(deployment["az"], "eu-west-1b")
        self.assertAlmostEqual(deployment["clock_offset_ms_max"], 0.01)
        self.assertEqual(deployment["cost_usd"], {"expected": 1.0, "bound": 2.0})
        self.assertEqual(result["runner"]["cpu_model"], "Neoverse-V2")
        saved = json.loads((run_dir / "result.json").read_text())
        self.assertEqual(saved["deployment"], deployment)
        summary = (run_dir / "summary.md").read_text()
        self.assertIn("Deployment", summary)
        self.assertIn("| node1 |", summary)

    def test_uncollected_host_makes_the_run_invalid(self):
        _, result = self.report(
            lambda d: (d / "hosts" / "node2" / "host.jsonl").unlink()
        )
        self.assertNotIn("node2", result["hosts"])
        self.assertIn(
            "node2 host samples cover only 0% of the window",
            result["validity"]["reasons"],
        )
        self.assertFalse(result["validity"]["valid"])

    # TEST:ebs-balance-noisy-ok
    def test_ebs_balance_drop_inside_the_load_makes_the_run_noisy(self):
        def drop(d: Path) -> None:
            data = balance_file([100.0, 91.0, 100.0], [100.0, 100.0, 100.0])
            (d / awsb.EC2_NODE0_FILE).write_text(json.dumps(data))

        _, result = self.report(drop)
        self.assertTrue(result["validity"]["valid"])
        self.assertTrue(result["validity"]["noisy"])
        self.assertIn(
            "node0 EBSByteBalance% fell to 91% during the load",
            result["validity"]["reasons"],
        )

    def test_ebs_balance_drop_after_the_load_is_ignored(self):
        def drop(d: Path) -> None:
            data = balance_file([100.0, 100.0, 40.0], [100.0, 100.0, 40.0])
            (d / awsb.EC2_NODE0_FILE).write_text(json.dumps(data))

        _, result = self.report(drop)
        self.assertFalse(result["validity"]["noisy"])

    # TEST:cloudwatch-datapoint-missing-noisy
    def test_ebs_balance_without_datapoints_in_the_window_is_noisy(self):
        def empty(d: Path) -> None:
            data = balance_file([100.0, 100.0, 100.0], [], first=60.0)
            data["metrics"]["EBSIOBalance%"] = {"timestamps": [], "values": []}
            (d / awsb.EC2_NODE0_FILE).write_text(json.dumps(data))

        _, result = self.report(empty)
        self.assertTrue(result["validity"]["valid"])
        self.assertIn(
            "node0 EBSIOBalance% has no datapoint in the load window",
            result["validity"]["reasons"],
        )

    def test_uncollected_ebs_balance_makes_the_run_noisy(self):
        _, result = self.report(lambda d: (d / awsb.EC2_NODE0_FILE).unlink())
        self.assertTrue(result["validity"]["valid"])
        self.assertIn("node0 EBS balance not collected", result["validity"]["reasons"])

    def test_render_is_repeatable(self):
        run_dir, first = self.report()
        self.assertEqual(awsb.write_report(run_dir), first)


NOW = datetime.fromtimestamp(FAKE_EPOCH, UTC)
EXPIRES_LATER = "2026-09-29T17:00:00Z"
EXPIRES_PAST = "2026-09-29T15:00:00Z"


def arn(kind: str, resource_id: str) -> str:
    return f"arn:aws:ec2:eu-west-1:1:{kind}/{resource_id}"


def tag_mapping(
    kind: str, resource_id: str, run: str, owner: str | None, expires: str | None
) -> dict:
    tags = {awsb.TAG_RUN: run}
    if owner:
        tags[awsb.TAG_OWNER] = owner
    if expires:
        tags[awsb.TAG_EXPIRES] = expires
    return {
        "arn": arn(kind, resource_id),
        "tags": [{"Key": k, "Value": v} for k, v in tags.items()],
    }


def instance(resource_id: str, state: str = "running", launch: str | None = None):
    return {
        "id": resource_id,
        "type": "c8g.4xlarge",
        "state": state,
        "launch": launch or "2026-09-29T15:30:00+00:00",
    }


class GroupRunsTest(unittest.TestCase):
    def test_groups_by_run_sorted_by_owner_and_drops_terminated(self):
        mappings = [
            tag_mapping("instance", "i-2", "zed", "alice", EXPIRES_LATER),
            tag_mapping("instance", "i-dead", "zed", "alice", EXPIRES_LATER),
            tag_mapping("volume", "vol-1", "zed", None, None),
            tag_mapping("security-group", "sg-1", "zed", "alice", EXPIRES_LATER),
            tag_mapping("instance", "i-1", "amy", "bob", EXPIRES_LATER),
            tag_mapping("volume", "vol-2", "lost", None, None),
        ]
        instances = [instance("i-1"), instance("i-2"), instance("i-dead", "terminated")]
        runs = awsb.group_runs(mappings, instances, ["vol-1", "vol-2"])
        self.assertEqual([r["name"] for r in runs], ["lost", "zed", "amy"])
        zed = runs[1]
        self.assertEqual(zed["owner"], "alice")
        self.assertEqual(zed["expires"], EXPIRES_LATER)
        self.assertEqual([i["id"] for i in zed["instances"]], ["i-2"])
        self.assertEqual(
            zed["arns"],
            [
                arn("instance", "i-2"),
                arn("volume", "vol-1"),
                arn("security-group", "sg-1"),
            ],
        )
        self.assertIsNone(runs[0]["owner"])
        self.assertEqual(runs[0]["instances"], [])

    def test_nothing_tagged_gives_no_runs(self):
        self.assertEqual(awsb.group_runs([], [], []), [])

    def test_drops_volumes_the_tag_api_lists_after_their_deletion(self):
        mappings = [
            tag_mapping("instance", "i-1", "amy", "bob", EXPIRES_LATER),
            tag_mapping("volume", "vol-live", "amy", None, None),
            tag_mapping("volume", "vol-gone", "amy", None, None),
            tag_mapping("volume", "vol-orphan", "old", None, None),
        ]
        runs = awsb.group_runs(mappings, [instance("i-1")], ["vol-live"])
        (amy,) = runs
        self.assertEqual(
            amy["arns"], [arn("instance", "i-1"), arn("volume", "vol-live")]
        )

    def test_arn_parts(self):
        self.assertEqual(
            awsb.arn_parts(arn("instance", "i-0abc")), ("instance", "i-0abc")
        )


class ClassifyOrphanTest(unittest.TestCase):
    def tagged(self, owner: str | None = "me", expires: str | None = EXPIRES_LATER):
        return {"name": "r", "owner": owner, "expires": expires}

    def classify(self, phase, **kwargs) -> str | None:
        return awsb.classify_orphan(self.tagged(**kwargs), phase, NOW, "me")

    def test_live_run_with_state_is_not_an_orphan(self):
        self.assertIsNone(self.classify("measuring"))
        self.assertIsNone(self.classify("left-running"))

    def test_past_expiry(self):
        self.assertEqual(
            self.classify("measuring", expires=EXPIRES_PAST), "past expiry"
        )
        self.assertEqual(
            self.classify(None, owner="bob", expires=EXPIRES_PAST), "past expiry"
        )

    def test_finished_run_with_leftovers(self):
        self.assertEqual(self.classify("done"), "run is done")
        self.assertEqual(self.classify("swept"), "run is swept")

    def test_lost_state_counts_only_for_the_current_user(self):
        self.assertEqual(self.classify(None), "no local state")
        self.assertIsNone(self.classify(None, owner="bob"))

    def test_untagged_expiry_never_expires(self):
        self.assertIsNone(self.classify("measuring", expires=None))


class LocalPhaseTest(unittest.TestCase):
    def test_no_manifest_is_no_state(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertIsNone(awsb.local_phase(Path(tmp), "r"))

    def test_live_run_without_tfstate_keeps_its_phase(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "r").mkdir()
            netbench.write_json(out / "r" / "fleet.json", {"phase": "applying"})
            self.assertEqual(awsb.local_phase(out, "r"), "applying")

    def test_terminal_phase_needs_terraform_state(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "r" / "terraform").mkdir(parents=True)
            netbench.write_json(out / "r" / "fleet.json", {"phase": "done"})
            self.assertIsNone(awsb.local_phase(out, "r"))
            (out / "r" / "terraform" / "terraform.tfstate").write_text("{}")
            self.assertEqual(awsb.local_phase(out, "r"), "done")


class FormatRunsTest(unittest.TestCase):
    def test_accrued_usd_is_instance_hours_at_the_price(self):
        two = [instance("i-1"), instance("i-2", launch="2026-09-29T15:00:00+00:00")]
        usd = awsb.accrued_usd(two, NOW)
        self.assertAlmostEqual(usd, 1.5 * awsb.PRICES["c8g.4xlarge"])

    def test_accrued_usd_of_an_unpriced_type_raises(self):
        with self.assertRaises(KeyError):
            awsb.accrued_usd([{**instance("i-1"), "type": "t4g.nano"}], NOW)

    def test_table_row_shows_owner_cost_and_orphan(self):
        runs = awsb.group_runs(
            [
                tag_mapping("instance", "i-1", "amy", "bob", EXPIRES_PAST),
                tag_mapping("key-pair", "key-1", "amy", "bob", EXPIRES_PAST),
            ],
            [instance("i-1")],
            [],
        )
        runs[0]["orphan"] = "past expiry"
        lines = awsb.format_runs(runs, NOW)
        self.assertEqual(
            lines[2],
            "| amy | bob | 2026-09-29T15:30 | 2026-09-29T15:00 | 1 instance, 1 key-pair "
            "| 0.34 | past expiry |",
        )


class TagRunner(FakeRunner):
    """Answers the tag API with `{arn, tags}` mappings, or with bare ARNs for a query that
    asks for `ResourceARN` only (`tagged_resources`)."""

    mappings: list[dict]
    # Volume ids the tag API still lists although EC2 no longer has them.
    stale_volumes: frozenset[str] = frozenset()

    def __call__(
        self, argv: list[str], env: dict[str, str] | None = None
    ) -> subprocess.CompletedProcess:
        if "describe-volumes" in argv:
            self.calls.append(argv)
            ids = [
                resource_id
                for kind, resource_id in (
                    awsb.arn_parts(m["arn"]) for m in self.mappings
                )
                if kind == "volume" and resource_id not in self.stale_volumes
            ]
            return completed(stdout=json.dumps(ids))
        if "resourcegroupstaggingapi" in argv:
            self.calls.append(argv)
            arns = "ResourceTagMappingList[].ResourceARN" in argv
            body = [m["arn"] for m in self.mappings] if arns else self.mappings
            return completed(stdout=json.dumps(body))
        return super().__call__(argv)


def tag_runner(
    mappings: list[dict], instances: list[dict], stale_volumes: Iterable[str] = ()
) -> FakeRunner:
    runner = TagRunner(
        {
            STS_CALL: sts_response("027574771971"),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "describe-instances",
            ): completed(stdout=json.dumps(instances)),
            ("aws",): completed(),
        }
    )
    runner.mappings = mappings
    runner.stale_volumes = frozenset(stale_volumes)
    return runner


def at(moment: datetime) -> FakeClock:
    return FakeClock(start=moment.timestamp())


def run_cmd(func, argv: list[str], system: FakeSystem) -> tuple[int, str]:
    parsed = awsb.parse_args(argv)
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        code = func(parsed, system)
    return code, out.getvalue()


class StatusAllTest(unittest.TestCase):
    def setUp(self):
        self.out = isolated_env(self, temp_dir(self))

    def test_empty_region(self):
        code, out = run_cmd(
            awsb.cmd_status,
            ["status", "--all"],
            FakeSystem(run=tag_runner([], []), clock=at(NOW)),
        )
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertEqual(out, "no espresso-bench resources in eu-west-1\n")

    def test_lists_runs_with_orphan_marker(self):
        mappings = [
            tag_mapping("instance", "i-1", "amy", "bob", EXPIRES_PAST),
            tag_mapping("instance", "i-2", "ok", "bob", EXPIRES_LATER),
        ]
        runner = tag_runner(mappings, [instance("i-1"), instance("i-2")])
        code, out = run_cmd(
            awsb.cmd_status, ["status", "--all"], FakeSystem(run=runner, clock=at(NOW))
        )
        self.assertEqual(code, awsb.EXIT_OK)
        lines = out.splitlines()
        self.assertIn("| amy | bob |", lines[2])
        self.assertTrue(lines[2].endswith("| past expiry |"))
        self.assertTrue(lines[3].endswith("| 0.34 |  |"), lines[3])

    def test_a_finished_fleet_whose_volumes_only_the_tag_api_lists_is_not_shown(self):
        mappings = [
            tag_mapping("instance", "i-1", "done", "bob", EXPIRES_LATER),
            tag_mapping("volume", "vol-1", "done", None, None),
            tag_mapping("volume", "vol-2", "done", None, None),
        ]
        runner = tag_runner(
            mappings, [instance("i-1", "terminated")], stale_volumes=["vol-1", "vol-2"]
        )
        code, out = run_cmd(
            awsb.cmd_status, ["status", "--all"], FakeSystem(run=runner, clock=at(NOW))
        )
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertEqual(out, "no espresso-bench resources in eu-west-1\n")

    def test_wrong_account_is_refused_before_listing(self):
        runner = FakeRunner({STS_CALL: sts_response("999")})
        with self.assertRaises(awsb.Refused):
            run_cmd(
                awsb.cmd_status,
                ["status", "--all"],
                FakeSystem(run=runner, clock=at(NOW)),
            )
        self.assertEqual(len(runner.calls), 1)

    def test_needs_dir_or_all(self):
        with self.assertRaises(awsb.Refused):
            run_cmd(awsb.cmd_status, ["status"], FakeSystem(clock=at(NOW)))


class DestroyOrphansTest(unittest.TestCase):
    def setUp(self):
        self.out = isolated_env(self, temp_dir(self))

    def runner(self) -> FakeRunner:
        mappings = [
            tag_mapping("instance", "i-1", "amy", "bob", EXPIRES_PAST),
            tag_mapping("volume", "vol-1", "amy", None, None),
            tag_mapping("security-group", "sg-1", "amy", "bob", EXPIRES_PAST),
            tag_mapping("key-pair", "key-1", "amy", "bob", EXPIRES_PAST),
            tag_mapping("instance", "i-2", "live", "bob", EXPIRES_LATER),
        ]
        return tag_runner(mappings, [instance("i-1"), instance("i-2")])

    def destroy(self, runner, *flags: str) -> tuple[int, str]:
        return run_cmd(
            awsb.cmd_destroy,
            ["destroy", "--orphans", *flags],
            FakeSystem(run=runner, clock=at(NOW)),
        )

    def test_sweeps_only_orphans_in_dependency_order(self):
        runner = self.runner()
        code, out = self.destroy(runner, "--yes")
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertIn("| amy | bob |", out)
        self.assertNotIn("| live |", out)
        ec2 = [
            c[4]
            for c in runner.calls
            if c[3:4] == ["ec2"] and not c[4].startswith("describe-")
        ]
        self.assertEqual(
            ec2,
            [
                "terminate-instances",
                "wait",
                "delete-volume",
                "delete-security-group",
                "delete-key-pair",
            ],
        )
        self.assertFalse(
            runner.ran(
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "terminate-instances",
                "--instance-ids",
                "i-2",
            )
        )

    def test_declined_prompt_deletes_nothing(self):
        runner = self.runner()
        with self.assertRaisesRegex(awsb.Refused, "not confirmed"):
            self.destroy(runner)
        self.assertFalse(any("terminate-instances" in c for c in runner.calls))

    def test_nothing_to_sweep(self):
        code, out = self.destroy(tag_runner([], []), "--yes")
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertIn("no orphaned", out)

    def test_failed_sweep_exits_4(self):
        runner = self.runner()
        runner.responses = {
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "terminate-instances",
            ): completed(returncode=1, stderr="denied"),
            **runner.responses,
        }
        code, _ = self.destroy(runner, "--yes")
        self.assertEqual(code, awsb.EXIT_LEFTOVER)

    def test_marks_local_state_swept(self):
        out = self.out
        (out / "amy").mkdir(parents=True)
        netbench.write_json(out / "amy" / "fleet.json", {"phase": "left-running"})
        key = out / "amy" / "ssh" / "id_ed25519"
        key.parent.mkdir()
        key.write_text("private")
        parsed = awsb.parse_args(["destroy", "--orphans", "--yes"])
        with contextlib.redirect_stdout(io.StringIO()):
            awsb.cmd_destroy(
                parsed,
                FakeSystem(run=self.runner(), clock=at(NOW)),
            )
        manifest = json.loads((out / "amy" / "fleet.json").read_text())
        self.assertFalse(key.exists())
        self.assertEqual(manifest["phase"], "swept")


class SweepVolumeTest(unittest.TestCase):
    def test_volume_between_instances_and_security_group(self):
        runner = tag_runner(
            [
                tag_mapping("key-pair", "key-1", "r", None, None),
                tag_mapping("security-group", "sg-1", "r", None, None),
                tag_mapping("volume", "vol-1", "r", None, None),
                tag_mapping("instance", "i-1", "r", None, None),
            ],
            [],
        )
        awsb.sweep(FakeSystem(run=runner), "r")
        ec2 = [c[4] for c in runner.calls if c[3:4] == ["ec2"]]
        self.assertEqual(
            ec2,
            [
                "terminate-instances",
                "wait",
                "delete-volume",
                "delete-security-group",
                "delete-key-pair",
            ],
        )

    def test_already_deleted_volume_is_tolerated(self):
        runner = tag_runner([tag_mapping("volume", "vol-1", "r", None, None)], [])
        runner.responses = {
            ("aws", "--profile", "timeboost-dev", "ec2", "delete-volume"): completed(
                returncode=255, stderr="InvalidVolume.NotFound: gone"
            ),
            **runner.responses,
        }
        awsb.sweep(FakeSystem(run=runner), "r")

    def test_other_volume_errors_raise(self):
        runner = tag_runner([tag_mapping("volume", "vol-1", "r", None, None)], [])
        runner.responses = {
            ("aws", "--profile", "timeboost-dev", "ec2", "delete-volume"): completed(
                returncode=255, stderr="VolumeInUse"
            ),
            **runner.responses,
        }
        with self.assertRaisesRegex(awsb.Refused, "VolumeInUse"):
            awsb.sweep(FakeSystem(run=runner), "r")


class ManifestCostTest(unittest.TestCase):
    def manifest(self) -> dict:
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        estimate = shot_estimate(hosts, cfg)
        return {"hosts": hosts, "estimate": estimate}

    def test_is_linear_in_duration_and_covers_instance_hours(self):
        manifest = self.manifest()
        hour = awsb.manifest_cost(manifest, 3600) - awsb.manifest_cost(manifest, 0)
        self.assertAlmostEqual(
            awsb.manifest_cost(manifest, 7200) - awsb.manifest_cost(manifest, 3600),
            hour,
        )
        self.assertGreaterEqual(
            hour, awsb.PRICES["c8g.2xlarge"] + 2 * awsb.PRICES["c8g.4xlarge"]
        )


class IndexKeepsRowsTest(unittest.TestCase):
    def test_append_keeps_existing_rows(self):
        manifest = IndexTest.manifest(IndexTest())
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "INDEX.md"
            path.write_text(awsb.INDEX_HEADER + "| older | row |\n")
            awsb.append_index(
                Path(tmp), awsb.index_row(manifest, "01-run", None, 4, None)
            )
            awsb.append_index(
                Path(tmp), awsb.index_row(manifest, "02-run", None, 0, {"usd": 1.0})
            )
            lines = path.read_text().splitlines()
        self.assertEqual(lines[2], "| older | row |")
        self.assertEqual(len(lines), 5)
        self.assertEqual(sum(l.startswith("| fleet/run |") for l in lines), 1)
        self.assertTrue(lines[3].endswith("| failed | 4 | - |"))
        self.assertTrue(lines[4].endswith("| failed | 0 | 1.00 |"))


STATUS_DESCRIBE = json.dumps(
    [
        {
            **instance("i-000000000001", launch="2026-09-29T15:00:00+00:00"),
            "reason": "",
        },
        {
            **instance("i-000000000002", launch="2026-09-29T15:00:01+00:00"),
            "reason": "",
        },
        {
            **instance("i-000000000003", launch="2026-09-29T15:00:02+00:00"),
            "reason": "",
        },
    ]
)


class KeptRunTest(unittest.TestCase):
    """`status`, `collect` and `down DIR` on the dirs of a run whose destroy failed."""

    def kept(self, describe: str = STATUS_DESCRIBE, states=None):
        harness = RunHarness(self)
        runner = FleetRunner(
            states or [DONE_STATE],
            describe=describe,
            destroys=[completed(returncode=1, stderr="locked")],
        )
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result()
        ):
            self.assertEqual(harness.run(runner), awsb.EXIT_LEFTOVER)
        runner.destroys = [completed()]
        (harness.fleet_dir / "terraform").mkdir(exist_ok=True)
        return harness, runner

    def args(self, harness, verb: str, *extra: str):
        target = harness.run_dir if verb == "collect" else harness.fleet_dir
        return awsb.parse_args([verb, str(target), *extra])

    def test_status_shows_phase_instances_agent_and_cost(self):
        harness, runner = self.kept()
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = awsb.cmd_status(
                self.args(harness, "status"),
                FakeSystem(run=runner, clock=at(NOW)),
            )
        self.assertEqual(code, awsb.EXIT_OK)
        text = out.getvalue()
        self.assertIn("- fleet run1: phase left-running", text)
        self.assertIn("- 01-run: phase left-running", text)
        self.assertIn("- ctl: running, c8g.4xlarge, launched 2026-09-29T15:00", text)
        self.assertIn("- agent: done, load finished", text)
        self.assertRegex(text, r"- cost: \$\d+\.\d\d so far, bound \$\d+\.\d\d")

    def test_status_of_a_destroyed_fleet_shows_actual_cost(self):
        harness, runner = self.kept()
        terminated = json.dumps(
            [
                {
                    **instance("i-000000000001", "terminated"),
                    "reason": "User initiated (2026-09-29 15:30:00 GMT)",
                }
            ]
        )
        runner.describe = terminated
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            awsb.cmd_status(
                self.args(harness, "status"),
                FakeSystem(run=runner, clock=at(NOW)),
            )
        self.assertRegex(out.getvalue(), r"- cost: \$\d+\.\d\d actual, bound")
        self.assertNotIn("agent", out.getvalue())

    def test_status_of_a_finished_run_reads_cost_json_without_aws(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result()
        ):
            self.assertEqual(harness.run(runner), awsb.EXIT_OK)
        offline = FleetRunner([DONE_STATE])
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = awsb.cmd_status(
                self.args(harness, "status"),
                FakeSystem(run=offline, clock=at(NOW)),
            )
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertEqual(offline.calls, [])
        self.assertRegex(out.getvalue(), r"- cost: \$\d+\.\d\d actual, bound \$\d+")

    def test_status_of_a_planned_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp) / "p"
            run_dir.mkdir()
            netbench.write_json(
                run_dir / "fleet.json",
                {
                    "name": "p",
                    "phase": "planned",
                    "created_at": "2026-09-29T15:00:00+00:00",
                },
            )
            runner = FakeRunner()
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                awsb.cmd_status(
                    awsb.parse_args(["status", str(run_dir)]),
                    FakeSystem(run=runner, clock=at(NOW)),
                )
        self.assertIn("planned only", out.getvalue())
        self.assertEqual(runner.calls, [])

    def test_down_destroys_and_prices(self):
        harness, runner = self.kept(describe=DESCRIBE)
        code = awsb.cmd_down(
            self.args(harness, "down", "--yes"),
            FakeSystem(run=runner, clock=FakeClock()),
        )
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertTrue(runner.ran("tofu", "destroy"))
        manifest = json.loads((harness.fleet_dir / "fleet.json").read_text())
        self.assertEqual(manifest["phase"], "done")
        self.assertGreater(manifest["cost_usd"]["actual"], 0)
        run_manifest = json.loads((harness.run_dir / "manifest.json").read_text())
        self.assertEqual(run_manifest["cost_usd"], manifest["cost_usd"])
        rows = harness.index().splitlines()
        self.assertEqual(len(rows), 3)
        self.assertIn("| valid | 4 |", rows[2])

    def test_down_removes_the_key(self):
        harness, runner = self.kept()
        key = harness.fleet_dir / "ssh" / "id_ed25519"
        self.assertTrue(key.exists())
        runner.describe = STATUS_DESCRIBE
        with contextlib.redirect_stdout(io.StringIO()):
            awsb.cmd_status(
                self.args(harness, "status"),
                FakeSystem(run=runner, clock=at(NOW)),
            )
        self.assertTrue(runner.ran("-i", str(key)))
        runner.describe = DESCRIBE
        awsb.cmd_down(
            self.args(harness, "down", "--yes"),
            FakeSystem(run=runner, clock=FakeClock()),
        )
        self.assertFalse(key.exists())
        self.assertTrue(key.with_name("id_ed25519.pub").exists())

    def test_down_twice_is_refused(self):
        harness, runner = self.kept(describe=DESCRIBE)
        awsb.cmd_down(
            self.args(harness, "down", "--yes"),
            FakeSystem(run=runner, clock=FakeClock()),
        )
        with self.assertRaisesRegex(awsb.Refused, "already done"):
            awsb.cmd_down(
                self.args(harness, "down", "--yes"),
                FakeSystem(run=runner, clock=FakeClock()),
            )

    def test_down_declined(self):
        harness, runner = self.kept()
        destroys = runner.count("tofu", "destroy")
        with self.assertRaisesRegex(awsb.Refused, "not confirmed"):
            awsb.cmd_down(
                self.args(harness, "down"), FakeSystem(run=runner, clock=FakeClock())
            )
        self.assertEqual(runner.count("tofu", "destroy"), destroys)

    def test_failed_destroy_dir_sweeps_and_exits_4(self):
        harness, runner = self.kept()
        runner.destroys = [completed(returncode=1, stderr="locked")]
        code = awsb.cmd_down(
            self.args(harness, "down", "--yes"),
            FakeSystem(run=runner, clock=FakeClock()),
        )
        self.assertEqual(code, awsb.EXIT_LEFTOVER)
        self.assertTrue(runner.ran("terminate-instances", "i-1"))
        manifest = json.loads((harness.fleet_dir / "fleet.json").read_text())
        self.assertEqual(manifest["phase"], "left-running")

    def test_collect_writes_numbered_subdirs(self):
        harness, runner = self.kept()
        for index in (1, 2):
            expected = harness.run_dir / "hosts" / "node0" / f"collect-{index}"
            with unittest.mock.patch.object(
                awsb.Remote,
                "rsync_from",
                autospec=True,
                side_effect=lambda self, host, remote, local, *flags: (
                    local.mkdir(parents=True, exist_ok=True),
                    (local / "df.txt").write_text("x"),
                ),
            ):
                code = awsb.cmd_collect(
                    self.args(harness, "collect"), FakeSystem(run=runner)
                )
            self.assertEqual(code, awsb.EXIT_OK)
            self.assertTrue((expected / "df.txt").exists())
        self.assertTrue(runner.ran("psql -At -c"))

    def test_collect_reports_hosts_that_yielded_nothing(self):
        harness, runner = self.kept()
        with unittest.mock.patch.object(awsb.Remote, "rsync_from", autospec=True):
            code = awsb.cmd_collect(
                self.args(harness, "collect"), FakeSystem(run=runner)
            )
        self.assertEqual(code, awsb.EXIT_FAILED)

    def test_collect_reports_a_host_whose_script_failed_and_copied_nothing(self):
        harness, runner = self.kept()
        real_ssh = awsb.Remote.ssh

        def failing(self, host, command, check=True):
            if "docker logs" in command:
                raise awsb.RemoteError(f"{host}: exited 255")
            return real_ssh(self, host, command, check)

        with (
            unittest.mock.patch.object(awsb.Remote, "rsync_from", autospec=True),
            unittest.mock.patch.object(awsb.Remote, "ssh", failing),
        ):
            code = awsb.cmd_collect(
                self.args(harness, "collect"), FakeSystem(run=runner)
            )
        self.assertEqual(code, awsb.EXIT_FAILED)

    def test_collect_of_an_older_run_is_refused(self):
        harness, runner = self.kept()
        (harness.fleet_dir / "runs" / "02-later").mkdir()
        mark = len(runner.calls)
        with self.assertRaisesRegex(awsb.Refused, "not the last run"):
            awsb.cmd_collect(self.args(harness, "collect"), FakeSystem(run=runner))
        self.assertEqual(len(runner.calls), mark)

    def test_collect_while_a_run_is_in_progress_is_refused(self):
        harness, runner = self.kept()
        fleet_json = harness.fleet_dir / "fleet.json"
        manifest = json.loads(fleet_json.read_text())
        netbench.write_json(fleet_json, {**manifest, "phase": "running"})
        with self.assertRaisesRegex(awsb.Refused, "is running"):
            awsb.cmd_collect(self.args(harness, "collect"), FakeSystem(run=runner))

    def test_collect_of_an_unprovisioned_run_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp) / "runs" / "01-run"
            run_dir.mkdir(parents=True)
            netbench.write_json(
                Path(tmp) / "fleet.json", {"name": "p", "phase": "planned"}
            )
            with self.assertRaisesRegex(awsb.Refused, "never provisioned"):
                awsb.cmd_collect(
                    awsb.parse_args(["collect", str(run_dir)]),
                    FakeSystem(run=FakeRunner()),
                )

    def test_collect_of_a_fleet_dir_is_refused(self):
        with (
            tempfile.TemporaryDirectory() as tmp,
            self.assertRaisesRegex(awsb.Refused, "not a run dir"),
        ):
            awsb.cmd_collect(
                awsb.parse_args(["collect", tmp]), FakeSystem(run=FakeRunner())
            )


class NextCollectIndexTest(unittest.TestCase):
    def test_counts_past_the_highest_across_hosts(self):
        with tempfile.TemporaryDirectory() as tmp:
            run_dir = Path(tmp)
            self.assertEqual(awsb.next_collect_index(run_dir), 1)
            (run_dir / "hosts" / "a" / "collect-1").mkdir(parents=True)
            (run_dir / "hosts" / "b" / "collect-3").mkdir(parents=True)
            self.assertEqual(awsb.next_collect_index(run_dir), 4)


class CmdRenderTest(unittest.TestCase):
    def collected(self) -> Path:
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        write_collected_run(tmp)
        return tmp

    def test_rewrites_result_and_summary_offline(self):
        run_dir = self.collected()
        (run_dir / "summary.md").write_text("stale")
        code = awsb.cmd_render(awsb.parse_args(["render", str(run_dir)]), FakeSystem())
        self.assertIn(code, (awsb.EXIT_OK, awsb.EXIT_INVALID))
        self.assertIn("Deployment", (run_dir / "summary.md").read_text())
        self.assertEqual(
            code == awsb.EXIT_OK,
            json.loads((run_dir / "result.json").read_text())["validity"]["valid"],
        )

    def test_baseline_adds_a_comparison(self):
        run_dir = self.collected()
        awsb.write_report(run_dir)
        baseline = run_dir / "baseline.json"
        baseline.write_text((run_dir / "result.json").read_text())
        awsb.cmd_render(
            awsb.parse_args(["render", str(run_dir), "--baseline", str(baseline)]),
            FakeSystem(),
        )
        self.assertIn("baseline", (run_dir / "summary.md").read_text().lower())

    def test_run_without_result_is_refused(self):
        run_dir = self.collected()
        (run_dir / "run.json").unlink()
        with self.assertRaisesRegex(awsb.Refused, "no run.json"):
            awsb.cmd_render(awsb.parse_args(["render", str(run_dir)]), FakeSystem())


class ConfigFromManifestTest(unittest.TestCase):
    def test_round_trips_config_to_json(self):
        cfg = awsb.RunConfig(
            tag="x",
            nodes=3,
            load=netbench.BenchConfig(submit_nodes=2),
            node_env=("A=1",),
        )
        saved = json.loads(json.dumps(awsb.config_to_json(cfg)))
        self.assertEqual(awsb.config_from_manifest(saved), cfg)
        del saved["node_env"]
        self.assertEqual(awsb.config_from_manifest(saved).node_env, ())


class CommandSystemTest(unittest.TestCase):
    def test_command_without_system_fails(self):
        args = awsb.parse_args(["render", "run"])
        with self.assertRaises(TypeError):
            awsb.cmd_render(args)


if __name__ == "__main__":
    unittest.main()
