"""Tests for `aws-bench`: cost estimate, budget refusal, `plan --offline` rendering, and the
`run` orchestration (apply failure, interrupt, destroy fallback, validity, cost, index).

Side effects go through a `FakeRunner` that maps an argv prefix to a canned
`subprocess.CompletedProcess`, and records every call, so a refused plan can be shown to have
made no `aws` call at all.

    just py::test
"""

import dataclasses
import importlib.util
import json
import logging
import os
import re
import shutil
import subprocess
import tempfile
import threading
import unittest
import unittest.mock
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from importlib.machinery import SourceFileLoader
from pathlib import Path

import netbench
import test_netbench

SCRIPT = Path(__file__).with_name("aws-bench")
_spec = importlib.util.spec_from_loader(
    "aws_bench", SourceFileLoader("aws_bench", str(SCRIPT))
)
assert _spec is not None
awsb = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(awsb)


def completed(
    stdout: str = "", returncode: int = 0, stderr: str = ""
) -> subprocess.CompletedProcess:
    return subprocess.CompletedProcess(
        args=[], returncode=returncode, stdout=stdout, stderr=stderr
    )


class FakeRunner:
    """Maps an argv prefix to a canned `CompletedProcess`; records every call. An argv with no
    matching prefix raises, so a test can prove a refused plan never reached `aws` or `tofu`."""

    def __init__(
        self,
        responses: dict[tuple[str, ...], subprocess.CompletedProcess] | None = None,
    ):
        self.responses = responses or {}
        self.calls: list[list[str]] = []

    def __call__(self, argv: list[str]) -> subprocess.CompletedProcess:
        self.calls.append(argv)
        for prefix, result in self.responses.items():
            if tuple(argv[: len(prefix)]) == prefix:
                return result
        raise AssertionError(f"unexpected command: {argv!r}")

    def ran(self, *prefix: str) -> bool:
        return any(tuple(call[: len(prefix)]) == prefix for call in self.calls)


def price_response(usd_hour: float) -> subprocess.CompletedProcess:
    product = json.dumps(
        {
            "terms": {
                "OnDemand": {
                    "x": {
                        "priceDimensions": {
                            "y": {
                                "unit": "Hrs",
                                "pricePerUnit": {"USD": str(usd_hour)},
                            }
                        }
                    }
                }
            }
        }
    )
    return completed(stdout=json.dumps({"PriceList": [product]}))


def parse_plan_args(argv: list[str]) -> "awsb.argparse.Namespace":
    full = ["plan", *argv]
    args = awsb.parse_args(full)
    args.argv = full
    return args


def cmd_plan_exit(args: "awsb.argparse.Namespace", run: "awsb.Runner") -> int:
    """Mirrors `main`'s single `Refused` catch, since these tests call `cmd_plan` directly."""
    try:
        return awsb.cmd_plan(args, run=run)
    except awsb.Refused:
        return awsb.EXIT_REFUSED


def sts_response(account: str) -> subprocess.CompletedProcess:
    return completed(stdout=json.dumps({"Account": account}))


STS_CALL = ("aws", "--profile", "timeboost-dev", "sts", "get-caller-identity")


def two_node_prices() -> "awsb.Prices":
    return {
        "instances": {
            "c8g.4xlarge": {"usd_hour": 0.71, "source": "test"},
            "c8g.2xlarge": {"usd_hour": 0.355, "source": "test"},
        }
    }


# REQ:awsbench-topology
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

    def test_rejects_single_node(self):
        with self.assertRaises(awsb.Refused):
            awsb.plan_hosts(awsb.RunConfig(tag="x", nodes=1))

    def test_rejects_submit_nodes_out_of_range(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=2)
        )
        with self.assertRaises(awsb.Refused):
            awsb.plan_hosts(cfg)


class PhaseSecondsTest(unittest.TestCase):
    def test_load_seconds(self):
        load = netbench.BenchConfig(
            steps=(4.0, 8.0), step_s=30, warmup_s=60, tx_timeout_s=30
        )
        self.assertEqual(awsb.load_seconds(load), 60 + 3 * 30 + 30)

    def test_worst_uses_ready_timeout_and_collect_max(self):
        cfg = awsb.RunConfig(tag="x", ready_timeout_s=900.0)
        phases = awsb.phase_seconds(cfg)
        self.assertEqual(phases["ready"], (awsb.READY_EXPECTED_S, 900.0))
        self.assertEqual(
            phases["collect"], (awsb.COLLECT_EXPECTED_S, awsb.COLLECT_MAX_S)
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
        self.prices = two_node_prices()
        self.minor = awsb.MINOR_PRICES["eu-west-1"]

    def test_matches_hand_computed_totals(self):
        # Literal dollar values for this 2-node config, hand-computed independently of
        # estimate_cost's formula, so a formula regression trips this test.
        estimate = awsb.estimate_cost(self.hosts, self.cfg, self.prices, self.minor)
        self.assertEqual(estimate["expected_s"], 1650.0)
        self.assertEqual(estimate["ttl_s"], 3270.0)
        self.assertAlmostEqual(estimate["expected_usd"], 0.8683328695776255, places=6)
        self.assertAlmostEqual(estimate["bound_usd"], 1.7053451983447487, places=6)

    def test_bound_is_cost_at_ttl(self):
        estimate = awsb.estimate_cost(self.hosts, self.cfg, self.prices, self.minor)
        ratio = estimate["ttl_s"] / estimate["expected_s"]
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


# EDGE:awsbench-unknown-region
class RegionMinorPricesTest(unittest.TestCase):
    def test_unknown_region_refuses(self):
        with self.assertRaises(awsb.Refused):
            awsb.region_minor_prices("ap-south-2")

    def test_known_region_ok(self):
        self.assertIn("gp3_gb_month_usd", awsb.region_minor_prices("eu-west-1"))


class FormatEstimateTest(unittest.TestCase):
    def test_contains_expected_and_bound(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        minor = awsb.region_minor_prices(cfg.region)
        estimate = awsb.estimate_cost(hosts, cfg, two_node_prices(), minor)
        text = awsb.format_estimate(estimate, cfg.max_usd)
        self.assertIn(f"expected ${estimate['expected_usd']:.2f}", text)
        self.assertIn(f"hard bound ${estimate['bound_usd']:.2f}", text)
        self.assertIn("limit $10.00", text)


class ConfirmTest(unittest.TestCase):
    def test_yes_flag_always_confirms(self):
        self.assertTrue(awsb.confirm("go?", yes=True, stdin_is_tty=False))

    def test_non_tty_without_yes_refuses(self):
        self.assertFalse(awsb.confirm("go?", yes=False, stdin_is_tty=False))

    def test_tty_reads_input(self):
        with unittest.mock.patch("builtins.input", return_value="y"):
            self.assertTrue(awsb.confirm("go?", yes=False, stdin_is_tty=True))
        with unittest.mock.patch("builtins.input", return_value="n"):
            self.assertFalse(awsb.confirm("go?", yes=False, stdin_is_tty=True))


# EDGE:awsbench-price-override
class FetchPricesTest(unittest.TestCase):
    def test_override_skips_fetch_entirely(self):
        with tempfile.TemporaryDirectory() as tmp:
            cfg = awsb.RunConfig(
                tag="x", nodes=2, price=("c8g.4xlarge=0.71", "c8g.2xlarge=0.355")
            )
            runner = FakeRunner({})
            prices = awsb.resolve_prices(
                runner, cfg, Path(tmp) / "prices.json", now=0.0
            )
            self.assertEqual(runner.calls, [])
            self.assertEqual(prices["instances"]["c8g.4xlarge"]["usd_hour"], 0.71)
            self.assertEqual(
                prices["instances"]["c8g.4xlarge"]["source"], "--price override"
            )

    def test_fresh_cache_skips_fetch(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp) / "prices.json"
            cache.write_text(
                json.dumps(
                    {"eu-west-1:c8g.4xlarge": {"usd_hour": 0.5, "fetched_at": 1000.0}}
                )
            )
            runner = FakeRunner({})
            prices = awsb.fetch_prices(
                runner,
                awsb.RunConfig(tag="x"),
                {"c8g.4xlarge"},
                cache,
                now=1000.0 + 3600,
            )
            self.assertEqual(runner.calls, [])
            self.assertEqual(prices["instances"]["c8g.4xlarge"]["usd_hour"], 0.5)

    def test_expired_cache_refetches(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp) / "prices.json"
            old = 1000.0
            cache.write_text(
                json.dumps(
                    {"eu-west-1:c8g.4xlarge": {"usd_hour": 0.5, "fetched_at": old}}
                )
            )
            now = old + awsb.PRICE_CACHE_TTL_S + 1
            runner = FakeRunner(
                {
                    (
                        "aws",
                        "--profile",
                        "timeboost-dev",
                        "pricing",
                        "get-products",
                    ): price_response(0.9)
                }
            )
            prices = awsb.fetch_prices(
                runner, awsb.RunConfig(tag="x"), {"c8g.4xlarge"}, cache, now=now
            )
            self.assertTrue(
                runner.ran(
                    "aws", "--profile", "timeboost-dev", "pricing", "get-products"
                )
            )
            self.assertEqual(prices["instances"]["c8g.4xlarge"]["usd_hour"], 0.9)
            self.assertEqual(
                json.loads(cache.read_text())["eu-west-1:c8g.4xlarge"]["usd_hour"], 0.9
            )

    def test_api_failure_without_override_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp) / "prices.json"
            runner = FakeRunner(
                {
                    (
                        "aws",
                        "--profile",
                        "timeboost-dev",
                        "pricing",
                        "get-products",
                    ): completed(returncode=1, stderr="AccessDenied")
                }
            )
            with self.assertRaises(awsb.Refused):
                awsb.fetch_prices(
                    runner, awsb.RunConfig(tag="x"), {"c8g.4xlarge"}, cache, now=0.0
                )


# REQ:awsbench-render-offline / REQ:awsbench-budget-refusal / EDGE:awsbench-name-collision
class CmdPlanTest(unittest.TestCase):
    def test_offline_renders_manifest_and_peers(self):
        with tempfile.TemporaryDirectory() as out_root:
            args = parse_plan_args(
                [
                    "--tag",
                    "x",
                    "--nodes",
                    "2",
                    "--offline",
                    "--name",
                    "run1",
                    "--out-root",
                    out_root,
                    "--price",
                    "c8g.4xlarge=0.71",
                    "--price",
                    "c8g.2xlarge=0.355",
                ]
            )
            runner = FakeRunner({})
            code = awsb.cmd_plan(args, run=runner)
            self.assertEqual(code, awsb.EXIT_OK)
            self.assertEqual(runner.calls, [])
            manifest = json.loads(
                (Path(out_root) / "run1" / "manifest.json").read_text()
            )
            self.assertEqual(manifest["phase"], "planned")
            self.assertEqual(manifest["peers"], {"node0": ["node1"], "node1": []})
            self.assertEqual(len(manifest["hosts"]), 3)
            self.assertTrue((Path(out_root) / "run1" / "driver.log").exists())
            self.assertTrue((Path(out_root) / "run1" / "events.jsonl").exists())

    def test_budget_refusal_makes_no_tofu_apply(self):
        with tempfile.TemporaryDirectory() as out_root:
            args = parse_plan_args(
                [
                    "--tag",
                    "x",
                    "--nodes",
                    "5",
                    "--max-usd",
                    "0.01",
                    "--out-root",
                    out_root,
                    "--price",
                    "c8g.4xlarge=0.71",
                    "--price",
                    "c8g.2xlarge=0.355",
                ]
            )
            runner = FakeRunner({STS_CALL: sts_response("027574771971")})
            code = cmd_plan_exit(args, run=runner)
            self.assertEqual(code, awsb.EXIT_REFUSED)
            self.assertFalse(runner.ran("tofu"))

    # REQ:awsbench-render-offline
    def test_offline_without_price_refuses(self):
        with tempfile.TemporaryDirectory() as out_root:
            args = parse_plan_args(
                ["--tag", "x", "--nodes", "2", "--offline", "--out-root", out_root]
            )
            runner = FakeRunner({})
            code = cmd_plan_exit(args, run=runner)
            self.assertEqual(code, awsb.EXIT_REFUSED)
            self.assertEqual(runner.calls, [])

    def test_name_collision_refuses(self):
        with tempfile.TemporaryDirectory() as out_root:
            common = [
                "--tag",
                "x",
                "--name",
                "dup",
                "--out-root",
                out_root,
                "--price",
                "c8g.4xlarge=0.71",
                "--price",
                "c8g.2xlarge=0.355",
            ]
            first = awsb.cmd_plan(
                parse_plan_args(common),
                run=FakeRunner({STS_CALL: sts_response("027574771971")}),
            )
            self.assertEqual(first, awsb.EXIT_OK)
            second_runner = FakeRunner({})
            second = cmd_plan_exit(parse_plan_args(common), run=second_runner)
            self.assertEqual(second, awsb.EXIT_REFUSED)
            # the name collision is caught before any AWS call, including the account guard
            self.assertEqual(second_runner.calls, [])

    # EDGE:awsbench-name-validation
    def test_name_path_escape_refuses(self):
        with tempfile.TemporaryDirectory() as out_root:
            args = parse_plan_args(
                [
                    "--tag",
                    "x",
                    "--nodes",
                    "2",
                    "--name",
                    "../escape",
                    "--out-root",
                    out_root,
                    "--price",
                    "c8g.4xlarge=0.71",
                    "--price",
                    "c8g.2xlarge=0.355",
                ]
            )
            runner = FakeRunner({})
            code = cmd_plan_exit(args, run=runner)
            self.assertEqual(code, awsb.EXIT_REFUSED)
            self.assertEqual(runner.calls, [])
            self.assertFalse((Path(out_root) / ".." / "escape").resolve().exists())

    def test_default_name_is_valid(self):
        self.assertRegex(awsb.default_run_name(), awsb.NAME_RE.pattern)

    # REQ:awsbench-account-guard
    def test_online_account_mismatch_makes_exactly_one_call(self):
        with tempfile.TemporaryDirectory() as out_root:
            args = parse_plan_args(
                [
                    "--tag",
                    "x",
                    "--nodes",
                    "2",
                    "--out-root",
                    out_root,
                    "--price",
                    "c8g.4xlarge=0.71",
                    "--price",
                    "c8g.2xlarge=0.355",
                ]
            )
            runner = FakeRunner({STS_CALL: sts_response("999999999999")})
            code = cmd_plan_exit(args, run=runner)
            self.assertEqual(code, awsb.EXIT_REFUSED)
            self.assertEqual(len(runner.calls), 1)


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
            awsb.caller_account(runner, awsb.RunConfig(tag="x"))
        self.assertEqual(len(runner.calls), 1)

    def test_match_returns_account(self):
        cfg = awsb.RunConfig(tag="x")
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "sts",
                    "get-caller-identity",
                ): completed(stdout=json.dumps({"Account": cfg.account}))
            }
        )
        self.assertEqual(awsb.caller_account(runner, cfg), cfg.account)


class DefaultVpcTest(unittest.TestCase):
    def test_no_default_vpc_refuses(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-vpcs",
                ): completed(stdout=json.dumps({"Vpcs": []}))
            }
        )
        with self.assertRaises(awsb.Refused):
            awsb.default_vpc(runner, awsb.RunConfig(tag="x"))

    def test_dns_hostnames_disabled_refuses(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-vpcs",
                ): completed(stdout=json.dumps({"Vpcs": [{"VpcId": "vpc-1"}]})),
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-vpc-attribute",
                ): completed(
                    stdout=json.dumps({"EnableDnsHostnames": {"Value": False}})
                ),
            }
        )
        with self.assertRaises(awsb.Refused):
            awsb.default_vpc(runner, awsb.RunConfig(tag="x"))

    def test_ok_returns_vpc_id(self):
        runner = FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-vpcs",
                ): completed(stdout=json.dumps({"Vpcs": [{"VpcId": "vpc-1"}]})),
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "ec2",
                    "describe-vpc-attribute",
                ): completed(
                    stdout=json.dumps({"EnableDnsHostnames": {"Value": True}})
                ),
            }
        )
        self.assertEqual(awsb.default_vpc(runner, awsb.RunConfig(tag="x")), "vpc-1")


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
        az = awsb.capable_az(runner, awsb.RunConfig(tag="x"), {"c8g.4xlarge"})
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
            awsb.capable_az(runner, awsb.RunConfig(tag="x"), {"c8g.4xlarge"})


class VcpuHeadroomTest(unittest.TestCase):
    def _quota_runner(self, quota):
        return FakeRunner(
            {
                (
                    "aws",
                    "--profile",
                    "timeboost-dev",
                    "service-quotas",
                    "get-service-quota",
                ): completed(stdout=json.dumps({"Quota": {"Value": quota}}))
            }
        )

    def _instance_types(self, vcpus_by_type):
        return {t: {"Type": t, "VCpus": v} for t, v in vcpus_by_type.items()}

    def test_within_quota_ok(self):
        instance_types = self._instance_types({"c8g.4xlarge": 16, "c8g.2xlarge": 8})
        needed, in_use, quota = awsb.vcpu_headroom(
            self._quota_runner(256.0),
            awsb.RunConfig(tag="x"),
            two_node_hosts(),
            running=[],
            instance_types=instance_types,
        )
        self.assertEqual((needed, in_use, quota), (16 + 8 + 16, 0, 256.0))

    def test_over_quota_refuses(self):
        instance_types = self._instance_types({"c8g.4xlarge": 16, "c8g.2xlarge": 8})
        with self.assertRaises(awsb.Refused):
            awsb.vcpu_headroom(
                self._quota_runner(10.0),
                awsb.RunConfig(tag="x"),
                two_node_hosts(),
                running=[],
                instance_types=instance_types,
            )

    def test_repeated_running_type_counted_per_instance(self):
        # Three running c8g.4xlarge must count as 3x16 vCPUs, not once.
        instance_types = self._instance_types({"c8g.4xlarge": 16, "c8g.2xlarge": 8})
        running = ["c8g.4xlarge", "c8g.4xlarge", "c8g.4xlarge"]
        _needed, in_use, _quota = awsb.vcpu_headroom(
            self._quota_runner(256.0),
            awsb.RunConfig(tag="x"),
            two_node_hosts(),
            running=running,
            instance_types=instance_types,
        )
        self.assertEqual(in_use, 48)


class PreflightTest(unittest.TestCase):
    """Runs `preflight` with a fake `which` (no real tools needed) and a stubbed
    `resolve_image`, and asserts `describe-instance-types` is called exactly once:
    `resolve_ami_arch` and `vcpu_headroom` used to each fetch it separately."""

    def test_describe_instance_types_called_once(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        responses = {
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "sts",
                "get-caller-identity",
            ): completed(stdout=json.dumps({"Account": cfg.account})),
            ("aws", "--profile", "timeboost-dev", "ec2", "describe-vpcs"): completed(
                stdout=json.dumps({"Vpcs": [{"VpcId": "vpc-1"}]})
            ),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "describe-vpc-attribute",
            ): completed(stdout=json.dumps({"EnableDnsHostnames": {"Value": True}})),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "describe-instance-type-offerings",
            ): completed(
                stdout=json.dumps(
                    {"InstanceTypeOfferings": [{"Location": "eu-west-1a"}]}
                )
            ),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "describe-instances",
            ): completed(stdout=json.dumps([])),
            (
                "aws",
                "--profile",
                "timeboost-dev",
                "ec2",
                "describe-instance-types",
            ): completed(
                stdout=json.dumps(
                    [
                        {"Type": "c8g.4xlarge", "VCpus": 16, "Archs": ["arm64"]},
                        {"Type": "c8g.2xlarge", "VCpus": 8, "Archs": ["arm64"]},
                    ]
                )
            ),
            ("aws", "--profile", "timeboost-dev", "service-quotas"): completed(
                stdout=json.dumps({"Quota": {"Value": 256.0}})
            ),
            ("aws", "--profile", "timeboost-dev", "ec2", "describe-images"): completed(
                stdout="ami-0abc\n"
            ),
            ("git", "status"): completed(stdout=""),
        }
        runner = FakeRunner(responses)
        fake_image = {
            "ref": "x",
            "digest": "sha256:" + "a" * 64,
            "revision": None,
            "platforms": ["linux/arm64"],
        }
        with unittest.mock.patch.object(awsb, "resolve_image", return_value=fake_image):
            result = awsb.preflight(runner, cfg, hosts, which=lambda name: "/usr/bin/x")
        describe_calls = [
            c
            for c in runner.calls
            if tuple(c[:5])
            == ("aws", "--profile", "timeboost-dev", "ec2", "describe-instance-types")
        ]
        self.assertEqual(len(describe_calls), 1)
        self.assertEqual(result["az"], "eu-west-1a")
        self.assertEqual(result["ami_id"], "ami-0abc")


class CheckGitCleanTest(unittest.TestCase):
    def test_clean_tree_returns_none(self):
        runner = FakeRunner({("git", "status"): completed(stdout="")})
        self.assertIsNone(awsb.check_git_clean(runner, allow_dirty=False))

    def test_dirty_without_allow_dirty_refuses(self):
        runner = FakeRunner({("git", "status"): completed(stdout=" M file.py\n")})
        with self.assertRaises(awsb.Refused):
            awsb.check_git_clean(runner, allow_dirty=False)

    def test_dirty_with_allow_dirty_returns_diff(self):
        runner = FakeRunner(
            {
                ("git", "status"): completed(stdout=" M file.py\n"),
                ("git", "diff"): completed(stdout="--- a/file.py\n+++ b/file.py\n"),
            }
        )
        diff = awsb.check_git_clean(runner, allow_dirty=True)
        self.assertIn("file.py", diff)


class ToolsOnPathTest(unittest.TestCase):
    def test_missing_tool_refuses(self):
        with self.assertRaises(awsb.Refused):
            awsb.tools_on_path(("definitely-not-a-real-tool-xyz",))

    def test_present_tools_ok(self):
        awsb.tools_on_path(("python3",))

    def test_injectable_which_needs_no_real_tools(self):
        awsb.tools_on_path(("tofu", "aws"), which=lambda name: f"/usr/bin/{name}")
        with self.assertRaises(awsb.Refused):
            awsb.tools_on_path(("tofu",), which=lambda name: None)


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
        self.assertEqual(tfvars["account_id"], cfg.account)
        self.assertEqual(tfvars["profile"], cfg.profile)
        self.assertEqual(tfvars["az"], "eu-west-1a")
        self.assertEqual(tfvars["ami_id"], "ami-0abc")
        self.assertEqual(tfvars["operator_cidr"], "203.0.113.5/32")
        self.assertFalse(tfvars["offline"])
        self.assertEqual(len(tfvars["hosts"]), 3)
        self.assertEqual(
            tfvars["hosts"]["node0"]["user_data_path"],
            str(run_dir / "hosts" / "node0" / "user-data.sh"),
        )

    def test_offline_flag_passes_through(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, offline=True, load=netbench.BenchConfig(submit_nodes=1)
        )
        tfvars = awsb.render_tfvars(
            cfg,
            Path("/tmp/aws-bench/run1"),
            "run1",
            "alice",
            awsb.plan_hosts(cfg),
            ssh_pub="k",
            operator_cidr="203.0.113.5/32",
            expires_at="2026-01-01T00:00:00Z",
            git_rev="abc1234",
            az="",
            ami_id="",
        )
        self.assertTrue(tfvars["offline"])


class ClassifyTfErrorTest(unittest.TestCase):
    def test_vcpu_limit(self):
        self.assertIn(
            "VcpuLimitExceeded", awsb.classify_tf_error("... VcpuLimitExceeded ...")
        )
        self.assertIn("quota-code", awsb.classify_tf_error("... VcpuLimitExceeded ..."))

    def test_insufficient_capacity(self):
        self.assertIn(
            "--az", awsb.classify_tf_error("Error: InsufficientInstanceCapacity")
        )

    def test_unauthorized(self):
        self.assertIn(
            "IAM", awsb.classify_tf_error("Error: UnauthorizedOperation: ...")
        )

    def test_unclassified_keeps_last_line(self):
        result = awsb.classify_tf_error("line one\nline two\nsome other tofu error\n")
        self.assertIn("some other tofu error", result)


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
        tf = awsb.Terraform(runner, "tofu", tf_dir, env={})
        tf.init()
        tf.plan()
        tf.apply()
        self.assertEqual(tf.output(), {"az": {"value": "eu-west-1a"}})
        tf.destroy()
        self.assertTrue(runner.ran("tofu", f"-chdir={tf_dir}", "apply"))

    def test_apply_failure_is_classified(self):
        tf_dir = Path("/tmp/aws-bench/run1/terraform")
        runner = FakeRunner(
            {
                (
                    "tofu",
                    f"-chdir={tf_dir}",
                    "apply",
                ): completed(returncode=1, stderr="Error: VcpuLimitExceeded: ...")
            }
        )
        tf = awsb.Terraform(runner, "tofu", tf_dir, env={})
        with self.assertRaises(awsb.TfFailed) as ctx:
            tf.apply()
        self.assertEqual(ctx.exception.stage, "apply")
        self.assertIn("VcpuLimitExceeded", str(ctx.exception))

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
        tf = awsb.Terraform(runner, "tofu", tf_dir, env={})
        with self.assertRaises(awsb.TfFailed) as ctx:
            tf.destroy()
        self.assertEqual(ctx.exception.stage, "destroy")

    def test_env_is_exported(self):
        # A key that never varies between tests: a per-run value like AWS_REGION would leak
        # into every test that runs afterward, since os.environ is process-global.
        tf_dir = Path("/tmp/aws-bench/run1/terraform")
        with unittest.mock.patch.dict(os.environ, {}, clear=False):
            awsb.Terraform(
                FakeRunner({}),
                "tofu",
                tf_dir,
                env={"TF_PLUGIN_CACHE_DIR": "/tmp/plugins"},
            )
            self.assertEqual(os.environ["TF_PLUGIN_CACHE_DIR"], "/tmp/plugins")


SINGLE_MANIFEST_MEDIA_TYPE = "application/vnd.oci.image.manifest.v1+json"


class FakeRegistry(ThreadingHTTPServer):
    """A minimal OCI/Docker registry: one repository and tag, anonymous token challenge.
    `platforms` is a list of `(os, architecture)` pairs for the index; a `tag` of `"missing"`
    makes the manifest request 404, `deny_token` makes the token endpoint 401 (a private
    image), `deny_manifest_status` makes the manifest request fail with that status without
    ever offering a token challenge, and `index=False` serves a single manifest at the tag
    instead of a multi-platform index."""

    def __init__(
        self,
        repository: str,
        tag: str,
        platforms: list[tuple[str, str]],
        revision: str | None = None,
        deny_token: bool = False,
        deny_manifest_status: int | None = None,
        index: bool = True,
    ):
        super().__init__(("127.0.0.1", 0), FakeRegistryHandler)
        self.repository = repository
        self.tag = tag
        self.platforms = platforms
        self.revision = revision
        self.deny_token = deny_token
        self.deny_manifest_status = deny_manifest_status
        self.index = index
        self.digests = {p: f"sha256:{i:064d}" for i, p in enumerate(platforms)}
        self.config_digest = "sha256:" + "c" * 64

    @property
    def host(self) -> str:
        return f"127.0.0.1:{self.server_address[1]}"

    @property
    def ref(self) -> str:
        return f"{self.host}/{self.repository}:{self.tag}"


class FakeRegistryHandler(BaseHTTPRequestHandler):
    server: FakeRegistry

    def log_message(self, format, *args):
        pass

    def _reply_json(
        self, status: int, obj: dict, content_type: str = "application/json"
    ):
        body = json.dumps(obj).encode()
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        server = self.server
        if self.path.startswith("/token"):
            if server.deny_token:
                self.send_response(401)
                self.end_headers()
                return
            self._reply_json(200, {"token": "faketoken"})
            return

        manifest_prefix = f"/v2/{server.repository}/manifests/"
        blob_prefix = f"/v2/{server.repository}/blobs/"

        if self.path.startswith(manifest_prefix):
            if server.deny_manifest_status is not None:
                self.send_response(server.deny_manifest_status)
                self.end_headers()
                return
            if self.headers.get("Authorization") != "Bearer faketoken":
                self.send_response(401)
                self.send_header(
                    "WWW-Authenticate",
                    f'Bearer realm="http://{server.host}/token",service="fake",'
                    f'scope="repository:{server.repository}:pull"',
                )
                self.end_headers()
                return
            ref = self.path[len(manifest_prefix) :]
            if ref == server.tag:
                if server.tag == "missing":
                    self.send_response(404)
                    self.end_headers()
                    return
                if not server.index:
                    self._reply_json(
                        200,
                        {
                            "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                            "config": {"digest": server.config_digest},
                        },
                    )
                    return
                manifests = [
                    {
                        "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                        "digest": server.digests[p],
                        "platform": {"os": p[0], "architecture": p[1]},
                    }
                    for p in server.platforms
                ]
                self._reply_json(
                    200,
                    {
                        "mediaType": "application/vnd.oci.image.index.v1+json",
                        "manifests": manifests,
                    },
                    content_type="application/vnd.oci.image.index.v1+json",
                )
                return
            if ref == server.config_digest:
                self.send_response(404)
                self.end_headers()
                return
            matching = [p for p, digest in server.digests.items() if digest == ref]
            if not matching:
                self.send_response(404)
                self.end_headers()
                return
            self._reply_json(
                200,
                {
                    "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                    "config": {"digest": server.config_digest},
                },
            )
            return

        if self.path.startswith(blob_prefix):
            digest = self.path[len(blob_prefix) :]
            if digest != server.config_digest:
                self.send_response(404)
                self.end_headers()
                return
            labels = (
                {"org.opencontainers.image.revision": server.revision}
                if server.revision is not None
                else {}
            )
            self._reply_json(
                200,
                {"architecture": "arm64", "os": "linux", "config": {"Labels": labels}},
            )
            return

        self.send_response(404)
        self.end_headers()


# REQ:awsbench-image-check
class ResolveImageTest(unittest.TestCase):
    def _serve(self, **kwargs) -> FakeRegistry:
        server = FakeRegistry(**kwargs)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        # addCleanup is LIFO: register in reverse of the intended shutdown -> join ->
        # server_close order, since join() needs shutdown() to have stopped serve_forever.
        self.addCleanup(server.server_close)
        self.addCleanup(thread.join)
        self.addCleanup(server.shutdown)
        return server

    def test_resolves_digest_platforms_and_revision(self):
        server = self._serve(
            repository="test/image",
            tag="v1",
            platforms=[("linux", "amd64"), ("linux", "arm64")],
            revision="abc1234",
        )
        info = awsb.resolve_image(server.ref)
        self.assertEqual(info["digest"], server.digests[("linux", "arm64")])
        self.assertCountEqual(info["platforms"], ["linux/amd64", "linux/arm64"])
        self.assertEqual(info["revision"], "abc1234")
        self.assertEqual(info["ref"], server.ref)

    def test_no_revision_label_is_none(self):
        server = self._serve(repository="x", tag="v1", platforms=[("linux", "arm64")])
        info = awsb.resolve_image(server.ref)
        self.assertIsNone(info["revision"])

    def test_single_manifest_without_index(self):
        server = self._serve(
            repository="x", tag="v1", platforms=[("linux", "arm64")], index=False
        )
        info = awsb.resolve_image(server.ref)
        self.assertEqual(info["platforms"], ["linux/arm64"])
        self.assertTrue(info["digest"].startswith("sha256:"))

    def test_attestation_platform_ignored(self):
        # buildx publishes an extra unknown/unknown manifest for SBOM/provenance attestations.
        server = self._serve(
            repository="x",
            tag="v1",
            platforms=[("unknown", "unknown"), ("linux", "arm64")],
        )
        info = awsb.resolve_image(server.ref)
        self.assertEqual(info["platforms"], ["linux/arm64"])

    # EDGE:awsbench-image-no-arm64
    def test_missing_arm64_refuses(self):
        server = self._serve(repository="x", tag="v1", platforms=[("linux", "amd64")])
        with self.assertRaises(awsb.Refused) as ctx:
            awsb.resolve_image(server.ref)
        self.assertIn("no linux/arm64 platform", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))

    def test_missing_tag_refuses(self):
        server = self._serve(
            repository="x", tag="missing", platforms=[("linux", "arm64")]
        )
        with self.assertRaises(awsb.Refused) as ctx:
            awsb.resolve_image(server.ref)
        self.assertIn("image not found", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))

    # EDGE:awsbench-image-private
    def test_private_image_denied_token_refuses(self):
        server = self._serve(
            repository="x", tag="v1", platforms=[("linux", "arm64")], deny_token=True
        )
        with self.assertRaises(awsb.Refused) as ctx:
            awsb.resolve_image(server.ref)
        self.assertIn("token request", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))

    def test_private_image_403_refuses(self):
        server = self._serve(
            repository="x",
            tag="v1",
            platforms=[("linux", "arm64")],
            deny_manifest_status=403,
        )
        with self.assertRaises(awsb.Refused) as ctx:
            awsb.resolve_image(server.ref)
        self.assertIn("private or inaccessible", str(ctx.exception))
        self.assertIn(server.ref, str(ctx.exception))


class ParseRefTest(unittest.TestCase):
    def test_docker_hub_official_image(self):
        self.assertEqual(
            awsb.parse_ref("postgres:16"),
            ("registry-1.docker.io", "library/postgres", "16"),
        )

    def test_ghcr_multi_segment(self):
        self.assertEqual(
            awsb.parse_ref("ghcr.io/espressosystems/espresso-network/espresso-node:x"),
            ("ghcr.io", "espressosystems/espresso-network/espresso-node", "x"),
        )

    def test_no_tag_defaults_to_latest(self):
        self.assertEqual(
            awsb.parse_ref("ghcr.io/foundry-rs/foundry"),
            ("ghcr.io", "foundry-rs/foundry", "latest"),
        )

    def test_docker_io_alias_resolves_to_registry_host(self):
        # SUPPORT_IMAGES["postgres"]; a literal docker.io host must not be sent to the
        # registry as-is (https://docker.io/v2/... 302s to www.docker.com).
        self.assertEqual(
            awsb.parse_ref("docker.io/library/postgres:16"),
            ("registry-1.docker.io", "library/postgres", "16"),
        )
        self.assertEqual(
            awsb.parse_ref("index.docker.io/library/postgres:16"),
            ("registry-1.docker.io", "library/postgres", "16"),
        )


TOFU = shutil.which("tofu")


@unittest.skipUnless(TOFU, "tofu/opentofu not on PATH")
class TofuValidateTest(unittest.TestCase):
    """TEST:awsbench-tofu-validate-ok: validates the module and plans it with offline=true, so
    it needs no real AWS credentials; skips entirely when tofu is absent from the environment."""

    def test_validate_and_offline_plan(self):
        assert TOFU is not None  # narrows the type; the class is skipped otherwise
        terraform_src = Path(__file__).parent / "aws" / "terraform"
        with tempfile.TemporaryDirectory() as tmp:
            terraform_dir = Path(tmp) / "terraform"
            shutil.copytree(terraform_src, terraform_dir)
            # offline=true still evaluates for_each = var.hosts (main.tf), so the user-data
            # file must exist for `file()` to succeed at plan time.
            user_data_path = terraform_dir / "ctl-user-data.sh"
            user_data_path.write_text("#!/bin/sh\necho ok\n")
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
                        "user_data_path": str(user_data_path),
                    }
                },
            }
            (terraform_dir / "terraform.tfvars.json").write_text(json.dumps(tfvars))
            env = {
                **os.environ,
                "AWS_ACCESS_KEY_ID": "test",
                "AWS_SECRET_ACCESS_KEY": "test",
                "AWS_REGION": "eu-west-1",
            }
            init = subprocess.run(
                [TOFU, f"-chdir={terraform_dir}", "init", "-input=false"],
                capture_output=True,
                text=True,
                env=env,
                check=False,
            )
            self.assertEqual(init.returncode, 0, init.stderr)
            validate = subprocess.run(
                [TOFU, f"-chdir={terraform_dir}", "validate"],
                capture_output=True,
                text=True,
                env=env,
                check=False,
            )
            self.assertEqual(validate.returncode, 0, validate.stderr)
            plan = subprocess.run(
                [
                    TOFU,
                    f"-chdir={terraform_dir}",
                    "plan",
                    "-input=false",
                    "-refresh=false",
                ],
                capture_output=True,
                text=True,
                env=env,
                check=False,
            )
            self.assertEqual(plan.returncode, 0, plan.stderr)


def host_info(name: str, role: str, index: int) -> dict:
    return {
        "name": name,
        "role": role,
        "public_ip": f"203.0.113.{index}",
        "private_ip": f"10.0.0.{index}",
        "private_dns": f"ip-10-0-0-{index}.eu-west-1.compute.internal",
        "instance_id": f"i-{index:012x}",
    }


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
        rendered = awsb.render_genesis(
            self.template, n=3, max_block_size="100mb"
        ).decode()
        self.assertIn("stake_table_capacity = 10", rendered)
        self.assertIn("capacity = 10", rendered)

    def test_capacity_scales_above_ten(self):
        rendered = awsb.render_genesis(
            self.template, n=50, max_block_size="100mb"
        ).decode()
        self.assertIn("stake_table_capacity = 50", rendered)
        self.assertIn("capacity = 50", rendered)

    def test_max_block_size_set_in_both_chain_configs(self):
        rendered = awsb.render_genesis(
            self.template, n=5, max_block_size="64mb"
        ).decode()
        self.assertEqual(rendered.count('max_block_size = "64mb"'), 2)
        self.assertNotIn("100mb", rendered)

    def test_raises_if_template_shape_changes(self):
        with self.assertRaises(ValueError):
            awsb.render_genesis(b"no capacity fields here", n=5, max_block_size="64mb")


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
        text = awsb.render_node_env(host, self.hosts)
        return dict(line.split("=", 1) for line in text.splitlines())

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
            for line in awsb.render_node_env(host, hosts).splitlines()
        )
        self.assertNotIn("ESPRESSO_NODE_STATE_PEERS", env)

    def test_journal_max_bytes_reserves_disk_and_halves_for_query(self):
        validator_gb = self.env("node1")["ESPRESSO_NODE_JOURNAL_MAX_BYTES"]
        query_gb = self.env("node0")["ESPRESSO_NODE_JOURNAL_MAX_BYTES"]
        usable = (awsb.NODE_ROOT_GB_DEFAULT - awsb.RESERVED_GB) * 1_000_000_000
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


class RenderCtlEnvTest(unittest.TestCase):
    def setUp(self):
        self.cfg = awsb.RunConfig(
            tag="x", nodes=5, load=netbench.BenchConfig(submit_nodes=4)
        )
        self.hosts = fleet(5)
        self.dotenv = awsb.parse_dotenv(
            (Path(__file__).parents[2] / ".env").read_text()
        )

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

    def test_port_drift_from_dotenv_raises(self):
        dotenv = dict(self.dotenv, ESPRESSO_L1_PORT="9999")
        with self.assertRaises(ValueError):
            awsb.render_ctl_env(self.hosts, self.cfg, dotenv)


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
            found, {"$digests", "$chrony", "$name", "$digest", "$pulled"}
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


# REQ:awsbench-hostmon-parsers
class DiskstatsTest(unittest.TestCase):
    def test_parses_known_columns(self):
        line = "   8       0 nvme0n1 100 0 2000 5 200 0 4000 10 0 20 20\n"
        result = awsb.diskstats(line)
        self.assertEqual(
            result["nvme0n1"],
            {"read_bytes": 2000 * 512, "write_bytes": 4000 * 512, "io_ticks_ms": 20},
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


def two_node_hosts_info() -> dict:
    return {
        "ctl": host_info("ctl", "ctl", 1),
        "node0": host_info("node0", "query", 2),
        "node1": host_info("node1", "validator", 3),
    }


def fake_preflight() -> dict:
    return {
        "account": "027574771971",
        "vpc_id": "vpc-1",
        "az": "eu-west-1b",
        "ami_id": "ami-0abc",
        "images": fake_images(),
        "vcpu_needed": 40,
        "vcpu_in_use": 0,
        "vcpu_quota": 256.0,
        "git_diff": None,
    }


class FleetRunner:
    """Answers every command `cmd_run` issues (git, tofu, ssh, rsync, aws) for a 2-node fleet.
    `states` are the successive agent-state.json contents; the last one repeats."""

    def __init__(
        self,
        states: list[dict],
        apply: subprocess.CompletedProcess | None = None,
        destroys: list[subprocess.CompletedProcess] | None = None,
        on_poll=None,
        describe: str = "[]",
    ):
        self.states = states
        self.apply = apply or completed()
        self.destroys = destroys or [completed()]
        self.on_poll = on_poll
        self.describe = describe
        self.polls = 0
        self.calls: list[list[str]] = []
        self.lock = threading.Lock()

    def ran(self, *needle: str) -> bool:
        return any(all(n in " ".join(call) for n in needle) for call in self.calls)

    def count(self, *needle: str) -> int:
        return sum(all(n in " ".join(c) for n in needle) for c in self.calls)

    def __call__(self, argv: list[str]) -> subprocess.CompletedProcess:
        with self.lock:
            self.calls.append(argv)
        if argv[0] == "git":
            return completed(stdout="a" * 40 + "\n")
        if argv[0] == "tofu":
            return self.tofu(argv[2])
        if argv[0] == "ssh":
            return self.ssh(argv[-1])
        if argv[0] == "rsync":
            return completed()
        return self.aws(argv)

    def tofu(self, verb: str) -> subprocess.CompletedProcess:
        if verb == "apply":
            return self.apply
        if verb == "output":
            hosts = two_node_hosts_info()
            return completed(stdout=json.dumps({"hosts": {"value": hosts}}))
        if verb == "destroy":
            with self.lock:
                return (
                    self.destroys.pop(0) if len(self.destroys) > 1 else self.destroys[0]
                )
        return completed(stdout="plan")

    def ssh(self, command: str) -> subprocess.CompletedProcess:
        if "ready.json" in command:
            digests = {n: f"{i['ref']}@{i['digest']}" for n, i in fake_images().items()}
            tracking = "System time     : 0.000001000 seconds fast of NTP time\n"
            return completed(
                stdout=json.dumps({"digests": digests, "chronyc_tracking": tracking})
            )
        if "agent-state.json" in command:
            with self.lock:
                index = min(self.polls, len(self.states) - 1)
                self.polls += 1
            if self.on_poll:
                self.on_poll(self.polls)
            return completed(stdout=json.dumps(self.states[index]))
        if "date +%s.%N" in command:
            return completed(stdout="1000.5\n")
        if "docker inspect -f" in command:
            return completed(stdout="2026-09-29T15:00:00.100000000Z\n")
        if "docker wait deploy" in command:
            return completed(stdout="0\n")
        return completed()

    def aws(self, argv: list[str]) -> subprocess.CompletedProcess:
        if "describe-instances" in argv:
            return completed(stdout=self.describe)
        if "get-resources" in argv:
            arns = [
                "arn:aws:ec2:eu-west-1:1:instance/i-1",
                "arn:aws:ec2:eu-west-1:1:security-group/sg-1",
                "arn:aws:ec2:eu-west-1:1:key-pair/key-1",
            ]
            return completed(stdout=json.dumps(arns))
        return completed()


DONE_STATE = {
    "phase": "done",
    "detail": "load finished",
    "ready_s": 30.0,
    "t0": 100.0,
    "t1": 200.0,
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
    """A temp out-root, ssh key and patched waits for driving `cmd_run` end to end."""

    def __init__(
        self, test: unittest.TestCase, name: str = "run1", confirmed: bool = False
    ):
        self.yes = not confirmed
        self.tmp = Path(tempfile.mkdtemp())
        test.addCleanup(shutil.rmtree, self.tmp)
        (self.tmp / "key").write_text("private")
        (self.tmp / "key.pub").write_text("ssh-ed25519 AAAA test")
        self.name = name
        self.run_dir = self.tmp / "out" / name
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
        ]
        if confirmed:
            # An interactive run: `confirm` says yes, but --yes is absent, so an interrupt asks.
            patches.append(
                unittest.mock.patch.object(awsb, "confirm", return_value=True)
            )
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
            "--name",
            self.name,
            "--out-root",
            str(self.tmp / "out"),
            "--ssh-key",
            str(self.tmp / "key"),
            "--operator-cidr",
            "203.0.113.5/32",
            "--genesis",
            str(Path(__file__).with_name("genesis.toml")),
            "--price",
            "c8g.4xlarge=0.71",
            "--price",
            "c8g.2xlarge=0.355",
            *(["--yes"] if self.yes else []),
            *extra,
        ]
        args = awsb.parse_args(argv)
        args.argv = argv
        return args

    def run(self, runner: FleetRunner, ask=lambda timeout: True, extra=()) -> int:
        return awsb.cmd_run(
            self.args(*extra),
            run=runner,
            interrupts=awsb.Interrupts(),
            ask_destroy=ask,
        )

    def last_log_line(self) -> str:
        return (self.run_dir / "driver.log").read_text().splitlines()[-1]

    def index(self) -> str:
        return (self.tmp / "out" / "INDEX.md").read_text()


# REQ:awsbench-apply-failure
class RunApplyFailureTest(unittest.TestCase):
    def test_classifies_destroys_and_exits_3(self):
        harness = RunHarness(self)
        runner = FleetRunner(
            [DONE_STATE],
            apply=completed(returncode=1, stderr="Error: InsufficientInstanceCapacity"),
        )
        code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_FAILED)
        self.assertTrue(runner.ran("tofu", "apply"))
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertFalse(runner.ran("ssh"))
        log = (harness.run_dir / "driver.log").read_text()
        self.assertIn("InsufficientInstanceCapacity", log)
        self.assertIn("| 3 |", harness.index())

    def test_declined_prompt_refuses_before_apply(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE])
        args = harness.args()
        args.yes = False
        with self.assertRaises(awsb.Refused):
            awsb.cmd_run(args, run=runner, interrupts=awsb.Interrupts())
        self.assertFalse(runner.ran("tofu", "apply"))
        self.assertFalse(runner.ran("ssh"))


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
        cost = json.loads((harness.run_dir / "cost.json").read_text())
        self.assertAlmostEqual(cost["duration_s"], 1800.0)
        self.assertGreater(cost["actual"], 0)
        self.assertLess(cost["actual"], cost["bound"])
        manifest = json.loads((harness.run_dir / "manifest.json").read_text())
        self.assertEqual(manifest["phase"], "done")
        self.assertEqual(manifest["cost_usd"]["actual"], cost["actual"])
        self.assertEqual(manifest["start_spread_s"], 0.0)
        row = harness.index().splitlines()[-1]
        self.assertIn("| run1 |", row)
        self.assertIn("| valid | 0 |", row)
        self.assertTrue(row.endswith("| yes |"))

    def test_invalid_result_exits_1(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result(valid=False)
        ):
            code = harness.run(runner)
        self.assertEqual(code, awsb.EXIT_INVALID)
        self.assertTrue(runner.ran("tofu", "destroy"))

    def test_keep_skips_destroy(self):
        harness = RunHarness(self)
        runner = FleetRunner([DONE_STATE])
        with unittest.mock.patch.object(
            awsb, "write_report", return_value=valid_result()
        ):
            code = harness.run(runner, extra=("--keep",))
        self.assertEqual(code, awsb.EXIT_OK)
        self.assertFalse(runner.ran("tofu", "destroy"))
        self.assertIn("| no |", harness.index())

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
    def run_interrupted(self, ask) -> tuple[RunHarness, FleetRunner, int]:
        harness = RunHarness(self, confirmed=True)
        interrupts = awsb.Interrupts()
        running = {"phase": "loading", "detail": "x"}
        runner = FleetRunner(
            [running], on_poll=lambda n: n == 2 and interrupts.event.set()
        )
        code = awsb.cmd_run(
            harness.args(), run=runner, interrupts=interrupts, ask_destroy=ask
        )
        return harness, runner, code

    def test_stops_agent_collects_destroys_and_exits_3(self):
        asked = []
        harness, runner, code = self.run_interrupted(
            lambda timeout: asked.append(timeout) or True
        )
        self.assertEqual(code, awsb.EXIT_FAILED)
        self.assertEqual(asked, [awsb.DESTROY_PROMPT_TIMEOUT_S])
        self.assertTrue(runner.ran("systemctl stop bench-agent"))
        self.assertTrue(runner.ran("rsync", "/opt/bench/out/"))
        self.assertTrue(runner.ran("tofu", "destroy"))
        self.assertIn("interrupted", (harness.run_dir / "summary.md").read_text())

    def test_declined_destroy_exits_4_with_command_last(self):
        harness, runner, code = self.run_interrupted(lambda timeout: False)
        self.assertEqual(code, awsb.EXIT_LEFTOVER)
        self.assertFalse(runner.ran("tofu", "destroy"))
        self.assertTrue(
            harness.last_log_line().endswith(f"aws-bench destroy {harness.run_dir}")
        )

    def test_interrupts_are_ignored_after_disarm(self):
        interrupts = awsb.Interrupts()
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
            harness.last_log_line().endswith(f"aws-bench destroy {harness.run_dir}")
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


# REQ:awsbench-manifest-repro
class ManifestReproTest(unittest.TestCase):
    def render(self) -> tuple[Path, "awsb.RunConfig"]:
        tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, tmp)
        cfg = awsb.RunConfig(
            tag="x",
            nodes=2,
            load=netbench.BenchConfig(submit_nodes=1),
            out_root=tmp,
        )
        hosts = awsb.plan_hosts(cfg)
        run_dir = tmp / "run1"
        run_dir.mkdir()
        prices = two_node_prices()
        estimate = awsb.estimate_cost(
            hosts, cfg, prices, awsb.region_minor_prices("eu-west-1")
        )
        awsb.write_manifest(
            run_dir, cfg, hosts, awsb.plan_peers(hosts), estimate, "planned", ["run"]
        )
        return run_dir, cfg

    def test_manifest_round_trips_and_update_keeps_keys(self):
        run_dir, _ = self.render()
        before = json.loads((run_dir / "manifest.json").read_text())
        updated = awsb.update_manifest(run_dir, "measuring", start_spread_s=0.4)
        after = json.loads((run_dir / "manifest.json").read_text())
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
        with unittest.mock.patch.object(awsb, "SSH_RETRY_S", 0.0):
            awsb.wait_ssh(remote, "ctl", awsb.Interrupts())
        self.assertEqual(len(calls), 3)

    def test_wait_ssh_gives_up_after_the_timeout(self):
        remote = self.remote(
            FakeRunner({("ssh",): completed(returncode=255, stderr="refused")})
        )
        with (
            unittest.mock.patch.object(awsb, "SSH_RETRY_S", 0.0),
            unittest.mock.patch.object(awsb, "SSH_READY_TIMEOUT_S", 0.05),
            self.assertRaisesRegex(awsb.RemoteError, "not reachable"),
        ):
            awsb.wait_ssh(remote, "ctl", awsb.Interrupts())

    def test_gate_times_out_naming_the_gate(self):
        remote = self.remote(
            FakeRunner({("ssh",): completed(returncode=1, stderr="no")})
        )
        with (
            unittest.mock.patch.object(awsb, "GATE_RETRY_S", 0.0),
            self.assertRaisesRegex(awsb.RemoteError, "gate `anvil`"),
        ):
            awsb.gate(remote, "ctl", "anvil", "curl x", awsb.Interrupts(), 0.05)

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
        arns = awsb.sweep(runner, "eu-west-1", "run1")
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

    def test_tagged_resources_without_name_matches_every_run(self):
        runner = FleetRunner([DONE_STATE])
        awsb.tagged_resources(runner, "eu-west-1")
        self.assertTrue(runner.ran("Key=espresso-bench-run"))
        self.assertFalse(runner.ran("Values="))

    def test_aws_failure_raises(self):
        runner = FakeRunner({("aws",): completed(returncode=1, stderr="denied")})
        with self.assertRaisesRegex(awsb.RemoteError, "denied"):
            awsb.tagged_resources(runner, "eu-west-1")

    def test_actual_cost_prices_the_observed_duration(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        minor = awsb.region_minor_prices("eu-west-1")
        estimate = awsb.estimate_cost(hosts, cfg, two_node_prices(), minor)
        manifest = {
            "name": "run1",
            "config": {"profile": "p", "region": "eu-west-1"},
            "hosts": hosts,
            "estimate": estimate,
        }
        runner = FleetRunner([DONE_STATE], describe=DESCRIBE)
        cost = awsb.actual_cost(runner, manifest)
        self.assertEqual(cost["duration_s"], 1800.0)
        expected_lines = awsb._cost_lines(hosts, two_node_prices(), minor, 1800.0)
        self.assertAlmostEqual(
            cost["actual"], sum(line["usd"] for line in expected_lines)
        )
        self.assertEqual(cost["bound"], estimate["bound_usd"])

    def test_actual_cost_without_termination_time_raises(self):
        cfg = awsb.RunConfig(
            tag="x", nodes=2, load=netbench.BenchConfig(submit_nodes=1)
        )
        hosts = awsb.plan_hosts(cfg)
        estimate = awsb.estimate_cost(
            hosts, cfg, two_node_prices(), awsb.region_minor_prices("eu-west-1")
        )
        manifest = {
            "name": "run1",
            "config": {"profile": "p", "region": "eu-west-1"},
            "hosts": hosts,
            "estimate": estimate,
        }
        running = json.dumps([{"launch": "2026-09-29T15:00:00+00:00", "reason": ""}])
        with self.assertRaises(ValueError):
            awsb.actual_cost(FleetRunner([DONE_STATE], describe=running), manifest)


class IndexTest(unittest.TestCase):
    def manifest(self) -> dict:
        return {
            "name": "run1",
            "created_at": "2026-09-29T15:00:00+00:00",
            "git_rev": "a" * 40,
            "config": {"tag": "release-x", "nodes": 5},
            "images": {"espresso-node": {"revision": "bd2ad6e1dc7abc"}},
            "estimate": {"bound_usd": 4.6},
        }

    def test_row_of_a_result(self):
        row = awsb.index_row(self.manifest(), valid_result(), 0, True, {"actual": 2.4})
        self.assertEqual(
            row,
            "| run1 | 2026-09-29T15:00 | aaaaaaaaaa | release-x@bd2ad6e1dc | 5 | 8 | 8 "
            "| yes | 120 | valid | 0 | 4.60/2.40 | yes |\n",
        )

    def test_row_without_result_or_cost(self):
        row = awsb.index_row(self.manifest(), None, 3, False, None)
        self.assertIn("| - | - | - | - | failed | 3 | 4.60/- | no |", row)

    def test_append_writes_the_header_once(self):
        with tempfile.TemporaryDirectory() as tmp:
            for _ in range(2):
                awsb.append_index(Path(tmp), self.manifest(), None, 3, False, None)
            lines = (Path(tmp) / "INDEX.md").read_text().splitlines()
        self.assertEqual(len(lines), 4)
        self.assertTrue(lines[0].startswith("| name |"))


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

    def run_agent(self, ready, load) -> int:
        with (
            unittest.mock.patch.object(netbench, "wait_ready", ready),
            unittest.mock.patch.object(netbench, "drive_load", load),
            unittest.mock.patch.object(netbench, "sample_metrics"),
            unittest.mock.patch.object(awsb.signal, "signal"),
        ):
            return awsb.cmd_agent_drive(self.args)

    def test_done_state_carries_ready_and_window(self):
        code = self.run_agent(lambda *a: 12.0, lambda *a: {"t0": 100.0, "t1": 200.0})
        self.assertEqual(code, awsb.EXIT_OK)
        state = self.state()
        self.assertEqual(state["phase"], "done")
        self.assertEqual(
            (state["ready_s"], state["t0"], state["t1"]), (12.0, 100.0, 200.0)
        )

    def test_progress_follows_netbench_log(self):
        def load(*args):
            netbench.log.info("height 7: 3 submitted")
            return {"t0": 1.0, "t1": 2.0}

        self.run_agent(lambda *a: 1.0, load)
        self.assertEqual(self.state()["progress"], "height 7: 3 submitted")

    def test_not_ready_is_an_error_state(self):
        def not_ready(*args):
            raise netbench.NetworkError("network not ready after 5 s: heights {}")

        code = self.run_agent(not_ready, lambda *a: {"t0": 1.0, "t1": 2.0})
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


def write_collected_run(run_dir: Path) -> dict:
    """A 3-node run dir as `cmd_run` leaves it after collection: netbench's synthetic run plus
    manifest, config, topology and `hosts/<name>/` files."""
    test_netbench.write_run_dir(run_dir)
    cfg = awsb.RunConfig(tag="x", nodes=3, load=netbench.BenchConfig(submit_nodes=2))
    hosts = awsb.plan_hosts(cfg)
    manifest = {
        "name": "run1",
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
    node0 = run_dir / "hosts" / "node0"
    (node0 / "lscpu.txt").write_text("CPU(s): 16\nModel name: Neoverse-V2\nFlags: fp\n")
    (node0 / "meminfo.txt").write_text("MemTotal:       32000000 kB\n")
    (node0 / "uname.txt").write_text("6.8.0-aws\n")
    return manifest


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

    def test_render_is_repeatable(self):
        run_dir, first = self.report()
        self.assertEqual(awsb.write_report(run_dir), first)


if __name__ == "__main__":
    unittest.main()
